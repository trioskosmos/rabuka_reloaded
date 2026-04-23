const fs = require('fs');
const path = require('path');

// Read cards.json
const cardsPath = path.join(__dirname, 'cards', 'cards.json');
const cardsData = JSON.parse(fs.readFileSync(cardsPath, 'utf8'));

const webpDir = path.join(__dirname, 'web_ui', 'img', 'cards_webp');
const mapping = {};
const totalCards = Object.keys(cardsData).length;

console.log(`Building card_no to webp mapping with enhanced rules...\n`);

// Get actual webp filenames for fuzzy matching
const actualWebpFiles = fs.readdirSync(webpDir).filter(f => f.endsWith('.webp'));
const webpSet = new Set(actualWebpFiles.map(f => f.replace('.webp', '')));

Object.entries(cardsData).forEach(([cardNo, card]) => {
    const imgPath = card._img || card.img;
    let imgFileName = `${cardNo}.webp`; // Default fallback
    
    if (imgPath) {
        // Extract filename from path
        const parts = imgPath.split('/');
        let filename = parts[parts.length - 1] || imgPath;
        // Replace common image extensions with .webp
        imgFileName = filename.replace(/\.(png|jpg|jpeg|gif)$/i, '.webp');
        // If no extension, add .webp
        if (!imgFileName.includes('.')) {
            imgFileName += '.webp';
        }
    }
    
    // Check if this webp file exists
    let webpPath = path.join(webpDir, imgFileName);
    if (fs.existsSync(webpPath)) {
        mapping[cardNo] = `img/cards_webp/${imgFileName}`;
    } else {
        // Try additional rules for missing cards
        
        // Rule 1: Full-width plus sign (＋) -> digit 2
        let altFileName = imgFileName.replace(/＋/g, '2');
        webpPath = path.join(webpDir, altFileName);
        if (fs.existsSync(webpPath)) {
            mapping[cardNo] = `img/cards_webp/${altFileName}`;
            return;
        }
        
        // Rule 2: Remove "proteinbar" suffix
        altFileName = imgFileName.replace(/proteinbar/g, '');
        webpPath = path.join(webpDir, altFileName);
        if (fs.existsSync(webpPath)) {
            mapping[cardNo] = `img/cards_webp/${altFileName}`;
            return;
        }
        
        // Rule 3: Replace "protein" with empty
        altFileName = imgFileName.replace(/protein/g, '');
        webpPath = path.join(webpDir, altFileName);
        if (fs.existsSync(webpPath)) {
            mapping[cardNo] = `img/cards_webp/${altFileName}`;
            return;
        }
        
        // Rule 4: Try fuzzy matching - find webp file with similar base
        const baseParts = cardNo.split('-');
        const potentialMatches = actualWebpFiles.filter(f => {
            const webpBase = f.replace('.webp', '');
            const webpParts = webpBase.split('-');
            // Match series and number
            return webpParts[0] === baseParts[0] && webpParts[webpParts.length - 1] === baseParts[baseParts.length - 1];
        });
        
        if (potentialMatches.length > 0) {
            mapping[cardNo] = `img/cards_webp/${potentialMatches[0]}`;
            return;
        }
        
        mapping[cardNo] = null; // No match found
    }
});

const foundCount = Object.values(mapping).filter(v => v !== null).length;
const missingCount = Object.values(mapping).filter(v => v === null).length;

console.log(`Total cards: ${totalCards}`);
console.log(`Found images: ${foundCount}`);
console.log(`Missing images: ${missingCount}`);

// Write mapping to file
const mappingPath = path.join(__dirname, 'web_ui', 'js', 'card_image_mapping.json');
fs.writeFileSync(mappingPath, JSON.stringify(mapping, null, 2));
console.log(`\nMapping written to: ${mappingPath}`);

if (missingCount > 0) {
    console.log(`\n=== ALL MISSING MAPPINGS ===`);
    Object.entries(mapping).filter(([k, v]) => v === null).forEach(([cardNo]) => {
        const card = cardsData[cardNo];
        console.log(`  ${cardNo} (_img: ${card._img})`);
    });
}

console.log(`\nDone.`);
