/**
 * Slimlytics Tracking Script
 * Lightweight analytics tracking for websites
 */

// biome-ignore lint/complexity/useArrowFunction: <explanation>
(function () {
	"use strict";

	// Get the site ID from the script tag
	const script = document.currentScript;
	const siteId = script ? script.dataset.site : null;

	if (!siteId) {
		console.error("Slimlytics: No site ID provided");
		return;
	}

	// Configuration
	const API_ENDPOINT =
		(script.src.includes("localhost")
			? "http://localhost:3000"
			: window.location.origin) + "/track";

	// Get page data
	function getPageData() {
		return {
			site_id: siteId,
			page_url: window.location.href,
			referrer: document.referrer || null,
			user_agent: navigator.userAgent,
			screen_resolution: `${window.screen.width}x${window.screen.height}`,
			language: navigator.language || navigator.userLanguage,
			timestamp: new Date().toISOString(),
		};
	}

	// Send tracking data
	function track() {
		const data = getPageData();

		// Use sendBeacon if available for better reliability
		if (navigator.sendBeacon) {
			const blob = new Blob([JSON.stringify(data)], {
				type: "application/json",
			});
			navigator.sendBeacon(API_ENDPOINT, blob);
		} else {
			// Fallback to fetch
			fetch(API_ENDPOINT, {
				method: "POST",
				headers: {
					"Content-Type": "application/json",
				},
				body: JSON.stringify(data),
				keepalive: true,
			}).catch(function (error) {
				console.error("Slimlytics tracking error:", error);
			});
		}
	}

	// Track page view immediately
	track();

	// Track when page becomes visible (for prerendered pages)
	if (document.visibilityState === "prerender") {
		document.addEventListener(
			"visibilitychange",
			function () {
				if (document.visibilityState === "visible") {
					track();
				}
			},
			{ once: true },
		);
	}

	// Track single page application navigation
	let lastPath = window.location.pathname;
	let lastFullUrl = window.location.href;
	let lastTrackTime = Date.now();
	const TRACK_DEBOUNCE_MS = 500; // Prevent duplicate tracking within 500ms

	// Debounced track function to prevent duplicates
	function trackDebounced() {
		const now = Date.now();
		const currentPath = window.location.pathname;
		const currentUrl = window.location.href;
		
		// Only track if enough time has passed AND the URL is actually different
		if (now - lastTrackTime > TRACK_DEBOUNCE_MS && currentUrl !== lastFullUrl) {
			lastPath = currentPath;
			lastFullUrl = currentUrl;
			lastTrackTime = now;
			track();
		}
	}

	// Check for route changes periodically (for SPAs that don't use History API)
	setInterval(function () {
		if (window.location.pathname !== lastPath) {
			trackDebounced();
		}
	}, 1000);

	// Also listen for history changes
	const originalPushState = history.pushState;
	const originalReplaceState = history.replaceState;

	history.pushState = function () {
		originalPushState.apply(history, arguments);
		setTimeout(trackDebounced, 10);
	};

	history.replaceState = function () {
		originalReplaceState.apply(history, arguments);
		setTimeout(trackDebounced, 10);
	};

	window.addEventListener("popstate", trackDebounced);

	// Expose tracking function globally for manual tracking
	window.slimlytics = {
		track: track,
		trackEvent: function (eventName, eventData) {
			const data = Object.assign(getPageData(), {
				event_name: eventName,
				event_data: eventData,
			});

			if (navigator.sendBeacon) {
				const blob = new Blob([JSON.stringify(data)], {
					type: "application/json",
				});
				navigator.sendBeacon(API_ENDPOINT, blob);
			} else {
				fetch(API_ENDPOINT, {
					method: "POST",
					headers: {
						"Content-Type": "application/json",
					},
					body: JSON.stringify(data),
					keepalive: true,
				}).catch(function (error) {
					console.error("Slimlytics event tracking error:", error);
				});
			}
		},
	};
})();
