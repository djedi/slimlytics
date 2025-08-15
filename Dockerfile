FROM oven/bun:1-alpine

WORKDIR /app

COPY package.json ./
RUN bun install --production

COPY . .

RUN bun run build

EXPOSE 3000 8080

CMD ["sh", "-c", "bun run db:init && bun src/api/server.js"]