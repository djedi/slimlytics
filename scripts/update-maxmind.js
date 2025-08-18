#!/usr/bin/env bun

import fs from 'fs';
import path from 'path';
import { execSync } from 'child_process';

const DATA_DIR = path.join(process.cwd(), 'data', 'maxmind');
const UPDATE_LOG = path.join(DATA_DIR, 'update.log');
const VERSION_FILE = path.join(DATA_DIR, 'version.json');

// Check if update is needed (MaxMind updates Tuesdays and Fridays)
function shouldUpdate() {
  // Always update if databases don't exist
  const databases = ['GeoLite2-City.mmdb', 'GeoLite2-Country.mmdb', 'GeoLite2-ASN.mmdb'];
  for (const db of databases) {
    if (!fs.existsSync(path.join(DATA_DIR, db))) {
      console.log(`Database ${db} not found, update needed`);
      return true;
    }
  }
  
  // Check last update time
  if (fs.existsSync(VERSION_FILE)) {
    try {
      const versionData = JSON.parse(fs.readFileSync(VERSION_FILE, 'utf8'));
      const lastUpdate = new Date(versionData.lastUpdate);
      const now = new Date();
      const daysSinceUpdate = (now - lastUpdate) / (1000 * 60 * 60 * 24);
      
      // Update if more than 3 days have passed (to catch Tuesday/Friday updates)
      if (daysSinceUpdate >= 3) {
        console.log(`Last update was ${daysSinceUpdate.toFixed(1)} days ago, update recommended`);
        return true;
      }
      
      console.log(`Last update was ${daysSinceUpdate.toFixed(1)} days ago, databases are current`);
      return false;
    } catch (error) {
      console.error('Error reading version file:', error);
      return true;
    }
  }
  
  // No version file, update needed
  return true;
}

// Log update attempt
function logUpdate(status, message) {
  const timestamp = new Date().toISOString();
  const logEntry = `${timestamp} - ${status} - ${message}\n`;
  
  // Ensure directory exists
  if (!fs.existsSync(DATA_DIR)) {
    fs.mkdirSync(DATA_DIR, { recursive: true });
  }
  
  // Append to log file
  fs.appendFileSync(UPDATE_LOG, logEntry);
  
  // Also log to console
  console.log(`[${status}] ${message}`);
}

// Get file sizes for comparison
function getDatabaseSizes() {
  const sizes = {};
  const databases = ['GeoLite2-City.mmdb', 'GeoLite2-Country.mmdb', 'GeoLite2-ASN.mmdb'];
  
  for (const db of databases) {
    const dbPath = path.join(DATA_DIR, db);
    if (fs.existsSync(dbPath)) {
      const stats = fs.statSync(dbPath);
      sizes[db] = stats.size;
    }
  }
  
  return sizes;
}

// Main update function
async function updateMaxMindDatabases() {
  console.log('=== MaxMind Database Auto-Update ===\n');
  
  // Check if update is needed
  if (!shouldUpdate() && process.argv[2] !== '--force') {
    logUpdate('SKIP', 'Databases are up to date');
    console.log('\nUse --force flag to force an update\n');
    return;
  }
  
  // Check for credentials
  if (!process.env.MAXMIND_ACCOUNT_ID || !process.env.MAXMIND_LICENSE_KEY) {
    logUpdate('ERROR', 'MaxMind credentials not found in environment');
    console.error('\nError: MAXMIND_ACCOUNT_ID and MAXMIND_LICENSE_KEY must be set in .env\n');
    process.exit(1);
  }
  
  // Get sizes before update
  const sizesBefore = getDatabaseSizes();
  
  try {
    // Run the download script
    console.log('Downloading updated databases...\n');
    execSync('bun run scripts/download-maxmind.js', { 
      stdio: 'inherit',
      env: process.env 
    });
    
    // Get sizes after update
    const sizesAfter = getDatabaseSizes();
    
    // Check if databases actually changed
    let hasChanges = false;
    for (const db in sizesAfter) {
      if (!sizesBefore[db] || sizesBefore[db] !== sizesAfter[db]) {
        hasChanges = true;
        const sizeDiff = sizesAfter[db] - (sizesBefore[db] || 0);
        const sign = sizeDiff > 0 ? '+' : '';
        console.log(`  ${db}: ${sign}${(sizeDiff / 1024).toFixed(1)} KB`);
      }
    }
    
    if (hasChanges) {
      // Update version file
      const versionData = {
        lastUpdate: new Date().toISOString(),
        databases: sizesAfter,
        updateCount: (JSON.parse(fs.readFileSync(VERSION_FILE, 'utf8')).updateCount || 0) + 1
      };
      fs.writeFileSync(VERSION_FILE, JSON.stringify(versionData, null, 2));
      
      logUpdate('SUCCESS', 'Databases updated successfully');
      console.log('\n✅ Databases have been updated with new data\n');
      
      // Restart the API server if it's running (for production)
      if (process.env.NODE_ENV === 'production') {
        console.log('Restarting API server to load new databases...');
        try {
          // Try systemctl first (for systemd systems)
          execSync('systemctl restart slimlytics', { stdio: 'ignore' });
          console.log('API server restarted successfully');
        } catch (error) {
          // Try pm2 as fallback
          try {
            execSync('pm2 restart slimlytics', { stdio: 'ignore' });
            console.log('API server restarted successfully (pm2)');
          } catch (pm2Error) {
            console.log('Note: Could not auto-restart server. Please restart manually.');
          }
        }
      }
    } else {
      logUpdate('SUCCESS', 'Databases checked - no changes detected');
      console.log('\n✅ Databases are already up to date\n');
    }
    
  } catch (error) {
    logUpdate('ERROR', error.message);
    console.error('\n❌ Update failed:', error.message, '\n');
    process.exit(1);
  }
}

// Show update statistics
function showStats() {
  if (fs.existsSync(VERSION_FILE)) {
    try {
      const versionData = JSON.parse(fs.readFileSync(VERSION_FILE, 'utf8'));
      console.log('\n📊 Update Statistics:');
      console.log(`  Last Update: ${new Date(versionData.lastUpdate).toLocaleString()}`);
      console.log(`  Total Updates: ${versionData.updateCount || 1}`);
      console.log(`  Database Sizes:`);
      for (const [db, size] of Object.entries(versionData.databases || {})) {
        console.log(`    - ${db}: ${(size / 1024 / 1024).toFixed(2)} MB`);
      }
    } catch (error) {
      console.error('Could not read version data');
    }
  }
  
  // Show recent log entries
  if (fs.existsSync(UPDATE_LOG)) {
    console.log('\n📝 Recent Updates:');
    const logs = fs.readFileSync(UPDATE_LOG, 'utf8').split('\n').filter(Boolean);
    const recent = logs.slice(-5);
    recent.forEach(log => {
      const [timestamp, ...rest] = log.split(' - ');
      const date = new Date(timestamp);
      console.log(`  ${date.toLocaleDateString()} ${date.toLocaleTimeString()} - ${rest.join(' - ')}`);
    });
  }
}

// Run the update
updateMaxMindDatabases().then(() => {
  showStats();
}).catch(console.error);