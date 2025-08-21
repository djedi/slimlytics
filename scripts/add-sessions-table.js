import { Database } from 'bun:sqlite';
import { mkdirSync } from 'fs';

try {
  mkdirSync('./data', { recursive: true });
} catch (e) {}

const db = new Database('./data/analytics.db');

console.log('Adding sessions table...');

db.exec(`
  CREATE TABLE IF NOT EXISTS sessions (
    id TEXT PRIMARY KEY,
    site_id TEXT NOT NULL,
    visitor_id TEXT NOT NULL,
    session_id TEXT NOT NULL,
    started_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    last_activity DATETIME DEFAULT CURRENT_TIMESTAMP,
    
    -- Traffic source - stored once per session
    referrer TEXT,
    utm_source TEXT,
    utm_medium TEXT,
    utm_campaign TEXT,
    utm_term TEXT,
    utm_content TEXT,
    traffic_source TEXT, -- direct, organic, referral, social, etc.
    
    -- Visitor info - stored once per session
    language TEXT,
    country TEXT,
    country_code TEXT,
    region TEXT,
    city TEXT,
    latitude REAL,
    longitude REAL,
    timezone TEXT,
    
    -- Device info - stored once per session
    user_agent TEXT,
    screen_resolution TEXT,
    browser TEXT,
    browser_version TEXT,
    os TEXT,
    os_version TEXT,
    device_type TEXT, -- desktop, mobile, tablet
    
    -- Session metrics
    page_views INTEGER DEFAULT 0,
    duration INTEGER DEFAULT 0, -- in seconds
    is_bounce BOOLEAN DEFAULT FALSE,
    
    FOREIGN KEY (site_id) REFERENCES sites(id),
    UNIQUE(site_id, session_id)
  );
  
  CREATE INDEX IF NOT EXISTS idx_sessions_site_visitor ON sessions(site_id, visitor_id);
  CREATE INDEX IF NOT EXISTS idx_sessions_site_started ON sessions(site_id, started_at);
  CREATE INDEX IF NOT EXISTS idx_sessions_session_id ON sessions(session_id);
`);

console.log('Sessions table created successfully');

// Add visitor_id and session_id columns to events table if they don't exist
try {
  db.exec(`ALTER TABLE events ADD COLUMN visitor_id TEXT`);
  console.log('Added visitor_id column to events table');
} catch (e) {
  console.log('visitor_id column already exists or error:', e.message);
}

try {
  db.exec(`ALTER TABLE events ADD COLUMN session_id TEXT`);
  console.log('Added session_id column to events table');
} catch (e) {
  console.log('session_id column already exists or error:', e.message);
}

// Add indexes for the new columns
db.exec(`
  CREATE INDEX IF NOT EXISTS idx_events_visitor_id ON events(site_id, visitor_id);
  CREATE INDEX IF NOT EXISTS idx_events_session_id ON events(site_id, session_id);
`);

console.log('Indexes created successfully');

db.close();