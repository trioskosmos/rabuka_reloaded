const express = require('express');
const { exec } = require('child_process');
const path = require('path');
const cors = require('cors');

const app = express();
const PORT = 8080;

app.use(cors());
app.use(express.json());
app.use(express.static(__dirname));

// Get game state
app.get('/api/game-state', (req, res) => {
    const enginePath = path.join(__dirname, '..', 'engine', 'target_build', 'debug', 'rabuka_engine.exe');
    exec(`"${enginePath}" get-state`, (error, stdout, stderr) => {
        if (error) {
            console.error(`Error: ${error.message}`);
            return res.status(500).json({ error: 'Failed to get game state' });
        }
        try {
            const state = JSON.parse(stdout);
            res.json(state);
        } catch (e) {
            res.status(500).json({ error: 'Invalid JSON output' });
        }
    });
});

// Get possible actions
app.get('/api/actions', (req, res) => {
    const enginePath = path.join(__dirname, '..', 'engine', 'target_build', 'debug', 'rabuka_engine.exe');
    exec(`"${enginePath}" get-actions`, (error, stdout, stderr) => {
        if (error) {
            console.error(`Error: ${error.message}`);
            return res.status(500).json({ error: 'Failed to get actions' });
        }
        try {
            const actions = JSON.parse(stdout);
            res.json(actions);
        } catch (e) {
            res.status(500).json({ error: 'Invalid JSON output' });
        }
    });
});

// Execute action
app.post('/api/execute-action', (req, res) => {
    const { action_index } = req.body;
    const enginePath = path.join(__dirname, '..', 'engine', 'target_build', 'debug', 'rabuka_engine.exe');
    exec(`"${enginePath}" execute-action ${action_index}`, (error, stdout, stderr) => {
        if (error) {
            console.error(`Error: ${error.message}`);
            return res.status(500).json({ error: 'Failed to execute action' });
        }
        // Return updated game state
        exec(`"${enginePath}" get-state`, (error, stdout, stderr) => {
            if (error) {
                console.error(`Error: ${error.message}`);
                return res.status(500).json({ error: 'Failed to get game state' });
            }
            try {
                const state = JSON.parse(stdout);
                res.json(state);
            } catch (e) {
                res.status(500).json({ error: 'Invalid JSON output' });
            }
        });
    });
});

app.listen(PORT, () => {
    console.log(`Server running on http://localhost:${PORT}`);
});
