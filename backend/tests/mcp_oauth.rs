use axum::{
    body::{to_bytes, Body},
    extract::ConnectInfo,
    http::Request,
    Router,
};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use slimlytics_backend::{app, AppState};
use sqlx::{postgres::PgPoolOptions, PgPool};
use std::net::SocketAddr;
use tower::ServiceExt;

const CALLBACK: &str = "http://127.0.0.1:9988/cb";
const RESOURCE: &str = "http://localhost:8080/api/mcp";
const ISSUER: &str = "http://localhost:8080";

async fn setup() -> (PgPool, Router) {
    let pool = PgPoolOptions::new()
        .connect(&std::env::var("TEST_DATABASE_URL").unwrap())
        .await
        .unwrap();
    sqlx::migrate!("../migrations").run(&pool).await.unwrap();
    let router = app(AppState::new(
        pool.clone(),
        "01234567890123456789012345678901".into(),
        vec![42; 32],
    ));
    (pool, router)
}

fn peer(last: u8) -> ConnectInfo<SocketAddr> {
    ConnectInfo(SocketAddr::from(([198, 51, 100, last], 40000)))
}

async fn json_post(
    router: &Router,
    path: &str,
    body: Value,
    bearer: Option<&str>,
) -> axum::response::Response {
    let mut request = Request::post(path).header("content-type", "application/json");
    if let Some(token) = bearer {
        request = request.header("authorization", format!("Bearer {token}"));
    }
    let mut request = request.body(Body::from(body.to_string())).unwrap();
    request.extensions_mut().insert(peer(1));
    router.clone().oneshot(request).await.unwrap()
}
async fn value(response: axum::response::Response) -> Value {
    serde_json::from_slice(&to_bytes(response.into_body(), 100000).await.unwrap()).unwrap()
}
async fn text(response: axum::response::Response) -> String {
    String::from_utf8_lossy(&to_bytes(response.into_body(), usize::MAX).await.unwrap()).into_owned()
}
async fn form(router: &Router, path: &str, pairs: &[(&str, &str)]) -> axum::response::Response {
    let body = url::form_urlencoded::Serializer::new(String::new())
        .extend_pairs(pairs)
        .finish();
    router
        .clone()
        .oneshot(
            Request::post(path)
                .header("content-type", "application/x-www-form-urlencoded")
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap()
}
async fn exchange(
    router: &Router,
    code: &str,
    client: &str,
    verifier: &str,
) -> axum::response::Response {
    form(
        router,
        "/api/oauth/token",
        &[
            ("grant_type", "authorization_code"),
            ("code", code),
            ("client_id", client),
            ("redirect_uri", CALLBACK),
            ("code_verifier", verifier),
            ("resource", RESOURCE),
        ],
    )
    .await
}
async fn refresh(router: &Router, refresh_token: &str, client: &str) -> axum::response::Response {
    form(
        router,
        "/api/oauth/token",
        &[
            ("grant_type", "refresh_token"),
            ("refresh_token", refresh_token),
            ("client_id", client),
            ("resource", RESOURCE),
        ],
    )
    .await
}
async fn ping(router: &Router, token: &str) -> u16 {
    json_post(
        router,
        "/api/mcp",
        json!({"jsonrpc":"2.0","id":9,"method":"ping"}),
        Some(token),
    )
    .await
    .status()
    .as_u16()
}

async fn register_user(router: &Router) -> (String, String) {
    let email = format!("oauth-{}@example.com", uuid::Uuid::new_v4());
    let response = json_post(
        router,
        "/api/auth/register",
        json!({"email":email,"password":"long-enough-password"}),
        None,
    )
    .await;
    assert_eq!(response.status(), 201);
    let session = value(response).await["token"].as_str().unwrap().to_owned();
    (email, session)
}
async fn register_client(router: &Router) -> String {
    let client = value(
        json_post(
            router,
            "/api/oauth/register",
            json!({"client_name":"Test agent","redirect_uris":[CALLBACK]}),
            None,
        )
        .await,
    )
    .await;
    client["client_id"].as_str().unwrap().to_owned()
}
fn challenge(verifier: &str) -> String {
    URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()))
}
fn authorize_path(pairs: &[(&str, &str)]) -> String {
    let query = url::form_urlencoded::Serializer::new(String::new())
        .extend_pairs(pairs)
        .finish();
    format!("/api/oauth/authorize?{query}")
}
fn valid_authorize_path(client: &str, verifier: &str) -> String {
    authorize_path(&[
        ("client_id", client),
        ("redirect_uri", CALLBACK),
        ("response_type", "code"),
        ("code_challenge", &challenge(verifier)),
        ("code_challenge_method", "S256"),
        ("resource", RESOURCE),
        ("state", "roundtrip"),
    ])
}
async fn get(router: &Router, path: &str) -> axum::response::Response {
    router
        .clone()
        .oneshot(Request::get(path).body(Body::empty()).unwrap())
        .await
        .unwrap()
}
fn csrf_cookie(response: &axum::response::Response) -> String {
    response.headers()["set-cookie"]
        .to_str()
        .unwrap()
        .split(';')
        .next()
        .unwrap()
        .to_owned()
}
async fn submit(
    router: &Router,
    path: &str,
    cookie: &str,
    pairs: &[(&str, &str)],
) -> axum::response::Response {
    let body = url::form_urlencoded::Serializer::new(String::new())
        .extend_pairs(pairs)
        .finish();
    router
        .clone()
        .oneshot(
            Request::post(path)
                .header("cookie", cookie)
                .header("content-type", "application/x-www-form-urlencoded")
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap()
}
fn location(response: &axum::response::Response) -> url::Url {
    url::Url::parse(response.headers()["location"].to_str().unwrap()).unwrap()
}
fn param(url: &url::Url, name: &str) -> Option<String> {
    url.query_pairs()
        .find(|p| p.0 == name)
        .map(|p| p.1.into_owned())
}

/// Logs in through the consent page and returns the token response.
async fn connect(router: &Router, email: &str, client: &str) -> Value {
    let verifier = "c".repeat(43);
    let path = valid_authorize_path(client, &verifier);
    let page = get(router, &path).await;
    assert_eq!(page.status(), 200);
    let cookie = csrf_cookie(&page);
    let csrf = cookie.strip_prefix("slyt_oauth_csrf=").unwrap().to_owned();
    let approval = submit(
        router,
        &path,
        &cookie,
        &[
            ("email", email),
            ("password", "long-enough-password"),
            ("csrf", &csrf),
        ],
    )
    .await;
    assert_eq!(approval.status(), 303);
    let code = param(&location(&approval), "code").unwrap();
    let response = exchange(router, &code, client, &verifier).await;
    assert_eq!(response.status(), 200);
    value(response).await
}

#[tokio::test]
#[ignore = "requires TEST_DATABASE_URL pointing to a disposable PostgreSQL database"]
async fn login_pkce_single_use_scopes_and_site_setup() {
    let (pool, router) = setup().await;
    let (email, _) = register_user(&router).await;
    let client = register_client(&router).await;
    let client = client.as_str();
    let verifier = "a".repeat(43);
    let path = valid_authorize_path(client, &verifier);
    let page = get(&router, &path).await;
    assert_eq!(page.status(), 200);
    // The form's redirect to the agent callback must be allowed, or browsers block it silently.
    let csp = page.headers()["content-security-policy"]
        .to_str()
        .unwrap()
        .to_owned();
    assert!(
        csp.contains("form-action 'self' http://127.0.0.1:9988;"),
        "{csp}"
    );
    assert!(!csp.contains("unsafe-inline"));
    let cookie = csrf_cookie(&page);
    let csrf = cookie.strip_prefix("slyt_oauth_csrf=").unwrap().to_owned();
    let credentials = [
        ("email", email.as_str()),
        ("password", "long-enough-password"),
        ("csrf", csrf.as_str()),
    ];
    let denied = submit(&router, &path, "", &credentials).await;
    assert_eq!(denied.status(), 403);
    assert!(
        text(denied).await.contains("already used"),
        "CSRF failure renders the page"
    );
    let retry = submit(
        &router,
        &path,
        &cookie,
        &[
            ("email", email.as_str()),
            ("password", "not-the-password"),
            ("csrf", csrf.as_str()),
        ],
    )
    .await;
    assert_eq!(retry.status(), 401);
    assert!(retry.headers()["set-cookie"]
        .to_str()
        .unwrap()
        .starts_with("slyt_oauth_csrf=slyt_"));
    let retry_page = text(retry).await;
    assert!(retry_page.contains("Email or password is incorrect."));
    assert!(
        retry_page.contains(&format!("value=\"{email}\"")),
        "keeps the typed email"
    );
    let approval = submit(&router, &path, &cookie, &credentials).await;
    assert_eq!(approval.status(), 303);
    let redirect = location(&approval);
    let code = param(&redirect, "code").unwrap();
    assert_eq!(param(&redirect, "state").unwrap(), "roundtrip");
    assert_eq!(
        param(&redirect, "iss").unwrap(),
        ISSUER,
        "RFC 9207 issuer identification"
    );
    assert_eq!(
        exchange(&router, &code, client, &"b".repeat(43))
            .await
            .status(),
        400
    );
    let response = exchange(&router, &code, client, &verifier).await;
    assert_eq!(response.status(), 200);
    assert_eq!(response.headers()["cache-control"], "no-store");
    let grant = value(response).await;
    assert_eq!(grant["token_type"], "Bearer");
    assert_eq!(grant["expires_in"], 3600, "short-lived access tokens");
    assert!(grant["refresh_token"]
        .as_str()
        .is_some_and(|v| v.starts_with("slyt_")));
    let token = grant["access_token"].as_str().unwrap();
    let reused = exchange(&router, &code, client, &verifier).await;
    assert_eq!(reused.status(), 400);
    assert_eq!(value(reused).await["error"], "invalid_grant");
    assert_eq!(
        json_post(
            &router,
            "/api/sites",
            json!({"name":"forbidden","domain":"forbidden.example"}),
            Some(token)
        )
        .await
        .status(),
        401
    );
    let domain = format!("{}.example.com", uuid::Uuid::new_v4());
    let call = json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"setup_site","arguments":{"name":"Agent website","domain":domain,"serverType":"nginx"}}});
    let first = value(json_post(&router, "/api/mcp", call.clone(), Some(token)).await).await;
    assert_eq!(first["result"]["structuredContent"]["created"], true);
    assert_eq!(
        first["result"]["structuredContent"]["setup"]["serverType"],
        "nginx"
    );
    let second = value(json_post(&router, "/api/mcp", call, Some(token)).await).await;
    assert_eq!(second["result"]["structuredContent"]["created"], false);
    assert_eq!(
        first["result"]["structuredContent"]["setup"]["siteId"],
        second["result"]["structuredContent"]["setup"]["siteId"]
    );
    sqlx::query(
        "UPDATE api_tokens SET scopes=ARRAY['sites:read','analytics:read'] WHERE token_hash=$1",
    )
    .bind(slimlytics_backend::auth::hash_api_token(token))
    .execute(&pool)
    .await
    .unwrap();
    let readonly = json_post(&router,"/api/mcp",json!({"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"setup_site","arguments":{"name":"Read only","domain":"readonly.example.com"}}}),Some(token)).await;
    assert_eq!(readonly.status(), 403);
    sqlx::query("UPDATE api_tokens SET revoked_at=now() WHERE token_hash=$1")
        .bind(slimlytics_backend::auth::hash_api_token(token))
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(ping(&router, token).await, 401);
}

#[tokio::test]
#[ignore = "requires TEST_DATABASE_URL pointing to a disposable PostgreSQL database"]
async fn refresh_tokens_rotate_and_reuse_revokes_the_connection() {
    let (_pool, router) = setup().await;
    let (email, _) = register_user(&router).await;
    let client = register_client(&router).await;
    let grant = connect(&router, &email, &client).await;
    let first_access = grant["access_token"].as_str().unwrap();
    let first_refresh = grant["refresh_token"].as_str().unwrap();
    assert_eq!(ping(&router, first_access).await, 200);

    let response = refresh(&router, first_refresh, &client).await;
    assert_eq!(response.status(), 200);
    assert_eq!(response.headers()["cache-control"], "no-store");
    let rotated = value(response).await;
    let second_access = rotated["access_token"].as_str().unwrap();
    let second_refresh = rotated["refresh_token"].as_str().unwrap();
    assert_ne!(second_access, first_access);
    assert_ne!(second_refresh, first_refresh, "refresh tokens rotate");
    assert_eq!(rotated["expires_in"], 3600);
    assert_eq!(rotated["scope"], grant["scope"]);
    assert_eq!(ping(&router, second_access).await, 200);
    assert_eq!(
        ping(&router, first_access).await,
        401,
        "refreshing replaces the previous access token"
    );

    let other_client = register_client(&router).await;
    let wrong_client = refresh(&router, second_refresh, &other_client).await;
    assert_eq!(wrong_client.status(), 400);
    assert_eq!(value(wrong_client).await["error"], "invalid_grant");
    assert_eq!(
        ping(&router, second_access).await,
        200,
        "a wrong client_id is not treated as theft"
    );

    // Presenting a rotated-out refresh token signals theft: revoke the whole connection.
    let replay = refresh(&router, first_refresh, &client).await;
    assert_eq!(replay.status(), 400);
    assert_eq!(value(replay).await["error"], "invalid_grant");
    assert_eq!(ping(&router, second_access).await, 401);
    let after = refresh(&router, second_refresh, &client).await;
    assert_eq!(value(after).await["error"], "invalid_grant");
}

#[tokio::test]
#[ignore = "requires TEST_DATABASE_URL pointing to a disposable PostgreSQL database"]
async fn connections_stay_listed_and_revocable_after_the_access_token_expires() {
    let (pool, router) = setup().await;
    let (email, session) = register_user(&router).await;
    let client = register_client(&router).await;
    let grant = connect(&router, &email, &client).await;
    let access = grant["access_token"].as_str().unwrap();
    let refresh_token = grant["refresh_token"].as_str().unwrap();
    sqlx::query(
        "UPDATE api_tokens SET access_expires_at=now()-interval '1 minute' WHERE token_hash=$1",
    )
    .bind(slimlytics_backend::auth::hash_api_token(access))
    .execute(&pool)
    .await
    .unwrap();
    assert_eq!(ping(&router, access).await, 401, "access token expired");

    let listed = router
        .clone()
        .oneshot(
            Request::get("/api/account/tokens")
                .header("authorization", format!("Bearer {session}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let listed = value(listed).await;
    let connections: Vec<&Value> = listed
        .as_array()
        .unwrap()
        .iter()
        .filter(|t| t["name"] == "MCP OAuth agent")
        .collect();
    assert_eq!(
        connections.len(),
        1,
        "an idle connection must stay visible so it can be revoked"
    );
    let id = connections[0]["id"].as_str().unwrap();

    let revoked = router
        .clone()
        .oneshot(
            Request::delete(format!("/api/account/tokens/{id}"))
                .header("authorization", format!("Bearer {session}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(revoked.status(), 204);
    let response = refresh(&router, refresh_token, &client).await;
    assert_eq!(response.status(), 400);
    assert_eq!(
        value(response).await["error"],
        "invalid_grant",
        "revoking the connection also kills its refresh token"
    );
}

#[tokio::test]
#[ignore = "requires TEST_DATABASE_URL pointing to a disposable PostgreSQL database"]
async fn refresh_can_narrow_but_not_widen_scopes() {
    let (_pool, router) = setup().await;
    let (email, _) = register_user(&router).await;
    let client = register_client(&router).await;
    let grant = connect(&router, &email, &client).await;
    let refresh_token = grant["refresh_token"].as_str().unwrap();
    let widened = form(
        &router,
        "/api/oauth/token",
        &[
            ("grant_type", "refresh_token"),
            ("refresh_token", refresh_token),
            ("client_id", &client),
            ("scope", "analytics:read integrations:read"),
        ],
    )
    .await;
    assert_eq!(widened.status(), 400);
    assert_eq!(value(widened).await["error"], "invalid_scope");
    let narrowed = form(
        &router,
        "/api/oauth/token",
        &[
            ("grant_type", "refresh_token"),
            ("refresh_token", refresh_token),
            ("client_id", &client),
            ("scope", "analytics:read"),
        ],
    )
    .await;
    assert_eq!(narrowed.status(), 200);
    assert_eq!(value(narrowed).await["scope"], "analytics:read");
}

#[tokio::test]
#[ignore = "requires TEST_DATABASE_URL pointing to a disposable PostgreSQL database"]
async fn client_registration_is_rate_limited_per_ip() {
    let (_pool, router) = setup().await;
    let register = |last: u8| {
        let router = router.clone();
        async move {
            let mut request = Request::post("/api/oauth/register")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({"client_name":"Burst","redirect_uris":[CALLBACK]}).to_string(),
                ))
                .unwrap();
            request.extensions_mut().insert(peer(last));
            router.oneshot(request).await.unwrap().status().as_u16()
        }
    };
    for _ in 0..20 {
        assert_eq!(register(50).await, 201);
    }
    assert_eq!(register(50).await, 429);
    assert_eq!(
        register(51).await,
        201,
        "one noisy address must not block registration for everyone"
    );
    let registered = value(
        json_post(
            &router,
            "/api/oauth/register",
            json!({"redirect_uris":[CALLBACK],"grant_types":["authorization_code","refresh_token"]}),
            None,
        )
        .await,
    )
    .await;
    assert_eq!(
        registered["grant_types"],
        json!(["authorization_code", "refresh_token"])
    );
}

#[tokio::test]
#[ignore = "requires TEST_DATABASE_URL pointing to a disposable PostgreSQL database"]
async fn authorization_errors_redirect_once_the_callback_is_trusted() {
    let (_pool, router) = setup().await;
    let client = register_client(&router).await;
    let verifier = "d".repeat(43);
    let good_challenge = challenge(&verifier);
    let base = [
        ("client_id", client.as_str()),
        ("redirect_uri", CALLBACK),
        ("response_type", "code"),
        ("code_challenge", good_challenge.as_str()),
        ("code_challenge_method", "S256"),
        ("resource", RESOURCE),
        ("state", "keep-me"),
    ];
    let with = |name: &str, value: Option<&str>| -> String {
        let pairs: Vec<(&str, &str)> = base
            .iter()
            .filter(|(key, _)| *key != name)
            .copied()
            .chain(value.map(|v| (name, v)))
            .collect();
        authorize_path(&pairs)
    };
    for (path, expected) in [
        (with("code_challenge", None), "invalid_request"),
        (
            with("code_challenge_method", Some("plain")),
            "invalid_request",
        ),
        (
            with("response_type", Some("token")),
            "unsupported_response_type",
        ),
        (
            with("resource", Some("https://elsewhere.example/api/mcp")),
            "invalid_target",
        ),
        (with("scope", Some("analytics:write")), "invalid_scope"),
    ] {
        let response = get(&router, &path).await;
        assert_eq!(response.status(), 303, "{expected}: {path}");
        let redirect = location(&response);
        assert_eq!(
            format!(
                "{}://{}:{}{}",
                redirect.scheme(),
                redirect.host_str().unwrap(),
                redirect.port().unwrap(),
                redirect.path()
            ),
            CALLBACK
        );
        assert_eq!(param(&redirect, "error").unwrap(), expected, "{path}");
        assert_eq!(param(&redirect, "state").unwrap(), "keep-me");
        assert_eq!(param(&redirect, "iss").unwrap(), ISSUER);
        assert!(param(&redirect, "code").is_none());
    }
    // Never redirect to a callback that was not registered, or for an unknown client.
    let unregistered = get(
        &router,
        &with("redirect_uri", Some("https://evil.example/cb")),
    )
    .await;
    assert_eq!(unregistered.status(), 400);
    assert!(unregistered.headers().get("location").is_none());
    let unknown = get(
        &router,
        &with("client_id", Some("00000000-0000-0000-0000-000000000000")),
    )
    .await;
    assert_eq!(unknown.status(), 400);
    assert!(unknown.headers().get("location").is_none());
}

#[tokio::test]
#[ignore = "requires TEST_DATABASE_URL pointing to a disposable PostgreSQL database"]
async fn cancel_returns_access_denied_to_the_agent() {
    let (_pool, router) = setup().await;
    let client = register_client(&router).await;
    let path = valid_authorize_path(&client, &"e".repeat(43));
    let page = get(&router, &path).await;
    let cookie = csrf_cookie(&page);
    let csrf = cookie.strip_prefix("slyt_oauth_csrf=").unwrap().to_owned();
    let html = text(page).await;
    assert!(html.contains("value=deny"), "consent page offers Cancel");
    let forged = submit(&router, &path, "", &[("csrf", &csrf), ("action", "deny")]).await;
    assert_eq!(forged.status(), 403, "cancel still requires CSRF");
    let cancelled = submit(
        &router,
        &path,
        &cookie,
        &[("csrf", &csrf), ("action", "deny")],
    )
    .await;
    assert_eq!(cancelled.status(), 303);
    let redirect = location(&cancelled);
    assert_eq!(param(&redirect, "error").unwrap(), "access_denied");
    assert_eq!(param(&redirect, "state").unwrap(), "roundtrip");
    assert!(param(&redirect, "code").is_none());
}

#[tokio::test]
#[ignore = "requires TEST_DATABASE_URL pointing to a disposable PostgreSQL database"]
async fn maintenance_keeps_replay_detection_for_active_connections() {
    let (pool, router) = setup().await;
    let (email, _) = register_user(&router).await;
    let client = register_client(&router).await;
    let grant = connect(&router, &email, &client).await;
    let stolen = grant["refresh_token"].as_str().unwrap().to_owned();
    // An attacker rotates the stolen token and keeps the connection alive for weeks.
    let rotated = value(refresh(&router, &stolen, &client).await).await;
    let current_access = rotated["access_token"].as_str().unwrap();
    sqlx::query(
        "UPDATE oauth_refresh_tokens SET used_at=now()-interval '30 days' WHERE token_hash=$1",
    )
    .bind(slimlytics_backend::auth::hash_api_token(&stolen))
    .execute(&pool)
    .await
    .unwrap();
    slimlytics_backend::prune_oauth_state(&pool).await.unwrap();
    // The legitimate client comes back with its old token: that must still revoke the thief.
    let replay = refresh(&router, &stolen, &client).await;
    assert_eq!(value(replay).await["error"], "invalid_grant");
    assert_eq!(ping(&router, current_access).await, 401);
}

#[tokio::test]
#[ignore = "requires TEST_DATABASE_URL pointing to a disposable PostgreSQL database"]
async fn maintenance_does_not_prune_a_connection_being_renewed() {
    let (pool, router) = setup().await;
    let (email, _) = register_user(&router).await;
    let client = register_client(&router).await;
    let grant = connect(&router, &email, &client).await;
    let rotated =
        value(refresh(&router, grant["refresh_token"].as_str().unwrap(), &client).await).await;
    let connection: uuid::Uuid =
        sqlx::query_scalar("SELECT id FROM api_tokens WHERE token_hash=$1")
            .bind(slimlytics_backend::auth::hash_api_token(
                rotated["access_token"].as_str().unwrap(),
            ))
            .fetch_one(&pool)
            .await
            .unwrap();
    sqlx::query("UPDATE api_tokens SET expires_at=created_at+interval '1 microsecond' WHERE id=$1")
        .bind(connection)
        .execute(&pool)
        .await
        .unwrap();
    // A refresh that began just before expiry holds the connection row while it renews it.
    let mut renewal = pool.begin().await.unwrap();
    sqlx::query("UPDATE api_tokens SET expires_at=now()+interval '90 days' WHERE id=$1")
        .bind(connection)
        .execute(&mut *renewal)
        .await
        .unwrap();
    tokio::time::timeout(
        std::time::Duration::from_secs(10),
        slimlytics_backend::prune_oauth_state(&pool),
    )
    .await
    .expect("pruning must not block on an in-flight refresh")
    .unwrap();
    renewal.commit().await.unwrap();
    let kept: i64 =
        sqlx::query_scalar("SELECT count(*) FROM oauth_refresh_tokens WHERE api_token_id=$1")
            .bind(connection)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(
        kept, 2,
        "the renewed connection keeps its rotated-out token for replay detection"
    );
}

#[tokio::test]
#[ignore = "requires TEST_DATABASE_URL pointing to a disposable PostgreSQL database"]
async fn concurrent_replay_and_refresh_never_fail_with_a_server_error() {
    let (_pool, router) = setup().await;
    let client = register_client(&router).await;
    for _ in 0..15 {
        // Fresh account each round: logins are rate limited per email.
        let (email, _) = register_user(&router).await;
        let grant = connect(&router, &email, &client).await;
        let old = grant["refresh_token"].as_str().unwrap().to_owned();
        let rotated = value(refresh(&router, &old, &client).await).await;
        let current = rotated["refresh_token"].as_str().unwrap().to_owned();
        let access = rotated["access_token"].as_str().unwrap().to_owned();
        let (replay, fresh) = tokio::join!(
            refresh(&router, &old, &client),
            refresh(&router, &current, &client)
        );
        assert_eq!(replay.status(), 400, "replay is rejected, not a deadlock");
        assert!(
            fresh.status() == 200 || fresh.status() == 400,
            "refresh racing a replay must not error: {}",
            fresh.status()
        );
        let fresh = value(fresh).await;
        // Whatever the ordering, the replay must leave the connection revoked.
        let survivor = fresh["access_token"].as_str().unwrap_or(&access).to_owned();
        assert_eq!(ping(&router, &survivor).await, 401);
    }
}

#[tokio::test]
#[ignore = "requires TEST_DATABASE_URL pointing to a disposable PostgreSQL database"]
async fn maintenance_prunes_abandoned_oauth_state() {
    let (pool, router) = setup().await;
    let (email, _) = register_user(&router).await;
    let abandoned = register_client(&router).await;
    let used = register_client(&router).await;
    connect(&router, &email, &used).await;
    sqlx::query("UPDATE oauth_clients SET created_at=now()-interval '2 days' WHERE id=ANY($1)")
        .bind(vec![
            uuid::Uuid::parse_str(&abandoned).unwrap(),
            uuid::Uuid::parse_str(&used).unwrap(),
        ])
        .execute(&pool)
        .await
        .unwrap();
    slimlytics_backend::prune_oauth_state(&pool).await.unwrap();
    let remaining: Vec<uuid::Uuid> =
        sqlx::query_scalar("SELECT id FROM oauth_clients WHERE id=ANY($1)")
            .bind(vec![
                uuid::Uuid::parse_str(&abandoned).unwrap(),
                uuid::Uuid::parse_str(&used).unwrap(),
            ])
            .fetch_all(&pool)
            .await
            .unwrap();
    assert_eq!(
        remaining,
        vec![uuid::Uuid::parse_str(&used).unwrap()],
        "only never-used registrations are pruned"
    );
}

#[tokio::test]
#[ignore = "requires TEST_DATABASE_URL pointing to a disposable PostgreSQL database"]
async fn disabled_accounts_cannot_exchange_codes_or_refresh() {
    let (pool, router) = setup().await;
    let (email, _) = register_user(&router).await;
    let client = register_client(&router).await;
    let grant = connect(&router, &email, &client).await;
    let refresh_token = grant["refresh_token"].as_str().unwrap();

    // Approve a second connection but hold its code until after the account is disabled.
    let verifier = "d".repeat(43);
    let path = valid_authorize_path(&client, &verifier);
    let page = get(&router, &path).await;
    let cookie = csrf_cookie(&page);
    let csrf = cookie.strip_prefix("slyt_oauth_csrf=").unwrap().to_owned();
    let approval = submit(
        &router,
        &path,
        &cookie,
        &[
            ("email", &email),
            ("password", "long-enough-password"),
            ("csrf", &csrf),
        ],
    )
    .await;
    let code = param(&location(&approval), "code").unwrap();

    sqlx::query("UPDATE users SET disabled_at=now() WHERE email=$1")
        .bind(&email)
        .execute(&pool)
        .await
        .unwrap();
    let exchanged = exchange(&router, &code, &client, &verifier).await;
    assert_eq!(exchanged.status(), 400);
    assert_eq!(value(exchanged).await["error"], "invalid_grant");
    let refreshed = refresh(&router, refresh_token, &client).await;
    assert_eq!(refreshed.status(), 400);
    assert_eq!(value(refreshed).await["error"], "invalid_grant");
}
