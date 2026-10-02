//! Billing HTTP routes: plans, account status and usage, Stripe Checkout and Customer
//! Portal sessions, the Stripe webhook, and site-limit enforcement. See `crate::billing` for
//! configuration. All routes report `enabled: false` (or 404 for actions) when billing is off.
use super::*;
use crate::billing::{
    verify_webhook_signature, BillingConfig, Interval, Plan, StripeConfig, STRIPE_API_VERSION,
};
use axum::body::Bytes;

/// Tags Checkout Sessions created by this integration in the Stripe Dashboard.
const INTEGRATION_IDENTIFIER: &str = "slimlytics-subscriptions-qhtwkzpd";
const WEBHOOK_TOLERANCE_SECONDS: i64 = 300;

pub(super) fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/billing/plans", get(plans))
        .route("/api/billing", get(status))
        .route("/api/billing/checkout", post(checkout))
        .route("/api/billing/portal", post(portal))
        .route("/api/billing/webhook", post(webhook))
}

/// A signed-in person. Billing changes money, so API and agent tokens are refused.
struct SessionUser(Uuid);
impl FromRequestParts<AppState> for SessionUser {
    type Rejection = ApiError;
    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let bearer = parts
            .headers
            .get(header::AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.strip_prefix("Bearer "))
            .unwrap_or("");
        if bearer.starts_with("slyt_") {
            return Err(ApiError::Forbidden);
        }
        let CurrentUser(user) = CurrentUser::from_request_parts(parts, state).await?;
        Ok(Self(user))
    }
}

fn config(state: &AppState) -> Result<&Arc<BillingConfig>, ApiError> {
    state.billing.as_ref().ok_or(ApiError::NotFound)
}

fn stripe(state: &AppState) -> Result<&StripeConfig, ApiError> {
    config(state)?
        .stripe
        .as_ref()
        .ok_or_else(|| ApiError::BadRequest("payments are not configured on this server".into()))
}

#[derive(sqlx::FromRow)]
struct Account {
    /// The plan the Stripe subscription grants; always kept in sync by webhooks.
    plan: String,
    /// An administrator's grant, which overrides `plan` until released.
    admin_plan: Option<String>,
    stripe_customer_id: Option<String>,
    subscription_status: Option<String>,
    billing_interval: Option<String>,
    current_period_end: Option<DateTime<Utc>>,
}

async fn account<'e>(
    executor: impl sqlx::PgExecutor<'e>,
    user: Uuid,
) -> Result<Option<Account>, ApiError> {
    Ok(sqlx::query_as(
        "SELECT plan,admin_plan,stripe_customer_id,subscription_status,billing_interval,current_period_end
         FROM account_billing WHERE user_id=$1",
    )
    .bind(user)
    .fetch_optional(executor)
    .await?)
}

/// The plan governing an account. Admin grants apply as stored; a Stripe-managed plan only
/// applies while its subscription is in good standing (e.g. after an admin grant is released
/// from an account that never subscribed), otherwise the default plan does.
fn effective_plan(config: &BillingConfig, account: Option<&Account>) -> Plan {
    let id = match account {
        Some(Account {
            admin_plan: Some(plan),
            ..
        }) => plan.as_str(),
        Some(a)
            if matches!(
                a.subscription_status.as_deref(),
                Some("active" | "trialing" | "past_due")
            ) =>
        {
            a.plan.as_str()
        }
        _ => config.default_plan.as_str(),
    };
    config.plan(id)
}

/// Serializes work per key for the life of the transaction (site creation per account,
/// checkout per account, subscription sync per customer).
async fn advisory_lock(conn: &mut sqlx::PgConnection, key: &str) -> Result<(), ApiError> {
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1, 0))")
        .bind(key)
        .execute(conn)
        .await?;
    Ok(())
}

async fn owned_sites<'e>(executor: impl sqlx::PgExecutor<'e>, user: Uuid) -> Result<i64, ApiError> {
    Ok(sqlx::query_scalar(
        "SELECT count(*) FROM site_memberships WHERE user_id=$1 AND role='owner'",
    )
    .bind(user)
    .fetch_one(executor)
    .await?)
}

/// Rejects creating another site when billing is on and the account's plan is full. Runs in
/// the creating transaction and locks the account, so concurrent creations can't both pass.
pub(super) async fn ensure_site_allowance(
    state: &AppState,
    tx: &mut sqlx::PgConnection,
    user: Uuid,
) -> Result<(), ApiError> {
    let Some(config) = state.billing.as_ref() else {
        return Ok(());
    };
    advisory_lock(tx, &format!("slimlytics-sites:{user}")).await?;
    let plan = effective_plan(config, account(&mut *tx, user).await?.as_ref());
    if let Some(limit) = plan.sites {
        if owned_sites(&mut *tx, user).await? >= i64::from(limit) {
            return Err(ApiError::PlanLimit(format!(
                "the {} plan includes {limit} {}; upgrade to add more",
                plan.name,
                if limit == 1 { "site" } else { "sites" }
            )));
        }
    }
    Ok(())
}

async fn plans(State(state): State<AppState>) -> Json<Value> {
    match state.billing.as_ref() {
        Some(config) => Json(
            json!({"enabled": true, "plans": config.plans, "checkoutAvailable": config.stripe.is_some()}),
        ),
        None => Json(json!({"enabled": false, "plans": []})),
    }
}

async fn status(
    State(state): State<AppState>,
    SessionUser(user): SessionUser,
) -> Result<Json<Value>, ApiError> {
    let Some(config) = state.billing.as_ref() else {
        return Ok(Json(json!({"enabled": false})));
    };
    let account = account(&state.pool, user).await?;
    let plan = effective_plan(config, account.as_ref());
    let sites = owned_sites(&state.pool, user).await?;
    // Human page views since UTC midnight across the account's own sites.
    let page_views_today: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM events e JOIN site_memberships m ON m.site_id=e.site_id
         WHERE m.user_id=$1 AND m.role='owner' AND e.event_name='pageview' AND e.traffic_class='human'
           AND e.occurred_at >= date_trunc('day', now() AT TIME ZONE 'UTC') AT TIME ZONE 'UTC'",
    )
    .bind(user)
    .fetch_one(&state.pool)
    .await?;
    Ok(Json(json!({
        "enabled": true,
        "plan": plan,
        "planSource": match &account {
            Some(a) if a.admin_plan.is_some() => "admin",
            Some(_) => "stripe",
            None => "default",
        },
        "subscriptionStatus": account.as_ref().and_then(|a| a.subscription_status.clone()),
        "interval": account.as_ref().and_then(|a| a.billing_interval.clone()),
        "currentPeriodEnd": account.as_ref().and_then(|a| a.current_period_end),
        "hasBillingAccount": account.as_ref().is_some_and(|a| a.stripe_customer_id.is_some()),
        "usage": {"sites": sites, "pageViewsToday": page_views_today},
        "plans": config.plans,
        "checkoutAvailable": config.stripe.is_some(),
    })))
}

#[derive(Deserialize)]
struct CheckoutInput {
    plan: String,
    #[serde(default = "monthly")]
    interval: Interval,
}
fn monthly() -> Interval {
    Interval::Month
}

async fn checkout(
    State(state): State<AppState>,
    SessionUser(user): SessionUser,
    Json(input): Json<CheckoutInput>,
) -> Result<Json<Value>, ApiError> {
    let config = config(&state)?.clone();
    let stripe = stripe(&state)?.clone();
    let plan = config
        .plans
        .iter()
        .find(|plan| plan.id == input.plan)
        .ok_or_else(|| ApiError::BadRequest("unknown plan".into()))?;
    let lookup_key = plan.lookup_key(input.interval).ok_or_else(|| {
        ApiError::BadRequest("this plan cannot be purchased with that interval".into())
    })?;
    // One checkout at a time per account, held until the new session exists. A try-lock, so
    // concurrent attempts fail fast instead of parking pooled connections behind the holder.
    let mut lock = state.pool.begin().await?;
    let acquired: bool =
        sqlx::query_scalar("SELECT pg_try_advisory_xact_lock(hashtextextended($1, 0))")
            .bind(format!("slimlytics-checkout:{user}"))
            .fetch_one(&mut *lock)
            .await?;
    if !acquired {
        return Err(ApiError::BadRequest(
            "a checkout is already being prepared; try again in a moment".into(),
        ));
    }
    // All database work below reuses the lock's connection, so a checkout never waits on the
    // pool while holding one.
    let existing = account(&mut *lock, user).await?;
    if existing.as_ref().is_some_and(|a| a.admin_plan.is_some()) {
        return Err(ApiError::BadRequest(
            "your plan is managed by an administrator".into(),
        ));
    }
    let customer = ensure_customer(&state, &mut lock, &stripe, user, existing.as_ref()).await?;
    // Retire earlier unpaid sessions first, so the reconciliation below sees any that completed
    // in the meantime. Expiring one that just completed fails; the sync then finds its
    // subscription and sends the user to the portal instead of a second checkout.
    let open = stripe_get(
        &state,
        &stripe,
        "/v1/checkout/sessions",
        &[
            ("customer", customer.as_str()),
            ("status", "open"),
            ("limit", "100"),
        ],
    )
    .await?;
    for session in open["data"].as_array().into_iter().flatten() {
        if let Some(id) = session["id"].as_str() {
            let path = format!("/v1/checkout/sessions/{id}/expire");
            if stripe_post(&state, &stripe, &path, &[], None)
                .await
                .is_err()
            {
                // Fine if it already completed or expired; otherwise it may still be payable,
                // so fail closed rather than open a second checkout.
                let current =
                    stripe_get(&state, &stripe, &format!("/v1/checkout/sessions/{id}"), &[])
                        .await?;
                if !matches!(current["status"].as_str(), Some("complete" | "expired")) {
                    return Err(ApiError::BadRequest(
                        "a previous checkout is still open; try again in a moment".into(),
                    ));
                }
            }
        }
    }
    // Reconcile with Stripe: a subscription may exist that no webhook has delivered yet.
    sync_customer(&state, &mut lock, &stripe, &customer).await?;
    let existing = account(&mut *lock, user).await?;
    // Plan changes for existing subscribers go through the Customer Portal (proration,
    // cancellation), never a second subscription.
    if existing
        .as_ref()
        .and_then(|a| a.subscription_status.as_deref())
        .is_some_and(|status| {
            matches!(
                status,
                "active" | "trialing" | "past_due" | "unpaid" | "paused"
            )
        })
    {
        let url = portal_session(&state, &stripe, &customer).await?;
        // Keep the reconciliation (and any new customer row) made above.
        lock.commit().await?;
        return Ok(Json(json!({"url": url, "portal": true})));
    }
    // A subscription whose first payment is still pending (e.g. awaiting authentication) would
    // otherwise sit beside the new one; cancel it, failing closed if Stripe refuses.
    let pending = stripe_get(
        &state,
        &stripe,
        "/v1/subscriptions",
        &[
            ("customer", customer.as_str()),
            ("status", "incomplete"),
            ("limit", "100"),
        ],
    )
    .await?;
    for subscription in pending["data"].as_array().into_iter().flatten() {
        if let Some(id) = subscription["id"].as_str() {
            let request = state
                .http
                .delete(format!("{}/v1/subscriptions/{id}", stripe.api_base));
            if send(request, &stripe).await.is_err() {
                return Err(ApiError::BadRequest(
                    "a previous subscription payment is still pending; try again in a moment"
                        .into(),
                ));
            }
        }
    }
    let prices = stripe_get(
        &state,
        &stripe,
        "/v1/prices",
        &[
            ("lookup_keys[]", lookup_key),
            ("active", "true"),
            ("limit", "1"),
        ],
    )
    .await?;
    let price = prices["data"][0]["id"]
        .as_str()
        .ok_or_else(|| {
            ApiError::BadRequest(format!(
                "no active Stripe price with lookup key {lookup_key}"
            ))
        })?
        .to_owned();
    let base = state.public_url.trim_end_matches('/');
    let user_id = user.to_string();
    let session = stripe_post(
        &state,
        &stripe,
        "/v1/checkout/sessions",
        &[
            ("mode", "subscription"),
            ("customer", customer.as_str()),
            ("client_reference_id", user_id.as_str()),
            ("line_items[0][price]", price.as_str()),
            ("line_items[0][quantity]", "1"),
            (
                "subscription_data[metadata][slimlytics_user_id]",
                user_id.as_str(),
            ),
            ("success_url", &format!("{base}/app?billing=success")),
            ("cancel_url", &format!("{base}/pricing")),
            ("integration_identifier", INTEGRATION_IDENTIFIER),
        ],
        None,
    )
    .await?;
    let url = session["url"].as_str().ok_or(ApiError::Internal)?;
    lock.commit().await?;
    Ok(Json(json!({"url": url, "portal": false})))
}

async fn portal(
    State(state): State<AppState>,
    SessionUser(user): SessionUser,
) -> Result<Json<Value>, ApiError> {
    let stripe = stripe(&state)?.clone();
    let customer = account(&state.pool, user)
        .await?
        .and_then(|a| a.stripe_customer_id)
        .ok_or_else(|| {
            ApiError::BadRequest("no billing account yet; choose a plan first".into())
        })?;
    Ok(Json(
        json!({"url": portal_session(&state, &stripe, &customer).await?}),
    ))
}

async fn portal_session(
    state: &AppState,
    stripe: &StripeConfig,
    customer: &str,
) -> Result<String, ApiError> {
    let return_url = format!("{}/app", state.public_url.trim_end_matches('/'));
    let session = stripe_post(
        state,
        stripe,
        "/v1/billing_portal/sessions",
        &[("customer", customer), ("return_url", &return_url)],
        None,
    )
    .await?;
    session["url"]
        .as_str()
        .map(str::to_owned)
        .ok_or(ApiError::Internal)
}

/// The account's Stripe customer, created on first checkout. The idempotency key makes
/// retries (double clicks, network errors) reuse one customer.
async fn ensure_customer(
    state: &AppState,
    conn: &mut sqlx::PgConnection,
    stripe: &StripeConfig,
    user: Uuid,
    existing: Option<&Account>,
) -> Result<String, ApiError> {
    if let Some(customer) = existing.and_then(|a| a.stripe_customer_id.clone()) {
        return Ok(customer);
    }
    let email: String = sqlx::query_scalar("SELECT email FROM users WHERE id=$1")
        .bind(user)
        .fetch_one(&mut *conn)
        .await?;
    let user_id = user.to_string();
    let customer = stripe_post(
        state,
        stripe,
        "/v1/customers",
        &[
            ("email", email.as_str()),
            ("metadata[slimlytics_user_id]", user_id.as_str()),
        ],
        Some(&format!("slimlytics-customer-{user}")),
    )
    .await?;
    let id = customer["id"]
        .as_str()
        .ok_or(ApiError::Internal)?
        .to_owned();
    let default_plan = config(state)?.default_plan.clone();
    let stored: String = sqlx::query_scalar(
        "INSERT INTO account_billing(user_id,plan,stripe_customer_id) VALUES($1,$2,$3)
         ON CONFLICT (user_id) DO UPDATE SET
           stripe_customer_id=COALESCE(account_billing.stripe_customer_id,EXCLUDED.stripe_customer_id),
           updated_at=now()
         RETURNING stripe_customer_id",
    )
    .bind(user)
    .bind(default_plan)
    .bind(&id)
    .fetch_one(&mut *conn)
    .await?;
    Ok(stored)
}

/// Stripe webhook: verifies the signature against the raw body, applies each event once, and
/// re-syncs the customer's subscription from Stripe rather than trusting event ordering.
async fn webhook(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Json<Value>, ApiError> {
    let stripe = stripe(&state)?.clone();
    let signature = headers
        .get("stripe-signature")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    if !verify_webhook_signature(
        &body,
        signature,
        &stripe.webhook_secret,
        Utc::now().timestamp(),
        WEBHOOK_TOLERANCE_SECONDS,
    ) {
        return Err(ApiError::BadRequest("invalid Stripe signature".into()));
    }
    let event: Value =
        serde_json::from_slice(&body).map_err(|_| ApiError::BadRequest("invalid event".into()))?;
    let id = event["id"]
        .as_str()
        .ok_or_else(|| ApiError::BadRequest("event without id".into()))?;
    let kind = event["type"].as_str().unwrap_or("");
    let seen: Option<String> =
        sqlx::query_scalar("SELECT id FROM stripe_webhook_events WHERE id=$1")
            .bind(id)
            .fetch_optional(&state.pool)
            .await?;
    if seen.is_some() {
        return Ok(Json(json!({"received": true, "duplicate": true})));
    }
    let relevant = matches!(
        kind,
        "checkout.session.completed"
            | "checkout.session.async_payment_succeeded"
            | "customer.subscription.created"
            | "customer.subscription.updated"
            | "customer.subscription.deleted"
            | "invoice.paid"
            | "invoice.payment_failed"
    );
    if relevant {
        if let Some(customer) = event["data"]["object"]["customer"].as_str() {
            // A failure here returns 5xx so Stripe retries; the event is recorded only after.
            let mut conn = state.pool.acquire().await?;
            sync_customer(&state, &mut conn, &stripe, customer).await?;
        }
    }
    sqlx::query("INSERT INTO stripe_webhook_events(id,type) VALUES($1,$2) ON CONFLICT DO NOTHING")
        .bind(id)
        .bind(kind)
        .execute(&state.pool)
        .await?;
    Ok(Json(json!({"received": true})))
}

/// Pulls the customer's subscriptions from Stripe and updates the account. Admin-comped plans
/// keep their plan but still record the subscription details.
async fn sync_customer(
    state: &AppState,
    conn: &mut sqlx::PgConnection,
    stripe: &StripeConfig,
    customer: &str,
) -> Result<(), ApiError> {
    use sqlx::Connection;
    let config = config(state)?;
    // Hold a per-customer lock across the fetch and the write, so a slower handler can't
    // overwrite a newer snapshot with an older one. Inside a caller's transaction this is a
    // savepoint, and the lock lasts until that transaction ends.
    let mut tx = conn.begin().await?;
    advisory_lock(&mut tx, &format!("slimlytics-stripe:{customer}")).await?;
    // Every subscription, following pagination: a live one may be older than many canceled ones.
    let mut list: Vec<Value> = Vec::new();
    loop {
        let mut query = vec![("customer", customer), ("status", "all"), ("limit", "100")];
        let after = list
            .last()
            .and_then(|s| s["id"].as_str())
            .map(str::to_owned);
        if let Some(after) = after.as_deref() {
            query.push(("starting_after", after));
        }
        let page = stripe_get(state, stripe, "/v1/subscriptions", &query).await?;
        let data = page["data"].as_array().cloned().unwrap_or_default();
        let done = data.is_empty() || !page["has_more"].as_bool().unwrap_or(false);
        list.extend(data);
        if done {
            break;
        }
    }
    // Prefer a subscription in good standing; otherwise the most recent one.
    let current = list
        .iter()
        .filter(|s| {
            matches!(
                s["status"].as_str(),
                Some("active" | "trialing" | "past_due")
            )
        })
        .max_by_key(|s| s["created"].as_i64().unwrap_or(0))
        .or_else(|| {
            list.iter()
                .max_by_key(|s| s["created"].as_i64().unwrap_or(0))
        });
    let (plan, interval) = current
        .map(|s| config.plan_for_subscription(s))
        .unwrap_or((config.default_plan.clone(), None));
    let subscription_id = current.and_then(|s| s["id"].as_str());
    let status = current.and_then(|s| s["status"].as_str());
    // Recent API versions report the billing period per subscription item.
    let period_end = current
        .and_then(|s| {
            s["items"]["data"][0]["current_period_end"]
                .as_i64()
                .or_else(|| s["current_period_end"].as_i64())
        })
        .and_then(|ts| DateTime::<Utc>::from_timestamp(ts, 0));
    let interval = interval.map(|i| if i == Interval::Year { "year" } else { "month" });
    let updated = sqlx::query(
        "UPDATE account_billing SET
           plan=$2,
           stripe_subscription_id=$3, subscription_status=$4, billing_interval=$5,
           current_period_end=$6, updated_at=now()
         WHERE stripe_customer_id=$1",
    )
    .bind(customer)
    .bind(&plan)
    .bind(subscription_id)
    .bind(status)
    .bind(interval)
    .bind(period_end)
    .execute(&mut *tx)
    .await?;
    if updated.rows_affected() == 0 {
        // No local link to this customer, e.g. checkout's transaction rolled back after Stripe
        // created a payable session. Recover the account from the customer's metadata (set at
        // creation) so a paid subscription is never orphaned.
        let found = stripe_get(state, stripe, &format!("/v1/customers/{customer}"), &[]).await?;
        let user = found["metadata"]["slimlytics_user_id"]
            .as_str()
            .and_then(|id| id.parse::<Uuid>().ok());
        if let Some(user) = user {
            sqlx::query(
                "INSERT INTO account_billing(user_id,plan,stripe_customer_id,stripe_subscription_id,
                   subscription_status,billing_interval,current_period_end)
                 SELECT $1,$2,$3,$4,$5,$6,$7 WHERE EXISTS (SELECT 1 FROM users WHERE id=$1)
                 ON CONFLICT (user_id) DO UPDATE SET
                   plan=EXCLUDED.plan, stripe_customer_id=EXCLUDED.stripe_customer_id,
                   stripe_subscription_id=EXCLUDED.stripe_subscription_id,
                   subscription_status=EXCLUDED.subscription_status,
                   billing_interval=EXCLUDED.billing_interval,
                   current_period_end=EXCLUDED.current_period_end, updated_at=now()
                 WHERE account_billing.stripe_customer_id IS NULL",
            )
            .bind(user)
            .bind(&plan)
            .bind(customer)
            .bind(subscription_id)
            .bind(status)
            .bind(interval)
            .bind(period_end)
            .execute(&mut *tx)
            .await?;
        }
    }
    tx.commit().await?;
    Ok(())
}

async fn stripe_get(
    state: &AppState,
    stripe: &StripeConfig,
    path: &str,
    query: &[(&str, &str)],
) -> Result<Value, ApiError> {
    let request = state
        .http
        .get(format!("{}{path}", stripe.api_base))
        .query(query);
    send(request, stripe).await
}

async fn stripe_post(
    state: &AppState,
    stripe: &StripeConfig,
    path: &str,
    form: &[(&str, &str)],
    idempotency_key: Option<&str>,
) -> Result<Value, ApiError> {
    let mut request = state
        .http
        .post(format!("{}{path}", stripe.api_base))
        .form(form);
    if let Some(key) = idempotency_key {
        request = request.header("Idempotency-Key", key);
    }
    send(request, stripe).await
}

async fn send(request: reqwest::RequestBuilder, stripe: &StripeConfig) -> Result<Value, ApiError> {
    let response = request
        .bearer_auth(&stripe.secret_key)
        .header("Stripe-Version", STRIPE_API_VERSION)
        .timeout(std::time::Duration::from_secs(20))
        .send()
        .await
        .map_err(|error| {
            tracing::error!(%error, "Stripe request failed");
            ApiError::Internal
        })?;
    let status = response.status();
    let body: Value = response.json().await.map_err(|_| ApiError::Internal)?;
    if !status.is_success() {
        // Stripe error messages are safe to log; they never contain the key.
        tracing::error!(%status, message = body["error"]["message"].as_str().unwrap_or(""), "Stripe API error");
        return Err(ApiError::Internal);
    }
    Ok(body)
}
