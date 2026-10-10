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
    /// Environment variable `serverConfig` reads the proxy key from.
    pub proxy_key_env: &'static str,
    /// How `serverConfig` references the proxy key in this server's configuration syntax.
    pub proxy_key_placeholder: String,
    /// Last four characters of the proxy key, so an operator can confirm which key is installed.
    pub proxy_key_hint: String,
    /// The full proxy key, sent as X-Slimlytics-Proxy-Key with X-Slimlytics-Client-IP by the
    /// collection route so visitor locations and IDs use the real visitor IP. A server-side
    /// secret, so it is only included when the caller asked for [`ProxyKey::Include`] (a site
    /// that was just created by a caller with write access). Otherwise operators copy it from the
    /// dashboard.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub proxy_key: Option<Uuid>,
    pub next_steps: Vec<String>,
}

/// Environment variable the generated proxy configuration reads the proxy key from.
pub const PROXY_KEY_ENV: &str = "SLIMLYTICS_PROXY_KEY";

/// Whether the full proxy key is returned alongside the generated configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProxyKey {
    /// Return only `proxyKeyHint`; the configuration references [`PROXY_KEY_ENV`].
    Redact,
    /// Also return the full key in `proxyKey`. The configuration still references
    /// [`PROXY_KEY_ENV`] so it can be committed without the key.
    Include,
}

/// The placeholder for the proxy key in each server's own environment-variable syntax.
fn proxy_key_placeholder(server: &str) -> String {
    match server {
        // Caddyfile substitutes {$VAR} from the environment when the config is loaded.
        "caddy" => format!("{{${PROXY_KEY_ENV}}}"),
        // Apache substitutes ${VAR} from the environment at startup; for Nginx, render it with
        // envsubst (or the official image's /etc/nginx/templates) before loading.
        _ => format!("${{{PROXY_KEY_ENV}}}"),
    }
}

fn proxy_key_hint(key: Uuid) -> String {
    let key = key.to_string();
    format!("…{}", &key[key.len() - 4..])
}

pub fn tracking_setup(
    site: &Site,
    analytics_origin: &str,
    reveal: ProxyKey,
) -> Result<TrackingSetup> {
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
    let key_placeholder = proxy_key_placeholder(&site.anti_adblock_server);
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
            key_placeholder
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
            key_placeholder,
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
            key_placeholder
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
        proxy_key_placeholder: key_placeholder.clone(),
        proxy_key_env: PROXY_KEY_ENV,
        proxy_key_hint: proxy_key_hint(site.proxy_key),
        proxy_key: (reveal == ProxyKey::Include).then_some(site.proxy_key),
        next_steps: vec![
            format!(
                "Set the {PROXY_KEY_ENV} environment variable for the website's web server to the site's proxy key (ending {}). {} Copy it from the site's Anti-adblock tracking settings in the Slimlytics dashboard and keep it in private deployment config, not in the repository.",
                proxy_key_hint(site.proxy_key),
                if reveal == ProxyKey::Include { "It is also returned once as proxyKey for this newly created site." } else { "It is not included in this response." },
            ),
            match site.anti_adblock_server.as_str() {
                "caddy" => format!("Install serverConfig in the website's Caddyfile; Caddy reads {key_placeholder} from the environment when the config loads. Reload the server."),
                "nginx" => format!("Nginx does not read environment variables itself: render serverConfig with envsubst '{key_placeholder}' (or place it in the official image's /etc/nginx/templates) so only {key_placeholder} is replaced, include the result in the website's server block, and reload."),
                _ => format!("Install serverConfig in the website's Apache configuration; Apache substitutes {key_placeholder} from the environment at startup (for example via envvars or the service environment). Reload the server."),
            },
            "Add snippet to every page before the closing </body> tag.".into(),
            "Open scriptTestUrl and beaconTestUrl; both must return HTTP 200.".into(),
            "Custom or edge routes must send the visitor IP as X-Slimlytics-Client-IP and the proxy key as X-Slimlytics-Proxy-Key on the collection route, or locations and visitor counts will reflect the website's server.".into(),
            "Rotate the proxy key with POST /api/sites/{siteId}/rotate-proxy-key (or from the dashboard) if it is ever exposed, then update the environment variable.".into(),
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

    const KEY: &str = "6f1f6c2e-1d5e-4a3b-9f0e-2b7d6c5a4f31";

    /// Every generated proxy forwards the real visitor IP, vouched for by the proxy key read from
    /// the environment, on the collection route only (the script route needs neither).
    #[test]
    fn collection_route_forwards_the_visitor_ip_with_the_proxy_key_placeholder() {
        for (server, client_ip, placeholder) in [
            (
                "caddy",
                "header_up X-Slimlytics-Client-IP {client_ip}",
                "header_up X-Slimlytics-Proxy-Key {$SLIMLYTICS_PROXY_KEY}",
            ),
            (
                "nginx",
                "proxy_set_header X-Slimlytics-Client-IP $remote_addr;",
                "proxy_set_header X-Slimlytics-Proxy-Key ${SLIMLYTICS_PROXY_KEY};",
            ),
            (
                "apache",
                "RequestHeader set X-Slimlytics-Client-IP \"expr=%{REMOTE_ADDR}\"",
                "RequestHeader set X-Slimlytics-Proxy-Key \"${SLIMLYTICS_PROXY_KEY}\"",
            ),
        ] {
            let setup =
                tracking_setup(&site(server), "https://slimlytics.com", ProxyKey::Redact).unwrap();
            let config = &setup.server_config;
            assert!(config.contains(client_ip), "{server}: {config}");
            assert_eq!(
                config.matches(placeholder).count(),
                1,
                "{server}: key placeholder once, on the collection route: {config}"
            );
            assert_eq!(
                config.matches("X-Slimlytics-Client-IP").count(),
                1,
                "{server}"
            );
            // The headers appear after the collection path, not in the script route.
            let collect = config.find("/api/collect/").unwrap();
            assert!(
                config.find(placeholder).unwrap() > collect,
                "{server}: {config}"
            );
            assert!(
                placeholder.contains(&setup.proxy_key_placeholder),
                "{server}"
            );
        }
    }

    /// Read-only callers get a hint and instructions, never the key itself.
    #[test]
    fn redacted_setup_never_contains_the_proxy_key() {
        for server in ["caddy", "nginx", "apache"] {
            let setup =
                tracking_setup(&site(server), "https://slimlytics.com", ProxyKey::Redact).unwrap();
            let json = serde_json::to_string(&setup).unwrap();
            assert!(!json.contains(KEY), "{server}: {json}");
            assert!(!json.contains(&KEY.replace('-', "")), "{server}");
            let value = serde_json::to_value(&setup).unwrap();
            assert!(value.get("proxyKey").is_none(), "{server}");
            assert_eq!(value["proxyKeyHint"], "…4f31");
            assert_eq!(value["proxyKeyEnv"], "SLIMLYTICS_PROXY_KEY");
            assert!(
                setup
                    .next_steps
                    .iter()
                    .any(|step| step.contains("dashboard")),
                "{server}: tells the operator where to get the key"
            );
        }
    }

    /// A caller that just created the site with write access receives the key once, but the
    /// configuration itself still references the environment variable.
    #[test]
    fn included_key_stays_out_of_server_config() {
        let setup =
            tracking_setup(&site("nginx"), "https://slimlytics.com", ProxyKey::Include).unwrap();
        assert_eq!(setup.proxy_key.unwrap().to_string(), KEY);
        assert!(!setup.server_config.contains(KEY));
        assert!(setup.server_config.contains("${SLIMLYTICS_PROXY_KEY}"));
    }
}
