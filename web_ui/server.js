import express from 'express';
import path from 'path';
import cors from 'cors';
import fs from 'fs';
import { fileURLToPath } from 'url';

const CACHE_DURATION = 7 * 24 * 60 * 60 * 1000; // 7 days for images

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

const app = express();
const PORT = 3000;
const RUST_API_URL = 'http://127.0.0.1:8080';


app.use(cors());
app.use(express.json());
// Serve the built dist folder
const distPath = path.resolve(__dirname, 'dist');
app.use(express.static(distPath));
// Also serve from assets folder for Vite build output
app.use('/assets', express.static(path.resolve(__dirname, 'dist', 'assets')));
// Serve cards directory for static card database access
app.use('/cards', express.static(path.resolve(__dirname, '..', 'cards')));
// Serve engine assets that the frontend may fetch directly
app.use('/engine', express.static(path.resolve(__dirname, '..', 'engine')));
// Serve img folder for card images and icons with caching
app.use('/img', express.static(path.resolve(__dirname, 'img'), {
    maxAge: CACHE_DURATION,
    immutable: true
}));

// Proxy requests to Rust backend
app.get('/api/game-state', async (req, res) => {
    try {
        const response = await fetch(`${RUST_API_URL}/api/game-state`);
        const data = await response.json();
        res.json(data);
    } catch (error) {
        console.error('Error proxying to Rust API:', error);
        res.status(500).json({ error: 'Failed to get game state' });
    }
});

app.get('/api/actions', async (req, res) => {
    try {
        const response = await fetch(`${RUST_API_URL}/api/actions`);
        const data = await response.json();
        res.json(data);
    } catch (error) {
        console.error('Error proxying to Rust API:', error);
        res.status(500).json({ error: 'Failed to get actions' });
    }
});

app.post('/api/execute-action', async (req, res) => {
    try {
        const response = await fetch(`${RUST_API_URL}/api/execute-action`, {
            method: 'POST',
            headers: {
                'Content-Type': 'application/json',
            },
            body: JSON.stringify(req.body)
        });
        const data = await response.json();
        res.json(data);
    } catch (error) {
        console.error('Error proxying to Rust API:', error);
        res.status(500).json({ error: 'Failed to execute action' });
    }
});

app.post('/api/init', async (req, res) => {
    try {
        const body = req.body || {};
        const response = await fetch(`${RUST_API_URL}/api/init`, {
            method: 'POST',
            headers: {
                'Content-Type': 'application/json',
            },
            body: JSON.stringify(body)
        });
        const text = await response.text();
        const data = JSON.parse(text);
        res.json(data);
    } catch (error) {
        console.error('Error proxying to Rust API (/api/init):', error);
        res.status(500).json({ error: 'Failed to initialize game' });
    }
});

// Alias for old UI compatibility
app.post('/api/reset', async (req, res) => {
    try {
        const response = await fetch(`${RUST_API_URL}/api/init`, {
            method: 'POST',
            headers: {
                'Content-Type': 'application/json',
            },
        });
        const data = await response.json();
        res.json(data);
    } catch (error) {
        console.error('Error proxying to Rust API:', error);
        res.status(500).json({ error: 'Failed to reset game' });
    }
});

// Serve deck files
app.get('/api/get_decks', async (req, res) => {
    try {
        const decksPath = path.resolve(__dirname, 'decks');
        const files = fs.readdirSync(decksPath).filter(f => f.endsWith('.txt'));

        const decks = files.map(file => {
            const filePath = path.join(decksPath, file);
            const content = fs.readFileSync(filePath, 'utf-8');
            const lines = content.split('\n').filter(l => l.trim());
            const cardCount = lines.reduce((sum, line) => {
                const match = line.match(/x (\d+)$/);
                return sum + (match ? parseInt(match[1]) : 1);
            }, 0);

            return {
                id: file.replace('.txt', ''),
                name: file.replace('.txt', '').replace(/_/g, ' ').replace(/\b\w/g, l => l.toUpperCase()),
                card_count: cardCount,
                content: content
            };
        });

        res.json({ success: true, decks });
    } catch (error) {
        console.error('Error reading deck files:', error);
        res.status(500).json({ error: 'Failed to read deck files' });
    }
});

// Set deck for player (simplified - just returns success for now)
app.post('/api/set_deck', async (req, res) => {
    res.json({ success: true });
});

// Get test deck (simplified - returns aqours_cup content)
app.get('/api/get_test_deck', async (req, res) => {
    try {
        const deckPath = path.join(__dirname, 'decks', 'aqours_cup.txt');
        const content = fs.readFileSync(deckPath, 'utf-8');
        res.json({ success: true, content });
    } catch (error) {
        console.error('Error loading test deck:', error);
        res.status(500).json({ error: 'Failed to load test deck' });
    }
});

// Get random deck (simplified - returns random deck content)
app.get('/api/get_random_deck', async (req, res) => {
    try {
        const decksPath = path.join(__dirname, 'decks');
        const files = fs.readdirSync(decksPath).filter(f => f.endsWith('.txt'));
        const randomFile = files[Math.floor(Math.random() * files.length)];
        const deckPath = path.join(decksPath, randomFile);
        const content = fs.readFileSync(deckPath, 'utf-8');
        res.json({ success: true, content });
    } catch (error) {
        console.error('Error loading random deck:', error);
        res.status(500).json({ error: 'Failed to load random deck' });
    }
});

// Proxy status endpoint to Rust backend
app.get('/api/status', async (req, res) => {
    try {
        const response = await fetch(`${RUST_API_URL}/api/status`);
        const data = await response.json();
        res.json(data);
    } catch (error) {
        console.error('Error proxying to Rust API:', error);
        res.status(500).json({ error: 'Failed to get status' });
    }
});

app.post('/api/set_ai', async (req, res) => {
    try {
        const response = await fetch(`${RUST_API_URL}/api/set_ai`, {
            method: 'POST',
            headers: {
                'Content-Type': 'application/json',
            },
            body: JSON.stringify(req.body)
        });
        const data = await response.json();
        res.json(data);
    } catch (error) {
        console.error('Error proxying to Rust API:', error);
        res.status(500).json({ error: 'Failed to set AI mode' });
    }
});

app.post('/api/exec', async (req, res) => {
    try {
        const response = await fetch(`${RUST_API_URL}/api/exec`, {
            method: 'POST',
            headers: {
                'Content-Type': 'application/json',
            },
            body: JSON.stringify(req.body)
        });
        const data = await response.json();
        res.json(data);
    } catch (error) {
        console.error('Error proxying to Rust API:', error);
        res.status(500).json({ error: 'Failed to execute code' });
    }
});

app.post('/api/rooms/create', async (req, res) => {
    try {
        const response = await fetch(`${RUST_API_URL}/api/rooms/create`, {
            method: 'POST',
            headers: {
                'Content-Type': 'application/json',
            },
            body: JSON.stringify(req.body)
        });
        const data = await response.json();
        res.json(data);
    } catch (error) {
        console.error('Error proxying to Rust API:', error);
        res.status(500).json({ error: 'Failed to create room' });
    }
});

app.post('/api/rooms/join', async (req, res) => {
    try {
        const response = await fetch(`${RUST_API_URL}/api/rooms/join`, {
            method: 'POST',
            headers: {
                'Content-Type': 'application/json',
            },
            body: JSON.stringify(req.body)
        });
        const data = await response.json();
        res.json(data);
    } catch (error) {
        console.error('Error proxying to Rust API:', error);
        res.status(500).json({ error: 'Failed to join room' });
    }
});

app.post('/api/rooms/leave', async (req, res) => {
    try {
        const response = await fetch(`${RUST_API_URL}/api/rooms/leave`, {
            method: 'POST',
            headers: {
                'Content-Type': 'application/json',
            },
            body: JSON.stringify(req.body)
        });
        const data = await response.json();
        res.json(data);
    } catch (error) {
        console.error('Error proxying to Rust API:', error);
        res.status(500).json({ error: 'Failed to leave room' });
    }
});

app.get('/api/rooms/list', async (req, res) => {
    try {
        const response = await fetch(`${RUST_API_URL}/api/rooms/list`);
        const data = await response.json();
        res.json(data);
    } catch (error) {
        console.error('Error proxying to Rust API:', error);
        res.status(500).json({ error: 'Failed to list rooms' });
    }
});

app.get('/api/get_card_registry', async (req, res) => {
    try {
        const cardsPath = path.resolve(__dirname, '..', 'cards', 'cards.json');
        const content = fs.readFileSync(cardsPath, 'utf-8');
        res.json(JSON.parse(content));
    } catch (error) {
        console.error('Error loading card registry:', error);
        res.status(500).json({ error: 'Failed to load card registry' });
    }
});

// Generic proxy for any /api/* routes not handled locally (undo, redo, debug, etc.)
app.all('/api/*', async (req, res) => {
    try {
        const targetUrl = `${RUST_API_URL}${req.originalUrl}`;
        const fetchOptions = {
            method: req.method,
            headers: { 'Content-Type': 'application/json' },
        };
        if (req.method !== 'GET' && req.method !== 'HEAD') {
            fetchOptions.body = JSON.stringify(req.body);
        }
        const response = await fetch(targetUrl, fetchOptions);
        const text = await response.text();
        res.status(response.status).type('application/json').send(text);
    } catch (error) {
        console.error(`Error proxying ${req.method} ${req.originalUrl} to Rust API:`, error);
        res.status(500).json({ error: `Failed to proxy ${req.originalUrl}` });
    }
});

// Fallback to index.html for SPA routing (must be last)
app.get('*', (req, res) => {
    res.sendFile(path.join(distPath, 'index.html'));
});

app.listen(PORT, () => {
    console.log(`Web server running on http://localhost:${PORT}`);
    console.log(`Proxying API requests to Rust backend at ${RUST_API_URL}`);
});
