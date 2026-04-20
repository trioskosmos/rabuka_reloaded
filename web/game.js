class GameScene extends Phaser.Scene {
    constructor() {
        super('GameScene');
        this.gameState = null;
        this.actions = [];
    }

    preload() {
        // Load card images if needed
        // For now, we'll use colored rectangles
    }

    create() {
        // Create game zones
        this.createZones();
        
        // Load initial game state from Rust engine
        this.loadGameState();
    }

    createZones() {
        const width = this.scale.width;
        const height = this.scale.height;

        // Player 1 zones (bottom)
        this.createZone('player1-hand', 100, height - 150, 400, 120, 0x4a5568, 'Player 1 Hand');
        this.createZone('player1-energy', 520, height - 150, 200, 120, 0x2d3748, 'Player 1 Energy');
        this.createZone('player1-stage-left', 100, height - 300, 120, 140, 0x1a202c, 'Stage Left');
        this.createZone('player1-stage-center', 230, height - 300, 120, 140, 0x1a202c, 'Stage Center');
        this.createZone('player1-stage-right', 360, height - 300, 120, 140, 0x1a202c, 'Stage Right');
        this.createZone('player1-live-zone', 490, height - 300, 230, 140, 0x2c5282, 'Live Card Zone');

        // Player 2 zones (top)
        this.createZone('player2-hand', 100, 30, 400, 120, 0x4a5568, 'Player 2 Hand');
        this.createZone('player2-energy', 520, 30, 200, 120, 0x2d3748, 'Player 2 Energy');
        this.createZone('player2-stage-left', 100, 180, 120, 140, 0x1a202c, 'Stage Left');
        this.createZone('player2-stage-center', 230, 180, 120, 140, 0x1a202c, 'Stage Center');
        this.createZone('player2-stage-right', 360, 180, 120, 140, 0x1a202c, 'Stage Right');
        this.createZone('player2-live-zone', 490, 180, 230, 140, 0x2c5282, 'Live Card Zone');

        // Center info area
        this.add.text(width / 2 - 150, height / 2 - 20, 'Game State', {
            fontSize: '24px',
            color: '#ffffff'
        });
    }

    createZone(name, x, y, width, height, color, label) {
        // Create zone background
        const graphics = this.add.graphics();
        graphics.fillStyle(color, 1);
        graphics.fillRect(x, y, width, height);
        graphics.lineStyle(2, 0xe94560, 1);
        graphics.strokeRect(x, y, width, height);

        // Add zone label
        const labelX = x + 10;
        const labelY = y + 10;
        this.add.text(labelX, labelY, label, {
            fontSize: '12px',
            color: '#ffffff',
            fontStyle: 'bold'
        });

        // Store zone reference
        this[name] = {
            graphics: graphics,
            x: x,
            y: y,
            width: width,
            height: height,
            cards: []
        };
    }

    async loadGameState() {
        try {
            const response = await fetch('http://127.0.0.1:8080/api/game-state');
            this.gameState = await response.json();
            this.updateDisplay();
            this.loadActions();
        } catch (error) {
            console.error('Failed to load game state:', error);
            // Use mock data if server is not available
            this.gameState = {
                turn: 1,
                phase: 'ActivePhase',
                player1: {
                    hand: [{ card_no: 'mock', name: 'Card 1', type: 'Member' }],
                    energy: [{ card_no: 'mock', name: 'Energy 1', type: 'Energy' }],
                    stage: { left_side: null, center: null, right_side: null },
                    live_zone: { cards: [] }
                },
                player2: {
                    hand: [{ card_no: 'mock', name: 'Card 1', type: 'Member' }],
                    energy: [{ card_no: 'mock', name: 'Energy 1', type: 'Energy' }],
                    stage: { left_side: null, center: null, right_side: null },
                    live_zone: { cards: [] }
                }
            };
            this.updateDisplay();
            this.generateActions();
        }
    }

    updateDisplay() {
        // Update zones with current game state
        this.displayCards('player1-hand', this.gameState.player1.hand.cards);
        this.displayCards('player1-energy', this.gameState.player1.energy.cards);
        this.displayCards('player2-hand', this.gameState.player2.hand.cards);
        this.displayCards('player2-energy', this.gameState.player2.energy.cards);
        
        // Display stage cards
        this.displayStageCard('player1-stage-left', this.gameState.player1.stage.left_side);
        this.displayStageCard('player1-stage-center', this.gameState.player1.stage.center);
        this.displayStageCard('player1-stage-right', this.gameState.player1.stage.right_side);
        this.displayStageCard('player2-stage-left', this.gameState.player2.stage.left_side);
        this.displayStageCard('player2-stage-center', this.gameState.player2.stage.center);
        this.displayStageCard('player2-stage-right', this.gameState.player2.stage.right_side);
    }

    displayCards(zoneName, cards) {
        const zone = this[zoneName];
        if (!zone) return;

        // Clear existing cards in zone
        zone.cards.forEach(card => card.destroy());
        zone.cards = [];

        // Display new cards
        const cardWidth = 60;
        const cardHeight = 80;
        const padding = 5;
        const startX = zone.x + padding;
        const startY = zone.y + padding + 20; // +20 for label

        cards.forEach((card, index) => {
            const cardX = startX + index * (cardWidth + padding);
            this.drawCard(cardX, startY, cardWidth, cardHeight, card, zone);
        });
    }

    displayStageCard(zoneName, card) {
        const zone = this[zoneName];
        if (!zone) return;

        // Clear existing card in zone
        zone.cards.forEach(c => c.destroy());
        zone.cards = [];

        if (card) {
            const cardWidth = 100;
            const cardHeight = 140;
            const cardX = zone.x + 10;
            const cardY = zone.y + 10;
            this.drawCard(cardX, cardY, cardWidth, cardHeight, card, zone);
        }
    }

    drawCard(x, y, width, height, card, zone) {
        const cardGraphics = this.add.graphics();
        cardGraphics.fillStyle(0x3182ce, 1);
        cardGraphics.fillRect(x, y, width, height);
        cardGraphics.lineStyle(2, 0xffffff, 1);
        cardGraphics.strokeRect(x, y, width, height);

        const cardText = this.add.text(x + 5, y + 5, card.name || card, {
            fontSize: '10px',
            color: '#ffffff',
            wordWrap: { width: width - 10 }
        });

        zone.cards.push(cardGraphics);
        zone.cards.push(cardText);
    }

    async loadActions() {
        try {
            const response = await fetch('http://127.0.0.1:8080/api/actions');
            const data = await response.json();
            this.actions = data.actions;
            this.updateActionPanel();
        } catch (error) {
            console.error('Failed to load actions:', error);
            // Use mock actions if server is not available
            this.actions = [
                { description: 'Play card from hand to stage', action_type: 'play_member_to_stage' },
                { description: 'Activate energy', action_type: 'activate_energy' },
                { description: 'Draw card', action_type: 'draw_card' },
                { description: 'Pass turn', action_type: 'pass_turn' }
            ];
            this.updateActionPanel();
        }
    }

    updateActionPanel() {
        const actionList = document.getElementById('action-list');
        actionList.innerHTML = '';

        this.actions.forEach((action, index) => {
            const actionItem = document.createElement('div');
            actionItem.className = 'action-item';
            actionItem.innerHTML = `
                <span class="action-index">[${index}]</span>
                <span class="action-description">${action.description}</span>
            `;
            actionItem.onclick = () => this.executeAction(index);
            actionList.appendChild(actionItem);
        });
    }

    async executeAction(index) {
        try {
            const response = await fetch('http://127.0.0.1:8080/api/execute-action', {
                method: 'POST',
                headers: {
                    'Content-Type': 'application/json',
                },
                body: JSON.stringify({ action_index: index })
            });
            
            if (response.ok) {
                // Reload game state after action
                await this.loadGameState();
            } else {
                console.error('Failed to execute action');
            }
        } catch (error) {
            console.error('Failed to execute action:', error);
        }
    }
}

const config = {
    type: Phaser.AUTO,
    width: 720,
    height: 600,
    parent: 'phaser-game',
    backgroundColor: '#1a1a2e',
    scene: [GameScene]
};

const game = new Phaser.Game(config);
