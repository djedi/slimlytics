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
<button>Log in and authorize</button></form>\
<p class=fine>Your password stays with Slimlytics. The agent receives a token that lasts 30 days, which you can revoke at any time in your API token settings.</p>\
<p class=fine>Not expecting this? Close this page to cancel.</p></main></body></html>",
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

async fn authorize(
    State(s): State<AppState>,
    Query(q): Query<Authorization>,
) -> Result<Response, ApiError> {
    let (name, scopes) = q.validate(&s).await?;
    let csrf = generate_api_token();
    let html = consent_page(&name, &scopes, &csrf, "", None);
    Ok(consent_response(
        &s,
        &q.redirect_uri,
        StatusCode::OK,
        html,
        &csrf,
    ))
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
    let (name, scopes) = q.validate(&s).await?;
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
            &name,
            &scopes,
            &csrf,
            &f.email,
            Some("This sign-in page expired or was already used. Please sign in again."),
        );
        return Ok(consent_response(
            &s,
            &q.redirect_uri,
            StatusCode::FORBIDDEN,
            html,
            &csrf,
        ));
    }
    let email = f.email.clone();
    let session = match login(
        State(s.clone()),
        Json(Credentials {
            email: f.email,
            password: f.password,
        }),
    )
    .await
    {
        Ok(Json(session)) => session,
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
            let html = consent_page(&name, &scopes, &csrf, &email, Some(message));
            return Ok(consent_response(&s, &q.redirect_uri, status, html, &csrf));
        }
        Err(error) => return Err(error),
    };
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
