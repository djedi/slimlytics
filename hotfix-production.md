# Production Hotfix Instructions

## Issues Found:
1. Database schema missing columns (`s.ip_address`, `s.visitor_id`)
2. JavaScript missing functions (`getPathFromUrl`, `formatTimeShort`)

## Quick Fix:

### 1. Fix Database Schema

SSH into your server and run these commands:

```bash
# Connect to the container
docker exec -it slimlytics-app sh

# Run SQLite to fix the schema
sqlite3 /app/data/analytics.db

# Add missing columns to sessions table (if not exists)
ALTER TABLE sessions ADD COLUMN ip_address TEXT;

# Exit SQLite
.exit

# Exit container
exit
```

### 2. Fix JavaScript Functions

The issue is that the new dashboard.js functions weren't deployed. You need to rebuild and redeploy:

```bash
# From your local machine
./deploy.js
```

## Alternative Quick Fix (Temporary):

If you need an immediate fix without rebuilding, you can revert the HTML changes:

1. Remove references to `getPathFromUrl` in the Content section
2. Remove references to `formatTimeShort` in Recent Visitors
3. Change the Recent Visitors query to not include `ip_address`

## Better Solution:

Update the stats.ts file to handle missing columns gracefully:

```typescript
// In getRecentVisitors function, use conditional selection
let query = `
    SELECT DISTINCT
        s.visitor_id,
        COALESCE(s.ip_address, s.visitor_id) as ip_address,
        s.visitor_id as ip_hash,
        e.page_url,
        s.started_at as timestamp,
        s.country,
        s.country_code,
        s.city,
        s.region
    FROM sessions s
    LEFT JOIN events e ON s.session_id = e.session_id AND s.site_id = e.site_id
    WHERE s.site_id = ?
`;
```

## Recommended Action:

1. Run the database migration on production
2. Rebuild and redeploy the application
3. Clear browser cache to get the updated JavaScript files