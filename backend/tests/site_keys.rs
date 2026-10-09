use axum::{
    body::{to_bytes, Body},
    http::{header, Request, StatusCode},
    Router,
};
use serde_json::{json, Value};
use slimlytics_backend::{app, AppState};
use sqlx::postgres::PgPoolOptions;
use tower::ServiceExt;

/// The site read endpoints include `serverWriteKey` and `proxyKey` only for owners and admins,
/// and for API tokens only with `sites:write`. Everyone else gets the last-four hints, and
/// owners and admins can fetch the keys from `POST /api/sites/{id}/keys`.
#[tokio::test]
#[ignore = "requires TEST_DATABASE_URL pointing to a disposable PostgreSQL database"]
async fn site_key_fields_follow_role_and_token_scope() {
    let url = std::env::var("TEST_DATABASE_URL").expect("TEST_DATABASE_URL required");
    let pool = PgPoolOptions::new().connect(&url).await.unwrap();
    sqlx::migrate!("../migrations").run(&pool).await.unwrap();
    let router = app(AppState::new(
        pool.clone(),
        "01234567890123456789012345678901".into(),
        b"abcdefghijklmnopqrstuvwxyz012345".to_vec(),
    ));

    let owner = register(&router).await;
    let domain = format!("{}.example.com", uuid::Uuid::new_v4());
    let created = call(
        &router,
        "POST",
        "/api/sites",
        &owner,
        json!({"name":"Keys","domain":domain,"timezone":"UTC","allowed_origins":[format!("https://{domain}")]}),
    )
    .await;
    assert_eq!(created.0, StatusCode::CREATED);
    let site_id = created.1["id"].as_str().unwrap().to_owned();
    let server_key = created.1["serverWriteKey"].as_str().unwrap().to_owned();
    let proxy_key = created.1["proxyKey"].as_str().unwrap().to_owned();
    let hint = |key: &str| format!("…{}", &key[key.len() - 4..]);
    let site_path = format!("/api/sites/{site_id}");
    let keys_path = format!("/api/sites/{site_id}/keys");

    let assert_redacted = |site: &Value| {
        assert!(site.get("serverWriteKey").is_none(), "{site}");
        assert!(site.get("proxyKey").is_none(), "{site}");
        assert!(!site.to_string().contains(&server_key));
        assert!(!site.to_string().contains(&proxy_key));
        assert_eq!(site["serverWriteKeyHint"], hint(&server_key));
        assert_eq!(site["proxyKeyHint"], hint(&proxy_key));
        assert_eq!(site["canManageKeys"], false);
        assert!(
            site["writeKey"].is_string(),
            "the browser key stays visible"
        );
    };
    let assert_included = |site: &Value| {
        assert_eq!(site["serverWriteKey"], server_key.as_str());
        assert_eq!(site["proxyKey"], proxy_key.as_str());
        assert_eq!(site["proxyKeyHint"], hint(&proxy_key));
        assert_eq!(site["canManageKeys"], true);
    };

    // Owner, dashboard session: keys included.
    let (status, list) = call(&router, "GET", "/api/sites", &owner, json!({})).await;
    assert_eq!(status, StatusCode::OK);
    assert_included(&list[0]);
    assert_included(&call(&router, "GET", &site_path, &owner, json!({})).await.1);

    // Owner, read-only API token: hints only, and the keys endpoint needs sites:write.
    let read_token = token(&router, &owner, &["sites:read"]).await;
    let (_, list) = call(&router, "GET", "/api/sites", &read_token, json!({})).await;
    assert_redacted(&list[0]);
    let (status, site) = call(&router, "GET", &site_path, &read_token, json!({})).await;
    assert_eq!(status, StatusCode::OK);
    assert_redacted(&site);
    assert_eq!(
        call(&router, "POST", &keys_path, &read_token, json!({}))
            .await
            .0,
        StatusCode::FORBIDDEN
    );

    // Owner, API token with sites:write: keys included and available from the keys endpoint.
    let write_token = token(&router, &owner, &["sites:read", "sites:write"]).await;
    assert_included(
        &call(&router, "GET", &site_path, &write_token, json!({}))
            .await
            .1,
    );
    let (status, keys) = call(&router, "POST", &keys_path, &write_token, json!({})).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(keys["serverWriteKey"], server_key.as_str());
    assert_eq!(keys["proxyKey"], proxy_key.as_str());
    assert!(keys["writeKey"].is_string());

    // Viewer, dashboard session: hints only, and no keys endpoint.
    let viewer = register(&router).await;
    let (_, me) = call(&router, "GET", "/api/auth/me", &viewer, json!({})).await;
    sqlx::query("INSERT INTO site_memberships(site_id,user_id,role) VALUES($1,$2,'viewer')")
        .bind(uuid::Uuid::parse_str(&site_id).unwrap())
        .bind(uuid::Uuid::parse_str(me["id"].as_str().unwrap()).unwrap())
        .execute(&pool)
        .await
        .unwrap();
    let (_, list) = call(&router, "GET", "/api/sites", &viewer, json!({})).await;
    assert_redacted(&list[0]);
    assert_redacted(&call(&router, "GET", &site_path, &viewer, json!({})).await.1);
    assert_eq!(
        call(&router, "POST", &keys_path, &viewer, json!({}))
            .await
            .0,
        StatusCode::FORBIDDEN
    );
}

async fn register(router: &Router) -> String {
    let email = format!("keys+{}@example.com", uuid::Uuid::new_v4());
    let (status, body) = call_with(
        router,
        "POST",
        "/api/auth/register",
        None,
        json!({"email":email,"password":"long-enough-password"}),
    )
    .await;
    assert!(status.is_success(), "{status}: {body}");
    body["token"].as_str().unwrap().to_owned()
}

async fn token(router: &Router, session: &str, scopes: &[&str]) -> String {
    let (status, body) = call(
        router,
        "POST",
        "/api/account/tokens",
        session,
        json!({"name":"keys test","expiresInDays":1,"scopes":scopes}),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{body}");
    body["token"].as_str().unwrap().to_owned()
}

async fn call(
    router: &Router,
    method: &str,
    uri: &str,
    token: &str,
    body: Value,
) -> (StatusCode, Value) {
    call_with(router, method, uri, Some(token), body).await
}

async fn call_with(
    router: &Router,
    method: &str,
    uri: &str,
    token: Option<&str>,
    body: Value,
) -> (StatusCode, Value) {
    let mut builder = Request::builder()
        .method(method)
        .uri(uri)
        .header(header::CONTENT_TYPE, "application/json");
    if let Some(token) = token {
        builder = builder.header(header::AUTHORIZATION, format!("Bearer {token}"));
    }
    let request = if method == "GET" {
        builder.body(Body::empty()).unwrap()
    } else {
        builder.body(Body::from(body.to_string())).unwrap()
    };
    let response = router.clone().oneshot(request).await.unwrap();
    let status = response.status();
    let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}
