import { Database } from 'bun:sqlite';
import { mkdirSync } from 'fs';

try {
  mkdirSync('./data', { recursive: true });
} catch (e) {}

const db = new Database('./data/analytics.db');

db.exec(`
  CREATE TABLE IF NOT EXISTS sites (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    domain TEXT NOT NULL,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
  );

  CREATE TABLE IF NOT EXISTS events (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    site_id TEXT NOT NULL,
    page_url TEXT NOT NULL,
    referrer TEXT,
    user_agent TEXT,
    ip_hash TEXT,
    screen_resolution TEXT,
    language TEXT,
    timestamp DATETIME DEFAULT CURRENT_TIMESTAMP,
    -- Geo location columns
    country TEXT,
    country_code TEXT,
    region TEXT,
    city TEXT,
    latitude REAL,
    longitude REAL,
    timezone TEXT,
    asn INTEGER,
    asn_org TEXT,
    FOREIGN KEY (site_id) REFERENCES sites(id)
  );

  CREATE INDEX IF NOT EXISTS idx_events_site_timestamp ON events(site_id, timestamp);
  CREATE INDEX IF NOT EXISTS idx_events_timestamp ON events(timestamp);
  CREATE INDEX IF NOT EXISTS idx_events_country ON events(site_id, country_code);
  CREATE INDEX IF NOT EXISTS idx_events_city ON events(site_id, city);

  CREATE TABLE IF NOT EXISTS daily_stats (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    site_id TEXT NOT NULL,
    date DATE NOT NULL,
    page_views INTEGER DEFAULT 0,
    unique_visitors INTEGER DEFAULT 0,
    avg_session_duration INTEGER DEFAULT 0,
    bounce_rate REAL DEFAULT 0,
    top_pages TEXT,
    top_referrers TEXT,
    top_countries TEXT,
    top_cities TEXT,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (site_id) REFERENCES sites(id),
    UNIQUE(site_id, date)
  );

  CREATE INDEX IF NOT EXISTS idx_daily_stats_site_date ON daily_stats(site_id, date);
`);

const checkSite = db.prepare('SELECT id FROM sites WHERE id = ?');
const demoSite = checkSite.get('demo');

if (!demoSite) {
  const insertSite = db.prepare('INSERT INTO sites (id, name, domain) VALUES (?, ?, ?)');
  insertSite.run('demo', 'Demo Site', 'localhost');
  console.log('Demo site created');
}

console.log('Database initialized successfully');
db.close();