//! Porkbun DNS automation: with a user's Porkbun API keys, Slimlytics creates the DNS record
//! that Google Search Console or Bing Webmaster Tools needs to verify a site, then verifies it.
//! Records are only ever created, never edited or deleted.

pub const PORKBUN_API_BASE: &str = "https://api.porkbun.com/api/json/v3";

#[derive(Clone)]
pub struct PorkbunConfig {
    pub api_base: String,
    pub encryption_key: [u8; 32],
}

/// The domains that could be the Porkbun zone holding `host`, longest first. Single labels
/// (bare TLDs) are never candidates.
pub fn zone_candidates(host: &str) -> Vec<String> {
    let host = host.trim_end_matches('.').to_ascii_lowercase();
    let labels: Vec<&str> = host.split('.').collect();
    (0..labels.len().saturating_sub(1))
        .map(|start| labels[start..].join("."))
        .collect()
}

/// `fqdn` relative to `zone`, as Porkbun's create endpoint wants it: `""` for the apex, `None`
/// when the name is not inside the zone.
pub fn relative_name(fqdn: &str, zone: &str) -> Option<String> {
    let fqdn = fqdn.trim_end_matches('.').to_ascii_lowercase();
    let zone = zone.trim_end_matches('.').to_ascii_lowercase();
    if fqdn == zone {
        return Some(String::new());
    }
    fqdn.strip_suffix(&format!(".{zone}"))
        .filter(|name| !name.is_empty())
        .map(str::to_owned)
}

/// Whether `value` is a plain DNS label, so a provider-supplied code can be used as a host name.
pub fn is_dns_label(value: &str) -> bool {
    (1..=63).contains(&value.len())
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_')
        && !value.starts_with('-')
        && !value.ends_with('-')
}

/// The last four characters of an API key, enough to recognise it.
pub fn key_hint(key: &str) -> String {
    let chars: Vec<char> = key.chars().collect();
    chars[chars.len().saturating_sub(4)..].iter().collect()
}
