use chrono::{DateTime, FixedOffset, NaiveDate};
use url::Url;

pub const BING_API_BASE: &str = "https://ssl.bing.com/webmaster/api.svc/json";

/// Bing Webmaster Tools needs no OAuth client: each site owner pastes the API key from Bing's
/// settings, which Slimlytics stores encrypted with the deployment's integration key.
#[derive(Clone)]
pub struct BingConfig {
    pub api_base: String,
    pub encryption_key: [u8; 32],
}

/// URL for a Bing Webmaster JSON API method, with the key and optional site as query parameters.
pub fn api_url(base: &str, method: &str, api_key: &str, site_url: Option<&str>) -> String {
    let mut url =
        Url::parse(&format!("{}/{method}", base.trim_end_matches('/'))).expect("valid URL");
    {
        let mut query = url.query_pairs_mut();
        if let Some(site_url) = site_url {
            query.append_pair("siteUrl", site_url);
        }
        query.append_pair("apikey", api_key);
    }
    url.into()
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
    let split = inner[1..].find(['+', '-']).map(|index| index + 1);
    let (millis, offset) = match split {
        Some(index) => (&inner[..index], Some(&inner[index..])),
        None => (inner, None),
    };
    let millis: i64 = millis.parse().ok()?;
    let offset_seconds = match offset {
        Some(offset) if offset.len() == 5 => {
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
