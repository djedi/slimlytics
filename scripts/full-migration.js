import { Database } from 'bun:sqlite';
import { mkdirSync } from 'fs';

try {
  mkdirSync('./data', { recursive: true });
} catch (e) {}

const db = new Database('./data/analytics.db');

console.log('Running full database migration...');

// Enable foreign keys
db.exec('PRAGMA foreign_keys = ON');

try {
  // Start a transaction
  db.exec('BEGIN TRANSACTION');

  // 1. Create sessions table if it doesn't exist
  console.log('Creating/updating sessions table...');
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
      
      -- IP address
      ip_address TEXT,
      
      -- Session metrics
      page_views INTEGER DEFAULT 0,
      duration INTEGER DEFAULT 0, -- in seconds
      is_bounce BOOLEAN DEFAULT FALSE,
      
      FOREIGN KEY (site_id) REFERENCES sites(id),
      UNIQUE(site_id, session_id)
    );
  `);

  // 2. Create indexes for sessions table
  console.log('Creating indexes for sessions table...');
  db.exec(`
    CREATE INDEX IF NOT EXISTS idx_sessions_site_visitor ON sessions(site_id, visitor_id);
    CREATE INDEX IF NOT EXISTS idx_sessions_site_started ON sessions(site_id, started_at);
    CREATE INDEX IF NOT EXISTS idx_sessions_session_id ON sessions(session_id);
  `);

  // 3. Add columns to events table if they don't exist
  console.log('Updating events table schema...');
  
  // Check and add visitor_id column
  const hasVisitorId = db.prepare(`
    SELECT COUNT(*) as count 
    FROM pragma_table_info('events') 
    WHERE name='visitor_id'
  `).get();
  
  if (hasVisitorId.count === 0) {
    console.log('Adding visitor_id column to events table...');
    db.exec(`ALTER TABLE events ADD COLUMN visitor_id TEXT`);
  }

  // Check and add session_id column
  const hasSessionId = db.prepare(`
    SELECT COUNT(*) as count 
    FROM pragma_table_info('events') 
    WHERE name='session_id'
  `).get();
  
  if (hasSessionId.count === 0) {
    console.log('Adding session_id column to events table...');
    db.exec(`ALTER TABLE events ADD COLUMN session_id TEXT`);
  }

  // 4. Add indexes for the new columns
  console.log('Creating indexes for events table...');
  db.exec(`
    CREATE INDEX IF NOT EXISTS idx_events_visitor_id ON events(site_id, visitor_id);
    CREATE INDEX IF NOT EXISTS idx_events_session_id ON events(site_id, session_id);
  `);

  // 5. Add geo columns to events if they don't exist
  console.log('Checking geo columns in events table...');
  const geoColumns = [
    'country', 'country_code', 'region', 'city', 
    'latitude', 'longitude', 'timezone', 'asn', 'asn_org'
  ];

  for (const column of geoColumns) {
    const hasColumn = db.prepare(`
      SELECT COUNT(*) as count 
      FROM pragma_table_info('events') 
      WHERE name=?
    `).get(column);
    
    if (hasColumn.count === 0) {
      console.log(`Adding ${column} column to events table...`);
      const type = column.includes('latitude') || column.includes('longitude') ? 'REAL' : 
                   column === 'asn' ? 'INTEGER' : 'TEXT';
      db.exec(`ALTER TABLE events ADD COLUMN ${column} ${type}`);
    }
  }

  // 6. Add geo indexes
  console.log('Creating geo indexes...');
  db.exec(`
    CREATE INDEX IF NOT EXISTS idx_events_country ON events(site_id, country_code);
    CREATE INDEX IF NOT EXISTS idx_events_city ON events(site_id, city);
  `);

  // 7. Update daily_stats table
  console.log('Checking daily_stats table...');
  const hasTopCountries = db.prepare(`
    SELECT COUNT(*) as count 
    FROM pragma_table_info('daily_stats') 
    WHERE name='top_countries'
  `).get();
  
  if (hasTopCountries.count === 0) {
    console.log('Adding geo columns to daily_stats table...');
    db.exec(`
      ALTER TABLE daily_stats ADD COLUMN top_countries TEXT;
      ALTER TABLE daily_stats ADD COLUMN top_cities TEXT;
    `);
  }

  // 8. Add ip_address to sessions if missing
  const hasIpAddress = db.prepare(`
    SELECT COUNT(*) as count 
    FROM pragma_table_info('sessions') 
    WHERE name='ip_address'
  `).get();
  
  if (hasIpAddress.count === 0) {
    console.log('Adding ip_address column to sessions table...');
    db.exec(`ALTER TABLE sessions ADD COLUMN ip_address TEXT`);
  }

  // Commit the transaction
  db.exec('COMMIT');
  
  console.log('✅ Migration completed successfully!');
  
  // Show some stats
  const stats = {
    sites: db.prepare('SELECT COUNT(*) as count FROM sites').get().count,
    events: db.prepare('SELECT COUNT(*) as count FROM events').get().count,
    sessions: db.prepare('SELECT COUNT(*) as count FROM sessions').get().count,
  };
  
  console.log('\nDatabase Statistics:');
  console.log(`  Sites: ${stats.sites}`);
  console.log(`  Events: ${stats.events}`);
  console.log(`  Sessions: ${stats.sessions}`);
  
} catch (error) {
  console.error('❌ Migration failed:', error.message);
  
  // Rollback on error
  try {
    db.exec('ROLLBACK');
    console.log('Transaction rolled back');
  } catch (rollbackError) {
    console.error('Failed to rollback:', rollbackError.message);
  }
  
  process.exit(1);
} finally {
  db.close();
}