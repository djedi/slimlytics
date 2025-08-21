import { Hono } from "hono";
import { cors } from "hono/cors";
import { basicAuth } from "hono/basic-auth";
import { serveStatic } from "hono/bun";
import { Database } from "bun:sqlite";
import { trackEvent, getStats } from "../db/queries.js";
import { sitesRoutes } from "./sites.ts";
import { statsRoutes } from "./stats.ts";
import { setupTrackingScriptRoute } from "./routes/tracking-script.js";
import { setupSimplifiedTrackingRoutes } from "./routes/simplified-tracking.js";
import geoip from "../../api/services/geoip.js";

const app = new Hono();
const db = new Database("./data/analytics.db");

// Initialize GeoIP service
await geoip.initialize();

// WebSocket clients tracked by site ID
const wsClients = new Map();

// Check if auth is configured
const AUTH_USERNAME = process.env.AUTH_USERNAME;
const AUTH_PASSWORD = process.env.AUTH_PASSWORD;
const isAuthEnabled = AUTH_USERNAME && AUTH_PASSWORD;

// Middleware for protected routes
const authMiddleware = isAuthEnabled
	? basicAuth({
			username: AUTH_USERNAME,
			password: AUTH_PASSWORD,
	  })
	: async (c, next) => await next(); // No-op middleware if auth not configured

app.use("*", cors());

// Health check endpoint
app.get("/health", (c) => {
	return c.json({
		status: "healthy",
		timestamp: new Date().toISOString(),
	});
});

// Setup tracking script routes
setupTrackingScriptRoute(app);
setupSimplifiedTrackingRoutes(app);

// Sites API routes (protected)
app.get("/api/sites", authMiddleware, async (c) => {
	return sitesRoutes.getAllSites(c.req.raw);
});

app.post("/api/sites", authMiddleware, async (c) => {
	return sitesRoutes.createSite(c.req.raw);
});

app.put("/api/sites/:id", authMiddleware, async (c) => {
	const { id } = c.req.param();
	return sitesRoutes.updateSite(c.req.raw, id);
});

app.delete("/api/sites/:id", authMiddleware, async (c) => {
	const { id } = c.req.param();
	return sitesRoutes.deleteSite(c.req.raw, id);
});

app.post("/track", async (c) => {
	let body;

	// Handle both JSON and text/plain content types (for sendBeacon compatibility)
	const contentType = c.req.header("content-type");
	if (contentType?.includes("text/plain")) {
		const text = await c.req.text();
		body = JSON.parse(text);
	} else {
		body = await c.req.json();
	}

	const {
		siteId,
		site_id,
		url,
		page_url,
		referrer,
		userAgent,
		user_agent,
		screenWidth,
		screenHeight,
		screen_resolution,
		language,
		visitorId,
		sessionId,
		eventType,
		eventData,
	} = body;

	// Handle both new and old field names for compatibility
	const finalSiteId = siteId || site_id;
	const finalUrl = url || page_url;
	const finalUserAgent = userAgent || user_agent;
	const finalScreenResolution =
		screen_resolution ||
		(screenWidth && screenHeight ? `${screenWidth}x${screenHeight}` : null);

	const ip =
		c.req.header("x-forwarded-for") || c.req.header("x-real-ip") || "unknown";

	// Perform GeoIP lookup
	const geoData = await geoip.lookup(ip);

	trackEvent(db, {
		site_id: finalSiteId,
		page_url: finalUrl,
		referrer: referrer || null,
		user_agent: finalUserAgent,
		ip_hash: hashIP(ip),
		screen_resolution: finalScreenResolution,
		language,
		timestamp: new Date().toISOString(),
		visitor_id: visitorId,
		session_id: sessionId,
		event_type: eventType || "pageview",
		event_data: eventData ? JSON.stringify(eventData) : null,
		// Add geo data
		country: geoData.country,
		country_code: geoData.countryCode,
		region: geoData.region,
		city: geoData.city,
		latitude: geoData.latitude,
		longitude: geoData.longitude,
		timezone: geoData.timezone,
		asn: geoData.asn,
		asn_org: geoData.asnOrg,
	});

	// Broadcast update to WebSocket clients watching this site
	broadcastStatsUpdate(finalSiteId);

	// Return a 1x1 transparent pixel for beacon compatibility
	if (c.req.header("accept")?.includes("image")) {
		const pixel = Buffer.from(
			"R0lGODlhAQABAIAAAP///wAAACH5BAEAAAAALAAAAAABAAEAAAICRAEAOw==",
			"base64",
		);
		return new Response(pixel, {
			headers: {
				"Content-Type": "image/gif",
				"Cache-Control": "no-cache, no-store, must-revalidate",
				Pragma: "no-cache",
			},
		});
	}

	return c.json({ success: true });
});

// Stats API routes (protected)
app.get("/api/stats/:siteId", authMiddleware, async (c) => {
	const { siteId } = c.req.param();
	return statsRoutes.getStats(c.req.raw, siteId);
});

app.get("/api/stats/:siteId/timeseries", authMiddleware, async (c) => {
	const { siteId } = c.req.param();
	return statsRoutes.getTimeSeries(c.req.raw, siteId);
});

app.get("/api/stats/:siteId/realtime", authMiddleware, async (c) => {
	const { siteId } = c.req.param();
	return statsRoutes.getRealtime(c.req.raw, siteId);
});

app.get("/api/stats/:siteId/recent-visitors", authMiddleware, async (c) => {
	const { siteId } = c.req.param();
	return statsRoutes.getRecentVisitorsEndpoint(c.req.raw, siteId);
});

// Clear analytics data for a site (protected)
app.delete("/api/stats/:siteId/data", authMiddleware, async (c) => {
	const { siteId } = c.req.param();
	const { range } = await c.req.json().catch(() => ({ range: 'all' }));
	
	try {
		let deleteQuery;
		const params = [siteId];
		
		switch (range) {
			case '7days':
				deleteQuery = `DELETE FROM events WHERE site_id = ? AND timestamp > datetime('now', '-7 days')`;
				break;
			case '30days':
				deleteQuery = `DELETE FROM events WHERE site_id = ? AND timestamp > datetime('now', '-30 days')`;
				break;
			case 'today':
				deleteQuery = `DELETE FROM events WHERE site_id = ? AND date(timestamp) = date('now')`;
				break;
			case 'all':
			default:
				deleteQuery = `DELETE FROM events WHERE site_id = ?`;
				break;
		}
		
		const stmt = db.prepare(deleteQuery);
		const result = stmt.run(...params);
		
		console.log(`[API] Cleared ${result.changes} events for site ${siteId} (range: ${range})`);
		
		// Broadcast update to WebSocket clients
		broadcastStatsUpdate(siteId);
		
		return c.json({ 
			success: true, 
			deleted: result.changes,
			message: `Successfully cleared ${result.changes} events` 
		});
	} catch (error) {
		console.error('[API] Error clearing data:', error);
		return c.json({ 
			success: false, 
			error: 'Failed to clear analytics data' 
		}, 500);
	}
});

// Serve static files from dist directory (protected) - MUST be after API routes
app.use("/*", authMiddleware, serveStatic({ root: "./dist" }));

function hashIP(ip) {
	const crypto = require("node:crypto");
	return crypto
		.createHash("sha256")
		.update(ip + process.env.SALT || "default-salt")
		.digest("hex")
		.substring(0, 16);
}

// Broadcast stats update to all WebSocket clients watching a specific site
async function broadcastStatsUpdate(siteId) {
	const clients = wsClients.get(siteId);
	if (!clients || clients.size === 0) return;

	try {
		// Get the latest stats for this site
		const dateRange = {
			start: new Date(new Date().setHours(0, 0, 0, 0)).toISOString(),
			end: new Date().toISOString(),
		};

		// Fetch stats (you'll need to import or implement getStatsForSite)
		const stats = await statsRoutes.getStatsData(
			siteId,
			dateRange.start,
			dateRange.end,
		);

		const message = JSON.stringify({
			type: "stats-update",
			siteId,
			stats,
			timestamp: new Date().toISOString(),
		});

		// Send to all connected clients for this site
		for (const ws of clients) {
			if (ws.readyState === 1) {
				// WebSocket.OPEN
				ws.send(message);
			}
		}
	} catch (error) {
		console.error("Error broadcasting stats update:", error);
	}
}

const port = process.env.PORT || 3000;
const host = process.env.HOST || "0.0.0.0";
console.log(`Analytics API running on ${host}:${port} with WebSocket support`);
if (isAuthEnabled) {
	console.log(`Basic authentication is ENABLED (username: ${AUTH_USERNAME})`);
} else {
	console.log(`Basic authentication is DISABLED`);
}

Bun.serve({
	port,
	hostname: host,
	fetch(req, server) {
		// Try to upgrade to WebSocket
		if (req.headers.get("upgrade") === "websocket") {
			// For WebSockets, we'll check auth in the first message instead
			// since browsers can't send custom headers with WebSocket connections
			if (server.upgrade(req)) {
				return; // WebSocket upgrade successful
			}
		}
		// Otherwise handle as normal HTTP request
		return app.fetch(req, server);
	},
	websocket: {
		open(ws) {
			console.log("[WebSocket] Client connected from:", ws.remoteAddress);
			ws.isAlive = true;
		},

		close(ws, code, reason) {
			console.log("[WebSocket] Client disconnected. Code:", code, "Reason:", reason);

			// Remove from all subscriber lists
			if (ws.siteId && wsClients.has(ws.siteId)) {
				wsClients.get(ws.siteId).delete(ws);
				console.log(`[WebSocket] Removed client from site ${ws.siteId} subscribers`);

				// Clean up empty sets
				if (wsClients.get(ws.siteId).size === 0) {
					wsClients.delete(ws.siteId);
					console.log(`[WebSocket] No more subscribers for site ${ws.siteId}, cleaned up`);
				}
			}
		},

		error(ws, error) {
			console.error("[WebSocket] Error:", error);
			console.error("[WebSocket] Error details:", error.message, error.stack);
		},

		message(ws, message) {
			// Handle messages
			console.log("[WebSocket] Message received:", message);
			try {
				const data = JSON.parse(message);
				console.log("[WebSocket] Parsed message type:", data.type);

				if (data.type === "subscribe" && data.siteId) {
					// For WebSockets, we'll skip auth check since the dashboard itself is already protected
					// The user had to authenticate to load the dashboard page that opens this WebSocket
					// This avoids complexity with passing auth credentials through WebSocket
					
					console.log(`[WebSocket] Subscribing client to site ${data.siteId}`);
					// Add client to the site's subscriber list
					if (!wsClients.has(data.siteId)) {
						wsClients.set(data.siteId, new Set());
					}
					wsClients.get(data.siteId).add(ws);
					ws.siteId = data.siteId;
					console.log(`[WebSocket] Client subscribed. Total subscribers for ${data.siteId}: ${wsClients.get(data.siteId).size}`);

					// Send initial stats
					broadcastStatsUpdate(data.siteId);

					ws.send(
						JSON.stringify({
							type: "subscribed",
							siteId: data.siteId,
							message: "Successfully subscribed to real-time updates",
						}),
					);
				}

				// Handle ping/pong for connection health
				if (data.type === "ping") {
					console.log("[WebSocket] Ping received, sending pong");
					ws.send(JSON.stringify({ type: "pong" }));
				}
			} catch (error) {
				console.error("[WebSocket] Message error:", error);
				console.error("[WebSocket] Raw message was:", message);
			}
		},
	},
});
