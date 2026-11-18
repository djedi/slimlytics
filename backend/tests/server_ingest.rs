use serde_json::json;
use slimlytics_backend::server_ingest::{
    server_event_classification, validate_server_event, ServerEvent,
};
use std::net::IpAddr;

fn event() -> ServerEvent {
    ServerEvent {
        idempotency_key: Some("edge-request-123".into()),
        url: "https://example.com/article?utm_source=agent".into(),
        user_agent: "GPTBot/1.0".into(),
        client_ip: "203.0.113.20".into(),
        occurred_at: None,
        referrer: None,
        method: Some("GET".into()),
        status: Some(200),
        event_name: None,
        properties: None,
        confirmed: None,
    }
}

fn confirmed_purchase() -> ServerEvent {
    let mut input = event();
    input.event_name = Some("paid".into());
    input.method = None;
    input.status = None;
    input.confirmed = Some(true);
    input
}

#[test]
fn accepts_bounded_server_request_metadata() {
    let validated = validate_server_event(event(), "example.com").unwrap();
    assert_eq!(validated.url.host_str(), Some("example.com"));
    assert_eq!(validated.event_name, "pageview");
    assert_eq!(validated.method.as_deref(), Some("GET"));
}

#[test]
fn rejects_cross_domain_private_or_malformed_metadata() {
    let mut input = event();
    input.url = "https://attacker.example/path".into();
    assert!(validate_server_event(input, "example.com").is_err());

    let mut input = event();
    input.client_ip = "not-an-ip".into();
    assert!(validate_server_event(input, "example.com").is_err());

    let mut input = event();
    input.method = Some("TRACE".into());
    assert!(validate_server_event(input, "example.com").is_err());

    let mut input = event();
    input.status = Some(999);
    assert!(validate_server_event(input, "example.com").is_err());
}

#[test]
fn confirmed_is_refused_on_pageviews() {
    // Log-shipped pageviews keep user-agent bot filtering, so they cannot opt out of it.
    let mut input = event();
    input.confirmed = Some(true);
    assert_eq!(
        validate_server_event(input, "example.com").err(),
        Some("confirmed applies only to custom events")
    );
    let mut input = event();
    input.event_name = Some("pageview".into());
    input.confirmed = Some(true);
    assert!(validate_server_event(input, "example.com").is_err());

    let mut input = event();
    input.confirmed = Some(false);
    assert!(
        !validate_server_event(input, "example.com")
            .unwrap()
            .confirmed
    );
    assert!(
        validate_server_event(confirmed_purchase(), "example.com")
            .unwrap()
            .confirmed
    );
}

#[test]
fn confirmed_custom_events_are_not_classified_as_bots_by_user_agent() {
    let ip: IpAddr = "203.0.113.20".parse().unwrap();
    let mut input = confirmed_purchase();
    input.user_agent = "GPTBot/1.0".into();
    let confirmed = validate_server_event(input, "example.com").unwrap();
    let (class, automation) = server_event_classification(&confirmed, &[]);
    assert_eq!(class, "human");
    assert!(automation.is_none());

    // Without the opt-in, the same event is still a bot.
    let mut input = confirmed_purchase();
    input.user_agent = "GPTBot/1.0".into();
    input.confirmed = None;
    let unconfirmed = validate_server_event(input, "example.com").unwrap();
    let (class, automation) = server_event_classification(&unconfirmed, &[]);
    assert_eq!(class, "bot");
    assert_eq!(automation.unwrap().name, "GPTBot");

    // Operators' own traffic stays internal even when confirmed.
    let (class, _) = server_event_classification(&confirmed, &[ip]);
    assert_eq!(class, "internal");

    // Pageviews keep bot filtering.
    let pageview = validate_server_event(event(), "example.com").unwrap();
    assert_eq!(server_event_classification(&pageview, &[]).0, "bot");
}

#[test]
fn accepts_flat_properties_and_keeps_request_metadata() {
    let mut input = confirmed_purchase();
    input.properties = Some(json!({"plan":"monthly","revenue":9.99,"currency":"usd"}));
    let validated = validate_server_event(input, "example.com").unwrap();
    assert_eq!(
        validated.properties,
        json!({"plan":"monthly","revenue":9.99,"currency":"usd","method":null,"status":null})
    );

    // Request metadata overrides same-named properties only when it was sent.
    let mut input = event();
    input.properties = Some(json!({"status":"trial","method":"card"}));
    input.status = None;
    let validated = validate_server_event(input, "example.com").unwrap();
    assert_eq!(
        validated.properties,
        json!({"status":"trial","method":"GET"})
    );

    // Without properties the stored shape is unchanged.
    let validated = validate_server_event(event(), "example.com").unwrap();
    assert_eq!(validated.properties, json!({"method":"GET","status":200}));
    let mut input = event();
    input.properties = Some(serde_json::Value::Null);
    assert_eq!(
        validate_server_event(input, "example.com")
            .unwrap()
            .properties,
        json!({"method":"GET","status":200})
    );
}

#[test]
fn rejects_properties_collect_would_reject() {
    for invalid in [json!(["plan"]), json!("plan"), json!(5)] {
        let mut input = confirmed_purchase();
        input.properties = Some(invalid);
        assert_eq!(
            validate_server_event(input, "example.com").err(),
            Some("properties must be an object")
        );
    }

    // The 16 KiB serialized limit collect applies: `{"k":"…"}` is 8 bytes plus the value.
    let mut input = confirmed_purchase();
    input.properties = Some(json!({ "k": "x".repeat(16 * 1024 - 8) }));
    assert!(validate_server_event(input, "example.com").is_ok());
    let mut input = confirmed_purchase();
    input.properties = Some(json!({ "k": "x".repeat(16 * 1024 - 7) }));
    assert_eq!(
        validate_server_event(input, "example.com").err(),
        Some("event payload too large")
    );
}
