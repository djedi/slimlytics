use axum::{
    body::{to_bytes, Body},
    http::{header, Request, StatusCode},
};
use serde_json::{json, Value};
use slimlytics_backend::{app, AppState};
use sqlx::postgres::PgPoolOptions;
use tower::ServiceExt;

/// Regression: first-touch attribution sorted by unaggregated join columns and returned
/// HTTP 500 ("database unavailable") for any site with traffic, blanking the Insights page.
#[tokio::test]
#[ignore = "requires TEST_DATABASE_URL pointing to a disposable PostgreSQL database"]
async fn attribution_ranks_channels_by_conversions_then_revenue() {
    let url = std::env::var("TEST_DATABASE_URL").expect("TEST_DATABASE_URL required");
    let pool = PgPoolOptions::new().connect(&url).await.unwrap();
    sqlx::migrate!("../migrations").run(&pool).await.unwrap();
    let router = app(AppState::new(
        pool.clone(),
        "01234567890123456789012345678901".into(),
        b"abcdefghijklmnopqrstuvwxyz012345".to_vec(),
    ));

    let email = format!("insights+{}@example.com", uuid::Uuid::new_v4());
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
    assert_eq!(register.status(), StatusCode::CREATED);
    let token = body_json(register.into_body()).await["token"]
        .as_str()
        .unwrap()
        .to_owned();

    let domain = format!("{}.example.com", uuid::Uuid::new_v4());
    let created = router
        .clone()
        .oneshot(json_request(
            "POST",
            "/api/sites",
            Some(&token),
            json!({"name":"Insights","domain":domain,"timezone":"UTC","allowed_origins":[format!("https://{domain}")]}),
        ))
        .await
        .unwrap();
    assert_eq!(created.status(), StatusCode::CREATED);
    let site_id: uuid::Uuid = body_json(created.into_body()).await["id"]
        .as_str()
        .unwrap()
        .parse()
        .unwrap();

    // Three first-touch channels: newsletter converts, search earns revenue, direct does neither.
    let rows = [
        ("v-news", Some("newsletter"), None, "pageview", None),
        ("v-news", Some("newsletter"), None, "signup", None),
        ("v-search", None, Some("google.com"), "pageview", Some(49.0)),
        ("v-direct", None, None, "pageview", None),
    ];
    for (visitor, utm_source, referrer_host, event_name, revenue) in rows {
        sqlx::query(
            "INSERT INTO events(site_id,occurred_at,visitor_id,session_id,url,path,event_name,
               utm_source,referrer_host,revenue_amount,revenue_currency)
             VALUES($1,now() - interval '1 hour',$2,$2,'https://x/','/',$3,$4,$5,$6,
               CASE WHEN $6::numeric IS NULL THEN NULL ELSE 'USD' END)",
        )
        .bind(site_id)
        .bind(visitor)
        .bind(event_name)
        .bind(utm_source)
        .bind(referrer_host)
        .bind(revenue)
        .execute(&pool)
        .await
        .unwrap();
    }
    sqlx::query(
        "WITH goal AS (INSERT INTO goals(site_id,name,event_name) VALUES($1,'Signup','signup') RETURNING id)
         INSERT INTO goal_completions(goal_id,event_id,site_id,visitor_id,occurred_at)
         SELECT goal.id,e.id,e.site_id,e.visitor_id,e.occurred_at
         FROM goal, events e WHERE e.site_id=$1 AND e.event_name='signup'",
    )
    .bind(site_id)
    .execute(&pool)
    .await
    .unwrap();

    let today = chrono::Utc::now().date_naive();
    let response = router
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!(
                    "/api/sites/{site_id}/insights/attribution?from={}&to={}",
                    today - chrono::Duration::days(7),
                    today
                ))
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let channels = body_json(response.into_body()).await;
    let sources: Vec<&str> = channels
        .as_array()
        .unwrap()
        .iter()
        .map(|row| row["source"].as_str().unwrap())
        .collect();
    assert_eq!(sources, ["newsletter", "google.com", "(direct)"]);
    assert_eq!(channels[0]["conversions"], 1);
    assert_eq!(channels[1]["revenue"], 49.0);
    assert_eq!(channels[2]["visitors"], 1);
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
