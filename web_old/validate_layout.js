// Layout validation script for Rabuka web app
// This script mathematically validates spacing calculations to ensure elements don't overlap

function validateMenuLayout(width, height) {
    console.log('=== MENU LAYOUT VALIDATION ===');
    const issues = [];
    
    const minDim = Math.min(width, height);
    const titleSize = Math.max(32, Math.floor(minDim * 0.05));
    const subtitleSize = Math.max(18, Math.floor(minDim * 0.028));
    const labelSize = Math.max(14, Math.floor(minDim * 0.022));
    const buttonSize = Math.max(16, Math.floor(minDim * 0.025));
    const startButtonSize = Math.max(22, Math.floor(minDim * 0.035));
    const buttonSpacing = Math.max(20, Math.floor(height * 0.025));
    
    const titleY = height * 0.18;
    const subtitleY = height * 0.26;
    const labelY = height * 0.35;
    const startY = height * 0.40;
    const startBtnY = height * 0.75;
    
    const decks = ['Aqours Cup', 'Muse Cup', 'Nijigaku Cup', 'Liella Cup', 'Hasunosora Cup', 'Fade Deck'];
    const lastDeckY = startY + (decks.length - 1) * buttonSpacing;
    
    // Check spacing between elements
    const titleToSubtitle = subtitleY - titleY;
    const subtitleToLabel = labelY - subtitleY;
    const labelToFirstDeck = startY - labelY;
    const lastDeckToStart = startBtnY - lastDeckY;
    
    console.log(`Screen: ${width}x${height}`);
    console.log(`Title Y: ${titleY}px, Subtitle Y: ${subtitleY}px (gap: ${titleToSubtitle}px)`);
    console.log(`Label Y: ${labelY}px (gap: ${subtitleToLabel}px)`);
    console.log(`First deck Y: ${startY}px (gap: ${labelToFirstDeck}px)`);
    console.log(`Last deck Y: ${lastDeckY}px, Start button Y: ${startBtnY}px (gap: ${lastDeckToStart}px)`);
    console.log(`Button spacing: ${buttonSpacing}px`);
    
    if (titleToSubtitle < 20) issues.push('Title to subtitle gap too small');
    if (subtitleToLabel < 15) issues.push('Subtitle to label gap too small');
    if (labelToFirstDeck < 12) issues.push('Label to first deck gap too small');
    if (lastDeckToStart < 30) issues.push('Last deck to start button gap too small');
    if (startBtnY > height * 0.95) issues.push('Start button too close to bottom');
    
    return issues;
}

function validateGameLayout(width, height) {
    console.log('\n=== GAME LAYOUT VALIDATION ===');
    const issues = [];
    
    const headerHeight = Math.max(60, Math.floor(height * 0.08));
    const rightPanelWidth = Math.max(280, Math.floor(width * 0.25));
    const zoneGap = Math.max(6, Math.floor(height * 0.008));
    const zonePadding = Math.max(15, Math.floor(width * 0.015));
    
    const playAreaWidth = width - rightPanelWidth;
    const playAreaHeight = height - headerHeight;
    
    const handHeight = Math.max(80, Math.floor(playAreaHeight * 0.12));
    const stageHeight = Math.max(120, Math.floor(playAreaHeight * 0.20));
    const supportHeight = Math.max(70, Math.floor(playAreaHeight * 0.10));
    
    // Calculate zone positions
    const p2HandY = headerHeight + zoneGap;
    const p2StageY = p2HandY + handHeight + zoneGap;
    const p2SupportY = p2StageY + stageHeight + zoneGap;
    
    const p1HandY = height - handHeight - zoneGap;
    const p1StageY = p1HandY - stageHeight - zoneGap;
    const p1SupportY = p1StageY - supportHeight - zoneGap;
    
    console.log(`Screen: ${width}x${height}`);
    console.log(`Header: ${headerHeight}px, Right panel: ${rightPanelWidth}px`);
    console.log(`Play area: ${playAreaWidth}x${playAreaHeight}px`);
    console.log(`Zone gap: ${zoneGap}px`);
    console.log(`Hand height: ${handHeight}px, Stage height: ${stageHeight}px, Support height: ${supportHeight}px`);
    
    console.log(`\nP2 zones (top):`);
    console.log(`  Hand: Y=${p2HandY}px, H=${handHeight}px`);
    console.log(`  Stage: Y=${p2StageY}px, H=${stageHeight}px (gap: ${p2StageY - (p2HandY + handHeight)}px)`);
    console.log(`  Support: Y=${p2SupportY}px, H=${supportHeight}px (gap: ${p2SupportY - (p2StageY + stageHeight)}px)`);
    
    console.log(`\nP1 zones (bottom):`);
    console.log(`  Hand: Y=${p1HandY}px, H=${handHeight}px`);
    console.log(`  Stage: Y=${p1StageY}px, H=${stageHeight}px (gap: ${p1HandY - (p1StageY + stageHeight)}px)`);
    console.log(`  Support: Y=${p1SupportY}px, H=${supportHeight}px (gap: ${p1StageY - (p1SupportY + supportHeight)}px)`);
    
    // Check for overlaps
    const p2HandBottom = p2HandY + handHeight;
    const p2StageBottom = p2StageY + stageHeight;
    const p2SupportBottom = p2SupportY + supportHeight;
    
    const p1HandTop = p1HandY;
    const p1StageTop = p1StageY;
    const p1SupportTop = p1SupportY;
    
    // Check P2 zone overlaps
    if (p2HandBottom > p2StageY) issues.push('P2 Hand overlaps with P2 Stage');
    if (p2StageBottom > p2SupportY) issues.push('P2 Stage overlaps with P2 Support');
    
    // Check P1 zone overlaps
    if (p1StageTop + stageHeight > p1HandY) issues.push('P1 Stage overlaps with P1 Hand');
    if (p1SupportTop + supportHeight > p1StageY) issues.push('P1 Support overlaps with P1 Stage');
    
    // Check if P2 and P1 zones overlap each other
    if (p2SupportBottom > p1SupportTop) issues.push('P2 Support overlaps with P1 Support');
    
    // Check minimum gaps
    if (zoneGap < 4) issues.push('Zone gap too small, may cause visual crowding');
    if (handHeight < 60) issues.push('Hand height too small for cards');
    if (stageHeight < 100) issues.push('Stage height too small for cards');
    
    // Check if zones fit in play area
    const totalP2Height = p2SupportBottom - p2HandY;
    const totalP1Height = p1HandY - p1SupportTop;
    const middleSpace = p1SupportTop - p2SupportBottom;
    
    console.log(`\nTotal P2 height: ${totalP2Height}px, Total P1 height: ${totalP1Height}px`);
    console.log(`Middle space: ${middleSpace}px`);
    
    if (middleSpace < 20) issues.push('Middle space between P2 and P1 zones too small');
    
    return issues;
}

function validateHeaderLayout(width, height) {
    console.log('\n=== HEADER LAYOUT VALIDATION ===');
    const issues = [];
    
    const headerHeight = Math.max(60, Math.floor(height * 0.08));
    const rightPanelWidth = Math.max(280, Math.floor(width * 0.25));
    
    const menuBtnPadding = Math.max(10, Math.floor(width * 0.012));
    const menuBtnSize = Math.max(14, Math.floor(height * 0.018));
    const titleSize = Math.max(20, Math.floor(height * 0.028));
    const statsSize = Math.max(12, Math.floor(height * 0.016));
    const turnSize = Math.max(16, Math.floor(height * 0.022));
    const phaseSize = Math.max(14, Math.floor(height * 0.02));
    
    const menuBtnX = menuBtnPadding;
    const menuBtnWidth = menuBtnSize * 4 + menuBtnPadding * 2; // Estimated
    const titleX = width / 2;
    const p1StatsX = menuBtnPadding * 5 + 60;
    const p2StatsX = p1StatsX + 140;
    const turnPhaseSpacing = Math.max(60, Math.floor(width * 0.05));
    const turnTextX = width - rightPanelWidth - turnPhaseSpacing;
    const phaseTextX = width - rightPanelWidth - 20;
    
    console.log(`Header height: ${headerHeight}px`);
    console.log(`Menu button X: ${menuBtnX}px (width est: ${menuBtnWidth}px)`);
    console.log(`Title X: ${titleX}px`);
    console.log(`P1 stats X: ${p1StatsX}px, P2 stats X: ${p2StatsX}px`);
    console.log(`Turn text X: ${turnTextX}px, Phase text X: ${phaseTextX}px`);
    
    // Check for overlaps
    const menuBtnRight = menuBtnX + menuBtnWidth;
    const p1StatsLeft = p1StatsX;
    const p2StatsRight = p2StatsX + 100; // Estimated width
    const turnTextLeft = turnTextX - 50; // Estimated half-width
    
    if (menuBtnRight > p1StatsLeft - 10) issues.push('Menu button may overlap with P1 stats');
    if (p2StatsRight > turnTextLeft - 10) issues.push('P2 stats may overlap with turn text');
    if (phaseTextX > width - 15) issues.push('Phase text too close to right edge');
    
    // Check if header fits
    if (headerHeight < 50) issues.push('Header height too small for content');
    if (headerHeight > height * 0.15) issues.push('Header height too large, wasting space');
    
    return issues;
}

function validateActionPanel(width, height) {
    console.log('\n=== ACTION PANEL VALIDATION ===');
    const issues = [];
    
    const headerHeight = Math.max(60, Math.floor(height * 0.08));
    const rightPanelWidth = Math.max(280, Math.floor(width * 0.25));
    const panelHeight = height - headerHeight;
    
    const startY = headerHeight + Math.max(30, Math.floor(height * 0.04));
    const skipBtnSpacing = Math.max(35, Math.floor(height * 0.045));
    const regularSpacing = Math.max(30, Math.floor(height * 0.04));
    const afterGroupedSpacing = Math.max(18, Math.floor(height * 0.025));
    const afterRegularSpacing = Math.max(10, Math.floor(height * 0.015));
    
    console.log(`Panel: ${rightPanelWidth}x${panelHeight}px`);
    console.log(`Start Y: ${startY}px`);
    console.log(`Skip button spacing: ${skipBtnSpacing}px`);
    console.log(`Regular spacing: ${regularSpacing}px`);
    console.log(`After grouped spacing: ${afterGroupedSpacing}px`);
    console.log(`After regular spacing: ${afterRegularSpacing}px`);
    
    // Estimate how many buttons fit
    const availableHeight = panelHeight - (startY - headerHeight) - 20;
    const estimatedSkipButtons = Math.floor(availableHeight / skipBtnSpacing);
    const estimatedRegularButtons = Math.floor(availableHeight / (regularSpacing + afterRegularSpacing));
    
    console.log(`Available height: ${availableHeight}px`);
    console.log(`Estimated skip buttons that fit: ${estimatedSkipButtons}`);
    console.log(`Estimated regular buttons that fit: ${estimatedRegularButtons}`);
    
    if (skipBtnSpacing < 25) issues.push('Skip button spacing too small');
    if (regularSpacing < 20) issues.push('Regular button spacing too small');
    if (estimatedSkipButtons < 2) issues.push('Action panel may not fit enough buttons');
    
    return issues;
}

// Run validations for common screen sizes
const screenSizes = [
    { width: 1920, height: 1080, name: '1920x1080 (Desktop)' },
    { width: 1366, height: 768, name: '1366x768 (Laptop)' },
    { width: 1280, height: 720, name: '1280x720 (Small laptop)' },
    { width: 768, height: 1024, name: '768x1024 (Tablet portrait)' },
    { width: 375, height: 667, name: '375x667 (Mobile)' }
];

screenSizes.forEach(screen => {
    console.log(`\n${'='.repeat(50)}`);
    console.log(`VALIDATING: ${screen.name}`);
    console.log('='.repeat(50));
    
    const menuIssues = validateMenuLayout(screen.width, screen.height);
    const gameIssues = validateGameLayout(screen.width, screen.height);
    const headerIssues = validateHeaderLayout(screen.width, screen.height);
    const actionIssues = validateActionPanel(screen.width, screen.height);
    
    const allIssues = [...menuIssues, ...gameIssues, ...headerIssues, ...actionIssues];
    
    if (allIssues.length === 0) {
        console.log('\n✓ No layout issues detected');
    } else {
        console.log('\n✗ ISSUES FOUND:');
        allIssues.forEach(issue => console.log(`  - ${issue}`));
    }
});

console.log('\n' + '='.repeat(50));
console.log('VALIDATION COMPLETE');
console.log('='.repeat(50));
