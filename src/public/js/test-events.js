// Sample data for generating random events
const pages = [
	"/",
	"/about",
	"/products",
	"/services",
	"/contact",
	"/blog",
	"/blog/post-1",
	"/blog/post-2",
	"/pricing",
	"/features",
	"/docs",
	"/api",
	"/help",
	"/login",
	"/signup",
];

const referrers = [
	"",
	"https://google.com",
	"https://facebook.com",
	"https://twitter.com",
	"https://linkedin.com",
	"https://reddit.com",
	"https://github.com",
	"https://stackoverflow.com",
	"https://news.ycombinator.com",
];

const userAgents = [
	"Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.124 Safari/537.36",
	"Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/14.1.1 Safari/605.1.15",
	"Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:89.0) Gecko/20100101 Firefox/89.0",
	"Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.124 Safari/537.36",
	"Mozilla/5.0 (iPhone; CPU iPhone OS 14_6 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/14.0 Mobile/15E148 Safari/604.1",
	"Mozilla/5.0 (iPad; CPU OS 14_6 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) CriOS/91.0.4472.80 Mobile/15E148 Safari/604.1",
];

const screenResolutions = [
	"1920x1080",
	"1366x768",
	"1440x900",
	"1536x864",
	"1280x720",
	"1600x900",
	"2560x1440",
	"3840x2160",
	"375x667",
	"414x896",
];

const languages = [
	"en-US",
	"en-GB",
	"es-ES",
	"fr-FR",
	"de-DE",
	"ja-JP",
	"zh-CN",
	"pt-BR",
	"ru-RU",
	"it-IT",
];

let eventCount = 0;

// Initialize on page load
async function initTestEvents() {
	// Nothing to load since site is managed by the header
	updateStats();
}

// Generate random event data
async function generateRandomEvent(siteId) {
	const currentSite = await window.SiteManager.getSelectedSite();
	const domain = currentSite ? currentSite.domain : "example.com";

	return {
		site_id: siteId,
		page_url: `https://${domain}${pages[Math.floor(Math.random() * pages.length)]}`,
		referrer: referrers[Math.floor(Math.random() * referrers.length)],
		user_agent: userAgents[Math.floor(Math.random() * userAgents.length)],
		screen_resolution:
			screenResolutions[Math.floor(Math.random() * screenResolutions.length)],
		language: languages[Math.floor(Math.random() * languages.length)],
		timestamp: new Date().toISOString(),
	};
}

// Send event to tracking endpoint
async function sendEvent(eventData) {
	try {
		const response = await fetch(
			window.SLIMLYTICS_CONFIG.apiEndpoint("/track"),
			{
				method: "POST",
				headers: {
					"Content-Type": "application/json",
				},
				body: JSON.stringify(eventData),
			},
		);

		if (response.ok) {
			logEvent(eventData, true);
			eventCount++;
			updateStats();
		} else {
			logEvent(eventData, false);
		}

		return response.ok;
	} catch (error) {
		console.error("Failed to send event:", error);
		logEvent(eventData, false);
		return false;
	}
}

// Send random page views
async function sendRandomPageViews() {
	const currentSite = await window.SiteManager.getSelectedSite();
	if (!currentSite) {
		showStatus("Please select a site first", "error");
		return;
	}
	const siteId = currentSite.id;

	const count = Number.parseInt(document.getElementById("event-count").value);
	const delay = Number.parseInt(document.getElementById("delay-ms").value);

	showStatus(`Sending ${count} random page views...`, "success");

	let successCount = 0;
	for (let i = 0; i < count; i++) {
		const eventData = await generateRandomEvent(siteId);
		const success = await sendEvent(eventData);
		if (success) successCount++;

		if (delay > 0 && i < count - 1) {
			await new Promise((resolve) => setTimeout(resolve, delay));
		}
	}

	showStatus(
		`Sent ${successCount} of ${count} events successfully`,
		successCount === count ? "success" : "error",
	);
}

// Send burst of events
async function sendBurstEvents() {
	const currentSite = await window.SiteManager.getSelectedSite();
	if (!currentSite) {
		showStatus("Please select a site first", "error");
		return;
	}
	const siteId = currentSite.id;

	showStatus("Sending burst of 50 events...", "success");

	const promises = [];
	for (let i = 0; i < 50; i++) {
		const eventData = await generateRandomEvent(siteId);
		promises.push(sendEvent(eventData));
	}

	const results = await Promise.all(promises);
	const successCount = results.filter((r) => r).length;

	showStatus(
		`Burst complete: ${successCount} of 50 events sent successfully`,
		successCount === 50 ? "success" : "error",
	);
}

// Send custom events
async function sendCustomEvents() {
	const currentSite = await window.SiteManager.getSelectedSite();
	if (!currentSite) {
		showStatus("Please select a site first", "error");
		return;
	}
	const siteId = currentSite.id;

	const count = Number.parseInt(document.getElementById("event-count").value);
	const delay = Number.parseInt(document.getElementById("delay-ms").value);

	showStatus(`Sending ${count} custom events...`, "success");

	const customEventNames = ["click", "signup", "purchase", "download", "share"];

	let successCount = 0;
	for (let i = 0; i < count; i++) {
		const baseEvent = await generateRandomEvent(siteId);
		const eventData = {
			...baseEvent,
			event_name:
				customEventNames[Math.floor(Math.random() * customEventNames.length)],
			event_data: {
				value: Math.floor(Math.random() * 1000),
				category: ["product", "blog", "feature"][Math.floor(Math.random() * 3)],
			},
		};

		const success = await sendEvent(eventData);
		if (success) successCount++;

		if (delay > 0 && i < count - 1) {
			await new Promise((resolve) => setTimeout(resolve, delay));
		}
	}

	showStatus(
		`Sent ${successCount} of ${count} custom events successfully`,
		successCount === count ? "success" : "error",
	);
}

// Log event to the display
function logEvent(eventData, success) {
	const logContent = document.getElementById("event-log-content");
	const logItem = document.createElement("div");
	logItem.className = "event-log-item";

	const time = new Date().toLocaleTimeString();
	const url = new URL(eventData.page_url).pathname;
	const status = success ? "✅" : "❌";

	logItem.innerHTML = `${status} [${time}] ${eventData.site_id} - ${url}`;

	logContent.insertBefore(logItem, logContent.firstChild);

	// Keep only last 50 entries
	while (logContent.children.length > 50) {
		logContent.removeChild(logContent.lastChild);
	}
}

// Clear event log
function clearEventLog() {
	document.getElementById("event-log-content").innerHTML = "";
	eventCount = 0;
	updateStats();
	showStatus("Event log cleared", "success");
}

// Show status message
function showStatus(message, type) {
	const statusEl = document.getElementById("status-message");
	statusEl.textContent = message;
	statusEl.className = `status-message ${type}`;

	setTimeout(() => {
		statusEl.className = "status-message";
	}, 5000);
}

// Update stats display
function updateStats() {
	const statsContent = document.getElementById("stats-content");
	statsContent.innerHTML = `
        <p><strong>Total Events Sent:</strong> ${eventCount}</p>
        <p><strong>Last Event:</strong> ${new Date().toLocaleTimeString()}</p>
        <p><em>Note: Dashboard stats may take a moment to update</em></p>
    `;
}

// Initialize on page load
document.addEventListener("DOMContentLoaded", initTestEvents);
