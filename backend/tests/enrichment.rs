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
