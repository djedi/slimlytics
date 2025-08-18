#!/usr/bin/env bun
import { existsSync, readFileSync } from 'fs';
import { Client } from 'ssh2';
import { createInterface } from 'readline';

const CONFIG_FILE = '.deploy.json';

// Color codes for terminal output
const colors = {
  reset: '\x1b[0m',
  bright: '\x1b[1m',
  green: '\x1b[32m',
  yellow: '\x1b[33m',
  blue: '\x1b[34m',
  red: '\x1b[31m',
  cyan: '\x1b[36m',
  magenta: '\x1b[35m'
};

const log = {
  info: (msg) => console.log(`${colors.blue}ℹ${colors.reset} ${msg}`),
  success: (msg) => console.log(`${colors.green}✓${colors.reset} ${msg}`),
  warning: (msg) => console.log(`${colors.yellow}⚠${colors.reset} ${msg}`),
  error: (msg) => console.log(`${colors.red}✗${colors.reset} ${msg}`),
  step: (msg) => console.log(`${colors.cyan}▸${colors.reset} ${msg}`),
  header: (msg) => console.log(`\n${colors.bright}${colors.magenta}═══ ${msg} ═══${colors.reset}\n`)
};

// Readline interface for user input
const rl = createInterface({
  input: process.stdin,
  output: process.stdout
});

const question = (query) => new Promise(resolve => rl.question(query, resolve));

// SSH execute command with output streaming
const sshExecStream = (conn, command, showCommand = true) => {
  return new Promise((resolve, reject) => {
    if (showCommand) {
      log.step(`Running: ${colors.yellow}${command}${colors.reset}`);
    }
    
    conn.exec(command, (err, stream) => {
      if (err) return reject(err);
      
      let hasOutput = false;
      stream.on('close', (code) => {
        if (!hasOutput) {
          console.log(colors.cyan + '  (no output)' + colors.reset);
        }
        if (code !== 0 && code !== null) {
          console.log(`${colors.red}  Exit code: ${code}${colors.reset}`);
        }
        resolve();
      }).on('data', (data) => {
        hasOutput = true;
        process.stdout.write('  ' + data.toString());
      }).stderr.on('data', (data) => {
        hasOutput = true;
        process.stderr.write('  ' + colors.yellow + data.toString() + colors.reset);
      });
    });
  });
};

// Load configuration
function loadConfig() {
  if (!existsSync(CONFIG_FILE)) {
    log.error('No deployment configuration found. Run ./deploy first.');
    process.exit(1);
  }
  return JSON.parse(readFileSync(CONFIG_FILE, 'utf-8'));
}

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

// Main debug flow
async function main() {
  console.log(`${colors.bright}${colors.cyan}Slimlytics Server Logs & Debugging${colors.reset}\n`);
  
  try {
    // Load configuration
    const config = loadConfig();
    const server = config.servers.production;
    
    log.info(`Connecting to ${server.host} as ${server.username}...`);
    
    // Connect to server
    const conn = await connectSSH(server);
    
    // Check container status
    log.header('Container Status');
    await sshExecStream(conn, `cd ${server.path} && docker compose ps`);
    
    // Check if containers are running
    log.header('Container Health');
    await sshExecStream(conn, `cd ${server.path} && docker ps --format "table {{.Names}}\t{{.Status}}\t{{.Ports}}"`);
    
    // Show recent Slimlytics logs
    log.header('Slimlytics Application Logs (last 50 lines)');
    await sshExecStream(conn, `cd ${server.path} && docker compose logs --tail=50 slimlytics`);
    
    // Show recent Caddy logs
    log.header('Caddy Web Server Logs (last 30 lines)');
    await sshExecStream(conn, `cd ${server.path} && docker compose logs --tail=30 caddy`);
    
    // Check if app is responding on internal port
    log.header('Internal Health Check');
    await sshExecStream(conn, `cd ${server.path} && docker compose exec slimlytics curl -f http://localhost:3000/health || echo "Health check failed"`);
    
    // Check database file
    log.header('Database Status');
    await sshExecStream(conn, `ls -lah ${server.path}/data/slimlytics.db 2>/dev/null || echo "Database file not found"`);
    
    // Check environment file
    log.header('Environment File');
    await sshExecStream(conn, `test -f ${server.path}/.env && echo ".env file exists" || echo ".env file missing!"`);
    
    // Show running processes
    log.header('Docker Process Details');
    await sshExecStream(conn, `cd ${server.path} && docker compose top`);
    
    // Interactive menu
    log.header('Interactive Options');
    console.log('Choose an action:');
    console.log('1. Follow live logs (Slimlytics)');
    console.log('2. Follow live logs (Caddy)');
    console.log('3. Follow live logs (Both)');
    console.log('4. Restart containers');
    console.log('5. Show full docker-compose.yml');
    console.log('6. Show Caddyfile');
    console.log('7. Check disk space');
    console.log('8. Check memory usage');
    console.log('9. Test database connection');
    console.log('10. Show .env file (masked)');
    console.log('0. Exit');
    
    const choice = await question('\nEnter choice (0-10): ');
    
    switch(choice.trim()) {
      case '1':
        log.info('Following Slimlytics logs (Ctrl+C to stop)...');
        await sshExecStream(conn, `cd ${server.path} && docker compose logs -f slimlytics`);
        break;
        
      case '2':
        log.info('Following Caddy logs (Ctrl+C to stop)...');
        await sshExecStream(conn, `cd ${server.path} && docker compose logs -f caddy`);
        break;
        
      case '3':
        log.info('Following all logs (Ctrl+C to stop)...');
        await sshExecStream(conn, `cd ${server.path} && docker compose logs -f`);
        break;
        
      case '4':
        log.warning('Restarting containers...');
        await sshExecStream(conn, `cd ${server.path} && docker compose restart`);
        log.success('Containers restarted');
        break;
        
      case '5':
        log.header('Docker Compose Configuration');
        await sshExecStream(conn, `cat ${server.path}/docker-compose.yml`);
        break;
        
      case '6':
        log.header('Caddy Configuration');
        await sshExecStream(conn, `cat ${server.path}/Caddyfile`);
        break;
        
      case '7':
        log.header('Disk Space');
        await sshExecStream(conn, `df -h ${server.path}`);
        break;
        
      case '8':
        log.header('Memory Usage');
        await sshExecStream(conn, `free -h && echo && docker stats --no-stream`);
        break;
        
      case '9':
        log.header('Testing Database Connection');
        await sshExecStream(conn, `cd ${server.path} && docker compose exec slimlytics ls -la /app/data/`);
        await sshExecStream(conn, `cd ${server.path} && docker compose exec slimlytics sqlite3 /app/data/slimlytics.db "SELECT COUNT(*) as sites FROM sites;" 2>&1 || echo "Database query failed"`);
        break;
        
      case '10':
        log.header('Environment Variables (masked)');
        await sshExecStream(conn, `cd ${server.path} && cat .env | sed 's/=.*/=***MASKED***/'`);
        break;
        
      case '0':
        log.info('Exiting...');
        break;
        
      default:
        log.warning('Invalid choice');
    }
    
    // Common debugging tips
    log.header('Common 502 Error Causes');
    console.log(`
${colors.yellow}If you're seeing a 502 error, check:${colors.reset}

1. ${colors.cyan}Container not running:${colors.reset}
   - Check if slimlytics container is running above
   - Try: docker compose up -d

2. ${colors.cyan}Application crash on startup:${colors.reset}
   - Check the Slimlytics logs for errors
   - Common issues: missing .env, database permissions

3. ${colors.cyan}Port mismatch:${colors.reset}
   - Ensure app is listening on port 3000
   - Check docker-compose.yml configuration

4. ${colors.cyan}Database issues:${colors.reset}
   - Check if database file exists and has correct permissions
   - Try running: docker compose exec slimlytics bun run db:init

5. ${colors.cyan}Network issues:${colors.reset}
   - Ensure containers are on the same network
   - Check if Caddy can reach slimlytics container

${colors.green}Quick fix attempts:${colors.reset}
  ssh ${server.username}@${server.host}
  cd ${server.path}
  docker compose down
  docker compose up -d
  docker compose logs -f
`);
    
    // Close connections
    conn.end();
    rl.close();
    
  } catch (error) {
    log.error(`Error: ${error.message}`);
    process.exit(1);
  }
}

// Run debugging
main();