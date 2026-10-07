//! Fetches a site's favicon for its dashboard avatar.
//!
//! Every request goes to a public address only (the same rules as report webhooks): HTTPS on
//! port 443, DNS checked and pinned before connecting, redirects followed by hand and
//! re-checked, and bodies capped while streaming.

use crate::webhooks::is_public_ip;
use regex::Regex;
use reqwest::redirect::Policy;
use std::{net::IpAddr, sync::LazyLock, time::Duration};
use url::Url;

const MAX_HTML_BYTES: usize = 512 * 1024;
pub const MAX_ICON_BYTES: usize = 256 * 1024;
const MAX_REDIRECTS: usize = 3;

pub struct Favicon {
    pub content_type: &'static str,
    pub body: Vec<u8>,
    pub source_url: String,
}

/// `#rrggbb`, lowercased, or `None` for an empty value.
pub fn normalize_color(value: Option<String>) -> Result<Option<String>, &'static str> {
    let Some(value) = value.map(|value| value.trim().to_ascii_lowercase()) else {
        return Ok(None);
    };
    if value.is_empty() {
        return Ok(None);
    }
    let valid = value.len() == 7
        && value.starts_with('#')
        && value[1..].bytes().all(|byte| byte.is_ascii_hexdigit());
    valid.then_some(Some(value)).ok_or("colors must be #rrggbb")
}

/// The image type from the file's leading bytes; anything else is rejected whatever the
/// server's Content-Type says.
pub fn sniff_image(body: &[u8]) -> Option<&'static str> {
    if body.starts_with(b"\x89PNG\r\n\x1a\n") {
        Some("image/png")
    } else if body.starts_with(b"\xff\xd8\xff") {
        Some("image/jpeg")
    } else if body.starts_with(b"GIF87a") || body.starts_with(b"GIF89a") {
        Some("image/gif")
    } else if body.len() > 12 && &body[..4] == b"RIFF" && &body[8..12] == b"WEBP" {
        Some("image/webp")
    } else if body.starts_with(b"\x00\x00\x01\x00") {
        Some("image/x-icon")
    } else {
        let head = String::from_utf8_lossy(&body[..body.len().min(1024)]).to_ascii_lowercase();
        let head = head.trim_start_matches('\u{feff}').trim_start();
        (head.starts_with("<svg") || (head.starts_with("<?xml") && head.contains("<svg")))
            .then_some("image/svg+xml")
    }
}

static LINK_TAG: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?is)<link\b[^>]*>").unwrap());
static ATTRIBUTE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?is)([a-z-]+)\s*=\s*(?:"([^"]*)"|'([^']*)'|([^\s"'>]+))"#).unwrap()
});

/// Icon URLs declared in the page, best first: apple-touch-icon (usually 180px), then the
/// largest sized icon, then SVG, then the rest. `/favicon.ico` is always the last resort.
pub fn icon_candidates(html: &str, page: &Url) -> Vec<Url> {
    let mut found: Vec<(u32, Url)> = Vec::new();
    for tag in LINK_TAG.find_iter(html) {
        let mut rel = String::new();
        let mut href = None;
        let mut sizes = String::new();
        let mut kind = String::new();
        for capture in ATTRIBUTE.captures_iter(tag.as_str()) {
            let value = capture
                .get(2)
                .or_else(|| capture.get(3))
                .or_else(|| capture.get(4))
                .map_or("", |value| value.as_str());
            match capture[1].to_ascii_lowercase().as_str() {
                "rel" => rel = value.to_ascii_lowercase(),
                "href" => href = Some(value.trim().to_owned()),
                "sizes" => sizes = value.to_ascii_lowercase(),
                "type" => kind = value.to_ascii_lowercase(),
                _ => {}
            }
        }
        let rels: Vec<&str> = rel.split_whitespace().collect();
        let Some(url) = href.and_then(|href| page.join(&href).ok()) else {
            continue;
        };
        let score = if rels.iter().any(|rel| rel.starts_with("apple-touch-icon")) {
            10_000
        } else if rels.contains(&"icon") {
            let size = sizes
                .split_whitespace()
                .filter_map(|size| size.split('x').next()?.parse::<u32>().ok())
                .max();
            match size {
                Some(size) => 1_000 + size.min(4_096),
                None if kind.contains("svg") || url.path().ends_with(".svg") => 900,
                None => 500,
            }
        } else {
            continue;
        };
        found.push((score, url));
    }
    found.sort_by_key(|entry| std::cmp::Reverse(entry.0));
    let mut urls: Vec<Url> = Vec::new();
    for (_, url) in found {
        if !urls.contains(&url) {
            urls.push(url);
        }
    }
    // The fallback always goes last, even if the page declares it, so attempt_order can rely
    // on it being the final candidate.
    if let Ok(fallback) = page.join("/favicon.ico") {
        urls.retain(|url| *url != fallback);
        urls.push(fallback);
    }
    urls
}

/// At most four declared icons, then always the `/favicon.ico` fallback (the last candidate).
fn attempt_order(candidates: Vec<Url>) -> Vec<Url> {
    let fallback = candidates.last().cloned();
    let mut urls: Vec<Url> = Vec::new();
    for url in candidates.into_iter().take(4).chain(fallback) {
        if !urls.contains(&url) {
            urls.push(url);
        }
    }
    urls
}

/// Finds and downloads the favicon of `https://{domain}/`.
pub async fn fetch_favicon(domain: &str) -> Result<Favicon, String> {
    let home =
        Url::parse(&format!("https://{domain}/")).map_err(|_| "invalid domain".to_owned())?;
    // Icon links live in <head>, so a long page is cut short rather than rejected.
    let candidates = match get(home.clone(), MAX_HTML_BYTES, true).await {
        Ok((page, body)) => icon_candidates(&String::from_utf8_lossy(&body), &page),
        Err(_) => icon_candidates("", &home),
    };
    let attempts = attempt_order(candidates);
    let mut last_error = "no icon found".to_owned();
    for url in attempts {
        match get(url, MAX_ICON_BYTES, false).await {
            Ok((url, body)) => match sniff_image(&body) {
                Some(content_type) => {
                    return Ok(Favicon {
                        content_type,
                        body,
                        source_url: url.to_string(),
                    })
                }
                None => last_error = format!("{url} is not an image"),
            },
            Err(error) => last_error = error,
        }
    }
    Err(last_error)
}

/// GET a public HTTPS URL, following up to three redirects and re-checking each hop. A body
/// over `limit` is an error, or with `truncate` is cut off at `limit`.
async fn get(mut url: Url, limit: usize, truncate: bool) -> Result<(Url, Vec<u8>), String> {
    for _ in 0..=MAX_REDIRECTS {
        let client = public_client(&url).await?;
        let mut response = client
            .get(url.clone())
            .header("accept", "text/html,image/*;q=0.9,*/*;q=0.5")
            .send()
            .await
            .map_err(|error| format!("{url}: {error}"))?;
        if response.status().is_redirection() {
            let location = response
                .headers()
                .get("location")
                .and_then(|value| value.to_str().ok())
                .ok_or_else(|| format!("{url}: redirect without a location"))?;
            url = url
                .join(location)
                .map_err(|_| format!("{url}: invalid redirect"))?;
            continue;
        }
        if !response.status().is_success() {
            return Err(format!("{url} returned HTTP {}", response.status()));
        }
        if !truncate
            && response
                .content_length()
                .is_some_and(|length| length as usize > limit)
        {
            return Err(format!("{url} is too large"));
        }
        let mut body = Vec::new();
        while let Some(chunk) = response
            .chunk()
            .await
            .map_err(|error| format!("{url}: {error}"))?
        {
            if body.len() + chunk.len() > limit {
                if truncate {
                    body.extend_from_slice(&chunk[..limit - body.len()]);
                    break;
                }
                return Err(format!("{url} is too large"));
            }
            body.extend_from_slice(&chunk);
        }
        return Ok((url, body));
    }
    Err("too many redirects".into())
}

async fn public_client(url: &Url) -> Result<reqwest::Client, String> {
    let host = url
        .host_str()
        .filter(|_| url.scheme() == "https" && url.port_or_known_default() == Some(443))
        .filter(|_| url.username().is_empty() && url.password().is_none())
        .ok_or_else(|| format!("{url}: only HTTPS on port 443 is allowed"))?;
    if host.eq_ignore_ascii_case("localhost")
        || host.ends_with(".localhost")
        || host.parse::<IpAddr>().is_ok_and(|ip| !is_public_ip(ip))
    {
        return Err(format!("{host} is not a public host"));
    }
    let addresses: Vec<_> = tokio::net::lookup_host((host, 443))
        .await
        .map_err(|error| format!("{host}: DNS lookup failed: {error}"))?
        .collect();
    if addresses.is_empty() || addresses.iter().any(|address| !is_public_ip(address.ip())) {
        return Err(format!("{host} resolves to a non-public address"));
    }
    reqwest::Client::builder()
        .redirect(Policy::none())
        // An environment proxy would resolve the host itself and bypass the pinned address.
        .no_proxy()
        .timeout(Duration::from_secs(8))
        .user_agent("Slimlytics favicon fetcher (+https://slimlytics.com)")
        .resolve(host, addresses[0])
        .build()
        .map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn colors_must_be_six_digit_hex() {
        assert_eq!(
            normalize_color(Some(" #A1B2C3 ".into())),
            Ok(Some("#a1b2c3".into()))
        );
        assert_eq!(normalize_color(Some(String::new())), Ok(None));
        assert_eq!(normalize_color(None), Ok(None));
        assert!(normalize_color(Some("red".into())).is_err());
        assert!(normalize_color(Some("#abc".into())).is_err());
        assert!(normalize_color(Some("#12345g".into())).is_err());
    }

    #[test]
    fn sniffs_only_real_images() {
        assert_eq!(sniff_image(b"\x89PNG\r\n\x1a\nrest"), Some("image/png"));
        assert_eq!(sniff_image(b"\x00\x00\x01\x00\x01"), Some("image/x-icon"));
        assert_eq!(
            sniff_image(b"<?xml version=\"1.0\"?>\n<svg xmlns=\"http://www.w3.org/2000/svg\"/>"),
            Some("image/svg+xml")
        );
        assert_eq!(sniff_image(b"<!doctype html><html>"), None);
    }

    #[test]
    fn prefers_touch_icons_then_largest_then_favicon_ico() {
        let page = Url::parse("https://example.com/blog/").unwrap();
        let html = r#"
          <link rel="stylesheet" href="/app.css">
          <link rel="icon" href="/small.png" sizes="16x16">
          <link href='/big.png' rel='icon' sizes='192x192'>
          <LINK REL="shortcut icon" HREF="favicon-local.ico">
          <link rel="apple-touch-icon" href="https://cdn.example.com/touch.png">
        "#;
        let urls: Vec<String> = icon_candidates(html, &page)
            .into_iter()
            .map(String::from)
            .collect();
        assert_eq!(
            urls,
            [
                "https://cdn.example.com/touch.png",
                "https://example.com/big.png",
                "https://example.com/small.png",
                "https://example.com/blog/favicon-local.ico",
                "https://example.com/favicon.ico",
            ]
        );
    }

    #[test]
    fn always_tries_favicon_ico_after_declared_icons() {
        let page = Url::parse("https://example.com/").unwrap();
        let html: String = (1..=6)
            .map(|size| format!(r#"<link rel="icon" sizes="{size}x{size}" href="/{size}.png">"#))
            .collect();
        let urls: Vec<String> = attempt_order(icon_candidates(&html, &page))
            .into_iter()
            .map(String::from)
            .collect();
        assert_eq!(urls.len(), 5);
        assert_eq!(urls[0], "https://example.com/6.png");
        assert_eq!(urls[4], "https://example.com/favicon.ico");
    }

    #[test]
    fn declared_favicon_ico_still_comes_last() {
        let page = Url::parse("https://example.com/").unwrap();
        let mut html: String = (2..=5)
            .map(|size| format!(r#"<link rel="icon" sizes="{size}x{size}" href="/{size}.png">"#))
            .collect();
        html.push_str(
            r#"<link rel="icon" sizes="1x1" href="/favicon.ico"><link rel="icon" href="/x.png">"#,
        );
        let urls: Vec<String> = attempt_order(icon_candidates(&html, &page))
            .into_iter()
            .map(String::from)
            .collect();
        assert_eq!(urls.last().unwrap(), "https://example.com/favicon.ico");
        assert_eq!(urls.len(), 5);
    }

    #[tokio::test]
    async fn refuses_private_and_non_https_hosts() {
        for url in [
            "https://127.0.0.1/favicon.ico",
            "https://localhost/",
            "http://example.com/",
            "https://example.com:8443/",
            "https://[::1]/",
        ] {
            assert!(
                public_client(&Url::parse(url).unwrap()).await.is_err(),
                "{url}"
            );
        }
    }
}
