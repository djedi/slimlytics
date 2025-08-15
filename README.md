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

### 3. Run in Development Mode

```bash
bun run dev
```

This starts:

- **API Server**: http://localhost:3000 (with hot reload)
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

## Project Structure

```
slimlytics/
├── src/
│   ├── api/          # API server
│   │   └── server.js # Main API endpoints
│   ├── dashboard/    # 11ty dashboard source
│   │   ├── _includes/# Templates
│   │   ├── _data/    # Data files
│   │   └── index.html# Main dashboard page
│   ├── db/           # Database utilities
│   │   ├── init.js   # Schema initialization
│   │   └── queries.js# Database queries
│   └── public/       # Static assets
│       ├── js/       # JavaScript files
│       │   └── tracker.js # Tracking script
│       └── css/      # Stylesheets
├── dist/             # Built dashboard (generated)
├── data/             # SQLite database (generated)
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

- `POST /track` - Track page view
- `GET /api/stats/:siteId` - Get statistics for a site
- `GET /api/realtime/:siteId` - Get real-time visitor count

## Environment Variables

For production, set these environment variables:

- `PORT` - API server port (default: 3000)
- `SALT` - Salt for IP hashing (important for privacy)
- `NODE_ENV` - Set to 'production' for production mode

## Docker Development

Build and run with Docker:

```bash
docker-compose up --build
```

## Notes

- The dashboard auto-refreshes real-time visitor count every 10 seconds
- IP addresses are hashed for privacy
- Database file is stored in `./data/` directory
- Dashboard builds to `./dist/` directory
