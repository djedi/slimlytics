# Production frontend image from a SvelteKit build made in CI (scripts/build-prebuilt.sh).
# Same runtime as docker/frontend.Dockerfile; production dependencies install on the host and
# stay cached until package-lock.json changes.
FROM node:24-bookworm-slim AS runtime-deps
WORKDIR /app
COPY package.json package-lock.json ./
RUN npm ci --omit=dev && npm cache clean --force

FROM node:24-bookworm-slim
ENV NODE_ENV=production
ENV HOST=0.0.0.0
ENV PORT=3000
WORKDIR /app
COPY build ./build
COPY package.json ./package.json
COPY --from=runtime-deps /app/node_modules ./node_modules
USER node
EXPOSE 3000
CMD ["node", "build"]
