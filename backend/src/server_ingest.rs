use crate::models::MAX_EVENT_PROPERTIES_BYTES;
use crate::traffic::{automation_for, traffic_class, AutomationMetadata};
use chrono::{DateTime, Utc};
use serde::Deserialize;
use serde_json::Value;
use std::net::IpAddr;
use url::Url;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerEvent {
    pub idempotency_key: Option<String>,
    pub url: String,
    pub user_agent: String,
    pub client_ip: String,
    pub occurred_at: Option<DateTime<Utc>>,
    pub referrer: Option<String>,
    pub method: Option<String>,
    pub status: Option<u16>,
    pub event_name: Option<String>,
    /// Optional flat event properties, validated and stored like `/api/collect` properties.
    pub properties: Option<Value>,
    /// The caller's server confirmed this custom event happened (a signup, a payment), so its
    /// user agent does not mark it as automated. Refused on pageviews.
    pub confirmed: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct ServerEventBatch {
    pub events: Vec<ServerEvent>,
}

pub struct ValidatedServerEvent {
    pub idempotency_key: Option<String>,
    pub url: Url,
    pub user_agent: String,
    pub client_ip: IpAddr,
    pub occurred_at: DateTime<Utc>,
    pub referrer: Option<String>,
    pub method: Option<String>,
    pub status: Option<u16>,
    pub event_name: String,
    /// Caller properties merged with the request `method` and `status`.
    pub properties: Value,
    pub confirmed: bool,
}

/// Event names server ingestion accepts: 1-64 ASCII letters, digits, `_`, `-` or `.`.
pub fn valid_server_event_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 64
        && name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.'))
}

pub fn validate_server_event(
    input: ServerEvent,
    site_domain: &str,
) -> Result<ValidatedServerEvent, &'static str> {
    let url = Url::parse(&input.url).map_err(|_| "invalid url")?;
    if !matches!(url.scheme(), "http" | "https") || url.host_str() != Some(site_domain) {
        return Err("url must use the site's domain");
    }
    if input.user_agent.is_empty() || input.user_agent.len() > 1024 {
        return Err("invalid userAgent");
    }
    let client_ip = input
        .client_ip
        .parse::<IpAddr>()
        .map_err(|_| "invalid clientIp")?;
    let occurred_at = input.occurred_at.unwrap_or_else(Utc::now);
    let age = Utc::now() - occurred_at;
    if age.num_days() > 7 || age.num_minutes() < -5 {
        return Err("occurredAt outside accepted window");
    }
    let method = input.method.map(|value| value.to_ascii_uppercase());
    if method
        .as_deref()
        .is_some_and(|value| !matches!(value, "GET" | "HEAD"))
    {
        return Err("only GET and HEAD requests can be ingested");
    }
    if input
        .status
        .is_some_and(|value| !(100..=599).contains(&value))
    {
        return Err("invalid status");
    }
    let event_name = input.event_name.unwrap_or_else(|| "pageview".into());
    if !valid_server_event_name(&event_name) {
        return Err("invalid eventName");
    }
    let confirmed = input.confirmed.unwrap_or(false);
    if confirmed && event_name == "pageview" {
        return Err("confirmed applies only to custom events");
    }
    let properties = server_event_properties(input.properties, method.as_deref(), input.status)?;
    if input
        .referrer
        .as_ref()
        .is_some_and(|value| value.len() > 2048)
    {
        return Err("invalid referrer");
    }
    if input.idempotency_key.as_ref().is_some_and(|value| {
        !(8..=128).contains(&value.len())
            || !value.bytes().all(|byte| {
                byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':')
            })
    }) {
        return Err("invalid idempotencyKey");
    }
    Ok(ValidatedServerEvent {
        idempotency_key: input.idempotency_key,
        url,
        user_agent: input.user_agent,
        client_ip,
        occurred_at,
        referrer: input.referrer,
        method,
        status: input.status,
        event_name,
        properties,
        confirmed,
    })
}

/// Caller properties, held to the limits `/api/collect` applies, with the request metadata
/// ingestion has always stored. A sent `method` or `status` overrides a same-named property.
fn server_event_properties(
    properties: Option<Value>,
    method: Option<&str>,
    status: Option<u16>,
) -> Result<Value, &'static str> {
    let mut properties = match properties {
        None | Some(Value::Null) => serde_json::Map::new(),
        Some(Value::Object(map)) => map,
        Some(_) => return Err("properties must be an object"),
    };
    // Serialized exactly as collect measures its `properties` value.
    if serde_json::to_string(&properties)
        .map_or(true, |json| json.len() > MAX_EVENT_PROPERTIES_BYTES)
    {
        return Err("event payload too large");
    }
    if method.is_some() || !properties.contains_key("method") {
        properties.insert("method".into(), method.into());
    }
    if status.is_some() || !properties.contains_key("status") {
        properties.insert("status".into(), status.into());
    }
    Ok(Value::Object(properties))
}

/// Traffic class and automation metadata for a server event. A confirmed custom event is a
/// visitor's action vouched for by the site's server, so its user agent or network does not
/// make it a bot; operators' internal addresses still apply.
pub fn server_event_classification(
    event: &ValidatedServerEvent,
    internal: &[IpAddr],
) -> (&'static str, Option<AutomationMetadata>) {
    let class = traffic_class(&event.user_agent, event.client_ip, internal);
    if event.confirmed {
        return (if class == "bot" { "human" } else { class }, None);
    }
    (class, automation_for(&event.user_agent, event.client_ip))
}
