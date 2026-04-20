class MenuScene extends Phaser.Scene {
    constructor() {
        super('MenuScene');
    }

    create() {
        const w = this.scale.width;
        const h = this.scale.height;

        // Title
        this.add.text(w / 2, h * 0.2, 'Rabuka Card Game', {
            fontSize: '48px',
            color: '#e94560',
            fontStyle: 'bold'
        }).setOrigin(0.5);

        this.add.text(w / 2, h * 0.3, 'Love Live! Rabuka', {
            fontSize: '24px',
            color: '#ffffff'
        }).setOrigin(0.5);

        // Deck selection
        this.add.text(w / 2, h * 0.45, 'Select Deck:', {
            fontSize: '20px',
            color: '#8888aa'
        }).setOrigin(0.5);

        const decks = ['Aqours Cup', 'Muse Cup', 'Nijigaku Cup', 'Liella Cup', 'Hasunosora Cup', 'Fade Deck'];
        this.selectedDeck = decks[0];

        decks.forEach((deck, i) => {
            const btn = this.add.text(w / 2, h * 0.52 + i * 40, deck, {
                fontSize: '18px',
                color: '#ffffff',
                backgroundColor: '#0f3460',
                padding: { x: 20, y: 10 }
            }).setOrigin(0.5);

            btn.setInteractive({ useHandCursor: true })
               .on('pointerover', () => {
                   btn.setStyle({ backgroundColor: '#1a1a2e', color: '#e94560' });
               })
               .on('pointerout', () => {
                   btn.setStyle({ backgroundColor: '#0f3460', color: '#ffffff' });
               })
               .on('pointerdown', () => {
                   this.selectedDeck = deck;
                   // Update selection visual
                   this.deckButtons.forEach(b => b.setStyle({ backgroundColor: '#0f3460', color: '#ffffff' }));
                   btn.setStyle({ backgroundColor: '#e94560', color: '#ffffff' });
               });

            if (!this.deckButtons) this.deckButtons = [];
            this.deckButtons.push(btn);
        });

        // Select first deck by default
        this.deckButtons[0].setStyle({ backgroundColor: '#e94560', color: '#ffffff' });

        // Start button
        const startBtn = this.add.text(w / 2, h * 0.85, 'Start Game', {
            fontSize: '24px',
            color: '#ffffff',
            backgroundColor: '#e94560',
            padding: { x: 30, y: 15 },
            fontStyle: 'bold'
        }).setOrigin(0.5);

        startBtn.setInteractive({ useHandCursor: true })
                .on('pointerover', () => startBtn.setStyle({ backgroundColor: '#ff6b6b' }))
                .on('pointerout', () => startBtn.setStyle({ backgroundColor: '#e94560' }))
                .on('pointerdown', () => {
                    this.scene.start('GameScene', { deck: this.selectedDeck });
                });
    }
}

class GameScene extends Phaser.Scene {
    constructor() {
        super('GameScene');
        this.gameState = null;
        this.actions = [];
        this.cardImages = new Map();
        this.cardData = new Map();
        this.selectedDeck = null;
    }

    preload() {
        // Load card data first
        this.load.json('cardsData', '/cards/cards.json')
            .on('complete', () => {
                const cardsData = this.cache.json.get('cardsData');
                // Store card data
                for (const cardNo in cardsData) {
                    this.cardData.set(cardNo, cardsData[cardNo]);
                }
                
                // Preload all card images
                for (const cardNo in cardsData) {
                    const imagePath = `/img/cards_webp/${cardNo}.webp`;
                    this.load.image(cardNo, imagePath);
                }
            });
    }

    create(data) {
        this.selectedDeck = data.deck || 'Aqours Cup';
        
        this.scale.resize(window.innerWidth, window.innerHeight);
        
        // Create zones
        this.createZones();
        
        // Create UI
        this.createUI();
        
        // Show loading message
        this.loadingText = this.add.text(this.scale.width / 2, this.scale.height / 2, 'Initializing game...', {
            fontSize: '24px',
            color: '#ffffff'
        }).setOrigin(0.5);
        
        // Initialize game then load state
        this.initializeGame().then(() => {
            if (this.loadingText) {
                this.loadingText.destroy();
            }
            this.loadGameState();
        });
        
        // Handle resize
        window.addEventListener('resize', () => {
            this.scale.resize(window.innerWidth, window.innerHeight);
            this.createZones();
            this.updateDisplay();
        });
    }

    async initializeGame() {
        try {
            // Call the init endpoint to initialize/restart the game
            const response = await fetch('/api/init', { method: 'POST' });
            if (!response.ok) {
                console.warn('Failed to initialize game, might already be initialized');
            }
        } catch (error) {
            console.warn('Init endpoint not available, game might already be initialized:', error);
        }
    }

    createZones() {
        const w = this.scale.width;
        const h = this.scale.height;
        
        // Clear existing zones
        if (this.zoneGraphics) {
            this.zoneGraphics.clear();
        } else {
            this.zoneGraphics = this.add.graphics();
        }
        
        this.zones = {
            // Player 2 (opponent - top)
            p2Hand: { x: 50, y: 20, w: w - 100, h: 100, label: 'Opponent Hand', color: 0x2d3748 },
            p2Stage: { x: 50, y: 140, w: w - 100, h: 160, label: 'Opponent Stage', color: 0x1a202c },
            p2Live: { x: 50, y: 320, w: 280, h: 90, label: 'Live Zone', color: 0x2c5282 },
            p2Success: { x: 350, y: 320, w: 180, h: 90, label: 'Success', color: 0x2c5282 },
            p2Energy: { x: w - 230, y: 320, w: 180, h: 90, label: 'Energy', color: 0x2d3748 },
            
            // Player 1 (active - bottom)
            p1Hand: { x: 50, y: h - 120, w: w - 100, h: 100, label: 'Your Hand', color: 0x4a5568 },
            p1Stage: { x: 50, y: h - 300, w: w - 100, h: 160, label: 'Your Stage', color: 0x1a202c },
            p1Live: { x: 50, y: h - 410, w: 280, h: 90, label: 'Live Zone', color: 0x2c5282 },
            p1Success: { x: 350, y: h - 410, w: 180, h: 90, label: 'Success', color: 0x2c5282 },
            p1Energy: { x: w - 230, y: h - 410, w: 180, h: 90, label: 'Energy', color: 0x2d3748 }
        };
        
        // Draw zones
        for (const [key, zone] of Object.entries(this.zones)) {
            // Zone background with gradient effect
            this.zoneGraphics.fillStyle(zone.color, 0.9);
            this.zoneGraphics.fillRoundedRect(zone.x, zone.y, zone.w, zone.h, 8);
            
            // Zone border
            this.zoneGraphics.lineStyle(2, 0x6666aa, 1);
            this.zoneGraphics.strokeRoundedRect(zone.x, zone.y, zone.w, zone.h, 8);
            
            // Zone label
            this.add.text(zone.x + 15, zone.y + 8, zone.label, {
                fontSize: '12px',
                color: '#a0aec0',
                fontStyle: 'bold'
            });
        }
    }

    createUI() {
        const w = this.scale.width;
        const h = this.scale.height;
        
        // Back to menu button
        const menuBtn = this.add.text(20, 20, '← Menu', {
            fontSize: '16px',
            color: '#ffffff',
            backgroundColor: '#0f3460',
            padding: { x: 15, y: 8 }
        }).setOrigin(0, 0);
        
        menuBtn.setInteractive({ useHandCursor: true })
               .on('pointerover', () => menuBtn.setStyle({ backgroundColor: '#1a1a2e' }))
               .on('pointerout', () => menuBtn.setStyle({ backgroundColor: '#0f3460' }))
               .on('pointerdown', () => {
                   this.scene.start('MenuScene');
               });
        
        // Game info panel
        this.infoBg = this.add.graphics();
        this.infoBg.fillStyle(0x0f3460, 0.9);
        this.infoBg.fillRoundedRect(w - 220, 20, 200, 150, 10);
        
        this.turnText = this.add.text(w - 120, 40, 'Turn: 1', {
            fontSize: '20px',
            color: '#fff',
            fontStyle: 'bold'
        }).setOrigin(0.5);
        
        this.phaseText = this.add.text(w - 120, 70, 'Phase: Main', {
            fontSize: '16px',
            color: '#e94560'
        }).setOrigin(0.5);
        
        // Actions panel
        this.actionsBg = this.add.graphics();
        this.actionsBg.fillStyle(0x16213e, 0.9);
        this.actionsBg.fillRoundedRect(20, h / 2 - 150, 250, 300, 10);
        
        this.add.text(145, h / 2 - 130, 'Actions', {
            fontSize: '18px',
            color: '#e94560',
            fontStyle: 'bold'
        }).setOrigin(0.5);
        
        this.actionButtons = [];
    }

    async loadGameState() {
        try {
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
            // Show error message on screen
            if (this.errorText) {
                this.errorText.destroy();
            }
            this.errorText = this.add.text(this.scale.width / 2, this.scale.height / 2, 
                'Game not initialized. Please restart the server or initialize game.', {
                fontSize: '16px',
                color: '#ff6b6b',
                backgroundColor: '#1a1a2e',
                padding: { x: 20, y: 10 }
            }).setOrigin(0.5);
        }
    }

    updateDisplay() {
        if (!this.gameState) return;
        
        // Update info
        this.turnText.setText(`Turn: ${this.gameState.turn}`);
        this.phaseText.setText(`Phase: ${this.gameState.phase}`);
        
        // Clear existing cards
        if (this.cardContainers) {
            this.cardContainers.forEach(c => c.destroy());
        }
        this.cardContainers = [];
        
        // Display cards
        this.displayHand('p1Hand', this.gameState.player1.hand.cards, true);
        this.displayHand('p2Hand', this.gameState.player2.hand.cards, false);
        this.displayStage('p1Stage', this.gameState.player1.stage, true);
        this.displayStage('p2Stage', this.gameState.player2.stage, false);
        this.displayZone('p1Live', this.gameState.player1.live_zone.cards);
        this.displayZone('p1Success', this.gameState.player1.success_live_card_zone.cards);
        this.displayZone('p1Energy', this.gameState.player1.energy.cards);
    }

    displayHand(zoneKey, cards, interactive) {
        const zone = this.zones[zoneKey];
        if (!zone || cards.length === 0) return;
        
        const cardWidth = 80;
        const cardHeight = 112;
        const overlap = 30;
        const totalWidth = cardWidth + (cards.length - 1) * overlap;
        const startX = zone.x + (zone.w - totalWidth) / 2;
        const startY = zone.y + (zone.h - cardHeight) / 2;
        
        cards.forEach((card, i) => {
            const x = startX + i * overlap;
            this.createCard(x, startY, cardWidth, cardHeight, card, interactive);
        });
    }

    displayStage(zoneKey, stage, interactive) {
        const zone = this.zones[zoneKey];
        if (!zone) return;
        
        const cardWidth = 120;
        const cardHeight = 168;
        const positions = [
            { x: zone.x + zone.w * 0.2, y: zone.y + (zone.h - cardHeight) / 2 },
            { x: zone.x + zone.w * 0.5 - cardWidth / 2, y: zone.y + (zone.h - cardHeight) / 2 },
            { x: zone.x + zone.w * 0.8 - cardWidth, y: zone.y + (zone.h - cardHeight) / 2 }
        ];
        
        const cards = [stage.left_side, stage.center, stage.right_side];
        cards.forEach((card, i) => {
            if (card && card.card_no) {
                this.createCard(positions[i].x, positions[i].y, cardWidth, cardHeight, card, interactive);
            }
        });
    }

    displayZone(zoneKey, cards) {
        const zone = this.zones[zoneKey];
        if (!zone || cards.length === 0) return;
        
        const cardWidth = 60;
        const cardHeight = 84;
        const overlap = 20;
        const totalWidth = cardWidth + (cards.length - 1) * overlap;
        const startX = zone.x + (zone.w - totalWidth) / 2;
        const startY = zone.y + (zone.h - cardHeight) / 2;
        
        cards.forEach((card, i) => {
            const x = startX + i * overlap;
            this.createCard(x, startY, cardWidth, cardHeight, card, false);
        });
    }

    createCard(x, y, width, height, card, interactive) {
        const container = this.add.container(x, y);
        
        // Card background
        const bg = this.add.graphics();
        bg.fillStyle(0x2a2a4e, 1);
        bg.fillRoundedRect(0, 0, width, height, 5);
        bg.lineStyle(2, 0x6666aa, 1);
        bg.strokeRoundedRect(0, 0, width, height, 5);
        container.add(bg);
        
        // Try to load card image
        const imageKey = card.card_no;
        if (this.textures.exists(imageKey)) {
            const cardImage = this.add.image(width / 2, height / 2, imageKey);
            const scale = Math.min(width / 451, height / 630);
            cardImage.setScale(scale);
            container.add(cardImage);
        } else {
            // Fallback: show card name
            const nameText = this.add.text(width / 2, height / 2, card.name || card.card_no || '?', {
                fontSize: '10px',
                color: '#fff',
                align: 'center'
            }).setOrigin(0.5);
            container.add(nameText);
        }
        
        // Card type indicator
        const typeColor = card.card_type === 'Live' ? 0x805ad5 : 
                         card.card_type === 'Energy' ? 0x38a169 : 0x3182ce;
        const typeBg = this.add.graphics();
        typeBg.fillStyle(typeColor, 1);
        typeBg.fillRoundedRect(2, 2, 20, 12, 3);
        container.add(typeBg);
        
        container.setSize(width, height);
        this.cardContainers.push(container);
    }

    async loadActions() {
        try {
            const response = await fetch('/api/actions');
            const data = await response.json();
            this.actions = data.actions;
            this.createActionButtons();
        } catch (error) {
            console.error('Failed to load actions:', error);
        }
    }

    createActionButtons() {
        // Clear existing buttons
        this.actionButtons.forEach(btn => btn.destroy());
        this.actionButtons = [];
        
        const h = this.scale.height;
        const startY = h / 2 - 90;
        
        // Separate Pass action and make it prominent
        const passAction = this.actions.find(a => a.action_type === 'pass');
        const otherActions = this.actions.filter(a => a.action_type !== 'pass');
        
        let currentIndex = 0;
        
        // Pass button - large and prominent
        if (passAction) {
            const passIndex = this.actions.indexOf(passAction);
            const passBtn = this.add.text(145, startY + currentIndex * 45, '⏭ PASS', {
                fontSize: '20px',
                color: '#ffffff',
                backgroundColor: '#e94560',
                padding: { x: 25, y: 12 },
                fontStyle: 'bold'
            }).setOrigin(0.5);
            
            passBtn.setInteractive({ useHandCursor: true })
                   .on('pointerover', () => passBtn.setStyle({ backgroundColor: '#ff6b6b' }))
                   .on('pointerout', () => passBtn.setStyle({ backgroundColor: '#e94560' }))
                   .on('pointerdown', () => this.executeAction(passIndex));
            
            this.actionButtons.push(passBtn);
            currentIndex++;
        }
        
        // Other action buttons
        otherActions.forEach((action) => {
            const actionIndex = this.actions.indexOf(action);
            const btn = this.add.text(145, startY + currentIndex * 35, action.description, {
                fontSize: '13px',
                color: '#fff',
                backgroundColor: '#0f3460',
                padding: { x: 15, y: 8 }
            }).setOrigin(0.5);
            
            btn.setInteractive({ useHandCursor: true })
               .on('pointerover', () => btn.setStyle({ backgroundColor: '#1a1a2e', color: '#e94560' }))
               .on('pointerout', () => btn.setStyle({ backgroundColor: '#0f3460', color: '#fff' }))
               .on('pointerdown', () => this.executeAction(actionIndex));
            
            this.actionButtons.push(btn);
            currentIndex++;
        });
        
        // No actions message
        if (this.actions.length === 0) {
            const noActions = this.add.text(145, h / 2, 'No actions available', {
                fontSize: '14px',
                color: '#666688'
            }).setOrigin(0.5);
            this.actionButtons.push(noActions);
        }
    }

    async executeAction(index) {
        try {
            const response = await fetch('/api/execute-action', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ action_index: index })
            });
            
            if (response.ok) {
                await this.loadGameState();
            }
        } catch (error) {
            console.error('Failed to execute action:', error);
        }
    }
}

const config = {
    type: Phaser.AUTO,
    width: window.innerWidth,
    height: window.innerHeight,
    parent: 'game-container',
    backgroundColor: '#1a1a2e',
    scene: [MenuScene, GameScene],
    scale: {
        mode: Phaser.Scale.RESIZE,
        autoCenter: Phaser.Scale.CENTER_BOTH
    }
};

const game = new Phaser.Game(config);
