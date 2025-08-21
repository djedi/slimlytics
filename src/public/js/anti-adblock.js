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
		
		// Use site ID as the base for paths - much harder to detect/block
		const jsPathClean = `${siteId}.js`;
		const beaconPathClean = siteId;

		// Get the API base URL (assuming it's the same server in production)
		const apiBase =
			window.location.hostname === "localhost"
				? "http://localhost:3000"
				: window.location.origin;

		// Extract hostname from apiBase for the Host header
		const apiHost = new URL(apiBase).hostname;
		
		// Generate Caddy configuration
		const caddyConfig = `### SLIM ANALYTICS ANTI-ADBLOCK PROXY - ${siteDomain}
### COPY INTO YOUR WEBSITE'S CADDYFILE

# Anti-adblock tracking using site ID: ${siteId}
# This configuration makes tracking undetectable by using your site ID as the path

# TRACKING SCRIPT
handle /${jsPathClean} {
    rewrite * /sa.js
    reverse_proxy ${apiBase} {
        header_up Host ${apiHost}
    }
}

# BEACON ENDPOINT  
handle /${beaconPathClean} {
    rewrite * /track
    reverse_proxy ${apiBase} {
        header_up Host ${apiHost}
    }
}

# NOSCRIPT GIF BEACON
handle /${siteId}ns.gif {
    rewrite * /t/${siteId}ns.gif
    reverse_proxy ${apiBase} {
        header_up Host ${apiHost}
    }
}`;

		// Generate tracking code (simplified like Clicky)
		const trackingCode = `<script async data-id="${siteId}" src="/${jsPathClean}"></script>
<noscript><p><img alt="Slimlytics" width="1" height="1" src="/${siteId}ns.gif" /></p></noscript>`;

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
