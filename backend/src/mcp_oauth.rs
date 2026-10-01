//! Native public-client authorization code flow for the MCP resource.
use super::*;
use axum::{extract::Form, response::Html};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use sha2::{Digest, Sha256};

pub(super) fn routes() -> Router<AppState> {
    Router::new()
        .route(
            "/.well-known/oauth-protected-resource/api/mcp",
            get(resource),
        )
        .route("/.well-known/oauth-protected-resource", get(resource))
        .route("/.well-known/oauth-authorization-server", get(metadata))
        .route("/api/oauth/register", post(register))
        .route("/api/oauth/authorize", get(authorize).post(approve))
        .route("/api/oauth/token", post(token))
}
fn base(s: &AppState) -> &str {
    &s.public_url
}
fn resource_url(s: &AppState) -> String {
    format!("{}/api/mcp", base(s))
}
async fn resource(State(s): State<AppState>) -> Json<Value> {
    Json(
        json!({"resource":resource_url(&s),"authorization_servers":[base(&s)],"scopes_supported":["sites:read","sites:write","analytics:read","integrations:read"]}),
    )
}
async fn metadata(State(s): State<AppState>) -> Json<Value> {
    Json(
        json!({"issuer":base(&s),"authorization_endpoint":format!("{}/api/oauth/authorize",base(&s)),"token_endpoint":format!("{}/api/oauth/token",base(&s)),"registration_endpoint":format!("{}/api/oauth/register",base(&s)),"response_types_supported":["code"],"grant_types_supported":["authorization_code"],"token_endpoint_auth_methods_supported":["none"],"code_challenge_methods_supported":["S256"],"scopes_supported":["sites:read","sites:write","analytics:read","integrations:read"]}),
    )
}
fn bad(message: &str) -> ApiError {
    ApiError::BadRequest(message.into())
}
fn valid_redirect(value: &str) -> bool {
    Url::parse(value).is_ok_and(|u| {
        u.fragment().is_none()
            && u.username().is_empty()
            && u.password().is_none()
            && (u.scheme() == "https"
                || (u.scheme() == "http"
                    && matches!(u.host_str(), Some("localhost" | "127.0.0.1" | "[::1]"))))
    })
}
#[derive(Deserialize)]
struct Registration {
    client_name: Option<String>,
    redirect_uris: Vec<String>,
    token_endpoint_auth_method: Option<String>,
}
async fn register(
    State(s): State<AppState>,
    Json(r): Json<Registration>,
) -> Result<impl IntoResponse, ApiError> {
    if !s.login_limiter.check("oauth-register") {
        return Err(ApiError::RateLimited);
    }
    if r.redirect_uris.is_empty()
        || r.redirect_uris.len() > 10
        || r.redirect_uris
            .iter()
            .any(|u| u.len() > 2048 || !valid_redirect(u))
        || r.token_endpoint_auth_method
            .as_deref()
            .is_some_and(|m| m != "none")
    {
        return Err(bad("invalid public OAuth client"));
    }
    let name = r.client_name.unwrap_or_else(|| "Analytics agent".into());
    if name.is_empty() || name.len() > 100 {
        return Err(bad("invalid client name"));
    }
    let id: Uuid = sqlx::query_scalar(
        "INSERT INTO oauth_clients(name,redirect_uris) VALUES($1,$2) RETURNING id",
    )
    .bind(&name)
    .bind(&r.redirect_uris)
    .fetch_one(&s.pool)
    .await?;
    Ok((
        StatusCode::CREATED,
        Json(
            json!({"client_id":id,"client_name":name,"redirect_uris":r.redirect_uris,"token_endpoint_auth_method":"none","grant_types":["authorization_code"],"response_types":["code"]}),
        ),
    ))
}
#[derive(Deserialize)]
struct Authorization {
    client_id: Uuid,
    redirect_uri: String,
    response_type: String,
    code_challenge: String,
    code_challenge_method: String,
    resource: String,
    scope: Option<String>,
    state: Option<String>,
}
impl Authorization {
    async fn validate(&self, s: &AppState) -> Result<(String, Vec<String>), ApiError> {
        if self.response_type != "code"
            || self.code_challenge_method != "S256"
            || self.code_challenge.len() != 43
            || URL_SAFE_NO_PAD
                .decode(&self.code_challenge)
                .map_or(true, |v| v.len() != 32)
            || self.resource != resource_url(s)
            || self.state.as_ref().is_some_and(|v| v.len() > 2048)
        {
            return Err(bad("invalid authorization request"));
        }
        let row: Option<(String, Vec<String>)> =
            sqlx::query_as("SELECT name,redirect_uris FROM oauth_clients WHERE id=$1")
                .bind(self.client_id)
                .fetch_optional(&s.pool)
                .await?;
        let (name, uris) = row.ok_or_else(|| bad("unknown client"))?;
        if !uris.contains(&self.redirect_uri) {
            return Err(bad("unregistered redirect URI"));
        }
        let scopes: Vec<String> = self
            .scope
            .as_deref()
            .unwrap_or("sites:read sites:write analytics:read")
            .split_whitespace()
            .map(str::to_owned)
            .collect();
        let scopes = validate_scopes(&scopes).map_err(bad)?;
        if scopes.iter().any(|v| {
            !matches!(
                v.as_str(),
                "sites:read" | "sites:write" | "analytics:read" | "integrations:read"
            )
        }) {
            return Err(bad("unsupported OAuth scope"));
        }
        Ok((name, scopes))
    }
}
fn escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}
async fn authorize(
    State(s): State<AppState>,
    Query(q): Query<Authorization>,
) -> Result<Response, ApiError> {
    let (name, scopes) = q.validate(&s).await?;
    let csrf = generate_api_token();
    let html=format!("<!doctype html><html><meta charset=utf-8><title>Connect Slimlytics</title><h1>Connect {}</h1><p>Allow this agent these permissions: {}. Site write access allows creating and configuring sites.</p><form method=post><input type=hidden name=csrf value=\"{}\"><label>Email <input type=email name=email required autocomplete=username></label><label>Password <input type=password name=password required autocomplete=current-password maxlength=1024></label><button>Log in and authorize</button></form><p>Close this page to cancel.</p></html>",escape(&name),escape(&scopes.join(", ")),csrf);
    Ok(([(header::SET_COOKIE,format!("slyt_oauth_csrf={csrf}; HttpOnly; SameSite=Strict; Path=/api/oauth/authorize; Max-Age=600{}",if base(&s).starts_with("https:"){"; Secure"}else{""})),(header::CACHE_CONTROL,"no-store".into()),(header::CONTENT_SECURITY_POLICY,"default-src 'none'; form-action 'self'; frame-ancestors 'none'; base-uri 'none'".into())],Html(html)).into_response())
}
#[derive(Deserialize)]
struct Approval {
    email: String,
    password: String,
    csrf: String,
}
async fn approve(
    State(s): State<AppState>,
    Query(q): Query<Authorization>,
    headers: HeaderMap,
    Form(f): Form<Approval>,
) -> Result<Response, ApiError> {
    let (_, scopes) = q.validate(&s).await?;
    let cookie = headers
        .get(header::COOKIE)
        .and_then(|h| h.to_str().ok())
        .unwrap_or("");
    if !cookie
        .split(';')
        .any(|p| p.trim() == format!("slyt_oauth_csrf={}", f.csrf))
        || !f.csrf.starts_with("slyt_")
    {
        return Err(ApiError::Forbidden);
    }
    let Json(session) = login(
        State(s.clone()),
        Json(Credentials {
            email: f.email,
            password: f.password,
        }),
    )
    .await?;
    let user = verify_token(&session.token, &s.jwt_secret)
        .map_err(|_| ApiError::Unauthorized)?
        .sub;
    let code = generate_api_token();
    sqlx::query("INSERT INTO oauth_codes(code_hash,client_id,user_id,redirect_uri,challenge,scopes) VALUES($1,$2,$3,$4,$5,$6)").bind(hash_api_token(&code)).bind(q.client_id).bind(user).bind(&q.redirect_uri).bind(q.code_challenge).bind(scopes).execute(&s.pool).await?;
    let mut redirect = Url::parse(&q.redirect_uri).map_err(|_| bad("invalid redirect"))?;
    redirect.query_pairs_mut().append_pair("code", &code);
    if let Some(state) = q.state {
        redirect.query_pairs_mut().append_pair("state", &state);
    }
    Ok((
        [
            (header::CACHE_CONTROL, "no-store"),
            (
                header::SET_COOKIE,
                "slyt_oauth_csrf=; HttpOnly; SameSite=Strict; Path=/api/oauth/authorize; Max-Age=0",
            ),
        ],
        Redirect::to(redirect.as_str()),
    )
        .into_response())
}
#[derive(Deserialize)]
struct Exchange {
    grant_type: String,
    code: String,
    client_id: Uuid,
    redirect_uri: String,
    code_verifier: String,
    resource: String,
}
async fn token(State(s): State<AppState>, Form(f): Form<Exchange>) -> Response {
    match exchange(&s, f).await {
        Ok(value) => (
            [
                (header::CACHE_CONTROL, "no-store"),
                (header::PRAGMA, "no-cache"),
            ],
            Json(value),
        )
            .into_response(),
        Err(_) => (
            StatusCode::BAD_REQUEST,
            Json(json!({"error":"invalid_grant"})),
        )
            .into_response(),
    }
}
async fn exchange(s: &AppState, f: Exchange) -> Result<Value, ApiError> {
    if f.grant_type != "authorization_code"
        || f.resource != resource_url(s)
        || !(43..=128).contains(&f.code_verifier.len())
        || !f
            .code_verifier
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"-._~".contains(&b))
    {
        return Err(bad("invalid grant"));
    }
    let challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(f.code_verifier.as_bytes()));
    let mut tx = s.pool.begin().await?;
    let row:Option<(Uuid,Vec<String>)>=sqlx::query_as("DELETE FROM oauth_codes WHERE code_hash=$1 AND client_id=$2 AND redirect_uri=$3 AND challenge=$4 AND expires_at>now() RETURNING user_id,scopes").bind(hash_api_token(&f.code)).bind(f.client_id).bind(f.redirect_uri).bind(challenge).fetch_optional(&mut *tx).await?;
    let (user, scopes) = row.ok_or_else(|| bad("invalid grant"))?;
    let access = generate_api_token();
    sqlx::query("INSERT INTO api_tokens(user_id,name,token_hash,token_prefix,expires_at,scopes,oauth_resource) VALUES($1,'MCP OAuth agent',$2,$3,now()+interval '30 days',$4,$5)").bind(user).bind(hash_api_token(&access)).bind(&access[..12]).bind(&scopes).bind(resource_url(s)).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(
        json!({"access_token":access,"token_type":"Bearer","expires_in":2592000,"scope":scopes.join(" ")}),
    )
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn redirects_require_tls_or_loopback() {
        assert!(valid_redirect("http://127.0.0.1:1234/callback"));
        assert!(valid_redirect("https://agent.example/callback"));
        for url in [
            "http://evil.example/callback",
            "https://user:pass@agent.example/cb",
            "https://agent.example/cb#fragment",
            "javascript:alert(1)",
        ] {
            assert!(!valid_redirect(url));
        }
    }
}
