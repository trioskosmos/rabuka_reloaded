class MenuScene extends Phaser.Scene {
    constructor() {
        super('MenuScene');
    }

    create() {
        const w = this.scale.width;
        const h = this.scale.height;

        // Title
        this.add.text(w / 2, h * 0.2, 'Rabuka Card Game', {
            fontSize: '52px',
            color: '#e94560',
            fontStyle: 'bold',
            shadow: { blur: 15, color: '#e94560', fill: true }
        }).setOrigin(0.5);

        this.add.text(w / 2, h * 0.3, 'Love Live! Rabuka', {
            fontSize: '28px',
            color: '#ffffff',
            fontStyle: 'italic'
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
                fontSize: '20px',
                color: '#ffffff',
                backgroundColor: '#1a1a2e',
                padding: { x: 25, y: 12 },
                border: '2px solid #4a5568',
                borderRadius: 8
            }).setOrigin(0.5);

            btn.setInteractive({ useHandCursor: true })
               .on('pointerover', () => {
                   btn.setStyle({ backgroundColor: '#2d3748', color: '#e94560', border: '2px solid #e94560' });
               })
               .on('pointerout', () => {
                   if (this.selectedDeck === deck) {
                       btn.setStyle({ backgroundColor: '#e94560', color: '#ffffff', border: '2px solid #e94560' });
                   } else {
                       btn.setStyle({ backgroundColor: '#1a1a2e', color: '#ffffff', border: '2px solid #4a5568' });
                   }
               })
               .on('pointerdown', () => {
                   this.selectedDeck = deck;
                   // Update selection visual
                   this.deckButtons.forEach(b => b.setStyle({ backgroundColor: '#1a1a2e', color: '#ffffff', border: '2px solid #4a5568' }));
                   btn.setStyle({ backgroundColor: '#e94560', color: '#ffffff', border: '2px solid #e94560' });
               });

            if (!this.deckButtons) this.deckButtons = [];
            this.deckButtons.push(btn);
        });

        // Select first deck by default
        this.deckButtons[0].setStyle({ backgroundColor: '#e94560', color: '#ffffff', border: '2px solid #e94560' });

        // Start button
        const startBtn = this.add.text(w / 2, h * 0.85, 'Start Game', {
            fontSize: '28px',
            color: '#ffffff',
            backgroundColor: '#e94560',
            padding: { x: 40, y: 18 },
            fontStyle: 'bold',
            border: '3px solid #ff6b6b',
            borderRadius: 12,
            shadow: { blur: 10, color: '#e94560', fill: true }
        }).setOrigin(0.5);

        startBtn.setInteractive({ useHandCursor: true })
                .on('pointerover', () => startBtn.setStyle({ backgroundColor: '#ff6b6b', border: '3px solid #ff6b6b' }))
                .on('pointerout', () => startBtn.setStyle({ backgroundColor: '#e94560', border: '3px solid #ff6b6b' }))
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
        this.loadingImages = new Set();
        this.pendingImageUpdates = new Map(); // Map imageKey to array of containers to update
        this.cardImageMap = new Map(); // Map card_no to image file name
        this.cardContainers = [];
    }

    preload() {
        // Load card data only - images will be loaded on-demand
        this.load.json('cardsData', '/cards/cards.json')
            .on('complete', () => {
                const cardsData = this.cache.json.get('cardsData');
                console.log('Loaded card data, total cards:', Object.keys(cardsData).length);
                
                // Create mapping from card_no to image file name
                this.cardImageMap = new Map();
                for (const cardNo in cardsData) {
                    const card = cardsData[cardNo];
                    // Extract image file name from _img field or use card_no
                    const imgPath = card._img || card.img;
                    const imgFileName = imgPath.split('/').pop().replace('.png', '.webp');
                    this.cardImageMap.set(cardNo, imgFileName);
                }
                
                console.log('Card image mapping created, images will load on-demand');
            });
    }

    create(data) {
        this.selectedDeck = data.deck || 'Aqours Cup';
        
        // Load card data from cache
        const cardsData = this.cache.json.get('cardsData');
        if (cardsData) {
            console.log('Loaded card data, total cards:', Object.keys(cardsData).length);
            for (const cardNo in cardsData) {
                this.cardData.set(cardNo, cardsData[cardNo]);
            }
        }
        
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
            this.createUI();
            this.updateDisplay();
            this.loadActions();
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
        const headerHeight = this.headerHeight || 65;
        const rightPanelWidth = 300;
        
        // Clear existing zones and labels
        if (this.zoneGraphics) {
            this.zoneGraphics.clear();
        } else {
            this.zoneGraphics = this.add.graphics();
        }
        if (this.zoneLabels) {
            this.zoneLabels.forEach(label => label.destroy());
        }
        this.zoneLabels = [];
        
        const playAreaWidth = w - rightPanelWidth;
        const playAreaHeight = h - headerHeight;
        
        // Reduce spacing between zones to use space better
        const zoneGap = 10;
        
        this.zones = {
            // Player 2 (opponent - top)
            p2Hand: { x: 20, y: headerHeight + 15, w: playAreaWidth - 40, h: 80, label: 'Opponent Hand', color: 0x2d3748, borderColor: 0x4a5568 },
            p2Stage: { x: 20, y: headerHeight + 105, w: playAreaWidth - 40, h: 120, label: 'Opponent Stage', color: 0x1a202c, borderColor: 0x718096 },
            p2Live: { x: 20, y: headerHeight + 235, w: playAreaWidth * 0.32, h: 70, label: 'Live Zone', color: 0x44337a, borderColor: 0x805ad5 },
            p2Success: { x: 20 + playAreaWidth * 0.34, y: headerHeight + 235, w: playAreaWidth * 0.20, h: 70, label: 'Success', color: 0x44337a, borderColor: 0x805ad5 },
            p2Energy: { x: 20 + playAreaWidth * 0.56, y: headerHeight + 235, w: playAreaWidth * 0.22, h: 70, label: 'Energy', color: 0x276749, borderColor: 0x48bb78 },
            
            // Player 1 (active - bottom)
            p1Hand: { x: 20, y: h - 100, w: playAreaWidth - 40, h: 80, label: 'Your Hand', color: 0x276749, borderColor: 0x48bb78 },
            p1Stage: { x: 20, y: h - 230, w: playAreaWidth - 40, h: 120, label: 'Your Stage', color: 0x1a202c, borderColor: 0x718096 },
            p1Live: { x: 20, y: h - 310, w: playAreaWidth * 0.32, h: 70, label: 'Live Zone', color: 0x44337a, borderColor: 0x805ad5 },
            p1Success: { x: 20 + playAreaWidth * 0.34, y: h - 310, w: playAreaWidth * 0.20, h: 70, label: 'Success', color: 0x44337a, borderColor: 0x805ad5 },
            p1Energy: { x: 20 + playAreaWidth * 0.56, y: h - 310, w: playAreaWidth * 0.22, h: 70, label: 'Energy', color: 0x276749, borderColor: 0x48bb78 }
        };
        
        // Draw zones
        for (const [key, zone] of Object.entries(this.zones)) {
            // Zone background with slight transparency
            this.zoneGraphics.fillStyle(zone.color, 0.85);
            this.zoneGraphics.fillRoundedRect(zone.x, zone.y, zone.w, zone.h, 12);
            
            // Zone border
            this.zoneGraphics.lineStyle(3, zone.borderColor, 1);
            this.zoneGraphics.strokeRoundedRect(zone.x, zone.y, zone.w, zone.h, 12);
            
            // Zone label with background
            const labelBg = this.zoneGraphics.fillStyle(0x1a202c, 0.9);
            this.zoneGraphics.fillRoundedRect(zone.x + 5, zone.y + 5, 100, 22, 5);
            
            const label = this.add.text(zone.x + 12, zone.y + 16, zone.label, {
                fontSize: '12px',
                color: '#e2e8f0',
                fontStyle: 'bold'
            });
            this.zoneLabels.push(label);
        }
        
        // Add counters display
        this.deckCounters = this.add.text(25, h - 345, '', {
            fontSize: '14px',
            color: '#fff',
            fontStyle: 'bold'
        });
        this.zoneLabels.push(this.deckCounters);
        
        // Add zone counter text objects
        this.zoneCounters = {};
        for (const [key, zone] of Object.entries(this.zones)) {
            const counter = this.add.text(zone.x + zone.w - 10, zone.y + zone.h - 10, '0', {
                fontSize: '16px',
                color: '#fff',
                fontStyle: 'bold',
                backgroundColor: '#000000',
                padding: { x: 6, y: 3 }
            }).setOrigin(1, 1);
            this.zoneCounters[key] = counter;
            this.zoneLabels.push(counter);
        }
    }

    createUI() {
        const w = this.scale.width;
        const h = this.scale.height;
        const headerHeight = 65;
        const rightPanelWidth = 300;
        
        // Clear existing UI elements
        if (this.uiElements) {
            this.uiElements.forEach(el => el.destroy());
        }
        this.uiElements = [];
        
        // Header bar at top
        this.headerBg = this.add.graphics();
        this.headerBg.fillStyle(0x1a202c, 1);
        this.headerBg.fillRect(0, 0, w, headerHeight);
        this.headerBg.lineStyle(3, 0xe94560, 1);
        this.headerBg.lineBetween(0, headerHeight, w, headerHeight);
        this.uiElements.push(this.headerBg);
        
        // Back to menu button in header
        const menuBtn = this.add.text(20, headerHeight / 2, '← Menu', {
            fontSize: '15px',
            color: '#ffffff',
            backgroundColor: '#4a5568',
            padding: { x: 15, y: 10 },
            fontStyle: 'bold',
            borderRadius: 8
        }).setOrigin(0, 0.5);
        
        menuBtn.setInteractive({ useHandCursor: true })
               .on('pointerover', () => menuBtn.setStyle({ backgroundColor: '#e94560' }))
               .on('pointerout', () => menuBtn.setStyle({ backgroundColor: '#4a5568' }))
               .on('pointerdown', () => {
                   this.scene.start('MenuScene');
               });
        this.uiElements.push(menuBtn);
        
        // Game title in header
        this.add.text(w / 2, headerHeight / 2, 'Rabuka Card Game', {
            fontSize: '24px',
            color: '#e94560',
            fontStyle: 'bold',
            shadow: { blur: 10, color: '#e94560', fill: true }
        }).setOrigin(0.5);
        
        // Turn and phase info in header
        this.turnText = this.add.text(w - rightPanelWidth - 160, headerHeight / 2, 'Turn: 1', {
            fontSize: '18px',
            color: '#fff',
            fontStyle: 'bold',
            backgroundColor: '#2d3748',
            padding: { x: 12, y: 6 },
            borderRadius: 6
        }).setOrigin(0.5);
        this.uiElements.push(this.turnText);
        
        this.phaseText = this.add.text(w - rightPanelWidth - 50, headerHeight / 2, 'Phase: Main', {
            fontSize: '16px',
            color: '#48bb78',
            backgroundColor: '#2d3748',
            padding: { x: 12, y: 6 },
            borderRadius: 6
        }).setOrigin(0.5);
        this.uiElements.push(this.phaseText);
        
        // Actions panel (right side, below header)
        const actionsPanelX = w - rightPanelWidth;
        const actionsPanelY = headerHeight;
        const actionsPanelHeight = h - headerHeight;
        
        this.actionsBg = this.add.graphics();
        this.actionsBg.fillStyle(0x1a202c, 0.95);
        this.actionsBg.fillRect(actionsPanelX, actionsPanelY, rightPanelWidth, actionsPanelHeight);
        this.actionsBg.lineStyle(3, 0x4a5568, 1);
        this.actionsBg.lineBetween(actionsPanelX, actionsPanelY, actionsPanelX, actionsPanelY + actionsPanelHeight);
        this.uiElements.push(this.actionsBg);
        
        this.add.text(actionsPanelX + rightPanelWidth / 2, actionsPanelY + 30, 'Actions', {
            fontSize: '20px',
            color: '#e94560',
            fontStyle: 'bold',
            shadow: { blur: 8, color: '#e94560', fill: true }
        }).setOrigin(0.5);
        
        this.actionsPanelX = actionsPanelX;
        this.actionsPanelWidth = rightPanelWidth;
        this.actionsPanelY = actionsPanelY;
        this.actionButtons = [];
        
        this.headerHeight = headerHeight;
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
        
        // Clear existing cards
        this.cardContainers.forEach(container => container.destroy());
        this.cardContainers = [];
        
        // Update turn and phase text
        this.turnText.setText(`Turn: ${this.gameState.turn}`);
        this.phaseText.setText(`Phase: ${this.gameState.phase}`);
        
        // Display cards with null checks
        if (this.gameState.player1?.hand?.cards) {
            this.displayHand('p1Hand', this.gameState.player1.hand.cards, true);
        }
        if (this.gameState.player2?.hand?.cards) {
            this.displayHand('p2Hand', this.gameState.player2.hand.cards, false);
        }
        if (this.gameState.player1?.stage) {
            this.displayStage('p1Stage', this.gameState.player1.stage, true);
        }
        if (this.gameState.player2?.stage) {
            this.displayStage('p2Stage', this.gameState.player2.stage, false);
        }
        if (this.gameState.player1?.live_zone?.cards) {
            this.displayZone('p1Live', this.gameState.player1.live_zone.cards);
        }
        if (this.gameState.player1?.success_live_card_zone?.cards) {
            this.displayZone('p1Success', this.gameState.player1.success_live_card_zone.cards);
        }
        if (this.gameState.player1?.energy?.cards) {
            this.displayZone('p1Energy', this.gameState.player1.energy.cards);
        }
        if (this.gameState.player2?.live_zone?.cards) {
            this.displayZone('p2Live', this.gameState.player2.live_zone.cards);
        }
        if (this.gameState.player2?.success_live_card_zone?.cards) {
            this.displayZone('p2Success', this.gameState.player2.success_live_card_zone.cards);
        }
        if (this.gameState.player2?.energy?.cards) {
            this.displayZone('p2Energy', this.gameState.player2.energy.cards);
        }
        
        // Update deck and waitroom counters
        if (this.deckCounters) {
            this.deckCounters.setText(
                `Deck: ${this.gameState.player1.main_deck_count} | Energy Deck: ${this.gameState.player1.energy_deck_count} | Waitroom: ${this.gameState.player1.waitroom_count}`
            );
        }
        
        // Update zone counters
        if (this.zoneCounters) {
            this.zoneCounters.p1Hand?.setText(this.gameState.player1?.hand?.cards?.length || 0);
            this.zoneCounters.p2Hand?.setText(this.gameState.player2?.hand?.cards?.length || 0);
            this.zoneCounters.p1Live?.setText(this.gameState.player1?.live_zone?.cards?.length || 0);
            this.zoneCounters.p1Success?.setText(this.gameState.player1?.success_live_card_zone?.cards?.length || 0);
            this.zoneCounters.p1Energy?.setText(this.gameState.player1?.energy?.cards?.length || 0);
            this.zoneCounters.p2Live?.setText(this.gameState.player2?.live_zone?.cards?.length || 0);
            this.zoneCounters.p2Success?.setText(this.gameState.player2?.success_live_card_zone?.cards?.length || 0);
            this.zoneCounters.p2Energy?.setText(this.gameState.player2?.energy?.cards?.length || 0);
            
            // Count stage cards
            let p1StageCount = 0;
            if (this.gameState.player1?.stage?.left_side?.card_no) p1StageCount++;
            if (this.gameState.player1?.stage?.center?.card_no) p1StageCount++;
            if (this.gameState.player1?.stage?.right_side?.card_no) p1StageCount++;
            this.zoneCounters.p1Stage?.setText(p1StageCount);
            
            let p2StageCount = 0;
            if (this.gameState.player2?.stage?.left_side?.card_no) p2StageCount++;
            if (this.gameState.player2?.stage?.center?.card_no) p2StageCount++;
            if (this.gameState.player2?.stage?.right_side?.card_no) p2StageCount++;
            this.zoneCounters.p2Stage?.setText(p2StageCount);
        }
    }

    displayHand(zoneKey, cards, interactive) {
        const zone = this.zones[zoneKey];
        if (!zone || cards.length === 0) return;
        
        const cardWidth = 80;
        const cardHeight = 112;
        
        // Calculate dynamic overlap based on available space
        const maxTotalWidth = zone.w - 20; // 10px padding on each side
        const cardsTotalWidth = cards.length * cardWidth;
        let overlap;
        
        if (cardsTotalWidth <= maxTotalWidth) {
            // No overlap needed - cards fit with spacing
            overlap = cardWidth + 5; // 5px gap between cards
        } else {
            // Calculate overlap needed to fit all cards
            overlap = (maxTotalWidth - cardWidth) / (cards.length - 1);
            // Ensure minimum overlap
            overlap = Math.max(overlap, 20); // Minimum 20px overlap
        }
        
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
        
        // Calculate dynamic overlap based on available space
        const maxTotalWidth = zone.w - 20; // 10px padding on each side
        const cardsTotalWidth = cards.length * cardWidth;
        let overlap;
        
        if (cardsTotalWidth <= maxTotalWidth) {
            // No overlap needed - cards fit with spacing
            overlap = cardWidth + 3; // 3px gap between cards
        } else {
            // Calculate overlap needed to fit all cards
            overlap = (maxTotalWidth - cardWidth) / (cards.length - 1);
            // Ensure minimum overlap
            overlap = Math.max(overlap, 15); // Minimum 15px overlap
        }
        
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
        
        // Get image file name from mapping
        const imageKey = card.card_no;
        const imgFileName = this.cardImageMap ? this.cardImageMap.get(imageKey) : `${imageKey}.webp`;
        
        // Check orientation - rotate if Wait
        const isWait = card.orientation === 'Wait';
        
        if (this.textures.exists(imageKey)) {
            // Image already loaded, display it
            const texture = this.textures.get(imageKey);
            const cardImage = this.add.image(width / 2, height / 2, imageKey);
            
            // Calculate scale to fit image within card dimensions while maintaining aspect ratio
            const imageWidth = texture.source[0].width;
            const imageHeight = texture.source[0].height;
            const scaleX = width / imageWidth;
            const scaleY = height / imageHeight;
            const scale = Math.min(scaleX, scaleY) * 0.95; // 95% to leave small margin
            
            cardImage.setScale(scale);
            
            // Rotate 90 degrees if in Wait state (tapped)
            if (isWait) {
                cardImage.setRotation(Math.PI / 2);
            }
            
            container.add(cardImage);
        } else {
            // Load image on-demand if not already loading
            if (!this.loadingImages.has(imageKey) && imgFileName) {
                this.loadingImages.add(imageKey);
                this.pendingImageUpdates.set(imageKey, []);
                
                this.load.image(imageKey, `/img/cards_webp/${imgFileName}`)
                    .on('complete', () => {
                        this.loadingImages.delete(imageKey);
                        // Image loaded, update all pending containers
                        const containers = this.pendingImageUpdates.get(imageKey) || [];
                        containers.forEach(c => {
                            // Remove the placeholder text if it exists
                            c.each(child => {
                                if (child.type === 'Text') {
                                    child.destroy();
                                }
                            });
                            // Add the image
                            const texture = this.textures.get(imageKey);
                            const cardImage = this.add.image(c.width / 2, c.height / 2, imageKey);
                            
                            // Calculate scale to fit image within card dimensions
                            const imageWidth = texture.source[0].width;
                            const imageHeight = texture.source[0].height;
                            const scaleX = c.width / imageWidth;
                            const scaleY = c.height / imageHeight;
                            const scale = Math.min(scaleX, scaleY) * 0.95;
                            
                            cardImage.setScale(scale);
                            
                            // Rotate if in Wait state
                            if (c.cardData && c.cardData.orientation === 'Wait') {
                                cardImage.setRotation(Math.PI / 2);
                            }
                            
                            c.add(cardImage);
                        });
                        this.pendingImageUpdates.delete(imageKey);
                    })
                    .on('loaderror', () => {
                        this.loadingImages.delete(imageKey);
                        this.pendingImageUpdates.delete(imageKey);
                        console.warn('Failed to load image:', imageKey, 'as', imgFileName, '- will show placeholder');
                        // Update failed containers to show permanent placeholder
                        const containers = this.pendingImageUpdates.get(imageKey) || [];
                        containers.forEach(c => {
                            c.each(child => {
                                if (child.type === 'Text') {
                                    child.setText(card.name || card.card_no || '?');
                                    child.setStyle({ color: '#ff6b6b' }); // Red color for failed loads
                                }
                            });
                        });
                    });
                this.load.start();
            }
            
            // Track this container for when the image loads
            if (this.pendingImageUpdates.has(imageKey)) {
                this.pendingImageUpdates.get(imageKey).push(container);
                // Store card data for later orientation check
                container.cardData = card;
            }
            
            // Fallback: show card name while loading
            const nameText = this.add.text(width / 2, height / 2, card.name || card.card_no || '?', {
                fontSize: '10px',
                color: '#fff',
                align: 'center',
                wordWrap: { width: width - 10 }
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
        
        const panelX = this.actionsPanelX;
        const panelWidth = this.actionsPanelWidth;
        const startY = this.actionsPanelY + 50;
        const buttonWidth = panelWidth - 20;
        
        // Separate Pass action and make it prominent
        const passAction = this.actions.find(a => a.action_type === 'pass');
        console.log('Pass action found:', passAction);
        const otherActions = this.actions.filter(a => a.action_type !== 'pass');
        
        let currentIndex = 0;
        
        // Pass button - large and prominent
        if (passAction) {
            const passIndex = this.actions.indexOf(passAction);
            const passBtn = this.add.text(panelX + panelWidth / 2, startY + currentIndex * 55, '⏭ PASS TURN', {
                fontSize: '18px',
                color: '#ffffff',
                backgroundColor: '#e94560',
                padding: { x: 30, y: 12 },
                fontStyle: 'bold',
                fixedWidth: buttonWidth,
                align: 'center'
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
            const btn = this.add.text(panelX + panelWidth / 2, startY + currentIndex * 45, action.description, {
                fontSize: '12px',
                color: '#fff',
                backgroundColor: '#2d3748',
                padding: { x: 15, y: 10 },
                fixedWidth: buttonWidth,
                align: 'center',
                wordWrap: { width: buttonWidth - 30 }
            }).setOrigin(0.5);
            
            btn.setInteractive({ useHandCursor: true })
               .on('pointerover', () => btn.setStyle({ backgroundColor: '#4a5568', color: '#e94560' }))
               .on('pointerout', () => btn.setStyle({ backgroundColor: '#2d3748', color: '#fff' }))
               .on('pointerdown', () => this.executeAction(actionIndex));
            
            this.actionButtons.push(btn);
            currentIndex++;
        });
        
        // No actions message
        if (this.actions.length === 0) {
            const noActions = this.add.text(panelX + panelWidth / 2, startY + 50, 'No actions available', {
                fontSize: '14px',
                color: '#718096'
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
