//! Platform admin portal. Every route needs an admin account signed in through a session
//! that verified a passkey within the MFA window; API tokens are never accepted. Admin
//! status is granted only out of band (scripts/admin-grant.sh), never through this API.
use super::*;

pub(super) fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/admin/overview", get(overview))
        .route("/api/admin/users", get(list_users))
        .route(
            "/api/admin/users/{user_id}",
            get(user_detail).delete(delete_user),
        )
        .route("/api/admin/users/{user_id}/disable", post(disable_user))
        .route("/api/admin/users/{user_id}/enable", post(enable_user))
        .route(
            "/api/admin/users/{user_id}/revoke-sessions",
            post(revoke_user_sessions),
        )
        .route("/api/admin/audit", get(audit_log))
}

struct Admin {
    user_id: Uuid,
    email: String,
}

impl FromRequestParts<AppState> for Admin {
    type Rejection = ApiError;
    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let signed_in = SignedIn::from_request_parts(parts, state).await?;
        let row: Option<(String, bool)> =
            sqlx::query_as("SELECT email,is_admin FROM users WHERE id=$1")
                .bind(signed_in.user_id)
                .fetch_optional(&state.pool)
                .await?;
        let (email, is_admin) = row.ok_or(ApiError::Unauthorized)?;
        if !is_admin {
            return Err(ApiError::Forbidden);
        }
        if !session_mfa_verified(&state.pool, signed_in.session_id).await? {
            return Err(ApiError::MfaRequired);
        }
        Ok(Self {
            user_id: signed_in.user_id,
            email,
        })
    }
}

async fn audit(
    executor: impl sqlx::PgExecutor<'_>,
    admin: &Admin,
    action: &str,
    target: Uuid,
    target_email: &str,
    metadata: Value,
) -> Result<(), ApiError> {
    sqlx::query(
        "INSERT INTO admin_audit_log(actor_user_id,actor_email,action,target_user_id,target_email,metadata)
         VALUES($1,$2,$3,$4,$5,$6)",
    )
    .bind(admin.user_id)
    .bind(&admin.email)
    .bind(action)
    .bind(target)
    .bind(target_email)
    .bind(metadata)
    .execute(executor)
    .await?;
    Ok(())
}

/// Locks the target account and refuses actions an admin must not take through the portal.
/// Runs inside the mutation's transaction, so a concurrent admin grant cannot slip in
/// between this check and the change.
async fn actionable_target(
    tx: &mut Transaction<'_, Postgres>,
    admin: &Admin,
    target: Uuid,
) -> Result<String, ApiError> {
    let row: Option<(String, bool)> =
        sqlx::query_as("SELECT email,is_admin FROM users WHERE id=$1 FOR NO KEY UPDATE")
            .bind(target)
            .fetch_optional(&mut **tx)
            .await?;
    let (email, is_admin) = row.ok_or(ApiError::NotFound)?;
    if target == admin.user_id {
        return Err(ApiError::BadRequest(
            "manage your own account from account settings".into(),
        ));
    }
    if is_admin {
        return Err(ApiError::BadRequest(
            "remove admin access with scripts/admin-grant.sh before changing this account".into(),
        ));
    }
    Ok(email)
}

async fn overview(State(state): State<AppState>, _: Admin) -> Result<Json<Value>, ApiError> {
    let row: (i64, i64, i64, i64, i64, i64) = sqlx::query_as(
        "SELECT
           (SELECT count(*) FROM users),
           (SELECT count(*) FROM users WHERE is_admin),
           (SELECT count(*) FROM users WHERE disabled_at IS NOT NULL),
           (SELECT count(*) FROM users WHERE created_at > now()-interval '7 days'),
           (SELECT count(*) FROM sites),
           (SELECT count(DISTINCT user_id) FROM user_sessions
              WHERE revoked_at IS NULL AND expires_at>now() AND last_used_at > now()-interval '7 days')",
    )
    .fetch_one(&state.pool)
    .await?;
    Ok(Json(json!({
        "users": row.0, "admins": row.1, "disabledUsers": row.2,
        "signupsLast7Days": row.3, "sites": row.4, "activeUsersLast7Days": row.5,
    })))
}

#[derive(Deserialize)]
struct UserQuery {
    q: Option<String>,
    limit: Option<i64>,
    offset: Option<i64>,
}

async fn list_users(
    State(state): State<AppState>,
    _: Admin,
    Query(query): Query<UserQuery>,
) -> Result<Json<Value>, ApiError> {
    let limit = query.limit.unwrap_or(50).clamp(1, 200);
    let offset = query.offset.unwrap_or(0).max(0);
    let search = query.q.unwrap_or_default();
    if search.chars().count() > 200 {
        return Err(ApiError::BadRequest("search is too long".into()));
    }
    // Match the search literally: escape LIKE wildcards before wrapping it in %...%.
    let pattern = format!(
        "%{}%",
        search
            .trim()
            .to_lowercase()
            .replace('\\', "\\\\")
            .replace('%', "\\%")
            .replace('_', "\\_")
    );
    let total: i64 = sqlx::query_scalar("SELECT count(*) FROM users WHERE email LIKE $1")
        .bind(&pattern)
        .fetch_one(&state.pool)
        .await?;
    type Row = (
        Uuid,
        String,
        DateTime<Utc>,
        Option<DateTime<Utc>>,
        Option<DateTime<Utc>>,
        bool,
        i64,
        i64,
        i64,
        Option<String>,
    );
    let rows: Vec<Row> = sqlx::query_as(
        "SELECT u.id,u.email,u.created_at,u.last_login_at,u.disabled_at,u.is_admin,
           (SELECT count(*) FROM site_memberships m WHERE m.user_id=u.id),
           (SELECT count(*) FROM user_passkeys p WHERE p.user_id=u.id),
           (SELECT count(*) FROM user_sessions s
              WHERE s.user_id=u.id AND s.revoked_at IS NULL AND s.expires_at>now()),
           (SELECT coalesce(b.admin_plan,b.plan) FROM account_billing b WHERE b.user_id=u.id)
         FROM users u WHERE u.email LIKE $1
         ORDER BY u.created_at DESC LIMIT $2 OFFSET $3",
    )
    .bind(&pattern)
    .bind(limit)
    .bind(offset)
    .fetch_all(&state.pool)
    .await?;
    let users: Vec<Value> = rows
        .into_iter()
        .map(|row| {
            json!({
                "id": row.0, "email": row.1, "createdAt": row.2, "lastLoginAt": row.3,
                "disabledAt": row.4, "isAdmin": row.5, "siteCount": row.6,
                "passkeyCount": row.7, "activeSessions": row.8, "plan": row.9,
            })
        })
        .collect();
    Ok(Json(json!({"users": users, "total": total})))
}

async fn user_detail(
    State(state): State<AppState>,
    _: Admin,
    Path(user): Path<Uuid>,
) -> Result<Json<Value>, ApiError> {
    type UserRow = (
        Uuid,
        String,
        DateTime<Utc>,
        Option<DateTime<Utc>>,
        Option<DateTime<Utc>>,
        bool,
        Option<String>,
        Option<String>,
    );
    let row: UserRow = sqlx::query_as(
        "SELECT u.id,u.email,u.created_at,u.last_login_at,u.disabled_at,u.is_admin,
                coalesce(b.admin_plan,b.plan),b.subscription_status
         FROM users u LEFT JOIN account_billing b ON b.user_id=u.id WHERE u.id=$1",
    )
    .bind(user)
    .fetch_optional(&state.pool)
    .await?
    .ok_or(ApiError::NotFound)?;
    let sites: Vec<(Uuid, String, String, String, DateTime<Utc>)> = sqlx::query_as(
        "SELECT s.id,s.name,s.domain,m.role::text,s.created_at
         FROM site_memberships m JOIN sites s ON s.id=m.site_id
         WHERE m.user_id=$1 ORDER BY s.created_at",
    )
    .bind(user)
    .fetch_all(&state.pool)
    .await?;
    type SessionRow = (
        Uuid,
        String,
        Option<String>,
        DateTime<Utc>,
        DateTime<Utc>,
        DateTime<Utc>,
    );
    let sessions: Vec<SessionRow> = sqlx::query_as(
        "SELECT id,auth_method,user_agent,created_at,last_used_at,expires_at FROM user_sessions
         WHERE user_id=$1 AND revoked_at IS NULL AND expires_at>now() ORDER BY last_used_at DESC",
    )
    .bind(user)
    .fetch_all(&state.pool)
    .await?;
    type PasskeyRow = (Uuid, String, DateTime<Utc>, Option<DateTime<Utc>>);
    let passkeys: Vec<PasskeyRow> = sqlx::query_as(
        "SELECT id,name,created_at,last_used_at FROM user_passkeys WHERE user_id=$1 ORDER BY created_at",
    )
    .bind(user)
    .fetch_all(&state.pool)
    .await?;
    let api_tokens: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM api_tokens WHERE user_id=$1 AND revoked_at IS NULL AND expires_at>now()",
    )
    .bind(user)
    .fetch_one(&state.pool)
    .await?;
    Ok(Json(json!({
        "user": {
            "id": row.0, "email": row.1, "createdAt": row.2, "lastLoginAt": row.3,
            "disabledAt": row.4, "isAdmin": row.5, "plan": row.6, "subscriptionStatus": row.7,
        },
        "sites": sites.into_iter().map(|s| json!({
            "id": s.0, "name": s.1, "domain": s.2, "role": s.3, "createdAt": s.4,
        })).collect::<Vec<_>>(),
        "sessions": sessions.into_iter().map(|s| json!({
            "id": s.0, "authMethod": s.1, "userAgent": s.2, "createdAt": s.3,
            "lastUsedAt": s.4, "expiresAt": s.5,
        })).collect::<Vec<_>>(),
        "passkeys": passkeys.into_iter().map(|p| json!({
            "id": p.0, "name": p.1, "createdAt": p.2, "lastUsedAt": p.3,
        })).collect::<Vec<_>>(),
        "activeApiTokens": api_tokens,
    })))
}

/// Ends every way the account can act: browser sessions, API tokens, MCP connections, and
/// MCP authorization codes that were approved but not yet exchanged.
async fn revoke_access(tx: &mut Transaction<'_, Postgres>, user: Uuid) -> Result<(), ApiError> {
    // Account row first, the same order credential issuance uses, so a sign-in or OAuth
    // exchange racing this either finishes first (and is revoked here) or waits and fails.
    sqlx::query("SELECT 1 FROM users WHERE id=$1 FOR NO KEY UPDATE")
        .bind(user)
        .execute(&mut **tx)
        .await?;
    sqlx::query(
        "UPDATE user_sessions SET revoked_at=now() WHERE user_id=$1 AND revoked_at IS NULL",
    )
    .bind(user)
    .execute(&mut **tx)
    .await?;
    sqlx::query("UPDATE api_tokens SET revoked_at=now() WHERE user_id=$1 AND revoked_at IS NULL")
        .bind(user)
        .execute(&mut **tx)
        .await?;
    sqlx::query("DELETE FROM oauth_codes WHERE user_id=$1")
        .bind(user)
        .execute(&mut **tx)
        .await?;
    Ok(())
}

async fn disable_user(
    State(state): State<AppState>,
    admin: Admin,
    Path(user): Path<Uuid>,
) -> Result<StatusCode, ApiError> {
    let mut tx = state.pool.begin().await?;
    let email = actionable_target(&mut tx, &admin, user).await?;
    sqlx::query("UPDATE users SET disabled_at=now() WHERE id=$1 AND disabled_at IS NULL")
        .bind(user)
        .execute(&mut *tx)
        .await?;
    revoke_access(&mut tx, user).await?;
    audit(&mut *tx, &admin, "user.disable", user, &email, json!({})).await?;
    tx.commit().await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn enable_user(
    State(state): State<AppState>,
    admin: Admin,
    Path(user): Path<Uuid>,
) -> Result<StatusCode, ApiError> {
    let mut tx = state.pool.begin().await?;
    let email = actionable_target(&mut tx, &admin, user).await?;
    sqlx::query("UPDATE users SET disabled_at=NULL WHERE id=$1")
        .bind(user)
        .execute(&mut *tx)
        .await?;
    audit(&mut *tx, &admin, "user.enable", user, &email, json!({})).await?;
    tx.commit().await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn revoke_user_sessions(
    State(state): State<AppState>,
    admin: Admin,
    Path(user): Path<Uuid>,
) -> Result<StatusCode, ApiError> {
    let mut tx = state.pool.begin().await?;
    let email = actionable_target(&mut tx, &admin, user).await?;
    revoke_access(&mut tx, user).await?;
    audit(
        &mut *tx,
        &admin,
        "user.revoke_sessions",
        user,
        &email,
        json!({}),
    )
    .await?;
    tx.commit().await?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct DeleteUser {
    confirm_email: String,
}

/// Deletes an account, plus any site it alone owns (nobody else could reach it afterwards).
async fn delete_user(
    State(state): State<AppState>,
    admin: Admin,
    Path(user): Path<Uuid>,
    Json(input): Json<DeleteUser>,
) -> Result<StatusCode, ApiError> {
    let mut tx = state.pool.begin().await?;
    let email = actionable_target(&mut tx, &admin, user).await?;
    if input.confirm_email.trim().to_lowercase() != email {
        return Err(ApiError::BadRequest(
            "type the account's email address to confirm".into(),
        ));
    }
    let billing: Option<String> = sqlx::query_scalar(
        "SELECT subscription_status FROM account_billing WHERE user_id=$1
         AND subscription_status IN ('active','trialing','past_due','unpaid','incomplete')",
    )
    .bind(user)
    .fetch_optional(&mut *tx)
    .await?;
    if billing.is_some() {
        return Err(ApiError::BadRequest(
            "cancel this account's Stripe subscription before deleting it".into(),
        ));
    }
    let sites = sqlx::query(
        "DELETE FROM sites s WHERE EXISTS (
           SELECT 1 FROM site_memberships m WHERE m.site_id=s.id AND m.user_id=$1 AND m.role='owner')
         AND NOT EXISTS (
           SELECT 1 FROM site_memberships o WHERE o.site_id=s.id AND o.user_id<>$1 AND o.role='owner')",
    )
    .bind(user)
    .execute(&mut *tx)
    .await?
    .rows_affected();
    sqlx::query("DELETE FROM users WHERE id=$1")
        .bind(user)
        .execute(&mut *tx)
        .await?;
    audit(
        &mut *tx,
        &admin,
        "user.delete",
        user,
        &email,
        json!({"deletedSites": sites}),
    )
    .await?;
    tx.commit().await?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Deserialize)]
struct AuditQuery {
    limit: Option<i64>,
}

async fn audit_log(
    State(state): State<AppState>,
    _: Admin,
    Query(query): Query<AuditQuery>,
) -> Result<Json<Vec<Value>>, ApiError> {
    type Row = (
        i64,
        String,
        String,
        Option<Uuid>,
        Option<String>,
        Value,
        DateTime<Utc>,
    );
    let rows: Vec<Row> = sqlx::query_as(
        "SELECT id,actor_email,action,target_user_id,target_email,metadata,created_at
         FROM admin_audit_log ORDER BY created_at DESC, id DESC LIMIT $1",
    )
    .bind(query.limit.unwrap_or(100).clamp(1, 500))
    .fetch_all(&state.pool)
    .await?;
    Ok(Json(
        rows.into_iter()
            .map(|row| {
                json!({
                    "id": row.0, "actorEmail": row.1, "action": row.2, "targetUserId": row.3,
                    "targetEmail": row.4, "metadata": row.5, "createdAt": row.6,
                })
            })
            .collect(),
    ))
}
