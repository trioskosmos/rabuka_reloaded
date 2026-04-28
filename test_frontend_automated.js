const puppeteer = require('puppeteer');

async function testFrontendTurnChoice() {
    console.log('=== AUTOMATED FRONTEND TEST ===');
    
    const browser = await puppeteer.launch({ headless: false });
    const page = await browser.newPage();
    
    try {
        // Navigate to frontend
        console.log('1. Opening frontend...');
        await page.goto('http://localhost:3000');
        await page.waitForTimeout(2000);
        
        // Open game setup modal
        console.log('2. Opening game setup...');
        await page.click('[data-action="open-setup-modal"]');
        await page.waitForTimeout(1000);
        
        // Select decks
        console.log('3. Selecting decks...');
        await page.select('#p0-deck-select', 'Aqours Cup');
        await page.select('#p1-deck-select', 'Aqours Cup');
        await page.waitForTimeout(500);
        
        // Start game
        console.log('4. Starting game...');
        await page.click('[data-action="submit-game-setup"]');
        await page.waitForTimeout(3000);
        
        // Check initial phase
        console.log('5. Checking initial phase...');
        const initialPhase = await page.evaluate(() => {
            return window.State?.data?.phase;
        });
        console.log('Initial phase:', initialPhase);
        
        if (initialPhase !== 'RockPaperScissors') {
            console.log('❌ Wrong initial phase, expected RockPaperScissors');
            return false;
        }
        
        // Get RPS actions
        console.log('6. Getting RPS actions...');
        const rpsActions = await page.evaluate(() => {
            return window.State?.data?.legal_actions?.filter(a => 
                a.action_type === 'rock_choice' || 
                a.action_type === 'paper_choice' || 
                a.action_type === 'scissors_choice'
            ) || [];
        });
        console.log('RPS actions:', rpsActions.length);
        
        if (rpsActions.length !== 3) {
            console.log('❌ Wrong number of RPS actions');
            return false;
        }
        
        // Click Rock
        console.log('7. Clicking Rock...');
        const rockButton = await page.evaluateHandle(() => {
            const actions = window.State?.data?.legal_actions || [];
            const rockAction = actions.find(a => a.action_type === 'rock_choice');
            return document.querySelector(`[data-action-id="${rockAction?.index}"]`);
        });
        if (rockButton) {
            await rockButton.click();
            await page.waitForTimeout(1000);
        }
        
        // Click Paper
        console.log('8. Clicking Paper...');
        const paperButton = await page.evaluateHandle(() => {
            const actions = window.State?.data?.legal_actions || [];
            const paperAction = actions.find(a => a.action_type === 'paper_choice');
            return document.querySelector(`[data-action-id="${paperAction?.index}"]`);
        });
        if (paperButton) {
            await paperButton.click();
            await page.waitForTimeout(1000);
        }
        
        // Check phase after RPS
        console.log('9. Checking phase after RPS...');
        const phaseAfterRPS = await page.evaluate(() => {
            return window.State?.data?.phase;
        });
        console.log('Phase after RPS:', phaseAfterRPS);
        
        if (phaseAfterRPS !== 'ChooseFirstAttacker') {
            console.log('❌ Wrong phase after RPS, expected ChooseFirstAttacker');
            return false;
        }
        
        // Get turn choice actions
        console.log('10. Getting turn choice actions...');
        const turnActions = await page.evaluate(() => {
            return window.State?.data?.legal_actions?.filter(a => 
                a.action_type === 'choose_first_attacker' || 
                a.action_type === 'choose_second_attacker'
            ) || [];
        });
        console.log('Turn choice actions:', turnActions.length);
        
        if (turnActions.length !== 2) {
            console.log('❌ Wrong number of turn choice actions');
            return false;
        }
        
        // Click "Go first"
        console.log('11. Clicking Go first...');
        const goFirstButton = await page.evaluateHandle(() => {
            const actions = window.State?.data?.legal_actions || [];
            const goFirstAction = actions.find(a => a.action_type === 'choose_first_attacker');
            return document.querySelector(`[data-action-id="${goFirstAction?.index}"]`);
        });
        if (goFirstButton) {
            await goFirstButton.click();
            await page.waitForTimeout(2000);
        }
        
        // Check phase after turn choice
        console.log('12. Checking phase after turn choice...');
        const phaseAfterTurn = await page.evaluate(() => {
            return window.State?.data?.phase;
        });
        console.log('Phase after turn choice:', phaseAfterTurn);
        
        if (phaseAfterTurn !== 'MulliganP1Turn' && phaseAfterTurn !== 'MulliganP2Turn') {
            console.log('❌ Wrong phase after turn choice, expected Mulligan phase');
            console.log('Got:', phaseAfterTurn);
            return false;
        }
        
        // Get mulligan actions
        console.log('13. Getting mulligan actions...');
        const mulliganActions = await page.evaluate(() => {
            return window.State?.data?.legal_actions?.filter(a => 
                a.action_type === 'confirm_mulligan' || 
                a.action_type === 'skip_mulligan'
            ) || [];
        });
        console.log('Mulligan actions:', mulliganActions.length);
        
        if (mulliganActions.length < 1) {
            console.log('❌ No mulligan actions available');
            return false;
        }
        
        console.log('\n=== TEST PASSED ===');
        console.log('Turn choice phase correctly advances to mulligan');
        return true;
        
    } catch (error) {
        console.error('\n=== TEST FAILED ===');
        console.error('Error:', error.message);
        return false;
    } finally {
        await browser.close();
    }
}

testFrontendTurnChoice().then(success => {
    process.exit(success ? 0 : 1);
}).catch(error => {
    console.error('Test error:', error);
    process.exit(1);
});
