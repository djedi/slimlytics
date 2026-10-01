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
            "# Slimlytics first-party tracking\nhandle {} {{\n\trewrite {} {}\n\treverse_proxy {} {{\n\t\theader_up Host {{upstream_hostport}}\n\t\theader_up -Cookie\n\t\theader_up -Authorization\n\t\theader_down -Set-Cookie\n\t}}\n}}\n\nhandle {} {{\n\trewrite {} {}\n\treverse_proxy {} {{\n\t\theader_up Host {{upstream_hostport}}\n\t\theader_up -Cookie\n\t\theader_up -Authorization\n\t\theader_down -Set-Cookie\n\t}}\n}}",
            site.anti_adblock_js_path,
            site.anti_adblock_js_path,
            bootstrap_path,
            analytics,
            site.anti_adblock_beacon_path,
            site.anti_adblock_beacon_path,
            collect_path,
            analytics
        ),
        "nginx" => format!(
            "# Slimlytics first-party tracking\nlocation = {} {{\n    proxy_pass {};\n    proxy_set_header Host {};\n    proxy_set_header Cookie \"\";\n    proxy_set_header Authorization \"\";\n    proxy_set_header X-Forwarded-For $remote_addr;\n    proxy_hide_header Set-Cookie;\n    proxy_ssl_server_name on;\n    proxy_ssl_name {};\n}}\n\nlocation = {} {{\n    proxy_pass {};\n    proxy_set_header Host {};\n    proxy_set_header Cookie \"\";\n    proxy_set_header Authorization \"\";\n    proxy_set_header X-Forwarded-For $remote_addr;\n    proxy_hide_header Set-Cookie;\n    proxy_ssl_server_name on;\n    proxy_ssl_name {};\n}}",
            site.anti_adblock_js_path,
            bootstrap,
            Url::parse(&analytics)?.host_str().unwrap(),
            Url::parse(&analytics)?.host_str().unwrap(),
            site.anti_adblock_beacon_path,
            collect,
            Url::parse(&analytics)?.host_str().unwrap(),
            Url::parse(&analytics)?.host_str().unwrap()
        ),
        "apache" => format!(
            "# Slimlytics first-party tracking\n# Requires mod_proxy, mod_proxy_http, mod_ssl, and mod_headers.\nSSLProxyEngine On\nProxyPassMatch \"^{}$\" \"{}\"\nProxyPassMatch \"^{}$\" \"{}\"\n<LocationMatch \"^(?:{}|{})$\">\n    RequestHeader unset Cookie\n    RequestHeader unset Authorization\n    RequestHeader unset X-Forwarded-For\n    Header always unset Set-Cookie\n</LocationMatch>",
            regex_escape(&site.anti_adblock_js_path),
            bootstrap,
            regex_escape(&site.anti_adblock_beacon_path),
            collect,
            regex_escape(&site.anti_adblock_js_path),
            regex_escape(&site.anti_adblock_beacon_path)
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
        next_steps: vec![
            "Install serverConfig in the website's Caddy, Nginx, or Apache configuration and reload the server.".into(),
            "Add snippet to every page before the closing </body> tag.".into(),
            "Open scriptTestUrl and beaconTestUrl; both must return HTTP 200.".into(),
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
