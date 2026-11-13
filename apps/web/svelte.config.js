import adapter from '@sveltejs/adapter-static';
import { vitePreprocess } from '@sveltejs/vite-plugin-svelte';

/** @type {import('@sveltejs/kit').Config} */
export default {
  preprocess: vitePreprocess(),
  kit: {
    // Single-page app: every route falls back to index.html. The API lives on
    // another host behind a session cookie, so there is nothing to render on a
    // server and no SEO surface to serve.
    adapter: adapter({ fallback: 'index.html', strict: false }),
    // Both apps read the one .env at the repository root.
    env: { dir: '../../' },
    alias: { $components: 'src/lib/components' }
  }
};
