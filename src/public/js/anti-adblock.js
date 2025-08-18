document.addEventListener("DOMContentLoaded", async () => {
	const form = document.getElementById("anti-adblock-form");
	const siteSelect = document.getElementById("site-select");
	const configOutput = document.getElementById("config-output");

	// Load sites
	try {
		const response = await fetch(
			window.SLIMLYTICS_CONFIG.apiEndpoint("/api/sites"),
		);
		const sites = await response.json();

		siteSelect.innerHTML = '<option value="">Select a site...</option>';
		for (const site of sites) {
			const option = document.createElement("option");
			option.value = site.id;
			option.textContent = `${site.name} (${site.domain})`;
			siteSelect.appendChild(option);
		}
	} catch (error) {
		console.error("Failed to load sites:", error);
		siteSelect.innerHTML = '<option value="">Failed to load sites</option>';
	}

	form.addEventListener("submit", async (e) => {
		e.preventDefault();

		const formData = new FormData(form);
		const siteId = formData.get("siteId");
		const jsPath = formData.get("jsPath");
		const beaconPath = formData.get("beaconPath");

		if (!siteId) {
			alert("Please select a site");
			return;
		}

		// Get selected site info
		const selectedOption = siteSelect.options[siteSelect.selectedIndex];
		const siteDomain = selectedOption.textContent.match(/\(([^)]+)\)/)[1];

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
    rewrite /${jsPathClean} /js/${jsPathClean}?siteId=${siteId}
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
}`;

		// Generate tracking code
		const trackingCode = `<script>
  (function() {
    var script = document.createElement('script');
    script.async = true;
    script.src = '/${jsPathClean}';
    script.setAttribute('data-site-id', '${siteId}');
    script.setAttribute('data-beacon-url', '/${beaconPathClean}');
    document.head.appendChild(script);
  })();
</script>`;

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
