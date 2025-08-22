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
	let lastTrackedUrl = window.location.href;
	let pendingTrack = null;

	// Function to track if URL has changed
	function trackIfChanged() {
		const currentUrl = window.location.href;
		
		// Clear any pending track
		if (pendingTrack) {
			clearTimeout(pendingTrack);
			pendingTrack = null;
		}
		
		// Only track if URL actually changed
		if (currentUrl !== lastTrackedUrl) {
			// Debounce to prevent multiple events
			pendingTrack = setTimeout(function() {
				lastTrackedUrl = currentUrl;
				track();
				pendingTrack = null;
			}, 100);
		}
	}

	// Listen for history changes (covers pushState, replaceState, and back/forward)
	const originalPushState = history.pushState;
	const originalReplaceState = history.replaceState;

	history.pushState = function () {
		originalPushState.apply(history, arguments);
		trackIfChanged();
	};

	history.replaceState = function () {
		originalReplaceState.apply(history, arguments);
		trackIfChanged();
	};

	window.addEventListener("popstate", trackIfChanged);
	
	// Fallback for SPAs that might not use History API
	// Check less frequently to avoid conflicts
	setInterval(trackIfChanged, 2000);

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
