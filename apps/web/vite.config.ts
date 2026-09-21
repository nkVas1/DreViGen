import react from '@vitejs/plugin-react';
import { defineConfig } from 'vite';
import { VitePWA } from 'vite-plugin-pwa';

/**
 * The web build.
 *
 * Note what is *absent*: the cross-origin isolation headers. Spike S4 found that the OPFS VFS
 * which avoids `SharedArrayBuffer` is also 7.1× faster than the one that needs it, so COOP and
 * COEP are dropped — see ADR 0008. That is a real simplification: cross-origin isolation blocks
 * embedding any resource that has not opted in, which for a genealogy application means maps,
 * archive imagery and external citations.
 */
export default defineConfig({
  plugins: [
    react(),
    VitePWA({
      registerType: 'prompt',
      // A family archive should never update itself out from under someone mid-edit. The
      // prompt strategy asks; `autoUpdate` would not.
      includeAssets: ['favicon.svg', 'favicon-32.png', 'apple-touch-icon.png'],
      manifest: {
        name: 'DreViGen',
        short_name: 'DreViGen',
        description: 'Семейный архив, по которому можно путешествовать как по карте.',
        lang: 'ru',
        start_url: '/',
        display: 'standalone',
        background_color: '#F7F3EB',
        theme_color: '#F7F3EB',
        orientation: 'any',
        icons: [
          { src: '/icon-192.png', sizes: '192x192', type: 'image/png' },
          { src: '/icon-512.png', sizes: '512x512', type: 'image/png' },
          // `maskable` is a separate entry rather than a second purpose on the one above:
          // a launcher that crops to a circle needs the inset drawing, and a launcher that
          // does not would show that one with too much margin.
          {
            src: '/icon-maskable-512.png',
            sizes: '512x512',
            type: 'image/png',
            purpose: 'maskable',
          },
        ],
      },
      workbox: {
        // Fonts and the WASM core are immutable and content-hashed; caching them is the whole
        // point of installing. HTML is not, so it is left to the network.
        globPatterns: ['**/*.{js,css,woff2,wasm}'],
        navigateFallback: null,
      },
      devOptions: { enabled: false },
    }),
  ],
  // No aliases. The workspace packages declare `exports`, and pnpm links them, so Vite
  // resolves them the way a published package would. An alias map here was tried and removed:
  // aliases match by prefix, so '@drevigen/app' swallowed '@drevigen/app/platform' and
  // resolved it to 'index.tsx/platform'.
  build: {
    target: 'es2023',
    sourcemap: true,
    // Fonts are vendored and already subset; inlining them would defeat caching.
    assetsInlineLimit: 4096,
  },
  server: { port: 5173, strictPort: true },
  preview: { port: 4173, strictPort: true },
});
