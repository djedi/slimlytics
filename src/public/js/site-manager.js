// Global site management - handles site selection persistence across pages
window.SiteManager = (() => {
	const STORAGE_KEY = "slimlytics_selected_site";

	// Get the currently selected site from localStorage
	function getSelectedSite() {
		const stored = localStorage.getItem(STORAGE_KEY);
		if (stored) {
			try {
				return JSON.parse(stored);
			} catch (e) {
				console.error("Failed to parse stored site:", e);
				localStorage.removeItem(STORAGE_KEY);
			}
		}
		return null;
	}

	// Set the selected site in localStorage
	function setSelectedSite(site) {
		if (site?.id) {
			localStorage.setItem(
				STORAGE_KEY,
				JSON.stringify({
					id: site.id,
					name: site.name,
					domain: site.domain,
				}),
			);

			// Dispatch custom event for other components to listen to
			window.dispatchEvent(new CustomEvent("site-changed", { detail: site }));
		}
	}

	// Get all sites from API
	async function getAllSites() {
		try {
			const apiEndpoint = window.SLIMLYTICS_CONFIG
				? window.SLIMLYTICS_CONFIG.apiEndpoint("/api/sites")
				: "/api/sites";

			const response = await fetch(apiEndpoint);

			// Check if response is OK and is JSON
			if (!response.ok) {
				console.error("API request failed with status:", response.status);
				return [];
			}

			const contentType = response.headers.get("content-type");
			if (!contentType || !contentType.includes("application/json")) {
				console.error("API returned non-JSON response:", contentType);
				return [];
			}

			return await response.json();
		} catch (error) {
			console.error("Failed to load sites:", error);
			return [];
		}
	}

	// Initialize site selection (auto-select if only one site or restore previous selection)
	async function initialize() {
		const sites = await getAllSites();

		if (sites.length === 0) {
			return null;
		}

		// Check if we have a stored selection
		const stored = getSelectedSite();
		if (stored && sites.find((s) => s.id === stored.id)) {
			return stored;
		}

		// If only one site, auto-select it
		if (sites.length === 1) {
			setSelectedSite(sites[0]);
			return sites[0];
		}

		// Otherwise return null (user needs to select)
		return null;
	}

	// Ensure a site is selected, redirect to site selection if needed
	async function ensureSelectedSite() {
		const selected = getSelectedSite();
		if (selected) {
			return selected;
		}

		// Try to auto-select if possible
		const autoSelected = await initialize();
		if (autoSelected) {
			return autoSelected;
		}

		// No site selected and can't auto-select
		// You might want to redirect to a site selection page
		return null;
	}

	// Get site-specific data for current page
	async function getCurrentSiteData() {
		const site = await ensureSelectedSite();
		if (!site) {
			return { site: null, siteId: null };
		}

		return {
			site: site,
			siteId: site.id,
			siteName: site.name,
			siteDomain: site.domain,
		};
	}

	// Public API
	return {
		getSelectedSite,
		setSelectedSite,
		getAllSites,
		initialize,
		ensureSelectedSite,
		getCurrentSiteData,
	};
})();
