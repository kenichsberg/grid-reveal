import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vite';

export default defineConfig({
  plugins: [sveltekit()],
  // One .env at the repository root serves both apps, so API and web
  // configuration never drift apart.
  envDir: '../../',
  server: { port: 5173 }
});
