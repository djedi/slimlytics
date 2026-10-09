#[path = "account_security.rs"]
mod account_security;
#[path = "admin_routes.rs"]
mod admin_routes;
#[path = "billing_routes.rs"]
mod billing_routes;
#[path = "mcp_oauth.rs"]
mod mcp_oauth;
#[path = "tracking_setup.rs"]
mod tracking_setup;
use crate::{
    agent::{
        required_scope, validate_idempotency_key, validate_scopes, ANALYTICS_READ, SITES_READ,
        SITES_WRITE,
    },
    auth::{
        generate_api_token, generate_refresh_token, hash_api_token, hash_password,
        issue_session_token, verify_password, verify_token,
    },
    briefs::{build_marketing_brief, deliver_report},
    enrichment::{location_from_headers, GeoIp},
    error::ApiError,
    identity::derive_ids,
    marketing::marketing_context,
    models::*,
    privacy::sanitize_url,
    reporting::{date_bounds, DateBounds},
    search_console::{
        authorization_url, decrypt_token, encrypt_token, oauth_state, preferred_property,
        state_hash, SearchConsoleConfig,
    },
    server_ingest::{validate_server_event, ServerEventBatch},
    site_icon::{fetch_favicon, normalize_color},
    traffic::{
        automation_for, client_metadata, collection_origin_allowed, origin_allowed, traffic_class,
        RateLimiter,
    },
    webhooks::{signing_secret, validate_webhook_url},
};
use axum::{
    extract::{ConnectInfo, FromRequestParts, Path, Query, State},
    http::{header, request::Parts, HeaderMap, StatusCode},
    response::{sse::Event, IntoResponse, Redirect, Response, Sse},
    routing::{delete, get, post},
    Json, Router,
};
use chrono::{DateTime, Duration as ChronoDuration, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sqlx::{PgPool, Postgres, Transaction};
use std::{
    collections::HashMap,
    convert::Infallible,
    net::{IpAddr, SocketAddr},
    sync::Arc,
    time::Duration,
};
use tokio::sync::broadcast;
use tokio_stream::{wrappers::BroadcastStream, StreamExt};
use tower_http::{
    request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer},
    trace::TraceLayer,
};
use url::Url;
use uuid::Uuid;

type CsvRow = (
    DateTime<Utc>,
    String,
    String,
    String,
    Option<String>,
    Option<String>,
);

#[derive(Clone, Debug, Serialize)]
pub struct StreamMessage {
    pub id: i64,
    pub site_id: Uuid,
    pub payload: Value,
}

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    public_url: Arc<String>,
    jwt_secret: Arc<String>,
    identity_secret: Arc<Vec<u8>>,
    limiter: RateLimiter,
    login_limiter: RateLimiter,
    /// Dynamic client registration, keyed by client IP so one source cannot block everyone.
    oauth_register_limiter: RateLimiter,
    /// Unauthenticated passkey sign-in challenges, keyed by client IP.
    passkey_limiter: RateLimiter,
    stream_tx: broadcast::Sender<StreamMessage>,
    internal_ips: Arc<Vec<IpAddr>>,
    access_token_ttl_seconds: i64,
    trust_proxy: bool,
    geoip: Option<Arc<GeoIp>>,
    search_console: Option<Arc<SearchConsoleConfig>>,
    /// Hosted-plan billing; `None` (the default) means no plans or limits.
    billing: Option<Arc<crate::billing::BillingConfig>>,
    http: reqwest::Client,
}
impl AppState {
    pub fn new(pool: PgPool, jwt_secret: String, identity_secret: Vec<u8>) -> Self {
        let (stream_tx, _) = broadcast::channel(1024);
        Self {
            pool,
            public_url: Arc::new("http://localhost:8080".into()),
            jwt_secret: Arc::new(jwt_secret),
            identity_secret: Arc::new(identity_secret),
            limiter: RateLimiter::new(120, Duration::from_secs(60)),
            login_limiter: RateLimiter::new(10, Duration::from_secs(60)),
            oauth_register_limiter: RateLimiter::new(20, Duration::from_secs(3600)),
            passkey_limiter: RateLimiter::new(30, Duration::from_secs(60)),
            stream_tx,
            internal_ips: Arc::new(Vec::new()),
            access_token_ttl_seconds: 3600,
            trust_proxy: false,
            geoip: None,
            search_console: None,
            billing: None,
            http: reqwest::Client::new(),
        }
    }
    pub fn with_public_url(mut self, value: &str) -> Result<Self, ApiError> {
        let url =
            Url::parse(value).map_err(|_| ApiError::BadRequest("invalid public URL".into()))?;
        if url.path() != "/"
            || url.query().is_some()
            || url.fragment().is_some()
            || !url.username().is_empty()
            || url.password().is_some()
            || !(url.scheme() == "https"
                || (url.scheme() == "http"
                    && matches!(url.host_str(), Some("localhost" | "127.0.0.1" | "[::1]"))))
        {
            return Err(ApiError::BadRequest(
                "public URL must be an HTTPS origin (HTTP loopback allowed)".into(),
            ));
        }
        self.public_url = Arc::new(url.as_str().trim_end_matches('/').into());
        Ok(self)
    }
    pub fn with_internal_ips(mut self, ips: Vec<IpAddr>) -> Self {
        self.internal_ips = Arc::new(ips);
        self
    }
    pub fn with_access_token_ttl(mut self, seconds: i64) -> Self {
        self.access_token_ttl_seconds = seconds;
        self
    }
    pub fn with_trust_proxy(mut self, trust_proxy: bool) -> Self {
        self.trust_proxy = trust_proxy;
        self
    }
    pub fn with_geoip(mut self, geoip: GeoIp) -> Self {
        self.geoip = Some(Arc::new(geoip));
        self
    }
    pub fn with_search_console(mut self, config: SearchConsoleConfig) -> Self {
        self.search_console = Some(Arc::new(config));
        self
    }
    pub fn with_billing(mut self, config: crate::billing::BillingConfig) -> Self {
        self.billing = Some(Arc::new(config));
        self
    }
}

#[derive(Clone, Copy)]
struct CurrentUser(Uuid);
impl FromRequestParts<AppState> for CurrentUser {
    type Rejection = ApiError;
    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        Ok(Self(
            Caller::from_request_parts(parts, state).await?.user_id,
        ))
    }
}

/// The signed-in user plus, for API tokens, the token's scopes (`None` for a dashboard session).
#[derive(Clone)]
struct Caller {
    user_id: Uuid,
    token_scopes: Option<Vec<String>>,
}

impl Caller {
    /// Whether this credential may receive a site's secret keys: dashboard sessions may, API
    /// tokens need `sites:write`. Membership role is checked separately.
    fn may_receive_site_keys(&self) -> bool {
        self.token_scopes
            .as_ref()
            .is_none_or(|scopes| scopes.iter().any(|scope| scope == SITES_WRITE))
    }
}

impl FromRequestParts<AppState> for Caller {
    type Rejection = ApiError;
    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let value = parts
            .headers
            .get(header::AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.strip_prefix("Bearer "))
            .ok_or(ApiError::Unauthorized)?;
        if value.starts_with("slyt_") {
            let token: Option<(Uuid, Vec<String>)> = sqlx::query_as(
                "UPDATE api_tokens SET last_used_at=now()
                 WHERE token_hash=$1 AND revoked_at IS NULL AND expires_at>now() AND oauth_resource IS NULL
                 RETURNING user_id,scopes",
            )
            .bind(hash_api_token(value))
            .fetch_optional(&state.pool)
            .await?;
            let (user, scopes) = token.ok_or(ApiError::Unauthorized)?;
            let required = required_scope(parts.method.as_str(), parts.uri.path());
            if !scopes.iter().any(|scope| scope == required) {
                return Err(ApiError::Forbidden);
            }
            return Ok(Self {
                user_id: user,
                token_scopes: Some(scopes),
            });
        }
        Ok(Self {
            user_id: authenticate_jwt(state, value).await?.user_id,
            token_scopes: None,
        })
    }
}

/// A verified browser/CLI sign-in (never an API token).
#[derive(Clone, Copy)]
struct SignedIn {
    user_id: Uuid,
    /// `None` only for access tokens issued before sessions existed.
    session_id: Option<Uuid>,
}

fn bearer(headers: &HeaderMap) -> Result<&str, ApiError> {
    headers
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .ok_or(ApiError::Unauthorized)
}

/// Verifies a session JWT and that its session is live and its account enabled, so signing
/// out, revoking a device, or disabling an account takes effect immediately. Tokens without a
/// session (issued before sessions existed) are refused: nothing could revoke them early.
async fn authenticate_jwt(state: &AppState, value: &str) -> Result<SignedIn, ApiError> {
    let claims = verify_token(value, &state.jwt_secret).map_err(|_| ApiError::Unauthorized)?;
    let session = claims.sid.ok_or(ApiError::Unauthorized)?;
    let active: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM user_sessions s JOIN users u ON u.id=s.user_id
         WHERE s.id=$1 AND s.user_id=$2 AND s.revoked_at IS NULL AND s.expires_at>now()
           AND u.disabled_at IS NULL)",
    )
    .bind(session)
    .bind(claims.sub)
    .fetch_one(&state.pool)
    .await?;
    if !active {
        return Err(ApiError::Unauthorized);
    }
    Ok(SignedIn {
        user_id: claims.sub,
        session_id: Some(session),
    })
}

impl FromRequestParts<AppState> for SignedIn {
    type Rejection = ApiError;
    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let value = bearer(&parts.headers)?;
        if value.starts_with("slyt_") {
            return Err(ApiError::Forbidden);
        }
        authenticate_jwt(state, value).await
    }
}

#[derive(Clone, Copy)]
struct SessionUser(Uuid);
impl FromRequestParts<AppState> for SessionUser {
    type Rejection = ApiError;
    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        Ok(Self(
            authenticate_jwt(state, bearer(&parts.headers)?)
                .await?
                .user_id,
        ))
    }
}

#[derive(Clone)]
struct AgentUser {
    user_id: Uuid,
    api_token_id: Option<Uuid>,
    scopes: Vec<String>,
}

impl AgentUser {
    fn require(&self, scope: &str) -> Result<(), ApiError> {
        self.scopes
            .iter()
            .any(|value| value == scope)
            .then_some(())
            .ok_or(ApiError::Forbidden)
    }
}

impl FromRequestParts<AppState> for AgentUser {
    type Rejection = ApiError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let value = parts
            .headers
            .get(header::AUTHORIZATION)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.strip_prefix("Bearer "))
            .ok_or(ApiError::Unauthorized)?;
        if value.starts_with("slyt_") {
            let row: Option<(Uuid, Uuid, Vec<String>)> = sqlx::query_as(
                "UPDATE api_tokens SET last_used_at=now()
                 WHERE token_hash=$1 AND revoked_at IS NULL AND expires_at>now() AND (oauth_resource IS NULL OR oauth_resource=$2)
                   AND (access_expires_at IS NULL OR access_expires_at>now())
                 RETURNING user_id,id,scopes",
            )
            .bind(hash_api_token(value))
            .bind(format!("{}/api/mcp",state.public_url))
            .fetch_optional(&state.pool)
            .await?;
            let (user_id, api_token_id, scopes) = row.ok_or(ApiError::Unauthorized)?;
            return Ok(Self {
                user_id,
                api_token_id: Some(api_token_id),
                scopes,
            });
        }
        let user_id = authenticate_jwt(state, value).await?.user_id;
        Ok(Self {
            user_id,
            api_token_id: None,
            scopes: vec![
                SITES_READ.into(),
                "sites:write".into(),
                ANALYTICS_READ.into(),
                "analytics:write".into(),
                "integrations:read".into(),
                "integrations:write".into(),
            ],
        })
    }
}

pub fn app(state: AppState) -> Router {
    Router::new()
        .merge(mcp_oauth::routes())
        .merge(billing_routes::routes())
        .merge(account_security::routes())
        .merge(admin_routes::routes())
        .route("/health", get(health))
        .route("/ready", get(ready))
        .route("/api/openapi.json", get(openapi_document))
        .route("/api/docs", get(scalar_docs))
        .route("/api/auth/register", post(register))
        .route("/api/auth/login", post(login))
        .route("/api/auth/me", get(me))
        .route("/api/auth/refresh", post(refresh_session))
        .route("/api/auth/logout", post(logout))
        .route(
            "/api/account/tokens",
            get(list_api_tokens).post(create_api_token),
        )
        .route(
            "/api/account/tokens/current",
            delete(revoke_current_api_token),
        )
        .route("/api/account/tokens/{token_id}", delete(revoke_api_token))
        .route("/api/sites", get(list_sites).post(create_site))
        .route("/api/sites/ensure", post(ensure_site))
        .route(
            "/api/sites/{site_id}",
            get(get_site).put(update_site).delete(delete_site),
        )
        .route(
            "/api/sites/{site_id}/icon",
            get(get_site_icon).put(update_site_icon),
        )
        .route("/api/sites/{site_id}/keys", post(site_keys))
        .route("/api/sites/{site_id}/rotate-key", post(rotate_key))
        .route(
            "/api/sites/{site_id}/rotate-server-key",
            post(rotate_server_key),
        )
        .route(
            "/api/sites/{site_id}/rotate-proxy-key",
            post(rotate_proxy_key),
        )
        .route(
            "/api/sites/{site_id}/anti-adblock",
            axum::routing::put(update_anti_adblock),
        )
        .route(
            "/api/sites/{site_id}/collection-health",
            get(get_collection_health),
        )
        .route("/api/sites/{site_id}/overview", get(overview))
        .route("/api/sites/{site_id}/reports/{dimension}", get(report))
        .route(
            "/api/sites/{site_id}/insights/journeys",
            get(common_journeys),
        )
        .route(
            "/api/sites/{site_id}/insights/attribution",
            get(attribution),
        )
        .route("/api/sites/{site_id}/insights/anomalies", get(anomalies))
        .route(
            "/api/sites/{site_id}/annotations",
            get(list_annotations).post(create_annotation),
        )
        .route(
            "/api/sites/{site_id}/annotations/{annotation_id}",
            delete(delete_annotation),
        )
        .route(
            "/api/sites/{site_id}/funnels",
            get(list_funnels).post(create_funnel),
        )
        .route(
            "/api/sites/{site_id}/funnels/{funnel_id}",
            delete(delete_funnel),
        )
        .route(
            "/api/sites/{site_id}/funnels/{funnel_id}/report",
            get(funnel_report),
        )
        .route(
            "/api/sites/{site_id}/report-subscriptions",
            get(list_report_subscriptions).post(create_report_subscription),
        )
        .route(
            "/api/sites/{site_id}/report-subscriptions/{subscription_id}",
            axum::routing::put(update_report_subscription).delete(delete_report_subscription),
        )
        .route(
            "/api/sites/{site_id}/report-subscriptions/{subscription_id}/deliver",
            post(deliver_report_subscription),
        )
        .route(
            "/api/sites/{site_id}/report-subscriptions/{subscription_id}/deliveries",
            get(list_report_deliveries),
        )
        .route(
            "/api/sites/{site_id}/integrations/search-console",
            get(search_console_status).delete(disconnect_search_console),
        )
        .route(
            "/api/sites/{site_id}/integrations/search-console/connect",
            post(connect_search_console),
        )
        .route(
            "/api/sites/{site_id}/integrations/search-console/sync",
            post(sync_search_console),
        )
        .route(
            "/api/sites/{site_id}/reports/search-console",
            get(search_console_report),
        )
        .route(
            "/api/integrations/search-console/callback",
            get(search_console_callback),
        )
        .route(
            "/api/mcp",
            post(mcp)
                .get(|| async { StatusCode::METHOD_NOT_ALLOWED })
                .layer(axum::middleware::map_response_with_state(
                    state.clone(),
                    mcp_challenge,
                )),
        )
        .route("/api/sites/{site_id}/visitors", get(list_visitors))
        .route(
            "/api/sites/{site_id}/visitors/{visitor_id}",
            get(visitor_timeline),
        )
        .route("/api/sites/{site_id}/events", get(custom_events))
        .route(
            "/api/sites/{site_id}/goals",
            get(list_goals).post(create_goal),
        )
        .route("/api/sites/{site_id}/goals/{goal_id}", delete(delete_goal))
        .route("/api/sites/{site_id}/export.csv", get(export_csv))
        .route("/api/sites/{site_id}/stream", get(stream))
        .route(
            "/api/collect/{write_key}",
            get(collect_test).post(collect).options(collect_options),
        )
        .route(
            "/api/e/{write_key}",
            get(collect_test).post(collect).options(collect_options),
        )
        .route("/api/ingest", post(server_collect))
        .layer(PropagateRequestIdLayer::x_request_id())
        .layer(SetRequestIdLayer::x_request_id(MakeRequestUuid))
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}
async fn health() -> impl IntoResponse {
    (StatusCode::OK, Json(json!({"status":"ok"})))
}

const OPENAPI_JSON: &str = include_str!("../../docs/openapi.json");
async fn openapi_document() -> impl IntoResponse {
    (
        [(header::CONTENT_TYPE, "application/vnd.oai.openapi+json")],
        OPENAPI_JSON,
    )
}

async fn scalar_docs() -> Redirect {
    Redirect::temporary("/docs/api")
}

async fn ready(State(state): State<AppState>) -> Result<impl IntoResponse, ApiError> {
    sqlx::query("SELECT 1").execute(&state.pool).await?;
    Ok(Json(json!({"status":"ready"})))
}

/// Idle sign-ins stay valid this long; every refresh extends them.
const SESSION_IDLE_DAYS: i64 = 30;
/// Even an active sign-in must re-enter credentials after this long.
const SESSION_MAX_DAYS: i64 = 365;
/// A passkey check counts as MFA for this long (admin portal, passkey management).
const MFA_WINDOW_HOURS: i64 = 12;
/// A rotated refresh token replayed within this window is a benign race between tabs.
const REFRESH_REUSE_GRACE_SECONDS: i64 = 60;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SessionTokens {
    token: String,
    refresh_token: String,
    expires_in: i64,
}

fn user_agent(headers: &HeaderMap) -> Option<String> {
    headers
        .get(header::USER_AGENT)
        .and_then(|value| value.to_str().ok())
        .map(|value| value.chars().take(255).collect())
}

/// Starts a revocable sign-in session and returns its access and refresh tokens.
/// Locks the account row and confirms it is enabled. Taken before issuing any credential so
/// issuance serializes with disabling (which locks the same row before revoking access):
/// either the disable waits and then revokes the new credential, or issuance sees it.
/// `exclusive` is for transactions that go on to update the account row.
async fn lock_enabled_user(
    tx: &mut Transaction<'_, Postgres>,
    user: Uuid,
    exclusive: bool,
) -> Result<(), ApiError> {
    let sql = if exclusive {
        "SELECT disabled_at IS NULL FROM users WHERE id=$1 FOR NO KEY UPDATE"
    } else {
        "SELECT disabled_at IS NULL FROM users WHERE id=$1 FOR SHARE"
    };
    let enabled: Option<bool> = sqlx::query_scalar(sql)
        .bind(user)
        .fetch_optional(&mut **tx)
        .await?;
    if enabled == Some(true) {
        Ok(())
    } else {
        Err(ApiError::Unauthorized)
    }
}

async fn create_session(
    state: &AppState,
    user: Uuid,
    method: &str,
    headers: &HeaderMap,
) -> Result<SessionTokens, ApiError> {
    let mut tx = state.pool.begin().await?;
    let tokens = create_session_in(&mut tx, state, user, method, headers).await?;
    tx.commit().await?;
    Ok(tokens)
}

/// Starts a session inside `tx`, so callers can make it atomic with their own checks.
async fn create_session_in(
    tx: &mut Transaction<'_, Postgres>,
    state: &AppState,
    user: Uuid,
    method: &str,
    headers: &HeaderMap,
) -> Result<SessionTokens, ApiError> {
    lock_enabled_user(tx, user, true).await?;
    let refresh_token = generate_refresh_token();
    let session: Uuid = sqlx::query_scalar(
        "INSERT INTO user_sessions(user_id,refresh_token_hash,auth_method,mfa_verified_at,user_agent,expires_at)
         VALUES($1,$2,$3,CASE WHEN $3='passkey' THEN now() END,$4,now()+make_interval(days=>$5))
         RETURNING id",
    )
    .bind(user)
    .bind(hash_api_token(&refresh_token))
    .bind(method)
    .bind(user_agent(headers))
    .bind(SESSION_IDLE_DAYS as i32)
    .fetch_one(&mut **tx)
    .await?;
    sqlx::query("UPDATE users SET last_login_at=now() WHERE id=$1")
        .bind(user)
        .execute(&mut **tx)
        .await?;
    session_tokens(state, user, session, refresh_token)
}

fn session_tokens(
    state: &AppState,
    user: Uuid,
    session: Uuid,
    refresh_token: String,
) -> Result<SessionTokens, ApiError> {
    Ok(SessionTokens {
        token: issue_session_token(
            user,
            session,
            &state.jwt_secret,
            state.access_token_ttl_seconds,
        )
        .map_err(|_| ApiError::Internal)?,
        refresh_token,
        expires_in: state.access_token_ttl_seconds,
    })
}

async fn register(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(input): Json<Credentials>,
) -> Result<impl IntoResponse, ApiError> {
    validate_credentials(&input)?;
    let hash = hash_password(&input.password).map_err(|_| ApiError::Internal)?;
    let id: Uuid = sqlx::query_scalar(
        "INSERT INTO users(email,password_hash) VALUES(lower($1),$2) RETURNING id",
    )
    .bind(input.email.trim())
    .bind(hash)
    .fetch_one(&state.pool)
    .await
    .map_err(map_conflict)?;
    let tokens = create_session(&state, id, "password", &headers).await?;
    Ok((StatusCode::CREATED, Json(tokens)))
}

/// Checks an email and password. Unknown, wrong, and disabled accounts look identical.
async fn authenticate_password(
    state: &AppState,
    email: &str,
    password: &str,
) -> Result<Uuid, ApiError> {
    let email = email.trim().to_lowercase();
    if password.len() > 1024 || !state.login_limiter.check(&email) {
        return Err(ApiError::RateLimited);
    }
    let row: Option<(Uuid, String, bool)> = sqlx::query_as(
        "SELECT id,password_hash,disabled_at IS NOT NULL FROM users WHERE email=lower($1)",
    )
    .bind(&email)
    .fetch_optional(&state.pool)
    .await?;
    let Some((id, hash, disabled)) = row else {
        // Spend comparable time so unknown emails are not distinguishable by latency.
        let _ = hash_password(password);
        return Err(ApiError::Unauthorized);
    };
    if !verify_password(password, &hash).map_err(|_| ApiError::Unauthorized)? || disabled {
        return Err(ApiError::Unauthorized);
    }
    Ok(id)
}

async fn login(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(input): Json<Credentials>,
) -> Result<Json<SessionTokens>, ApiError> {
    let id = authenticate_password(&state, &input.email, &input.password).await?;
    Ok(Json(
        create_session(&state, id, "password", &headers).await?,
    ))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RefreshInput {
    refresh_token: String,
}

/// Exchanges a refresh token for a new access token and a rotated refresh token.
async fn refresh_session(
    State(state): State<AppState>,
    Json(input): Json<RefreshInput>,
) -> Result<Json<SessionTokens>, ApiError> {
    if input.refresh_token.len() > 128 {
        return Err(ApiError::Unauthorized);
    }
    let presented = hash_api_token(&input.refresh_token);
    let mut tx = state.pool.begin().await?;
    // Lock order matches disabling an account: the user row, then its sessions.
    let owner: Option<Uuid> =
        sqlx::query_scalar("SELECT user_id FROM user_sessions WHERE refresh_token_hash=$1")
            .bind(&presented)
            .fetch_optional(&mut *tx)
            .await?;
    if let Some(owner) = owner {
        lock_enabled_user(&mut tx, owner, false).await?;
    }
    let row: Option<(Uuid, Uuid)> = sqlx::query_as(
        "SELECT s.id,s.user_id FROM user_sessions s JOIN users u ON u.id=s.user_id
         WHERE s.refresh_token_hash=$1 AND s.revoked_at IS NULL AND s.expires_at>now()
           AND u.disabled_at IS NULL
         FOR UPDATE OF s",
    )
    .bind(&presented)
    .fetch_optional(&mut *tx)
    .await?;
    let Some((session, user)) = row else {
        // Any rotated-out token replayed after the grace window means it leaked: end the
        // session. Inside the window it is most likely two tabs racing, so only refuse it.
        sqlx::query(
            "UPDATE user_sessions s SET revoked_at=now()
             FROM user_session_used_tokens t
             WHERE t.token_hash=$1 AND t.session_id=s.id AND s.revoked_at IS NULL
               AND t.used_at < now()-make_interval(secs=>$2)",
        )
        .bind(&presented)
        .bind(REFRESH_REUSE_GRACE_SECONDS as f64)
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        return Err(ApiError::Unauthorized);
    };
    sqlx::query("INSERT INTO user_session_used_tokens(token_hash,session_id) VALUES($1,$2)")
        .bind(&presented)
        .bind(session)
        .execute(&mut *tx)
        .await?;
    let refresh_token = generate_refresh_token();
    sqlx::query(
        "UPDATE user_sessions SET refresh_token_hash=$2, last_used_at=now(),
           expires_at=LEAST(now()+make_interval(days=>$3), created_at+make_interval(days=>$4))
         WHERE id=$1",
    )
    .bind(session)
    .bind(hash_api_token(&refresh_token))
    .bind(SESSION_IDLE_DAYS as i32)
    .bind(SESSION_MAX_DAYS as i32)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(Json(session_tokens(&state, user, session, refresh_token)?))
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct LogoutInput {
    refresh_token: Option<String>,
}

/// Ends a session identified by its access token or, when that has already expired, by its
/// current refresh token. Signing out always succeeds so a stale client can clear itself.
async fn logout(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Option<Json<LogoutInput>>,
) -> Result<StatusCode, ApiError> {
    let Json(input) = body.unwrap_or_default();
    if let Some(session) = match bearer(&headers) {
        Ok(value) if !value.starts_with("slyt_") => authenticate_jwt(&state, value)
            .await
            .ok()
            .and_then(|signed_in| signed_in.session_id),
        _ => None,
    } {
        sqlx::query("UPDATE user_sessions SET revoked_at=now() WHERE id=$1 AND revoked_at IS NULL")
            .bind(session)
            .execute(&state.pool)
            .await?;
    }
    if let Some(refresh_token) = input.refresh_token.filter(|value| value.len() <= 128) {
        sqlx::query(
            "UPDATE user_sessions SET revoked_at=now() WHERE refresh_token_hash=$1 AND revoked_at IS NULL",
        )
        .bind(hash_api_token(&refresh_token))
        .execute(&state.pool)
        .await?;
    }
    Ok(StatusCode::NO_CONTENT)
}

/// Whether this sign-in session proved a passkey recently enough to count as MFA.
async fn session_mfa_verified(pool: &PgPool, session: Option<Uuid>) -> Result<bool, ApiError> {
    let Some(session) = session else {
        return Ok(false);
    };
    Ok(sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM user_sessions
         WHERE id=$1 AND mfa_verified_at > now()-make_interval(hours=>$2))",
    )
    .bind(session)
    .bind(MFA_WINDOW_HOURS as i32)
    .fetch_one(pool)
    .await?)
}

async fn me(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    headers: HeaderMap,
) -> Result<Json<Value>, ApiError> {
    // API tokens have no session; JWTs carry one.
    let session = bearer(&headers)
        .ok()
        .filter(|value| !value.starts_with("slyt_"))
        .and_then(|value| verify_token(value, &state.jwt_secret).ok())
        .and_then(|claims| claims.sid);
    let row: (Uuid, String, DateTime<Utc>, bool, i64) = sqlx::query_as(
        "SELECT id,email,created_at,is_admin,(SELECT count(*) FROM user_passkeys WHERE user_id=users.id)
         FROM users WHERE id=$1",
    )
    .bind(user)
    .fetch_optional(&state.pool)
    .await?
    .ok_or(ApiError::Unauthorized)?;
    let mfa = session_mfa_verified(&state.pool, session).await?;
    Ok(Json(json!({
        "id": row.0, "email": row.1, "createdAt": row.2,
        "isAdmin": row.3, "passkeyCount": row.4, "mfaVerified": mfa,
    })))
}

async fn create_api_token(
    State(state): State<AppState>,
    SessionUser(user): SessionUser,
    Json(input): Json<ApiTokenInput>,
) -> Result<impl IntoResponse, ApiError> {
    let name = input.name.trim();
    if name.is_empty() || name.chars().count() > 100 {
        return Err(ApiError::BadRequest(
            "token name must contain between 1 and 100 characters".into(),
        ));
    }
    let days = input.expires_in_days.unwrap_or(365);
    if !(1..=3650).contains(&days) {
        return Err(ApiError::BadRequest(
            "expiresInDays must be between 1 and 3650".into(),
        ));
    }
    let scopes =
        validate_scopes(&input.scopes).map_err(|message| ApiError::BadRequest(message.into()))?;
    let token = generate_api_token();
    let token_prefix: String = token.chars().take(12).collect();
    let expires_at = Utc::now() + ChronoDuration::days(days);
    let row: (Uuid, DateTime<Utc>) = sqlx::query_as(
        "INSERT INTO api_tokens(user_id,name,token_hash,token_prefix,expires_at,scopes)
         VALUES($1,$2,$3,$4,$5,$6) RETURNING id,created_at",
    )
    .bind(user)
    .bind(name)
    .bind(hash_api_token(&token))
    .bind(&token_prefix)
    .bind(expires_at)
    .bind(&scopes)
    .fetch_one(&state.pool)
    .await?;
    Ok((
        StatusCode::CREATED,
        Json(ApiTokenCreated {
            id: row.0,
            name: name.to_owned(),
            token_prefix,
            scopes,
            token,
            expires_at,
            created_at: row.1,
        }),
    ))
}

async fn list_api_tokens(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
) -> Result<Json<Vec<ApiTokenSummary>>, ApiError> {
    let tokens = sqlx::query_as(
        "SELECT id,name,token_prefix,scopes,last_used_at,expires_at,created_at
         FROM api_tokens WHERE user_id=$1 AND revoked_at IS NULL AND expires_at>now()
         ORDER BY created_at DESC",
    )
    .bind(user)
    .fetch_all(&state.pool)
    .await?;
    Ok(Json(tokens))
}

async fn revoke_api_token(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(token): Path<Uuid>,
) -> Result<StatusCode, ApiError> {
    let result = sqlx::query(
        "UPDATE api_tokens SET revoked_at=now() WHERE id=$1 AND user_id=$2 AND revoked_at IS NULL",
    )
    .bind(token)
    .bind(user)
    .execute(&state.pool)
    .await?;
    if result.rows_affected() == 0 {
        return Err(ApiError::NotFound);
    }
    Ok(StatusCode::NO_CONTENT)
}

async fn revoke_current_api_token(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    headers: HeaderMap,
) -> Result<StatusCode, ApiError> {
    let token = headers
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .filter(|value| value.starts_with("slyt_"))
        .ok_or(ApiError::Unauthorized)?;
    let result = sqlx::query(
        "UPDATE api_tokens SET revoked_at=now() WHERE user_id=$1 AND token_hash=$2 AND revoked_at IS NULL",
    )
    .bind(user)
    .bind(hash_api_token(token))
    .execute(&state.pool)
    .await?;
    if result.rows_affected() == 1 {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(ApiError::Unauthorized)
    }
}

fn validate_credentials(value: &Credentials) -> Result<(), ApiError> {
    if !value.email.contains('@')
        || value.password.chars().count() < 12
        || value.password.len() > 1024
        || value.email.len() > 320
    {
        return Err(ApiError::BadRequest(
            "valid email and password of at least 12 characters required".into(),
        ));
    }
    Ok(())
}
fn map_conflict(error: sqlx::Error) -> ApiError {
    if matches!(&error, sqlx::Error::Database(db) if db.is_unique_violation()) {
        ApiError::BadRequest("resource already exists".into())
    } else {
        ApiError::Database(error)
    }
}

async fn require_site(pool: &PgPool, user: Uuid, site: Uuid, write: bool) -> Result<(), ApiError> {
    let role: Option<String> = sqlx::query_scalar(
        "SELECT role::text FROM site_memberships WHERE site_id=$1 AND user_id=$2",
    )
    .bind(site)
    .bind(user)
    .fetch_optional(pool)
    .await?;
    match role.as_deref() {
        None => Err(ApiError::NotFound),
        Some("viewer") if write => Err(ApiError::Forbidden),
        Some(_) => Ok(()),
    }
}
async fn list_sites(
    State(state): State<AppState>,
    caller: Caller,
) -> Result<Json<Vec<Value>>, ApiError> {
    let user = caller.user_id;
    let sites: Vec<Site> = sqlx::query_as("SELECT s.id,s.name,s.domain,s.timezone,s.allowed_origins,s.retention_days,s.write_key,s.server_write_key,s.proxy_key,s.anti_adblock_server,s.anti_adblock_js_path,s.anti_adblock_beacon_path,s.icon_mode,s.icon_background,s.icon_background_end,s.icon_foreground,s.icon_updated_at,s.created_at FROM sites s JOIN site_memberships m ON m.site_id=s.id WHERE m.user_id=$1 ORDER BY s.created_at").bind(user).fetch_all(&state.pool).await?;
    let roles: HashMap<Uuid, String> = sqlx::query_as::<_, (Uuid, String)>(
        "SELECT site_id,role::text FROM site_memberships WHERE user_id=$1",
    )
    .bind(user)
    .fetch_all(&state.pool)
    .await?
    .into_iter()
    .collect();
    Ok(Json(
        sites
            .into_iter()
            .map(|site| {
                let admin = roles
                    .get(&site.id)
                    .is_some_and(|role| role_manages_site(role));
                site_response(site, admin && caller.may_receive_site_keys())
            })
            .collect(),
    ))
}

/// Owners and admins manage a site's keys; viewers only read its analytics.
fn role_manages_site(role: &str) -> bool {
    matches!(role, "owner" | "admin")
}

/// Last four characters of a key, so a key can be recognized without being shown.
fn key_hint(key: Uuid) -> String {
    let key = key.to_string();
    format!("…{}", &key[key.len() - 4..])
}

/// A site as returned by the read endpoints. `serverWriteKey` and `proxyKey` are included only
/// when `include_keys` (an owner or admin, and for API tokens the `sites:write` scope); the
/// hints are always present.
fn site_response(site: Site, include_keys: bool) -> Value {
    let server_hint = key_hint(site.server_write_key);
    let proxy_hint = key_hint(site.proxy_key);
    let mut value = serde_json::to_value(site).unwrap_or_else(|_| json!({}));
    if let Some(object) = value.as_object_mut() {
        if !include_keys {
            object.remove("serverWriteKey");
            object.remove("proxyKey");
        }
        object.insert("serverWriteKeyHint".into(), json!(server_hint));
        object.insert("proxyKeyHint".into(), json!(proxy_hint));
        object.insert("canManageKeys".into(), json!(include_keys));
    }
    value
}

async fn create_site(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Json(mut input): Json<SiteInput>,
) -> Result<impl IntoResponse, ApiError> {
    validate_site(&input)?;
    input.domain = canonical_domain(&input.domain)?;
    let mut tx = state.pool.begin().await?;
    billing_routes::ensure_site_allowance(&state, &mut tx, user).await?;
    let site: Site = sqlx::query_as("INSERT INTO sites(name,domain,timezone,allowed_origins,retention_days) VALUES($1,$2,$3,$4,$5) RETURNING id,name,domain,timezone,allowed_origins,retention_days,write_key,server_write_key,proxy_key,anti_adblock_server,anti_adblock_js_path,anti_adblock_beacon_path,icon_mode,icon_background,icon_background_end,icon_foreground,icon_updated_at,created_at")
        .bind(input.name)
        .bind(input.domain)
        .bind(input.timezone)
        .bind(input.allowed_origins)
        .bind(input.retention_days)
        .fetch_one(&mut *tx)
        .await
        .map_err(map_conflict)?;
    sqlx::query("INSERT INTO site_memberships(site_id,user_id,role) VALUES($1,$2,'owner')")
        .bind(site.id)
        .bind(user)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok((StatusCode::CREATED, Json(site)))
}

async fn ensure_site(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Json(mut input): Json<SiteInput>,
) -> Result<(StatusCode, Json<EnsureSiteResponse>), ApiError> {
    validate_site(&input)?;
    input.domain = canonical_domain(&input.domain)?;
    let mut tx = state.pool.begin().await?;
    if state.billing.is_some() {
        billing_routes::lock_account_sites(&mut tx, user).await?;
    }
    let inserted: Option<Site> = sqlx::query_as(
        "INSERT INTO sites(name,domain,timezone,allowed_origins,retention_days) VALUES($1,$2,$3,$4,$5) ON CONFLICT (lower(domain)) DO NOTHING RETURNING id,name,domain,timezone,allowed_origins,retention_days,write_key,server_write_key,proxy_key,anti_adblock_server,anti_adblock_js_path,anti_adblock_beacon_path,icon_mode,icon_background,icon_background_end,icon_foreground,icon_updated_at,created_at",
    )
    .bind(&input.name)
    .bind(&input.domain)
    .bind(&input.timezone)
    .bind(&input.allowed_origins)
    .bind(input.retention_days)
    .fetch_optional(&mut *tx)
    .await?;
    let (created, site) = if let Some(site) = inserted {
        // Reusing an existing site is always allowed; only a new one counts against the plan.
        // Returning here drops the transaction, rolling back the inserted site.
        billing_routes::ensure_site_allowance(&state, &mut tx, user).await?;
        sqlx::query("INSERT INTO site_memberships(site_id,user_id,role) VALUES($1,$2,'owner')")
            .bind(site.id)
            .bind(user)
            .execute(&mut *tx)
            .await?;
        (true, site)
    } else {
        let site = sqlx::query_as(
            "SELECT s.id,s.name,s.domain,s.timezone,s.allowed_origins,s.retention_days,s.write_key,s.server_write_key,s.proxy_key,s.anti_adblock_server,s.anti_adblock_js_path,s.anti_adblock_beacon_path,s.icon_mode,s.icon_background,s.icon_background_end,s.icon_foreground,s.icon_updated_at,s.created_at FROM sites s JOIN site_memberships m ON m.site_id=s.id WHERE m.user_id=$1 AND lower(s.domain)=lower($2)",
        )
        .bind(user)
        .bind(&input.domain)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| ApiError::BadRequest("domain is already managed by another account".into()))?;
        (false, site)
    };
    tx.commit().await?;
    let status = if created {
        StatusCode::CREATED
    } else {
        StatusCode::OK
    };
    Ok((status, Json(EnsureSiteResponse { created, site })))
}

fn validate_site(input: &SiteInput) -> Result<(), ApiError> {
    if input.name.trim().is_empty()
        || input.domain.trim().is_empty()
        || !(1..=3650).contains(&input.retention_days)
    {
        return Err(ApiError::BadRequest("invalid site".into()));
    }
    canonical_domain(&input.domain)?;
    input
        .timezone
        .parse::<chrono_tz::Tz>()
        .map_err(|_| ApiError::BadRequest("invalid timezone".into()))?;
    for origin in &input.allowed_origins {
        let parsed =
            Url::parse(origin).map_err(|_| ApiError::BadRequest("invalid origin".into()))?;
        if !matches!(parsed.scheme(), "http" | "https") || parsed.path() != "/" {
            return Err(ApiError::BadRequest(
                "origins must contain scheme and host only".into(),
            ));
        }
    }
    Ok(())
}

fn canonical_domain(value: &str) -> Result<String, ApiError> {
    let value = value.trim().trim_end_matches('.');
    if value.contains("://") || value.chars().any(char::is_whitespace) {
        return Err(ApiError::BadRequest("invalid domain".into()));
    }
    let parsed = Url::parse(&format!("https://{value}"))
        .map_err(|_| ApiError::BadRequest("invalid domain".into()))?;
    let host = parsed
        .host_str()
        .filter(|_| parsed.port().is_none() && parsed.path() == "/")
        .filter(|host| host.contains('.') || *host == "localhost")
        .ok_or_else(|| ApiError::BadRequest("invalid domain".into()))?;
    Ok(host.to_ascii_lowercase())
}

async fn get_site(
    State(state): State<AppState>,
    caller: Caller,
    Path(site): Path<Uuid>,
) -> Result<Json<Value>, ApiError> {
    let role = site_role(&state.pool, caller.user_id, site).await?;
    let include_keys = role_manages_site(&role) && caller.may_receive_site_keys();
    Ok(Json(site_response(
        fetch_site(&state.pool, site).await?,
        include_keys,
    )))
}

async fn site_role(pool: &PgPool, user: Uuid, site: Uuid) -> Result<String, ApiError> {
    sqlx::query_scalar("SELECT role::text FROM site_memberships WHERE site_id=$1 AND user_id=$2")
        .bind(site)
        .bind(user)
        .fetch_optional(pool)
        .await?
        .ok_or(ApiError::NotFound)
}

/// A site's keys for its owners and admins (API tokens need `sites:write`, as for any POST).
/// The read endpoints omit the secret ones for everyone else.
async fn site_keys(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(site): Path<Uuid>,
) -> Result<Json<Value>, ApiError> {
    require_site(&state.pool, user, site, true).await?;
    let (write_key, server_write_key, proxy_key): (Uuid, Uuid, Uuid) =
        sqlx::query_as("SELECT write_key,server_write_key,proxy_key FROM sites WHERE id=$1")
            .bind(site)
            .fetch_one(&state.pool)
            .await?;
    Ok(Json(json!({
        "writeKey": write_key,
        "serverWriteKey": server_write_key,
        "proxyKey": proxy_key,
    })))
}
async fn fetch_site(pool: &PgPool, id: Uuid) -> Result<Site, ApiError> {
    sqlx::query_as("SELECT id,name,domain,timezone,allowed_origins,retention_days,write_key,server_write_key,proxy_key,anti_adblock_server,anti_adblock_js_path,anti_adblock_beacon_path,icon_mode,icon_background,icon_background_end,icon_foreground,icon_updated_at,created_at FROM sites WHERE id=$1").bind(id).fetch_optional(pool).await?.ok_or(ApiError::NotFound)
}
async fn update_site(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(site): Path<Uuid>,
    Json(mut i): Json<SiteInput>,
) -> Result<Json<Site>, ApiError> {
    require_site(&state.pool, user, site, true).await?;
    validate_site(&i)?;
    i.domain = canonical_domain(&i.domain)?;
    sqlx::query("UPDATE sites SET name=$2,domain=$3,timezone=$4,allowed_origins=$5,retention_days=$6,updated_at=now() WHERE id=$1")
        .bind(site)
        .bind(i.name)
        .bind(i.domain)
        .bind(i.timezone)
        .bind(i.allowed_origins)
        .bind(i.retention_days)
        .execute(&state.pool)
        .await
        .map_err(map_conflict)?;
    Ok(Json(fetch_site(&state.pool, site).await?))
}

async fn update_site_icon(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(site): Path<Uuid>,
    Json(input): Json<SiteIconInput>,
) -> Result<Json<Site>, ApiError> {
    require_site(&state.pool, user, site, true).await?;
    if !matches!(input.mode.as_str(), "initials" | "favicon") {
        return Err(ApiError::BadRequest(
            "mode must be initials or favicon".into(),
        ));
    }
    let color =
        |value| normalize_color(value).map_err(|message| ApiError::BadRequest(message.into()));
    let background = color(input.background)?;
    let background_end = color(input.background_end)?;
    let foreground = color(input.foreground)?;
    // Choosing (or re-saving) favicon mode fetches a fresh copy; if no complete image can be
    // fetched the save fails and the previous icon and settings stay as they were.
    // Fetch over the network first, then store the icon, its cache version, and the settings
    // together so a failure never leaves a new image behind an old version (or half a save).
    let icon = if input.mode == "favicon" {
        let domain = fetch_site(&state.pool, site).await?.domain;
        Some(fetch_favicon(&domain).await.map_err(|error| {
            ApiError::BadRequest(format!("couldn't load a favicon for {domain}: {error}"))
        })?)
    } else {
        None
    };
    let mut tx = state.pool.begin().await?;
    if let Some(icon) = icon {
        sqlx::query(
            "INSERT INTO site_icons(site_id,content_type,body,source_url) VALUES($1,$2,$3,$4)
             ON CONFLICT (site_id) DO UPDATE SET content_type=$2,body=$3,source_url=$4,fetched_at=now()",
        )
        .bind(site)
        .bind(icon.content_type)
        .bind(icon.body)
        .bind(icon.source_url)
        .execute(&mut *tx)
        .await?;
        sqlx::query("UPDATE sites SET icon_updated_at=now() WHERE id=$1")
            .bind(site)
            .execute(&mut *tx)
            .await?;
    }
    sqlx::query(
        "UPDATE sites SET icon_mode=$2,icon_background=$3,icon_background_end=$4,icon_foreground=$5,updated_at=now() WHERE id=$1",
    )
    .bind(site)
    .bind(input.mode)
    .bind(background)
    .bind(background_end)
    .bind(foreground)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(Json(fetch_site(&state.pool, site).await?))
}

/// The stored favicon. Public like the favicon itself, so a plain <img> can load it; SVGs are
/// sandboxed so they can never run script on this origin.
async fn get_site_icon(
    State(state): State<AppState>,
    Path(site): Path<Uuid>,
) -> Result<Response, ApiError> {
    let (content_type, body): (String, Vec<u8>) =
        sqlx::query_as("SELECT content_type,body FROM site_icons WHERE site_id=$1")
            .bind(site)
            .fetch_optional(&state.pool)
            .await?
            .ok_or(ApiError::NotFound)?;
    Ok((
        [
            (header::CONTENT_TYPE, content_type),
            (header::CACHE_CONTROL, "public, max-age=86400".into()),
            (header::X_CONTENT_TYPE_OPTIONS, "nosniff".into()),
            (
                header::CONTENT_SECURITY_POLICY,
                "default-src 'none'; style-src 'unsafe-inline'; sandbox".into(),
            ),
        ],
        body,
    )
        .into_response())
}

async fn update_anti_adblock(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(site): Path<Uuid>,
    Json(input): Json<AntiAdblockInput>,
) -> Result<Json<Site>, ApiError> {
    require_site(&state.pool, user, site, true).await?;
    input
        .validate()
        .map_err(|message| ApiError::BadRequest(message.into()))?;
    sqlx::query(
        "UPDATE sites SET anti_adblock_server=$2,anti_adblock_js_path=$3,anti_adblock_beacon_path=$4,updated_at=now() WHERE id=$1",
    )
    .bind(site)
    .bind(input.server_type)
    .bind(input.js_path)
    .bind(input.beacon_path)
    .execute(&state.pool)
    .await?;
    Ok(Json(fetch_site(&state.pool, site).await?))
}

async fn get_collection_health(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(site): Path<Uuid>,
) -> Result<Json<CollectionHealth>, ApiError> {
    require_site(&state.pool, user, site, false).await?;
    Ok(Json(
        sqlx::query_as(
            "SELECT COALESCE(accepted_total,0)::bigint accepted_total, \
             COALESCE(rejected_total,0)::bigint rejected_total, \
             last_accepted_at,last_rejected_at,last_rejection_code,last_tracker_version \
             FROM sites LEFT JOIN collection_health ON collection_health.site_id=sites.id \
             WHERE sites.id=$1",
        )
        .bind(site)
        .fetch_one(&state.pool)
        .await?,
    ))
}

async fn delete_site(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(site): Path<Uuid>,
) -> Result<StatusCode, ApiError> {
    require_site(&state.pool, user, site, true).await?;
    sqlx::query("DELETE FROM sites WHERE id=$1")
        .bind(site)
        .execute(&state.pool)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}
async fn rotate_key(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(site): Path<Uuid>,
) -> Result<Json<WriteKeyResponse>, ApiError> {
    require_site(&state.pool, user, site, true).await?;
    let key:Uuid=sqlx::query_scalar("UPDATE sites SET write_key=gen_random_uuid(),updated_at=now() WHERE id=$1 RETURNING write_key").bind(site).fetch_one(&state.pool).await?;
    Ok(Json(WriteKeyResponse { write_key: key }))
}

async fn rotate_server_key(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(site): Path<Uuid>,
) -> Result<Json<Value>, ApiError> {
    require_site(&state.pool, user, site, true).await?;
    let key: Uuid = sqlx::query_scalar(
        "UPDATE sites SET server_write_key=gen_random_uuid(),updated_at=now() \
         WHERE id=$1 RETURNING server_write_key",
    )
    .bind(site)
    .fetch_one(&state.pool)
    .await?;
    Ok(Json(json!({"serverWriteKey":key})))
}

/// Replace a site's proxy key, e.g. after it leaked. The old key stops vouching for forwarded
/// visitor IPs immediately; the site's proxy configuration must be updated with the new one.
async fn rotate_proxy_key(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(site): Path<Uuid>,
) -> Result<Json<Value>, ApiError> {
    require_site(&state.pool, user, site, true).await?;
    let key: Uuid = sqlx::query_scalar(
        "UPDATE sites SET proxy_key=gen_random_uuid(),updated_at=now() \
         WHERE id=$1 RETURNING proxy_key",
    )
    .bind(site)
    .fetch_one(&state.pool)
    .await?;
    Ok(Json(json!({"proxyKey":key})))
}

/// The visitor IP for a collected event. A site's first-party proxy connects from the
/// website's own server, so it forwards the visitor IP in `x-slimlytics-client-ip`, vouched for
/// by the site's proxy key. Without a matching key the header is ignored.
fn collection_ip(headers: &HeaderMap, peer: IpAddr, trust_proxy: bool, proxy_key: Uuid) -> IpAddr {
    let vouched = headers
        .get("x-slimlytics-proxy-key")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| Uuid::parse_str(value.trim()).ok())
        .is_some_and(|key| constant_time_eq(key.as_bytes(), proxy_key.as_bytes()));
    if vouched {
        if let Some(ip) = headers
            .get("x-slimlytics-client-ip")
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.trim().parse().ok())
        {
            return ip;
        }
    }
    client_ip(headers, peer, trust_proxy)
}

fn constant_time_eq(left: &[u8], right: &[u8]) -> bool {
    left.len() == right.len()
        && left
            .iter()
            .zip(right)
            .fold(0u8, |acc, (a, b)| acc | (a ^ b))
            == 0
}

fn client_ip(headers: &HeaderMap, peer: IpAddr, trust_proxy: bool) -> IpAddr {
    if trust_proxy {
        headers
            .get("x-forwarded-for")
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.split(',').next())
            .and_then(|value| value.trim().parse().ok())
            .unwrap_or(peer)
    } else {
        peer
    }
}

async fn collect_test(
    State(state): State<AppState>,
    Path(key): Path<Uuid>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, ApiError> {
    let allowed: Option<Vec<String>> =
        sqlx::query_scalar("SELECT allowed_origins FROM sites WHERE write_key=$1")
            .bind(key)
            .fetch_optional(&state.pool)
            .await?;
    let allowed = allowed.ok_or(ApiError::NotFound)?;
    let origin = headers
        .get(header::ORIGIN)
        .and_then(|value| value.to_str().ok());
    if origin.is_some() && !origin_allowed(origin, &allowed) {
        return Err(ApiError::Forbidden);
    }
    let mut response_headers = HeaderMap::new();
    response_headers.insert(header::CACHE_CONTROL, "no-store".parse().unwrap());
    if let Some(origin) = origin {
        response_headers.insert(
            header::ACCESS_CONTROL_ALLOW_ORIGIN,
            origin
                .parse()
                .map_err(|_| ApiError::BadRequest("invalid origin".into()))?,
        );
        response_headers.insert(header::VARY, "Origin".parse().unwrap());
    }
    Ok((response_headers, Json(json!({"status":"ok"}))))
}

#[axum::debug_handler]
async fn collect(
    State(state): State<AppState>,
    Path(key): Path<Uuid>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Json(input): Json<CollectInput>,
) -> Result<impl IntoResponse, ApiError> {
    let site: Option<(Uuid, Vec<String>, Uuid)> =
        sqlx::query_as("SELECT id,allowed_origins,proxy_key FROM sites WHERE write_key=$1")
            .bind(key)
            .fetch_optional(&state.pool)
            .await?;
    let (site, allowed, proxy_key) = site.ok_or(ApiError::NotFound)?;
    let origin = headers.get(header::ORIGIN).and_then(|v| v.to_str().ok());
    let referer = headers.get(header::REFERER).and_then(|v| v.to_str().ok());
    if !collection_origin_allowed(origin, referer, &allowed) {
        record_collection_rejection(&state.pool, site, "origin").await;
        return Err(ApiError::Forbidden);
    }
    let ip = collection_ip(&headers, peer.ip(), state.trust_proxy, proxy_key);
    if !state.limiter.check(&format!("{site}:{ip}")) {
        record_collection_rejection(&state.pool, site, "rate_limited").await;
        return Err(ApiError::RateLimited);
    }
    let ua = headers
        .get(header::USER_AGENT)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    let at = input.occurred_at.unwrap_or_else(Utc::now);
    if (at - Utc::now()).num_hours().abs() > 24 {
        return Err(ApiError::BadRequest(
            "occurred_at outside accepted window".into(),
        ));
    }
    if input.name.trim().is_empty()
        || input.name.chars().count() > 200
        || input.url.len() > 2048
        || input.referrer.as_ref().is_some_and(|v| v.len() > 2048)
        || input
            .title
            .as_ref()
            .is_some_and(|v| v.chars().count() > 500)
        || input.properties.to_string().len() > 16 * 1024
    {
        record_collection_rejection(&state.pool, site, "invalid_payload").await;
        return Err(ApiError::BadRequest("event payload too large".into()));
    }
    let clean = sanitize_url(&input.url).map_err(|_| ApiError::BadRequest("invalid url".into()))?;
    let parsed = Url::parse(&input.url).map_err(|_| ApiError::BadRequest("invalid url".into()))?;
    let ids = derive_ids(
        &state.identity_secret,
        &site.to_string(),
        &ip.to_string(),
        ua,
        at,
    );
    let referrer = input
        .referrer
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(sanitize_url)
        .transpose()
        .map_err(|_| ApiError::BadRequest("invalid referrer".into()))?;
    let ref_host = referrer
        .as_deref()
        .and_then(|v| Url::parse(v).ok())
        .and_then(|v| v.host_str().map(str::to_owned));
    let class = traffic_class(ua, ip, &state.internal_ips);
    let automation = automation_for(ua, ip);
    let client = client_metadata(ua);
    if !input.properties.is_object() {
        record_collection_rejection(&state.pool, site, "invalid_properties").await;
        return Err(ApiError::BadRequest("properties must be an object".into()));
    }
    let privacy_mode = match input.privacy_control.as_deref() {
        None => "standard",
        Some("gpc") => "gpc",
        Some(_) => {
            record_collection_rejection(&state.pool, site, "invalid_privacy_control").await;
            return Err(ApiError::BadRequest("invalid privacyControl".into()));
        }
    };
    let tracker_version = input
        .tracker_version
        .as_deref()
        .filter(|value| {
            (1..=32).contains(&value.len())
                && value
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-' | b'+'))
        })
        .map(str::to_owned);
    if input.tracker_version.is_some() && tracker_version.is_none() {
        record_collection_rejection(&state.pool, site, "invalid_tracker_version").await;
        return Err(ApiError::BadRequest("invalid trackerVersion".into()));
    }
    let properties = if privacy_mode == "gpc" {
        json!({})
    } else {
        input.properties
    };
    let marketing = marketing_context(&properties);
    let title = if privacy_mode == "gpc" {
        None
    } else {
        input.title
    };
    let mut location = location_from_headers(&headers, state.trust_proxy)
        .or_else(|| state.geoip.as_ref().and_then(|geoip| geoip.lookup(ip)))
        .unwrap_or_default();
    if privacy_mode == "gpc" {
        location.region = None;
        location.city = None;
    }
    let device = if input.screen_width.is_some_and(|width| width < 768) {
        "mobile"
    } else {
        client.device_type
    };
    let q = |name: &str| {
        parsed
            .query_pairs()
            .find(|(k, _)| k == name)
            .map(|(_, v)| v.into_owned())
    };
    let mut tx = state.pool.begin().await?;
    let event_id: Uuid = sqlx::query_scalar(
        "INSERT INTO events(
            site_id,occurred_at,visitor_id,session_id,event_name,url,path,referrer,referrer_host,
            title,country_code,region,city,continent_code,device_type,browser,browser_version,os,
            os_version,utm_source,utm_medium,utm_campaign,utm_term,utm_content,revenue_amount,
            revenue_currency,content_id,content_type,content_author,automation_name,
            automation_category,properties,traffic_class,privacy_mode,tracker_version
         ) VALUES(
            $1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,$18,$19,$20,$21,
            $22,$23,$24,$25::numeric,$26,$27,$28,$29,$30,$31,$32,$33::traffic_class,$34,$35
         ) RETURNING id",
    )
    .bind(site)
    .bind(at)
    .bind(&ids.visitor_id)
    .bind(&ids.session_id)
    .bind(&input.name)
    .bind(&clean)
    .bind(parsed.path())
    .bind(referrer)
    .bind(ref_host.as_deref())
    .bind(title)
    .bind(location.country_code.as_deref())
    .bind(location.region.as_deref())
    .bind(location.city.as_deref())
    .bind(location.continent.as_deref())
    .bind(device)
    .bind(&client.browser)
    .bind(client.browser_version.as_deref())
    .bind(&client.os)
    .bind(client.os_version.as_deref())
    .bind(q("utm_source"))
    .bind(q("utm_medium"))
    .bind(q("utm_campaign"))
    .bind(q("utm_term"))
    .bind(q("utm_content"))
    .bind(marketing.revenue_amount.as_deref())
    .bind(marketing.revenue_currency.as_deref())
    .bind(marketing.content_id.as_deref())
    .bind(marketing.content_type.as_deref())
    .bind(marketing.content_author.as_deref())
    .bind(automation.map(|value| value.name))
    .bind(automation.map(|value| value.category))
    .bind(properties)
    .bind(class)
    .bind(privacy_mode)
    .bind(tracker_version.as_deref())
    .fetch_one(&mut *tx)
    .await?;
    complete_goals(
        &mut tx,
        site,
        event_id,
        &ids.visitor_id,
        at,
        &input.name,
        parsed.path(),
    )
    .await?;
    let payload = json!({
        "id": event_id,
        "type": input.name,
        "page": parsed.path(),
        "visitorId": ids.visitor_id,
        "country": location.country_code,
        "timestamp": at,
        "referrer": ref_host
    });
    let stream_id: i64 = sqlx::query_scalar(
        "INSERT INTO stream_events(site_id,event_id,payload) VALUES($1,$2,$3) RETURNING id",
    )
    .bind(site)
    .bind(event_id)
    .bind(&payload)
    .fetch_one(&mut *tx)
    .await?;
    tx.commit().await?;
    record_collection_acceptance(&state.pool, site, tracker_version.as_deref()).await;
    let _ = state.stream_tx.send(StreamMessage {
        id: stream_id,
        site_id: site,
        payload,
    });
    let mut response_headers = HeaderMap::new();
    if let Some(origin) = origin {
        response_headers.insert(
            header::ACCESS_CONTROL_ALLOW_ORIGIN,
            origin
                .parse()
                .map_err(|_| ApiError::BadRequest("invalid origin".into()))?,
        );
        response_headers.insert(header::VARY, "Origin".parse().unwrap());
    }
    Ok((response_headers, StatusCode::ACCEPTED))
}

#[axum::debug_handler]
async fn server_collect(
    State(state): State<AppState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Json(batch): Json<ServerEventBatch>,
) -> Result<impl IntoResponse, ApiError> {
    if batch.events.is_empty() || batch.events.len() > 100 {
        return Err(ApiError::BadRequest(
            "events must contain between 1 and 100 items".into(),
        ));
    }
    let key = headers
        .get("x-slimlytics-server-key")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| Uuid::parse_str(value).ok())
        .ok_or(ApiError::Unauthorized)?;
    let site: Option<(Uuid, String)> =
        sqlx::query_as("SELECT id,domain FROM sites WHERE server_write_key=$1")
            .bind(key)
            .fetch_optional(&state.pool)
            .await?;
    let (site, domain) = site.ok_or(ApiError::Unauthorized)?;
    if !state.limiter.check(&format!("server:{site}:{}", peer.ip())) {
        record_collection_rejection(&state.pool, site, "server_rate_limited").await;
        return Err(ApiError::RateLimited);
    }

    let mut accepted = 0_i64;
    let mut duplicates = 0_i64;
    let mut tx = state.pool.begin().await?;
    for raw in batch.events {
        let event = validate_server_event(raw, &domain)
            .map_err(|message| ApiError::BadRequest(message.into()))?;
        let clean = sanitize_url(event.url.as_str())
            .map_err(|_| ApiError::BadRequest("invalid url".into()))?;
        let referrer = event
            .referrer
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(sanitize_url)
            .transpose()
            .map_err(|_| ApiError::BadRequest("invalid referrer".into()))?;
        let referrer_host = referrer
            .as_deref()
            .and_then(|value| Url::parse(value).ok())
            .and_then(|value| value.host_str().map(str::to_owned));
        let ids = derive_ids(
            &state.identity_secret,
            &site.to_string(),
            &event.client_ip.to_string(),
            &event.user_agent,
            event.occurred_at,
        );
        let class = traffic_class(&event.user_agent, event.client_ip, &state.internal_ips);
        let automation = automation_for(&event.user_agent, event.client_ip);
        let client = client_metadata(&event.user_agent);
        let location = state
            .geoip
            .as_ref()
            .and_then(|geoip| geoip.lookup(event.client_ip))
            .unwrap_or_default();
        let query = |name: &str| {
            event
                .url
                .query_pairs()
                .find(|(key, _)| key == name)
                .map(|(_, value)| value.into_owned())
        };
        let properties = json!({"method":event.method,"status":event.status});
        let event_id: Option<Uuid> = sqlx::query_scalar(
            "INSERT INTO events(
               site_id,occurred_at,visitor_id,session_id,event_name,url,path,referrer,
               referrer_host,country_code,region,city,continent_code,device_type,browser,
               browser_version,os,os_version,utm_source,utm_medium,utm_campaign,utm_term,
               utm_content,automation_name,automation_category,properties,traffic_class,
               privacy_mode,tracker_version,ingestion_source,source_event_id
             ) VALUES(
               $1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,$18,$19,
               $20,$21,$22,$23,$24,$25,$26,$27::traffic_class,'standard','server','server',$28
             ) ON CONFLICT(site_id,source_event_id) WHERE source_event_id IS NOT NULL
               DO NOTHING RETURNING id",
        )
        .bind(site)
        .bind(event.occurred_at)
        .bind(&ids.visitor_id)
        .bind(&ids.session_id)
        .bind(&event.event_name)
        .bind(&clean)
        .bind(event.url.path())
        .bind(referrer)
        .bind(referrer_host.as_deref())
        .bind(location.country_code.as_deref())
        .bind(location.region.as_deref())
        .bind(location.city.as_deref())
        .bind(location.continent.as_deref())
        .bind(client.device_type)
        .bind(&client.browser)
        .bind(client.browser_version.as_deref())
        .bind(&client.os)
        .bind(client.os_version.as_deref())
        .bind(query("utm_source"))
        .bind(query("utm_medium"))
        .bind(query("utm_campaign"))
        .bind(query("utm_term"))
        .bind(query("utm_content"))
        .bind(automation.map(|value| value.name))
        .bind(automation.map(|value| value.category))
        .bind(properties)
        .bind(class)
        .bind(event.idempotency_key.as_deref())
        .fetch_optional(&mut *tx)
        .await?;
        let Some(event_id) = event_id else {
            duplicates += 1;
            continue;
        };
        if class == "human" {
            complete_goals(
                &mut tx,
                site,
                event_id,
                &ids.visitor_id,
                event.occurred_at,
                &event.event_name,
                event.url.path(),
            )
            .await?;
        }
        accepted += 1;
    }
    tx.commit().await?;
    if accepted > 0 {
        record_collection_acceptance_by(&state.pool, site, accepted, Some("server")).await;
    }
    Ok((
        StatusCode::ACCEPTED,
        Json(json!({"accepted":accepted,"duplicates":duplicates})),
    ))
}

async fn record_collection_acceptance(pool: &PgPool, site: Uuid, tracker_version: Option<&str>) {
    record_collection_acceptance_by(pool, site, 1, tracker_version).await;
}

async fn record_collection_acceptance_by(
    pool: &PgPool,
    site: Uuid,
    count: i64,
    tracker_version: Option<&str>,
) {
    if let Err(error) = sqlx::query(
        "INSERT INTO collection_health(site_id,accepted_total,last_accepted_at,last_tracker_version) \
         VALUES($1,$3,now(),$2) \
         ON CONFLICT(site_id) DO UPDATE SET \
         accepted_total=collection_health.accepted_total+EXCLUDED.accepted_total, \
         last_accepted_at=now(), \
         last_tracker_version=COALESCE(EXCLUDED.last_tracker_version,collection_health.last_tracker_version), \
         updated_at=now()",
    )
    .bind(site)
    .bind(tracker_version)
    .bind(count)
    .execute(pool)
    .await
    {
        tracing::warn!(%error, %site, "failed to update collection acceptance health");
    }
}

async fn record_collection_rejection(pool: &PgPool, site: Uuid, code: &str) {
    if let Err(error) = sqlx::query(
        "INSERT INTO collection_health(site_id,rejected_total,last_rejected_at,last_rejection_code) \
         VALUES($1,1,now(),$2) \
         ON CONFLICT(site_id) DO UPDATE SET \
         rejected_total=collection_health.rejected_total+1, \
         last_rejected_at=now(), \
         last_rejection_code=EXCLUDED.last_rejection_code, \
         updated_at=now()",
    )
    .bind(site)
    .bind(code)
    .execute(pool)
    .await
    {
        tracing::warn!(%error, %site, "failed to update collection rejection health");
    }
}

async fn collect_options(
    State(state): State<AppState>,
    Path(key): Path<Uuid>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, ApiError> {
    let allowed: Option<Vec<String>> =
        sqlx::query_scalar("SELECT allowed_origins FROM sites WHERE write_key=$1")
            .bind(key)
            .fetch_optional(&state.pool)
            .await?;
    let allowed = allowed.ok_or(ApiError::NotFound)?;
    let origin = headers
        .get(header::ORIGIN)
        .and_then(|value| value.to_str().ok())
        .ok_or(ApiError::Forbidden)?;
    if !origin_allowed(Some(origin), &allowed) {
        return Err(ApiError::Forbidden);
    }
    let mut response_headers = HeaderMap::new();
    response_headers.insert(
        header::ACCESS_CONTROL_ALLOW_ORIGIN,
        origin
            .parse()
            .map_err(|_| ApiError::BadRequest("invalid origin".into()))?,
    );
    response_headers.insert(
        header::ACCESS_CONTROL_ALLOW_METHODS,
        "POST, OPTIONS".parse().unwrap(),
    );
    response_headers.insert(
        header::ACCESS_CONTROL_ALLOW_HEADERS,
        "content-type".parse().unwrap(),
    );
    response_headers.insert(header::ACCESS_CONTROL_MAX_AGE, "86400".parse().unwrap());
    response_headers.insert(header::VARY, "Origin".parse().unwrap());
    Ok((response_headers, StatusCode::NO_CONTENT))
}

async fn complete_goals(
    tx: &mut Transaction<'_, Postgres>,
    site: Uuid,
    event: Uuid,
    visitor: &str,
    at: DateTime<Utc>,
    name: &str,
    path: &str,
) -> Result<(), ApiError> {
    sqlx::query("INSERT INTO goal_completions(goal_id,event_id,site_id,visitor_id,occurred_at) SELECT id,$2,$1,$3,$4 FROM goals WHERE site_id=$1 AND event_name=$5 AND (path_pattern IS NULL OR $6 LIKE path_pattern) ON CONFLICT DO NOTHING").bind(site).bind(event).bind(visitor).bind(at).bind(name).bind(path).execute(&mut **tx).await?;
    Ok(())
}

async fn bounds(
    pool: &PgPool,
    site: Uuid,
    from: chrono::NaiveDate,
    to: chrono::NaiveDate,
) -> Result<(DateBounds, String), ApiError> {
    let timezone: String = sqlx::query_scalar("SELECT timezone FROM sites WHERE id=$1")
        .bind(site)
        .fetch_one(pool)
        .await?;
    let bounds =
        date_bounds(from, to, &timezone).map_err(|message| ApiError::BadRequest(message.into()))?;
    Ok((bounds, timezone))
}
async fn counts(
    pool: &PgPool,
    site: Uuid,
    a: DateTime<Utc>,
    b: DateTime<Utc>,
) -> Result<(i64, i64, i64, i64), ApiError> {
    Ok(sqlx::query_as("SELECT count(*) FILTER(WHERE event_name='pageview'),count(DISTINCT visitor_id),count(DISTINCT session_id),count(*) FILTER(WHERE event_name<>'pageview') FROM events WHERE site_id=$1 AND occurred_at >= $2 AND occurred_at < $3 AND traffic_class='human'").bind(site).bind(a).bind(b).fetch_one(pool).await?)
}
fn metric(current: i64, previous: i64) -> Metric {
    Metric {
        current,
        previous,
        change_percent: if previous == 0 {
            None
        } else {
            Some((current - previous) as f64 * 100.0 / previous as f64)
        },
    }
}
async fn overview(
    State(s): State<AppState>,
    CurrentUser(u): CurrentUser,
    Path(site): Path<Uuid>,
    Query(q): Query<RangeQuery>,
) -> Result<Json<Overview>, ApiError> {
    require_site(&s.pool, u, site, false).await?;
    let ((a, b, p), timezone) = bounds(&s.pool, site, q.from, q.to).await?;
    let c = counts(&s.pool, site, a, b).await?;
    let old = counts(&s.pool, site, p, a).await?;
    let current_online: i64 = sqlx::query_scalar(
        "SELECT count(DISTINCT visitor_id) FROM events \
         WHERE site_id=$1 AND traffic_class='human' \
         AND occurred_at > now() - interval '5 minutes'",
    )
    .bind(site)
    .fetch_one(&s.pool)
    .await?;
    let (bounce_rate, avg_duration_seconds): (f64, f64) = sqlx::query_as(
        "SELECT \
         COALESCE(100.0 * count(*) FILTER (WHERE page_views=1) / NULLIF(count(*),0),0)::float8, \
         COALESCE(avg(duration_seconds),0)::float8 \
         FROM ( \
           SELECT session_id, \
             count(*) FILTER (WHERE event_name='pageview') page_views, \
             EXTRACT(EPOCH FROM max(occurred_at)-min(occurred_at)) duration_seconds \
           FROM events \
           WHERE site_id=$1 AND occurred_at >= $2 AND occurred_at < $3 AND traffic_class='human' \
           GROUP BY session_id \
         ) sessions WHERE page_views > 0",
    )
    .bind(site)
    .bind(a)
    .bind(b)
    .fetch_one(&s.pool)
    .await?;
    let trend: Vec<TrendPoint> = sqlx::query_as(
        "SELECT day::date date, \
         COALESCE(r.visitors,live.visitors,0)::bigint visitors, \
         COALESCE(r.page_views,live.page_views,0)::bigint page_views \
         FROM generate_series($2::date,$3::date,interval '1 day') day \
         LEFT JOIN daily_site_rollups r ON r.site_id=$1 AND r.metric_date=day::date \
         LEFT JOIN LATERAL ( \
           SELECT count(DISTINCT visitor_id) FILTER(WHERE traffic_class='human') visitors, \
             count(*) FILTER(WHERE traffic_class='human' AND event_name='pageview') page_views \
           FROM events WHERE site_id=$1 AND occurred_at >= $5 AND occurred_at < $6 \
             AND (occurred_at AT TIME ZONE $4)::date=day::date \
         ) live ON r.site_id IS NULL \
         ORDER BY day",
    )
    .bind(site)
    .bind(q.from)
    .bind(q.to)
    .bind(timezone)
    .bind(a)
    .bind(b)
    .fetch_all(&s.pool)
    .await?;
    Ok(Json(Overview {
        views: metric(c.0, old.0),
        visitors: metric(c.1, old.1),
        sessions: metric(c.2, old.2),
        events: metric(c.3, old.3),
        current_online,
        bounce_rate,
        avg_duration_seconds,
        trend,
    }))
}
async fn report(
    State(s): State<AppState>,
    CurrentUser(u): CurrentUser,
    Path((site, dimension)): Path<(Uuid, String)>,
    Query(q): Query<ReportQuery>,
) -> Result<Json<Vec<ReportRow>>, ApiError> {
    require_site(&s.pool, u, site, false).await?;
    let ((a, b, _), _) = bounds(&s.pool, site, q.from, q.to).await?;
    let limit = q.limit.unwrap_or(100).clamp(1, 500);
    if matches!(dimension.as_str(), "landing-pages" | "exit-pages") {
        let direction = if dimension == "landing-pages" {
            "ASC"
        } else {
            "DESC"
        };
        let sql = format!(
            "WITH ranked AS (
               SELECT path,visitor_id,
                 row_number() OVER(PARTITION BY session_id ORDER BY occurred_at {direction}) rank
               FROM events WHERE site_id=$1 AND occurred_at >= $2 AND occurred_at < $3
                 AND traffic_class='human' AND event_name='pageview'
             )
             SELECT path value,count(*)::bigint views,count(DISTINCT visitor_id)::bigint visitors
             FROM ranked WHERE rank=1 GROUP BY path ORDER BY views DESC LIMIT $4"
        );
        return Ok(Json(
            sqlx::query_as(&sql)
                .bind(site)
                .bind(a)
                .bind(b)
                .bind(limit)
                .fetch_all(&s.pool)
                .await?,
        ));
    }
    if dimension == "ai-crawlers" {
        return Ok(Json(
            sqlx::query_as(
                "SELECT COALESCE(automation_name,'unknown') value,count(*)::bigint views,
                 count(DISTINCT visitor_id)::bigint visitors
                 FROM events WHERE site_id=$1 AND occurred_at >= $2 AND occurred_at < $3
                   AND automation_category='ai-crawler'
                 GROUP BY 1 ORDER BY views DESC LIMIT $4",
            )
            .bind(site)
            .bind(a)
            .bind(b)
            .bind(limit)
            .fetch_all(&s.pool)
            .await?,
        ));
    }
    let column = match dimension.as_str() {
        "pages" => "path",
        "referrers" => "COALESCE(referrer_host, '(direct)')",
        "countries" => "COALESCE(country_code, 'unknown')",
        "regions" => "COALESCE(region, 'unknown')",
        "cities" => "COALESCE(city, 'unknown')",
        "devices" => "COALESCE(device_type, 'unknown')",
        "browsers" => "COALESCE(browser, 'unknown')",
        "operating-systems" => "COALESCE(os, 'unknown')",
        "campaigns" => "COALESCE(utm_campaign, '(none)')",
        "sources" => "COALESCE(utm_source, referrer_host, '(direct)')",
        "mediums" => "COALESCE(utm_medium, '(none)')",
        "content" => "COALESCE(content_id, '(not set)')",
        "content-types" => "COALESCE(content_type, '(not set)')",
        "content-authors" => "COALESCE(content_author, '(not set)')",
        "ai-referrers" => "referrer_host",
        _ => return Err(ApiError::NotFound),
    };
    let extra_filter = if dimension == "ai-referrers" {
        "AND referrer_host IN (
          'chatgpt.com','chat.openai.com','perplexity.ai','claude.ai',
          'copilot.microsoft.com','gemini.google.com'
        )"
    } else {
        ""
    };
    let sql=format!("SELECT {column}::text value,count(*) views,count(DISTINCT visitor_id) visitors FROM events WHERE site_id=$1 AND occurred_at >= $2 AND occurred_at < $3 AND traffic_class='human' {extra_filter} GROUP BY 1 ORDER BY views DESC LIMIT $4");
    Ok(Json(
        sqlx::query_as(&sql)
            .bind(site)
            .bind(a)
            .bind(b)
            .bind(limit)
            .fetch_all(&s.pool)
            .await?,
    ))
}

async fn common_journeys(
    State(s): State<AppState>,
    CurrentUser(u): CurrentUser,
    Path(site): Path<Uuid>,
    Query(q): Query<ReportQuery>,
) -> Result<Json<Vec<Value>>, ApiError> {
    require_site(&s.pool, u, site, false).await?;
    let ((a, b, _), _) = bounds(&s.pool, site, q.from, q.to).await?;
    let rows: Vec<(Vec<String>, i64, i64)> = sqlx::query_as(
        "WITH journeys AS (
           SELECT session_id, visitor_id, array_agg(path ORDER BY occurred_at) steps
           FROM events
           WHERE site_id=$1 AND occurred_at >= $2 AND occurred_at < $3
             AND traffic_class='human' AND event_name='pageview'
           GROUP BY session_id,visitor_id
         )
         SELECT steps,count(*)::bigint,count(DISTINCT visitor_id)::bigint
         FROM journeys GROUP BY steps ORDER BY count(*) DESC LIMIT $4",
    )
    .bind(site)
    .bind(a)
    .bind(b)
    .bind(q.limit.unwrap_or(50).clamp(1, 200))
    .fetch_all(&s.pool)
    .await?;
    Ok(Json(
        rows.into_iter()
            .map(|row| json!({"steps":row.0,"sessions":row.1,"visitors":row.2}))
            .collect(),
    ))
}

async fn attribution(
    State(s): State<AppState>,
    CurrentUser(u): CurrentUser,
    Path(site): Path<Uuid>,
    Query(q): Query<ReportQuery>,
) -> Result<Json<Vec<Value>>, ApiError> {
    require_site(&s.pool, u, site, false).await?;
    let ((a, b, _), _) = bounds(&s.pool, site, q.from, q.to).await?;
    type AttributionRow = (String, String, String, i64, i64, f64);
    let rows: Vec<AttributionRow> = sqlx::query_as(
        "WITH first_touch AS (
           SELECT DISTINCT ON(visitor_id) visitor_id,
             COALESCE(utm_source,referrer_host,'(direct)') source,
             COALESCE(utm_medium,'(none)') medium,
             COALESCE(utm_campaign,'(none)') campaign
           FROM events
           WHERE site_id=$1 AND occurred_at >= $2 AND occurred_at < $3
             AND traffic_class='human'
           ORDER BY visitor_id,occurred_at
        ), conversions AS (
           SELECT e.visitor_id,count(DISTINCT gc.id)::bigint conversions
           FROM goal_completions gc JOIN events e ON e.id=gc.event_id
           WHERE e.site_id=$1 AND e.occurred_at >= $2 AND e.occurred_at < $3
             AND e.traffic_class='human' GROUP BY e.visitor_id
        ), revenue AS (
           SELECT visitor_id,COALESCE(sum(revenue_amount),0)::float8 revenue
           FROM events WHERE site_id=$1 AND occurred_at >= $2 AND occurred_at < $3
             AND traffic_class='human' GROUP BY visitor_id
         )
         SELECT source,medium,campaign,count(*)::bigint AS visitor_total,
           COALESCE(sum(conversions.conversions),0)::bigint AS conversion_total,
           COALESCE(sum(revenue.revenue),0)::float8 AS revenue_total
         FROM first_touch
         LEFT JOIN conversions USING(visitor_id)
         LEFT JOIN revenue USING(visitor_id)
         GROUP BY source,medium,campaign
         ORDER BY conversion_total DESC,revenue_total DESC,visitor_total DESC LIMIT $4",
    )
    .bind(site)
    .bind(a)
    .bind(b)
    .bind(q.limit.unwrap_or(100).clamp(1, 500))
    .fetch_all(&s.pool)
    .await?;
    Ok(Json(
        rows.into_iter()
            .map(|row| {
                json!({
                    "source":row.0,"medium":row.1,"campaign":row.2,
                    "visitors":row.3,"conversions":row.4,"revenue":row.5
                })
            })
            .collect(),
    ))
}

async fn anomalies(
    State(s): State<AppState>,
    CurrentUser(u): CurrentUser,
    Path(site): Path<Uuid>,
    Query(q): Query<RangeQuery>,
) -> Result<Json<Vec<Value>>, ApiError> {
    require_site(&s.pool, u, site, false).await?;
    let ((a, b, _), timezone) = bounds(&s.pool, site, q.from, q.to).await?;
    let rows: Vec<(chrono::NaiveDate, i64)> = sqlx::query_as(
        "SELECT day::date,count(events.id)::bigint
         FROM generate_series($5::date,$6::date,interval '1 day') day
         LEFT JOIN events ON site_id=$1 AND occurred_at >= $2 AND occurred_at < $3
           AND (occurred_at AT TIME ZONE $4)::date=day::date
           AND traffic_class='human' AND event_name='pageview'
         GROUP BY day ORDER BY day",
    )
    .bind(site)
    .bind(a)
    .bind(b)
    .bind(timezone)
    .bind(q.from)
    .bind(q.to)
    .fetch_all(&s.pool)
    .await?;
    let mut history = Vec::<f64>::new();
    let mut result = Vec::new();
    for (date, views) in rows {
        if history.len() >= 7 {
            let baseline = history[history.len() - 7..].iter().sum::<f64>() / 7.0;
            let deviation = if baseline == 0.0 {
                0.0
            } else {
                (views as f64 - baseline) * 100.0 / baseline
            };
            if deviation.abs() >= 30.0 {
                result.push(json!({
                    "date":date,"metric":"pageViews","value":views,
                    "baseline":baseline,"deviationPercent":deviation,
                    "direction":if deviation > 0.0 {"up"} else {"down"}
                }));
            }
        }
        history.push(views as f64);
    }
    Ok(Json(result))
}

async fn list_annotations(
    State(s): State<AppState>,
    CurrentUser(u): CurrentUser,
    Path(site): Path<Uuid>,
    Query(q): Query<RangeQuery>,
) -> Result<Json<Vec<Annotation>>, ApiError> {
    require_site(&s.pool, u, site, false).await?;
    Ok(Json(
        sqlx::query_as(
            "SELECT id,site_id,occurred_on,label,created_at FROM annotations
         WHERE site_id=$1 AND occurred_on BETWEEN $2 AND $3 ORDER BY occurred_on DESC",
        )
        .bind(site)
        .bind(q.from)
        .bind(q.to)
        .fetch_all(&s.pool)
        .await?,
    ))
}

fn idempotency_key(headers: &HeaderMap) -> Result<Option<&str>, ApiError> {
    headers
        .get("idempotency-key")
        .map(|value| {
            value
                .to_str()
                .map_err(|_| ApiError::BadRequest("invalid Idempotency-Key".into()))
                .and_then(|value| {
                    validate_idempotency_key(value)
                        .map_err(|message| ApiError::BadRequest(message.into()))
                })
        })
        .transpose()
}

async fn cached_idempotent_response(
    tx: &mut Transaction<'_, Postgres>,
    user: Uuid,
    site: Uuid,
    operation: &str,
    key: &str,
) -> Result<Option<(StatusCode, Value)>, ApiError> {
    let lock = format!("{user}:{site}:{operation}:{key}");
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1,0))")
        .bind(lock)
        .execute(&mut **tx)
        .await?;
    let cached: Option<(i32, Value)> = sqlx::query_as(
        "SELECT response_status,response_body FROM idempotency_keys
         WHERE user_id=$1 AND site_id=$2 AND operation=$3 AND idempotency_key=$4
           AND expires_at>now()",
    )
    .bind(user)
    .bind(site)
    .bind(operation)
    .bind(key)
    .fetch_optional(&mut **tx)
    .await?;
    cached
        .map(|(status, body)| {
            StatusCode::from_u16(status as u16)
                .map(|status| (status, body))
                .map_err(|_| ApiError::Internal)
        })
        .transpose()
}

async fn save_idempotent_response(
    tx: &mut Transaction<'_, Postgres>,
    identity: (Uuid, Uuid),
    operation: &str,
    key: &str,
    status: StatusCode,
    body: &Value,
) -> Result<(), ApiError> {
    sqlx::query(
        "INSERT INTO idempotency_keys(
           user_id,site_id,operation,idempotency_key,response_status,response_body
         ) VALUES($1,$2,$3,$4,$5,$6)",
    )
    .bind(identity.0)
    .bind(identity.1)
    .bind(operation)
    .bind(key)
    .bind(i32::from(status.as_u16()))
    .bind(body)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

async fn create_annotation(
    State(s): State<AppState>,
    CurrentUser(u): CurrentUser,
    Path(site): Path<Uuid>,
    headers: HeaderMap,
    Json(input): Json<AnnotationInput>,
) -> Result<Response, ApiError> {
    require_site(&s.pool, u, site, true).await?;
    let label = input.label.trim();
    if label.is_empty() || label.len() > 240 {
        return Err(ApiError::BadRequest("invalid annotation label".into()));
    }
    let key = idempotency_key(&headers)?;
    let mut tx = s.pool.begin().await?;
    if let Some(key) = key {
        if let Some((status, body)) =
            cached_idempotent_response(&mut tx, u, site, "create_annotation", key).await?
        {
            tx.commit().await?;
            return Ok((status, Json(body)).into_response());
        }
    }
    let annotation: Annotation = sqlx::query_as(
        "INSERT INTO annotations(site_id,occurred_on,label,created_by)
         VALUES($1,$2,$3,$4)
         RETURNING id,site_id,occurred_on,label,created_at",
    )
    .bind(site)
    .bind(input.occurred_on)
    .bind(label)
    .bind(u)
    .fetch_one(&mut *tx)
    .await?;
    let body = serde_json::to_value(annotation).map_err(|_| ApiError::Internal)?;
    if let Some(key) = key {
        save_idempotent_response(
            &mut tx,
            (u, site),
            "create_annotation",
            key,
            StatusCode::CREATED,
            &body,
        )
        .await?;
    }
    tx.commit().await?;
    Ok((StatusCode::CREATED, Json(body)).into_response())
}

async fn delete_annotation(
    State(s): State<AppState>,
    CurrentUser(u): CurrentUser,
    Path((site, annotation)): Path<(Uuid, Uuid)>,
) -> Result<StatusCode, ApiError> {
    require_site(&s.pool, u, site, true).await?;
    sqlx::query("DELETE FROM annotations WHERE id=$1 AND site_id=$2")
        .bind(annotation)
        .bind(site)
        .execute(&s.pool)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

fn validate_funnel(input: &FunnelInput) -> Result<(), ApiError> {
    if input.name.trim().is_empty()
        || input.name.len() > 120
        || !(2..=10).contains(&input.steps.len())
        || input.steps.iter().any(|step| {
            step.label.trim().is_empty()
                || step.label.len() > 120
                || (step.event_name.is_some() == step.path.is_some())
                || step
                    .event_name
                    .as_ref()
                    .or(step.path.as_ref())
                    .is_some_and(|value| value.is_empty() || value.len() > 500)
        })
    {
        return Err(ApiError::BadRequest("invalid funnel".into()));
    }
    Ok(())
}

async fn list_funnels(
    State(s): State<AppState>,
    CurrentUser(u): CurrentUser,
    Path(site): Path<Uuid>,
) -> Result<Json<Vec<Value>>, ApiError> {
    require_site(&s.pool, u, site, false).await?;
    type FunnelRow = (Uuid, String, Value, DateTime<Utc>);
    let rows: Vec<FunnelRow> = sqlx::query_as(
        "SELECT id,name,steps,created_at FROM funnels WHERE site_id=$1 ORDER BY created_at",
    )
    .bind(site)
    .fetch_all(&s.pool)
    .await?;
    Ok(Json(
        rows.into_iter()
            .map(|row| json!({"id":row.0,"name":row.1,"steps":row.2,"createdAt":row.3}))
            .collect(),
    ))
}

async fn create_funnel(
    State(s): State<AppState>,
    CurrentUser(u): CurrentUser,
    Path(site): Path<Uuid>,
    headers: HeaderMap,
    Json(input): Json<FunnelInput>,
) -> Result<Response, ApiError> {
    require_site(&s.pool, u, site, true).await?;
    validate_funnel(&input)?;
    let key = idempotency_key(&headers)?;
    let mut tx = s.pool.begin().await?;
    if let Some(key) = key {
        if let Some((status, body)) =
            cached_idempotent_response(&mut tx, u, site, "create_funnel", key).await?
        {
            tx.commit().await?;
            return Ok((status, Json(body)).into_response());
        }
    }
    let steps = serde_json::to_value(&input.steps).map_err(|_| ApiError::Internal)?;
    let row: (Uuid, String, Value, DateTime<Utc>) = sqlx::query_as(
        "INSERT INTO funnels(site_id,name,steps) VALUES($1,$2,$3)
         RETURNING id,name,steps,created_at",
    )
    .bind(site)
    .bind(input.name.trim())
    .bind(steps)
    .fetch_one(&mut *tx)
    .await
    .map_err(map_conflict)?;
    let body = json!({"id":row.0,"name":row.1,"steps":row.2,"createdAt":row.3});
    if let Some(key) = key {
        save_idempotent_response(
            &mut tx,
            (u, site),
            "create_funnel",
            key,
            StatusCode::CREATED,
            &body,
        )
        .await?;
    }
    tx.commit().await?;
    Ok((StatusCode::CREATED, Json(body)).into_response())
}

async fn delete_funnel(
    State(s): State<AppState>,
    CurrentUser(u): CurrentUser,
    Path((site, funnel_id)): Path<(Uuid, Uuid)>,
) -> Result<StatusCode, ApiError> {
    require_site(&s.pool, u, site, true).await?;
    sqlx::query("DELETE FROM funnels WHERE id=$1 AND site_id=$2")
        .bind(funnel_id)
        .bind(site)
        .execute(&s.pool)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn funnel_report(
    State(s): State<AppState>,
    CurrentUser(u): CurrentUser,
    Path((site, funnel_id)): Path<(Uuid, Uuid)>,
    Query(q): Query<RangeQuery>,
) -> Result<Json<Value>, ApiError> {
    require_site(&s.pool, u, site, false).await?;
    let ((a, b, _), _) = bounds(&s.pool, site, q.from, q.to).await?;
    let funnel: Option<(String, Value)> =
        sqlx::query_as("SELECT name,steps FROM funnels WHERE id=$1 AND site_id=$2")
            .bind(funnel_id)
            .bind(site)
            .fetch_optional(&s.pool)
            .await?;
    let (name, steps) = funnel.ok_or(ApiError::NotFound)?;
    let counts: Vec<(i32, i64)> = sqlx::query_as(
        "WITH RECURSIVE step_defs AS (
           SELECT ordinality::int step_index,step
           FROM jsonb_array_elements($4::jsonb) WITH ORDINALITY AS item(step,ordinality)
         ), progress(step_index,visitor_id,matched_at) AS (
           SELECT 1,e.visitor_id,min(e.occurred_at)
           FROM events e JOIN step_defs step ON step.step_index=1
           WHERE e.site_id=$1 AND e.occurred_at >= $2 AND e.occurred_at < $3
             AND e.traffic_class='human' AND (
               (step.step ? 'eventName' AND e.event_name=step.step->>'eventName') OR
               (step.step ? 'path' AND e.path=step.step->>'path')
             )
           GROUP BY e.visitor_id
           UNION ALL
           SELECT step.step_index,progress.visitor_id,next_event.matched_at
           FROM progress
           JOIN step_defs step ON step.step_index=progress.step_index+1
           CROSS JOIN LATERAL (
             SELECT min(e.occurred_at) matched_at FROM events e
             WHERE e.site_id=$1 AND e.visitor_id=progress.visitor_id
               AND e.occurred_at > progress.matched_at AND e.occurred_at < $3
               AND e.traffic_class='human' AND (
                 (step.step ? 'eventName' AND e.event_name=step.step->>'eventName') OR
                 (step.step ? 'path' AND e.path=step.step->>'path')
               )
           ) next_event
           WHERE next_event.matched_at IS NOT NULL
         )
         SELECT step_index,count(DISTINCT visitor_id)::bigint
         FROM progress GROUP BY step_index ORDER BY step_index",
    )
    .bind(site)
    .bind(a)
    .bind(b)
    .bind(&steps)
    .fetch_all(&s.pool)
    .await?;
    let step_values = steps.as_array().cloned().unwrap_or_default();
    let first_count = counts.first().map(|row| row.1).unwrap_or(0);
    let report_steps: Vec<Value> = step_values
        .into_iter()
        .enumerate()
        .map(|(index, step)| {
            let visitors = counts
                .iter()
                .find(|row| row.0 == index as i32 + 1)
                .map(|row| row.1)
                .unwrap_or(0);
            json!({
                "index":index+1,
                "label":step.get("label").and_then(Value::as_str).unwrap_or("Step"),
                "visitors":visitors,
                "conversionRate":if first_count == 0 {0.0} else {visitors as f64*100.0/first_count as f64}
            })
        })
        .collect();
    Ok(Json(json!({
        "id":funnel_id,"name":name,"from":q.from,"to":q.to,"steps":report_steps
    })))
}

fn validate_report_subscription(input: &ReportSubscriptionInput) -> Result<(), ApiError> {
    if input.name.trim().is_empty()
        || input.name.trim().len() > 120
        || !matches!(input.frequency.as_str(), "daily" | "weekly")
    {
        return Err(ApiError::BadRequest("invalid report subscription".into()));
    }
    validate_webhook_url(&input.webhook_url)
        .map_err(|message| ApiError::BadRequest(message.into()))?;
    Ok(())
}

async fn list_report_subscriptions(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(site): Path<Uuid>,
) -> Result<Json<Vec<ReportSubscription>>, ApiError> {
    require_site(&state.pool, user, site, false).await?;
    Ok(Json(
        sqlx::query_as(
            "SELECT id,site_id,name,webhook_url,frequency,anomaly_only,enabled,next_run_at,
           last_sent_at,last_status,last_error,created_at
         FROM report_subscriptions WHERE site_id=$1 ORDER BY created_at",
        )
        .bind(site)
        .fetch_all(&state.pool)
        .await?,
    ))
}

async fn create_report_subscription(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(site): Path<Uuid>,
    Json(input): Json<ReportSubscriptionInput>,
) -> Result<impl IntoResponse, ApiError> {
    require_site(&state.pool, user, site, true).await?;
    validate_report_subscription(&input)?;
    let subscription: ReportSubscription = sqlx::query_as(
        "INSERT INTO report_subscriptions(
           site_id,created_by,name,webhook_url,frequency,anomaly_only,enabled,next_run_at
         ) VALUES($1,$2,$3,$4,$5,$6,$7,now()+
           CASE $5 WHEN 'daily' THEN interval '1 day' ELSE interval '7 days' END)
         RETURNING id,site_id,name,webhook_url,frequency,anomaly_only,enabled,next_run_at,
           last_sent_at,last_status,last_error,created_at",
    )
    .bind(site)
    .bind(user)
    .bind(input.name.trim())
    .bind(input.webhook_url)
    .bind(input.frequency)
    .bind(input.anomaly_only)
    .bind(input.enabled.unwrap_or(true))
    .fetch_one(&state.pool)
    .await
    .map_err(map_conflict)?;
    let mut body = serde_json::to_value(&subscription).map_err(|_| ApiError::Internal)?;
    body.as_object_mut()
        .expect("subscription is object")
        .insert(
            "signingSecret".into(),
            json!(signing_secret(&state.identity_secret, subscription.id)),
        );
    Ok((StatusCode::CREATED, Json(body)))
}

async fn update_report_subscription(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path((site, subscription)): Path<(Uuid, Uuid)>,
    Json(input): Json<ReportSubscriptionInput>,
) -> Result<Json<ReportSubscription>, ApiError> {
    require_site(&state.pool, user, site, true).await?;
    validate_report_subscription(&input)?;
    let row: Option<ReportSubscription> = sqlx::query_as(
        "UPDATE report_subscriptions SET name=$3,webhook_url=$4,frequency=$5,
           anomaly_only=$6,enabled=COALESCE($7,enabled),updated_at=now()
         WHERE id=$1 AND site_id=$2
         RETURNING id,site_id,name,webhook_url,frequency,anomaly_only,enabled,next_run_at,
           last_sent_at,last_status,last_error,created_at",
    )
    .bind(subscription)
    .bind(site)
    .bind(input.name.trim())
    .bind(input.webhook_url)
    .bind(input.frequency)
    .bind(input.anomaly_only)
    .bind(input.enabled)
    .fetch_optional(&state.pool)
    .await
    .map_err(map_conflict)?;
    Ok(Json(row.ok_or(ApiError::NotFound)?))
}

async fn delete_report_subscription(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path((site, subscription)): Path<(Uuid, Uuid)>,
) -> Result<StatusCode, ApiError> {
    require_site(&state.pool, user, site, true).await?;
    let result = sqlx::query("DELETE FROM report_subscriptions WHERE id=$1 AND site_id=$2")
        .bind(subscription)
        .bind(site)
        .execute(&state.pool)
        .await?;
    if result.rows_affected() == 0 {
        return Err(ApiError::NotFound);
    }
    Ok(StatusCode::NO_CONTENT)
}

async fn deliver_report_subscription(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path((site, subscription)): Path<(Uuid, Uuid)>,
) -> Result<Json<Value>, ApiError> {
    require_site(&state.pool, user, site, true).await?;
    let exists: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM report_subscriptions WHERE id=$1 AND site_id=$2)",
    )
    .bind(subscription)
    .bind(site)
    .fetch_one(&state.pool)
    .await?;
    if !exists {
        return Err(ApiError::NotFound);
    }
    let status = deliver_report(&state.pool, &state.identity_secret, subscription).await?;
    Ok(Json(json!({"status":status})))
}

async fn list_report_deliveries(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path((site, subscription)): Path<(Uuid, Uuid)>,
) -> Result<Json<Vec<ReportDelivery>>, ApiError> {
    require_site(&state.pool, user, site, false).await?;
    Ok(Json(
        sqlx::query_as(
            "SELECT id,status,response_status,error,created_at FROM report_deliveries
         WHERE site_id=$1 AND subscription_id=$2 ORDER BY created_at DESC LIMIT 50",
        )
        .bind(site)
        .bind(subscription)
        .fetch_all(&state.pool)
        .await?,
    ))
}

fn search_console_config(state: &AppState) -> Result<&SearchConsoleConfig, ApiError> {
    state
        .search_console
        .as_deref()
        .ok_or_else(|| ApiError::BadRequest("Search Console integration is not configured".into()))
}

async fn connect_search_console(
    State(s): State<AppState>,
    CurrentUser(u): CurrentUser,
    Path(site): Path<Uuid>,
) -> Result<Json<Value>, ApiError> {
    require_site(&s.pool, u, site, true).await?;
    let config = search_console_config(&s)?;
    let state = oauth_state();
    sqlx::query(
        "INSERT INTO oauth_states(state_hash,user_id,site_id,expires_at)
         VALUES($1,$2,$3,now()+interval '10 minutes')",
    )
    .bind(state_hash(&state))
    .bind(u)
    .bind(site)
    .execute(&s.pool)
    .await?;
    Ok(Json(
        json!({"authorizationUrl":authorization_url(config,&state)}),
    ))
}

#[derive(Deserialize)]
struct GoogleTokenResponse {
    access_token: String,
    refresh_token: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct GoogleSitesResponse {
    #[serde(default)]
    site_entry: Vec<GoogleSiteEntry>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct GoogleSiteEntry {
    site_url: String,
}

async fn google_token_request(
    http: &reqwest::Client,
    config: &SearchConsoleConfig,
    parameters: &[(&str, &str)],
) -> Result<GoogleTokenResponse, ApiError> {
    let body = {
        let mut serializer = url::form_urlencoded::Serializer::new(String::new());
        serializer
            .append_pair("client_id", &config.client_id)
            .append_pair("client_secret", &config.client_secret);
        for (key, value) in parameters {
            serializer.append_pair(key, value);
        }
        serializer.finish()
    };
    let response = http
        .post("https://oauth2.googleapis.com/token")
        .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
        .body(body)
        .send()
        .await
        .map_err(|error| {
            tracing::warn!(%error, "Google token request failed");
            ApiError::Internal
        })?;
    if !response.status().is_success() {
        tracing::warn!(status=%response.status(), "Google token request rejected");
        return Err(ApiError::BadRequest(
            "Google authorization could not be completed".into(),
        ));
    }
    response.json().await.map_err(|error| {
        tracing::warn!(%error, "invalid Google token response");
        ApiError::Internal
    })
}

#[axum::debug_handler]
async fn search_console_callback(
    State(s): State<AppState>,
    Query(q): Query<SearchConsoleCallbackQuery>,
) -> Result<Redirect, ApiError> {
    let config = search_console_config(&s)?;
    let state_row: Option<(Uuid, Uuid)> = sqlx::query_as(
        "DELETE FROM oauth_states WHERE state_hash=$1 AND expires_at>now()
         RETURNING user_id,site_id",
    )
    .bind(state_hash(&q.state))
    .fetch_optional(&s.pool)
    .await?;
    let (user, site) =
        state_row.ok_or_else(|| ApiError::BadRequest("invalid or expired OAuth state".into()))?;
    let tokens = google_token_request(
        &s.http,
        config,
        &[
            ("code", &q.code),
            ("grant_type", "authorization_code"),
            ("redirect_uri", &config.redirect_uri),
        ],
    )
    .await?;
    let refresh_token = tokens
        .refresh_token
        .ok_or_else(|| ApiError::BadRequest("Google did not return an offline token".into()))?;
    let sites: GoogleSitesResponse = s
        .http
        .get("https://www.googleapis.com/webmasters/v3/sites")
        .bearer_auth(&tokens.access_token)
        .send()
        .await
        .map_err(|error| {
            tracing::warn!(%error, "Search Console property request failed");
            ApiError::Internal
        })?
        .error_for_status()
        .map_err(|error| {
            tracing::warn!(%error, "Search Console property request rejected");
            ApiError::BadRequest("Google Search Console access was rejected".into())
        })?
        .json()
        .await
        .map_err(|_| ApiError::Internal)?;
    let domain: String = sqlx::query_scalar("SELECT domain FROM sites WHERE id=$1")
        .bind(site)
        .fetch_one(&s.pool)
        .await?;
    let property_urls: Vec<String> = sites
        .site_entry
        .into_iter()
        .map(|entry| entry.site_url)
        .collect();
    let property = preferred_property(&domain, &property_urls);
    let encrypted =
        encrypt_token(&config.encryption_key, &refresh_token).map_err(|_| ApiError::Internal)?;
    let error = property
        .is_none()
        .then(|| "No matching Search Console property was found".to_string());
    sqlx::query(
        "INSERT INTO search_console_integrations(
           site_id,property_url,refresh_token_encrypted,connected_by,last_error
         ) VALUES($1,$2,$3,$4,$5)
         ON CONFLICT(site_id) DO UPDATE SET
           property_url=EXCLUDED.property_url,
           refresh_token_encrypted=EXCLUDED.refresh_token_encrypted,
           connected_by=EXCLUDED.connected_by,last_error=EXCLUDED.last_error,updated_at=now()",
    )
    .bind(site)
    .bind(property)
    .bind(encrypted)
    .bind(user)
    .bind(error.as_deref())
    .execute(&s.pool)
    .await?;
    let mut return_url = Url::parse(&config.return_uri).map_err(|_| ApiError::Internal)?;
    return_url
        .query_pairs_mut()
        .append_pair(
            "searchConsole",
            if error.is_some() {
                "warning"
            } else {
                "connected"
            },
        )
        .append_pair("site", &site.to_string());
    Ok(Redirect::to(return_url.as_str()))
}

async fn search_console_status(
    State(s): State<AppState>,
    CurrentUser(u): CurrentUser,
    Path(site): Path<Uuid>,
) -> Result<Json<Value>, ApiError> {
    require_site(&s.pool, u, site, false).await?;
    let configured = s.search_console.is_some();
    type SearchConsoleStatusRow = (Option<String>, Option<DateTime<Utc>>, Option<String>);
    let row: Option<SearchConsoleStatusRow> = sqlx::query_as(
        "SELECT property_url,last_synced_at,last_error
         FROM search_console_integrations WHERE site_id=$1",
    )
    .bind(site)
    .fetch_optional(&s.pool)
    .await?;
    Ok(Json(match row {
        Some(row) => json!({
            "configured":configured,"connected":true,"propertyUrl":row.0,
            "lastSyncedAt":row.1,"lastError":row.2
        }),
        None => json!({"configured":configured,"connected":false}),
    }))
}

async fn disconnect_search_console(
    State(s): State<AppState>,
    CurrentUser(u): CurrentUser,
    Path(site): Path<Uuid>,
) -> Result<StatusCode, ApiError> {
    require_site(&s.pool, u, site, true).await?;
    let mut tx = s.pool.begin().await?;
    sqlx::query("DELETE FROM search_console_metrics WHERE site_id=$1")
        .bind(site)
        .execute(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM search_console_integrations WHERE site_id=$1")
        .bind(site)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn refresh_google_access_token(
    state: &AppState,
    encrypted: &str,
) -> Result<String, ApiError> {
    let config = search_console_config(state)?;
    let refresh = decrypt_token(&config.encryption_key, encrypted).map_err(|error| {
        tracing::error!(error, "stored Search Console token could not be decrypted");
        ApiError::Internal
    })?;
    Ok(google_token_request(
        &state.http,
        config,
        &[("refresh_token", &refresh), ("grant_type", "refresh_token")],
    )
    .await?
    .access_token)
}

#[derive(Deserialize)]
struct SearchAnalyticsResponse {
    #[serde(default)]
    rows: Vec<SearchAnalyticsRow>,
}

#[derive(Deserialize)]
struct SearchAnalyticsRow {
    keys: Vec<String>,
    clicks: f64,
    impressions: f64,
    ctr: f64,
    position: f64,
}

#[axum::debug_handler]
async fn sync_search_console(
    State(s): State<AppState>,
    CurrentUser(u): CurrentUser,
    Path(site): Path<Uuid>,
    Query(q): Query<RangeQuery>,
) -> Result<Json<Value>, ApiError> {
    require_site(&s.pool, u, site, true).await?;
    let _: DateBounds = bounds(&s.pool, site, q.from, q.to).await?.0;
    let integration: Option<(Option<String>, String)> = sqlx::query_as(
        "SELECT property_url,refresh_token_encrypted
         FROM search_console_integrations WHERE site_id=$1",
    )
    .bind(site)
    .fetch_optional(&s.pool)
    .await?;
    let (property, encrypted) = integration.ok_or(ApiError::NotFound)?;
    let property = property
        .ok_or_else(|| ApiError::BadRequest("No matching Search Console property".into()))?;
    let access_token = refresh_google_access_token(&s, &encrypted).await?;
    let url = format!(
        "https://www.googleapis.com/webmasters/v3/sites/{}/searchAnalytics/query",
        url::form_urlencoded::byte_serialize(property.as_bytes()).collect::<String>()
    );
    let mut rows = Vec::new();
    for page in 0..10 {
        let response = s
            .http
            .post(&url)
            .bearer_auth(&access_token)
            .json(&json!({
                "startDate":q.from,"endDate":q.to,
                "dimensions":["date","query","page","country","device"],
                "rowLimit":25000,"startRow":page*25000,
                "dataState":"all"
            }))
            .send()
            .await
            .map_err(|error| {
                tracing::warn!(%error, "Search Console sync request failed");
                ApiError::Internal
            })?
            .error_for_status()
            .map_err(|error| {
                tracing::warn!(%error, "Search Console sync rejected");
                ApiError::BadRequest("Search Console sync was rejected".into())
            })?
            .json::<SearchAnalyticsResponse>()
            .await
            .map_err(|_| ApiError::Internal)?;
        let count = response.rows.len();
        rows.extend(response.rows);
        if count < 25_000 {
            break;
        }
    }
    let mut tx = s.pool.begin().await?;
    sqlx::query(
        "DELETE FROM search_console_metrics
         WHERE site_id=$1 AND metric_date BETWEEN $2 AND $3",
    )
    .bind(site)
    .bind(q.from)
    .bind(q.to)
    .execute(&mut *tx)
    .await?;
    for row in &rows {
        if row.keys.len() != 5 {
            continue;
        }
        let date = chrono::NaiveDate::parse_from_str(&row.keys[0], "%Y-%m-%d")
            .map_err(|_| ApiError::Internal)?;
        sqlx::query(
            "INSERT INTO search_console_metrics(
               site_id,metric_date,query,page,country,device,clicks,impressions,ctr,position
             ) VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,$10)
             ON CONFLICT(site_id,metric_date,query,page,country,device) DO UPDATE SET
               clicks=EXCLUDED.clicks,impressions=EXCLUDED.impressions,ctr=EXCLUDED.ctr,
               position=EXCLUDED.position,synced_at=now()",
        )
        .bind(site)
        .bind(date)
        .bind(&row.keys[1])
        .bind(&row.keys[2])
        .bind(&row.keys[3])
        .bind(&row.keys[4])
        .bind(row.clicks)
        .bind(row.impressions)
        .bind(row.ctr)
        .bind(row.position)
        .execute(&mut *tx)
        .await?;
    }
    sqlx::query(
        "UPDATE search_console_integrations
         SET last_synced_at=now(),last_error=NULL,updated_at=now() WHERE site_id=$1",
    )
    .bind(site)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(Json(json!({"status":"ok","rows":rows.len()})))
}

async fn search_console_report(
    State(s): State<AppState>,
    CurrentUser(u): CurrentUser,
    Path(site): Path<Uuid>,
    Query(q): Query<SearchConsoleReportQuery>,
) -> Result<Json<Vec<Value>>, ApiError> {
    require_site(&s.pool, u, site, false).await?;
    if q.to < q.from || (q.to - q.from).num_days() > 366 {
        return Err(ApiError::BadRequest("invalid date range".into()));
    }
    let dimension = match q.dimension.as_str() {
        "query" => "query",
        "page" => "page",
        "country" => "country",
        "device" => "device",
        "date" => "metric_date::text",
        _ => return Err(ApiError::BadRequest("invalid dimension".into())),
    };
    let sql = format!(
        "SELECT {dimension}::text,sum(clicks)::float8,sum(impressions)::float8,
         CASE WHEN sum(impressions)=0 THEN 0 ELSE sum(clicks)/sum(impressions) END::float8,
         CASE WHEN sum(impressions)=0 THEN 0
              ELSE sum(position*impressions)/sum(impressions) END::float8
         FROM search_console_metrics
         WHERE site_id=$1 AND metric_date BETWEEN $2 AND $3
         GROUP BY 1 ORDER BY sum(clicks) DESC LIMIT $4"
    );
    let rows: Vec<(String, f64, f64, f64, f64)> = sqlx::query_as(&sql)
        .bind(site)
        .bind(q.from)
        .bind(q.to)
        .bind(q.limit.unwrap_or(100).clamp(1, 500))
        .fetch_all(&s.pool)
        .await?;
    Ok(Json(
        rows.into_iter()
            .map(|row| {
                json!({"value":row.0,"clicks":row.1,"impressions":row.2,"ctr":row.3,"position":row.4})
            })
            .collect(),
    ))
}

#[derive(Deserialize)]
struct McpRequest {
    jsonrpc: String,
    id: Option<Value>,
    method: String,
    #[serde(default)]
    params: Value,
}

fn mcp_response(id: Option<Value>, result: Value) -> Value {
    json!({"jsonrpc":"2.0","id":id.unwrap_or(Value::Null),"result":result})
}

fn mcp_protocol_error(id: Option<Value>, code: i32, message: &str) -> Value {
    json!({"jsonrpc":"2.0","id":id.unwrap_or(Value::Null),"error":{"code":code,"message":message}})
}

fn mcp_tool_result(data: Value, is_error: bool) -> Value {
    let text = serde_json::to_string_pretty(&data).unwrap_or_else(|_| "{}".into());
    json!({
        "content":[{"type":"text","text":text}],
        "structuredContent":data,
        "isError":is_error
    })
}

fn mcp_tools() -> Value {
    json!([
      {"name":"setup_site","description":"Create or reuse a site by domain and return first-party tracker installation configuration. Install the returned proxy routes and script in the website, preserving consent and DNT.","inputSchema":{"type":"object","required":["name","domain"],"additionalProperties":false,"properties":{"name":{"type":"string"},"domain":{"type":"string"},"timezone":{"type":"string","default":"UTC"},"allowedOrigins":{"type":"array","items":{"type":"string"}},"retentionDays":{"type":"integer","default":365},"serverType":{"type":"string","enum":["caddy","nginx","apache"]}}},"annotations":{"readOnlyHint":false,"destructiveHint":false,"idempotentHint":true}},
      {"name":"tracking_setup","description":"Get first-party script, reverse proxy configuration and verification URLs for an existing site.","inputSchema":{"type":"object","required":["siteId"],"additionalProperties":false,"properties":{"siteId":{"type":"string","format":"uuid"}}},"annotations":{"readOnlyHint":true}},
      {
        "name":"list_sites",
        "description":"List analytics sites available to this account. Does not expose collection write keys.",
        "inputSchema":{"type":"object","additionalProperties":false},
        "outputSchema":{"type":"object","required":["sites"],"properties":{"sites":{"type":"array"}}},
        "annotations":{"readOnlyHint":true,"idempotentHint":true}
      },
      {
        "name":"analytics_summary",
        "description":"Get deterministic overview metrics, comparisons, top pages, definitions, and data freshness for one site and explicit inclusive dates.",
        "inputSchema":{
          "type":"object","required":["siteId","from","to"],"additionalProperties":false,
          "properties":{
            "siteId":{"type":"string","format":"uuid"},
            "from":{"type":"string","format":"date"},
            "to":{"type":"string","format":"date"}
          }
        },
        "outputSchema":{"type":"object"},
        "annotations":{"readOnlyHint":true,"idempotentHint":true}
      },
      {
        "name":"dimension_report",
        "description":"Rank a stable analytics dimension for one site and explicit inclusive dates.",
        "inputSchema":{
          "type":"object","required":["siteId","from","to","dimension"],"additionalProperties":false,
          "properties":{
            "siteId":{"type":"string","format":"uuid"},
            "from":{"type":"string","format":"date"},
            "to":{"type":"string","format":"date"},
            "dimension":{"type":"string","enum":["page","referrer","country","region","city","device","browser","os","campaign","source","medium","content"]},
            "limit":{"type":"integer","minimum":1,"maximum":500,"default":100}
          }
        },
        "outputSchema":{"type":"object"},
        "annotations":{"readOnlyHint":true,"idempotentHint":true}
      },
      {
        "name":"search_console_report",
        "description":"Get cached Google Search Console clicks, impressions, CTR, and weighted position with explicit dates.",
        "inputSchema":{
          "type":"object","required":["siteId","from","to"],"additionalProperties":false,
          "properties":{
            "siteId":{"type":"string","format":"uuid"},
            "from":{"type":"string","format":"date"},
            "to":{"type":"string","format":"date"},
            "dimension":{"type":"string","enum":["query","page","country","device","date"],"default":"query"},
            "limit":{"type":"integer","minimum":1,"maximum":500,"default":100}
          }
        },
        "outputSchema":{"type":"object"},
        "annotations":{"readOnlyHint":true,"idempotentHint":true}
      },
      {
        "name":"marketing_brief",
        "description":"Generate the same evidence-rich completed-day marketing brief used by scheduled webhook delivery.",
        "inputSchema":{
          "type":"object","required":["siteId"],"additionalProperties":false,
          "properties":{
            "siteId":{"type":"string","format":"uuid"},
            "days":{"type":"integer","minimum":1,"maximum":90,"default":7}
          }
        },
        "outputSchema":{"type":"object"},
        "annotations":{"readOnlyHint":true,"idempotentHint":true}
      }
    ])
}

fn mcp_argument<'a>(arguments: &'a Value, name: &str) -> Result<&'a str, ApiError> {
    arguments
        .get(name)
        .and_then(Value::as_str)
        .ok_or_else(|| ApiError::BadRequest(format!("{name} is required")))
}

fn mcp_site_and_range(
    arguments: &Value,
) -> Result<(Uuid, chrono::NaiveDate, chrono::NaiveDate), ApiError> {
    let site = Uuid::parse_str(mcp_argument(arguments, "siteId")?)
        .map_err(|_| ApiError::BadRequest("invalid siteId".into()))?;
    let from = chrono::NaiveDate::parse_from_str(mcp_argument(arguments, "from")?, "%Y-%m-%d")
        .map_err(|_| ApiError::BadRequest("invalid from date".into()))?;
    let to = chrono::NaiveDate::parse_from_str(mcp_argument(arguments, "to")?, "%Y-%m-%d")
        .map_err(|_| ApiError::BadRequest("invalid to date".into()))?;
    Ok((site, from, to))
}

async fn mcp_analytics_summary(
    state: &AppState,
    user: Uuid,
    arguments: &Value,
) -> Result<(Uuid, Value), ApiError> {
    let (site, from, to) = mcp_site_and_range(arguments)?;
    require_site(&state.pool, user, site, false).await?;
    let ((start, end, prior), timezone) = bounds(&state.pool, site, from, to).await?;
    let current = counts(&state.pool, site, start, end).await?;
    let previous = counts(&state.pool, site, prior, start).await?;
    let engagement: (f64, f64) = sqlx::query_as(
        "SELECT
           COALESCE(100.0*count(*) FILTER(WHERE page_views=1)/NULLIF(count(*),0),0)::float8,
           COALESCE(avg(duration_seconds),0)::float8
         FROM (
           SELECT session_id,count(*) FILTER(WHERE event_name='pageview') page_views,
             EXTRACT(EPOCH FROM max(occurred_at)-min(occurred_at)) duration_seconds
           FROM events WHERE site_id=$1 AND occurred_at >= $2 AND occurred_at < $3
             AND traffic_class='human' GROUP BY session_id
         ) sessions WHERE page_views>0",
    )
    .bind(site)
    .bind(start)
    .bind(end)
    .fetch_one(&state.pool)
    .await?;
    let top_pages: Vec<ReportRow> = sqlx::query_as(
        "SELECT path value,count(*)::bigint views,count(DISTINCT visitor_id)::bigint visitors
         FROM events WHERE site_id=$1 AND occurred_at >= $2 AND occurred_at < $3
           AND traffic_class='human' AND event_name='pageview'
         GROUP BY path ORDER BY views DESC LIMIT 10",
    )
    .bind(site)
    .bind(start)
    .bind(end)
    .fetch_all(&state.pool)
    .await?;
    let data_through: Option<DateTime<Utc>> =
        sqlx::query_scalar("SELECT max(received_at) FROM events WHERE site_id=$1")
            .bind(site)
            .fetch_one(&state.pool)
            .await?;
    Ok((
        site,
        json!({
          "siteId":site,"from":from,"to":to,"timezone":timezone,
          "generatedAt":Utc::now(),"dataThrough":data_through,
          "metrics":{
            "pageViews":metric(current.0,previous.0),
            "visitors":metric(current.1,previous.1),
            "sessions":metric(current.2,previous.2),
            "customEvents":metric(current.3,previous.3),
            "bounceRate":engagement.0,
            "averageSessionDurationSeconds":engagement.1
          },
          "topPages":top_pages,
          "definitions":{
            "visitor":"Distinct site-scoped rotating visitor identifier among human-classified events.",
            "session":"Distinct 30-minute-window session identifier among human-classified events.",
            "bounceRate":"Percent of human sessions with exactly one page view.",
            "comparison":"Immediately preceding date range with the same number of site-local calendar days."
          },
          "evidence":{
            "source":"Slimlytics event store",
            "trafficClass":"human",
            "dateBoundary":"site-local inclusive dates",
            "freshnessField":"dataThrough"
          }
        }),
    ))
}

async fn mcp_dimension_report(
    state: &AppState,
    user: Uuid,
    arguments: &Value,
) -> Result<(Uuid, Value), ApiError> {
    let (site, from, to) = mcp_site_and_range(arguments)?;
    require_site(&state.pool, user, site, false).await?;
    let ((start, end, _), timezone) = bounds(&state.pool, site, from, to).await?;
    let dimension = mcp_argument(arguments, "dimension")?;
    let column = match dimension {
        "page" => "path",
        "referrer" => "COALESCE(referrer_host,'(direct)')",
        "country" => "COALESCE(country_code,'unknown')",
        "region" => "COALESCE(region,'unknown')",
        "city" => "COALESCE(city,'unknown')",
        "device" => "COALESCE(device_type,'unknown')",
        "browser" => "COALESCE(browser,'unknown')",
        "os" => "COALESCE(os,'unknown')",
        "campaign" => "COALESCE(utm_campaign,'(none)')",
        "source" => "COALESCE(utm_source,referrer_host,'(direct)')",
        "medium" => "COALESCE(utm_medium,'(none)')",
        "content" => "COALESCE(content_id,'(not set)')",
        _ => return Err(ApiError::BadRequest("invalid dimension".into())),
    };
    let limit = arguments
        .get("limit")
        .and_then(Value::as_i64)
        .unwrap_or(100)
        .clamp(1, 500);
    let sql = format!(
        "SELECT {column}::text value,count(*)::bigint views,
         count(DISTINCT visitor_id)::bigint visitors
         FROM events WHERE site_id=$1 AND occurred_at >= $2 AND occurred_at < $3
           AND traffic_class='human'
         GROUP BY 1 ORDER BY views DESC LIMIT $4"
    );
    let rows: Vec<ReportRow> = sqlx::query_as(&sql)
        .bind(site)
        .bind(start)
        .bind(end)
        .bind(limit)
        .fetch_all(&state.pool)
        .await?;
    Ok((
        site,
        json!({
          "siteId":site,"from":from,"to":to,"timezone":timezone,
          "generatedAt":Utc::now(),"dimension":dimension,"rows":rows,
          "evidence":{"source":"Slimlytics event store","trafficClass":"human"}
        }),
    ))
}

async fn mcp_search_console_report(
    state: &AppState,
    user: Uuid,
    arguments: &Value,
) -> Result<(Uuid, Value), ApiError> {
    let (site, from, to) = mcp_site_and_range(arguments)?;
    require_site(&state.pool, user, site, false).await?;
    let dimension = arguments
        .get("dimension")
        .and_then(Value::as_str)
        .unwrap_or("query");
    let column = match dimension {
        "query" => "query",
        "page" => "page",
        "country" => "country",
        "device" => "device",
        "date" => "metric_date::text",
        _ => return Err(ApiError::BadRequest("invalid dimension".into())),
    };
    let limit = arguments
        .get("limit")
        .and_then(Value::as_i64)
        .unwrap_or(100)
        .clamp(1, 500);
    let sql = format!(
        "SELECT {column}::text,sum(clicks)::float8,sum(impressions)::float8,
         CASE WHEN sum(impressions)=0 THEN 0 ELSE sum(clicks)/sum(impressions) END::float8,
         CASE WHEN sum(impressions)=0 THEN 0 ELSE sum(position*impressions)/sum(impressions) END::float8
         FROM search_console_metrics WHERE site_id=$1 AND metric_date BETWEEN $2 AND $3
         GROUP BY 1 ORDER BY sum(clicks) DESC LIMIT $4"
    );
    let rows: Vec<(String, f64, f64, f64, f64)> = sqlx::query_as(&sql)
        .bind(site)
        .bind(from)
        .bind(to)
        .bind(limit)
        .fetch_all(&state.pool)
        .await?;
    let last_synced_at: Option<DateTime<Utc>> = sqlx::query_scalar(
        "SELECT last_synced_at FROM search_console_integrations WHERE site_id=$1",
    )
    .bind(site)
    .fetch_optional(&state.pool)
    .await?
    .flatten();
    Ok((
        site,
        json!({
          "siteId":site,"from":from,"to":to,"dimension":dimension,
          "generatedAt":Utc::now(),"dataThrough":last_synced_at,
          "rows":rows.into_iter().map(|row|json!({
            "value":row.0,"clicks":row.1,"impressions":row.2,"ctr":row.3,"position":row.4
          })).collect::<Vec<_>>(),
          "evidence":{"source":"Google Search Console cached sync","freshnessField":"dataThrough"}
        }),
    ))
}

async fn record_agent_audit(
    state: &AppState,
    principal: &AgentUser,
    site: Option<Uuid>,
    action: &str,
    request_id: Option<&str>,
    input: &Value,
    outcome: &str,
) {
    if let Err(error) = sqlx::query(
        "INSERT INTO agent_audit_log(
           user_id,api_token_id,site_id,action,request_id,input,outcome
         ) VALUES($1,$2,$3,$4,$5,$6,$7)",
    )
    .bind(principal.user_id)
    .bind(principal.api_token_id)
    .bind(site)
    .bind(action)
    .bind(request_id)
    .bind(input)
    .bind(outcome)
    .execute(&state.pool)
    .await
    {
        tracing::warn!(%error, action, "failed to write agent audit record");
    }
}

async fn mcp_challenge(State(state): State<AppState>, mut response: Response) -> Response {
    if response.status() == StatusCode::UNAUTHORIZED {
        if let Ok(value) = format!(
            "Bearer resource_metadata=\"{}/.well-known/oauth-protected-resource/api/mcp\"",
            state.public_url
        )
        .parse()
        {
            response
                .headers_mut()
                .insert(header::WWW_AUTHENTICATE, value);
        }
    }
    response
}
async fn mcp(
    State(state): State<AppState>,
    principal: AgentUser,
    headers: HeaderMap,
    Json(request): Json<McpRequest>,
) -> Result<Response, ApiError> {
    if headers.get(header::ORIGIN).is_some_and(|origin| {
        origin
            .to_str()
            .map_or(true, |value| value != state.public_url.as_str())
    }) {
        return Err(ApiError::Forbidden);
    }
    if request.jsonrpc != "2.0" {
        return Ok(Json(mcp_protocol_error(
            request.id,
            -32600,
            "Invalid JSON-RPC request",
        ))
        .into_response());
    }
    if request.method == "initialize" {
        let protocol = match request
            .params
            .get("protocolVersion")
            .and_then(Value::as_str)
        {
            Some("2025-03-26") => "2025-03-26",
            Some("2025-06-18") => "2025-06-18",
            _ => "2025-11-25",
        };
        return Ok(Json(mcp_response(
            request.id,
            json!({
              "protocolVersion":protocol,
              "capabilities":{"tools":{"listChanged":false}},
              "serverInfo":{
                "name":"slimlytics","title":"Slimlytics Analytics","version":"1.0.0",
                "description":"Deterministic privacy-minded web and marketing analytics."
              },
              "instructions":"For website installation call setup_site with name, domain and optional serverType (caddy, nginx, apache). Install returned first-party proxy routes and snippet in the website, then verify both URLs and a page view. Preserve consent, DNT and GPC. Repeated setup reuses the domain. Use explicit site IDs and inclusive date ranges for analytics; report dataThrough and evidence."
            }),
        )).into_response());
    }
    if request.method == "notifications/initialized" {
        return Ok(StatusCode::ACCEPTED.into_response());
    }
    if request.method == "ping" {
        return Ok(Json(mcp_response(request.id, json!({}))).into_response());
    }
    if request.method == "tools/list" {
        return Ok(Json(mcp_response(request.id, json!({"tools":mcp_tools()}))).into_response());
    }
    if request.method != "tools/call" {
        return Ok(
            Json(mcp_protocol_error(request.id, -32601, "Method not found")).into_response(),
        );
    }
    let name = request
        .params
        .get("name")
        .and_then(Value::as_str)
        .ok_or_else(|| ApiError::BadRequest("tool name is required".into()))?;
    let arguments = request
        .params
        .get("arguments")
        .cloned()
        .unwrap_or_else(|| json!({}));
    let request_id = headers
        .get("x-request-id")
        .and_then(|value| value.to_str().ok());
    let scope = match name {
        "setup_site" => "sites:write",
        "tracking_setup" | "list_sites" => SITES_READ,
        _ => ANALYTICS_READ,
    };
    principal.require(scope)?;
    let result: Result<(Option<Uuid>, Value), ApiError> = async {
        match name {
            "setup_site" => {
                if arguments.get("serverType").is_some_and(|value| {
                    !matches!(value.as_str(), Some("caddy" | "nginx" | "apache"))
                }) {
                    return Err(ApiError::BadRequest("invalid serverType".into()));
                }
                let input: SiteInput = serde_json::from_value(arguments.clone())
                    .map_err(|_| ApiError::BadRequest("invalid site input".into()))?;
                let (_, Json(ensured)) = ensure_site(
                    State(state.clone()),
                    CurrentUser(principal.user_id),
                    Json(input),
                )
                .await?;
                let mut site = ensured.site;
                require_site(&state.pool, principal.user_id, site.id, true).await?;
                if let Some(server) = arguments.get("serverType").and_then(Value::as_str) {
                    let Json(updated) = update_anti_adblock(
                        State(state.clone()),
                        CurrentUser(principal.user_id),
                        Path(site.id),
                        Json(AntiAdblockInput {
                            server_type: server.into(),
                            js_path: site.anti_adblock_js_path.clone(),
                            beacon_path: site.anti_adblock_beacon_path.clone(),
                        }),
                    )
                    .await?;
                    site = updated;
                }
                let setup = tracking_setup::tracking_setup(&site, &state.public_url)
                    .map_err(|_| ApiError::Internal)?;
                Ok((
                    Some(site.id),
                    json!({"created":ensured.created,"setup":setup}),
                ))
            }
            "tracking_setup" => {
                let id = Uuid::parse_str(mcp_argument(&arguments, "siteId")?)
                    .map_err(|_| ApiError::BadRequest("invalid siteId".into()))?;
                require_site(&state.pool, principal.user_id, id, false).await?;
                let site = fetch_site(&state.pool, id).await?;
                Ok((
                    Some(id),
                    serde_json::to_value(
                        tracking_setup::tracking_setup(&site, &state.public_url)
                            .map_err(|_| ApiError::Internal)?,
                    )
                    .map_err(|_| ApiError::Internal)?,
                ))
            }
            "list_sites" => {
                principal.require(SITES_READ)?;
                let sites: Vec<(Uuid, String, String, String)> = sqlx::query_as(
                    "SELECT s.id,s.name,s.domain,s.timezone FROM sites s
                 JOIN site_memberships m ON m.site_id=s.id
                 WHERE m.user_id=$1 ORDER BY s.created_at",
                )
                .bind(principal.user_id)
                .fetch_all(&state.pool)
                .await?;
                Ok((
                    None,
                    json!({"sites":sites.into_iter().map(|row|json!({
                  "id":row.0,"name":row.1,"domain":row.2,"timezone":row.3
                })).collect::<Vec<_>>(),"generatedAt":Utc::now()}),
                ))
            }
            "analytics_summary" => mcp_analytics_summary(&state, principal.user_id, &arguments)
                .await
                .map(|(site, value)| (Some(site), value)),
            "dimension_report" => mcp_dimension_report(&state, principal.user_id, &arguments)
                .await
                .map(|(site, value)| (Some(site), value)),
            "search_console_report" => {
                principal.require("integrations:read")?;
                mcp_search_console_report(&state, principal.user_id, &arguments)
                    .await
                    .map(|(site, value)| (Some(site), value))
            }
            "marketing_brief" => {
                let site = Uuid::parse_str(mcp_argument(&arguments, "siteId")?)
                    .map_err(|_| ApiError::BadRequest("invalid siteId".into()))?;
                require_site(&state.pool, principal.user_id, site, false).await?;
                let days = arguments
                    .get("days")
                    .and_then(Value::as_i64)
                    .unwrap_or(7)
                    .clamp(1, 90);
                build_marketing_brief(&state.pool, site, days)
                    .await
                    .map(|value| (Some(site), value))
                    .map_err(ApiError::Database)
            }
            _ => Err(ApiError::BadRequest("unknown tool".into())),
        }
    }
    .await;
    let response = match result {
        Ok((site, data)) => {
            record_agent_audit(
                &state, &principal, site, name, request_id, &arguments, "success",
            )
            .await;
            mcp_tool_result(data, false)
        }
        Err(error) => {
            record_agent_audit(
                &state, &principal, None, name, request_id, &arguments, "error",
            )
            .await;
            mcp_tool_result(json!({"error":error.to_string()}), true)
        }
    };
    Ok(Json(mcp_response(request.id, response)).into_response())
}
async fn visitor_timeline(
    State(s): State<AppState>,
    CurrentUser(u): CurrentUser,
    Path((site, visitor)): Path<(Uuid, String)>,
) -> Result<Json<Vec<Value>>, ApiError> {
    require_site(&s.pool, u, site, false).await?;
    let rows:Vec<(Uuid,DateTime<Utc>,String,String,Value)>=sqlx::query_as("SELECT id,occurred_at,event_name,path,properties FROM events WHERE site_id=$1 AND visitor_id=$2 ORDER BY occurred_at DESC LIMIT 500").bind(site).bind(visitor).fetch_all(&s.pool).await?;
    Ok(Json(
        rows.into_iter()
            .map(|r| json!({"id":r.0,"occurred_at":r.1,"name":r.2,"path":r.3,"properties":r.4}))
            .collect(),
    ))
}
async fn list_visitors(
    State(s): State<AppState>,
    CurrentUser(u): CurrentUser,
    Path(site): Path<Uuid>,
    Query(q): Query<RangeQuery>,
) -> Result<Json<Vec<Value>>, ApiError> {
    require_site(&s.pool, u, site, false).await?;
    let ((a, b, _), _) = bounds(&s.pool, site, q.from, q.to).await?;
    type VisitorRow = (
        String,
        Option<String>,
        Option<String>,
        Option<String>,
        Option<String>,
        Option<String>,
        String,
        DateTime<Utc>,
        i64,
    );
    let rows: Vec<VisitorRow> = sqlx::query_as(
        "SELECT visitor_id, max(country_code)::text, max(region), max(city), max(device_type), \
         max(browser), \
         (array_agg(path ORDER BY occurred_at DESC))[1], max(occurred_at), \
         count(DISTINCT session_id) \
         FROM events WHERE site_id=$1 AND occurred_at >= $2 AND occurred_at < $3 \
         AND traffic_class='human' GROUP BY visitor_id ORDER BY max(occurred_at) DESC LIMIT 500",
    )
    .bind(site)
    .bind(a)
    .bind(b)
    .fetch_all(&s.pool)
    .await?;
    Ok(Json(
        rows.into_iter()
            .map(|row| {
                json!({
                    "id": row.0,
                    "country": row.1.unwrap_or_else(|| "Unknown".into()),
                    "region": row.2,
                    "city": row.3,
                    "device": row.4,
                    "browser": row.5,
                    "page": row.6,
                    "lastSeen": row.7,
                    "sessions": row.8
                })
            })
            .collect(),
    ))
}
async fn custom_events(
    State(s): State<AppState>,
    CurrentUser(u): CurrentUser,
    Path(site): Path<Uuid>,
    Query(q): Query<RangeQuery>,
) -> Result<Json<Vec<Value>>, ApiError> {
    require_site(&s.pool, u, site, false).await?;
    let ((a, b, _), _) = bounds(&s.pool, site, q.from, q.to).await?;
    type EventRow = (
        Uuid,
        DateTime<Utc>,
        String,
        String,
        String,
        Option<String>,
        Option<String>,
    );
    let rows: Vec<EventRow> = sqlx::query_as(
        "SELECT id,occurred_at,event_name,path,visitor_id,country_code::text,referrer_host \
         FROM events WHERE site_id=$1 AND occurred_at >= $2 AND occurred_at < $3 \
         ORDER BY occurred_at DESC LIMIT 500",
    )
    .bind(site)
    .bind(a)
    .bind(b)
    .fetch_all(&s.pool)
    .await?;
    Ok(Json(
        rows.into_iter()
            .map(|row| {
                json!({
                    "id": row.0,
                    "timestamp": row.1,
                    "type": row.2,
                    "page": row.3,
                    "visitorId": row.4,
                    "country": row.5,
                    "referrer": row.6
                })
            })
            .collect(),
    ))
}
async fn list_goals(
    State(s): State<AppState>,
    CurrentUser(u): CurrentUser,
    Path(site): Path<Uuid>,
) -> Result<Json<Vec<GoalWithStats>>, ApiError> {
    require_site(&s.pool, u, site, false).await?;
    Ok(Json(
        sqlx::query_as(
            "SELECT g.id,g.site_id,g.name,g.event_name,g.path_pattern,g.created_at, \
             (SELECT count(*) FROM goal_completions gc JOIN events e ON e.id=gc.event_id \
              WHERE gc.goal_id=g.id AND e.traffic_class='human')::bigint conversions, \
             COALESCE(100.0 * \
               (SELECT count(DISTINCT gc.visitor_id) FROM goal_completions gc \
                JOIN events e ON e.id=gc.event_id WHERE gc.goal_id=g.id AND e.traffic_class='human') / \
               NULLIF((SELECT count(DISTINCT visitor_id) FROM events \
                       WHERE site_id=g.site_id AND traffic_class='human'),0),0)::float8 conversion_rate \
             FROM goals g WHERE g.site_id=$1 ORDER BY g.created_at",
        )
        .bind(site)
        .fetch_all(&s.pool)
        .await?,
    ))
}
async fn create_goal(
    State(s): State<AppState>,
    CurrentUser(u): CurrentUser,
    Path(site): Path<Uuid>,
    Json(i): Json<GoalInput>,
) -> Result<impl IntoResponse, ApiError> {
    require_site(&s.pool, u, site, true).await?;
    if i.name.is_empty() || i.event_name.is_empty() {
        return Err(ApiError::BadRequest("name and event_name required".into()));
    }
    let goal:Goal=sqlx::query_as("INSERT INTO goals(site_id,name,event_name,path_pattern) VALUES($1,$2,$3,$4) RETURNING id,site_id,name,event_name,path_pattern,created_at").bind(site).bind(i.name).bind(i.event_name).bind(i.path_pattern).fetch_one(&s.pool).await.map_err(map_conflict)?;
    Ok((StatusCode::CREATED, Json(goal)))
}
async fn delete_goal(
    State(s): State<AppState>,
    CurrentUser(u): CurrentUser,
    Path((site, goal)): Path<(Uuid, Uuid)>,
) -> Result<StatusCode, ApiError> {
    require_site(&s.pool, u, site, true).await?;
    sqlx::query("DELETE FROM goals WHERE id=$1 AND site_id=$2")
        .bind(goal)
        .bind(site)
        .execute(&s.pool)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}
async fn export_csv(
    State(s): State<AppState>,
    CurrentUser(u): CurrentUser,
    Path(site): Path<Uuid>,
    Query(q): Query<RangeQuery>,
) -> Result<impl IntoResponse, ApiError> {
    require_site(&s.pool, u, site, false).await?;
    let ((a, b, _), _) = bounds(&s.pool, site, q.from, q.to).await?;
    let rows: Vec<CsvRow> = sqlx::query_as("SELECT occurred_at,event_name,path,visitor_id,referrer_host,utm_campaign FROM events WHERE site_id=$1 AND occurred_at >= $2 AND occurred_at < $3 ORDER BY occurred_at").bind(site).bind(a).bind(b).fetch_all(&s.pool).await?;
    let mut w = csv::Writer::from_writer(vec![]);
    w.write_record([
        "occurred_at",
        "event_name",
        "path",
        "visitor_id",
        "referrer",
        "campaign",
    ])
    .map_err(|_| ApiError::Internal)?;
    for r in rows {
        w.serialize(r).map_err(|_| ApiError::Internal)?
    }
    let data = w.into_inner().map_err(|_| ApiError::Internal)?;
    Ok((
        [
            (header::CONTENT_TYPE, "text/csv; charset=utf-8"),
            (
                header::CONTENT_DISPOSITION,
                "attachment; filename=events.csv",
            ),
        ],
        data,
    ))
}
async fn stream(
    State(s): State<AppState>,
    Path(site): Path<Uuid>,
    headers: HeaderMap,
    Query(q): Query<StreamQuery>,
) -> Result<Sse<impl tokio_stream::Stream<Item = Result<Event, Infallible>>>, ApiError> {
    let token = q.token.as_deref().ok_or(ApiError::Unauthorized)?;
    // EventSource cannot set Authorization; accept session JWTs or personal API tokens.
    let u = if token.starts_with("slyt_") {
        sqlx::query_scalar(
            "UPDATE api_tokens SET last_used_at=now() WHERE token_hash=$1 AND revoked_at IS NULL AND expires_at>now() AND oauth_resource IS NULL RETURNING user_id",
        )
        .bind(hash_api_token(token))
        .fetch_optional(&s.pool)
        .await?
        .ok_or(ApiError::Unauthorized)?
    } else {
        authenticate_jwt(&s, token).await?.user_id
    };
    require_site(&s.pool, u, site, false).await?;
    let last = q
        .last_event_id
        .or_else(|| {
            headers
                .get("last-event-id")
                .and_then(|v| v.to_str().ok())?
                .parse()
                .ok()
        })
        .unwrap_or(0);
    let replay: Vec<(i64, Value)> = sqlx::query_as(
        "SELECT id,payload FROM stream_events WHERE site_id=$1 AND id>$2 ORDER BY id LIMIT 1001",
    )
    .bind(site)
    .bind(last)
    .fetch_all(&s.pool)
    .await?;
    let resync = replay.len() > 1000;
    let replay = if resync { Vec::new() } else { replay };
    let rx = s.stream_tx.subscribe();
    let initial = tokio_stream::iter(replay.into_iter().map(|(id, payload)| {
        Ok(Event::default()
            .id(id.to_string())
            .event("event")
            .json_data(payload)
            .unwrap())
    }));
    let marker = tokio_stream::iter(resync.then(|| {
        Ok(Event::default()
            .event("resync")
            .data("replay window exceeded"))
    }));
    let live = BroadcastStream::new(rx).filter_map(move |item| match item {
        Ok(m) if m.site_id == site => Some(Ok(Event::default()
            .id(m.id.to_string())
            .event("event")
            .json_data(m.payload)
            .unwrap())),
        Err(tokio_stream::wrappers::errors::BroadcastStreamRecvError::Lagged(_)) => {
            Some(Ok(Event::default()
                .event("resync")
                .data("subscriber lagged")))
        }
        _ => None,
    });
    Ok(Sse::new(initial.chain(marker).chain(live)).keep_alive(
        axum::response::sse::KeepAlive::new()
            .interval(Duration::from_secs(15))
            .text("heartbeat"),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn forwarded_ip_is_ignored_unless_proxy_is_trusted() {
        let mut headers = HeaderMap::new();
        headers.insert("x-forwarded-for", "198.51.100.9, 10.0.0.2".parse().unwrap());
        let peer: IpAddr = "203.0.113.7".parse().unwrap();

        assert_eq!(client_ip(&headers, peer, false), peer);
        assert_eq!(
            client_ip(&headers, peer, true),
            "198.51.100.9".parse::<IpAddr>().unwrap()
        );
    }

    /// First-party proxies forward the visitor's IP with the site's proxy key; anything
    /// unauthenticated keeps today's behaviour so the header can't be used to spoof.
    #[test]
    fn site_proxy_forwards_the_visitor_ip_only_with_its_key() {
        let key: Uuid = "6f1f6c2e-1d5e-4a3b-9f0e-2b7d6c5a4f31".parse().unwrap();
        let peer: IpAddr = "172.18.0.5".parse().unwrap();
        let visitor: IpAddr = "81.2.69.160".parse().unwrap();
        let mut headers = HeaderMap::new();
        headers.insert("x-slimlytics-client-ip", "81.2.69.160".parse().unwrap());
        headers.insert("x-slimlytics-proxy-key", key.to_string().parse().unwrap());
        assert_eq!(collection_ip(&headers, peer, false, key), visitor);
        assert_eq!(collection_ip(&headers, peer, true, key), visitor);

        headers.insert(
            "x-slimlytics-proxy-key",
            Uuid::nil().to_string().parse().unwrap(),
        );
        assert_eq!(
            collection_ip(&headers, peer, false, key),
            peer,
            "wrong key is ignored"
        );

        headers.remove("x-slimlytics-proxy-key");
        assert_eq!(
            collection_ip(&headers, peer, false, key),
            peer,
            "missing key is ignored"
        );

        headers.insert("x-slimlytics-proxy-key", key.to_string().parse().unwrap());
        headers.insert("x-slimlytics-client-ip", "not-an-ip".parse().unwrap());
        assert_eq!(
            collection_ip(&headers, peer, false, key),
            peer,
            "malformed IP is ignored"
        );
    }

    #[test]
    fn malformed_forwarded_ip_falls_back_to_peer() {
        let mut headers = HeaderMap::new();
        headers.insert("x-forwarded-for", "not-an-ip".parse().unwrap());
        let peer: IpAddr = "203.0.113.7".parse().unwrap();

        assert_eq!(client_ip(&headers, peer, true), peer);
    }
}
