# Multi-stage Dockerfile for Slimlytics

# Stage 1: Build the dashboard
FROM node:20-alpine AS dashboard-builder

WORKDIR /build

# Copy package files
COPY package*.json ./

# Install dependencies
RUN npm ci

# Copy dashboard source files and public assets
COPY src/dashboard/ ./src/dashboard/
COPY src/public/ ./src/public/
COPY .eleventy.js* ./

# Build the dashboard
RUN npm run build

# Stage 2: API Server
FROM oven/bun:1-alpine AS api

WORKDIR /app

# Install necessary packages
RUN apk add --no-cache \
    sqlite \
    curl

# Copy package files
COPY package*.json ./
COPY bun.lockb* ./

# Install production dependencies only
RUN bun install --production

# Copy application code
COPY src/api/ ./src/api/
COPY src/db/ ./src/db/
COPY src/utils/ ./src/utils/
COPY api/ ./api/
COPY scripts/ ./scripts/

# Create data directory
RUN mkdir -p /app/data

# Set environment variables
ENV NODE_ENV=production
ENV PORT=3000

# Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
  CMD curl -f http://localhost:3000/health || exit 1

# Expose port
EXPOSE 3000

# Run the application
CMD ["sh", "-c", "bun run db:init && bun src/api/server.js"]

# Stage 3: Web Server with Static Files
FROM caddy:2-alpine AS web

WORKDIR /srv

# Copy built dashboard from dashboard-builder stage (includes all static files)
COPY --from=dashboard-builder /build/dist ./

# Default Caddyfile for serving static files (will be overridden in production)
RUN echo ':80 { root * /srv; file_server; try_files {path} /index.html }' > /etc/caddy/Caddyfile