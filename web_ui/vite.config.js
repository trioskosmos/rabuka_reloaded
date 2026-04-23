import { defineConfig } from 'vite';
import { resolve } from 'path';
import { copyFileSync, mkdirSync, existsSync, readdirSync } from 'fs';
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
        deck_builder: resolve(__dirname, 'deck_builder.html'),
        deck_converter: resolve(__dirname, 'deck_converter.html'),
        deck_viewer: resolve(__dirname, 'deck_viewer.html'),
        interactive_deck_viewer: resolve(__dirname, 'interactive_deck_viewer.html'),
      },
    },
  },
  server: {
    port: 3000,
    open: true,
  },
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
        console.log('Asset copying complete');
      },
    },
  ],
});
