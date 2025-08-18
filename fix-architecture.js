#!/usr/bin/env bun
// Quick fix script for architecture mismatch issue

import { spawn } from 'child_process';

const colors = {
  reset: '\x1b[0m',
  bright: '\x1b[1m',
  green: '\x1b[32m',
  yellow: '\x1b[33m',
  blue: '\x1b[34m',
  red: '\x1b[31m',
  cyan: '\x1b[36m'
};

const log = {
  info: (msg) => console.log(`${colors.blue}ℹ${colors.reset} ${msg}`),
  success: (msg) => console.log(`${colors.green}✓${colors.reset} ${msg}`),
  warning: (msg) => console.log(`${colors.yellow}⚠${colors.reset} ${msg}`),
  error: (msg) => console.log(`${colors.red}✗${colors.reset} ${msg}`),
  step: (msg) => console.log(`${colors.cyan}▸${colors.reset} ${msg}`)
};

const exec = (command) => {
  return new Promise((resolve, reject) => {
    const [cmd, ...args] = command.split(' ');
    const child = spawn(cmd, args, { stdio: 'inherit' });
    child.on('close', (code) => {
      if (code !== 0) {
        reject(new Error(`Command failed: ${command}`));
      } else {
        resolve();
      }
    });
  });
};

async function main() {
  console.log(`${colors.bright}${colors.cyan}Architecture Fix for Slimlytics${colors.reset}\n`);
  
  log.info('This will rebuild your Docker image with the correct architecture and redeploy.');
  
  try {
    // Step 1: Set up buildx if not already done
    log.step('Setting up Docker buildx...');
    try {
      await exec('docker buildx create --use --name slimlytics-builder');
    } catch {
      log.info('Buildx builder already exists, continuing...');
    }
    
    // Step 2: Build for AMD64 only (server architecture)
    log.step('Building Docker image for linux/amd64 (server architecture)...');
    await exec('docker buildx build --platform linux/amd64 -t xhenxhe/slimlytics:latest --push .');
    log.success('Image built and pushed successfully!');
    
    // Step 3: Run deployment to update the server
    log.step('Redeploying to server...');
    await exec('bun run deploy.js');
    
    log.success('Architecture issue fixed! Your server should be working now.');
    log.info('Visit your site to verify it\'s working correctly.');
    
  } catch (error) {
    log.error(`Fix failed: ${error.message}`);
    log.warning('Manual steps to fix:');
    console.log('1. Run: docker buildx create --use --name slimlytics-builder');
    console.log('2. Run: docker buildx build --platform linux/amd64 -t xhenxhe/slimlytics:latest --push .');
    console.log('3. Run: bun run deploy');
    process.exit(1);
  }
}

main();