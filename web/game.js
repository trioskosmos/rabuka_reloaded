class MenuScene extends Phaser.Scene {
    constructor() {
        super('MenuScene');
    }

    create() {
        const w = this.scale.width;
        const h = this.scale.height;

        // Calculate responsive sizes based on screen dimensions
        const minDim = Math.min(w, h);
        const titleSize = Math.max(32, Math.floor(minDim * 0.05));
        const subtitleSize = Math.max(18, Math.floor(minDim * 0.028));
        const labelSize = Math.max(14, Math.floor(minDim * 0.022));
        const buttonSize = Math.max(16, Math.floor(minDim * 0.025));
        const startButtonSize = Math.max(22, Math.floor(minDim * 0.035));
        const buttonSpacing = Math.max(30, Math.floor(h * 0.04));
        const buttonPaddingX = Math.max(20, Math.floor(w * 0.03));
        const buttonPaddingY = Math.max(10, Math.floor(h * 0.015));

        // Title with enhanced glow effect
        this.add.text(w / 2, h * 0.18, 'Rabuka Card Game', {
            fontSize: `${titleSize}px`,
            color: '#e94560',
            fontStyle: 'bold',
            shadow: { blur: 20, color: '#e94560', fill: true, offsetX: 0, offsetY: 0 }
        }).setOrigin(0.5);

        this.add.text(w / 2, h * 0.26, 'Love Live! Rabuka', {
            fontSize: `${subtitleSize}px`,
            color: '#ffffff',
            fontStyle: 'italic',
            shadow: { blur: 8, color: '#ffffff', fill: true }
        }).setOrigin(0.5);

        // Deck selection label
        this.add.text(w / 2, h * 0.38, 'Select Deck:', {
            fontSize: `${labelSize}px`,
            color: '#a0aec0',
            fontStyle: 'bold'
        }).setOrigin(0.5);

        const decks = ['Aqours Cup', 'Muse Cup', 'Nijigaku Cup', 'Liella Cup', 'Hasunosora Cup', 'Fade Deck'];
        this.selectedDeck = decks[0];

        decks.forEach((deck, i) => {
            const startY = h * 0.43;
            const btn = this.add.text(w / 2, startY + i * buttonSpacing, deck, {
                fontSize: `${buttonSize}px`,
                color: '#ffffff',
                backgroundColor: '#1a202c',
                padding: { x: buttonPaddingX, y: buttonPaddingY },
                border: '2px solid #4a5568',
                borderRadius: 10,
                shadow: { blur: 5, color: '#000000', fill: true }
            }).setOrigin(0.5);

            btn.setInteractive({ useHandCursor: true })
               .on('pointerover', () => {
                   btn.setStyle({ 
                       backgroundColor: '#2d3748', 
                       color: '#e94560', 
                       border: '2px solid #e94560',
                       shadow: { blur: 8, color: '#e94560', fill: true }
                   });
               })
               .on('pointerout', () => {
                   if (this.selectedDeck === deck) {
                       btn.setStyle({ 
                           backgroundColor: '#e94560', 
                           color: '#ffffff', 
                           border: '2px solid #e94560',
                           shadow: { blur: 8, color: '#e94560', fill: true }
                       });
                   } else {
                       btn.setStyle({ 
                           backgroundColor: '#1a202c', 
                           color: '#ffffff', 
                           border: '2px solid #4a5568',
                           shadow: { blur: 5, color: '#000000', fill: true }
                       });
                   }
               })
               .on('pointerdown', () => {
                   this.selectedDeck = deck;
                   this.deckButtons.forEach(b => b.setStyle({ 
                       backgroundColor: '#1a202c', 
                       color: '#ffffff', 
                       border: '2px solid #4a5568',
                       shadow: { blur: 5, color: '#000000', fill: true }
                   }));
                   btn.setStyle({ 
                       backgroundColor: '#e94560', 
                       color: '#ffffff', 
                       border: '2px solid #e94560',
                       shadow: { blur: 8, color: '#e94560', fill: true }
                   });
               });

            if (!this.deckButtons) this.deckButtons = [];
            this.deckButtons.push(btn);
        });

        // Select first deck by default
        this.deckButtons[0].setStyle({ 
            backgroundColor: '#e94560', 
            color: '#ffffff', 
            border: '2px solid #e94560',
            shadow: { blur: 8, color: '#e94560', fill: true }
        });

        // Start button with enhanced styling
        const startBtn = this.add.text(w / 2, h * 0.82, 'Start Game', {
            fontSize: `${startButtonSize}px`,
            color: '#ffffff',
            backgroundColor: '#e94560',
            padding: { x: buttonPaddingX * 1.5, y: buttonPaddingY * 1.5 },
            fontStyle: 'bold',
            border: '3px solid #ff6b6b',
            borderRadius: 15,
            shadow: { blur: 15, color: '#e94560', fill: true, offsetX: 0, offsetY: 0 }
        }).setOrigin(0.5);

        startBtn.setInteractive({ useHandCursor: true })
                .on('pointerover', () => startBtn.setStyle({ 
                    backgroundColor: '#ff6b6b', 
                    border: '3px solid #ff8787',
                    shadow: { blur: 20, color: '#ff6b6b', fill: true }
                }))
                .on('pointerout', () => startBtn.setStyle({ 
                    backgroundColor: '#e94560', 
                    border: '3px solid #ff6b6b',
                    shadow: { blur: 15, color: '#e94560', fill: true }
                }))
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
        this.loadingActions = false; // Flag to prevent concurrent action loading
        this.pendingActionsLoad = false; // Flag to track if a load is pending
    }

    preload() {
        // Load texticon images for hearts and blades
        this.load.image('icon_blade', '/img/texticon/icon_blade.png');
        for (let i = 0; i <= 6; i++) {
            const fileName = i.toString().padStart(2, '0');
            this.load.image(`heart_${fileName}`, `/img/texticon/heart_${fileName}.png`);
        }
        
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
                    let imgFileName = `${cardNo}.webp`; // Default fallback
                    if (imgPath) {
                        // Extract filename from path, handling various formats
                        const parts = imgPath.split('/');
                        const filename = parts[parts.length - 1] || imgPath;
                        // Replace common image extensions with .webp
                        imgFileName = filename.replace(/\.(png|jpg|jpeg|gif)$/i, '.webp');
                        // If no extension, add .webp
                        if (!imgFileName.includes('.')) {
                            imgFileName += '.webp';
                        }
                    }
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
        
        // Show loading message with responsive sizing
        const h = this.scale.height;
        const loadingSize = Math.max(20, Math.floor(h * 0.028));
        this.loadingText = this.add.text(this.scale.width / 2, this.scale.height / 2, 'Initializing game...', {
            fontSize: `${loadingSize}px`,
            color: '#ffffff',
            fontStyle: 'bold',
            shadow: { blur: 8, color: '#e94560', fill: true }
        }).setOrigin(0.5);
        
        // Initialize game then load state
        this.initializeGame().then(() => {
            if (this.loadingText) {
                this.loadingText.destroy();
            }
            this.loadGameState();
        });
        
        // Handle resize with debounce to prevent multiple rapid calls
        this.resizeTimeout = null;
        window.addEventListener('resize', () => {
            if (this.resizeTimeout) {
                clearTimeout(this.resizeTimeout);
            }
            this.resizeTimeout = setTimeout(() => {
                this.scale.resize(window.innerWidth, window.innerHeight);
                this.handleResize();
            }, 100);
        });
    }

    async initializeGame() {
        try {
            // Call the init endpoint to initialize/restart the game with selected deck
            const response = await fetch('/api/init', { 
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ deck: this.selectedDeck })
            });
            if (!response.ok) {
                console.warn('Failed to initialize game, might already be initialized');
            }
        } catch (error) {
            console.warn('Init endpoint not available, game might already be initialized:', error);
        }
    }

    handleResize() {
        // Clear all UI elements before recreation
        this.clearAllUI();
        this.createZones();
        this.createUI();
        this.updateDisplay();
        this.loadActions();
    }

    clearAllUI() {
        // Clear action buttons
        if (this.actionButtons) {
            this.actionButtons.forEach(btn => btn.destroy());
            this.actionButtons = [];
        }
        
        // Clear zone labels
        if (this.zoneLabels) {
            this.zoneLabels.forEach(label => label.destroy());
            this.zoneLabels = [];
        }
        
        // Clear UI elements
        if (this.uiElements) {
            this.uiElements.forEach(el => el.destroy());
            this.uiElements = [];
        }
        
        // Clear card containers
        if (this.cardContainers) {
            this.cardContainers.forEach(container => container.destroy());
            this.cardContainers = [];
        }
        
        // Clear stats containers
        if (this.p1StatsContainer) {
            this.p1StatsContainer.destroy();
            this.p1StatsContainer = null;
        }
        if (this.p2StatsContainer) {
            this.p2StatsContainer.destroy();
            this.p2StatsContainer = null;
        }
        
        // Clear zone graphics
        if (this.zoneGraphics) {
            this.zoneGraphics.clear();
            this.zoneGraphics.destroy();
            this.zoneGraphics = null;
        }
        
        // Clear actions panel graphics
        if (this.actionsBg) {
            this.actionsBg.clear();
            this.actionsBg.destroy();
            this.actionsBg = null;
        }
        
        // Clear header graphics
        if (this.headerBg) {
            this.headerBg.clear();
            this.headerBg.destroy();
            this.headerBg = null;
        }
    }

    createZones() {
        const w = this.scale.width;
        const h = this.scale.height;
        
        // Calculate responsive sizes based on screen dimensions
        const headerHeight = Math.max(60, Math.floor(h * 0.08));
        const rightPanelWidth = Math.max(280, Math.floor(w * 0.25));
        const zoneGap = Math.max(8, Math.floor(h * 0.01));
        const zonePadding = Math.max(15, Math.floor(w * 0.015));
        
        this.headerHeight = headerHeight;
        
        // Create zone graphics
        this.zoneGraphics = this.add.graphics();
        this.zoneLabels = [];
        
        const playAreaWidth = w - rightPanelWidth;
        const playAreaHeight = h - headerHeight;
        
        // Calculate zone heights proportionally
        const handHeight = Math.max(90, Math.floor(playAreaHeight * 0.14));
        const stageHeight = Math.max(130, Math.floor(playAreaHeight * 0.22));
        const supportHeight = Math.max(80, Math.floor(playAreaHeight * 0.12));
        
        // Calculate vertical positions
        const p2HandY = headerHeight + zoneGap;
        const p2StageY = p2HandY + handHeight + zoneGap;
        const p2SupportY = p2StageY + stageHeight + zoneGap;
        
        const p1HandY = h - handHeight - zoneGap;
        const p1StageY = p1HandY - stageHeight - zoneGap;
        const p1SupportY = p1StageY - supportHeight - zoneGap;
        
        this.zones = {
            // Player 2 (opponent - top)
            p2Hand: { x: zonePadding, y: p2HandY, w: playAreaWidth - zonePadding * 2, h: handHeight, label: 'Opponent Hand', color: 0x2d3748, borderColor: 0x4a5568 },
            p2Stage: { x: zonePadding, y: p2StageY, w: playAreaWidth - zonePadding * 2, h: stageHeight, label: 'Opponent Stage', color: 0x1a202c, borderColor: 0x718096 },
            p2Live: { x: zonePadding, y: p2SupportY, w: playAreaWidth * 0.32, h: supportHeight, label: 'Live Zone', color: 0x44337a, borderColor: 0x805ad5 },
            p2Success: { x: zonePadding + playAreaWidth * 0.34, y: p2SupportY, w: playAreaWidth * 0.20, h: supportHeight, label: 'Success', color: 0x44337a, borderColor: 0x805ad5 },
            p2Energy: { x: zonePadding + playAreaWidth * 0.56, y: p2SupportY, w: playAreaWidth * 0.22, h: supportHeight, label: 'Energy', color: 0x276749, borderColor: 0x48bb78 },
            
            // Player 1 (active - bottom)
            p1Hand: { x: zonePadding, y: p1HandY, w: playAreaWidth - zonePadding * 2, h: handHeight, label: 'Your Hand', color: 0x276749, borderColor: 0x48bb78 },
            p1Stage: { x: zonePadding, y: p1StageY, w: playAreaWidth - zonePadding * 2, h: stageHeight, label: 'Your Stage', color: 0x1a202c, borderColor: 0x718096 },
            p1Live: { x: zonePadding, y: p1SupportY, w: playAreaWidth * 0.32, h: supportHeight, label: 'Live Zone', color: 0x44337a, borderColor: 0x805ad5 },
            p1Success: { x: zonePadding + playAreaWidth * 0.34, y: p1SupportY, w: playAreaWidth * 0.20, h: supportHeight, label: 'Success', color: 0x44337a, borderColor: 0x805ad5 },
            p1Energy: { x: zonePadding + playAreaWidth * 0.56, y: p1SupportY, w: playAreaWidth * 0.22, h: supportHeight, label: 'Energy', color: 0x276749, borderColor: 0x48bb78 }
        };
        
        // Calculate responsive font sizes
        const labelFontSize = Math.max(11, Math.floor(h * 0.015));
        const counterFontSize = Math.max(14, Math.floor(h * 0.018));
        const deckCounterFontSize = Math.max(12, Math.floor(h * 0.016));
        
        // Draw zones with enhanced styling
        for (const [key, zone] of Object.entries(this.zones)) {
            // Zone background with gradient-like effect (multiple layers)
            this.zoneGraphics.fillStyle(zone.color, 0.9);
            this.zoneGraphics.fillRoundedRect(zone.x, zone.y, zone.w, zone.h, 14);
            
            // Zone border with glow effect
            this.zoneGraphics.lineStyle(2, zone.borderColor, 1);
            this.zoneGraphics.strokeRoundedRect(zone.x, zone.y, zone.w, zone.h, 14);
            
            // Inner glow effect
            this.zoneGraphics.lineStyle(1, zone.borderColor, 0.3);
            this.zoneGraphics.strokeRoundedRect(zone.x + 2, zone.y + 2, zone.w - 4, zone.h - 4, 12);
            
            // Zone label with enhanced background
            const labelBgWidth = Math.max(100, Math.floor(zone.w * 0.25));
            const labelBgHeight = Math.max(20, Math.floor(h * 0.025));
            this.zoneGraphics.fillStyle(0x1a202c, 0.95);
            this.zoneGraphics.fillRoundedRect(zone.x + 6, zone.y + 6, labelBgWidth, labelBgHeight, 6);
            
            const label = this.add.text(zone.x + 14, zone.y + 6 + labelBgHeight / 2, zone.label, {
                fontSize: `${labelFontSize}px`,
                color: '#e2e8f0',
                fontStyle: 'bold'
            }).setOrigin(0, 0.5);
            this.zoneLabels.push(label);
        }
        
        // Add counters display for both players with responsive positioning
        const p1CounterY = p1SupportY - supportHeight - zoneGap - 20;
        const p2CounterY = p2SupportY + supportHeight + zoneGap + 10;
        
        this.p1DeckCounters = this.add.text(zonePadding, p1CounterY, '', {
            fontSize: `${deckCounterFontSize}px`,
            color: '#48bb78',
            fontStyle: 'bold',
            backgroundColor: '#1a202c',
            padding: { x: 10, y: 5 },
            borderRadius: 6
        });
        this.zoneLabels.push(this.p1DeckCounters);
        
        this.p2DeckCounters = this.add.text(zonePadding, p2CounterY, '', {
            fontSize: `${deckCounterFontSize}px`,
            color: '#718096',
            fontStyle: 'bold',
            backgroundColor: '#1a202c',
            padding: { x: 10, y: 5 },
            borderRadius: 6
        });
        this.zoneLabels.push(this.p2DeckCounters);
        
        // Add zone counter text objects with responsive sizing
        this.zoneCounters = {};
        for (const [key, zone] of Object.entries(this.zones)) {
            const counter = this.add.text(zone.x + zone.w - 12, zone.y + zone.h - 12, '0', {
                fontSize: `${counterFontSize}px`,
                color: '#fff',
                fontStyle: 'bold',
                backgroundColor: '#000000',
                padding: { x: 8, y: 4 },
                borderRadius: 4
            }).setOrigin(1, 1);
            this.zoneCounters[key] = counter;
            this.zoneLabels.push(counter);
        }
    }

    createUI() {
        const w = this.scale.width;
        const h = this.scale.height;
        
        // Calculate responsive sizes based on screen dimensions
        const headerHeight = this.headerHeight || Math.max(60, Math.floor(h * 0.08));
        const rightPanelWidth = Math.max(280, Math.floor(w * 0.25));
        
        // Calculate responsive font sizes
        const menuBtnSize = Math.max(14, Math.floor(h * 0.018));
        const titleSize = Math.max(20, Math.floor(h * 0.028));
        const statsSize = Math.max(12, Math.floor(h * 0.016));
        const turnSize = Math.max(16, Math.floor(h * 0.022));
        const phaseSize = Math.max(14, Math.floor(h * 0.02));
        const actionsTitleSize = Math.max(18, Math.floor(h * 0.025));
        
        // Clear existing UI elements
        if (this.uiElements) {
            this.uiElements.forEach(el => el.destroy());
        }
        this.uiElements = [];
        
        // Header bar at top with gradient effect
        this.headerBg = this.add.graphics();
        // Main header background
        this.headerBg.fillStyle(0x1a202c, 1);
        this.headerBg.fillRect(0, 0, w, headerHeight);
        // Bottom border with glow
        this.headerBg.lineStyle(2, 0xe94560, 1);
        this.headerBg.lineBetween(0, headerHeight - 1, w, headerHeight - 1);
        // Subtle top highlight
        this.headerBg.lineStyle(1, 0x4a5568, 0.5);
        this.headerBg.lineBetween(0, 1, w, 1);
        this.uiElements.push(this.headerBg);
        
        // Back to menu button in header
        const menuBtnPadding = Math.max(10, Math.floor(w * 0.012));
        const menuBtn = this.add.text(menuBtnPadding, headerHeight / 2, 'Menu', {
            fontSize: `${menuBtnSize}px`,
            color: '#ffffff',
            backgroundColor: '#2d3748',
            padding: { x: menuBtnPadding, y: menuBtnPadding * 0.6 },
            fontStyle: 'bold',
            borderRadius: 8,
            shadow: { blur: 4, color: '#000000', fill: true }
        }).setOrigin(0, 0.5);
        
        menuBtn.setInteractive({ useHandCursor: true })
               .on('pointerover', () => menuBtn.setStyle({ 
                   backgroundColor: '#e94560',
                   shadow: { blur: 6, color: '#e94560', fill: true }
               }))
               .on('pointerout', () => menuBtn.setStyle({ 
                   backgroundColor: '#2d3748',
                   shadow: { blur: 4, color: '#000000', fill: true }
               }))
               .on('pointerdown', () => {
                   this.scene.start('MenuScene');
               });
        this.uiElements.push(menuBtn);
        
        // Game title in header with enhanced glow
        this.add.text(w / 2, headerHeight / 2, 'Rabuka Card Game', {
            fontSize: `${titleSize}px`,
            color: '#e94560',
            fontStyle: 'bold',
            shadow: { blur: 12, color: '#e94560', fill: true }
        }).setOrigin(0.5);
        
        // Player stats in header (heart vector and blade count) - using images
        const p1StatsX = menuBtnPadding * 4 + 60;
        this.p1StatsContainer = this.add.container(p1StatsX, headerHeight / 2);
        this.p1StatsLabel = this.add.text(0, 0, 'You: ', {
            fontSize: `${statsSize}px`,
            color: '#48bb78',
            fontStyle: 'bold'
        }).setOrigin(0, 0.5);
        this.p1HeartIcons = [];
        this.p1HeartCounts = [];
        const iconScale = Math.max(0.4, Math.min(0.6, h * 0.0008));
        // Get actual blade icon width to position text correctly
        const bladeTexture = this.textures.get('icon_blade');
        const bladeWidth = bladeTexture.getSourceImage().width * iconScale;
        this.p1BladeIcon = this.add.image(50, 0, 'icon_blade').setScale(iconScale).setOrigin(0, 0.5);
        this.p1BladeCount = this.add.text(50 + bladeWidth + 5, 0, '0', {
            fontSize: `${statsSize}px`,
            color: '#48bb78',
            fontStyle: 'bold'
        }).setOrigin(0, 0.5);
        this.p1StatsContainer.add([this.p1StatsLabel, this.p1BladeIcon, this.p1BladeCount]);
        this.uiElements.push(this.p1StatsContainer);
        
        const p2StatsX = p1StatsX + 180;
        this.p2StatsContainer = this.add.container(p2StatsX, headerHeight / 2);
        this.p2StatsLabel = this.add.text(0, 0, 'Opponent: ', {
            fontSize: `${statsSize}px`,
            color: '#718096',
            fontStyle: 'bold'
        }).setOrigin(0, 0.5);
        this.p2HeartIcons = [];
        this.p2HeartCounts = [];
        this.p2BladeIcon = this.add.image(70, 0, 'icon_blade').setScale(iconScale).setOrigin(0, 0.5);
        this.p2BladeCount = this.add.text(70 + bladeWidth + 5, 0, '0', {
            fontSize: `${statsSize}px`,
            color: '#718096',
            fontStyle: 'bold'
        }).setOrigin(0, 0.5);
        this.p2StatsContainer.add([this.p2StatsLabel, this.p2BladeIcon, this.p2BladeCount]);
        this.uiElements.push(this.p2StatsContainer);
        
        // Turn and phase info in header with enhanced styling
        const phasePadding = Math.max(8, Math.floor(w * 0.01));
        this.turnText = this.add.text(w - rightPanelWidth - 140, headerHeight / 2, 'Turn: 1', {
            fontSize: `${turnSize}px`,
            color: '#fff',
            fontStyle: 'bold',
            backgroundColor: '#2d3748',
            padding: { x: phasePadding, y: phasePadding * 0.7 },
            borderRadius: 8,
            shadow: { blur: 4, color: '#000000', fill: true }
        }).setOrigin(0.5);
        this.uiElements.push(this.turnText);
        
        this.phaseText = this.add.text(w - rightPanelWidth - 40, headerHeight / 2, 'Phase: Main', {
            fontSize: `${phaseSize}px`,
            color: '#48bb78',
            fontStyle: 'bold',
            backgroundColor: '#2d3748',
            padding: { x: phasePadding, y: phasePadding * 0.7 },
            borderRadius: 8,
            shadow: { blur: 4, color: '#000000', fill: true }
        }).setOrigin(0.5);
        this.uiElements.push(this.phaseText);
        
        // Actions panel (right side, below header)
        const actionsPanelX = w - rightPanelWidth;
        const actionsPanelY = headerHeight;
        const actionsPanelHeight = h - headerHeight;
        
        this.actionsBg = this.add.graphics();
        this.actionsBg.fillStyle(0x1a202c, 0.95);
        this.actionsBg.fillRect(actionsPanelX, actionsPanelY, rightPanelWidth, actionsPanelHeight);
        // Left border with glow effect
        this.actionsBg.lineStyle(2, 0x4a5568, 1);
        this.actionsBg.lineBetween(actionsPanelX, actionsPanelY, actionsPanelX, actionsPanelY + actionsPanelHeight);
        // Inner border for depth
        this.actionsBg.lineStyle(1, 0x718096, 0.3);
        this.actionsBg.lineBetween(actionsPanelX + 2, actionsPanelY, actionsPanelX + 2, actionsPanelY + actionsPanelHeight);
        this.uiElements.push(this.actionsBg);
        
        this.add.text(actionsPanelX + rightPanelWidth / 2, actionsPanelY + 35, 'Actions', {
            fontSize: `${actionsTitleSize}px`,
            color: '#e94560',
            fontStyle: 'bold',
            shadow: { blur: 10, color: '#e94560', fill: true }
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
            // Show error message on screen with responsive sizing
            if (this.errorText) {
                this.errorText.destroy();
            }
            const h = this.scale.height;
            const errorSize = Math.max(14, Math.floor(h * 0.02));
            const errorPadding = Math.max(10, Math.floor(h * 0.015));
            this.errorText = this.add.text(this.scale.width / 2, this.scale.height / 2, 
                'Game not initialized. Please restart the server or initialize game.', {
                fontSize: `${errorSize}px`,
                color: '#ff6b6b',
                backgroundColor: '#1a202c',
                padding: { x: errorPadding, y: errorPadding * 0.6 },
                borderRadius: 8,
                fontStyle: 'bold',
                shadow: { blur: 6, color: '#ff6b6b', fill: true }
            }).setOrigin(0.5);
        }
    }

    calculateHeartVector(stage) {
        if (!stage) return {};
        const hearts = {};
        const members = [stage.left_side, stage.center, stage.right_side];
        members.forEach(member => {
            if (member && member.card_no) {
                // Look up card data to get heart information
                const cardData = this.cardData.get(member.card_no);
                if (cardData && cardData.base_heart) {
                    // heart is an object like { heart01: 1, heart03: 2, heart06: 1 }
                    // Handle all heart types dynamically, but exclude blade hearts (b_heart)
                    // Blade hearts only exist on cards revealed by エール, not on stage members
                    for (const [heartType, count] of Object.entries(cardData.base_heart)) {
                        // Skip blade hearts (b_heart) - they are not on stage members
                        if (count && !heartType.startsWith('b_heart')) {
                            hearts[heartType] = (hearts[heartType] || 0) + count;
                        }
                    }
                }
            }
        });
        return hearts;
    }

    calculateBladeCount(stage) {
        if (!stage) return '0';
        let totalBlades = 0;
        const members = [stage.left_side, stage.center, stage.right_side];
        members.forEach(member => {
            if (member && member.card_no) {
                // Look up card data to get blade information
                const cardData = this.cardData.get(member.card_no);
                if (cardData && cardData.blade) {
                    // blade is a number
                    totalBlades += cardData.blade || 0;
                }
            }
        });
        return totalBlades.toString();
    }

    updateDisplay() {
        if (!this.gameState) return;
        
        // Clear existing cards
        this.cardContainers.forEach(container => container.destroy());
        this.cardContainers = [];
        
        // Update turn and phase text
        this.turnText.setText(`Turn: ${this.gameState.turn}`);
        this.phaseText.setText(`Phase: ${this.gameState.phase}`);
        
        // Update player stats (heart vector and blade count)
        // Calculate heart vector from stage members
        const p1Hearts = this.calculateHeartVector(this.gameState.player1?.stage);
        const p1Blades = this.calculateBladeCount(this.gameState.player1?.stage);
        const p2Hearts = this.calculateHeartVector(this.gameState.player2?.stage);
        const p2Blades = this.calculateBladeCount(this.gameState.player2?.stage);

        console.log('P1 Hearts:', p1Hearts, 'P1 Blades:', p1Blades);
        console.log('P2 Hearts:', p2Hearts, 'P2 Blades:', p2Blades);
        
        // Clear old heart icons and counts
        this.p1HeartIcons.forEach(icon => icon.destroy());
        this.p1HeartCounts.forEach(count => count.destroy());
        this.p1HeartIcons = [];
        this.p1HeartCounts = [];
        
        this.p2HeartIcons.forEach(icon => icon.destroy());
        this.p2HeartCounts.forEach(count => count.destroy());
        this.p2HeartIcons = [];
        this.p2HeartCounts = [];
        
        // Create dynamic heart icons for player 1 with responsive sizing
        const h = this.scale.height;
        const iconScale = Math.max(0.35, Math.min(0.55, h * 0.0007));
        const heartCountSize = Math.max(10, Math.floor(h * 0.014));
        const heartSpacing = Math.max(20, Math.floor(h * 0.025));
        
        let xOffset = 40;
        for (const [heartType, count] of Object.entries(p1Hearts).sort()) {
            if (count > 0) {
                // Extract the number from heartType (e.g., "heart01" -> "01", "b_heart01" -> "b_01")
                let heartNum = heartType;
                if (heartType.startsWith('heart')) {
                    heartNum = heartType.replace('heart', '');
                } else if (heartType.startsWith('b_heart')) {
                    heartNum = 'b_' + heartType.replace('b_heart', '');
                }
                // Ensure heartNum is 2 digits for icon key (e.g., "01", "b_01")
                if (!heartNum.startsWith('b_')) {
                    heartNum = heartNum.padStart(2, '0');
                }
                const iconKey = `heart_${heartNum}`;
                const heartIcon = this.add.image(xOffset, 0, iconKey).setScale(iconScale).setOrigin(0, 0.5);
                // Get actual heart icon width to position text correctly
                const heartTexture = this.textures.get(iconKey);
                const heartWidth = heartTexture.getSourceImage().width * iconScale;
                const heartCount = this.add.text(xOffset + heartWidth + 5, 0, count.toString(), {
                    fontSize: `${heartCountSize}px`,
                    color: '#48bb78',
                    fontStyle: 'bold'
                }).setOrigin(0, 0.5);

                this.p1HeartIcons.push(heartIcon);
                this.p1HeartCounts.push(heartCount);
                this.p1StatsContainer.add([heartIcon, heartCount]);
                xOffset += heartWidth + 5 + Math.max(15, heartCountSize * 2); // Spacing based on actual content
            }
        }
        
        // Reposition blade icon and count based on heart icons
        const bladeTexture = this.textures.get('icon_blade');
        const bladeWidth = bladeTexture.getSourceImage().width * iconScale;
        this.p1BladeIcon.setScale(iconScale);
        this.p1BladeIcon.setX(xOffset);
        this.p1BladeCount.setX(xOffset + bladeWidth + 5);
        this.p1BladeCount.setFontSize(`${heartCountSize}px`);
        
        // Create dynamic heart icons for player 2 with responsive sizing
        xOffset = 80;
        for (const [heartType, count] of Object.entries(p2Hearts).sort()) {
            if (count > 0) {
                // Extract the number from heartType (e.g., "heart01" -> "01", "b_heart01" -> "b_01")
                let heartNum = heartType;
                if (heartType.startsWith('heart')) {
                    heartNum = heartType.replace('heart', '');
                } else if (heartType.startsWith('b_heart')) {
                    heartNum = 'b_' + heartType.replace('b_heart', '');
                }
                // Ensure heartNum is 2 digits for icon key (e.g., "01", "b_01")
                if (!heartNum.startsWith('b_')) {
                    heartNum = heartNum.padStart(2, '0');
                }
                const iconKey = `heart_${heartNum}`;
                const heartIcon = this.add.image(xOffset, 0, iconKey).setScale(iconScale).setOrigin(0, 0.5);
                // Get actual heart icon width to position text correctly
                const heartTexture = this.textures.get(iconKey);
                const heartWidth = heartTexture.getSourceImage().width * iconScale;
                const heartCount = this.add.text(xOffset + heartWidth + 5, 0, count.toString(), {
                    fontSize: `${heartCountSize}px`,
                    color: '#718096',
                    fontStyle: 'bold'
                }).setOrigin(0, 0.5);

                this.p2HeartIcons.push(heartIcon);
                this.p2HeartCounts.push(heartCount);
                this.p2StatsContainer.add([heartIcon, heartCount]);
                xOffset += heartWidth + 5 + Math.max(15, heartCountSize * 2); // Spacing based on actual content
            }
        }
        
        // Reposition blade icon and count based on heart icons
        this.p2BladeIcon.setScale(iconScale);
        this.p2BladeIcon.setX(xOffset);
        this.p2BladeCount.setX(xOffset + bladeWidth + 5);
        this.p2BladeCount.setFontSize(`${heartCountSize}px`);
        
        if (this.p1BladeCount) {
            this.p1BladeCount.setText(p1Blades);
        }
        if (this.p2BladeCount) {
            this.p2BladeCount.setText(p2Blades);
        }
        
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
        
        // Update deck and waitroom counters for both players
        if (this.p1DeckCounters) {
            this.p1DeckCounters.setText(
                `You: Deck ${this.gameState.player1.main_deck_count} | Energy ${this.gameState.player1.energy_deck_count} | Waitroom ${this.gameState.player1.waitroom_count}`
            );
        }
        if (this.p2DeckCounters) {
            this.p2DeckCounters.setText(
                `Opponent: Deck ${this.gameState.player2.main_deck_count} | Energy ${this.gameState.player2.energy_deck_count} | Waitroom ${this.gameState.player2.waitroom_count}`
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
        
        // Calculate responsive card sizes based on zone dimensions
        const cardHeight = Math.max(70, Math.floor(zone.h * 0.7));
        const cardWidth = Math.max(50, Math.floor(cardHeight * 0.72));
        
        // Calculate dynamic overlap based on available space
        const padding = Math.max(10, Math.floor(zone.w * 0.02));
        const maxTotalWidth = zone.w - padding * 2;
        const cardsTotalWidth = cards.length * cardWidth;
        let overlap;
        
        if (cardsTotalWidth <= maxTotalWidth) {
            // No overlap needed - cards fit with spacing
            overlap = cardWidth + Math.max(4, Math.floor(cardWidth * 0.06));
        } else if (cards.length > 1) {
            // Calculate overlap needed to fit all cards
            overlap = (maxTotalWidth - cardWidth) / (cards.length - 1);
            // Ensure minimum overlap
            overlap = Math.max(overlap, Math.max(15, Math.floor(cardWidth * 0.2)));
        } else {
            // Single card, no overlap needed
            overlap = cardWidth;
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
        
        // Calculate responsive card sizes based on zone dimensions
        const cardHeight = Math.max(100, Math.floor(zone.h * 0.65));
        const cardWidth = Math.max(70, Math.floor(cardHeight * 0.72));
        
        // Calculate positions with spacing
        const spacing = Math.max(10, Math.floor(zone.w * 0.05));
        const totalCardsWidth = cardWidth * 3 + spacing * 2;
        const startX = zone.x + (zone.w - totalCardsWidth) / 2;
        
        const positions = [
            { x: startX, y: zone.y + (zone.h - cardHeight) / 2 },
            { x: startX + cardWidth + spacing, y: zone.y + (zone.h - cardHeight) / 2 },
            { x: startX + (cardWidth + spacing) * 2, y: zone.y + (zone.h - cardHeight) / 2 }
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
        
        // Calculate responsive card sizes based on zone dimensions
        const cardHeight = Math.max(50, Math.floor(zone.h * 0.6));
        const cardWidth = Math.max(35, Math.floor(cardHeight * 0.72));
        
        // Calculate dynamic overlap based on available space
        const padding = Math.max(8, Math.floor(zone.w * 0.02));
        const maxTotalWidth = zone.w - padding * 2;
        const cardsTotalWidth = cards.length * cardWidth;
        let overlap;
        
        if (cardsTotalWidth <= maxTotalWidth) {
            // No overlap needed - cards fit with spacing
            overlap = cardWidth + Math.max(2, Math.floor(cardWidth * 0.05));
        } else if (cards.length > 1) {
            // Calculate overlap needed to fit all cards
            overlap = (maxTotalWidth - cardWidth) / (cards.length - 1);
            // Ensure minimum overlap
            overlap = Math.max(overlap, Math.max(10, Math.floor(cardWidth * 0.2)));
        } else {
            // Single card, no overlap needed
            overlap = cardWidth;
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
        
        // Card background with enhanced styling
        const bg = this.add.graphics();
        // Main background with gradient-like effect
        bg.fillStyle(0x2a2a4e, 1);
        bg.fillRoundedRect(0, 0, width, height, Math.max(4, Math.floor(width * 0.06)));
        // Outer border with glow
        bg.lineStyle(2, 0x6666aa, 1);
        bg.strokeRoundedRect(0, 0, width, height, Math.max(4, Math.floor(width * 0.06)));
        // Inner highlight for depth
        bg.lineStyle(1, 0x8888cc, 0.5);
        bg.strokeRoundedRect(2, 2, width - 4, height - 4, Math.max(3, Math.floor(width * 0.05)));
        container.add(bg);
        
        // Get image file name from mapping
        const imageKey = card.card_no;
        const imgFileName = this.cardImageMap ? this.cardImageMap.get(imageKey) : `${imageKey}.webp`;

        // Set container size before rotation for correct hit detection
        container.setSize(width, height);

        // Check orientation - rotate if Wait
        const isWait = card.orientation === 'Wait';

        // Rotate entire container if in Wait state
        if (isWait) {
            container.setRotation(Math.PI / 2);
        }
        
        if (this.textures.exists(imageKey)) {
            // Image already loaded, display it
            const texture = this.textures.get(imageKey);
            const cardImage = this.add.image(width / 2, height / 2, imageKey);

            // Calculate scale to fit image within card dimensions with validation
            let imageWidth = 100;
            let imageHeight = 140;
            if (texture && texture.source && texture.source[0]) {
                imageWidth = texture.source[0].width || 100;
                imageHeight = texture.source[0].height || 140;
            }
            const scaleX = width / imageWidth;
            const scaleY = height / imageHeight;
            const scale = Math.min(scaleX, scaleY) * 0.95; // 95% to leave small margin
            
            cardImage.setScale(scale);
            
            // Remove background when image is loaded
            container.each(child => {
                if (child.type === 'Graphics') {
                    child.destroy();
                }
            });
            
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
                            // Check orientation from stored cardData and apply rotation if needed
                            const isWait = c.cardData && c.cardData.orientation === 'Wait';
                            if (isWait) {
                                c.setRotation(Math.PI / 2);
                            }

                            // Remove the placeholder text and background if they exist
                            c.each(child => {
                                if (child.type === 'Text' || child.type === 'Graphics') {
                                    child.destroy();
                                }
                            });
                            // Add the image
                            const texture = this.textures.get(imageKey);
                            const cardImage = this.add.image(c.width / 2, c.height / 2, imageKey);

                            // Calculate scale to fit image within card dimensions with validation
                            let imageWidth = 100;
                            let imageHeight = 140;
                            if (texture && texture.source && texture.source[0]) {
                                imageWidth = texture.source[0].width || 100;
                                imageHeight = texture.source[0].height || 140;
                            }
                            const scaleX = c.width / imageWidth;
                            const scaleY = c.height / imageHeight;
                            const scale = Math.min(scaleX, scaleY) * 0.95;

                            cardImage.setScale(scale);

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
            
            // Fallback: show card name while loading with responsive font size
            const placeholderSize = Math.max(8, Math.floor(width * 0.12));
            const nameText = this.add.text(width / 2, height / 2, card.name || card.card_no || '?', {
                fontSize: `${placeholderSize}px`,
                color: '#fff',
                align: 'center',
                wordWrap: { width: width - Math.max(6, Math.floor(width * 0.1)) }
            }).setOrigin(0.5);
            container.add(nameText);
        }

        this.cardContainers.push(container);
    }

    async loadActions() {
        // If already loading, mark as pending and return
        if (this.loadingActions) {
            this.pendingActionsLoad = true;
            return;
        }

        this.loadingActions = true;
        this.pendingActionsLoad = false;

        try {
            const response = await fetch('/api/actions');
            const data = await response.json();
            this.actions = data.actions;
            this.createActionButtons();
        } catch (error) {
            console.error('Failed to load actions:', error);
        } finally {
            this.loadingActions = false;

            // If a load was pending, trigger it now
            if (this.pendingActionsLoad) {
                this.pendingActionsLoad = false;
                // Small delay to ensure UI has time to update
                setTimeout(() => this.loadActions(), 50);
            }
        }
    }

    createActionButtons() {
        // Clear existing buttons
        this.actionButtons.forEach(btn => btn.destroy());
        this.actionButtons = [];
        
        const panelX = this.actionsPanelX;
        const panelWidth = this.actionsPanelWidth;
        const h = this.scale.height;
        
        // Calculate responsive sizes
        const startY = this.actionsPanelY + Math.max(40, Math.floor(h * 0.05));
        const buttonWidth = panelWidth - Math.max(20, Math.floor(panelWidth * 0.08));
        
        // Calculate responsive font sizes
        const skipBtnSize = Math.max(16, Math.floor(h * 0.022));
        const titleSize = Math.max(12, Math.floor(h * 0.017));
        const smallBtnSize = Math.max(10, Math.floor(h * 0.014));
        const regularBtnSize = Math.max(11, Math.floor(h * 0.016));
        const noActionsSize = Math.max(13, Math.floor(h * 0.018));
        
        // Calculate responsive spacing
        const skipBtnSpacing = Math.max(50, Math.floor(h * 0.06));
        const regularSpacing = Math.max(42, Math.floor(h * 0.05));
        const afterGroupedSpacing = Math.max(25, Math.floor(h * 0.035));
        const afterRegularSpacing = Math.max(15, Math.floor(h * 0.022));
        
        // Calculate responsive padding
        const skipBtnPadding = { x: Math.max(15, Math.floor(buttonWidth * 0.05)), y: Math.max(8, Math.floor(h * 0.012)) };
        const titlePadding = { x: Math.max(6, Math.floor(buttonWidth * 0.02)), y: Math.max(3, Math.floor(h * 0.005)) };
        const smallBtnPadding = { x: Math.max(5, Math.floor(buttonWidth * 0.015)), y: Math.max(3, Math.floor(h * 0.005)) };
        const regularBtnPadding = { x: Math.max(8, Math.floor(buttonWidth * 0.025)), y: Math.max(4, Math.floor(h * 0.006)) };
        
        // Track actual Y position instead of using index
        let currentY = startY;
        
        // Separate Pass/Skip/Finish actions and make them prominent
        let skipActions = [];
        
        // Find skip/finish/pass actions
        this.actions.forEach((action) => {
            if (action.action_type === 'pass' || action.action_type === 'skip_mulligan' || action.action_type === 'finish_live_card_set') {
                skipActions.push(action);
            }
        });
        
        // Skip/Finish/Pass buttons - large and prominent at the top
        skipActions.forEach((skipAction) => {
            let buttonText = 'PASS TURN';
            if (skipAction.action_type === 'skip_mulligan') {
                buttonText = 'SKIP MULLIGAN';
            } else if (skipAction.action_type === 'finish_live_card_set') {
                buttonText = 'FINISH LIVE SET';
            }
            
            const skipBtn = this.add.text(panelX + panelWidth / 2, currentY, buttonText, {
                fontSize: `${skipBtnSize}px`,
                color: '#ffffff',
                backgroundColor: '#e94560',
                padding: skipBtnPadding,
                fontStyle: 'bold',
                fixedWidth: buttonWidth,
                align: 'center',
                borderRadius: 10,
                shadow: { blur: 8, color: '#e94560', fill: true }
            }).setOrigin(0.5);
            
            // Capture skip action in closure
            const capturedSkipAction = skipAction;
            skipBtn.setInteractive({ useHandCursor: true })
                   .on('pointerover', () => skipBtn.setStyle({ 
                       backgroundColor: '#ff6b6b',
                       shadow: { blur: 12, color: '#ff6b6b', fill: true }
                   }))
                   .on('pointerout', () => skipBtn.setStyle({ 
                       backgroundColor: '#e94560',
                       shadow: { blur: 8, color: '#e94560', fill: true }
                   }))
                   .on('pointerdown', () => this.executeActionDirect(capturedSkipAction));
            
            this.actionButtons.push(skipBtn);
            currentY += skipBtnSpacing;
        });
        
        // Other action buttons - check if they have available_areas for grouping
        this.actions.forEach((action, originalIndex) => {
            // Skip skip/finish/pass actions as they're already handled
            if (action.action_type === 'pass' || action.action_type === 'skip_mulligan' || action.action_type === 'finish_live_card_set') return;
            
            const params = action.parameters;
            
            // Check if this is a grouped card action with available_areas
            if (params && params.available_areas && params.available_areas.length > 0) {
                // Card title with base cost
                const titleText = this.add.text(panelX + Math.max(10, Math.floor(panelWidth * 0.04)), currentY, action.description, {
                    fontSize: `${titleSize}px`,
                    color: '#e94560',
                    fontStyle: 'bold',
                    backgroundColor: '#1a202c',
                    padding: titlePadding,
                    borderRadius: 6
                }).setOrigin(0, 0.5);
                this.actionButtons.push(titleText);
                
                // Area buttons for left, center, right
                const areas = params.available_areas;
                const buttonWidthSmall = (buttonWidth - Math.max(15, Math.floor(buttonWidth * 0.05))) / 3;
                const smallBtnX = panelX + Math.max(10, Math.floor(panelWidth * 0.04));
                const areaButtonOffset = Math.max(22, Math.floor(h * 0.03));
                
                areas.forEach((areaInfo, i) => {
                    if (areaInfo.available) {
                        const label = areaInfo.area.charAt(0).toUpperCase() + areaInfo.area.slice(1);
                        const costText = areaInfo.is_baton_touch ? 
                            `${label} (${areaInfo.cost} - Baton)` : 
                            `${label} (${areaInfo.cost})`;
                        
                        const btn = this.add.text(smallBtnX + i * buttonWidthSmall, currentY + areaButtonOffset, costText, {
                            fontSize: `${smallBtnSize}px`,
                            color: '#fff',
                            backgroundColor: areaInfo.is_baton_touch ? '#48bb78' : '#2d3748',
                            padding: smallBtnPadding,
                            fixedWidth: buttonWidthSmall - Math.max(3, Math.floor(buttonWidth * 0.01)),
                            align: 'center',
                            borderRadius: 6,
                            shadow: { blur: 4, color: '#000000', fill: true }
                        }).setOrigin(0, 0.5);
                        
                        // Capture action and areaInfo in closure
                        const capturedAction = action;
                        const capturedAreaInfo = areaInfo;
                        btn.setInteractive({ useHandCursor: true })
                           .on('pointerover', () => btn.setStyle({ 
                               backgroundColor: capturedAreaInfo.is_baton_touch ? '#68d391' : '#4a5568', 
                               color: '#e94560',
                               shadow: { blur: 6, color: '#e94560', fill: true }
                           }))
                           .on('pointerout', () => btn.setStyle({ 
                               backgroundColor: capturedAreaInfo.is_baton_touch ? '#48bb78' : '#2d3748', 
                               color: '#fff',
                               shadow: { blur: 4, color: '#000000', fill: true }
                           }))
                           .on('pointerdown', () => this.executeActionWithAreaDirect(capturedAction, capturedAreaInfo.area));
                        
                        this.actionButtons.push(btn);
                    }
                });
                
                // Move Y position to account for title + area buttons
                currentY += areaButtonOffset + afterGroupedSpacing;
            } else {
                // Regular action button (not grouped)
                const btn = this.add.text(panelX + panelWidth / 2, currentY, action.description, {
                    fontSize: `${regularBtnSize}px`,
                    color: '#fff',
                    backgroundColor: '#2d3748',
                    padding: regularBtnPadding,
                    fixedWidth: buttonWidth,
                    align: 'center',
                    wordWrap: { width: buttonWidth - Math.max(25, Math.floor(buttonWidth * 0.08)) },
                    borderRadius: 8,
                    shadow: { blur: 4, color: '#000000', fill: true }
                }).setOrigin(0.5);
                
                // Capture action in closure
                const capturedAction = action;
                btn.setInteractive({ useHandCursor: true })
                   .on('pointerover', () => btn.setStyle({ 
                       backgroundColor: '#4a5568', 
                       color: '#e94560',
                       shadow: { blur: 6, color: '#e94560', fill: true }
                   }))
                   .on('pointerout', () => btn.setStyle({ 
                       backgroundColor: '#2d3748', 
                       color: '#fff',
                       shadow: { blur: 4, color: '#000000', fill: true }
                   }))
                   .on('pointerdown', () => this.executeActionDirect(capturedAction));
                
                this.actionButtons.push(btn);
                currentY += regularSpacing + afterRegularSpacing;
            }
        });
        
        // No actions message
        if (this.actions.length === 0) {
            const noActions = this.add.text(panelX + panelWidth / 2, startY + Math.max(50, Math.floor(h * 0.06)), 'No actions available', {
                fontSize: `${noActionsSize}px`,
                color: '#718096',
                fontStyle: 'italic'
            }).setOrigin(0.5);
            this.actionButtons.push(noActions);
        }
    }

    async executeActionWithAreaDirect(action, stageArea) {
        try {
            // Find the selected area info to get use_baton_touch
            const areaInfo = action.parameters?.available_areas?.find(a => a.area === stageArea);
            
            const modifiedAction = {
                action_index: 0, // Not used by backend anymore
                stage_area: stageArea,
                action_type: action.action_type,
                card_id: action.parameters?.card_id,
                card_index: action.parameters?.card_index, // Keep for backward compatibility
                card_indices: action.parameters?.card_indices,
                card_no: action.parameters?.card_no,
                use_baton_touch: areaInfo?.is_baton_touch
            };
            
            const response = await fetch('/api/execute-action', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify(modifiedAction)
            });
            
            if (response.ok) {
                await this.loadGameState();
            } else {
                const errorText = await response.text();
                console.error('Action execution failed:', response.status, errorText);
            }
        } catch (error) {
            console.error('Failed to execute action:', error);
        }
    }

    async executeActionDirect(action) {
        try {
            const response = await fetch('/api/execute-action', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ 
                    action_index: 0, // Not used by backend anymore
                    action_type: action.action_type,
                    card_id: action.parameters?.card_id,
                    card_index: action.parameters?.card_index, // Keep for backward compatibility
                    card_indices: action.parameters?.card_indices,
                    card_no: action.parameters?.card_no,
                    use_baton_touch: action.parameters?.use_baton_touch
                })
            });
            
            if (response.ok) {
                await this.loadGameState();
            } else {
                const errorText = await response.text();
                console.error('Action execution failed:', response.status, errorText);
            }
        } catch (error) {
            console.error('Failed to execute action:', error);
        }
    }

    async executeAction(index) {
        try {
            const action = this.actions[index];
            const response = await fetch('/api/execute-action', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ 
                    action_index: index,
                    action_type: action.action_type,
                    card_id: action.parameters?.card_id,
                    card_index: action.parameters?.card_index,
                    card_indices: action.parameters?.card_indices,
                    card_no: action.parameters?.card_no,
                    use_baton_touch: action.parameters?.use_baton_touch
                })
            });
            
            if (response.ok) {
                await this.loadGameState();
            } else {
                const errorText = await response.text();
                console.error('Action execution failed:', response.status, errorText);
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
