import { defineConfig } from 'vite';
import { resolve } from 'path';
import { copyFileSync, mkdirSync, existsSync, readdirSync, readFileSync } from 'fs';
import { join } from 'path';

function copyDirSync(src, dest) {
  if (!existsSync(dest)) mkdirSync(dest, { recursive: true });
  const entries = readdirSync(src, { withFileTypes: true });
  for (const entry of entries) {
    const srcPath = join(src, entry.name);
    const destPath = join(dest, entry.name);
    if (entry.isDirectory()) {
      copyDirSync(srcPath, destPath);
    } else {
      copyFileSync(srcPath, destPath);
    }
  }
}

export default defineConfig({
  root: './',
  base: './',
  build: {
    outDir: 'dist',
    rollupOptions: {
      input: {
        main: resolve(__dirname, 'index.html'),
      },
    },
  },
  server: {
    port: 3000,
    open: true,
    fs: {
      strict: false,
      allow: ['..'],
    },
  },
  configureServer(server) {
    server.middlewares.use((req, res, next) => {
      if (req.url?.startsWith('/cards/') || req.url?.startsWith('/engine/')) {
        const filePath = resolve(__dirname, '..', req.url);
        console.log(`[Vite Middleware] Serving ${req.url} from ${filePath}`);
        try {
          if (!existsSync(filePath)) {
            console.warn(`[Vite Middleware] File not found: ${filePath}`);
            res.statusCode = 404;
            res.end('File not found');
            return;
          }
          const content = readFileSync(filePath);
          if (req.url.endsWith('.json')) {
            res.setHeader('Content-Type', 'application/json');
          }
          console.log(`[Vite Middleware] Successfully served ${req.url} (${content.length} bytes)`);
          res.end(content);
        } catch (e) {
          console.error(`[Vite Middleware] Error serving ${req.url}:`, e.message);
          res.statusCode = 500;
          res.end('Server error');
        }
      } else {
        next();
      }
    });
  },
  publicDir: false,
  assetsInclude: ['**/*.json'],
  plugins: [
    {
      name: 'copy-assets',
      closeBundle() {
        console.log('Copying assets...');
        // Copy i18n folder to dist/js/i18n
        const i18nSrc = resolve(__dirname, 'js', 'i18n');
        const i18nDest = resolve(__dirname, 'dist', 'js', 'i18n');
        console.log('Copying i18n from', i18nSrc, 'to', i18nDest);
        if (existsSync(i18nSrc)) {
          copyDirSync(i18nSrc, i18nDest);
          console.log('i18n copied successfully');
        } else {
          console.log('i18n source not found:', i18nSrc);
        }
        // Copy img folder to dist/img
        const imgSrc = resolve(__dirname, 'img');
        const imgDest = resolve(__dirname, 'dist', 'img');
        console.log('Copying img from', imgSrc, 'to', imgDest);
        if (existsSync(imgSrc)) {
          copyDirSync(imgSrc, imgDest);
          console.log('img copied successfully');
        } else {
          console.log('img source not found:', imgSrc);
        }
        // Copy decks folder to dist/decks
        const decksSrc = resolve(__dirname, 'decks');
        const decksDest = resolve(__dirname, 'dist', 'decks');
        console.log('Copying decks from', decksSrc, 'to', decksDest);
        if (existsSync(decksSrc)) {
          copyDirSync(decksSrc, decksDest);
          console.log('decks copied successfully');
        } else {
          console.log('decks source not found:', decksSrc);
        }
        // Copy cards folder to dist/cards
        const cardsSrc = resolve(__dirname, '..', 'cards');
        const cardsDest = resolve(__dirname, 'dist', 'cards');
        console.log('Copying cards from', cardsSrc, 'to', cardsDest);
        if (existsSync(cardsSrc)) {
          copyDirSync(cardsSrc, cardsDest);
          console.log('cards copied successfully');
        } else {
          console.log('cards source not found:', cardsSrc);
        }
        console.log('Asset copying complete');
      },
    },
  ],
});
