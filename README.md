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

## Docker Development

Build and run with Docker:

```bash
docker-compose up --build
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

## Production Deployment

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
