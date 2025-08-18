# Slim Analytics - Development Setup

A lightweight, privacy-friendly analytics application built with Bun, SQLite, 11ty, HTMX, and Alpine.js.

## Prerequisites

- [Bun](https://bun.sh) (latest version)
- Node.js 18+ (for 11ty)

## Quick Start

### 1. Install Dependencies

```bash
bun install
```

### 2. Initialize Database

```bash
bun run db:init
```

This creates the SQLite database at `./data/analytics.db` with the required schema and a demo site.

### 3. Set up GeoIP Location Tracking (Optional but Recommended)

To enable geographic location tracking for your visitors:

1. Create a free MaxMind account at https://www.maxmind.com/en/geolite2/signup
2. Create a `.env` file and add your MaxMind credentials:
   ```
   MAXMIND_ACCOUNT_ID=your_account_id
   MAXMIND_LICENSE_KEY=your_license_key
   ```
3. Download the GeoIP databases:
   ```bash
   bun run maxmind:download
   ```
4. Set up automatic updates (recommended):
   ```bash
   bun run maxmind:setup-auto-update
   ```
5. If upgrading an existing installation, run the migration:
   ```bash
   bun run db:migrate
   ```

### 4. Run in Development Mode

```bash
bun run dev
```

This starts:

- **API Server**: http://localhost:3000 (with hot reload and GeoIP support)
- **Dashboard**: http://localhost:8080 (with live reload)

## Development Commands

### Individual Services

Run API server only:

```bash
bun run dev:api
```

Run dashboard only:

```bash
bun run dev:dashboard
```

Build dashboard for production:

```bash
bun run build
```

### Database Management

Initialize database:
```bash
bun run db:init
```

Run geo columns migration (for existing databases):
```bash
bun run db:migrate
```

### Deployment & Production Commands

Deploy to production server:
```bash
bun run deploy    # Deploy application to configured server
```

Debug production server:
```bash
bun run logs      # Interactive server debugging and log viewer
```

### GeoIP Database Management

#### Manual Updates
```bash
bun run maxmind:download   # Force download all databases
bun run maxmind:update     # Smart update - only downloads if needed
bun run maxmind:update -- --force  # Force update even if current
```

#### Automatic Updates (Recommended)

Set up twice-weekly automatic updates:
```bash
bun run maxmind:setup-auto-update
```

This interactive script will:
- Detect your operating system
- Offer scheduling options (cron, systemd, or launchd)
- Configure updates for Wednesdays and Saturdays at 3 AM
- Only download when databases are >3 days old or missing

#### Update System Features

- **Smart Updates**: Only downloads when needed (databases >3 days old)
- **Version Tracking**: Maintains update history in `data/maxmind/version.json`
- **Logging**: All updates logged to `data/maxmind/update.log`
- **Multiple Schedulers**: Support for cron, systemd, and launchd
- **Production Ready**: Auto-restarts API server after updates

#### Monitoring Updates

```bash
# Check last update and statistics
cat data/maxmind/version.json

# View update history
tail -20 data/maxmind/update.log

# Check database ages
ls -la data/maxmind/*.mmdb
```

**Update Schedule**: 
- MaxMind releases new data on Tuesdays and Fridays
- Auto-updates run on Wednesdays and Saturdays to catch these releases
- You can download up to 30 times per day
- Each update uses ~80MB bandwidth (only when changes detected)

For detailed setup instructions and troubleshooting, see `docs/MAXMIND_UPDATES.md`.

## Project Structure

```
slimlytics/
├── src/
│   ├── api/          # API server
│   │   ├── server.js # Main API endpoints with WebSocket support
│   │   ├── sites.ts  # Sites management API
│   │   └── stats.ts  # Statistics API
│   ├── dashboard/    # 11ty dashboard source
│   │   ├── _includes/# Templates
│   │   ├── _data/    # Data files
│   │   └── index.html# Main dashboard page
│   ├── db/           # Database utilities
│   │   ├── init.js   # Schema initialization
│   │   └── queries.js# Database queries
│   └── public/       # Static assets
│       ├── js/       # JavaScript files
│       │   ├── tracker.js   # Tracking script
│       │   └── dashboard.js # Dashboard functionality
│       └── css/      # Stylesheets
├── api/
│   └── services/
│       └── geoip.js  # GeoIP location service
├── scripts/          # Utility scripts
│   ├── download-maxmind.js  # Download GeoIP databases
│   ├── update-maxmind.js    # Smart update with version tracking
│   ├── setup-auto-update.sh # Configure automatic updates
│   └── add-geo-columns.js   # Database migration
├── dist/             # Built dashboard (generated)
├── data/             # SQLite database and GeoIP data (generated)
│   ├── analytics.db  # Main database
│   └── maxmind/      # GeoIP database files
│       ├── GeoLite2-City.mmdb     # ~58MB - City-level data
│       ├── GeoLite2-Country.mmdb  # ~9MB - Country data
│       ├── GeoLite2-ASN.mmdb      # ~10MB - ISP/ASN data
│       ├── version.json            # Update tracking
│       └── update.log              # Update history
└── package.json      # Dependencies
```

## Testing the Tracker

Add this script to any HTML page to track visits:

```html
<script>
  window.SLIMLYTICS_SITE_ID = "demo"; // Your site ID
</script>
<script src="http://localhost:3000/js/tracker.js"></script>
```

Or test with curl:

```bash
curl -X POST http://localhost:3000/track \
  -H "Content-Type: application/json" \
  -d '{
    "site_id": "demo",
    "page_url": "http://example.com/test",
    "referrer": "http://google.com",
    "user_agent": "Mozilla/5.0",
    "screen_resolution": "1920x1080",
    "language": "en-US"
  }'
```

## API Endpoints

- `POST /track` - Track page view (with automatic GeoIP lookup)
- `GET /api/sites` - List all sites
- `POST /api/sites` - Create a new site
- `PUT /api/sites/:id` - Update a site
- `DELETE /api/sites/:id` - Delete a site
- `GET /api/stats/:siteId` - Get statistics for a site (including location data)
- `GET /api/stats/:siteId/timeseries` - Get time series data for charts
- `GET /api/stats/:siteId/realtime` - Get real-time visitor count
- WebSocket at `ws://localhost:3000/` - Real-time stats updates

## Environment Variables

For production, set these environment variables:

- `PORT` - API server port (default: 3000)
- `HOST` - API server host (default: localhost)
- `SALT` - Salt for IP hashing (important for privacy)
- `NODE_ENV` - Set to 'production' for production mode
- `MAXMIND_ACCOUNT_ID` - Your MaxMind account ID (for GeoIP)
- `MAXMIND_LICENSE_KEY` - Your MaxMind license key (for GeoIP)

### Production-Only Variables

- `DOCKER_REGISTRY` - Your Docker Hub username (configured in deploy.js)
- Server configuration in `.deploy.json` (auto-generated during first deployment)

## Docker Development

Build and run with Docker:

```bash
docker-compose up --build
```

## Production Deployment

Slimlytics includes a smart deployment system that handles Docker builds, server setup, and zero-downtime updates.

### Prerequisites

- Docker Hub account (free tier works)
- Ubuntu/Debian server (tested on Ubuntu 20.04+)
- SSH access to your server
- Domain name pointing to your server (for SSL)

### Initial Setup

1. **Configure Docker Hub**:
   ```bash
   docker login
   # Username: your-docker-username
   # Password: your-docker-password
   ```
   
2. **Update deploy.js with your Docker Hub username** (line 14):
   ```javascript
   const DOCKER_REGISTRY = 'your-username';  // Default is 'xhenxhe'
   ```

3. **Run the deployment**:
   ```bash
   ./deploy
   # or
   npm run deploy
   # or
   bun run deploy
   ```

   On first run, the script will:
   - Prompt for server IP/domain
   - Prompt for SSH username (default: root)
   - Prompt for application domain (for SSL)
   - Install Docker on the server if needed
   - Build and push Docker image to Docker Hub
   - Set up Caddy for automatic SSL
   - Deploy the application

### Updating an Existing Deployment

Simply run the deploy script again:
```bash
./deploy
```

The script will:
- Backup your database to `prod_db_backups/`
- Build and push new Docker image
- Deploy with zero downtime
- Verify service health

### Server Requirements

- **Minimum**: 1GB RAM, 1 CPU (works on $5 DigitalOcean droplet)
- **Recommended**: 2GB RAM, 2 CPUs for better performance
- **Storage**: ~500MB for application + database growth

### Configuration

The deployment configuration is stored in `.deploy.json` (created on first run):
```json
{
  "servers": {
    "production": {
      "host": "your-server-ip",
      "username": "root",
      "domain": "analytics.yourdomain.com",
      "path": "/opt/slimlytics"
    }
  }
}
```

### SSL/HTTPS

SSL certificates are automatically provisioned and renewed by Caddy. Just ensure:
- Your domain points to the server
- Ports 80 and 443 are open
- No other services are using these ports

### Database Management

- **Location on server**: `/opt/slimlytics/data/slimlytics.db`
- **Automatic backups**: Created before each deployment in `prod_db_backups/`
- **Manual backup**: 
  ```bash
  scp root@your-server:/opt/slimlytics/data/slimlytics.db ./backup.db
  ```

### Monitoring

#### Quick Debugging with Server Logs Script

Use the built-in server logs helper for comprehensive debugging:
```bash
./server-logs.js
# or
npm run logs
# or
bun run logs
```

This interactive script will:
- Show container status and health
- Display recent application and web server logs
- Run internal health checks
- Verify database and environment files
- Provide interactive options for:
  - Following live logs (Slimlytics, Caddy, or both)
  - Restarting containers
  - Checking configurations
  - Testing database connections
  - Viewing system resources (disk, memory)
  - Showing masked environment variables

#### Manual Monitoring

Alternatively, check service status manually on the server:
```bash
ssh root@your-server
cd /opt/slimlytics
docker compose ps
docker compose logs -f
```

### Troubleshooting Deployment

#### 502 Bad Gateway Errors

If you encounter a 502 error after deployment, run the server logs script:
```bash
./server-logs.js
```

Common causes and fixes:
1. **Container not running**: Check container status, restart if needed
2. **Application crash on startup**: Review Slimlytics logs for errors
3. **Missing environment file**: Ensure `.env` exists on server
4. **Database not initialized**: Run `docker compose exec slimlytics bun run db:init`
5. **Port mismatch**: Verify app is listening on port 3000
6. **Network issues**: Ensure containers are on the same Docker network

#### Other Common Issues

**Build fails:**
- Ensure you're logged into Docker Hub: `docker login`
- Check Docker Hub rate limits

**Connection issues:**
- Verify SSH key is set up: `ssh-copy-id root@your-server`
- Check firewall allows SSH (port 22)

**SSL not working:**
- Verify domain DNS points to server
- Check Caddy logs: `docker compose logs caddy`
- Ensure ports 80/443 are accessible

**Application errors:**
- Use `./server-logs.js` for comprehensive debugging
- Check logs: `docker compose logs slimlytics`
- Verify `.env` file exists on server
- Check database permissions

**Quick recovery steps:**
```bash
ssh root@your-server
cd /opt/slimlytics
docker compose down
docker compose up -d
docker compose logs -f
```

## Features

- **Real-time Analytics**: WebSocket-based live dashboard updates
- **Geographic Insights**: Track visitor locations using MaxMind GeoIP2
  - Automatic GeoIP lookups for all tracked events
  - Country, city, region, timezone, and ISP data
  - Smart auto-update system for GeoIP databases
  - Dashboard shows top countries with flags and top cities
- **Privacy-Focused**: IP addresses are hashed, no personal data stored
- **Lightweight**: Runs on minimal resources (perfect for $5 VPS)
- **Multi-Site Support**: Track multiple websites from one dashboard
- **Modern Stack**: Built with Bun for performance, SQLite for simplicity

## Notes

- The dashboard shows real-time updates via WebSocket connection
- IP addresses are hashed for privacy protection
- Geographic data includes country, city, region, timezone, and ASN
- Database file is stored in `./data/` directory
- Dashboard builds to `./dist/` directory
- MaxMind databases auto-update twice weekly when configured
- Location tracking works for both IPv4 and IPv6 addresses
- Private/local IP addresses are detected and labeled appropriately

## Production Best Practices

For production environments:

1. Set up automatic GeoIP updates:
   ```bash
   sudo ./scripts/setup-auto-update.sh  # Choose systemd option
   ```

2. Monitor update status:
   ```bash
   systemctl status slimlytics-maxmind-update.timer
   systemctl list-timers slimlytics-maxmind-update
   ```

3. Set up monitoring alerts for databases older than 7 days

4. Consider using a process manager (pm2, systemd) for the API server

## Troubleshooting

### GeoIP Issues

**Databases not downloading:**
- Verify MaxMind credentials in `.env`
- Check account status at https://www.maxmind.com
- Ensure internet connectivity
- Check logs: `tail data/maxmind/update.log`

**Location data not showing:**
- Ensure databases exist: `ls -la data/maxmind/*.mmdb`
- Run migration if upgrading: `bun run db:migrate`
- Restart API server to load databases
- Check API logs for GeoIP initialization messages

**Auto-updates not working:**
- Verify scheduler is running:
  - Cron: `crontab -l`
  - Systemd: `systemctl status slimlytics-maxmind-update.timer`
  - macOS: `launchctl list | grep slimlytics`
- Check update logs: `tail -50 data/maxmind/update.log`
- Manually test update: `bun run maxmind:update`

For more detailed troubleshooting, see `docs/MAXMIND_UPDATES.md`
