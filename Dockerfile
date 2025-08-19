# Multi-stage Dockerfile for Slimlytics

# Build stage
FROM oven/bun:1-alpine AS builder

WORKDIR /app

# Install Node.js for 11ty build
RUN apk add --no-cache nodejs npm

# Copy package files
COPY package*.json ./
COPY bun.lockb* ./

# Install all dependencies
RUN bun install

# Copy all source files
COPY src/ ./src/
COPY api/ ./api/
COPY scripts/ ./scripts/
COPY .eleventy.js* ./

# Build the dashboard
RUN npm run build

# Production stage - Slimlytics app
FROM oven/bun:1-alpine AS app

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

# Copy application code from builder
COPY --from=builder /app/dist ./dist
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
ENV HOST=0.0.0.0

# Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
  CMD curl -f http://localhost:3000/health || exit 1

# Expose port
EXPOSE 3000

# Run the application
CMD ["sh", "-c", "bun run db:init && bun src/api/server.js"]

# Caddy stage for SSL/reverse proxy
FROM caddy:2-alpine AS caddy

# Copy default Caddyfile (will be overridden by volume mount in production)
COPY Caddyfile /etc/caddy/Caddyfile
