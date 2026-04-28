const http = require('http');

// Test the complete game flow
async function testGameFlow() {
    console.log('Testing game flow...');
    
    try {
        // 1. Initialize game
        console.log('1. Initializing game...');
        const initResponse = await makeRequest('POST', '/api/init', {
            deck1: "Aqours Cup",
            deck2: "Aqours Cup"
        });
        console.log('Game initialized:', JSON.parse(initResponse).phase);
        
        // 2. Get initial state
        console.log('2. Getting initial state...');
        const stateResponse = await makeRequest('GET', '/api/game-state');
        const gameState = JSON.parse(stateResponse);
        console.log('Initial phase:', gameState.phase);
        
        // 3. Get actions
        console.log('3. Getting actions...');
        const actionsResponse = await makeRequest('GET', '/api/actions');
        const actions = JSON.parse(actionsResponse);
        console.log('Available actions:', actions.actions.map(a => a.description));
        
        // 4. Execute RPS action for Player 1 (Rock)
        console.log('4. Executing RPS action for Player 1 (Rock)...');
        const rpsAction = actions.actions.find(a => a.action_type === 'rock_choice');
        if (rpsAction) {
            const rpsResponse = await makeRequest('POST', '/api/execute-action', {
                action_index: 0,
                action_type: rpsAction.action_type,
                card_id: null,
                card_index: null,
                card_indices: null,
                card_no: null,
                stage_area: null,
                use_baton_touch: null
            });
            console.log('RPS P1 response raw:', rpsResponse);
            try {
                const rpsState = JSON.parse(rpsResponse);
                console.log('RPS P1 executed:', rpsState.phase);
            } catch (e) {
                console.log('RPS P1 JSON parse error:', e.message);
                console.log('RPS P1 response was:', rpsResponse);
            }
        }
        
        // 5. Execute RPS action for Player 2 (Paper)
        console.log('5. Executing RPS action for Player 2 (Paper)...');
        const rpsResponse2 = await makeRequest('POST', '/api/execute-action', {
            action_index: 0,
            action_type: 'paper_choice',
            card_id: null,
            card_index: null,
            card_indices: null,
            card_no: null,
            stage_area: null,
            use_baton_touch: null
        });
        console.log('RPS P2 response raw:', rpsResponse2);
        try {
            const rpsState2 = JSON.parse(rpsResponse2);
            console.log('RPS P2 executed:', rpsState2.phase);
        } catch (e) {
            console.log('RPS P2 JSON parse error:', e.message);
            console.log('RPS P2 response was:', rpsResponse2);
        }
        
        // 6. Get actions again (should be turn choice)
        console.log('6. Getting actions after RPS...');
        const actions2Response = await makeRequest('GET', '/api/actions');
        const actions2 = JSON.parse(actions2Response);
        console.log('Actions after RPS:', actions2.actions.map(a => a.description));
        
        // 7. Execute turn choice (Go first)
        console.log('7. Executing turn choice (Go first)...');
        const turnChoiceAction = actions2.actions.find(a => a.action_type === 'choose_first_attacker');
        if (turnChoiceAction) {
            const turnResponse = await makeRequest('POST', '/api/execute-action', {
                action_index: 0,
                action_type: turnChoiceAction.action_type,
                card_id: null,
                card_index: null,
                card_indices: null,
                card_no: null,
                stage_area: null,
                use_baton_touch: null
            });
            console.log('Turn choice response raw:', turnResponse);
            try {
                const turnState = JSON.parse(turnResponse);
                console.log('Turn choice executed:', turnState.phase);
                console.log('SUCCESS: Phase advanced to:', turnState.phase);
                
                // 8. Get mulligan actions
                console.log('8. Getting mulligan actions...');
                const mulliganResponse = await makeRequest('GET', '/api/actions');
                const mulliganActions = JSON.parse(mulliganResponse);
                console.log('Mulligan actions available:', mulliganActions.actions.map(a => a.description));
                
                return turnState.phase === 'MulliganP1Turn' || turnState.phase === 'MulliganP2Turn';
            } catch (e) {
                console.log('Turn choice JSON parse error:', e.message);
                console.log('Turn choice response was:', turnResponse);
            }
        }
        
        return false;
    } catch (error) {
        console.error('Test failed:', error);
        return false;
    }
}

function makeRequest(method, path, data = null) {
    return new Promise((resolve, reject) => {
        const options = {
            hostname: 'localhost',
            port: 8080,
            path: path,
            method: method,
            headers: {
                'Content-Type': 'application/json',
            }
        };
        
        const req = http.request(options, (res) => {
            let body = '';
            res.on('data', (chunk) => {
                body += chunk;
            });
            res.on('end', () => {
                resolve(body);
            });
        });
        
        req.on('error', reject);
        
        if (data) {
            req.write(JSON.stringify(data));
        }
        
        req.end();
    });
}

testGameFlow().then(success => {
    console.log('\n=== TEST RESULT ===');
    console.log(success ? '✅ PASS: Turn choice advances to mulligan correctly' : '❌ FAIL: Turn choice does not advance to mulligan');
    process.exit(success ? 0 : 1);
}).catch(error => {
    console.error('Test error:', error);
    process.exit(1);
});
