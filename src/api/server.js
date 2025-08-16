import { Hono } from 'hono';
import { cors } from 'hono/cors';
import { Database } from 'bun:sqlite';
import { trackEvent, getStats } from '../db/queries.js';
import { sitesRoutes } from './sites.ts';
import { statsRoutes } from './stats.ts';

const app = new Hono();
const db = new Database('./data/analytics.db');

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
  
  trackEvent(db, {
    site_id,
    page_url,
    referrer: referrer || null,
    user_agent,
    ip_hash: hashIP(ip),
    screen_resolution,
    language,
    timestamp: new Date().toISOString()
  });

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

function hashIP(ip) {
  const crypto = require('crypto');
  return crypto.createHash('sha256').update(ip + process.env.SALT || 'default-salt').digest('hex').substring(0, 16);
}

const port = process.env.PORT || 3000;
const host = process.env.HOST || 'localhost';
console.log(`Analytics API running on ${host}:${port}`);

export default {
  port,
  fetch: app.fetch,
};