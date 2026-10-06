use slimlytics_backend::traffic::{
    automation_for, automation_metadata, client_metadata, collection_origin_allowed,
    origin_allowed, traffic_class, RateLimiter,
};
use std::{net::IpAddr, time::Duration};

#[test]
fn origin_must_exactly_match_allowlist() {
    let allowed = vec!["https://example.com".to_string()];
    assert!(origin_allowed(Some("https://example.com"), &allowed));
    assert!(!origin_allowed(Some("https://evil.example"), &allowed));
    assert!(!origin_allowed(None, &allowed));
}

#[test]
fn identifies_ai_crawlers_by_product() {
    let automation = automation_metadata(
        "Mozilla/5.0 AppleWebKit/537.36 (KHTML, like Gecko); compatible; GPTBot/1.1",
    )
    .unwrap();
    assert_eq!(automation.name, "GPTBot");
    assert_eq!(automation.category, "ai-crawler");

    let automation = automation_metadata("ClaudeBot/1.0").unwrap();
    assert_eq!(automation.name, "ClaudeBot");
    assert_eq!(automation.category, "ai-crawler");
}

#[test]
fn collection_allows_missing_origin_when_referer_matches() {
    let allowed = vec!["https://example.com".to_string()];
    assert!(collection_origin_allowed(
        None,
        Some("https://example.com/pricing"),
        &allowed
    ));
    assert!(!collection_origin_allowed(
        None,
        Some("https://evil.example/"),
        &allowed
    ));
    assert!(!collection_origin_allowed(
        Some("https://evil.example"),
        Some("https://example.com/"),
        &allowed
    ));
    assert!(collection_origin_allowed(
        Some("https://example.com"),
        None,
        &allowed
    ));
}

#[test]
fn classifies_bots_and_internal_addresses() {
    let internal: IpAddr = "10.0.0.1".parse().unwrap();
    assert_eq!(
        traffic_class("Googlebot", "1.1.1.1".parse().unwrap(), &[]),
        "bot"
    );
    assert_eq!(traffic_class("Mozilla", internal, &[internal]), "internal");
    assert_eq!(
        traffic_class("Mozilla", "1.1.1.1".parse().unwrap(), &[]),
        "human"
    );
}

#[test]
fn extracts_coarse_client_metadata() {
    let metadata = client_metadata(
        "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) \
         AppleWebKit/537.36 (KHTML, like Gecko) Chrome/126.0.0.0 Safari/537.36",
    );
    assert_eq!(metadata.browser, "Chrome");
    assert_eq!(metadata.browser_version.as_deref(), Some("126.0.0.0"));
    assert_eq!(metadata.os, "Mac OSX");
    assert_eq!(metadata.device_type, "desktop");
}

#[test]
fn rate_limiter_rejects_over_limit() {
    let limiter = RateLimiter::new(2, Duration::from_secs(60));
    assert!(limiter.check("key"));
    assert!(limiter.check("key"));
    assert!(!limiter.check("key"));
}

#[test]
fn referer_fallback_keeps_the_port_when_matching_origins() {
    let allowed = vec!["http://localhost:3000".to_string()];
    assert!(slimlytics_backend::traffic::collection_origin_allowed(
        None,
        Some("http://localhost:3000/page?x=1"),
        &allowed
    ));
    assert!(!slimlytics_backend::traffic::collection_origin_allowed(
        None,
        Some("http://localhost:4000/page"),
        &allowed
    ));
}

const CHROME_UA: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 \
    (KHTML, like Gecko) Chrome/128.0.0.0 Safari/537.36";

#[test]
fn identifies_search_social_and_seo_crawlers_by_product() {
    let cases = [
        ("facebookexternalhit/1.1 (+http://www.facebook.com/externalhit_uatext.php)", "Meta", "crawler"),
        ("meta-externalagent/1.1 (+https://developers.facebook.com/docs/sharing/webmasters/crawler)", "Meta", "ai-crawler"),
        ("AdsBot-Google (+http://www.google.com/adsbot.html)", "Google Ads", "crawler"),
        ("Mozilla/5.0 (compatible; Google-InspectionTool/1.0)", "Googlebot", "crawler"),
        ("Mozilla/5.0 (compatible; Baiduspider/2.0; +http://www.baidu.com/search/spider.html)", "Baiduspider", "crawler"),
        ("Mozilla/5.0 (compatible; YandexBot/3.0)", "YandexBot", "crawler"),
        ("Mozilla/5.0 (compatible; AhrefsBot/7.0)", "AhrefsBot", "seo"),
        ("Mozilla/5.0 (compatible; SemrushBot/7~bl)", "SemrushBot", "seo"),
        ("Mozilla/5.0 (Macintosh) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17 Safari/605.1.15 (Applebot/0.1)", "Applebot", "crawler"),
        ("Mozilla/5.0 (compatible; bingbot/2.0)", "Bingbot", "crawler"),
    ];
    for (ua, name, category) in cases {
        let found = automation_metadata(ua).unwrap_or_else(|| panic!("missed {ua}"));
        assert_eq!((found.name, found.category), (name, category), "{ua}");
    }
}

#[test]
fn flags_crawler_networks_that_send_browser_user_agents() {
    // Google ad-review and preview fetchers often send a stock Chrome user agent.
    for ip in ["66.249.89.103", "66.249.64.40", "66.102.8.169"] {
        let found = automation_for(CHROME_UA, ip.parse().unwrap()).unwrap();
        assert_eq!(found.name, "Google (network)", "{ip}");
        assert_eq!(traffic_class(CHROME_UA, ip.parse().unwrap(), &[]), "bot");
    }
    // Meta ad-review crawlers arriving with a facebook.com referrer.
    let found = automation_for(CHROME_UA, "57.141.0.62".parse().unwrap()).unwrap();
    assert_eq!(found.name, "Meta (network)");
    assert_eq!(found.category, "crawler");
    // IPv6 Googlebot range.
    assert!(automation_for(CHROME_UA, "2001:4860:4801:10::1".parse().unwrap()).is_some());
}

#[test]
fn network_detection_leaves_real_visitors_alone() {
    for ip in [
        "203.0.113.10",
        "198.51.100.7",
        "66.250.1.1",
        "57.142.0.1",
        "8.8.8.8",
    ] {
        assert!(
            automation_for(CHROME_UA, ip.parse().unwrap()).is_none(),
            "{ip}"
        );
        assert_eq!(
            traffic_class(CHROME_UA, ip.parse().unwrap(), &[]),
            "human",
            "{ip}"
        );
    }
}

#[test]
fn product_name_wins_over_network_and_internal_wins_over_all() {
    let ip: IpAddr = "66.249.66.1".parse().unwrap();
    assert_eq!(
        automation_for("Googlebot/2.1", ip).unwrap().name,
        "Googlebot"
    );
    assert_eq!(traffic_class(CHROME_UA, ip, &[ip]), "internal");
}
