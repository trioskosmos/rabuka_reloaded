const express = require('express');
const http = require('http');
const path = require('path');
const cors = require('cors');

const app = express();
const PORT = 3000;
const RUST_API_URL = 'http://127.0.0.1:8080';

// Log the current directory for debugging
console.log('Current directory:', __dirname);
console.log('Resolved dist path:', path.resolve(__dirname, '..', 'web_ui', 'dist'));

app.use(cors());
app.use(express.json());
// Serve the built dist folder from web_ui
const distPath = path.resolve(__dirname, '..', 'web_ui', 'dist');
app.use(express.static(distPath));
// Also serve from assets folder for Vite build output
app.use('/assets', express.static(path.resolve(__dirname, '..', 'web_ui', 'dist', 'assets')));

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
        const fs = require('fs');
        const decksPath = path.resolve(__dirname, '..', 'web_ui', 'decks');
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
    try {
        // For now, just return success - the Rust backend doesn't have this endpoint yet
        // In a full implementation, this would proxy to the Rust backend
        console.log('Set deck request:', req.body);
        res.json({ success: true });
    } catch (error) {
        console.error('Error setting deck:', error);
        res.status(500).json({ error: 'Failed to set deck' });
    }
});

// Get test deck (simplified - returns aqours_cup content)
app.get('/api/get_test_deck', async (req, res) => {
    try {
        const fs = require('fs');
        const deckPath = path.join(__dirname, '..', 'web_ui', 'decks', 'aqours_cup.txt');
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
        const fs = require('fs');
        const decksPath = path.join(__dirname, '..', 'web_ui', 'decks');
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

// Stub endpoints for old UI compatibility
app.get('/api/status', async (req, res) => {
    res.json({ status: 'ok', version: '1.0.0' });
});

app.post('/api/rooms/leave', async (req, res) => {
    res.json({ success: true });
});

app.post('/api/rooms/create', async (req, res) => {
    res.json({ success: true, room_id: 'local', session: {} });
});

app.post('/api/rooms/join', async (req, res) => {
    res.json({ success: true, room_id: 'local', session: {} });
});

app.get('/api/rooms/list', async (req, res) => {
    res.json({ success: true, rooms: [] });
});

app.get('/api/get_card_registry', async (req, res) => {
    try {
        const fs = require('fs');
        const cardsPath = path.resolve(__dirname, '..', 'cards', 'cards.json');
        const content = fs.readFileSync(cardsPath, 'utf-8');
        res.json(JSON.parse(content));
    } catch (error) {
        console.error('Error loading card registry:', error);
        res.status(500).json({ error: 'Failed to load card registry' });
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
