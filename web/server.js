const express = require('express');
const http = require('http');
const path = require('path');
const cors = require('cors');

const app = express();
const PORT = 3000;
const RUST_API_URL = 'http://127.0.0.1:8080';

app.use(cors());
app.use(express.json());
app.use(express.static(__dirname));

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

app.listen(PORT, () => {
    console.log(`Web server running on http://localhost:${PORT}`);
    console.log(`Proxying API requests to Rust backend at ${RUST_API_URL}`);
});
