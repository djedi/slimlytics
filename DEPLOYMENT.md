# Slimlytics Deployment Guide

## Overview

Slimlytics uses a dynamic configuration system that automatically detects whether it's running in development or production and sets the API URL accordingly.

## Development Environment

When running on localhost, 127.0.0.1, or local network IPs (192.168.x.x):
- Frontend: http://localhost:8080
- API: http://localhost:3000

## Production Environment

When deployed to a production domain:
- The frontend JavaScript automatically uses the same domain for API calls
- This assumes the API is served from the same domain (typically behind a reverse proxy)

## Docker Deployment

For a typical Docker deployment on a $5 DigitalOcean droplet:

```dockerfile
# Example Dockerfile
FROM oven/bun:latest

WORKDIR /app
COPY . .

RUN bun install
RUN bun run build

EXPOSE 3000 8080

CMD ["bun", "run", "start"]
```

## Nginx Configuration

Example Nginx configuration for production:

```nginx
server {
    listen 80;
    server_name yourdomain.com;

    # Serve static dashboard files
    location / {
        root /app/dist;
        try_files $uri $uri/ /index.html;
    }

    # Proxy API requests
    location /api {
        proxy_pass http://localhost:3000;
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection 'upgrade';
        proxy_set_header Host $host;
        proxy_cache_bypass $http_upgrade;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
    }

    # Proxy tracking endpoint
    location /track {
        proxy_pass http://localhost:3000;
        proxy_http_version 1.1;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
    }
}
```

## Environment Variables

Set these environment variables in production:

```bash
# API Port (default: 3000)
PORT=3000

# Host (default: localhost)
HOST=0.0.0.0

# Salt for IP hashing
SALT=your-random-salt-here
```

## Configuration Details

The configuration system (`/js/config.js`) automatically:

1. Detects the current hostname
2. If hostname is localhost/127.0.0.1/192.168.x.x:
   - Uses port 3000 for API calls
3. If hostname is anything else (production domain):
   - Uses the same domain/port for API calls
   - Assumes API is proxied through the same domain

This means you don't need to change any configuration when moving from development to production!

## Testing the Configuration

Open `/test-config.html` in your browser to verify the configuration is working correctly. It will show:
- Current environment details
- Configured API URL
- All API endpoints

## Troubleshooting

1. **API calls failing in production**: Ensure your reverse proxy (like Nginx) is correctly routing `/api/*` and `/track` to the Bun server on port 3000.

2. **CORS errors**: The API already includes CORS headers for development. In production with a reverse proxy, CORS shouldn't be an issue as everything is on the same domain.

3. **Wrong API URL detected**: Check the hostname detection in `/js/config.js`. You may need to add additional hostname patterns for your specific deployment.