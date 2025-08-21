document.addEventListener("DOMContentLoaded", async () => {
	const codeElement = document.getElementById("tracking-code");
	const noscriptCheckbox = document.getElementById("noscript-tracking");
	const tagManagerCheckbox = document.getElementById("tag-manager");

	// Get the selected site using SiteManager
	const siteData = await window.SiteManager.getCurrentSiteData();
	
	if (!siteData.site) {
		// No site selected, try to initialize
		const site = await window.SiteManager.initialize();
		if (!site) {
			codeElement.textContent = '// Please select a site first';
			return;
		}
		siteData.siteId = site.id;
	}
	
	const siteId = siteData.siteId;
	window.currentSiteId = siteId; // Store for verify function
	window.currentSiteDomain = siteData.site?.domain;

	function updateTrackingCode() {
		const includeNoscript = noscriptCheckbox.checked;
		const isTagManager = tagManagerCheckbox.checked;

		const apiBase =
			window.location.hostname === "localhost"
				? "http://localhost:3000"
				: window.location.origin;

		let code = "";

		if (isTagManager) {
			// Tag Manager format (for GTM, Segment, etc.)
			code = `// Slimlytics Tag Manager Code
(function() {
  var script = document.createElement('script');
  script.async = true;
  script.setAttribute('data-id', '${siteId}');
  script.src = '${apiBase}/sa.js';
  document.head.appendChild(script);
})();`;
		} else {
			// Standard HTML format (simplified like Clicky)
			code = `<script async data-id="${siteId}" src="${apiBase}/sa.js"></script>`;

			if (includeNoscript) {
				code += `\n<noscript><p><img alt="Slimlytics" width="1" height="1" src="${apiBase}/${siteId}ns.gif" /></p></noscript>`;
			}
		}

		codeElement.textContent = code;
	}

	// Update code when checkboxes change
	noscriptCheckbox.addEventListener("change", updateTrackingCode);
	tagManagerCheckbox.addEventListener("change", updateTrackingCode);

	// Initial code generation
	updateTrackingCode();
});

function copyCode() {
	const codeElement = document.getElementById("tracking-code");
	const copyBtn = document.querySelector(".copy-btn");

	// Create a temporary textarea to copy from
	const textarea = document.createElement("textarea");
	textarea.value = codeElement.textContent;
	textarea.style.position = "absolute";
	textarea.style.left = "-9999px";
	document.body.appendChild(textarea);

	// Select and copy
	textarea.select();
	document.execCommand("copy");
	document.body.removeChild(textarea);

	// Update button text
	copyBtn.textContent = "Copied!";
	copyBtn.classList.add("copied");

	// Reset after 2 seconds
	setTimeout(() => {
		copyBtn.textContent = "Copy";
		copyBtn.classList.remove("copied");
	}, 2000);
}

async function verifyTrackingCode() {
	if (!window.currentSiteDomain) {
		alert("Please select a site first");
		return;
	}
	
	const btn = event.target;
	const originalText = btn.textContent;
	btn.textContent = "⏳ Verifying...";
	btn.disabled = true;
	
	try {
		// Construct the URL to check
		let url = window.currentSiteDomain;
		if (!url.startsWith("http://") && !url.startsWith("https://")) {
			url = "https://" + url;
		}
		
		// Try to fetch the site and check for tracking script
		const response = await fetch(window.SLIMLYTICS_CONFIG.apiEndpoint(`/api/verify-tracking?url=${encodeURIComponent(url)}&siteId=${window.currentSiteId}`))
			.catch(() => null);
		
		if (!response) {
			// If API endpoint doesn't exist, do a simple check
			alert(`⚠️ Please manually verify that the tracking code is installed on ${window.currentSiteDomain}\n\nLook for:\n- The script tag with data-id="${window.currentSiteId}"\n- Network requests to /sa.js or /track`);
		} else if (response.ok) {
			const result = await response.json();
			if (result.verified) {
				alert(`✅ Tracking code verified!\n\nThe tracking script is properly installed on ${window.currentSiteDomain}`);
			} else {
				alert(`❌ Tracking code not found\n\nCould not find the tracking script on ${window.currentSiteDomain}\n\nPlease ensure:\n1. The tracking code is added to your site\n2. It's placed before the closing </body> tag\n3. The site is publicly accessible`);
			}
		} else {
			alert(`⚠️ Could not verify tracking code\n\nUnable to check ${window.currentSiteDomain}\n\nThis might be because:\n- The site is not publicly accessible\n- The domain is incorrect\n- CORS restrictions prevent checking\n\nPlease manually verify the installation.`);
		}
	} catch (error) {
		console.error("Verification error:", error);
		alert(`⚠️ Verification failed\n\nPlease manually check that the tracking code is installed on your site.`);
	} finally {
		btn.textContent = originalText;
		btn.disabled = false;
	}
}
