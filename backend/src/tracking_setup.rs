use crate::models::Site;
use anyhow::{bail, Result};
use serde::Serialize;
use url::Url;
use uuid::Uuid;
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TrackingSetup {
    pub site_id: Uuid,
    pub domain: String,
    pub server_type: String,
    pub javascript_path: String,
    pub beacon_path: String,
    pub server_config: String,
    pub snippet: String,
    pub script_test_url: String,
    pub beacon_test_url: String,
    pub server_ingest_url: String,
    /// Sent as X-Slimlytics-Proxy-Key with X-Slimlytics-Client-IP by the collection route so
    /// visitor locations and IDs use the real visitor IP. A server-side secret: anyone holding it
    /// can choose the IP Slimlytics records, so keep it out of public repositories.
    pub proxy_key: Uuid,
    pub next_steps: Vec<String>,
}

pub fn tracking_setup(site: &Site, analytics_origin: &str) -> Result<TrackingSetup> {
    if !valid_proxy_path(&site.anti_adblock_js_path, true)
        || !valid_proxy_path(&site.anti_adblock_beacon_path, false)
        || site.anti_adblock_js_path == site.anti_adblock_beacon_path
    {
        bail!("Slimlytics API returned unsafe first-party tracking paths");
    }
    let analytics = analytics_origin.to_owned();
    let domain = site.domain.clone();
    let website = format!("https://{domain}");
    let bootstrap_path = format!(
        "/p/{}/{}",
        site.write_key,
        site.anti_adblock_beacon_path.trim_start_matches('/')
    );
    let collect_path = format!("/api/collect/{}", site.write_key);
    let bootstrap = format!("{analytics}{bootstrap_path}");
    let collect = format!("{analytics}{collect_path}");
    let server_config = match site.anti_adblock_server.as_str() {
        "caddy" => format!(
            "# Slimlytics first-party tracking\nhandle {} {{\n\trewrite {} {}\n\treverse_proxy {} {{\n\t\theader_up Host {{upstream_hostport}}\n\t\theader_up -Cookie\n\t\theader_up -Authorization\n\t\theader_down -Set-Cookie\n\t}}\n}}\n\nhandle {} {{\n\trewrite {} {}\n\treverse_proxy {} {{\n\t\theader_up Host {{upstream_hostport}}\n\t\theader_up -Cookie\n\t\theader_up -Authorization\n\t\theader_up X-Slimlytics-Client-IP {{client_ip}}\n\t\theader_up X-Slimlytics-Proxy-Key {}\n\t\theader_down -Set-Cookie\n\t}}\n}}",
            site.anti_adblock_js_path,
            site.anti_adblock_js_path,
            bootstrap_path,
            analytics,
            site.anti_adblock_beacon_path,
            site.anti_adblock_beacon_path,
            collect_path,
            analytics,
            site.proxy_key
        ),
        "nginx" => format!(
            "# Slimlytics first-party tracking\nlocation = {} {{\n    proxy_pass {};\n    proxy_set_header Host {};\n    proxy_set_header Cookie \"\";\n    proxy_set_header Authorization \"\";\n    proxy_set_header X-Forwarded-For $remote_addr;\n    proxy_hide_header Set-Cookie;\n    proxy_ssl_server_name on;\n    proxy_ssl_name {};\n}}\n\nlocation = {} {{\n    proxy_pass {};\n    proxy_set_header Host {};\n    proxy_set_header Cookie \"\";\n    proxy_set_header Authorization \"\";\n    proxy_set_header X-Forwarded-For $remote_addr;\n    proxy_set_header X-Slimlytics-Client-IP $remote_addr;\n    proxy_set_header X-Slimlytics-Proxy-Key {};\n    proxy_hide_header Set-Cookie;\n    proxy_ssl_server_name on;\n    proxy_ssl_name {};\n}}",
            site.anti_adblock_js_path,
            bootstrap,
            Url::parse(&analytics)?.host_str().unwrap(),
            Url::parse(&analytics)?.host_str().unwrap(),
            site.anti_adblock_beacon_path,
            collect,
            Url::parse(&analytics)?.host_str().unwrap(),
            site.proxy_key,
            Url::parse(&analytics)?.host_str().unwrap()
        ),
        "apache" => format!(
            "# Slimlytics first-party tracking\n# Requires mod_proxy, mod_proxy_http, mod_ssl, and mod_headers.\nSSLProxyEngine On\nProxyPassMatch \"^{}$\" \"{}\"\nProxyPassMatch \"^{}$\" \"{}\"\n<LocationMatch \"^(?:{}|{})$\">\n    RequestHeader unset Cookie\n    RequestHeader unset Authorization\n    RequestHeader unset X-Forwarded-For\n    Header always unset Set-Cookie\n</LocationMatch>\n<LocationMatch \"^{}$\">\n    RequestHeader set X-Slimlytics-Client-IP \"expr=%{{REMOTE_ADDR}}\"\n    RequestHeader set X-Slimlytics-Proxy-Key \"{}\"\n</LocationMatch>",
            regex_escape(&site.anti_adblock_js_path),
            bootstrap,
            regex_escape(&site.anti_adblock_beacon_path),
            collect,
            regex_escape(&site.anti_adblock_js_path),
            regex_escape(&site.anti_adblock_beacon_path),
            regex_escape(&site.anti_adblock_beacon_path),
            site.proxy_key
        ),
        other => bail!("unsupported server type: {other}"),
    };
    Ok(TrackingSetup {
        site_id: site.id,
        domain: site.domain.clone(),
        server_type: site.anti_adblock_server.clone(),
        javascript_path: site.anti_adblock_js_path.clone(),
        beacon_path: site.anti_adblock_beacon_path.clone(),
        server_config,
        snippet: format!(r#"<script async src="{}"></script>"#, site.anti_adblock_js_path),
        script_test_url: format!("{website}{}", site.anti_adblock_js_path),
        beacon_test_url: format!("{website}{}", site.anti_adblock_beacon_path),
        server_ingest_url: format!("{analytics}/api/ingest"),
        proxy_key: site.proxy_key,
        next_steps: vec![
            "Install serverConfig in the website's Caddy, Nginx, or Apache configuration and reload the server.".into(),
            "Add snippet to every page before the closing </body> tag.".into(),
            "Open scriptTestUrl and beaconTestUrl; both must return HTTP 200.".into(),
            "Custom or edge routes must send the visitor IP as X-Slimlytics-Client-IP and proxyKey as X-Slimlytics-Proxy-Key on the collection route, or locations and visitor counts will reflect the website's server.".into(),
            "Treat proxyKey as a secret: keep serverConfig out of public repositories (load the key from private deployment config) and rotate it with POST /api/sites/{siteId}/rotate-proxy-key if it is exposed.".into(),
        ],
    })
}

fn regex_escape(value: &str) -> String {
    let mut escaped = String::new();
    for character in value.chars() {
        if matches!(
            character,
            '.' | '+' | '*' | '?' | '^' | '$' | '(' | ')' | '[' | ']' | '{' | '}' | '|' | '\\'
        ) {
            escaped.push('\\');
        }
        escaped.push(character);
    }
    escaped
}

fn valid_proxy_path(value: &str, javascript: bool) -> bool {
    let Some(name) = value.strip_prefix('/') else {
        return false;
    };
    if name.contains('/') || name.len() > 67 {
        return false;
    }
    let stem = if javascript {
        let Some(stem) = name.strip_suffix(".js") else {
            return false;
        };
        stem
    } else {
        name
    };
    let max_stem_len = if javascript { 63 } else { 64 };
    (6..=max_stem_len).contains(&stem.len())
        && stem
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'~' | b'-'))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn site(server: &str) -> Site {
        Site {
            id: Uuid::nil(),
            name: "Shop".into(),
            domain: "shop.example.com".into(),
            timezone: "UTC".into(),
            allowed_origins: vec!["https://shop.example.com".into()],
            retention_days: 365,
            write_key: Uuid::from_u128(1),
            server_write_key: Uuid::from_u128(2),
            proxy_key: "6f1f6c2e-1d5e-4a3b-9f0e-2b7d6c5a4f31".parse().unwrap(),
            anti_adblock_server: server.into(),
            anti_adblock_js_path: "/ef691dcaa3fd.js".into(),
            anti_adblock_beacon_path: "/9b3c2d1e4f5a".into(),
            icon_mode: "initials".into(),
            icon_background: None,
            icon_background_end: None,
            icon_foreground: None,
            icon_updated_at: None,
            created_at: Utc::now(),
        }
    }

    /// Every generated proxy forwards the real visitor IP, vouched for by the site's proxy key,
    /// on the collection route only (the script route needs neither).
    #[test]
    fn collection_route_forwards_the_visitor_ip_with_the_proxy_key() {
        let key = "6f1f6c2e-1d5e-4a3b-9f0e-2b7d6c5a4f31";
        for (server, client_ip) in [
            ("caddy", "header_up X-Slimlytics-Client-IP {client_ip}"),
            (
                "nginx",
                "proxy_set_header X-Slimlytics-Client-IP $remote_addr;",
            ),
            (
                "apache",
                "RequestHeader set X-Slimlytics-Client-IP \"expr=%{REMOTE_ADDR}\"",
            ),
        ] {
            let setup = tracking_setup(&site(server), "https://slimlytics.com").unwrap();
            let config = &setup.server_config;
            assert!(config.contains(client_ip), "{server}: {config}");
            assert_eq!(
                config.matches(key).count(),
                1,
                "{server}: key once, on the collection route"
            );
            assert_eq!(
                config.matches("X-Slimlytics-Client-IP").count(),
                1,
                "{server}"
            );
            // The headers appear after the collection path, not in the script route.
            let collect = config.find("/api/collect/").unwrap();
            assert!(config.find(key).unwrap() > collect, "{server}: {config}");
            assert_eq!(setup.proxy_key.to_string(), key);
        }
    }
}
