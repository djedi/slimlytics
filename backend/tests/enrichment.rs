use axum::http::HeaderMap;
use slimlytics_backend::enrichment::location_from_headers;

#[test]
fn ignores_edge_location_headers_without_proxy_trust() {
    let mut headers = HeaderMap::new();
    headers.insert("cf-ipcountry", "US".parse().unwrap());
    headers.insert("cf-region-code", "CO".parse().unwrap());
    headers.insert("cf-ipcity", "Denver".parse().unwrap());
    headers.insert("cf-ipcontinent", "NA".parse().unwrap());

    assert_eq!(location_from_headers(&headers, false), None);
}

#[test]
fn reads_and_validates_trusted_edge_location_headers() {
    let mut headers = HeaderMap::new();
    headers.insert("cf-ipcountry", "us".parse().unwrap());
    headers.insert("cf-region-code", "CO".parse().unwrap());
    headers.insert("cf-ipcity", "Denver".parse().unwrap());
    headers.insert("cf-ipcontinent", "NA".parse().unwrap());

    let location = location_from_headers(&headers, true).unwrap();
    assert_eq!(location.country_code.as_deref(), Some("US"));
    assert_eq!(location.region.as_deref(), Some("CO"));
    assert_eq!(location.city.as_deref(), Some("Denver"));
    assert_eq!(location.continent.as_deref(), Some("NA"));
}

#[test]
fn drops_invalid_edge_location_values() {
    let mut headers = HeaderMap::new();
    headers.insert("cf-ipcountry", "USA".parse().unwrap());
    headers.insert("cf-region-code", "<script>".parse().unwrap());
    headers.insert("cf-ipcity", "x".repeat(129).parse().unwrap());

    assert_eq!(location_from_headers(&headers, true), None);
}

/// Verifies a real downloaded database (`make geoip`) resolves public addresses. DB-IP's
/// schema matches GeoIP2 City, which is what the reader decodes.
/// CI runs ignored tests for PostgreSQL but has no 120 MB GeoIP file, so this skips
/// unless GEOIP_TEST_DATABASE is set.
#[test]
#[ignore = "set GEOIP_TEST_DATABASE to a DB-IP or MaxMind City .mmdb file"]
fn resolves_public_addresses_from_a_downloaded_city_database() {
    let Ok(path) = std::env::var("GEOIP_TEST_DATABASE") else {
        eprintln!("skipping: GEOIP_TEST_DATABASE is not set");
        return;
    };
    let geoip = slimlytics_backend::enrichment::GeoIp::open(path).unwrap();
    let google = geoip.lookup("8.8.8.8".parse().unwrap()).unwrap();
    assert_eq!(google.country_code.as_deref(), Some("US"));
    assert_eq!(google.continent.as_deref(), Some("NA"));
    let cloudflare = geoip.lookup("1.1.1.1".parse().unwrap());
    assert!(cloudflare.is_some());
    assert!(
        geoip.lookup("10.0.0.1".parse().unwrap()).is_none(),
        "private ranges have no location"
    );
}

#[test]
fn reads_the_preferred_base_language_from_accept_language() {
    use slimlytics_backend::enrichment::language_from_header;

    assert_eq!(
        language_from_header(Some("es-MX,es;q=0.9,en-US;q=0.8")).as_deref(),
        Some("es")
    );
    assert_eq!(language_from_header(Some("pt-BR")).as_deref(), Some("pt"));
    assert_eq!(language_from_header(Some("EN_us")).as_deref(), Some("en"));
    assert_eq!(language_from_header(Some("fil-PH")).as_deref(), Some("fil"));
    // The highest quality wins, not the first listed.
    assert_eq!(
        language_from_header(Some("en;q=0.5, de;q=0.9")).as_deref(),
        Some("de")
    );
    // Wildcards and refused languages are skipped.
    assert_eq!(
        language_from_header(Some("*, fr;q=0.8")).as_deref(),
        Some("fr")
    );
    assert_eq!(
        language_from_header(Some("ja;q=0, ko;q=0.1")).as_deref(),
        Some("ko")
    );
}

#[test]
fn ignores_missing_or_malformed_accept_language() {
    use slimlytics_backend::enrichment::language_from_header;

    assert_eq!(language_from_header(None), None);
    assert_eq!(language_from_header(Some("")), None);
    assert_eq!(language_from_header(Some("*")), None);
    assert_eq!(language_from_header(Some("<script>")), None);
    assert_eq!(language_from_header(Some("x")), None);
    assert_eq!(language_from_header(Some("english")), None);
    assert_eq!(language_from_header(Some("en;q=0")), None);
    assert_eq!(language_from_header(Some(&"en,".repeat(200))), None);
}
