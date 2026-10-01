# syntax=docker/dockerfile:1.7
FROM node:24-bookworm-slim AS tracker
WORKDIR /src/tracker
COPY tracker/package.json tracker/package-lock.json* ./
RUN npm ci
COPY tracker/ ./
ARG RUN_CHECKS=true
RUN if [ "$RUN_CHECKS" != false ]; then npm run test -- --run; fi \
    && npm run build

FROM node:24-bookworm-slim AS frontend-builder
WORKDIR /src/frontend
COPY frontend/package.json frontend/package-lock.json* ./
RUN npm ci
COPY frontend/ ./
COPY --from=tracker /src/tracker/dist/slimlytics.js ./static/tracker.js
COPY --from=tracker /src/tracker/dist/slimlytics.js ./static/s.js
COPY --from=tracker /src/tracker/dist/slimlytics.js ./src/lib/server/generated/tracker.iife.txt
ARG PUBLIC_API_BASE_URL=/api
ENV PUBLIC_API_BASE_URL=$PUBLIC_API_BASE_URL
ENV PUBLIC_DEMO_MODE=false
ARG RUN_CHECKS=true
RUN if [ "$RUN_CHECKS" != false ]; then npm run check && npm run test -- --run; fi \
    && npm run build

FROM node:24-bookworm-slim AS runtime-deps
WORKDIR /app
COPY frontend/package.json frontend/package-lock.json ./
RUN npm ci --omit=dev && npm cache clean --force

# Development target for compose.dev.yaml: the Vite dev server with HMR. Source is bind-mounted
# at runtime and node_modules lives in a named volume, so nothing is copied here.
FROM node:24-bookworm-slim AS dev
ENV PUBLIC_API_BASE_URL=/api
ENV PUBLIC_DEMO_MODE=false
WORKDIR /app
# Pre-create the node_modules mount point so the named volume is owned by the node user.
RUN mkdir -p /app/node_modules && chown -R node:node /app
USER node
EXPOSE 3000

FROM node:24-bookworm-slim AS runtime
ENV NODE_ENV=production
ENV HOST=0.0.0.0
ENV PORT=3000
WORKDIR /app
COPY --from=frontend-builder /src/frontend/build ./build
COPY --from=frontend-builder /src/frontend/package.json ./package.json
COPY --from=runtime-deps /app/node_modules ./node_modules
USER node
EXPOSE 3000
CMD ["node", "build"]
