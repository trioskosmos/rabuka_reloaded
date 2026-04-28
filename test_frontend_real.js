// Test actual frontend game flow through proxy
const http = require('http');

async function testFrontendReal() {
    console.log('=== FRONTEND REAL GAME TEST ===');
    
    try {
        // Test 1: Create game through frontend proxy (port 3000)
        console.log('1. Creating game through frontend proxy...');
        const roomRes = await makeRequest('POST', '/api/rooms/create', {
            mode: 'pve',
            card_set: 'compiled',
            p0_deck: [],
            p0_energy: [],
            public: true
        }, 3000);
        
        const roomData = JSON.parse(roomRes);
        if (!roomData.success) {
            console.log('❌ Room creation failed:', roomData);
            return false;
        }
        console.log('✓ Room created through frontend');
        
        // Test 2: Get state through frontend proxy
        console.log('2. Getting game state through frontend...');
        const stateRes = await makeRequest('GET', '/api/game-state', null, 3000);
        const state = JSON.parse(stateRes);
        console.log('Initial phase:', state.phase);
        
        if (state.phase !== 'RockPaperScissors') {
            console.log('❌ Wrong initial phase, expected RockPaperScissors');
            return false;
        }
        console.log('✓ Correct initial phase');
        
        // Test 3: Get actions through frontend proxy
        console.log('3. Getting actions through frontend...');
        const actionsRes = await makeRequest('GET', '/api/actions', null, 3000);
        const actionsData = JSON.parse(actionsRes);
        const rpsActions = actionsData.actions.filter(a => 
            a.action_type === 'rock_choice' || 
            a.action_type === 'paper_choice' || 
            a.action_type === 'scissors_choice'
        );
        console.log('RPS actions available:', rpsActions.length);
        
        if (rpsActions.length !== 3) {
            console.log('❌ Wrong number of RPS actions:', rpsActions.length);
            return false;
        }
        console.log('✓ RPS actions available');
        
        // Test 4: Execute RPS actions
        console.log('4. Executing RPS actions...');
        const rockAction = rpsActions.find(a => a.action_type === 'rock_choice');
        const rockRes = await makeRequest('POST', '/api/execute-action', {
            action_index: rockAction.index,
            action_type: rockAction.action_type,
            card_id: rockAction.parameters?.card_id,
            card_index: rockAction.parameters?.card_index,
            card_indices: rockAction.parameters?.card_indices,
            card_no: rockAction.parameters?.card_no,
            stage_area: rockAction.parameters?.stage_area,
            use_baton_touch: rockAction.parameters?.use_baton_touch
        }, 3000);
        
        const rockState = JSON.parse(rockRes);
        console.log('After Rock:', rockState.phase);
        
        const paperAction = actionsData.actions.find(a => a.action_type === 'paper_choice');
        const paperRes = await makeRequest('POST', '/api/execute-action', {
            action_index: paperAction.index,
            action_type: paperAction.action_type,
            card_id: paperAction.parameters?.card_id,
            card_index: paperAction.parameters?.card_index,
            card_indices: paperAction.parameters?.card_indices,
            card_no: paperAction.parameters?.card_no,
            stage_area: paperAction.parameters?.stage_area,
            use_baton_touch: paperAction.parameters?.use_baton_touch
        }, 3000);
        
        const paperState = JSON.parse(paperRes);
        console.log('After Paper:', paperState.phase);
        
        if (paperState.phase !== 'ChooseFirstAttacker') {
            console.log('❌ Wrong phase after RPS, expected ChooseFirstAttacker');
            return false;
        }
        console.log('✓ Phase advanced to ChooseFirstAttacker');
        
        // Test 5: Get turn choice actions
        console.log('5. Getting turn choice actions...');
        const turnActionsRes = await makeRequest('GET', '/api/actions', null, 3000);
        const turnActionsData = JSON.parse(turnActionsRes);
        const turnActions = turnActionsData.actions.filter(a => 
            a.action_type === 'choose_first_attacker' || 
            a.action_type === 'choose_second_attacker'
        );
        console.log('Turn choice actions available:', turnActions.length);
        
        if (turnActions.length !== 2) {
            console.log('❌ Wrong number of turn choice actions:', turnActions.length);
            return false;
        }
        console.log('✓ Turn choice actions available');
        
        // Test 6: Execute turn choice
        console.log('6. Executing turn choice...');
        const goFirstAction = turnActions.find(a => a.action_type === 'choose_first_attacker');
        const turnRes = await makeRequest('POST', '/api/execute-action', {
            action_index: goFirstAction.index,
            action_type: goFirstAction.action_type,
            card_id: goFirstAction.parameters?.card_id,
            card_index: goFirstAction.parameters?.card_index,
            card_indices: goFirstAction.parameters?.card_indices,
            card_no: goFirstAction.parameters?.card_no,
            stage_area: goFirstAction.parameters?.stage_area,
            use_baton_touch: goFirstAction.parameters?.use_baton_touch
        }, 3000);
        
        const turnState = JSON.parse(turnRes);
        console.log('After turn choice:', turnState.phase);
        
        if (turnState.phase !== 'MulliganP1Turn' && turnState.phase !== 'MulliganP2Turn') {
            console.log('❌ Wrong phase after turn choice, expected Mulligan phase');
            console.log('Got:', turnState.phase);
            return false;
        }
        console.log('✓ Phase advanced to Mulligan');
        
        // Test 7: Get mulligan actions
        console.log('7. Getting mulligan actions...');
        const mulliganRes = await makeRequest('GET', '/api/actions', null, 3000);
        const mulliganData = JSON.parse(mulliganRes);
        const mulliganActions = mulliganData.actions.filter(a => 
            a.action_type === 'skip_mulligan' || 
            a.action_type === 'confirm_mulligan'
        );
        console.log('Mulligan actions available:', mulliganActions.length);
        
        if (mulliganActions.length < 1) {
            console.log('❌ No mulligan actions available');
            return false;
        }
        console.log('✓ Mulligan actions available');
        
        console.log('\n🎉 FRONTEND REAL GAME TEST PASSED');
        console.log('Turn choice phase correctly advances to mulligan through frontend proxy');
        return true;
        
    } catch (error) {
        console.error('\n💥 FRONTEND REAL GAME TEST FAILED');
        console.error('Error:', error.message);
        return false;
    }
}

function makeRequest(method, path, data = null, port = 8080) {
    return new Promise((resolve, reject) => {
        const options = {
            hostname: 'localhost',
            port: port,
            path: path,
            method: method,
            headers: { 'Content-Type': 'application/json' }
        };
        
        const req = http.request(options, (res) => {
            let body = '';
            res.on('data', chunk => body += chunk);
            res.on('end', () => resolve(body));
        });
        
        req.on('error', reject);
        if (data) req.write(JSON.stringify(data));
        req.end();
    });
}

testFrontendReal().then(success => {
    console.log(success ? '\n✅ FRONTEND VERIFICATION COMPLETE' : '\n❌ FRONTEND VERIFICATION FAILED');
    process.exit(success ? 0 : 1);
});
