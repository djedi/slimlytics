# Slim Analytics

The goal of this project is to create a small, open source analytics app. I find Google Analytics too complicated. I like Clicky, but I don't want to pay to track my 10 visitors per day on 10 different websites.

So I want a small, fun, easy to read analytics app that can run in docker on a $5 digital ocean droplet.

## Tech Stack

- Bun: Because it is JavaScript and it is fast - surprisingly close to Go when it comes to performance
- SQLite: Has more power than we give it credit for. Small footprint, easy to back up, and quite sufficient for this use case.
- 11ty: A simple static site generator used to build the dashboard with minimal overhead.
- HTMX: Enables server-driven UI updates with minimal JavaScript by handling AJAX, CSS transitions, and more.
- Alpine.js: Provides lightweight client-side interactivity and state management to enhance the user experience.

## Architecture

The system uses a Bun-based ingest API to efficiently receive tracking data, which is stored in SQLite for lightweight and reliable persistence. The dashboard is built as a static site using 11ty, with HTMX and Alpine.js enabling server-driven UI updates and small client-side enhancements for a responsive and dynamic interface without heavy frontend frameworks.

### Real-time Updates

The dashboard features WebSocket-based real-time updates. When new tracking events are received, the server broadcasts updated statistics to all connected dashboard clients watching that specific site. This provides instant feedback without polling, reducing server load while improving user experience.

## Features

- Lightweight tracking optimized for low-traffic sites.
- Batched inserts to SQLite to reduce write overhead.
- Privacy-friendly design avoiding invasive tracking techniques.
- Daily rollups of analytics data for efficient reporting.
- Retention strategy to manage data size and relevance over time.
- Real-time dashboard updates via WebSockets for instant analytics feedback.
- GeoIP location tracking using MaxMind GeoLite2 databases for visitor geography insights.

## Why

This stack is ideal for small websites running on a $5 DigitalOcean droplet because it combines simplicity, performance, and minimal resource usage. Bun and SQLite provide fast and efficient backend processing, while 11ty, HTMX, and Alpine.js deliver a dynamic yet lightweight frontend without the complexity of heavier frameworks.

## Coding Style

- Put CSS and JS in their own files rather than mixing with html

## Development

### Running the Application

```bash
# Start both API and dashboard development servers
npm run dev

# Or run them separately:
npm run dev:api      # Starts Bun API server with WebSocket support on port 3000
npm run dev:dashboard # Starts 11ty dashboard with live reload on port 8080

# Build the dashboard for production
npm run build

# Initialize the database
npm run db:init

# Run database migration for geo columns (if upgrading)
npm run db:migrate

# Download/update MaxMind GeoIP databases
npm run maxmind:download
```

### API Endpoints

- `POST /track` - Receive tracking events
- `GET /api/sites` - List all sites
- `POST /api/sites` - Create a new site
- `PUT /api/sites/:id` - Update a site
- `DELETE /api/sites/:id` - Delete a site
- `GET /api/stats/:siteId` - Get dashboard statistics
- `GET /api/stats/:siteId/timeseries` - Get time series data for charts
- `GET /api/stats/:siteId/realtime` - Get real-time visitor count
- WebSocket endpoint at `ws://localhost:3000/` - Real-time stats updates

### WebSocket Protocol

The dashboard connects to the WebSocket server and subscribes to updates for a specific site:

1. Client connects to `ws://localhost:3000/`
2. Client sends: `{ "type": "subscribe", "siteId": "site-id-here" }`
3. Server responds: `{ "type": "subscribed", "siteId": "...", "message": "..." }`
4. When new events are tracked, server broadcasts: `{ "type": "stats-update", "siteId": "...", "stats": {...} }`
5. Client sends periodic heartbeat: `{ "type": "ping" }`
6. Server responds: `{ "type": "pong" }`

The dashboard automatically reconnects if the connection is lost and displays a live connection status indicator.

## GeoIP Location Tracking

The application uses MaxMind GeoLite2 databases to provide geographic information about visitors:

### Setup

1. Create a free MaxMind account at https://www.maxmind.com/en/geolite2/signup
2. Add your credentials to `.env`:
   ```
   MAXMIND_ACCOUNT_ID=your_account_id
   MAXMIND_LICENSE_KEY=your_license_key
   ```
3. Download databases: `npm run maxmind:download`

### Features

- **Location Data**: Tracks country, city, region, coordinates, timezone, and ASN for each visitor
- **Dashboard Display**: Shows top countries with flag emojis and top cities
- **Privacy-Friendly**: IP addresses are hashed, only location data is stored
- **Database Storage**: Location data is stored in additional columns in the events table

### Architecture

- **GeoIP Service** (`api/services/geoip.js`): Handles database loading and IP lookups
- **Download Script** (`scripts/download-maxmind.js`): Downloads and extracts MaxMind databases
- **Database Migration** (`scripts/add-geo-columns.js`): Adds geo columns to existing database
- **Integration**: The `/track` endpoint automatically performs GeoIP lookups for incoming events

### Database Updates

MaxMind updates their databases twice weekly (Tuesdays and Fridays). You can download updates up to 30 times per day. To keep data current:

- Run `npm run maxmind:download` periodically (weekly recommended)
- Consider setting up a cron job for automatic updates
- Databases are stored in `data/maxmind/` (gitignored)

## Notes

- Don't do this `await fetch('/api/sites')`; Instead, do this: `await fetch(window.SLIMLYTICS_CONFIG.apiEndpoint("/api/sites"));`
