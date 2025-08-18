#!/usr/bin/env bun
import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'fs';
import { join, dirname } from 'path';
import { spawn } from 'child_process';
import { createInterface } from 'readline';
import { Client } from 'ssh2';
import * as dotenv from 'dotenv';

// Load environment variables
dotenv.config();

const CONFIG_FILE = '.deploy.json';
const BACKUP_DIR = 'prod_db_backups';
const DOCKER_REGISTRY = 'xhenxhe';
const IMAGE_NAME = 'slimlytics';

// Color codes for terminal output
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

// Readline interface for user input
const rl = createInterface({
  input: process.stdin,
  output: process.stdout
});

const question = (query) => new Promise(resolve => rl.question(query, resolve));

// Execute shell command
const exec = (command, options = {}) => {
  return new Promise((resolve, reject) => {
    const [cmd, ...args] = command.split(' ');
    const child = spawn(cmd, args, {
      stdio: options.silent ? 'pipe' : 'inherit',
      ...options
    });

    let output = '';
    if (options.silent) {
      child.stdout?.on('data', (data) => output += data.toString());
      child.stderr?.on('data', (data) => output += data.toString());
    }

    child.on('close', (code) => {
      if (code !== 0) {
        reject(new Error(`Command failed: ${command}\n${output}`));
      } else {
        resolve(output);
      }
    });
  });
};

// SSH execute command
const sshExec = (conn, command) => {
  return new Promise((resolve, reject) => {
    let output = '';
    conn.exec(command, (err, stream) => {
      if (err) return reject(err);
      
      stream.on('close', (code) => {
        if (code !== 0) {
          reject(new Error(`SSH command failed: ${command}\n${output}`));
        } else {
          resolve(output);
        }
      }).on('data', (data) => {
        output += data.toString();
      }).stderr.on('data', (data) => {
        output += data.toString();
      });
    });
  });
};

// SCP file transfer
const scpDownload = (conn, remotePath, localPath) => {
  return new Promise((resolve, reject) => {
    conn.sftp((err, sftp) => {
      if (err) return reject(err);
      
      sftp.fastGet(remotePath, localPath, (err) => {
        if (err) return reject(err);
        resolve();
      });
    });
  });
};

const scpUpload = (conn, localPath, remotePath) => {
  return new Promise((resolve, reject) => {
    conn.sftp((err, sftp) => {
      if (err) return reject(err);
      
      sftp.fastPut(localPath, remotePath, (err) => {
        if (err) return reject(err);
        resolve();
      });
    });
  });
};

// Load or create configuration
async function loadConfig() {
  if (existsSync(CONFIG_FILE)) {
    log.info('Loading deployment configuration...');
    return JSON.parse(readFileSync(CONFIG_FILE, 'utf-8'));
  }
  
  log.info('No deployment configuration found. Let\'s set up your server.');
  
  const config = {
    servers: {
      production: {}
    },
    docker: {
      registry: DOCKER_REGISTRY,
      imageName: IMAGE_NAME
    }
  };
  
  // Prompt for server details
  config.servers.production.host = await question('Server IP or domain: ');
  config.servers.production.username = await question('SSH username (default: root): ') || 'root';
  config.servers.production.domain = await question('Application domain (for SSL): ');
  config.servers.production.path = '/opt/slimlytics';
  
  // Save configuration
  writeFileSync(CONFIG_FILE, JSON.stringify(config, null, 2));
  log.success('Configuration saved to .deploy.json');
  
  return config;
}

// Connect to SSH
function connectSSH(server) {
  return new Promise((resolve, reject) => {
    const conn = new Client();
    
    conn.on('ready', () => {
      log.success('SSH connection established');
      resolve(conn);
    }).on('error', (err) => {
      reject(err);
    });
    
    // Try to connect with SSH key first
    conn.connect({
      host: server.host,
      username: server.username,
      port: 22,
      privateKey: existsSync(`${process.env.HOME}/.ssh/id_rsa`) 
        ? readFileSync(`${process.env.HOME}/.ssh/id_rsa`) 
        : undefined,
      tryKeyboard: true
    });
  });
}

// Check if Docker is installed on server
async function checkDocker(conn) {
  try {
    await sshExec(conn, 'docker --version');
    log.success('Docker is installed');
    return true;
  } catch {
    return false;
  }
}

// Install Docker on server
async function installDocker(conn) {
  log.step('Installing Docker...');
  
  const commands = [
    'apt-get update',
    'apt-get install -y ca-certificates curl gnupg',
    'install -m 0755 -d /etc/apt/keyrings',
    'curl -fsSL https://download.docker.com/linux/ubuntu/gpg | gpg --dearmor -o /etc/apt/keyrings/docker.gpg',
    'chmod a+r /etc/apt/keyrings/docker.gpg',
    'echo "deb [arch="$(dpkg --print-architecture)" signed-by=/etc/apt/keyrings/docker.gpg] https://download.docker.com/linux/ubuntu "$(. /etc/os-release && echo "$VERSION_CODENAME")" stable" | tee /etc/apt/sources.list.d/docker.list > /dev/null',
    'apt-get update',
    'apt-get install -y docker-ce docker-ce-cli containerd.io docker-buildx-plugin docker-compose-plugin',
    'systemctl enable docker',
    'systemctl start docker'
  ];
  
  for (const cmd of commands) {
    await sshExec(conn, cmd);
  }
  
  log.success('Docker installed successfully');
}

// Build and push Docker image
async function buildAndPushImage(tag = 'latest') {
  log.step(`Building multi-architecture Docker image: ${DOCKER_REGISTRY}/${IMAGE_NAME}:${tag}`);
  
  try {
    // Check if buildx is available
    try {
      await exec('docker buildx version', { silent: true });
    } catch {
      log.step('Setting up Docker buildx for multi-architecture builds...');
      await exec('docker buildx create --use --name slimlytics-builder');
    }
    
    // Build and push multi-architecture image (AMD64 and ARM64)
    log.step('Building for linux/amd64 and linux/arm64...');
    await exec(`docker buildx build --platform linux/amd64,linux/arm64 -t ${DOCKER_REGISTRY}/${IMAGE_NAME}:${tag} --push .`);
    log.success('Multi-architecture image built and pushed to Docker Hub');
  } catch (error) {
    log.error('Failed to build or push Docker image');
    log.warning('Make sure you are logged in to Docker Hub: docker login');
    log.warning('If buildx fails, you may need to run: docker buildx create --use');
    throw error;
  }
}

// Backup database
async function backupDatabase(conn, server) {
  if (!existsSync(BACKUP_DIR)) {
    mkdirSync(BACKUP_DIR, { recursive: true });
  }
  
  const timestamp = new Date().toISOString().replace(/[:.]/g, '-');
  const backupFile = join(BACKUP_DIR, `backup-${timestamp}.db`);
  const remotePath = `${server.path}/data/slimlytics.db`;
  
  log.step('Backing up database...');
  
  try {
    await scpDownload(conn, remotePath, backupFile);
    log.success(`Database backed up to ${backupFile}`);
  } catch (error) {
    log.warning('No existing database to backup (this is normal for first deployment)');
  }
}

// Generate Caddyfile from template
function generateCaddyfile(domain) {
  const template = readFileSync('Caddyfile.template', 'utf-8');
  return template.replace(/{DOMAIN}/g, domain);
}

// Setup server for first deployment
async function setupServer(conn, server) {
  log.step('Setting up server for first deployment...');
  
  // Create directory structure
  await sshExec(conn, `mkdir -p ${server.path}/data`);
  await sshExec(conn, `mkdir -p ${server.path}/caddy_data`);
  await sshExec(conn, `mkdir -p ${server.path}/caddy_config`);
  
  // Upload configuration files
  log.step('Uploading configuration files...');
  
  // Generate and upload Caddyfile
  const caddyfile = generateCaddyfile(server.domain);
  writeFileSync('.tmp.caddyfile', caddyfile);
  await scpUpload(conn, '.tmp.caddyfile', `${server.path}/Caddyfile`);
  require('fs').unlinkSync('.tmp.caddyfile');
  
  // Upload docker-compose file
  await scpUpload(conn, 'docker-compose.production.yml', `${server.path}/docker-compose.yml`);
  
  // Upload .env file if it exists
  if (existsSync('.env')) {
    await scpUpload(conn, '.env', `${server.path}/.env`);
  } else {
    log.warning('No .env file found. You may need to configure environment variables on the server.');
  }
  
  log.success('Server setup complete');
}

// Deploy application
async function deploy(conn, server, isFirstDeployment = false) {
  if (isFirstDeployment) {
    log.step('Starting services for the first time...');
    await sshExec(conn, `cd ${server.path} && docker compose up -d`);
  } else {
    log.step('Updating services...');
    await sshExec(conn, `cd ${server.path} && docker compose pull`);
    await sshExec(conn, `cd ${server.path} && docker compose up -d --no-deps --force-recreate slimlytics`);
  }
  
  // Wait for services to be healthy
  log.step('Waiting for services to be healthy...');
  await new Promise(resolve => setTimeout(resolve, 5000));
  
  // Check service status
  const status = await sshExec(conn, `cd ${server.path} && docker compose ps`);
  log.info('Service status:\n' + status);
  
  log.success('Deployment complete!');
  log.info(`Your application is running at https://${server.domain}`);
}

// Main deployment flow
async function main() {
  console.log(`${colors.bright}${colors.cyan}Slimlytics Deploy Script${colors.reset}\n`);
  
  try {
    // Load configuration
    const config = await loadConfig();
    const server = config.servers.production;
    
    // Connect to server
    log.step('Connecting to server...');
    const conn = await connectSSH(server);
    
    // Check if this is first deployment
    let isFirstDeployment = false;
    try {
      await sshExec(conn, `test -d ${server.path}`);
    } catch {
      isFirstDeployment = true;
      log.info('This appears to be the first deployment to this server');
    }
    
    // Check and install Docker if needed
    if (!await checkDocker(conn)) {
      const install = await question('Docker is not installed. Install it now? (y/n): ');
      if (install.toLowerCase() === 'y') {
        await installDocker(conn);
      } else {
        throw new Error('Docker is required for deployment');
      }
    }
    
    // Backup database (if not first deployment)
    if (!isFirstDeployment) {
      await backupDatabase(conn, server);
    }
    
    // Build and push Docker image
    await buildAndPushImage();
    
    // Setup server if first deployment
    if (isFirstDeployment) {
      await setupServer(conn, server);
    }
    
    // Deploy application
    await deploy(conn, server, isFirstDeployment);
    
    // Close connections
    conn.end();
    rl.close();
    
  } catch (error) {
    log.error(`Deployment failed: ${error.message}`);
    process.exit(1);
  }
}

// Run deployment
main();