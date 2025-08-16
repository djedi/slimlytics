#!/usr/bin/env bun

import fs from 'fs';
import path from 'path';
import { pipeline } from 'stream/promises';
import tar from 'tar-fs';
import { createGunzip } from 'zlib';

// Load environment variables
const MAXMIND_ACCOUNT_ID = process.env.MAXMIND_ACCOUNT_ID;
const MAXMIND_LICENSE_KEY = process.env.MAXMIND_LICENSE_KEY;

if (!MAXMIND_ACCOUNT_ID || !MAXMIND_LICENSE_KEY) {
  console.error('Error: MAXMIND_ACCOUNT_ID and MAXMIND_LICENSE_KEY must be set in .env');
  process.exit(1);
}

const DATABASES = [
  { name: 'GeoLite2-City', url: 'https://download.maxmind.com/geoip/databases/GeoLite2-City/download?suffix=tar.gz' },
  { name: 'GeoLite2-Country', url: 'https://download.maxmind.com/geoip/databases/GeoLite2-Country/download?suffix=tar.gz' },
  { name: 'GeoLite2-ASN', url: 'https://download.maxmind.com/geoip/databases/GeoLite2-ASN/download?suffix=tar.gz' }
];

const DATA_DIR = path.join(process.cwd(), 'data', 'maxmind');

// Ensure data directory exists
if (!fs.existsSync(DATA_DIR)) {
  fs.mkdirSync(DATA_DIR, { recursive: true });
}

async function downloadDatabase(database) {
  console.log(`Downloading ${database.name}...`);
  
  const auth = Buffer.from(`${MAXMIND_ACCOUNT_ID}:${MAXMIND_LICENSE_KEY}`).toString('base64');
  
  try {
    const response = await fetch(database.url, {
      headers: {
        'Authorization': `Basic ${auth}`
      }
    });

    if (!response.ok) {
      throw new Error(`Failed to download ${database.name}: ${response.status} ${response.statusText}`);
    }

    const tempFile = path.join(DATA_DIR, `${database.name}.tar.gz`);
    const tempDir = path.join(DATA_DIR, `${database.name}-temp`);
    
    // Save the tarball
    const buffer = await response.arrayBuffer();
    fs.writeFileSync(tempFile, Buffer.from(buffer));
    
    // Create temp extraction directory
    if (!fs.existsSync(tempDir)) {
      fs.mkdirSync(tempDir, { recursive: true });
    }
    
    // Extract the tarball
    await pipeline(
      fs.createReadStream(tempFile),
      createGunzip(),
      tar.extract(tempDir)
    );
    
    // Find the .mmdb file in the extracted directory
    const extractedDir = fs.readdirSync(tempDir).find(f => f.startsWith(database.name));
    if (extractedDir) {
      const mmdbFile = path.join(tempDir, extractedDir, `${database.name}.mmdb`);
      const targetFile = path.join(DATA_DIR, `${database.name}.mmdb`);
      
      if (fs.existsSync(mmdbFile)) {
        // Move the .mmdb file to the data directory
        fs.renameSync(mmdbFile, targetFile);
        console.log(`✓ ${database.name} downloaded and extracted successfully`);
      } else {
        console.error(`✗ Could not find ${database.name}.mmdb in extracted files`);
      }
    }
    
    // Clean up temp files
    fs.rmSync(tempFile);
    fs.rmSync(tempDir, { recursive: true });
    
  } catch (error) {
    console.error(`✗ Error downloading ${database.name}:`, error.message);
  }
}

async function main() {
  console.log('Starting MaxMind database download...\n');
  
  for (const database of DATABASES) {
    await downloadDatabase(database);
  }
  
  console.log('\nDatabase download complete!');
  
  // List downloaded databases
  const files = fs.readdirSync(DATA_DIR).filter(f => f.endsWith('.mmdb'));
  if (files.length > 0) {
    console.log('\nAvailable databases:');
    files.forEach(file => {
      const stats = fs.statSync(path.join(DATA_DIR, file));
      console.log(`  - ${file} (${(stats.size / 1024 / 1024).toFixed(2)} MB)`);
    });
  }
}

main().catch(console.error);