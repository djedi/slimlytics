import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vitest/config';

// Docker dev (compose.dev.yaml) serves Vite behind Caddy, so the browser reaches the HMR
// websocket on Caddy's published port rather than Vite's own. Polling is opt-in for file
// systems that do not deliver change events into the container.
const hmrClientPort = Number(process.env.VITE_HMR_CLIENT_PORT) || undefined;
const usePolling = process.env.VITE_WATCH_POLLING === 'true';

export default defineConfig({
  plugins: [sveltekit()],
  resolve: { conditions: ['browser'] },
  server: {
    hmr: hmrClientPort ? { clientPort: hmrClientPort } : undefined,
    watch: usePolling ? { usePolling: true, interval: 300 } : undefined
  },
  test: { environment: 'jsdom', setupFiles: ['./tests/setup.ts'], restoreMocks: true }
});
