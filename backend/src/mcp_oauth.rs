//! Native public-client OAuth 2.1 authorization server for the MCP resource:
//! dynamic client registration (RFC 7591), S256 PKCE, resource indicators (RFC 8707),
//! issuer identification (RFC 9207), and rotating refresh tokens with reuse detection.
use super::*;
use axum::{
    extract::{
        rejection::{FormRejection, JsonRejection},
        Form,
    },
    http::{HeaderName, Method},
    response::Html,
};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use sha2::{Digest, Sha256};
use tower_http::cors::{Any, CorsLayer};

const SCOPES: [&str; 4] = [
    "sites:read",
    "sites:write",
    "analytics:read",
    "integrations:read",
];
const DEFAULT_SCOPE: &str = "sites:read sites:write analytics:read";
/// Lifetime of one access token. Clients renew it with the refresh token.
const ACCESS_TOKEN_SECONDS: i64 = 3600;

pub(super) fn routes() -> Router<AppState> {
    // Discovery, registration and token exchange carry no cookies, so any origin may call
    // them; browser-based MCP clients need this. The consent page is deliberately excluded.
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods([Method::GET, Method::POST])
        .allow_headers([
            header::CONTENT_TYPE,
            header::AUTHORIZATION,
            HeaderName::from_static("mcp-protocol-version"),
        ])
        .max_age(Duration::from_secs(86400));
    Router::new()
        .route(
            "/.well-known/oauth-protected-resource/api/mcp",
            get(resource),
        )
        .route("/.well-known/oauth-protected-resource", get(resource))
        .route("/.well-known/oauth-authorization-server", get(metadata))
        .route("/api/oauth/register", post(register))
        .route("/api/oauth/token", post(token))
        .layer(cors)
        .route("/api/oauth/authorize", get(authorize).post(approve))
}
fn base(s: &AppState) -> &str {
    &s.public_url
}
fn resource_url(s: &AppState) -> String {
    format!("{}/api/mcp", base(s))
}
async fn resource(State(s): State<AppState>) -> Json<Value> {
    Json(
        json!({"resource":resource_url(&s),"authorization_servers":[base(&s)],"scopes_supported":SCOPES,"bearer_methods_supported":["header"]}),
    )
}
async fn metadata(State(s): State<AppState>) -> Json<Value> {
    Json(json!({
        "issuer":base(&s),
        "authorization_endpoint":format!("{}/api/oauth/authorize",base(&s)),
        "token_endpoint":format!("{}/api/oauth/token",base(&s)),
        "registration_endpoint":format!("{}/api/oauth/register",base(&s)),
        "response_types_supported":["code"],
        "response_modes_supported":["query"],
        "grant_types_supported":["authorization_code","refresh_token"],
        "token_endpoint_auth_methods_supported":["none"],
        "code_challenge_methods_supported":["S256"],
        "authorization_response_iss_parameter_supported":true,
        "scopes_supported":SCOPES
    }))
}
fn bad(message: &str) -> ApiError {
    ApiError::BadRequest(message.into())
}
/// An RFC 6749 / RFC 7591 JSON error. Never cached: token responses may follow it.
fn oauth_error(status: StatusCode, error: &str, description: &str) -> Response {
    (
        status,
        [
            (header::CACHE_CONTROL, "no-store"),
            (header::PRAGMA, "no-cache"),
        ],
        Json(json!({"error":error,"error_description":description})),
    )
        .into_response()
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
    #[serde(default)]
    redirect_uris: Vec<String>,
    token_endpoint_auth_method: Option<String>,
    grant_types: Option<Vec<String>>,
    response_types: Option<Vec<String>>,
}
async fn register(
    State(s): State<AppState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    body: Result<Json<Registration>, JsonRejection>,
) -> Result<Response, ApiError> {
    let ip = client_ip(&headers, peer.ip(), s.trust_proxy);
    if !s.oauth_register_limiter.check(&ip.to_string()) {
        return Err(ApiError::RateLimited);
    }
    let invalid = |description| {
        Ok(oauth_error(
            StatusCode::BAD_REQUEST,
            "invalid_client_metadata",
            description,
        ))
    };
    let Ok(Json(r)) = body else {
        return invalid("registration must be a JSON object");
    };
    if r.redirect_uris.is_empty()
        || r.redirect_uris.len() > 10
        || r.redirect_uris
            .iter()
            .any(|u| u.len() > 2048 || !valid_redirect(u))
    {
        return Ok(oauth_error(
            StatusCode::BAD_REQUEST,
            "invalid_redirect_uri",
            "redirect URIs must use HTTPS or an HTTP loopback address, without fragments",
        ));
    }
    if r.token_endpoint_auth_method
        .as_deref()
        .is_some_and(|m| m != "none")
    {
        return invalid("only public clients (token_endpoint_auth_method none) are supported");
    }
    if r.grant_types.as_ref().is_some_and(|grants| {
        grants
            .iter()
            .any(|g| !matches!(g.as_str(), "authorization_code" | "refresh_token"))
    }) {
        return invalid("supported grant types are authorization_code and refresh_token");
    }
    if r.response_types
        .as_ref()
        .is_some_and(|types| types.iter().any(|t| t != "code"))
    {
        return invalid("the only supported response type is code");
    }
    let name = r.client_name.unwrap_or_else(|| "Analytics agent".into());
    if name.is_empty() || name.len() > 100 {
        return invalid("client_name must be 1 to 100 characters");
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
        [(header::CACHE_CONTROL, "no-store")],
        Json(
            json!({"client_id":id,"client_name":name,"redirect_uris":r.redirect_uris,"token_endpoint_auth_method":"none","grant_types":["authorization_code","refresh_token"],"response_types":["code"]}),
        ),
    )
        .into_response())
}

/// Authorization request parameters. Everything is optional so that, once the client and
/// callback are trusted, problems are reported to the agent instead of a bare error page.
#[derive(Deserialize)]
struct Authorization {
    client_id: Option<String>,
    redirect_uri: Option<String>,
    response_type: Option<String>,
    code_challenge: Option<String>,
    code_challenge_method: Option<String>,
    resource: Option<String>,
    scope: Option<String>,
    state: Option<String>,
}
struct Approved {
    client_id: Uuid,
    client_name: String,
    redirect_uri: String,
    challenge: String,
    scopes: Vec<String>,
    state: Option<String>,
}
enum Rejected {
    /// The client or callback cannot be trusted: never redirect (RFC 6749 section 4.1.2.1).
    Fatal(ApiError),
    /// Report the error to the registered callback.
    Redirect {
        redirect_uri: String,
        state: Option<String>,
        error: &'static str,
        description: &'static str,
    },
}
impl From<sqlx::Error> for Rejected {
    fn from(error: sqlx::Error) -> Self {
        Self::Fatal(error.into())
    }
}
impl Authorization {
    async fn validate(&self, s: &AppState) -> Result<Approved, Rejected> {
        let client_id = self
            .client_id
            .as_deref()
            .and_then(|id| Uuid::parse_str(id).ok())
            .ok_or_else(|| Rejected::Fatal(bad("unknown client")))?;
        let redirect_uri = self
            .redirect_uri
            .clone()
            .ok_or_else(|| Rejected::Fatal(bad("redirect_uri is required")))?;
        if self.state.as_ref().is_some_and(|v| v.len() > 2048) {
            return Err(Rejected::Fatal(bad("state is too long")));
        }
        let row: Option<(String, Vec<String>)> =
            sqlx::query_as("SELECT name,redirect_uris FROM oauth_clients WHERE id=$1")
                .bind(client_id)
                .fetch_optional(&s.pool)
                .await?;
        let (client_name, uris) = row.ok_or_else(|| Rejected::Fatal(bad("unknown client")))?;
        if !uris.contains(&redirect_uri) {
            return Err(Rejected::Fatal(bad("unregistered redirect URI")));
        }
        let reject = |error, description| Rejected::Redirect {
            redirect_uri: redirect_uri.clone(),
            state: self.state.clone(),
            error,
            description,
        };
        match self.response_type.as_deref() {
            Some("code") => {}
            None => return Err(reject("invalid_request", "response_type is required")),
            Some(_) => {
                return Err(reject(
                    "unsupported_response_type",
                    "only the authorization code flow is supported",
                ))
            }
        }
        let challenge = self.code_challenge.clone().unwrap_or_default();
        if self.code_challenge_method.as_deref() != Some("S256")
            || challenge.len() != 43
            || URL_SAFE_NO_PAD
                .decode(&challenge)
                .map_or(true, |v| v.len() != 32)
        {
            return Err(reject(
                "invalid_request",
                "an S256 PKCE code_challenge is required",
            ));
        }
        match self.resource.as_deref() {
            None => return Err(reject("invalid_request", "resource is required")),
            Some(resource) if resource != resource_url(s) => {
                return Err(reject("invalid_target", "unknown resource"))
            }
            Some(_) => {}
        }
        let requested: Vec<String> = self
            .scope
            .as_deref()
            .unwrap_or(DEFAULT_SCOPE)
            .split_whitespace()
            .map(str::to_owned)
            .collect();
        let scopes = validate_scopes(&requested)
            .ok()
            .filter(|scopes| scopes.iter().all(|v| SCOPES.contains(&v.as_str())))
            .ok_or_else(|| reject("invalid_scope", "unsupported OAuth scope"))?;
        Ok(Approved {
            client_id,
            client_name,
            redirect_uri: redirect_uri.clone(),
            challenge,
            scopes,
            state: self.state.clone(),
        })
    }
}
/// Redirect to the registered callback with the response parameters, `state`, and `iss`.
fn callback(
    s: &AppState,
    redirect_uri: &str,
    state: Option<&str>,
    pairs: &[(&str, &str)],
) -> Response {
    let Ok(mut url) = Url::parse(redirect_uri) else {
        return bad("invalid redirect").into_response();
    };
    {
        let mut query = url.query_pairs_mut();
        for (key, value) in pairs {
            query.append_pair(key, value);
        }
        if let Some(state) = state {
            query.append_pair("state", state);
        }
        query.append_pair("iss", base(s));
    }
    (
        [
            (header::CACHE_CONTROL, "no-store"),
            (
                header::SET_COOKIE,
                "slyt_oauth_csrf=; HttpOnly; SameSite=Strict; Path=/api/oauth/authorize; Max-Age=0",
            ),
        ],
        Redirect::to(url.as_str()),
    )
        .into_response()
}
fn rejection(s: &AppState, rejected: Rejected) -> Response {
    match rejected {
        Rejected::Fatal(error) => error.into_response(),
        Rejected::Redirect {
            redirect_uri,
            state,
            error,
            description,
        } => callback(
            s,
            &redirect_uri,
            state.as_deref(),
            &[("error", error), ("error_description", description)],
        ),
    }
}
fn escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}
/// Stylesheet for the consent page. The CSP allows exactly this text by hash, so the page
/// keeps `default-src 'none'` without falling back to `'unsafe-inline'`.
const CONSENT_CSS: &str = r#":root{color-scheme:light dark;--bg:#f4f6f8;--card:#fff;--text:#0d1520;--muted:#4a5668;--line:#e2e8ee;--accent:#047857;--accent-soft:#e6f6ef;--warn:#9a5b00;--warn-soft:#fff4e0;--danger:#b42318;--danger-soft:#fdecea}
@media(prefers-color-scheme:dark){:root{--bg:#070c14;--card:#0e1622;--text:#eef3f8;--muted:#a3b0c2;--line:#1d2836;--accent:#34d399;--accent-soft:rgba(52,211,153,.12);--warn:#f5b544;--warn-soft:rgba(245,181,68,.12);--danger:#f87171;--danger-soft:rgba(248,113,113,.12)}}
*{box-sizing:border-box}
body{margin:0;min-height:100vh;display:grid;place-items:center;padding:24px 16px;background:var(--bg);background-image:radial-gradient(600px 400px at 80% 0%,rgba(16,185,129,.16),transparent 70%),radial-gradient(500px 360px at 0% 100%,rgba(122,162,255,.12),transparent 70%);color:var(--text);font:16px/1.55 ui-sans-serif,system-ui,-apple-system,"Segoe UI",Roboto,sans-serif}
main{width:min(440px,100%);padding:32px;border:1px solid var(--line);border-radius:20px;background:var(--card);box-shadow:0 30px 70px -30px rgba(0,0,0,.35)}
.brand{display:flex;align-items:center;gap:10px;font-weight:800;font-size:17px;letter-spacing:-.02em}
.mark{display:grid;place-items:center;width:32px;height:32px;border-radius:9px;background:linear-gradient(140deg,#10b981,#047857)}
.eyebrow{margin:26px 0 6px;color:var(--accent);font-size:12px;font-weight:700;letter-spacing:.12em;text-transform:uppercase}
h1{margin:0;font-size:24px;line-height:1.25;letter-spacing:-.02em}
.client{color:var(--accent)}
h2{margin:22px 0 10px;color:var(--muted);font-size:13px;font-weight:600}
ul{list-style:none;margin:0;padding:0;display:grid;gap:8px}
li{display:flex;align-items:center;gap:10px;padding:10px 12px;border:1px solid var(--line);border-radius:12px;font-size:15px}
li svg{flex:none;color:var(--accent)}
li.write{border-color:color-mix(in srgb,var(--warn) 40%,var(--line));background:var(--warn-soft)}
li.write svg{color:var(--warn)}
li small{margin-left:auto;color:var(--warn);font-size:12px;font-weight:700}
.alert{margin:20px 0 0;padding:12px 14px;border-radius:12px;background:var(--danger-soft);color:var(--danger);font-size:14px;font-weight:600}
form{display:grid;gap:14px;margin-top:24px}
label{display:grid;gap:6px;font-size:14px;font-weight:600}
input{width:100%;min-height:48px;padding:10px 14px;border:1px solid var(--line);border-radius:12px;background:transparent;color:var(--text);font:inherit}
input:focus{outline:none;border-color:var(--accent);box-shadow:0 0 0 4px var(--accent-soft)}
button{min-height:50px;margin-top:4px;border:0;border-radius:999px;background:#047857;color:#fff;font:inherit;font-weight:700;cursor:pointer}
button:hover{background:#065f46}
button.cancel{margin-top:0;border:1px solid var(--line);background:transparent;color:var(--text);font-weight:600}
button.cancel:hover{background:var(--accent-soft)}
button:focus-visible{outline:3px solid var(--accent);outline-offset:3px}
.fine{margin:18px 0 0;color:var(--muted);font-size:13px}
.fine+.fine{margin-top:6px}"#;

/// CSP for the consent page. Styles are allowed only by hash. `form-action` must include the
/// client's callback origin: browsers apply it to the redirect after the form is submitted.
fn consent_csp(redirect_uri: &str) -> String {
    static STYLE_HASH: std::sync::LazyLock<String> = std::sync::LazyLock::new(|| {
        base64::engine::general_purpose::STANDARD.encode(Sha256::digest(CONSENT_CSS.as_bytes()))
    });
    // The redirect URI was validated (HTTPS or loopback) before rendering; only its origin is used.
    let callback = Url::parse(redirect_uri)
        .ok()
        .map(|url| url.origin())
        .filter(|origin| origin.is_tuple())
        .map(|origin| format!(" {}", origin.ascii_serialization()))
        .unwrap_or_default();
    format!(
        "default-src 'none'; style-src 'sha256-{}'; form-action 'self'{callback}; frame-ancestors 'none'; base-uri 'none'",
        *STYLE_HASH
    )
}

/// What each OAuth scope lets the agent do, in plain language. Write access is flagged.
fn describe_scope(scope: &str) -> (&'static str, bool) {
    match scope {
        "sites:read" => ("See your sites and their settings", false),
        "sites:write" => ("Create and configure sites", true),
        "analytics:read" => ("Read analytics reports", false),
        "integrations:read" => ("Read connected integrations like Search Console", false),
        _ => ("Additional access", false),
    }
}

fn consent_page(
    client: &str,
    scopes: &[String],
    csrf: &str,
    email: &str,
    error: Option<&str>,
) -> String {
    const CHECK: &str = "<svg width=18 height=18 viewBox=\"0 0 24 24\" fill=none stroke=currentColor stroke-width=2.5 stroke-linecap=round stroke-linejoin=round aria-hidden=true><path d=\"M20 6 9 17l-5-5\"/></svg>";
    const PEN: &str = "<svg width=18 height=18 viewBox=\"0 0 24 24\" fill=none stroke=currentColor stroke-width=2 stroke-linecap=round stroke-linejoin=round aria-hidden=true><path d=\"M12 20h9\"/><path d=\"M16.5 3.5a2.1 2.1 0 0 1 3 3L7 19l-4 1 1-4Z\"/></svg>";
    let client = escape(client);
    let permissions: String = scopes
        .iter()
        .map(|scope| {
            let (text, write) = describe_scope(scope);
            if write {
                format!("<li class=write>{PEN}{text}<small>Write</small></li>")
            } else {
                format!("<li>{CHECK}{text}</li>")
            }
        })
        .collect();
    let alert = error
        .map(|message| format!("<p class=alert role=\"alert\">{}</p>", escape(message)))
        .unwrap_or_default();
    format!(
        "<!doctype html><html lang=en><head><meta charset=utf-8><meta name=viewport content=\"width=device-width,initial-scale=1\"><meta name=robots content=noindex><title>Connect {client} to Slimlytics</title><style>{CONSENT_CSS}</style></head><body><main>\
<div class=brand><span class=mark><svg width=18 height=18 viewBox=\"0 0 24 24\" fill=#fff aria-hidden=true><rect x=\"4\" y=\"13\" width=\"4\" height=\"7\" rx=\"1\"/><rect x=\"10\" y=\"8\" width=\"4\" height=\"12\" rx=\"1\"/><rect x=\"16\" y=\"4\" width=\"4\" height=\"16\" rx=\"1\"/></svg></span>Slimlytics</div>\
<p class=eyebrow>Authorize an agent</p><h1><span class=client>{client}</span> wants to connect to your Slimlytics account</h1>{alert}\
<h2>It will be able to</h2><ul>{permissions}</ul>\
<form method=post><input type=hidden name=csrf value=\"{csrf}\">\
<label>Email<input type=email name=email value=\"{email}\" required autocomplete=username></label>\
<label>Password<input type=password name=password required autocomplete=current-password maxlength=1024></label>\
<button name=action value=approve>Log in and authorize</button>\
<button class=cancel name=action value=deny formnovalidate>Cancel</button></form>\
<p class=fine>Your password stays with Slimlytics. The agent stays connected until you revoke it in your API token settings, or after 90 days without use.</p>\
<p class=fine>Not expecting this? Choose Cancel and nothing is shared.</p></main></body></html>",
        email = escape(email),
    )
}

/// The consent page response: fresh CSRF cookie, no caching, and the hashed-style CSP.
fn consent_response(
    s: &AppState,
    redirect_uri: &str,
    status: StatusCode,
    html: String,
    csrf: &str,
) -> Response {
    let secure = if base(s).starts_with("https:") {
        "; Secure"
    } else {
        ""
    };
    (
        status,
        [
            (
                header::SET_COOKIE,
                format!("slyt_oauth_csrf={csrf}; HttpOnly; SameSite=Strict; Path=/api/oauth/authorize; Max-Age=600{secure}"),
            ),
            (header::CACHE_CONTROL, "no-store".into()),
            (header::CONTENT_SECURITY_POLICY, consent_csp(redirect_uri)),
        ],
        Html(html),
    )
        .into_response()
}

async fn authorize(State(s): State<AppState>, Query(q): Query<Authorization>) -> Response {
    let approved = match q.validate(&s).await {
        Ok(approved) => approved,
        Err(rejected) => return rejection(&s, rejected),
    };
    let csrf = generate_api_token();
    let html = consent_page(&approved.client_name, &approved.scopes, &csrf, "", None);
    consent_response(&s, &approved.redirect_uri, StatusCode::OK, html, &csrf)
}
#[derive(Deserialize)]
struct Approval {
    #[serde(default)]
    email: String,
    #[serde(default)]
    password: String,
    #[serde(default)]
    csrf: String,
    action: Option<String>,
}
async fn approve(
    State(s): State<AppState>,
    Query(q): Query<Authorization>,
    headers: HeaderMap,
    Form(f): Form<Approval>,
) -> Result<Response, ApiError> {
    let approved = match q.validate(&s).await {
        Ok(approved) => approved,
        Err(rejected) => return Ok(rejection(&s, rejected)),
    };
    let cookie = headers
        .get(header::COOKIE)
        .and_then(|h| h.to_str().ok())
        .unwrap_or("");
    if !cookie
        .split(';')
        .any(|p| p.trim() == format!("slyt_oauth_csrf={}", f.csrf))
        || !f.csrf.starts_with("slyt_")
    {
        // Usually an expired page or a repeated submit. Nothing is authorized; show a fresh form.
        let csrf = generate_api_token();
        let html = consent_page(
            &approved.client_name,
            &approved.scopes,
            &csrf,
            &f.email,
            Some("This sign-in page expired or was already used. Please sign in again."),
        );
        return Ok(consent_response(
            &s,
            &approved.redirect_uri,
            StatusCode::FORBIDDEN,
            html,
            &csrf,
        ));
    }
    if f.action.as_deref() == Some("deny") {
        return Ok(callback(
            &s,
            &approved.redirect_uri,
            approved.state.as_deref(),
            &[
                ("error", "access_denied"),
                (
                    "error_description",
                    "The account owner cancelled the connection",
                ),
            ],
        ));
    }
    let email = f.email.clone();
    let user = match authenticate_password(&s, &f.email, &f.password).await {
        Ok(user) => user,
        // Show credential problems on the page instead of a bare JSON error.
        Err(error @ (ApiError::Unauthorized | ApiError::RateLimited)) => {
            let (status, message) = match error {
                ApiError::RateLimited => (
                    StatusCode::TOO_MANY_REQUESTS,
                    "Too many sign-in attempts. Wait a moment and try again.",
                ),
                _ => (StatusCode::UNAUTHORIZED, "Email or password is incorrect."),
            };
            let csrf = generate_api_token();
            let html = consent_page(
                &approved.client_name,
                &approved.scopes,
                &csrf,
                &email,
                Some(message),
            );
            return Ok(consent_response(
                &s,
                &approved.redirect_uri,
                status,
                html,
                &csrf,
            ));
        }
        Err(error) => return Err(error),
    };
    let code = generate_api_token();
    sqlx::query("INSERT INTO oauth_codes(code_hash,client_id,user_id,redirect_uri,challenge,scopes) VALUES($1,$2,$3,$4,$5,$6)")
        .bind(hash_api_token(&code))
        .bind(approved.client_id)
        .bind(user)
        .bind(&approved.redirect_uri)
        .bind(&approved.challenge)
        .bind(&approved.scopes)
        .execute(&s.pool)
        .await?;
    Ok(callback(
        &s,
        &approved.redirect_uri,
        approved.state.as_deref(),
        &[("code", &code)],
    ))
}

#[derive(Deserialize)]
struct TokenRequest {
    grant_type: Option<String>,
    code: Option<String>,
    client_id: Option<String>,
    redirect_uri: Option<String>,
    code_verifier: Option<String>,
    resource: Option<String>,
    refresh_token: Option<String>,
    scope: Option<String>,
}
enum TokenError {
    OAuth(&'static str, &'static str),
    Server(ApiError),
}
impl From<sqlx::Error> for TokenError {
    fn from(error: sqlx::Error) -> Self {
        Self::Server(error.into())
    }
}
fn required<'a>(value: &'a Option<String>, name: &'static str) -> Result<&'a str, TokenError> {
    value
        .as_deref()
        .filter(|v| !v.is_empty())
        .ok_or(TokenError::OAuth("invalid_request", name))
}
async fn token(
    State(s): State<AppState>,
    body: Result<Form<TokenRequest>, FormRejection>,
) -> Response {
    let Ok(Form(f)) = body else {
        return oauth_error(
            StatusCode::BAD_REQUEST,
            "invalid_request",
            "expected a form-encoded token request",
        );
    };
    let result = match f.grant_type.as_deref() {
        Some("authorization_code") => exchange_code(&s, &f).await,
        Some("refresh_token") => exchange_refresh(&s, &f).await,
        Some(_) => Err(TokenError::OAuth(
            "unsupported_grant_type",
            "supported grant types are authorization_code and refresh_token",
        )),
        None => Err(TokenError::OAuth(
            "invalid_request",
            "grant_type is required",
        )),
    };
    match result {
        Ok(value) => (
            [
                (header::CACHE_CONTROL, "no-store"),
                (header::PRAGMA, "no-cache"),
            ],
            Json(value),
        )
            .into_response(),
        Err(TokenError::OAuth(error, description)) => {
            oauth_error(StatusCode::BAD_REQUEST, error, description)
        }
        Err(TokenError::Server(error)) => error.into_response(),
    }
}
/// Shared checks: the client must be named, and a resource, when sent, must be this server.
fn token_client(s: &AppState, f: &TokenRequest) -> Result<Uuid, TokenError> {
    let client = required(&f.client_id, "client_id is required")?;
    if f.resource.as_deref().is_some_and(|r| r != resource_url(s)) {
        return Err(TokenError::OAuth("invalid_target", "unknown resource"));
    }
    Uuid::parse_str(client).map_err(|_| TokenError::OAuth("invalid_grant", "unknown client"))
}
fn token_response(access: &str, refresh: &str, scopes: &[String]) -> Value {
    json!({"access_token":access,"token_type":"Bearer","expires_in":ACCESS_TOKEN_SECONDS,"refresh_token":refresh,"scope":scopes.join(" ")})
}
async fn exchange_code(s: &AppState, f: &TokenRequest) -> Result<Value, TokenError> {
    let code = required(&f.code, "code is required")?;
    let redirect_uri = required(&f.redirect_uri, "redirect_uri is required")?;
    let verifier = required(&f.code_verifier, "code_verifier is required")?;
    let client = token_client(s, f)?;
    let invalid = TokenError::OAuth(
        "invalid_grant",
        "the authorization code is invalid or expired",
    );
    if !(43..=128).contains(&verifier.len())
        || !verifier
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"-._~".contains(&b))
    {
        return Err(invalid);
    }
    let challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()));
    let mut tx = s.pool.begin().await?;
    // Lock the account before the code, the same order disabling uses (account row, then
    // its codes and tokens), so a connection can never be minted after a disable commits.
    let owner: Option<Uuid> =
        sqlx::query_scalar("SELECT user_id FROM oauth_codes WHERE code_hash=$1")
            .bind(hash_api_token(code))
            .fetch_optional(&mut *tx)
            .await?;
    let owner = owner.ok_or(TokenError::OAuth(
        "invalid_grant",
        "the authorization code is invalid or expired",
    ))?;
    if lock_enabled_user(&mut tx, owner, false).await.is_err() {
        return Err(invalid);
    }
    let row: Option<(Uuid, Vec<String>)> = sqlx::query_as(
        "DELETE FROM oauth_codes WHERE code_hash=$1 AND client_id=$2 AND redirect_uri=$3 AND challenge=$4 AND user_id=$5 AND expires_at>now() RETURNING user_id,scopes",
    )
    .bind(hash_api_token(code))
    .bind(client)
    .bind(redirect_uri)
    .bind(challenge)
    .bind(owner)
    .fetch_optional(&mut *tx)
    .await?;
    let (user, scopes) = row.ok_or(invalid)?;
    let access = generate_api_token();
    let connection: Uuid = sqlx::query_scalar(
        "INSERT INTO api_tokens(user_id,name,token_hash,token_prefix,expires_at,access_expires_at,scopes,oauth_resource)
         VALUES($1,'MCP OAuth agent',$2,$3,now()+interval '90 days',now()+make_interval(secs=>$4),$5,$6) RETURNING id",
    )
    .bind(user)
    .bind(hash_api_token(&access))
    .bind(&access[..12])
    .bind(ACCESS_TOKEN_SECONDS as f64)
    .bind(&scopes)
    .bind(resource_url(s))
    .fetch_one(&mut *tx)
    .await?;
    let refresh = issue_refresh_token(&mut tx, connection, client).await?;
    tx.commit().await?;
    Ok(token_response(&access, &refresh, &scopes))
}
async fn issue_refresh_token(
    tx: &mut Transaction<'_, Postgres>,
    connection: Uuid,
    client: Uuid,
) -> Result<String, sqlx::Error> {
    let refresh = generate_api_token();
    sqlx::query(
        "INSERT INTO oauth_refresh_tokens(token_hash,api_token_id,client_id) VALUES($1,$2,$3)",
    )
    .bind(hash_api_token(&refresh))
    .bind(connection)
    .bind(client)
    .execute(&mut **tx)
    .await?;
    sqlx::query("UPDATE oauth_clients SET last_used_at=now() WHERE id=$1")
        .bind(client)
        .execute(&mut **tx)
        .await?;
    Ok(refresh)
}
async fn exchange_refresh(s: &AppState, f: &TokenRequest) -> Result<Value, TokenError> {
    let presented = required(&f.refresh_token, "refresh_token is required")?;
    let client = token_client(s, f)?;
    let invalid = || TokenError::OAuth("invalid_grant", "the refresh token is invalid or expired");
    let presented_hash = hash_api_token(presented);
    let mut tx = s.pool.begin().await?;
    // Lock order is always connection, then refresh rows. Rotation and replay revocation
    // both follow it, so a replay racing a refresh serializes instead of deadlocking.
    let connection: Option<Uuid> = sqlx::query_scalar(
        "SELECT api_token_id FROM oauth_refresh_tokens WHERE token_hash=$1 AND client_id=$2",
    )
    .bind(&presented_hash)
    .bind(client)
    .fetch_optional(&mut *tx)
    .await?;
    let connection = connection.ok_or_else(invalid)?;
    // Account row first, as disabling does, so a refresh cannot renew a revoked account.
    let owner: Option<Uuid> = sqlx::query_scalar("SELECT user_id FROM api_tokens WHERE id=$1")
        .bind(connection)
        .fetch_optional(&mut *tx)
        .await?;
    match owner {
        Some(owner) if lock_enabled_user(&mut tx, owner, false).await.is_ok() => {}
        _ => return Err(invalid()),
    }
    let grant: Option<(Vec<String>, bool)> = sqlx::query_as(
        "SELECT t.scopes,(t.revoked_at IS NULL AND t.expires_at>now() AND u.disabled_at IS NULL)
         FROM api_tokens t JOIN users u ON u.id=t.user_id WHERE t.id=$1 FOR UPDATE OF t",
    )
    .bind(connection)
    .fetch_optional(&mut *tx)
    .await?;
    let used_at: Option<Option<DateTime<Utc>>> = sqlx::query_scalar(
        "SELECT used_at FROM oauth_refresh_tokens WHERE token_hash=$1 AND api_token_id=$2 FOR UPDATE",
    )
    .bind(&presented_hash)
    .bind(connection)
    .fetch_optional(&mut *tx)
    .await?;
    // The row can vanish between the lookup and the lock if a replay revoked the connection.
    let ((granted, active), used_at) = grant.zip(used_at).ok_or_else(invalid)?;
    if used_at.is_some() {
        // A rotated-out token came back: someone else holds a copy. End the connection.
        sqlx::query("UPDATE api_tokens SET revoked_at=now() WHERE id=$1 AND revoked_at IS NULL")
            .bind(connection)
            .execute(&mut *tx)
            .await?;
        sqlx::query("DELETE FROM oauth_refresh_tokens WHERE api_token_id=$1")
            .bind(connection)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        tracing::warn!(%connection, "MCP OAuth refresh token reused; connection revoked");
        return Err(invalid());
    }
    if !active {
        return Err(invalid());
    }
    let scopes = match f.scope.as_deref() {
        None => granted,
        Some(scope) => {
            let requested: Vec<String> = scope.split_whitespace().map(str::to_owned).collect();
            validate_scopes(&requested)
                .ok()
                .filter(|scopes| scopes.iter().all(|v| granted.contains(v)))
                .ok_or(TokenError::OAuth(
                    "invalid_scope",
                    "a refresh can only keep or narrow the granted scopes",
                ))?
        }
    };
    sqlx::query("UPDATE oauth_refresh_tokens SET used_at=now() WHERE token_hash=$1")
        .bind(&presented_hash)
        .execute(&mut *tx)
        .await?;
    let access = generate_api_token();
    sqlx::query(
        "UPDATE api_tokens SET token_hash=$2,token_prefix=$3,scopes=$4,
           access_expires_at=now()+make_interval(secs=>$5),expires_at=now()+interval '90 days'
         WHERE id=$1",
    )
    .bind(connection)
    .bind(hash_api_token(&access))
    .bind(&access[..12])
    .bind(&scopes)
    .bind(ACCESS_TOKEN_SECONDS as f64)
    .execute(&mut *tx)
    .await?;
    let refresh = issue_refresh_token(&mut tx, connection, client).await?;
    tx.commit().await?;
    Ok(token_response(&access, &refresh, &scopes))
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

    #[test]
    fn consent_page_escapes_input_and_explains_scopes_in_plain_language() {
        let scopes = ["analytics:read".to_owned(), "sites:write".to_owned()];
        let page = consent_page("<script>Agent</script>", &scopes, "slyt_csrf", "", None);
        assert!(page.contains("&lt;script&gt;Agent&lt;/script&gt;"));
        assert!(!page.contains("<script>Agent"));
        assert!(page.contains("Read analytics reports"));
        assert!(page.contains("Create and configure sites"));
        assert!(page.contains("value=\"slyt_csrf\""));
        assert!(!page.contains("role=\"alert\""));
        let failed = consent_page(
            "Agent",
            &scopes,
            "slyt_csrf",
            "me@example.com",
            Some("Email or password is incorrect."),
        );
        assert!(failed.contains("role=\"alert\""));
        assert!(failed.contains("Email or password is incorrect."));
        assert!(
            failed.contains("value=\"me@example.com\""),
            "keeps the email after a failed login"
        );
    }

    #[test]
    fn consent_csp_allows_exactly_the_inline_stylesheet() {
        let digest = base64::engine::general_purpose::STANDARD
            .encode(Sha256::digest(CONSENT_CSS.as_bytes()));
        let csp = consent_csp("http://127.0.0.1:9988/cb");
        assert!(csp.contains(&format!("style-src 'sha256-{digest}'")));
        assert!(csp.starts_with("default-src 'none'"));
        assert!(!csp.contains("unsafe-inline"));
        let page = consent_page("Agent", &[], "slyt_csrf", "", None);
        assert!(page.contains(&format!("<style>{CONSENT_CSS}</style>")));
    }

    /// Browsers enforce form-action on the redirect that follows a form submission. Without the
    /// client's callback origin, Chrome silently blocks the redirect after a successful login
    /// and a second click then fails CSRF.
    #[test]
    fn consent_csp_lets_the_form_redirect_to_the_registered_callback() {
        let csp = consent_csp("http://127.0.0.1:9988/cb?x=1");
        assert!(csp.contains("form-action 'self' http://127.0.0.1:9988;"));
        let csp = consent_csp("https://agent.example/oauth/callback");
        assert!(csp.contains("form-action 'self' https://agent.example;"));
        assert!(consent_csp("not a url").contains("form-action 'self';"));
    }
}
