document.addEventListener("DOMContentLoaded", async () => {
	const codeElement = document.getElementById("tracking-code");
	const noscriptCheckbox = document.getElementById("noscript-tracking");
	const affiliateCheckbox = document.getElementById("affiliate-badge");
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

	function updateTrackingCode() {
		const includeNoscript = noscriptCheckbox.checked;
		const includeAffiliate = affiliateCheckbox.checked;
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

			if (includeAffiliate) {
				code += `\n<!-- Powered by Slimlytics -->`;
			}
		}

		codeElement.textContent = code;
	}

	// Update code when checkboxes change
	noscriptCheckbox.addEventListener("change", updateTrackingCode);
	affiliateCheckbox.addEventListener("change", updateTrackingCode);
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
