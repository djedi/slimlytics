use axum::{
    body::{to_bytes, Body},
    http::Request,
};
use slimlytics_backend::{app, AppState};
use sqlx::postgres::PgPoolOptions;
use tower::ServiceExt;

fn state() -> AppState {
    let pool = PgPoolOptions::new()
        .connect_lazy("postgres://unused:***@localhost/unused")
        .unwrap();
    AppState::new(
        pool,
        "test-secret-at-least-32-characters".into(),
        b"identity-secret".to_vec(),
    )
}

#[tokio::test]
async fn liveness_does_not_depend_on_database() {
    let response = app(state())
        .oneshot(Request::get("/health").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(response.status(), 200);
}

#[tokio::test]
async fn protected_routes_require_bearer_token() {
    let response = app(state())
        .oneshot(Request::get("/api/sites").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(response.status(), 401);
}

#[tokio::test]
async fn neutral_collection_alias_is_routed() {
    let response = app(state())
        .oneshot(
            Request::post("/api/e/not-a-uuid")
                .header("content-type", "application/json")
                .body(Body::from("{}"))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), 400);
}

#[tokio::test]
async fn collection_proxy_test_is_routed() {
    let response = app(state())
        .oneshot(
            Request::get("/api/collect/not-a-uuid")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), 400);
}

#[tokio::test]
async fn anti_adblock_configuration_requires_authentication() {
    let response = app(state())
        .oneshot(
            Request::put("/api/sites/00000000-0000-4000-8000-000000000000/anti-adblock")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"serverType":"caddy","jsPath":"/456bbb63bb86.js","beaconPath":"/0d31360a3101"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), 401);
}

#[tokio::test]
async fn site_icon_settings_require_authentication() {
    let response = app(state())
        .oneshot(
            Request::put("/api/sites/00000000-0000-4000-8000-000000000000/icon")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"mode":"initials"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), 401);
}

#[tokio::test]
async fn api_token_management_requires_a_session() {
    let response = app(state())
        .oneshot(
            Request::post("/api/account/tokens")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"name":"agent"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), 401);
}

#[tokio::test]
async fn openapi_document_covers_every_public_backend_route() {
    let response = app(state())
        .oneshot(
            Request::get("/api/openapi.json")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), 200);
    assert_eq!(
        response.headers()["content-type"],
        "application/vnd.oai.openapi+json"
    );
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let document: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(document["openapi"], "3.1.0");
    assert_eq!(document["info"]["title"], "Slimlytics API");

    let required_operations = [
        ("/health", "get"),
        ("/ready", "get"),
        ("/api/openapi.json", "get"),
        ("/api/docs", "get"),
        ("/api/auth/register", "post"),
        ("/api/auth/login", "post"),
        ("/api/auth/me", "get"),
        ("/api/account/tokens", "get"),
        ("/api/account/tokens", "post"),
        ("/api/account/tokens/current", "delete"),
        ("/api/account/tokens/{tokenId}", "delete"),
        ("/api/sites", "get"),
        ("/api/sites", "post"),
        ("/api/sites/ensure", "post"),
        ("/api/sites/{siteId}", "get"),
        ("/api/sites/{siteId}", "put"),
        ("/api/sites/{siteId}", "delete"),
        ("/api/sites/{siteId}/keys", "post"),
        ("/api/sites/{siteId}/rotate-key", "post"),
        ("/api/sites/{siteId}/rotate-server-key", "post"),
        ("/api/sites/{siteId}/rotate-proxy-key", "post"),
        ("/api/billing/plans", "get"),
        ("/api/billing", "get"),
        ("/api/billing/checkout", "post"),
        ("/api/billing/portal", "post"),
        ("/api/billing/webhook", "post"),
        ("/api/sites/{siteId}/anti-adblock", "put"),
        ("/api/sites/{siteId}/icon", "get"),
        ("/api/sites/{siteId}/icon", "put"),
        ("/api/sites/{siteId}/collection-health", "get"),
        ("/api/sites/{siteId}/overview", "get"),
        ("/api/sites/{siteId}/reports/{dimension}", "get"),
        ("/api/sites/{siteId}/insights/journeys", "get"),
        ("/api/sites/{siteId}/insights/attribution", "get"),
        ("/api/sites/{siteId}/insights/anomalies", "get"),
        ("/api/sites/{siteId}/annotations", "get"),
        ("/api/sites/{siteId}/annotations", "post"),
        ("/api/sites/{siteId}/annotations/{annotationId}", "delete"),
        ("/api/sites/{siteId}/funnels", "get"),
        ("/api/sites/{siteId}/funnels", "post"),
        ("/api/sites/{siteId}/funnels/{funnelId}", "delete"),
        ("/api/sites/{siteId}/funnels/{funnelId}/report", "get"),
        ("/api/sites/{siteId}/report-subscriptions", "get"),
        ("/api/sites/{siteId}/report-subscriptions", "post"),
        (
            "/api/sites/{siteId}/report-subscriptions/{subscriptionId}",
            "put",
        ),
        (
            "/api/sites/{siteId}/report-subscriptions/{subscriptionId}",
            "delete",
        ),
        (
            "/api/sites/{siteId}/report-subscriptions/{subscriptionId}/deliver",
            "post",
        ),
        (
            "/api/sites/{siteId}/report-subscriptions/{subscriptionId}/deliveries",
            "get",
        ),
        ("/api/sites/{siteId}/integrations/search-console", "get"),
        ("/api/sites/{siteId}/integrations/search-console", "delete"),
        (
            "/api/sites/{siteId}/integrations/search-console/connect",
            "post",
        ),
        (
            "/api/sites/{siteId}/integrations/search-console/sync",
            "post",
        ),
        ("/api/sites/{siteId}/reports/search-console", "get"),
        ("/api/integrations/search-console/callback", "get"),
        ("/api/mcp", "post"),
        ("/api/sites/{siteId}/visitors", "get"),
        ("/api/sites/{siteId}/visitors/{visitorId}", "get"),
        ("/api/sites/{siteId}/events", "get"),
        ("/api/sites/{siteId}/goals", "get"),
        ("/api/sites/{siteId}/goals", "post"),
        ("/api/sites/{siteId}/goals/{goalId}", "delete"),
        ("/api/sites/{siteId}/export.csv", "get"),
        ("/api/sites/{siteId}/stream", "get"),
        ("/api/collect/{writeKey}", "post"),
        ("/api/collect/{writeKey}", "options"),
        ("/api/e/{writeKey}", "post"),
        ("/api/e/{writeKey}", "options"),
        ("/api/ingest", "post"),
        ("/api/auth/refresh", "post"),
        ("/api/auth/logout", "post"),
        ("/api/auth/passkey/start", "post"),
        ("/api/auth/passkey/finish", "post"),
        ("/api/auth/mfa/start", "post"),
        ("/api/auth/mfa/finish", "post"),
        ("/api/account/sessions", "get"),
        ("/api/account/sessions/{sessionId}", "delete"),
        ("/api/account/passkeys", "get"),
        ("/api/account/passkeys/register/start", "post"),
        ("/api/account/passkeys/register/finish", "post"),
        ("/api/account/passkeys/{passkeyId}", "delete"),
        ("/api/admin/overview", "get"),
        ("/api/admin/users", "get"),
        ("/api/admin/users/{userId}", "get"),
        ("/api/admin/users/{userId}", "delete"),
        ("/api/admin/users/{userId}/disable", "post"),
        ("/api/admin/users/{userId}/enable", "post"),
        ("/api/admin/users/{userId}/revoke-sessions", "post"),
        ("/api/admin/audit", "get"),
    ];
    for (path, method) in required_operations {
        assert!(
            document["paths"][path].get(method).is_some(),
            "OpenAPI is missing {method} {path}"
        );
    }
    for item in document["paths"].as_object().unwrap().values() {
        for operation in item.as_object().unwrap().values() {
            if let Some(responses) = operation.get("responses") {
                assert!(responses.get("409").is_none(), "API emits 400, not 409");
            }
        }
    }
    assert_eq!(
        document["paths"]["/api/sites/{siteId}/rotate-key"]["post"]["responses"]["200"]["content"]
            ["application/json"]["schema"]["$ref"],
        "#/components/schemas/WriteKeyResponse"
    );
    assert_eq!(
        document["paths"]["/api/sites/{siteId}/visitors/{visitorId}"]["get"]["responses"]["200"]
            ["content"]["application/json"]["schema"]["type"],
        "array"
    );
    let tracker = &document["paths"]["/p/{writeKey}/{beaconName}"]["get"]["responses"];
    assert!(tracker.get("304").is_some());
    assert!(tracker.get("404").is_none());
    assert!(tracker["200"]["content"].get("text/javascript").is_some());
    assert!(tracker["400"]["content"].get("text/plain").is_some());
    assert_eq!(
        document["components"]["securitySchemes"]["bearerAuth"]["type"],
        "http"
    );
    assert_eq!(
        document["paths"]["/health"]["get"]["responses"]
            .as_object()
            .unwrap()
            .keys()
            .collect::<Vec<_>>(),
        vec!["200"]
    );
    assert_eq!(
        document["components"]["schemas"]["Visitor"]["additionalProperties"],
        false
    );
    assert_eq!(
        document["components"]["schemas"]["Event"]["additionalProperties"],
        false
    );
    let get_site_parameters = document["paths"]["/api/sites/{siteId}"]["get"]["parameters"]
        .as_array()
        .unwrap();
    let site_id = get_site_parameters
        .iter()
        .find(|parameter| parameter["name"] == "siteId")
        .unwrap();
    assert_eq!(site_id["schema"]["format"], "uuid");
    let collect_responses = &document["paths"]["/api/collect/{writeKey}"]["post"]["responses"];
    assert!(collect_responses["400"]["content"]
        .get("text/plain")
        .is_some());
    assert!(collect_responses["415"]["content"]
        .get("text/plain")
        .is_some());
    assert!(collect_responses["422"]["content"]
        .get("text/plain")
        .is_some());
    for path in ["/api/collect/{writeKey}", "/api/e/{writeKey}"] {
        assert!(
            document["paths"][path]["get"]["responses"]["403"]["content"]
                .get("application/json")
                .is_some()
        );
    }
}

#[tokio::test]
async fn scalar_ui_redirects_to_the_locally_bundled_reference() {
    let response = app(state())
        .oneshot(Request::get("/api/docs").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(response.status(), 307);
    assert_eq!(response.headers()["location"], "/docs/api");
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let html = String::from_utf8(body.to_vec()).unwrap();
    assert!(!html.contains("cdn.jsdelivr.net"));
}

#[tokio::test]
async fn mcp_discovers_oauth_without_database() {
    let response = app(state())
        .oneshot(
            Request::get("/.well-known/oauth-protected-resource/api/mcp")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), 200);
    let body: serde_json::Value =
        serde_json::from_slice(&to_bytes(response.into_body(), 10000).await.unwrap()).unwrap();
    assert!(body["resource"].as_str().unwrap().ends_with("/api/mcp"));
    let response = app(state())
        .oneshot(
            Request::post("/api/mcp")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"jsonrpc":"2.0","id":1,"method":"initialize"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), 401);
    assert!(response.headers()["www-authenticate"]
        .to_str()
        .unwrap()
        .contains("resource_metadata"));
}

async fn body_json(response: axum::response::Response) -> serde_json::Value {
    serde_json::from_slice(&to_bytes(response.into_body(), 100_000).await.unwrap()).unwrap()
}

fn form_post(path: &str, pairs: &[(&str, &str)]) -> Request<Body> {
    let form = url::form_urlencoded::Serializer::new(String::new())
        .extend_pairs(pairs)
        .finish();
    Request::post(path)
        .header("content-type", "application/x-www-form-urlencoded")
        .body(Body::from(form))
        .unwrap()
}

#[tokio::test]
async fn oauth_metadata_advertises_refresh_tokens_and_issuer_identification() {
    let response = app(state())
        .oneshot(
            Request::get("/.well-known/oauth-authorization-server")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), 200);
    let body = body_json(response).await;
    assert_eq!(
        body["grant_types_supported"],
        serde_json::json!(["authorization_code", "refresh_token"])
    );
    assert_eq!(body["authorization_response_iss_parameter_supported"], true);
}

#[tokio::test]
async fn oauth_discovery_and_token_endpoints_allow_browser_clients() {
    for path in [
        "/.well-known/oauth-authorization-server",
        "/.well-known/oauth-protected-resource/api/mcp",
        "/api/oauth/register",
        "/api/oauth/token",
    ] {
        let preflight = app(state())
            .oneshot(
                Request::options(path)
                    .header("origin", "https://inspector.example")
                    .header("access-control-request-method", "POST")
                    .header("access-control-request-headers", "content-type")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert!(
            preflight.status().is_success(),
            "{path}: {}",
            preflight.status()
        );
        assert_eq!(
            preflight.headers()["access-control-allow-origin"],
            "*",
            "{path}"
        );
    }
    let metadata = app(state())
        .oneshot(
            Request::get("/.well-known/oauth-authorization-server")
                .header("origin", "https://inspector.example")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(metadata.headers()["access-control-allow-origin"], "*");
    // The consent page carries a CSRF cookie and must never be readable cross-origin.
    let authorize = app(state())
        .oneshot(
            Request::options("/api/oauth/authorize")
                .header("origin", "https://inspector.example")
                .header("access-control-request-method", "POST")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert!(authorize
        .headers()
        .get("access-control-allow-origin")
        .is_none());
}

#[tokio::test]
async fn oauth_token_endpoint_returns_rfc6749_errors() {
    let unsupported = app(state())
        .oneshot(form_post(
            "/api/oauth/token",
            &[("grant_type", "client_credentials")],
        ))
        .await
        .unwrap();
    assert_eq!(unsupported.status(), 400);
    assert_eq!(unsupported.headers()["cache-control"], "no-store");
    assert_eq!(
        body_json(unsupported).await["error"],
        "unsupported_grant_type"
    );
    let missing = app(state())
        .oneshot(form_post(
            "/api/oauth/token",
            &[("grant_type", "refresh_token")],
        ))
        .await
        .unwrap();
    assert_eq!(missing.status(), 400);
    assert_eq!(body_json(missing).await["error"], "invalid_request");
    let wrong_resource = app(state())
        .oneshot(form_post(
            "/api/oauth/token",
            &[
                ("grant_type", "refresh_token"),
                ("refresh_token", "slyt_whatever"),
                ("client_id", "00000000-0000-0000-0000-000000000000"),
                ("resource", "https://elsewhere.example/api/mcp"),
            ],
        ))
        .await
        .unwrap();
    assert_eq!(body_json(wrong_resource).await["error"], "invalid_target");
}

#[tokio::test]
async fn oauth_registration_rejects_unsupported_metadata_with_rfc7591_errors() {
    let mut request = Request::post("/api/oauth/register")
        .header("content-type", "application/json")
        .body(Body::from(
            r#"{"redirect_uris":["https://agent.example/cb"],"grant_types":["client_credentials"]}"#,
        ))
        .unwrap();
    request
        .extensions_mut()
        .insert(axum::extract::ConnectInfo(std::net::SocketAddr::from((
            [203, 0, 113, 7],
            40000,
        ))));
    let response = app(state()).oneshot(request).await.unwrap();
    assert_eq!(response.status(), 400);
    assert_eq!(
        body_json(response).await["error"],
        "invalid_client_metadata"
    );
    let mut request = Request::post("/api/oauth/register")
        .header("content-type", "application/json")
        .body(Body::from(
            r#"{"redirect_uris":["http://evil.example/cb"]}"#,
        ))
        .unwrap();
    request
        .extensions_mut()
        .insert(axum::extract::ConnectInfo(std::net::SocketAddr::from((
            [203, 0, 113, 7],
            40000,
        ))));
    let response = app(state()).oneshot(request).await.unwrap();
    assert_eq!(response.status(), 400);
    assert_eq!(body_json(response).await["error"], "invalid_redirect_uri");
}

/// Session JWTs are checked against the live account, so this needs a real database.
async fn database_state() -> AppState {
    let pool = PgPoolOptions::new()
        .connect(&std::env::var("TEST_DATABASE_URL").expect("TEST_DATABASE_URL required"))
        .await
        .unwrap();
    sqlx::migrate!("../migrations").run(&pool).await.unwrap();
    AppState::new(
        pool,
        "test-secret-at-least-32-characters".into(),
        b"identity-secret".to_vec(),
    )
}

#[tokio::test]
#[ignore = "requires TEST_DATABASE_URL pointing to a disposable PostgreSQL database"]
async fn mcp_negotiates_client_revision_and_accepts_notifications() {
    let state = database_state().await;
    let user: uuid::Uuid =
        sqlx::query_scalar("INSERT INTO users(email,password_hash) VALUES($1,'x') RETURNING id")
            .bind(format!("mcp-{}@example.com", uuid::Uuid::new_v4().simple()))
            .fetch_one(&state.pool)
            .await
            .unwrap();
    let session: uuid::Uuid = sqlx::query_scalar(
        "INSERT INTO user_sessions(user_id,refresh_token_hash,auth_method,expires_at)
         VALUES($1,gen_random_bytes(32),'password',now()+interval '1 day') RETURNING id",
    )
    .bind(user)
    .fetch_one(&state.pool)
    .await
    .unwrap();
    let token = slimlytics_backend::auth::issue_session_token(
        user,
        session,
        "test-secret-at-least-32-characters",
        3600,
    )
    .unwrap();
    let response = app(state.clone()).oneshot(Request::post("/api/mcp").header("authorization",format!("Bearer {token}")).header("content-type","application/json").body(Body::from(r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18"}}"#)).unwrap()).await.unwrap();
    let body: serde_json::Value =
        serde_json::from_slice(&to_bytes(response.into_body(), 10000).await.unwrap()).unwrap();
    assert_eq!(body["result"]["protocolVersion"], "2025-06-18");
    let response = app(state)
        .oneshot(
            Request::post("/api/mcp")
                .header("authorization", format!("Bearer {token}"))
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), 202);
}
