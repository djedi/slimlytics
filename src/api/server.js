import { Hono } from 'hono';
import { cors } from 'hono/cors';
import { serveStatic } from 'hono/bun';
import { Database } from 'bun:sqlite';
import { trackEvent, getStats } from '../db/queries.js';
import { sitesRoutes } from './sites.ts';
import { statsRoutes } from './stats.ts';
import geoip from '../../api/services/geoip.js';

const app = new Hono();
const db = new Database('./data/analytics.db');

// Initialize GeoIP service
await geoip.initialize();

// WebSocket clients tracked by site ID
const wsClients = new Map();

app.use('*', cors());

// Sites API routes
app.get('/api/sites', async (c) => {
  return sitesRoutes.getAllSites(c.req.raw);
});

app.post('/api/sites', async (c) => {
  return sitesRoutes.createSite(c.req.raw);
});

app.put('/api/sites/:id', async (c) => {
  const { id } = c.req.param();
  return sitesRoutes.updateSite(c.req.raw, id);
});

app.delete('/api/sites/:id', async (c) => {
  const { id } = c.req.param();
  return sitesRoutes.deleteSite(c.req.raw, id);
});

app.post('/track', async (c) => {
  const body = await c.req.json();
  const { 
    site_id, 
    page_url, 
    referrer, 
    user_agent,
    screen_resolution,
    language
  } = body;

  const ip = c.req.header('x-forwarded-for') || c.req.header('x-real-ip') || 'unknown';
  
  // Perform GeoIP lookup
  const geoData = await geoip.lookup(ip);
  
  trackEvent(db, {
    site_id,
    page_url,
    referrer: referrer || null,
    user_agent,
    ip_hash: hashIP(ip),
    screen_resolution,
    language,
    timestamp: new Date().toISOString(),
    // Add geo data
    country: geoData.country,
    country_code: geoData.countryCode,
    region: geoData.region,
    city: geoData.city,
    latitude: geoData.latitude,
    longitude: geoData.longitude,
    timezone: geoData.timezone,
    asn: geoData.asn,
    asn_org: geoData.asnOrg
  });

  // Broadcast update to WebSocket clients watching this site
  broadcastStatsUpdate(site_id);

  return c.json({ success: true });
});

// Stats API routes
app.get('/api/stats/:siteId', async (c) => {
  const { siteId } = c.req.param();
  return statsRoutes.getStats(c.req.raw, siteId);
});

app.get('/api/stats/:siteId/timeseries', async (c) => {
  const { siteId } = c.req.param();
  return statsRoutes.getTimeSeries(c.req.raw, siteId);
});

app.get('/api/stats/:siteId/realtime', async (c) => {
  const { siteId } = c.req.param();
  return statsRoutes.getRealtime(c.req.raw, siteId);
});

// Serve static files from dist directory - MUST be after API routes
app.use('/*', serveStatic({ root: './dist' }));

function hashIP(ip) {
  const crypto = require('crypto');
  return crypto.createHash('sha256').update(ip + process.env.SALT || 'default-salt').digest('hex').substring(0, 16);
}

// Broadcast stats update to all WebSocket clients watching a specific site
async function broadcastStatsUpdate(siteId) {
  const clients = wsClients.get(siteId);
  if (!clients || clients.size === 0) return;

  try {
    // Get the latest stats for this site
    const dateRange = {
      start: new Date(new Date().setHours(0, 0, 0, 0)).toISOString(),
      end: new Date().toISOString()
    };
    
    // Fetch stats (you'll need to import or implement getStatsForSite)
    const stats = await statsRoutes.getStatsData(siteId, dateRange.start, dateRange.end);
    
    const message = JSON.stringify({
      type: 'stats-update',
      siteId,
      stats,
      timestamp: new Date().toISOString()
    });

    // Send to all connected clients for this site
    for (const ws of clients) {
      if (ws.readyState === 1) { // WebSocket.OPEN
        ws.send(message);
      }
    }
  } catch (error) {
    console.error('Error broadcasting stats update:', error);
  }
}

const port = process.env.PORT || 3000;
const host = process.env.HOST || 'localhost';
console.log(`Analytics API running on ${host}:${port} with WebSocket support`);

Bun.serve({
  port,
  hostname: host,
  fetch(req, server) {
    // Try to upgrade to WebSocket
    if (server.upgrade(req)) {
      return; // WebSocket upgrade successful
    }
    // Otherwise handle as normal HTTP request
    return app.fetch(req, server);
  },
  websocket: {
    open(ws) {
      console.log('WebSocket client connected');
      ws.isAlive = true;
    },
    
    close(ws) {
      console.log('WebSocket client disconnected');
      
      // Remove from all subscriber lists
      if (ws.siteId && wsClients.has(ws.siteId)) {
        wsClients.get(ws.siteId).delete(ws);
        
        // Clean up empty sets
        if (wsClients.get(ws.siteId).size === 0) {
          wsClients.delete(ws.siteId);
        }
      }
    },
    
    error(ws, error) {
      console.error('WebSocket error:', error);
    },
    
    message(ws, message) {
      // Handle messages
      try {
        const data = JSON.parse(message);
        
        if (data.type === 'subscribe' && data.siteId) {
          // Add client to the site's subscriber list
          if (!wsClients.has(data.siteId)) {
            wsClients.set(data.siteId, new Set());
          }
          wsClients.get(data.siteId).add(ws);
          ws.siteId = data.siteId;
          
          // Send initial stats
          broadcastStatsUpdate(data.siteId);
          
          ws.send(JSON.stringify({ 
            type: 'subscribed', 
            siteId: data.siteId,
            message: 'Successfully subscribed to real-time updates'
          }));
        }
        
        // Handle ping/pong for connection health
        if (data.type === 'ping') {
          ws.send(JSON.stringify({ type: 'pong' }));
        }
      } catch (error) {
        console.error('WebSocket message error:', error);
      }
    },
  },
});