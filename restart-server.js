#!/usr/bin/env bun
import { existsSync, readFileSync } from 'fs';
import { Client } from 'ssh2';

const CONFIG_FILE = '.deploy.json';

const colors = {
  reset: '\x1b[0m',
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

// SSH execute command
const sshExec = (conn, command) => {
  return new Promise((resolve, reject) => {
    log.step(`Running: ${command}`);
    let output = '';
    conn.exec(command, (err, stream) => {
      if (err) return reject(err);
      
      stream.on('close', (code) => {
        if (code !== 0) {
          reject(new Error(`Command failed with code ${code}: ${output}`));
        } else {
          resolve(output);
        }
      }).on('data', (data) => {
        const str = data.toString();
        output += str;
        process.stdout.write('  ' + str);
      }).stderr.on('data', (data) => {
        const str = data.toString();
        output += str;
        process.stderr.write('  ' + colors.yellow + str + colors.reset);
      });
    });
  });
};

// Connect to SSH
function connectSSH(server) {
  return new Promise((resolve, reject) => {
    const conn = new Client();
    
    conn.on('ready', () => {
      log.success('Connected to server');
      resolve(conn);
    }).on('error', (err) => {
      reject(err);
    });
    
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

async function main() {
  log.info('Restarting server with fixed architecture...');
  
  try {
    // Load configuration
    const config = JSON.parse(readFileSync(CONFIG_FILE, 'utf-8'));
    const server = config.servers.production;
    
    // Connect to server
    const conn = await connectSSH(server);
    
    // Pull the latest image and restart
    await sshExec(conn, `cd ${server.path} && docker compose pull slimlytics`);
    await sshExec(conn, `cd ${server.path} && docker compose down`);
    await sshExec(conn, `cd ${server.path} && docker compose up -d`);
    
    // Wait a moment for services to start
    log.info('Waiting for services to start...');
    await new Promise(resolve => setTimeout(resolve, 5000));
    
    // Check status
    await sshExec(conn, `cd ${server.path} && docker compose ps`);
    
    // Test health
    try {
      await sshExec(conn, `cd ${server.path} && docker compose exec slimlytics curl -f http://localhost:3000/health`);
      log.success('Health check passed!');
    } catch {
      log.warning('Health check failed - the app may still be starting up');
    }
    
    log.success(`Deployment complete! Visit https://${server.domain} to check your site.`);
    
    conn.end();
  } catch (error) {
    log.error(`Failed: ${error.message}`);
    process.exit(1);
  }
}

main();