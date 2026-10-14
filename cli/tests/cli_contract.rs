use slimlytics_cli::{
    find_site, normalize_api_url, normalize_domain, save_auth, tracking_setup, ProxyKey, Site,
    StoredAuth,
};
use std::fs;
use uuid::Uuid;

fn site() -> Site {
    Site {
        id: Uuid::parse_str("df222f1c-8d95-4917-872e-98b30115aac8").unwrap(),
        name: "Example".into(),
        domain: "example.com".into(),
        timezone: "UTC".into(),
        allowed_origins: vec!["https://example.com".into()],
        retention_days: 365,
        write_key: Uuid::parse_str("d8f6f152-7a9e-4eb9-a8a1-468db4c0ea33").unwrap(),
        server_write_key: Some(Uuid::parse_str("7e55bd93-2601-46fc-881a-e847209f25f1").unwrap()),
        proxy_key: None,
        server_write_key_hint: None,
        proxy_key_hint: None,
        anti_adblock_server: "caddy".into(),
        anti_adblock_js_path: "/456bbb63bb86.js".into(),
        anti_adblock_beacon_path: "/0d31360a3101".into(),
        created_at: "2026-07-29T00:00:00Z".into(),
    }
}

#[test]
fn domains_are_normalized_for_site_creation() {
    assert_eq!(normalize_domain("Example.COM").unwrap(), "example.com");
    assert_eq!(
        normalize_domain("https://example.com/").unwrap(),
        "example.com"
    );
    assert!(normalize_domain("https://example.com/path").is_err());
    assert!(normalize_domain("example.com:8443").is_err());
    assert!(normalize_domain("not a domain").is_err());
}

#[test]
fn a_site_can_be_selected_by_id_or_domain() {
    let sites = vec![site()];
    assert_eq!(find_site(&sites, "example.com").unwrap().id, sites[0].id);
    assert_eq!(
        find_site(&sites, "df222f1c-8d95-4917-872e-98b30115aac8")
            .unwrap()
            .domain,
        "example.com"
    );
    assert!(find_site(&sites, "missing.example").is_err());

    let mut duplicate = site();
    duplicate.id = Uuid::parse_str("a20be759-6926-49e1-ab95-bd2d75f1b883").unwrap();
    assert!(find_site(&[site(), duplicate], "example.com").is_err());
}

#[test]
fn tracking_setup_is_complete_and_ai_friendly() {
    let setup = tracking_setup(&site(), "https://slimlytics.com", ProxyKey::Placeholder).unwrap();
    assert_eq!(
        setup.snippet,
        r#"<script async src="/456bbb63bb86.js"></script>"#
    );
    assert!(setup.server_config.contains("handle /456bbb63bb86.js"));
    assert!(setup
        .server_config
        .contains("rewrite /456bbb63bb86.js /p/d8f6f152-7a9e-4eb9-a8a1-468db4c0ea33/0d31360a3101"));
    assert!(setup
        .server_config
        .contains("reverse_proxy https://slimlytics.com"));
    assert!(!setup
        .server_config
        .contains("reverse_proxy https://slimlytics.com/p/"));
    assert!(setup.server_config.contains("header_up -Authorization"));
    assert!(setup
        .server_config
        .contains("/api/collect/d8f6f152-7a9e-4eb9-a8a1-468db4c0ea33"));
    assert_eq!(setup.script_test_url, "https://example.com/456bbb63bb86.js");
    assert_eq!(setup.beacon_test_url, "https://example.com/0d31360a3101");
    assert_eq!(setup.server_ingest_url, "https://slimlytics.com/api/ingest");

    let mut unsafe_site = site();
    unsafe_site.anti_adblock_js_path = "/valid.js\nheader injected".into();
    assert!(tracking_setup(
        &unsafe_site,
        "https://slimlytics.com",
        ProxyKey::Placeholder
    )
    .is_err());
}

#[test]
fn beacon_route_forwards_the_visitor_ip_with_the_proxy_key_placeholder() {
    let key = Uuid::parse_str("6f1f6c2e-1d5e-4a3b-9f0e-2b7d6c5a4f31").unwrap();
    for (server, header, placeholder) in [
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
        // Default: the config references the environment variable, even when the key is known.
        let mut keyed = site();
        keyed.proxy_key = Some(key);
        keyed.anti_adblock_server = server.into();
        let setup =
            tracking_setup(&keyed, "https://slimlytics.com", ProxyKey::Placeholder).unwrap();
        assert!(setup.server_config.contains(header), "{server}");
        assert_eq!(
            setup.server_config.matches(placeholder).count(),
            1,
            "{server}"
        );
        assert!(!setup.server_config.contains(&key.to_string()), "{server}");
        assert!(setup.proxy_key.is_none(), "{server}");
        assert_eq!(setup.proxy_key_hint.as_deref(), Some("…4f31"));
        assert!(
            setup.server_config.find(placeholder).unwrap()
                > setup.server_config.find("/api/collect/").unwrap(),
            "{server}: headers belong to the collection route"
        );
        let json = serde_json::to_string(&setup).unwrap();
        assert!(!json.contains(&key.to_string()), "{server}: {json}");

        // A caller that only sees the hint gets the same placeholder config.
        let mut hinted = site();
        hinted.proxy_key_hint = Some("…4f31".into());
        hinted.anti_adblock_server = server.into();
        let setup =
            tracking_setup(&hinted, "https://slimlytics.com", ProxyKey::Placeholder).unwrap();
        assert!(setup.server_config.contains(placeholder), "{server}");
        // ...but cannot ask for the key to be embedded.
        assert!(tracking_setup(&hinted, "https://slimlytics.com", ProxyKey::Include).is_err());

        // --include-proxy-key writes the key once, on the collection route.
        let included = tracking_setup(&keyed, "https://slimlytics.com", ProxyKey::Include).unwrap();
        assert_eq!(
            included.server_config.matches(&key.to_string()).count(),
            1,
            "{server}"
        );
        assert!(
            !included.server_config.contains("SLIMLYTICS_PROXY_KEY"),
            "{server}"
        );
        assert_eq!(included.proxy_key, Some(key));

        // Without a key or hint (older servers) the config is unchanged.
        let mut unkeyed = site();
        unkeyed.anti_adblock_server = server.into();
        let plain =
            tracking_setup(&unkeyed, "https://slimlytics.com", ProxyKey::Placeholder).unwrap();
        assert!(
            !plain.server_config.contains("X-Slimlytics-Client-IP"),
            "{server}"
        );
    }
}

#[test]
fn printed_sites_never_include_secret_keys() {
    let mut keyed = site();
    keyed.proxy_key = Some(Uuid::parse_str("6f1f6c2e-1d5e-4a3b-9f0e-2b7d6c5a4f31").unwrap());
    let json = serde_json::to_string(&keyed).unwrap();
    assert!(
        !json.contains("7e55bd93-2601-46fc-881a-e847209f25f1"),
        "{json}"
    );
    assert!(
        !json.contains("6f1f6c2e-1d5e-4a3b-9f0e-2b7d6c5a4f31"),
        "{json}"
    );
    assert!(
        json.contains("d8f6f152-7a9e-4eb9-a8a1-468db4c0ea33"),
        "write key stays"
    );
}

#[test]
fn sites_without_secret_keys_still_parse() {
    let site: Site = serde_json::from_str(r#"{"id":"df222f1c-8d95-4917-872e-98b30115aac8","name":"Example","domain":"example.com","timezone":"UTC","allowedOrigins":[],"retentionDays":365,"writeKey":"d8f6f152-7a9e-4eb9-a8a1-468db4c0ea33","serverWriteKeyHint":"…25f1","proxyKeyHint":"…4f31","canManageKeys":false,"antiAdblockServer":"nginx","antiAdblockJsPath":"/456bbb63bb86.js","antiAdblockBeaconPath":"/0d31360a3101","createdAt":"2026-07-29T00:00:00Z"}"#).unwrap();
    assert!(site.server_write_key.is_none());
    assert!(site.proxy_key.is_none());
    let setup = tracking_setup(&site, "https://slimlytics.com", ProxyKey::Placeholder).unwrap();
    assert!(setup.server_config.contains("${SLIMLYTICS_PROXY_KEY}"));
}

#[test]
fn auth_file_is_private_and_round_trips() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("auth.json");
    let auth = StoredAuth {
        api_url: "https://slimlytics.com".into(),
        token: "slyt_test-token".into(),
    };
    save_auth(&path, &auth).unwrap();
    let decoded: StoredAuth = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    assert_eq!(decoded, auth);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            fs::metadata(path).unwrap().permissions().mode() & 0o777,
            0o600
        );
    }
}

#[cfg(unix)]
#[test]
fn auth_file_does_not_follow_symbolic_links() {
    use std::os::unix::fs::symlink;
    let directory = tempfile::tempdir().unwrap();
    let victim = directory.path().join("victim");
    let path = directory.path().join("auth.json");
    fs::write(&victim, "do not overwrite").unwrap();
    symlink(&victim, &path).unwrap();
    let auth = StoredAuth {
        api_url: "https://slimlytics.com".into(),
        token: "slyt_test-token".into(),
    };
    assert!(save_auth(&path, &auth).is_err());
    assert_eq!(fs::read_to_string(victim).unwrap(), "do not overwrite");
}

#[test]
fn plaintext_api_urls_are_limited_to_loopback_development() {
    assert!(normalize_api_url("http://127.0.0.1:8080").is_ok());
    assert!(normalize_api_url("http://localhost:8080").is_ok());
    assert!(normalize_api_url("http://analytics.example.com").is_err());
    assert!(normalize_api_url("https://analytics.example.com").is_ok());
}
