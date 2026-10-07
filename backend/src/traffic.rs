use dashmap::DashMap;
use std::{
    net::IpAddr,
    time::{Duration, Instant},
};
use woothee::parser::Parser;

pub fn traffic_class(user_agent: &str, ip: IpAddr, internal: &[IpAddr]) -> &'static str {
    if internal.contains(&ip) {
        return "internal";
    }
    if automation_for(user_agent, ip).is_some() {
        return "bot";
    }
    let ua = user_agent.to_ascii_lowercase();
    if ["bot", "spider", "crawler", "headless", "preview"]
        .iter()
        .any(|v| ua.contains(v))
    {
        "bot"
    } else {
        "human"
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AutomationMetadata {
    pub name: &'static str,
    pub category: &'static str,
}

/// Known automated agents, matched case-insensitively against the user agent.
/// Order matters: more specific products come before generic ones.
const AGENTS: &[(&str, &str, &str)] = &[
    ("OAI-SearchBot", "OAI-SearchBot", "ai-crawler"),
    ("ChatGPT-User", "ChatGPT-User", "ai-crawler"),
    ("GPTBot", "GPTBot", "ai-crawler"),
    ("ClaudeBot", "ClaudeBot", "ai-crawler"),
    ("Claude-User", "Claude-User", "ai-crawler"),
    ("PerplexityBot", "PerplexityBot", "ai-crawler"),
    ("Perplexity-User", "Perplexity-User", "ai-crawler"),
    ("Google-Extended", "Google-Extended", "ai-crawler"),
    ("Bytespider", "Bytespider", "ai-crawler"),
    ("meta-externalagent", "Meta", "ai-crawler"),
    ("CCBot", "CCBot", "ai-crawler"),
    ("Amazonbot", "Amazonbot", "ai-crawler"),
    ("AdsBot-Google", "Google Ads", "crawler"),
    ("Mediapartners-Google", "Google Ads", "crawler"),
    ("Google-InspectionTool", "Googlebot", "crawler"),
    ("GoogleOther", "Googlebot", "crawler"),
    ("Googlebot", "Googlebot", "crawler"),
    ("Google-Read-Aloud", "Google Read Aloud", "crawler"),
    ("FeedFetcher-Google", "Google Feedfetcher", "crawler"),
    ("Bingbot", "Bingbot", "crawler"),
    ("BingPreview", "Bingbot", "crawler"),
    ("facebookexternalhit", "Meta", "crawler"),
    ("meta-externalfetcher", "Meta", "crawler"),
    ("facebookcatalog", "Meta", "crawler"),
    ("Applebot", "Applebot", "crawler"),
    ("DuckDuckBot", "DuckDuckBot", "crawler"),
    ("Baiduspider", "Baiduspider", "crawler"),
    ("YandexBot", "YandexBot", "crawler"),
    ("Twitterbot", "Twitterbot", "crawler"),
    ("LinkedInBot", "LinkedInBot", "crawler"),
    ("Slackbot", "Slackbot", "crawler"),
    ("Discordbot", "Discordbot", "crawler"),
    ("AhrefsBot", "AhrefsBot", "seo"),
    ("SemrushBot", "SemrushBot", "seo"),
    ("MJ12bot", "MJ12bot", "seo"),
    ("DotBot", "DotBot", "seo"),
    ("PetalBot", "PetalBot", "seo"),
];

pub fn automation_metadata(user_agent: &str) -> Option<AutomationMetadata> {
    let ua = user_agent.to_ascii_lowercase();
    AGENTS
        .iter()
        .find(|(needle, _, _)| ua.contains(&needle.to_ascii_lowercase()))
        .map(|&(_, name, category)| AutomationMetadata { name, category })
}

/// Published crawler networks whose fetchers frequently send stock browser user agents
/// (ad review, link previews, rendering). Traffic from these ranges is never a site visitor.
const CRAWLER_NETWORKS: &[(&str, &str)] = &[
    ("66.249.64.0/19", "Google (network)"),
    ("66.102.0.0/20", "Google (network)"),
    ("2001:4860:4801::/48", "Google (network)"),
    ("57.141.0.0/24", "Meta (network)"),
    ("69.63.176.0/20", "Meta (network)"),
    ("66.220.144.0/20", "Meta (network)"),
    ("173.252.64.0/18", "Meta (network)"),
    ("2a03:2880::/32", "Meta (network)"),
    ("157.55.39.0/24", "Bing (network)"),
    ("207.46.13.0/24", "Bing (network)"),
    ("40.77.167.0/24", "Bing (network)"),
];

fn in_network(ip: IpAddr, cidr: &str) -> bool {
    let Some((base, bits)) = cidr.split_once('/') else {
        return false;
    };
    let (Ok(base), Ok(bits)) = (base.parse::<IpAddr>(), bits.parse::<u32>()) else {
        return false;
    };
    match (ip, base) {
        (IpAddr::V4(ip), IpAddr::V4(base)) if bits <= 32 => {
            let mask = u32::MAX.checked_shl(32 - bits).unwrap_or(0);
            u32::from(ip) & mask == u32::from(base) & mask
        }
        (IpAddr::V6(ip), IpAddr::V6(base)) if bits <= 128 => {
            let mask = u128::MAX.checked_shl(128 - bits).unwrap_or(0);
            u128::from(ip) & mask == u128::from(base) & mask
        }
        _ => false,
    }
}

/// Identify automation by product name first, then by published crawler network.
pub fn automation_for(user_agent: &str, ip: IpAddr) -> Option<AutomationMetadata> {
    automation_metadata(user_agent).or_else(|| {
        let ip = match ip {
            IpAddr::V6(v6) => v6.to_ipv4_mapped().map(IpAddr::V4).unwrap_or(ip),
            v4 => v4,
        };
        CRAWLER_NETWORKS
            .iter()
            .find(|(cidr, _)| in_network(ip, cidr))
            .map(|&(_, name)| AutomationMetadata {
                name,
                category: "crawler",
            })
    })
}

#[derive(Clone)]
pub struct RateLimiter {
    hits: std::sync::Arc<DashMap<String, (Instant, u32)>>,
    limit: u32,
    window: Duration,
}
impl RateLimiter {
    pub fn new(limit: u32, window: Duration) -> Self {
        Self {
            hits: Default::default(),
            limit,
            window,
        }
    }
    pub fn check(&self, key: &str) -> bool {
        let now = Instant::now();
        if self.hits.len() > 10_000 {
            self.hits
                .retain(|_, (start, _)| now.duration_since(*start) < self.window);
        }
        let mut entry = self.hits.entry(key.to_owned()).or_insert((now, 0));
        if now.duration_since(entry.0) >= self.window {
            *entry = (now, 0);
        }
        entry.1 += 1;
        entry.1 <= self.limit
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClientMetadata {
    pub browser: String,
    pub browser_version: Option<String>,
    pub os: String,
    pub os_version: Option<String>,
    pub device_type: &'static str,
}

pub fn client_metadata(user_agent: &str) -> ClientMetadata {
    let parsed = Parser::new().parse(user_agent);
    let value = |raw: &str| (raw != "UNKNOWN" && !raw.is_empty()).then(|| raw.to_owned());
    ClientMetadata {
        browser: parsed
            .as_ref()
            .and_then(|result| value(result.name))
            .unwrap_or_else(|| "Other".into()),
        browser_version: parsed.as_ref().and_then(|result| value(result.version)),
        os: parsed
            .as_ref()
            .and_then(|result| value(result.os))
            .unwrap_or_else(|| "Other".into()),
        os_version: parsed
            .as_ref()
            .and_then(|result| value(result.os_version.as_ref())),
        device_type: match parsed.as_ref().map(|result| result.category) {
            Some("smartphone" | "mobilephone") => "mobile",
            Some("tablet") => "tablet",
            _ => "desktop",
        },
    }
}

pub fn origin_allowed(origin: Option<&str>, allowed: &[String]) -> bool {
    origin.is_some_and(|value| allowed.iter().any(|allowed| allowed == value))
}

/// Allow collection when Origin matches, or when Origin is absent but Referer is an allowlisted origin.
/// Some mobile browsers omit Origin on same-origin beacon/fetch while still sending Referer.
pub fn collection_origin_allowed(
    origin: Option<&str>,
    referer: Option<&str>,
    allowed: &[String],
) -> bool {
    if origin_allowed(origin, allowed) {
        return true;
    }
    if origin.is_some() {
        return false;
    }
    referer
        .and_then(|value| url::Url::parse(value).ok())
        .and_then(|url| {
            url.host_str()?;
            Some(url.origin().ascii_serialization())
        })
        .is_some_and(|ref_origin| allowed.iter().any(|item| item == &ref_origin))
}
