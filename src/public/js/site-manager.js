// Global site management - handles site selection persistence across pages
window.SiteManager = (() => {
	const STORAGE_KEY = "slimlytics_selected_site";

	// Get the currently selected site from localStorage
	function getSelectedSite() {
		console.log('[SiteManager] Getting selected site from localStorage');
		const stored = localStorage.getItem(STORAGE_KEY);
		console.log('[SiteManager] Stored value:', stored);
		if (stored) {
			try {
				const parsed = JSON.parse(stored);
				console.log('[SiteManager] Parsed site:', parsed);
				return parsed;
			} catch (e) {
				console.error('[SiteManager] Failed to parse stored site:', e);
				localStorage.removeItem(STORAGE_KEY);
			}
		}
		console.log('[SiteManager] No stored site found');
		return null;
	}

	// Set the selected site in localStorage
	function setSelectedSite(site) {
		console.log('[SiteManager] Setting selected site:', site);
		if (site?.id) {
			const toStore = {
				id: site.id,
				name: site.name,
				domain: site.domain,
			};
			console.log('[SiteManager] Storing in localStorage:', toStore);
			localStorage.setItem(
				STORAGE_KEY,
				JSON.stringify(toStore),
			);

			// Dispatch custom event for other components to listen to
			window.dispatchEvent(new CustomEvent("site-changed", { detail: site }));
		} else {
			console.warn('[SiteManager] Invalid site object, not storing:', site);
		}
	}

	// Get all sites from API
	async function getAllSites() {
		console.log('[SiteManager] Getting all sites from API');
		try {
			const apiEndpoint = window.SLIMLYTICS_CONFIG
				? window.SLIMLYTICS_CONFIG.apiEndpoint("/api/sites")
				: "/api/sites";
			console.log('[SiteManager] API endpoint:', apiEndpoint);

			// Use cached fetch if available
			const response = window.APICache 
				? await window.APICache.fetch(apiEndpoint)
				: await fetch(apiEndpoint);
			console.log('[SiteManager] Response status:', response.status);

			// Check if response is OK and is JSON
			if (!response.ok) {
				console.error('[SiteManager] API request failed with status:', response.status);
				try {
					const errorText = await response.text();
					console.error('[SiteManager] Error response body:', errorText);
				} catch (e) {
					console.error('[SiteManager] Could not read error response');
				}
				return [];
			}

			const contentType = response.headers.get("content-type");
			console.log('[SiteManager] Response content-type:', contentType);
			if (!contentType || !contentType.includes("application/json")) {
				console.error('[SiteManager] API returned non-JSON response:', contentType);
				try {
					const bodyText = await response.text();
					console.error('[SiteManager] Response body:', bodyText.substring(0, 500));
				} catch (e) {
					console.error('[SiteManager] Could not read response body');
				}
				return [];
			}

			const sites = await response.json();
			console.log('[SiteManager] Sites received:', sites);
			return sites;
		} catch (error) {
			console.error('[SiteManager] Failed to load sites:', error);
			console.error('[SiteManager] Error details:', error.message, error.stack);
			return [];
		}
	}

	// Initialize site selection (auto-select if only one site or restore previous selection)
	async function initialize() {
		console.log('[SiteManager] Initializing...');
		const sites = await getAllSites();
		console.log('[SiteManager] Sites available:', sites.length);

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
