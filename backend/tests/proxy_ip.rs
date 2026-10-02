use axum::{
    body::{to_bytes, Body},
    extract::ConnectInfo,
    http::{header, Request, StatusCode},
};
use serde_json::{json, Value};
use slimlytics_backend::{app, AppState};
use sqlx::postgres::PgPoolOptions;
use tower::ServiceExt;

/// Regression: a site's first-party proxy connects from the website's own server, so every
/// visitor used to share that server's IP. Locations came back unknown (or as the server's
/// datacenter) and different visitors with the same browser merged into one. The proxy now
/// forwards the visitor IP with the site's proxy key, and only a matching key is trusted.
#[tokio::test]
#[ignore = "requires TEST_DATABASE_URL pointing to a disposable PostgreSQL database"]
async fn proxy_forwarded_visitor_ips_count_as_separate_visitors_only_with_the_key() {
    let url = std::env::var("TEST_DATABASE_URL").expect("TEST_DATABASE_URL required");
    let pool = PgPoolOptions::new().connect(&url).await.unwrap();
    sqlx::migrate!("../migrations").run(&pool).await.unwrap();
    let router = app(AppState::new(
        pool.clone(),
        "01234567890123456789012345678901".into(),
        b"abcdefghijklmnopqrstuvwxyz012345".to_vec(),
    ));

    let email = format!("proxy+{}@example.com", uuid::Uuid::new_v4());
    let register = router
        .clone()
        .oneshot(json_request(
            "POST",
            "/api/auth/register",
            None,
            json!({"email":email,"password":"long-enough-password"}),
        ))
        .await
        .unwrap();
    let token = body_json(register.into_body()).await["token"]
        .as_str()
        .unwrap()
        .to_owned();
    let domain = format!("{}.example.com", uuid::Uuid::new_v4());
    let origin = format!("https://{domain}");
    let created = router
        .clone()
        .oneshot(json_request(
            "POST",
            "/api/sites",
            Some(&token),
            json!({"name":"Proxied","domain":domain,"timezone":"UTC","allowed_origins":[origin]}),
        ))
        .await
        .unwrap();
    assert_eq!(created.status(), StatusCode::CREATED);
    let site = body_json(created.into_body()).await;
    let site_id: uuid::Uuid = site["id"].as_str().unwrap().parse().unwrap();
    let write_key = site["writeKey"].as_str().unwrap().to_owned();
    let proxy_key = site["proxyKey"]
        .as_str()
        .expect("site responses include proxyKey")
        .to_owned();

    // Two different visitors with identical browsers, both arriving from the website server.
    let website_server = "172.82.74.120:443";
    for (path, client_ip, key) in [
        ("/keyed", "81.2.69.160", Some(proxy_key.as_str())),
        ("/keyed", "8.8.8.8", Some(proxy_key.as_str())),
        ("/unkeyed", "81.2.69.160", None),
        (
            "/unkeyed",
            "8.8.8.8",
            Some("00000000-0000-0000-0000-000000000000"),
        ),
    ] {
        let mut request = json_request(
            "POST",
            &format!("/api/e/{write_key}"),
            None,
            json!({"name":"pageview","url":format!("{origin}{path}")}),
        );
        let headers = request.headers_mut();
        headers.insert(header::ORIGIN, origin.parse().unwrap());
        headers.insert(
            header::USER_AGENT,
            "Mozilla/5.0 (Macintosh) Chrome/129".parse().unwrap(),
        );
        headers.insert("x-slimlytics-client-ip", client_ip.parse().unwrap());
        if let Some(key) = key {
            headers.insert("x-slimlytics-proxy-key", key.parse().unwrap());
        }
        request.extensions_mut().insert(ConnectInfo(
            website_server.parse::<std::net::SocketAddr>().unwrap(),
        ));
        let response = router.clone().oneshot(request).await.unwrap();
        assert_eq!(
            response.status(),
            StatusCode::ACCEPTED,
            "{path} {client_ip}"
        );
    }

    let distinct = |path: &'static str| {
        let pool = pool.clone();
        async move {
            sqlx::query_scalar::<_, i64>(
                "SELECT count(DISTINCT visitor_id) FROM events WHERE site_id=$1 AND path=$2",
            )
            .bind(site_id)
            .bind(path)
            .fetch_one(&pool)
            .await
            .unwrap()
        }
    };
    assert_eq!(
        distinct("/keyed").await,
        2,
        "vouched visitor IPs stay distinct"
    );
    assert_eq!(
        distinct("/unkeyed").await,
        1,
        "missing or wrong key falls back to the connecting server's IP"
    );

    // A leaked key can be replaced: the old one stops vouching immediately.
    let rotated = router
        .clone()
        .oneshot(json_request(
            "POST",
            &format!("/api/sites/{site_id}/rotate-proxy-key"),
            Some(&token),
            json!({}),
        ))
        .await
        .unwrap();
    assert_eq!(rotated.status(), StatusCode::OK);
    let new_key = body_json(rotated.into_body()).await["proxyKey"]
        .as_str()
        .unwrap()
        .to_owned();
    assert_ne!(new_key, proxy_key);
    for (path, key) in [
        ("/old-key", proxy_key.as_str()),
        ("/new-key", new_key.as_str()),
    ] {
        for client_ip in ["81.2.69.160", "8.8.8.8"] {
            let mut request = json_request(
                "POST",
                &format!("/api/e/{write_key}"),
                None,
                json!({"name":"pageview","url":format!("{origin}{path}")}),
            );
            let headers = request.headers_mut();
            headers.insert(header::ORIGIN, origin.parse().unwrap());
            headers.insert(
                header::USER_AGENT,
                "Mozilla/5.0 (Macintosh) Chrome/129".parse().unwrap(),
            );
            headers.insert("x-slimlytics-client-ip", client_ip.parse().unwrap());
            headers.insert("x-slimlytics-proxy-key", key.parse().unwrap());
            request.extensions_mut().insert(ConnectInfo(
                website_server.parse::<std::net::SocketAddr>().unwrap(),
            ));
            assert_eq!(
                router.clone().oneshot(request).await.unwrap().status(),
                StatusCode::ACCEPTED
            );
        }
    }
    assert_eq!(
        distinct("/old-key").await,
        1,
        "rotated-out key is no longer trusted"
    );
    assert_eq!(
        distinct("/new-key").await,
        2,
        "new key vouches for visitor IPs"
    );
}

fn json_request(method: &str, uri: &str, token: Option<&str>, body: Value) -> Request<Body> {
    let mut builder = Request::builder()
        .method(method)
        .uri(uri)
        .header(header::CONTENT_TYPE, "application/json");
    if let Some(token) = token {
        builder = builder.header(header::AUTHORIZATION, format!("Bearer {token}"));
    }
    builder.body(Body::from(body.to_string())).unwrap()
}

async fn body_json(body: Body) -> Value {
    serde_json::from_slice(&to_bytes(body, usize::MAX).await.unwrap()).unwrap()
}
