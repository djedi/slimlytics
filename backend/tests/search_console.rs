use slimlytics_backend::search_console::{
    authorization_url, decrypt_token, domain_property, encrypt_token, forbidden_message,
    missing_scopes, preferred_property, site_resource_url, verification_request,
    verified_property_urls, SearchConsoleConfig, GOOGLE_SCOPES,
};

#[test]
fn encrypts_refresh_tokens_with_authenticated_encryption() {
    let key = [7_u8; 32];
    let encrypted = encrypt_token(&key, "refresh-secret").unwrap();

    assert_ne!(encrypted, "refresh-secret");
    assert_eq!(decrypt_token(&key, &encrypted).unwrap(), "refresh-secret");

    let mut tampered = encrypted;
    tampered.push('x');
    assert!(decrypt_token(&key, &tampered).is_err());
}

#[test]
fn prefers_domain_property_then_https_prefix() {
    let properties = vec![
        "https://example.com/".to_string(),
        "sc-domain:example.com".to_string(),
        "https://other.example/".to_string(),
    ];
    assert_eq!(
        preferred_property("example.com", &properties).as_deref(),
        Some("sc-domain:example.com")
    );
    assert_eq!(
        preferred_property("other.example", &properties).as_deref(),
        Some("https://other.example/")
    );
}

#[test]
fn ignores_properties_the_account_has_not_verified() {
    let entries = vec![
        (
            "sc-domain:example.com".to_string(),
            "siteUnverifiedUser".to_string(),
        ),
        ("https://example.com/".to_string(), "siteOwner".to_string()),
    ];
    assert_eq!(
        verified_property_urls(entries),
        vec!["https://example.com/".to_string()]
    );
}

#[test]
fn requests_permission_to_add_and_verify_properties() {
    let config = SearchConsoleConfig {
        client_id: "client".into(),
        client_secret: "secret".into(),
        redirect_uri: "https://app.example/callback".into(),
        return_uri: "https://app.example/app".into(),
        encryption_key: [1_u8; 32],
    };
    let url = url::Url::parse(&authorization_url(&config, "state")).unwrap();
    let scope = url
        .query_pairs()
        .find(|(key, _)| key == "scope")
        .map(|(_, value)| value.into_owned())
        .unwrap();
    let scopes: Vec<&str> = scope.split(' ').collect();
    assert_eq!(scopes, GOOGLE_SCOPES);
    assert!(scopes.contains(&"https://www.googleapis.com/auth/webmasters"));
    assert!(scopes.contains(&"https://www.googleapis.com/auth/siteverification"));
    assert!(!scope.contains("readonly"));
}

#[test]
fn adds_sites_as_domain_properties() {
    assert_eq!(domain_property("Example.com"), "sc-domain:example.com");
    assert_eq!(
        site_resource_url("sc-domain:example.com"),
        "https://www.googleapis.com/webmasters/v3/sites/sc-domain%3Aexample.com"
    );
    assert_eq!(
        site_resource_url("https://example.com/"),
        "https://www.googleapis.com/webmasters/v3/sites/https%3A%2F%2Fexample.com%2F"
    );
}

#[test]
fn verifies_domain_properties_with_a_dns_txt_record() {
    assert_eq!(
        verification_request("Example.com"),
        serde_json::json!({
            "site":{"type":"INET_DOMAIN","identifier":"example.com"},
            "verificationMethod":"DNS_TXT"
        })
    );
}

#[test]
fn reports_scopes_the_user_did_not_grant() {
    assert!(missing_scopes(&GOOGLE_SCOPES.join(" ")).is_empty());
    assert_eq!(
        missing_scopes("openid https://www.googleapis.com/auth/webmasters"),
        vec!["https://www.googleapis.com/auth/siteverification"]
    );
    assert_eq!(missing_scopes(""), GOOGLE_SCOPES.to_vec());
}

#[test]
fn explains_why_google_denied_permission() {
    let disabled = r#"{"error":{"code":403,"message":"Site Verification API has not been used in project 1 before or it is disabled.","status":"PERMISSION_DENIED","details":[{"@type":"type.googleapis.com/google.rpc.ErrorInfo","reason":"SERVICE_DISABLED"}]}}"#;
    assert!(forbidden_message(disabled).contains("not enabled for its Google Cloud project"));

    let scope = r#"{"error":{"code":403,"message":"Insufficient Permission","errors":[{"reason":"insufficientPermissions"}]}}"#;
    assert!(forbidden_message(scope).contains("Reconnect Search Console"));

    let other =
        r#"{"error":{"code":403,"message":"User does not have sufficient permission for site"}}"#;
    assert_eq!(
        forbidden_message(other),
        "Google denied permission: User does not have sufficient permission for site"
    );
    assert!(!forbidden_message("not json").is_empty());
}
