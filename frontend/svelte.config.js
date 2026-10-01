import adapter from '@sveltejs/adapter-node';
import { vitePreprocess } from '@sveltejs/vite-plugin-svelte';

export default {
  preprocess: vitePreprocess(),
  kit: {
    adapter: adapter(),
    // Inline the shared app/marketing stylesheet so server-rendered pages paint without a
    // render-blocking CSS request. The large API reference stylesheet stays external.
    inlineStyleThreshold: 64 * 1024
  }
};
