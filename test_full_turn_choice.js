// Test complete turn choice flow
const http = require('http');

async function testFullTurnChoice() {
    console.log('=== COMPLETE TURN CHOICE TEST ===');
    
    try {
        // Step 1: Create room
        console.log('1. Creating room...');
        const roomRes = await makeRequest('POST', '/api/rooms/create', {
            mode: 'pve',
            card_set: 'compiled',
            p0_deck: [],
            p0_energy: [],
            public: true
        });
        
        const roomData = JSON.parse(roomRes);
        if (!roomData.success) {
            console.log('❌ Room creation failed:', roomData);
            return false;
        }
        console.log('✓ Room created');
        
        // Step 2: Get initial state and actions
        console.log('2. Getting initial state...');
        const [stateRes, actionsRes] = await Promise.all([
            makeRequest('GET', '/api/game-state'),
            makeRequest('GET', '/api/actions')
        ]);
        
        const state = JSON.parse(stateRes);
        const actionsData = JSON.parse(actionsRes);
        const actions = actionsData.actions || [];
        
        console.log('Initial phase:', state.phase);
        console.log('Available actions:', actions.length);
        
        if (state.phase !== 'RockPaperScissors') {
            console.log('❌ Wrong initial phase');
            return false;
        }
        
        const rockAction = actions.find(a => a.action_type === 'rock_choice');
        const paperAction = actions.find(a => a.action_type === 'paper_choice');
        
        if (!rockAction || !paperAction) {
            console.log('❌ RPS actions not available');
            return false;
        }
        
        console.log('✓ RPS actions available');
        
        // Step 3: Execute RPS (Rock vs Paper)
        console.log('3. Executing RPS...');
        const rockRes = await makeRequest('POST', '/api/execute-action', {
            action_index: rockAction.index,
            action_type: rockAction.action_type,
            card_id: rockAction.parameters?.card_id,
            card_index: rockAction.parameters?.card_index,
            card_indices: rockAction.parameters?.card_indices,
            card_no: rockAction.parameters?.card_no,
            stage_area: rockAction.parameters?.stage_area,
            use_baton_touch: rockAction.parameters?.use_baton_touch
        });
        
        const rockState = JSON.parse(rockRes);
        console.log('After Rock:', rockState.phase);
        
        const paperRes = await makeRequest('POST', '/api/execute-action', {
            action_index: paperAction.index,
            action_type: paperAction.action_type,
            card_id: paperAction.parameters?.card_id,
            card_index: paperAction.parameters?.card_index,
            card_indices: paperAction.parameters?.card_indices,
            card_no: paperAction.parameters?.card_no,
            stage_area: paperAction.parameters?.stage_area,
            use_baton_touch: paperAction.parameters?.use_baton_touch
        });
        
        const paperState = JSON.parse(paperRes);
        console.log('After Paper:', paperState.phase);
        
        if (paperState.phase !== 'ChooseFirstAttacker') {
            console.log('❌ Phase did not advance to ChooseFirstAttacker');
            return false;
        }
        
        console.log('✓ Phase advanced to ChooseFirstAttacker');
        
        // Step 4: Get turn choice actions
        console.log('4. Getting turn choice actions...');
        const turnActionsRes = await makeRequest('GET', '/api/actions');
        const turnActionsData = JSON.parse(turnActionsRes);
        const turnActions = turnActionsData.actions || [];
        
        const goFirstAction = turnActions.find(a => a.action_type === 'choose_first_attacker');
        
        if (!goFirstAction) {
            console.log('❌ Turn choice actions not available');
            return false;
        }
        
        console.log('✓ Turn choice actions available');
        
        // Step 5: Execute turn choice
        console.log('5. Executing turn choice...');
        const turnRes = await makeRequest('POST', '/api/execute-action', {
            action_index: goFirstAction.index,
            action_type: goFirstAction.action_type,
            card_id: goFirstAction.parameters?.card_id,
            card_index: goFirstAction.parameters?.card_index,
            card_indices: goFirstAction.parameters?.card_indices,
            card_no: goFirstAction.parameters?.card_no,
            stage_area: goFirstAction.parameters?.stage_area,
            use_baton_touch: goFirstAction.parameters?.use_baton_touch
        });
        
        const turnState = JSON.parse(turnRes);
        console.log('After turn choice:', turnState.phase);
        
        if (turnState.phase !== 'MulliganP1Turn' && turnState.phase !== 'MulliganP2Turn') {
            console.log('❌ Phase did not advance to Mulligan');
            return false;
        }
        
        console.log('✓ Phase advanced to Mulligan');
        
        // Step 6: Get mulligan actions
        console.log('6. Getting mulligan actions...');
        const mulliganActionsRes = await makeRequest('GET', '/api/actions');
        const mulliganActionsData = JSON.parse(mulliganActionsRes);
        const mulliganActions = mulliganActionsData.actions || [];
        
        const hasMulliganActions = mulliganActions.some(a => 
            a.action_type === 'skip_mulligan' || 
            a.action_type === 'confirm_mulligan'
        );
        
        if (!hasMulliganActions) {
            console.log('❌ Mulligan actions not available');
            return false;
        }
        
        console.log('✓ Mulligan actions available');
        console.log('\n🎉 COMPLETE TURN CHOICE TEST PASSED');
        console.log('Turn choice phase correctly advances to mulligan');
        return true;
        
    } catch (error) {
        console.error('\n💥 COMPLETE TURN CHOICE TEST FAILED');
        console.error('Error:', error.message);
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

testFullTurnChoice().then(success => {
    console.log(success ? '\n✅ TURN CHOICE FIX VERIFIED' : '\n❌ TURN CHOICE FIX FAILED');
    process.exit(success ? 0 : 1);
});
