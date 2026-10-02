use axum::{
    body::{to_bytes, Body},
    extract::{Query, State},
    http::{header, HeaderMap, Request, StatusCode},
    routing::{get, post},
    Form, Json, Router,
};
use hmac::{Hmac, Mac};
use serde_json::{json, Value};
use sha2::Sha256;
use slimlytics_backend::billing::{default_plans, BillingConfig, StripeConfig};
use slimlytics_backend::{app, AppState};
use sqlx::postgres::PgPoolOptions;
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};
use tower::ServiceExt;

const WEBHOOK_SECRET: &str = "whsec_billing_test";

/// What the fake Stripe API received, and the subscription it currently reports.
#[derive(Default)]
struct FakeStripe {
    requests: Vec<(String, HashMap<String, String>, Option<String>)>,
    subscription: Value,
    customer_user: String,
}
type Shared = Arc<Mutex<FakeStripe>>;

async fn fake_stripe() -> (String, Shared) {
    let shared: Shared = Arc::default();
    let record = |path: &'static str| {
        move |State(s): State<Shared>,
              headers: HeaderMap,
              Form(form): Form<HashMap<String, String>>| async move {
            let key = headers
                .get("idempotency-key")
                .map(|v| v.to_str().unwrap().to_owned());
            s.lock().unwrap().requests.push((path.into(), form, key));
            Json(match path {
                "customers" => json!({"id": "cus_test_1"}),
                "checkout" => {
                    json!({"id": "cs_test_1", "url": "https://checkout.stripe.test/cs_test_1"})
                }
                _ => json!({"url": "https://billing.stripe.test/portal"}),
            })
        }
    };
    let router = Router::new()
        .route("/v1/customers", post(record("customers")))
        .route(
            "/v1/customers/cus_test_1",
            get(|State(s): State<Shared>| async move {
                Json(json!({"id": "cus_test_1", "metadata": {"slimlytics_user_id": s.lock().unwrap().customer_user.clone()}}))
            }),
        )
        .route(
            "/v1/checkout/sessions",
            post(record("checkout")).get(|| async { Json(json!({"data": [{"id": "cs_stale"}]})) }),
        )
        .route(
            "/v1/checkout/sessions/cs_stale/expire",
            post(record("expire")),
        )
        .route("/v1/billing_portal/sessions", post(record("portal")))
        .route(
            "/v1/prices",
            get(|Query(q): Query<HashMap<String, String>>| async move {
                Json(json!({"data": [{"id": format!("price_for_{}", q["lookup_keys[]"])}]}))
            }),
        )
        .route(
            "/v1/subscriptions",
            get(|State(s): State<Shared>| async move {
                Json(json!({"data": [s.lock().unwrap().subscription.clone()]}))
            }),
        )
        .with_state(shared.clone());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
    (format!("http://{address}"), shared)
}

#[tokio::test]
#[ignore = "requires TEST_DATABASE_URL pointing to a disposable PostgreSQL database"]
async fn plans_limit_sites_and_stripe_subscriptions_drive_the_plan() {
    let url = std::env::var("TEST_DATABASE_URL").expect("TEST_DATABASE_URL required");
    let pool = PgPoolOptions::new().connect(&url).await.unwrap();
    sqlx::migrate!("../migrations").run(&pool).await.unwrap();
    let (stripe_base, stripe) = fake_stripe().await;
    let config = BillingConfig::new(
        default_plans(),
        Some(StripeConfig {
            secret_key: "rk_test_fake".into(),
            webhook_secret: WEBHOOK_SECRET.into(),
            api_base: stripe_base,
        }),
    )
    .unwrap();
    let router = app(AppState::new(
        pool.clone(),
        "01234567890123456789012345678901".into(),
        b"abcdefghijklmnopqrstuvwxyz012345".to_vec(),
    )
    .with_billing(config));

    let email = format!("billing+{}@example.com", uuid::Uuid::new_v4());
    let register = call(
        &router,
        "POST",
        "/api/auth/register",
        None,
        json!({"email":email,"password":"long-enough-password"}),
    )
    .await;
    let token = body(register).await["token"].as_str().unwrap().to_owned();
    let user: uuid::Uuid = sqlx::query_scalar("SELECT id FROM users WHERE email=$1")
        .bind(&email)
        .fetch_one(&pool)
        .await
        .unwrap();
    stripe.lock().unwrap().customer_user = user.to_string();
    let site = |name: &str| json!({"name": name, "domain": format!("{}-{}.example.com", name, uuid::Uuid::new_v4()), "timezone": "UTC"});

    // Free allows one site; the second is refused with 402 through both creation paths.
    let first = call(&router, "POST", "/api/sites", Some(&token), site("one")).await;
    assert_eq!(first.status(), StatusCode::CREATED);
    let first_domain = body(first).await["domain"].as_str().unwrap().to_owned();
    let second = call(&router, "POST", "/api/sites", Some(&token), site("two")).await;
    assert_eq!(second.status(), StatusCode::PAYMENT_REQUIRED);
    assert_eq!(body(second).await["error"]["code"], "plan_limit");
    assert_eq!(
        call(
            &router,
            "POST",
            "/api/sites/ensure",
            Some(&token),
            site("three")
        )
        .await
        .status(),
        StatusCode::PAYMENT_REQUIRED
    );
    let reuse = call(
        &router,
        "POST",
        "/api/sites/ensure",
        Some(&token),
        json!({"name":"one","domain":first_domain,"timezone":"UTC"}),
    )
    .await;
    assert!(
        reuse.status().is_success(),
        "reusing an existing site is always allowed"
    );

    let status = body(call(&router, "GET", "/api/billing", Some(&token), Value::Null).await).await;
    assert_eq!(
        (
            status["plan"]["id"].as_str(),
            status["usage"]["sites"].as_i64()
        ),
        (Some("free"), Some(1))
    );
    assert!(
        status["plans"][1].get("stripeMonthlyLookupKey").is_none(),
        "lookup keys stay server-side"
    );

    // Agent/API tokens cannot reach billing.
    let api_token = body(call(&router, "POST", "/api/account/tokens", Some(&token), json!({"name":"agent","expiresInDays":1,"scopes":["sites:read","sites:write","analytics:read"]})).await).await["token"]
        .as_str()
        .unwrap()
        .to_owned();
    assert_eq!(
        call(
            &router,
            "GET",
            "/api/billing",
            Some(&api_token),
            Value::Null
        )
        .await
        .status(),
        StatusCode::FORBIDDEN
    );

    // Checkout: one customer (idempotent), price by lookup key, subscription mode.
    let checkout = body(
        call(
            &router,
            "POST",
            "/api/billing/checkout",
            Some(&token),
            json!({"plan":"pro","interval":"year"}),
        )
        .await,
    )
    .await;
    assert_eq!(checkout["url"], "https://checkout.stripe.test/cs_test_1");
    {
        let fake = stripe.lock().unwrap();
        let customer = fake.requests.iter().find(|r| r.0 == "customers").unwrap();
        assert_eq!(customer.1["metadata[slimlytics_user_id]"], user.to_string());
        assert_eq!(
            customer.2.as_deref(),
            Some(format!("slimlytics-customer-{user}").as_str())
        );
        let checkout_request = fake.requests.iter().find(|r| r.0 == "checkout").unwrap();
        assert_eq!(
            checkout_request.2.as_deref(),
            Some(format!("slimlytics-checkout-{user}-0-price_for_slimlytics_pro_annual").as_str()),
            "retries after an ambiguous failure reuse the same session"
        );
        let session = &checkout_request.1;
        assert_eq!(session["mode"], "subscription");
        assert_eq!(session["customer"], "cus_test_1");
        assert_eq!(
            session["line_items[0][price]"],
            "price_for_slimlytics_pro_annual"
        );
        assert!(
            session
                .keys()
                .all(|k| !k.starts_with("payment_method_types")),
            "dynamic payment methods"
        );
        assert!(session.contains_key("integration_identifier"));
        assert!(
            fake.requests.iter().any(|r| r.0 == "expire"),
            "earlier open sessions are expired so only one is payable"
        );
    }

    // Webhook: bad signatures are rejected; a valid event syncs the plan from Stripe once.
    stripe.lock().unwrap().subscription = json!({
        "id": "sub_1", "status": "active", "created": 1,
        "items": {"data": [{"price": {"lookup_key": "slimlytics_pro_annual"}, "current_period_end": 1_900_000_000}]}
    });
    let event = json!({"id": "evt_1", "type": "customer.subscription.created", "data": {"object": {"customer": "cus_test_1"}}}).to_string();
    assert_eq!(
        webhook(&router, &event, "t=1,v1=00").await.status(),
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        webhook(&router, &event, &sign(&event)).await.status(),
        StatusCode::OK
    );
    let (plan, interval): (String, String) =
        sqlx::query_as("SELECT plan,billing_interval FROM account_billing WHERE user_id=$1")
            .bind(user)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!((plan.as_str(), interval.as_str()), ("pro", "year"));
    assert_eq!(
        call(&router, "POST", "/api/sites", Some(&token), site("two"))
            .await
            .status(),
        StatusCode::CREATED,
        "Pro allows more sites"
    );

    // A lost local link (e.g. checkout rolled back after Stripe made the session) is recovered
    // from the customer's metadata.
    sqlx::query("DELETE FROM account_billing WHERE user_id=$1")
        .bind(user)
        .execute(&pool)
        .await
        .unwrap();
    let orphan = json!({"id": "evt_orphan", "type": "invoice.paid", "data": {"object": {"customer": "cus_test_1"}}}).to_string();
    assert_eq!(
        webhook(&router, &orphan, &sign(&orphan)).await.status(),
        StatusCode::OK
    );
    let recovered: String = sqlx::query_scalar("SELECT plan FROM account_billing WHERE user_id=$1")
        .bind(user)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(recovered, "pro");

    // A redelivered event is ignored; cancellation drops back to Free.
    stripe.lock().unwrap().subscription =
        json!({"id": "sub_1", "status": "canceled", "created": 1, "items": {"data": []}});
    let duplicate = body(webhook(&router, &event, &sign(&event)).await).await;
    assert_eq!(duplicate["duplicate"], true);
    let canceled = json!({"id": "evt_2", "type": "customer.subscription.deleted", "data": {"object": {"customer": "cus_test_1"}}}).to_string();
    assert_eq!(
        webhook(&router, &canceled, &sign(&canceled)).await.status(),
        StatusCode::OK
    );
    let plan: String = sqlx::query_scalar("SELECT plan FROM account_billing WHERE user_id=$1")
        .bind(user)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(plan, "free");

    // Admin comps survive webhooks and lift limits.
    sqlx::query("UPDATE account_billing SET admin_plan='unlimited' WHERE user_id=$1")
        .bind(user)
        .execute(&pool)
        .await
        .unwrap();
    let again = json!({"id": "evt_3", "type": "invoice.paid", "data": {"object": {"customer": "cus_test_1"}}}).to_string();
    webhook(&router, &again, &sign(&again)).await;
    let plan: String =
        sqlx::query_scalar("SELECT admin_plan FROM account_billing WHERE user_id=$1")
            .bind(user)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(plan, "unlimited");
    assert_eq!(
        call(&router, "POST", "/api/sites", Some(&token), site("five"))
            .await
            .status(),
        StatusCode::CREATED
    );

    // Releasing a grant without a live subscription falls back to the default plan at once.
    sqlx::query("UPDATE account_billing SET admin_plan=NULL WHERE user_id=$1")
        .bind(user)
        .execute(&pool)
        .await
        .unwrap();
    let status = body(call(&router, "GET", "/api/billing", Some(&token), Value::Null).await).await;
    assert_eq!(status["plan"]["id"], "free");

    // Releasing a grant from a paying subscriber restores their subscription's plan.
    stripe.lock().unwrap().subscription = json!({
        "id": "sub_2", "status": "active", "created": 2,
        "items": {"data": [{"price": {"lookup_key": "slimlytics_business_monthly"}, "current_period_end": 1_900_000_000}]}
    });
    sqlx::query("UPDATE account_billing SET admin_plan='unlimited' WHERE user_id=$1")
        .bind(user)
        .execute(&pool)
        .await
        .unwrap();
    let renewed = json!({"id": "evt_4", "type": "customer.subscription.created", "data": {"object": {"customer": "cus_test_1"}}}).to_string();
    webhook(&router, &renewed, &sign(&renewed)).await;
    sqlx::query("UPDATE account_billing SET admin_plan=NULL WHERE user_id=$1")
        .bind(user)
        .execute(&pool)
        .await
        .unwrap();
    let status = body(call(&router, "GET", "/api/billing", Some(&token), Value::Null).await).await;
    assert_eq!(status["plan"]["id"], "business");
}

fn sign(payload: &str) -> String {
    let t = chrono::Utc::now().timestamp();
    let mut mac = Hmac::<Sha256>::new_from_slice(WEBHOOK_SECRET.as_bytes()).unwrap();
    mac.update(format!("{t}.{payload}").as_bytes());
    let hex: String = mac
        .finalize()
        .into_bytes()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    format!("t={t},v1={hex}")
}

async fn webhook(router: &Router, payload: &str, signature: &str) -> axum::response::Response {
    router
        .clone()
        .oneshot(
            Request::post("/api/billing/webhook")
                .header("stripe-signature", signature)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(payload.to_owned()))
                .unwrap(),
        )
        .await
        .unwrap()
}

async fn call(
    router: &Router,
    method: &str,
    uri: &str,
    token: Option<&str>,
    payload: Value,
) -> axum::response::Response {
    let mut builder = Request::builder()
        .method(method)
        .uri(uri)
        .header(header::CONTENT_TYPE, "application/json");
    if let Some(token) = token {
        builder = builder.header(header::AUTHORIZATION, format!("Bearer {token}"));
    }
    let body = if payload.is_null() {
        Body::empty()
    } else {
        Body::from(payload.to_string())
    };
    router
        .clone()
        .oneshot(builder.body(body).unwrap())
        .await
        .unwrap()
}

async fn body(response: axum::response::Response) -> Value {
    serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap()).unwrap()
}
