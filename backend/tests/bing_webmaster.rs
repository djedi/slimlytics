use chrono::NaiveDate;
use slimlytics_backend::bing_webmaster::{api_url, matching_site_url, parse_date, position};

#[test]
fn builds_api_urls_with_site_and_key() {
    assert_eq!(
        api_url(
            "https://ssl.bing.com/webmaster/api.svc/json/",
            "GetQueryStats",
            "k&y",
            Some("https://example.com/")
        ),
        "https://ssl.bing.com/webmaster/api.svc/json/GetQueryStats?siteUrl=https%3A%2F%2Fexample.com%2F&apikey=k%26y"
    );
    assert_eq!(
        api_url("https://bing.test/json", "GetUserSites", "key", None),
        "https://bing.test/json/GetUserSites?apikey=key"
    );
}

#[test]
fn matches_verified_sites_preferring_https_and_bare_domain() {
    let sites = vec![
        ("http://example.com/".to_string(), true),
        ("https://www.example.com/".to_string(), true),
        ("https://example.com".to_string(), false),
        ("https://other.example/".to_string(), true),
    ];
    assert_eq!(
        matching_site_url("Example.com", &sites).as_deref(),
        Some("https://www.example.com/")
    );
    assert_eq!(
        matching_site_url("www.other.example", &sites).as_deref(),
        Some("https://other.example/")
    );
    assert_eq!(matching_site_url("missing.example", &sites), None);
}

#[test]
fn parses_wcf_dates_in_their_offset() {
    assert_eq!(
        parse_date("/Date(1316156400000-0700)/"),
        NaiveDate::from_ymd_opt(2011, 9, 16)
    );
    assert_eq!(
        parse_date("/Date(1316131200000)/"),
        NaiveDate::from_ymd_opt(2011, 9, 16)
    );
    assert_eq!(
        parse_date("/Date(1316131200000+0900)/"),
        NaiveDate::from_ymd_opt(2011, 9, 16)
    );
    assert_eq!(parse_date("2011-09-16"), None);
    assert_eq!(parse_date("/Date(abc)/"), None);
}

#[test]
fn treats_non_positive_positions_as_unranked() {
    assert_eq!(position(-1.0), None);
    assert_eq!(position(0.0), None);
    assert_eq!(position(3.5), Some(3.5));
}

mod flow {
    use axum::{
        body::{to_bytes, Body},
        extract::Query,
        http::{Request, StatusCode},
        routing::get,
        Json, Router,
    };
    use serde_json::{json, Value};
    use slimlytics_backend::{app, bing_webmaster::BingConfig, AppState};
    use sqlx::postgres::PgPoolOptions;
    use std::collections::HashMap;
    use tower::ServiceExt;

    /// Answers like Bing's JSON API for one key and one verified site.
    async fn fake_bing() -> String {
        type Params = Query<HashMap<String, String>>;
        fn checked(
            params: &HashMap<String, String>,
            site: bool,
        ) -> Result<(), (StatusCode, Json<Value>)> {
            let key_ok = params.get("apikey").map(String::as_str) == Some("good-key");
            let site_ok = !site
                || params.get("siteUrl").map(String::as_str) == Some("https://bing-flow.example/");
            if key_ok && site_ok {
                Ok(())
            } else {
                Err((
                    StatusCode::BAD_REQUEST,
                    Json(json!({"ErrorCode":3,"Message":"InvalidApiKey"})),
                ))
            }
        }
        let router = Router::new()
            .route("/json/GetUserSites", get(|Query(p): Params| async move {
                checked(&p, false)?;
                Ok::<_, (StatusCode, Json<Value>)>(Json(json!({"d":[
                    {"Url":"https://bing-flow.example/","IsVerified":true},
                    {"Url":"https://other.example/","IsVerified":true}
                ]})))
            }))
            .route("/json/GetQueryStats", get(|Query(p): Params| async move {
                checked(&p, true)?;
                Ok::<_, (StatusCode, Json<Value>)>(Json(json!({"d":[
                    {"Query":"privacy analytics","Date":"/Date(1760857200000-0700)/","Clicks":3,"Impressions":30,"AvgImpressionPosition":2.0,"AvgClickPosition":2.0},
                    {"Query":"privacy analytics","Date":"/Date(1761462000000-0700)/","Clicks":1,"Impressions":10,"AvgImpressionPosition":6.0,"AvgClickPosition":6.0},
                    {"Query":"slimlytics","Date":"/Date(1761462000000-0700)/","Clicks":2,"Impressions":8,"AvgImpressionPosition":-1,"AvgClickPosition":-1}
                ]})))
            }))
            .route("/json/GetPageStats", get(|Query(p): Params| async move {
                checked(&p, true)?;
                Ok::<_, (StatusCode, Json<Value>)>(Json(json!({"d":[
                    {"Query":"https://bing-flow.example/","Date":"/Date(1761462000000-0700)/","Clicks":9,"Impressions":40,"AvgImpressionPosition":3.0,"AvgClickPosition":3.0}
                ]})))
            }))
            .route("/json/GetRankAndTrafficStats", get(|Query(p): Params| async move {
                checked(&p, true)?;
                Ok::<_, (StatusCode, Json<Value>)>(Json(json!({"d":[
                    {"Date":"/Date(1761462000000-0700)/","Clicks":6,"Impressions":20},
                    {"Date":"/Date(1761548400000-0700)/","Clicks":4,"Impressions":30}
                ]})))
            }));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
        format!("http://{address}/json")
    }

    fn request(method: &str, uri: &str, token: &str, body: Value) -> Request<Body> {
        Request::builder()
            .method(method)
            .uri(uri)
            .header("authorization", format!("Bearer {token}"))
            .header("content-type", "application/json")
            .body(Body::from(body.to_string()))
            .unwrap()
    }

    async fn send(router: &Router, request: Request<Body>) -> (StatusCode, Value) {
        let response = router.clone().oneshot(request).await.unwrap();
        let status = response.status();
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        (status, serde_json::from_slice(&body).unwrap_or(Value::Null))
    }

    #[tokio::test]
    #[ignore = "requires TEST_DATABASE_URL pointing to a disposable PostgreSQL database"]
    async fn connects_syncs_reports_and_disconnects() {
        let url = std::env::var("TEST_DATABASE_URL").expect("TEST_DATABASE_URL required");
        let pool = PgPoolOptions::new().connect(&url).await.unwrap();
        sqlx::migrate!("../migrations").run(&pool).await.unwrap();
        let router = app(AppState::new(
            pool.clone(),
            "01234567890123456789012345678901".into(),
            b"abcdefghijklmnopqrstuvwxyz012345".to_vec(),
        )
        .with_bing(BingConfig {
            api_base: fake_bing().await,
            encryption_key: [9_u8; 32],
        }));
        sqlx::query("DELETE FROM sites WHERE domain='bing-flow.example'")
            .execute(&pool)
            .await
            .unwrap();
        let email = format!("bing+{}@example.com", uuid::Uuid::new_v4());
        let (_, registered) = send(
            &router,
            request(
                "POST",
                "/api/auth/register",
                "",
                json!({"email":email,"password":"long-enough-password"}),
            ),
        )
        .await;
        let token = registered["token"].as_str().unwrap().to_owned();
        let (status, site) = send(
            &router,
            request(
                "POST",
                "/api/sites",
                &token,
                json!({"name":"Bing","domain":"bing-flow.example","timezone":"UTC"}),
            ),
        )
        .await;
        assert_eq!(status, StatusCode::CREATED);
        let base = format!("/api/sites/{}", site["id"].as_str().unwrap());

        let (_, status_before) = send(
            &router,
            request(
                "GET",
                &format!("{base}/integrations/bing"),
                &token,
                json!({}),
            ),
        )
        .await;
        assert_eq!(status_before, json!({"configured":true,"connected":false}));

        let (status, rejected) = send(
            &router,
            request(
                "POST",
                &format!("{base}/integrations/bing"),
                &token,
                json!({"apiKey":"bad-key"}),
            ),
        )
        .await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{rejected}");
        assert!(rejected.to_string().contains("InvalidApiKey"), "{rejected}");

        let (status, connected) = send(
            &router,
            request(
                "POST",
                &format!("{base}/integrations/bing"),
                &token,
                json!({"apiKey":" good-key "}),
            ),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{connected}");
        assert_eq!(connected["siteUrl"], "https://bing-flow.example/");
        let stored: String = sqlx::query_scalar(
            "SELECT api_key_encrypted FROM bing_webmaster_integrations i JOIN sites s ON s.id=i.site_id WHERE s.domain='bing-flow.example'",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert!(!stored.contains("good-key"));

        let (status, synced) = send(
            &router,
            request(
                "POST",
                &format!("{base}/integrations/bing/sync"),
                &token,
                json!({}),
            ),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{synced}");
        assert_eq!(synced["rows"], 6);

        let report = |dimension: &str| {
            request(
                "GET",
                &format!("{base}/reports/bing?from=2025-10-01&to=2025-10-31&dimension={dimension}"),
                &token,
                json!({}),
            )
        };
        let (_, queries) = send(&router, report("query")).await;
        assert_eq!(queries[0]["value"], "privacy analytics");
        assert_eq!(queries[0]["clicks"], 4.0);
        assert_eq!(queries[0]["impressions"], 40.0);
        assert_eq!(queries[0]["position"], 3.0);
        assert_eq!(queries[1]["value"], "slimlytics");
        assert_eq!(queries[1]["position"], 0.0);
        let (_, pages) = send(&router, report("page")).await;
        assert_eq!(pages.as_array().unwrap().len(), 1);
        let (_, dates) = send(&router, report("date")).await;
        assert_eq!(
            dates
                .as_array()
                .unwrap()
                .iter()
                .map(|row| row["value"].clone())
                .collect::<Vec<_>>(),
            vec![json!("2025-10-26"), json!("2025-10-27")]
        );
        let (status, _) = send(&router, report("country")).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);

        let (status, _) = send(
            &router,
            request(
                "DELETE",
                &format!("{base}/integrations/bing"),
                &token,
                json!({}),
            ),
        )
        .await;
        assert_eq!(status, StatusCode::NO_CONTENT);
        let left: i64 = sqlx::query_scalar(
            "SELECT (SELECT count(*) FROM bing_webmaster_integrations i JOIN sites s ON s.id=i.site_id WHERE s.domain='bing-flow.example')
                  + (SELECT count(*) FROM bing_webmaster_metrics m JOIN sites s ON s.id=m.site_id WHERE s.domain='bing-flow.example')",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(left, 0);
    }
}
