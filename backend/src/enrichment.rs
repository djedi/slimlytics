use axum::http::HeaderMap;
use maxminddb::{geoip2, Reader};
use std::{net::IpAddr, path::Path};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct GeoLocation {
    pub country_code: Option<String>,
    pub region: Option<String>,
    pub city: Option<String>,
    pub continent: Option<String>,
}

impl GeoLocation {
    fn is_empty(&self) -> bool {
        self.country_code.is_none()
            && self.region.is_none()
            && self.city.is_none()
            && self.continent.is_none()
    }
}

pub struct GeoIp {
    reader: Reader<Vec<u8>>,
}

impl GeoIp {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, maxminddb::MaxMindDbError> {
        Ok(Self {
            reader: Reader::open_readfile(path)?,
        })
    }

    pub fn lookup(&self, ip: IpAddr) -> Option<GeoLocation> {
        let result = self.reader.lookup(ip).ok()?;
        let city = result.decode::<geoip2::City>().ok()??;
        let location = GeoLocation {
            country_code: city.country.iso_code.map(str::to_owned),
            region: city
                .subdivisions
                .first()
                .and_then(|subdivision| subdivision.iso_code.or(subdivision.names.english))
                .map(str::to_owned),
            city: city.city.names.english.map(str::to_owned),
            continent: city.continent.code.map(str::to_owned),
        };
        (!location.is_empty()).then_some(location)
    }
}

pub fn location_from_headers(headers: &HeaderMap, trust_proxy: bool) -> Option<GeoLocation> {
    if !trust_proxy {
        return None;
    }
    let location = GeoLocation {
        country_code: header_value(headers, "cf-ipcountry", 2, 2)
            .filter(|value| value.bytes().all(|byte| byte.is_ascii_alphabetic()))
            .map(|value| value.to_ascii_uppercase()),
        region: header_value(headers, "cf-region-code", 1, 64).map(str::to_owned),
        city: header_value(headers, "cf-ipcity", 1, 128).map(str::to_owned),
        continent: header_value(headers, "cf-ipcontinent", 2, 2)
            .filter(|value| value.bytes().all(|byte| byte.is_ascii_alphabetic()))
            .map(|value| value.to_ascii_uppercase()),
    };
    (!location.is_empty()).then_some(location)
}

fn header_value<'a>(
    headers: &'a HeaderMap,
    name: &'static str,
    min: usize,
    max: usize,
) -> Option<&'a str> {
    headers
        .get(name)
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| (min..=max).contains(&value.len()))
        .filter(|value| {
            value.bytes().all(|byte| {
                byte.is_ascii_alphanumeric() || matches!(byte, b' ' | b'-' | b'_' | b'.')
            })
        })
}

/// The visitor's preferred base language ("es" for `es-MX,es;q=0.9,en;q=0.8`), from the
/// `Accept-Language` header the browser already sends with every beacon. Only the primary
/// language subtag is kept: regional variants and the rest of the preference list add
/// fingerprinting entropy without telling a site owner which translations to offer.
pub fn language_from_header(value: Option<&str>) -> Option<String> {
    let value = value?;
    if value.len() > 512 {
        return None;
    }
    let mut best: Option<(String, u16)> = None;
    for entry in value.split(',').take(16) {
        let mut parts = entry.split(';');
        let tag = parts.next().unwrap_or("").trim();
        let quality = parts
            .find_map(|part| part.trim().strip_prefix("q="))
            .map_or(Some(1000), parse_quality);
        let Some(quality) = quality.filter(|quality| *quality > 0) else {
            continue;
        };
        let base = tag.split(['-', '_']).next().unwrap_or("");
        if !(2..=3).contains(&base.len()) || !base.bytes().all(|byte| byte.is_ascii_alphabetic()) {
            continue;
        }
        if best.as_ref().is_none_or(|(_, current)| quality > *current) {
            best = Some((base.to_ascii_lowercase(), quality));
        }
    }
    best.map(|(language, _)| language)
}

/// An HTTP quality value ("0.8") in thousandths, or `None` when it is malformed.
fn parse_quality(raw: &str) -> Option<u16> {
    let quality: f32 = raw.trim().parse().ok()?;
    (0.0..=1.0)
        .contains(&quality)
        .then(|| (quality * 1000.0).round() as u16)
}
