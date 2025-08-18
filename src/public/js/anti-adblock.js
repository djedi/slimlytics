document.addEventListener("DOMContentLoaded", async () => {
	const form = document.getElementById("anti-adblock-form");
	const configOutput = document.getElementById("config-output");

	form.addEventListener("submit", async (e) => {
		e.preventDefault();

		// Get current site from SiteManager
		const currentSite = await window.SiteManager.getSelectedSite();
		if (!currentSite) {
			alert("Please select a site first");
			return;
		}

		const formData = new FormData(form);
		const siteId = currentSite.id;
		const siteDomain = currentSite.domain;
		const jsPath = formData.get("jsPath");
		const beaconPath = formData.get("beaconPath");

		// Generate random-looking paths if user wants to obfuscate
		const jsPathClean = jsPath.startsWith("/") ? jsPath.substring(1) : jsPath;
		const beaconPathClean = beaconPath.startsWith("/")
			? beaconPath.substring(1)
			: beaconPath;

		// Get the API base URL (assuming it's the same server in production)
		const apiBase =
			window.location.hostname === "localhost"
				? "http://localhost:3000"
				: window.location.origin;

		// Generate Caddy configuration
		const caddyConfig = `### SLIM ANALYTICS ANTI-ADBLOCK PROXY - ${apiBase}
### COPY INTO YOUR WEBSITE'S CADDYFILE

# Note: Caddy overrides X-Forwarded-For headers by default, unless "trusted_proxies" is configured.
# Please see the following links for more information:
# https://caddyserver.com/docs/caddyfile/directives/reverse_proxy#defaults
# https://caddyserver.com/docs/caddyfile/options#trusted-proxies

# TRACKING SCRIPT
handle /${jsPathClean} {
    rewrite /${jsPathClean} /sa.js
    reverse_proxy ${apiBase} {
        header_up Host {upstream_hostport}
    }
}

# BEACON ENDPOINT
handle /${beaconPathClean} {
    rewrite /${beaconPathClean} /track
    reverse_proxy ${apiBase} {
        header_up Host {upstream_hostport}
    }
}

# NOSCRIPT GIF BEACON
handle /${beaconPathClean}/*.gif {
    reverse_proxy ${apiBase} {
        header_up Host {upstream_hostport}
    }
}`;

		// Generate tracking code (simplified like Clicky)
		const trackingCode = `<script async data-id="${siteId}" src="/${jsPathClean}"></script>
<noscript><p><img alt="Slimlytics" width="1" height="1" src="/${beaconPathClean}/${siteId}ns.gif" /></p></noscript>`;

		// Generate test URLs
		const testUrls = `
      <a href="https://${siteDomain}/${jsPathClean}" target="_blank">
        https://${siteDomain}/${jsPathClean} (should load the tracking script)
      </a>
      <a href="https://${siteDomain}/${beaconPathClean}" target="_blank">
        https://${siteDomain}/${beaconPathClean} (should return a 1x1 pixel or empty response)
      </a>
    `;

		// Display configuration
		document.getElementById("caddy-config").textContent = caddyConfig;
		document.getElementById("tracking-code").textContent = trackingCode;
		document.getElementById("test-urls").innerHTML = testUrls;
		configOutput.style.display = "block";

		// Scroll to output
		configOutput.scrollIntoView({ behavior: "smooth" });
	});
});
