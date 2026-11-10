use chrono::{DateTime, FixedOffset, NaiveDate};
use url::Url;

pub const BING_API_BASE: &str = "https://ssl.bing.com/webmaster/api.svc/json";

pub const BING_OAUTH_BASE: &str = "https://www.bing.com/webmasters";

/// Bing Webmaster Tools connects either with the site owner's API key or, when this deployment
/// has registered a Bing OAuth client, with a one-click sign-in. Credentials are stored encrypted
/// with the deployment's integration key.
#[derive(Clone)]
pub struct BingConfig {
    pub api_base: String,
    pub encryption_key: [u8; 32],
    pub oauth: Option<BingOAuthConfig>,
}

/// A Bing OAuth client (Bing Webmaster Tools → Settings → API Access → OAuth Client).
#[derive(Clone)]
pub struct BingOAuthConfig {
    pub client_id: String,
    pub client_secret: String,
    /// Slimlytics' `/app` page, which finishes the sign-in with the signed-in user's session.
    pub redirect_uri: String,
    pub oauth_base: String,
}

/// How a Bing API call authenticates: the API key as a query parameter, or an OAuth access token
/// as a Bearer header.
pub enum Credential<'a> {
    ApiKey(&'a str),
    Bearer(&'a str),
}

impl Credential<'_> {
    /// The secret, so error text that might echo it can be redacted.
    pub fn secret(&self) -> &str {
        match self {
            Credential::ApiKey(value) | Credential::Bearer(value) => value,
        }
    }
}

/// Bing's sign-in page; `webmaster.manage` lets Slimlytics add and verify sites as well as read.
pub fn authorization_url(config: &BingOAuthConfig, state: &str) -> String {
    let mut url = Url::parse(&format!(
        "{}/OAuth/authorize",
        config.oauth_base.trim_end_matches('/')
    ))
    .expect("valid URL");
    url.query_pairs_mut()
        .append_pair("response_type", "code")
        .append_pair("client_id", &config.client_id)
        .append_pair("redirect_uri", &config.redirect_uri)
        .append_pair("scope", "webmaster.manage")
        .append_pair("state", state);
    url.into()
}

pub fn token_url(config: &BingOAuthConfig) -> String {
    format!("{}/oauth/token", config.oauth_base.trim_end_matches('/'))
}

/// URL for a Bing Webmaster JSON API method, with the optional site as a query parameter.
pub fn method_url(base: &str, method: &str, site_url: Option<&str>) -> Url {
    let mut url =
        Url::parse(&format!("{}/{method}", base.trim_end_matches('/'))).expect("valid URL");
    if let Some(site_url) = site_url {
        url.query_pairs_mut().append_pair("siteUrl", site_url);
    }
    url
}

/// URL for a Bing Webmaster JSON API method, with the key and optional site as query parameters.
pub fn api_url(base: &str, method: &str, api_key: &str, site_url: Option<&str>) -> String {
    let mut url = method_url(base, method, site_url);
    url.query_pairs_mut().append_pair("apikey", api_key);
    url.into()
}

/// The URL Slimlytics adds to Bing for a domain.
pub fn site_url_for(domain: &str) -> String {
    format!("https://{}/", domain.to_ascii_lowercase())
}

/// Whether two Bing site URLs name the same site (Bing may change case or the trailing slash).
pub fn same_site_url(a: &str, b: &str) -> bool {
    a.trim_end_matches('/')
        .eq_ignore_ascii_case(b.trim_end_matches('/'))
}

/// What the site owner can publish so Bing's `VerifySite` succeeds: a DNS CNAME, a meta tag, or
/// an XML file, built from the codes Bing lists for the site in `GetUserSites`.
pub fn verification_methods(
    domain: &str,
    authentication_code: &str,
    dns_code: &str,
) -> serde_json::Value {
    // Bing lists the CNAME host either bare or already qualified with the domain.
    let host = dns_code.trim_end_matches('.');
    let host = if host
        .to_ascii_lowercase()
        .ends_with(&format!(".{}", domain.to_ascii_lowercase()))
    {
        host.to_owned()
    } else {
        format!("{host}.{domain}")
    };
    serde_json::json!({
        "dnsRecord":{"type":"CNAME","name":host,"value":"verify.bing.com"},
        "metaTag":format!(r#"<meta name="msvalidate.01" content="{authentication_code}" />"#),
        "xmlFile":{
            "path":"/BingSiteAuth.xml",
            "content":format!(r#"<?xml version="1.0"?><users><user>{authentication_code}</user></users>"#)
        }
    })
}

/// Bing's `Message` from an error body, decoded and with the API key (raw or URL-encoded)
/// redacted, since Bing or a proxy may echo the request URL that carries it.
pub fn bing_error_message(body: &str, api_key: &str) -> Option<String> {
    let value: serde_json::Value = serde_json::from_str(body).ok()?;
    let message = value["Message"].as_str()?;
    let encoded: String = url::form_urlencoded::byte_serialize(api_key.as_bytes()).collect();
    Some(
        message
            .replace(api_key, "[redacted]")
            .replace(&encoded, "[redacted]"),
    )
}

/// The verified Bing site that matches a Slimlytics domain, preferring HTTPS over HTTP and the
/// bare domain over `www.`.
pub fn matching_site_url(domain: &str, sites: &[(String, bool)]) -> Option<String> {
    let domain = domain.to_ascii_lowercase();
    let host = domain.strip_prefix("www.").unwrap_or(&domain);
    let candidates = [
        format!("https://{host}/"),
        format!("https://www.{host}/"),
        format!("http://{host}/"),
        format!("http://www.{host}/"),
    ];
    candidates.iter().find_map(|candidate| {
        sites
            .iter()
            .find(|(url, verified)| {
                *verified
                    && url.to_ascii_lowercase().trim_end_matches('/')
                        == candidate.trim_end_matches('/')
            })
            .map(|(url, _)| url.clone())
    })
}

/// Parses Bing's WCF JSON date, e.g. `/Date(1316156400000-0700)/`, into the calendar date in the
/// offset Bing sent, which is the day the bucket starts.
pub fn parse_date(value: &str) -> Option<NaiveDate> {
    let inner = value.strip_prefix("/Date(")?.strip_suffix(")/")?;
    // Skip a leading sign so a negative timestamp is not read as an offset.
    let split = inner
        .char_indices()
        .skip(1)
        .find(|(_, c)| matches!(c, '+' | '-'))
        .map(|(index, _)| index);
    let (millis, offset) = match split {
        Some(index) => (&inner[..index], Some(&inner[index..])),
        None => (inner, None),
    };
    let millis: i64 = millis.parse().ok()?;
    let offset_seconds = match offset {
        Some(offset) if offset.len() == 5 && offset.is_ascii() => {
            let sign = if offset.starts_with('-') { -1 } else { 1 };
            let hours: i32 = offset[1..3].parse().ok()?;
            let minutes: i32 = offset[3..5].parse().ok()?;
            sign * (hours * 3600 + minutes * 60)
        }
        Some(_) => return None,
        None => 0,
    };
    let offset = FixedOffset::east_opt(offset_seconds)?;
    Some(
        DateTime::from_timestamp_millis(millis)?
            .with_timezone(&offset)
            .date_naive(),
    )
}

/// Longest query or page value cached, in bytes. The value is part of the metrics primary key, and
/// PostgreSQL rejects B-tree entries over roughly 2.7 KB, so one huge value would fail a sync.
pub const MAX_VALUE_BYTES: usize = 1024;

/// The value cached for a Bing query or page row, or `None` for a page that is not an http(s)
/// URL (it would be rendered as a link). Page URLs lose their query string and fragment, like
/// every other URL Slimlytics stores, and every value is cut to `MAX_VALUE_BYTES`. Rows that end up
/// equal are merged by `merge_metrics`.
pub fn report_value(kind: &str, raw: &str) -> Option<String> {
    let mut value = if kind == "page" {
        crate::privacy::sanitize_url(raw).ok()?
    } else {
        raw.to_owned()
    };
    if value.len() > MAX_VALUE_BYTES {
        let mut end = MAX_VALUE_BYTES;
        while !value.is_char_boundary(end) {
            end -= 1;
        }
        value.truncate(end);
    }
    Some(value)
}

/// Bing reports a position of -1 or 0 when a row has no ranking; those carry no position.
pub fn position(value: f64) -> Option<f64> {
    (value > 0.0).then_some(value)
}

/// One cached Bing row: a query, page, or daily total in one date bucket.
#[derive(Debug, Clone, PartialEq)]
pub struct Metric {
    pub kind: &'static str,
    pub date: NaiveDate,
    pub value: String,
    pub clicks: f64,
    pub impressions: f64,
    /// Average position over `ranked_impressions`; `None` when no impressions were ranked.
    pub position: Option<f64>,
    pub ranked_impressions: f64,
}

/// Merges rows Bing repeats for one key and bucket: clicks and impressions add up, and the
/// position stays weighted by the impressions of the rows that were ranked.
pub fn merge_metrics(rows: impl IntoIterator<Item = Metric>) -> Vec<Metric> {
    let mut merged: Vec<Metric> = Vec::new();
    let mut index = std::collections::HashMap::new();
    for row in rows {
        let ranked = if row.position.is_some() {
            row.impressions
        } else {
            0.0
        };
        let weighted = row.position.unwrap_or(0.0) * ranked;
        let key = (row.kind, row.date, row.value.clone());
        match index.get(&key) {
            Some(&at) => {
                let existing: &mut Metric = &mut merged[at];
                let total_weighted =
                    existing.position.unwrap_or(0.0) * existing.ranked_impressions + weighted;
                existing.clicks += row.clicks;
                existing.impressions += row.impressions;
                existing.ranked_impressions += ranked;
                existing.position = (existing.ranked_impressions > 0.0)
                    .then(|| total_weighted / existing.ranked_impressions);
            }
            None => {
                index.insert(key, merged.len());
                merged.push(Metric {
                    ranked_impressions: ranked,
                    position: (ranked > 0.0).then_some(weighted / ranked),
                    ..row
                });
            }
        }
    }
    merged
}
