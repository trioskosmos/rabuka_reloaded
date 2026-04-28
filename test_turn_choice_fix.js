const http = require('http');

// Test the complete game flow through the web server
async function testTurnChoiceFix() {
    console.log('=== TESTING TURN CHOICE FIX ===');
    
    try {
        // 1. Create a room (frontend uses this)
        console.log('1. Creating room...');
        const roomResponse = await makeRequest('POST', '/api/rooms/create', {
            mode: 'pve',
            card_set: 'compiled',
            p0_deck: [],
            p0_energy: [],
            public: true
        });
        const roomData = JSON.parse(roomResponse);
        console.log('Room response:', JSON.stringify(roomData, null, 2));
        if (!roomData.success) {
            console.log('❌ Room creation failed');
            return false;
        }
        console.log('✓ Room created');
        
        // 2. Check initial state
        console.log('2. Checking initial game state...');
        const initialState = await makeRequest('GET', '/api/game-state');
        const gameState = JSON.parse(initialState);
        console.log('Initial phase:', gameState.phase);
        
        // Check if legal_actions is present
        console.log('Legal actions in state:', gameState.legal_actions?.length || 0);
        
        if (gameState.phase !== 'RockPaperScissors') {
            console.log('❌ Wrong initial phase, expected RockPaperScissors');
            return false;
        }
        console.log('✓ Correct initial phase');
        
        // 3. Get RPS actions
        console.log('3. Getting RPS actions...');
        const actionsResponse = await makeRequest('GET', '/api/actions');
        const actionsData = JSON.parse(actionsResponse);
        const rpsActions = actionsData.actions.filter(a => 
            a.action_type === 'rock_choice' || 
            a.action_type === 'paper_choice' || 
            a.action_type === 'scissors_choice'
        );
        console.log('Actions received:', actionsData.actions.length);
        console.log('RPS actions:', rpsActions.length);
        
        if (rpsActions.length !== 3) {
            console.log('❌ Wrong number of RPS actions:', rpsActions.length);
            return false;
        }
        console.log('✓ RPS actions available');
        
        // 4. Execute RPS for player 1 (exactly like frontend does)
        console.log('4. Executing RPS for player 1 (Rock)...');
        const rockAction = rpsActions.find(a => a.action_type === 'rock_choice');
        const rps1Body = {
            action_index: rockAction.index || 0,
            action_type: rockAction.action_type,
            card_id: rockAction.parameters?.card_id,
            card_index: rockAction.parameters?.card_index,
            card_indices: rockAction.parameters?.card_indices,
            card_no: rockAction.parameters?.card_no,
            stage_area: rockAction.parameters?.stage_area,
            use_baton_touch: rockAction.parameters?.use_baton_touch
        };
        console.log('Request body:', JSON.stringify(rps1Body, null, 2));
        const rps1Response = await makeRequest('POST', '/api/execute-action', rps1Body);
        const rps1State = JSON.parse(rps1Response);
        console.log('After P1 RPS:', rps1State.phase);
        console.log('Legal actions after P1 RPS:', rps1State.legal_actions?.length || 0);
        
        // 5. Execute RPS for player 2 (exactly like frontend does)
        console.log('5. Executing RPS for player 2 (Paper)...');
        const paperAction = rps1State.legal_actions?.find(a => a.action_type === 'paper_choice') || rpsActions.find(a => a.action_type === 'paper_choice');
        const rps2Body = {
            action_index: paperAction.index || 0,
            action_type: paperAction.action_type,
            card_id: paperAction.parameters?.card_id,
            card_index: paperAction.parameters?.card_index,
            card_indices: paperAction.parameters?.card_indices,
            card_no: paperAction.parameters?.card_no,
            stage_area: paperAction.parameters?.stage_area,
            use_baton_touch: paperAction.parameters?.use_baton_touch
        };
        console.log('Request body:', JSON.stringify(rps2Body, null, 2));
        const rps2Response = await makeRequest('POST', '/api/execute-action', rps2Body);
        const rps2State = JSON.parse(rps2Response);
        console.log('After P2 RPS:', rps2State.phase);
        console.log('Legal actions after P2 RPS:', rps2State.legal_actions?.length || 0);
        
        if (rps2State.phase !== 'ChooseFirstAttacker') {
            console.log('❌ Wrong phase after RPS, expected ChooseFirstAttacker');
            console.log('Got:', rps2State.phase);
            return false;
        }
        console.log('✓ Phase advanced to ChooseFirstAttacker');
        
        // 6. Get turn choice actions
        console.log('6. Getting turn choice actions...');
        const turnActionsResponse = await makeRequest('GET', '/api/actions');
        const turnActionsData = JSON.parse(turnActionsResponse);
        const turnActions = turnActionsData.actions.filter(a => 
            a.action_type === 'choose_first_attacker' || 
            a.action_type === 'choose_second_attacker'
        );
        console.log('Turn choice actions:', turnActions.length);
        
        if (turnActions.length !== 2) {
            console.log('❌ Wrong number of turn choice actions:', turnActions.length);
            console.log('All actions:', turnActionsData.actions.map(a => a.action_type));
            return false;
        }
        console.log('✓ Turn choice actions available');
        
        // 7. Execute turn choice (Go first) exactly like frontend does
        console.log('7. Executing turn choice (Go first)...');
        const goFirstAction = turnActions.find(a => a.action_type === 'choose_first_attacker');
        const turnBody = {
            action_index: goFirstAction.index || 0,
            action_type: goFirstAction.action_type,
            card_id: goFirstAction.parameters?.card_id,
            card_index: goFirstAction.parameters?.card_index,
            card_indices: goFirstAction.parameters?.card_indices,
            card_no: goFirstAction.parameters?.card_no,
            stage_area: goFirstAction.parameters?.stage_area,
            use_baton_touch: goFirstAction.parameters?.use_baton_touch
        };
        console.log('Request body:', JSON.stringify(turnBody, null, 2));
        const turnResponse = await makeRequest('POST', '/api/execute-action', turnBody);
        const turnState = JSON.parse(turnResponse);
        console.log('After turn choice:', turnState.phase);
        console.log('Legal actions after turn choice:', turnState.legal_actions?.length || 0);
        
        if (turnState.phase !== 'MulliganP1Turn' && turnState.phase !== 'MulliganP2Turn') {
            console.log('❌ Wrong phase after turn choice, expected Mulligan phase');
            console.log('Got:', turnState.phase);
            console.log('Full state:', JSON.stringify(turnState, null, 2));
            return false;
        }
        console.log('✓ Phase advanced to Mulligan');
        
        // 8. Get mulligan actions
        console.log('8. Getting mulligan actions...');
        const mulliganResponse = await makeRequest('GET', '/api/actions');
        const mulliganData = JSON.parse(mulliganResponse);
        const mulliganActions = mulliganData.actions.filter(a => 
            a.action_type === 'confirm_mulligan' || 
            a.action_type === 'skip_mulligan'
        );
        console.log('Mulligan actions:', mulliganActions.length);
        
        if (mulliganActions.length < 1) {
            console.log('❌ No mulligan actions available');
            console.log('All actions:', mulliganData.actions.map(a => a.action_type));
            return false;
        }
        console.log('✓ Mulligan actions available');
        
        console.log('\n=== TEST PASSED ===');
        console.log('Turn choice phase correctly advances to mulligan');
        return true;
        
    } catch (error) {
        console.error('\n=== TEST FAILED ===');
        console.error('Error:', error.message);
        return false;
    }
}

function makeRequest(method, path, data = null) {
    return new Promise((resolve, reject) => {
        const options = {
            hostname: 'localhost',
            port: 3000,
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

testTurnChoiceFix().then(success => {
    process.exit(success ? 0 : 1);
}).catch(error => {
    console.error('Test error:', error);
    process.exit(1);
});
