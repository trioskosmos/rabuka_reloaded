class GameScene extends Phaser.Scene {
    constructor() {
        super('GameScene');
        this.gameState = null;
        this.actions = [];
        this.draggedCard = null;
        this.dragOffset = { x: 0, y: 0 };
        this.cardImages = new Map();
    }

    preload() {
        console.log('Phaser: Preload started');
        
        // Preload all card images
        this.load.json('cardsData', '/cards/cards.json')
            .on('complete', () => {
                const cardsData = this.cache.json.get('cardsData');
                console.log('Cards data loaded, preloading images...');
                
                // Preload all card images
                for (const cardNo in cardsData) {
                    const imagePath = `/img/cards_webp/${cardNo}.webp`;
                    this.load.image(cardNo, imagePath);
                }
                
                console.log('Card images preloaded');
            })
            .on('error', (err) => {
                console.error('Failed to load cards data:', err);
            });
    }

    create() {
        console.log('Phaser: Create started');
        
        // Create game zones
        this.createZones();
        
        // Create game info display
        this.createGameInfo();
        
        // Load game state from backend
        this.loadGameState();
        
        console.log('Phaser: Create completed');
    }

    async loadDeck(deckName) {
        try {
            const response = await fetch(`/decks/${deckName}.txt`);
            if (!response.ok) {
                throw new Error(`Failed to load deck: ${response.statusText}`);
            }
            const deckText = await response.text();
            return this.parseDeck(deckText);
        } catch (error) {
            console.error('Failed to load deck:', error);
            console.log('Trying alternative path...');
            // Try alternative path if first fails
            try {
                const altResponse = await fetch(`/decks/${deckName}.txt`);
                if (altResponse.ok) {
                    const deckText = await altResponse.text();
                    return this.parseDeck(deckText);
                }
            } catch (e) {
                console.error('Alternative path also failed:', e);
            }
            return [];
        }
    }

    parseDeck(deckText) {
        const cards = [];
        const lines = deckText.split('\n');
        
        for (const line of lines) {
            const trimmedLine = line.trim();
            if (!trimmedLine) continue;
            
            const parts = trimmedLine.split(' x ');
            if (parts.length === 2) {
                const cardNo = parts[0].trim();
                const quantity = parseInt(parts[1].trim());
                
                for (let i = 0; i < quantity; i++) {
                    cards.push({ card_no: cardNo });
                }
            }
        }
        
        // Shuffle the deck
        for (let i = cards.length - 1; i > 0; i--) {
            const j = Math.floor(Math.random() * (i + 1));
            [cards[i], cards[j]] = [cards[j], cards[i]];
        }
        
        return cards;
    }

    async loadCardData(cardNo) {
        try {
            const response = await fetch('/cards/cards.json');
            if (!response.ok) {
                throw new Error(`Failed to load cards.json: ${response.statusText}`);
            }
            const cardsData = await response.json();
            return cardsData[cardNo] || null;
        } catch (error) {
            console.error(`Failed to load card data for ${cardNo}:`, error);
            return null;
        }
    }

    async loadDeckAndDisplay(deckName) {
        const deckCards = await this.loadDeck(deckName);
        const handZone = this['player1-hand'];
        // Actual card aspect ratio from images: 451x630 = 0.715
        const cardHeight = 180;
        const cardWidth = Math.round(cardHeight * 0.715);
        const padding = 20;
        
        // Display first 6 cards from deck in hand
        const cardsToDisplay = deckCards.slice(0, 6);
        
        // Fallback cards if deck loading fails
        const fallbackCards = [
            { card_no: 'PL!S-bp2-022-L', name: 'Test Live Card', card_type: 'Live', score: 10 },
            { card_no: 'PL!S-bp2-001-P', name: 'Test Member 1', card_type: 'Member', cost: 10, blade: 3 },
            { card_no: 'PL!S-bp2-005-SEC', name: 'Test Member 2', card_type: 'Member', cost: 11, blade: 2 },
            { card_no: 'PL!S-bp2-009-P', name: 'Test Member 3', card_type: 'Member', cost: 9, blade: 4 },
            { card_no: 'PL!-sd1-001-SD', name: 'Test Energy', card_type: 'Energy' }
        ];
        
        const cardsToUse = cardsToDisplay.length > 0 ? cardsToDisplay : fallbackCards.map(c => ({ card_no: c.card_no }));
        
        for (let i = 0; i < cardsToUse.length; i++) {
            const deckCard = cardsToUse[i];
            let cardData = null;
            
            if (cardsToDisplay.length > 0) {
                cardData = await this.loadCardData(deckCard.card_no);
            }
            
            // Use fallback data if loading failed
            const gameCard = cardData ? {
                card_no: deckCard.card_no,
                name: cardData.name,
                card_type: this.determineCardType(cardData),
                cost: cardData.cost,
                blade: cardData.blade,
                score: cardData.score,
                required_hearts: cardData.required_hearts
            } : fallbackCards[i];
            
            const cardX = handZone.x + padding + i * (cardWidth + padding);
            const cardY = handZone.y + padding + 25;
            
            this.createCard(cardX, cardY, cardWidth, cardHeight, gameCard, handZone, true);
        }
    }

    determineCardType(cardData) {
        if (cardData.score !== undefined || cardData.required_hearts !== undefined) {
            return 'Live';
        } else if (cardData.type === 'エネルギー' || cardData.type === 'Energy') {
            return 'Energy';
        }
        return 'Member';
    }

    getCardImagePath(cardNo) {
        // Convert card_no to image file path
        // Format: SERIES-SET-NUMBER-RARITY.webp
        // Example: PL!-sd1-001-SD.webp
        if (!cardNo) return null;
        return `/img/cards_webp/${cardNo}.webp`;
    }

    loadCardImage(cardNo) {
        const imagePath = this.getCardImagePath(cardNo);
        if (!imagePath) return null;

        // Check if already loaded
        if (this.cardImages.has(cardNo)) {
            return this.cardImages.get(cardNo);
        }

        // Load the image
        try {
            this.load.image(cardNo, imagePath);
            this.cardImages.set(cardNo, cardNo);
            return cardNo;
        } catch (error) {
            console.error(`Failed to load image for ${cardNo}:`, error);
            return null;
        }
    }

    createZones() {
        const width = this.scale.width;
        const height = this.scale.height;

        // Player 2 zones (top - opponent)
        this.createZone('player2-hand', 50, 20, 1000, 200, 0x4a5568, 'Opponent Hand', false);
        this.createZone('player2-energy', 1100, 20, 300, 200, 0x2d3748, 'Opponent Energy', false);
        this.createZone('player2-live-zone', 50, 250, 400, 200, 0x2c5282, 'Live Card Zone', false);
        this.createZone('player2-success-live-zone', 500, 250, 250, 200, 0x2c5282, 'Success Live Zone', false);
        this.createZone('player2-stage-left', 800, 250, 250, 200, 0x1a202c, 'Left Side', false);
        this.createZone('player2-stage-center', 1100, 250, 250, 200, 0x1a202c, 'Center', false);
        this.createZone('player2-stage-right', 1350, 250, 250, 200, 0x1a202c, 'Right Side', false);

        // Player 1 zones (bottom - active player)
        // Live card zone on top of stage
        this.createZone('player1-live-zone', 50, height - 400, 400, 200, 0x2c5282, 'Live Card Zone', true);
        this.createZone('player1-success-live-zone', 500, height - 400, 250, 200, 0x2c5282, 'Success Live Zone', true);
        this.createZone('player1-stage-left', 800, height - 400, 250, 200, 0x1a202c, 'Left Side', true);
        this.createZone('player1-stage-center', 1100, height - 400, 250, 200, 0x1a202c, 'Center', true);
        this.createZone('player1-stage-right', 1350, height - 400, 250, 200, 0x1a202c, 'Right Side', true);
        this.createZone('player1-hand', 50, height - 200, 1000, 200, 0x4a5568, 'Hand', true);
        this.createZone('player1-energy', 1100, height - 200, 300, 200, 0x2d3748, 'Energy Zone', true);

        // Decks (sides)
        this.createZone('deck-area', 1650, height - 400, 200, 200, 0x1e3a5f, 'Deck', true);
        this.createZone('waitroom-area', 1650, height - 200, 200, 200, 0x3d1e5f, 'Waitroom', true);
    }

    createGameInfo() {
        const width = this.scale.width;
        const height = this.scale.height;

        // Game info panel (moved to side to not block interactions)
        this.infoPanel = this.add.container(width - 150, height / 2);
        
        const bg = this.add.graphics();
        bg.fillStyle(0x0f3460, 0.9);
        bg.fillRect(-120, -100, 240, 200);
        
        this.turnText = this.add.text(0, -60, 'Turn: 1', {
            fontSize: '24px',
            color: '#ffffff',
            fontStyle: 'bold'
        }).setOrigin(0.5);
        
        this.phaseText = this.add.text(0, -25, 'Phase: Active', {
            fontSize: '20px',
            color: '#e94560',
            fontStyle: 'bold'
        }).setOrigin(0.5);
        
        this.bladeText = this.add.text(0, 15, 'Blades: 0', {
            fontSize: '18px',
            color: '#ffffff'
        }).setOrigin(0.5);
        
        this.heartText = this.add.text(0, 50, 'Hearts: 0', {
            fontSize: '18px',
            color: '#ffffff'
        }).setOrigin(0.5);

        this.infoPanel.add([bg, this.turnText, this.phaseText, this.bladeText, this.heartText]);
    }

    createZone(name, x, y, width, height, color, label, interactive) {
        // Create zone background
        const graphics = this.add.graphics();
        graphics.fillStyle(color, 1);
        graphics.fillRect(x, y, width, height);
        graphics.lineStyle(4, 0xe94560, 1);
        graphics.strokeRect(x, y, width, height);

        // Add zone label
        const labelX = x + 15;
        const labelY = y + 20;
        this.add.text(labelX, labelY, label, {
            fontSize: '18px',
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
            cards: [],
            interactive: interactive
        };
    }

    async loadGameState() {
        try {
            console.log('Loading game state from /api/game-state');
            const response = await fetch('/api/game-state');
            if (!response.ok) {
                throw new Error(`HTTP ${response.status}: ${response.statusText}`);
            }
            this.gameState = await response.json();
            console.log('Game state loaded:', this.gameState);
            this.updateDisplay();
            this.loadActions();
        } catch (error) {
            console.error('Failed to load game state:', error);
            // Show error on screen
            const errorText = this.add.text(this.scale.width / 2, this.scale.height / 2, 
                `Error loading game: ${error.message}`, {
                fontSize: '16px',
                color: '#ff0000'
            }).setOrigin(0.5);
        }
    }

    updateDisplay() {
        if (!this.gameState) return;

        // Update game info
        this.turnText.setText(`Turn: ${this.gameState.turn}`);
        this.phaseText.setText(`Phase: ${this.gameState.phase.split(' - ')[1] || this.gameState.phase}`);
        
        // Calculate blades and hearts for active player
        const activePlayer = this.gameState.player1;
        const blades = activePlayer.stage.total_blades ? activePlayer.stage.total_blades() : 0;
        this.bladeText.setText(`Blades: ${blades}`);
        this.heartText.setText(`Hearts: 0`);

        // Update zones with current game state
        this.displayCards('player1-hand', this.gameState.player1.hand.cards);
        this.displayCards('player1-energy', this.gameState.player1.energy.cards);
        this.displayCards('player1-live-zone', this.gameState.player1.live_zone.cards);
        this.displayCards('player1-success-live-zone', this.gameState.player1.success_live_card_zone.cards);
        this.displayCards('player2-hand', this.gameState.player2.hand.cards);
        this.displayCards('player2-energy', this.gameState.player2.energy.cards);
        this.displayCards('player2-live-zone', this.gameState.player2.live_zone.cards);
        this.displayCards('player2-success-live-zone', this.gameState.player2.success_live_card_zone.cards);
        
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
        zone.cards.forEach(card => {
            if (card.container) card.container.destroy();
        });
        zone.cards = [];

        // Calculate card size based on zone width and number of cards (6 cards max for hand)
        const maxCards = 6;
        const padding = 15;
        const availableWidth = zone.width - (padding * 2);
        const cardWidth = Math.floor((availableWidth - (padding * (maxCards - 1))) / maxCards);
        const cardHeight = Math.round(cardWidth / 0.715); // Maintain aspect ratio
        const startX = zone.x + padding;
        const startY = zone.y + padding + 25; // +25 for label

        cards.forEach((card, index) => {
            const cardX = startX + index * (cardWidth + padding);
            this.createCard(cardX, startY, cardWidth, cardHeight, card, zone, zone.interactive);
        });
    }

    displayStageCard(zoneName, card) {
        const zone = this[zoneName];
        if (!zone) return;

        // Clear existing card in zone
        zone.cards.forEach(c => {
            if (c.container) c.container.destroy();
        });
        zone.cards = [];

        if (card) {
            const padding = 20;
            const cardWidth = zone.width - (padding * 2);
            const cardHeight = Math.round(cardWidth / 0.715);
            const cardX = zone.x + padding;
            const cardY = zone.y + 25;
            this.createCard(cardX, cardY, cardWidth, cardHeight, card, zone, zone.interactive);
        }
    }

    createCard(x, y, width, height, card, zone, interactive) {
        const container = this.add.container(x, y);

        // Determine card orientation based on card type
        const isLandscape = card.card_type === 'Live';
        
        // Swap dimensions for landscape cards
        const cardWidth = isLandscape ? height : width;
        const cardHeight = isLandscape ? width : height;

        // Try to load and display card image
        const imageKey = this.loadCardImage(card.card_no);
        
        if (imageKey && this.textures.exists(imageKey)) {
            // Display actual card image
            const cardImage = this.add.image(cardWidth / 2, cardHeight / 2, imageKey);
            cardImage.setDisplaySize(cardWidth, cardHeight);
            
            // Rotate landscape cards 90 degrees
            if (isLandscape) {
                cardImage.setRotation(Phaser.Math.DegToRad(90));
            }
            
            container.add(cardImage);
        } else {
            // Fallback to colored rectangle if image not available
            const bg = this.add.graphics();
            const cardColor = this.getCardColor(card);
            bg.fillStyle(cardColor, 1);
            bg.fillRect(0, 0, cardWidth, cardHeight);
            bg.lineStyle(2, 0xffffff, 1);
            bg.strokeRect(0, 0, cardWidth, cardHeight);
            container.add(bg);

            // Card name
            const nameText = this.add.text(cardWidth / 2, 15, card.name || 'Unknown', {
                fontSize: '10px',
                color: '#ffffff',
                fontStyle: 'bold'
            }).setOrigin(0.5);
            container.add(nameText);

            // Card type
            const typeText = this.add.text(cardWidth / 2, cardHeight - 10, card.card_type || 'Card', {
                fontSize: '8px',
                color: '#e94560'
            }).setOrigin(0.5);
            container.add(typeText);
        }

        // Cost if member card (overlay on image)
        if (card.cost) {
            const costText = this.add.text(10, cardHeight - 25, `Cost: ${card.cost}`, {
                fontSize: '8px',
                color: '#ffd700',
                backgroundColor: '#000000',
                backgroundColorAlpha: 0.7
            });
            container.add(costText);
        }

        // Blade icons if present (overlay on image)
        if (card.blade > 0) {
            const bladeText = this.add.text(cardWidth - 10, cardHeight - 25, `⚔${card.blade}`, {
                fontSize: '10px',
                color: '#ff6b6b',
                backgroundColor: '#000000',
                backgroundColorAlpha: 0.7
            });
            container.add(bladeText);
        }

        // Score for live cards
        if (card.score !== undefined) {
            const scoreText = this.add.text(cardWidth / 2, cardHeight - 25, `Score: ${card.score}`, {
                fontSize: '8px',
                color: '#00ff00',
                backgroundColor: '#000000',
                backgroundColorAlpha: 0.7
            }).setOrigin(0.5);
            container.add(scoreText);
        }

        container.setSize(cardWidth, cardHeight);
        
        if (interactive) {
            container.setInteractive({ useHandCursor: true });
            container.on('pointerdown', (pointer) => {
                this.draggedCard = { container, card, zone };
                this.dragOffset.x = pointer.x - container.x;
                this.dragOffset.y = pointer.y - container.y;
                container.setDepth(1000);
            });

            container.on('pointerup', () => {
                if (this.draggedCard && this.draggedCard.container === container) {
                    this.handleCardDrop(container, card, zone);
                }
                container.setDepth(0);
                this.draggedCard = null;
            });
        }

        zone.cards.push({ container, card });
    }

    getCardColor(card) {
        if (!card) return 0x3182ce;
        switch (card.card_type) {
            case 'Member': return 0x3182ce;
            case 'Energy': return 0x38a169;
            case 'Live': return 0x805ad5;
            default: return 0x3182ce;
        }
    }

    handleCardDrop(container, card, sourceZone) {
        // Check which zone the card was dropped on
        const dropZones = [
            'player1-stage-left', 'player1-stage-center', 'player1-stage-right',
            'player1-energy', 'player1-live-zone'
        ];

        for (const zoneName of dropZones) {
            const zone = this[zoneName];
            if (Phaser.Geom.Rectangle.Contains(zone, container.x, container.y)) {
                this.attemptCardPlay(card, sourceZone, zoneName);
                return;
            }
        }

        // Return to original position if dropped outside valid zones
        container.setPosition(sourceZone.x + 10, sourceZone.y + 25);
    }

    async attemptCardPlay(card, sourceZone, targetZone) {
        // Find the corresponding action
        const actionIndex = this.actions.findIndex(action => {
            if (targetZone.includes('stage') && action.action_type === 'play_member_to_stage') {
                return true;
            }
            if (targetZone.includes('energy') && action.action_type === 'play_energy_to_zone') {
                return true;
            }
            if (targetZone.includes('live') && action.action_type === 'place_in_live_zone') {
                return true;
            }
            return false;
        });

        if (actionIndex >= 0) {
            await this.executeAction(actionIndex);
        } else {
            // Return card to hand
            const handZone = this['player1-hand'];
            container.setPosition(handZone.x + 10, handZone.y + 25);
        }
    }

    async loadActions() {
        try {
            const response = await fetch('/api/actions');
            const data = await response.json();
            this.actions = data.actions;
            this.updateActionPanel();
        } catch (error) {
            console.error('Failed to load actions:', error);
        }
    }

    updateActionPanel() {
        const actionList = document.getElementById('action-list');
        actionList.innerHTML = '';

        if (this.actions.length === 0) {
            actionList.innerHTML = '<div style="color: #666; padding: 10px;">No legal actions available</div>';
            return;
        }

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
            const response = await fetch('/api/execute-action', {
                method: 'POST',
                headers: {
                    'Content-Type': 'application/json',
                },
                body: JSON.stringify({ action_index: index })
            });
            
            if (response.ok) {
                await this.loadGameState();
            } else {
                const error = await response.text();
                console.error('Failed to execute action:', error);
                alert('Action failed: ' + error);
            }
        } catch (error) {
            console.error('Failed to execute action:', error);
        }
    }
}

const config = {
    type: Phaser.AUTO,
    width: 1920,
    height: 1080,
    parent: 'phaser-game',
    backgroundColor: '#1a1a2e',
    scene: [GameScene],
    scale: {
        mode: Phaser.Scale.FIT,
        autoCenter: Phaser.Scale.CENTER_BOTH
    },
    render: {
        pixelArt: false,
        antialias: true,
        roundPixels: false
    },
    physics: {
        default: null
    }
};

const game = new Phaser.Game(config);

// Expose game scene globally for HTML controls
game.events.once('ready', () => {
    window.gameScene = game.scene.getScene('GameScene');
});
