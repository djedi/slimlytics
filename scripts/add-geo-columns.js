#!/usr/bin/env bun

import { Database } from 'bun:sqlite';

const db = new Database('./data/analytics.db');

console.log('Adding geo location columns to events table...');

try {
  // Add geo location columns to events table
  db.exec(`
    -- Add country columns
    ALTER TABLE events ADD COLUMN country TEXT;
    ALTER TABLE events ADD COLUMN country_code TEXT;
    
    -- Add region/city columns
    ALTER TABLE events ADD COLUMN region TEXT;
    ALTER TABLE events ADD COLUMN city TEXT;
    
    -- Add coordinates
    ALTER TABLE events ADD COLUMN latitude REAL;
    ALTER TABLE events ADD COLUMN longitude REAL;
    
    -- Add timezone
    ALTER TABLE events ADD COLUMN timezone TEXT;
    
    -- Add ASN info
    ALTER TABLE events ADD COLUMN asn INTEGER;
    ALTER TABLE events ADD COLUMN asn_org TEXT;
    
    -- Add indexes for geo data
    CREATE INDEX IF NOT EXISTS idx_events_country ON events(site_id, country_code);
    CREATE INDEX IF NOT EXISTS idx_events_city ON events(site_id, city);
  `);
  
  console.log('✓ Geo location columns added successfully');
  
  // Also add geo stats to daily_stats table
  db.exec(`
    -- Add geo summary columns to daily_stats
    ALTER TABLE daily_stats ADD COLUMN top_countries TEXT;
    ALTER TABLE daily_stats ADD COLUMN top_cities TEXT;
  `);
  
  console.log('✓ Geo summary columns added to daily_stats');
  
} catch (error) {
  if (error.message.includes('duplicate column name')) {
    console.log('Geo columns already exist, skipping...');
  } else {
    console.error('Error adding geo columns:', error);
    process.exit(1);
  }
}

db.close();
console.log('Database migration complete!');