use axum::{
    body::{to_bytes, Body},
    http::Request,
    Router,
};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use slimlytics_backend::{app, AppState};
use sqlx::postgres::PgPoolOptions;
use tower::ServiceExt;

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
    router
        .clone()
        .oneshot(request.body(Body::from(body.to_string())).unwrap())
        .await
        .unwrap()
}
async fn value(response: axum::response::Response) -> Value {
    serde_json::from_slice(&to_bytes(response.into_body(), 100000).await.unwrap()).unwrap()
}
async fn exchange(
    router: &Router,
    code: &str,
    client: &str,
    verifier: &str,
) -> axum::response::Response {
    let form = url::form_urlencoded::Serializer::new(String::new())
        .extend_pairs([
            ("grant_type", "authorization_code"),
            ("code", code),
            ("client_id", client),
            ("redirect_uri", "http://127.0.0.1:9988/cb"),
            ("code_verifier", verifier),
            ("resource", "http://localhost:8080/api/mcp"),
        ])
        .finish();
    router
        .clone()
        .oneshot(
            Request::post("/api/oauth/token")
                .header("content-type", "application/x-www-form-urlencoded")
                .body(Body::from(form))
                .unwrap(),
        )
        .await
        .unwrap()
}
#[tokio::test]
#[ignore = "requires TEST_DATABASE_URL pointing to a disposable PostgreSQL database"]
async fn login_pkce_single_use_scopes_and_site_setup() {
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
    let email = format!("oauth-{}@example.com", uuid::Uuid::new_v4());
    assert_eq!(
        json_post(
            &router,
            "/api/auth/register",
            json!({"email":email,"password":"long-enough-password"}),
            None
        )
        .await
        .status(),
        201
    );
    let client = value(
        json_post(
            &router,
            "/api/oauth/register",
            json!({"client_name":"Test agent","redirect_uris":["http://127.0.0.1:9988/cb"]}),
            None,
        )
        .await,
    )
    .await;
    let client = client["client_id"].as_str().unwrap();
    let verifier = "a".repeat(43);
    let challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()));
    let query = url::form_urlencoded::Serializer::new(String::new())
        .extend_pairs([
            ("client_id", client),
            ("redirect_uri", "http://127.0.0.1:9988/cb"),
            ("response_type", "code"),
            ("code_challenge", &challenge),
            ("code_challenge_method", "S256"),
            ("resource", "http://localhost:8080/api/mcp"),
            ("state", "roundtrip"),
        ])
        .finish();
    let path = format!("/api/oauth/authorize?{query}");
    let page = router
        .clone()
        .oneshot(Request::get(&path).body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(page.status(), 200);
    let cookie = page.headers()["set-cookie"]
        .to_str()
        .unwrap()
        .split(';')
        .next()
        .unwrap()
        .to_owned();
    let csrf = cookie.strip_prefix("slyt_oauth_csrf=").unwrap();
    let form = url::form_urlencoded::Serializer::new(String::new())
        .extend_pairs([
            ("email", email.as_str()),
            ("password", "long-enough-password"),
            ("csrf", csrf),
        ])
        .finish();
    let denied = router
        .clone()
        .oneshot(
            Request::post(&path)
                .header("content-type", "application/x-www-form-urlencoded")
                .body(Body::from(form.clone()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(denied.status(), 403);
    let approval = router
        .clone()
        .oneshot(
            Request::post(&path)
                .header("cookie", cookie)
                .header("content-type", "application/x-www-form-urlencoded")
                .body(Body::from(form))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(approval.status(), 303);
    let redirect = url::Url::parse(approval.headers()["location"].to_str().unwrap()).unwrap();
    let code = redirect
        .query_pairs()
        .find(|p| p.0 == "code")
        .unwrap()
        .1
        .to_string();
    assert_eq!(
        redirect.query_pairs().find(|p| p.0 == "state").unwrap().1,
        "roundtrip"
    );
    assert_eq!(
        exchange(&router, &code, client, &"b".repeat(43))
            .await
            .status(),
        400
    );
    let response = exchange(&router, &code, client, &verifier).await;
    assert_eq!(response.status(), 200);
    let token = value(response).await;
    let token = token["access_token"].as_str().unwrap();
    assert_eq!(
        exchange(&router, &code, client, &verifier).await.status(),
        400
    );
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
    assert_eq!(
        json_post(
            &router,
            "/api/mcp",
            json!({"jsonrpc":"2.0","id":3,"method":"ping"}),
            Some(token)
        )
        .await
        .status(),
        401
    );
}
