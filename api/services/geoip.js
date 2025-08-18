import { Reader } from "@maxmind/geoip2-node";
import path from "node:path";
import fs from "node:fs";

class GeoIPService {
	constructor() {
		this.cityReader = null;
		this.countryReader = null;
		this.asnReader = null;
		this.initialized = false;
	}

	async initialize() {
		if (this.initialized) return;

		const dataDir = path.join(process.cwd(), "data", "maxmind");

		try {
			// Load City database if available
			const cityDbPath = path.join(dataDir, "GeoLite2-City.mmdb");
			if (fs.existsSync(cityDbPath)) {
				this.cityReader = await Reader.open(cityDbPath);
				console.log("✓ GeoLite2-City database loaded");
			}

			// Load Country database if available
			const countryDbPath = path.join(dataDir, "GeoLite2-Country.mmdb");
			if (fs.existsSync(countryDbPath)) {
				this.countryReader = await Reader.open(countryDbPath);
				console.log("✓ GeoLite2-Country database loaded");
			}

			// Load ASN database if available
			const asnDbPath = path.join(dataDir, "GeoLite2-ASN.mmdb");
			if (fs.existsSync(asnDbPath)) {
				this.asnReader = await Reader.open(asnDbPath);
				console.log("✓ GeoLite2-ASN database loaded");
			}

			this.initialized = true;
		} catch (error) {
			console.error("Error initializing GeoIP service:", error);
			throw error;
		}
	}

	async lookup(ip) {
		if (!this.initialized) {
			await this.initialize();
		}

		const result = {
			ip,
			country: null,
			countryCode: null,
			region: null,
			city: null,
			postalCode: null,
			latitude: null,
			longitude: null,
			timezone: null,
			asn: null,
			asnOrg: null,
		};

		// Skip private/local IPs
		if (this.isPrivateIP(ip)) {
			result.country = "Private Network";
			result.countryCode = "XX";
			return result;
		}

		try {
			// Try city lookup first (most detailed)
			if (this.cityReader) {
				try {
					const cityResponse = this.cityReader.city(ip);

					result.country = cityResponse.country?.names?.en || null;
					result.countryCode = cityResponse.country?.isoCode || null;
					result.region = cityResponse.subdivisions?.[0]?.names?.en || null;
					result.city = cityResponse.city?.names?.en || null;
					result.postalCode = cityResponse.postal?.code || null;
					result.latitude = cityResponse.location?.latitude || null;
					result.longitude = cityResponse.location?.longitude || null;
					result.timezone = cityResponse.location?.timeZone || null;
				} catch (error) {
					// IP not found in city database, try country
					if (this.countryReader) {
						try {
							const countryResponse = this.countryReader.country(ip);
							result.country = countryResponse.country?.names?.en || null;
							result.countryCode = countryResponse.country?.isoCode || null;
						} catch (e) {
							// IP not found in country database either
						}
					}
				}
			} else if (this.countryReader) {
				// Fallback to country-only lookup
				try {
					const countryResponse = this.countryReader.country(ip);
					result.country = countryResponse.country?.names?.en || null;
					result.countryCode = countryResponse.country?.isoCode || null;
				} catch (error) {
					// IP not found
				}
			}

			// ASN lookup
			if (this.asnReader) {
				try {
					const asnResponse = this.asnReader.asn(ip);
					result.asn = asnResponse.autonomousSystemNumber || null;
					result.asnOrg = asnResponse.autonomousSystemOrganization || null;
				} catch (error) {
					// IP not found in ASN database
				}
			}
		} catch (error) {
			console.error("Error looking up IP:", ip, error.message);
		}

		return result;
	}

	isPrivateIP(ip) {
		// Check for IPv4 private ranges
		const parts = ip.split(".");
		if (parts.length === 4) {
			const first = Number.parseInt(parts[0]);
			const second = Number.parseInt(parts[1]);

			// 10.0.0.0 - 10.255.255.255
			if (first === 10) return true;

			// 172.16.0.0 - 172.31.255.255
			if (first === 172 && second >= 16 && second <= 31) return true;

			// 192.168.0.0 - 192.168.255.255
			if (first === 192 && second === 168) return true;

			// 127.0.0.0 - 127.255.255.255 (localhost)
			if (first === 127) return true;
		}

		// Check for IPv6 localhost
		if (ip === "::1" || ip === "::" || ip.startsWith("fe80:")) return true;

		return false;
	}

	async close() {
		if (this.cityReader) this.cityReader.close();
		if (this.countryReader) this.countryReader.close();
		if (this.asnReader) this.asnReader.close();
		this.initialized = false;
	}
}

// Export singleton instance
export const geoip = new GeoIPService();
export default geoip;
