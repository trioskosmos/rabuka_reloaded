// Test complete game flow beyond turn choice
const http = require('http');

async function testCompleteGame() {
    console.log('=== COMPLETE GAME FLOW TEST ===');
    
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
        
        // Step 2: Get initial state
        console.log('2. Getting initial state...');
        const [stateRes, actionsRes] = await Promise.all([
            makeRequest('GET', '/api/game-state'),
            makeRequest('GET', '/api/actions')
        ]);
        
        const state = JSON.parse(stateRes);
        const actionsData = JSON.parse(actionsRes);
        let actions = actionsData.actions || [];
        
        console.log('Initial phase:', state.phase);
        console.log('Available actions:', actions.length);
        
        if (state.phase !== 'RockPaperScissors') {
            console.log('❌ Wrong initial phase');
            return false;
        }
        
        // Step 3: Execute RPS (Rock vs Paper)
        console.log('3. Executing RPS...');
        const rockAction = actions.find(a => a.action_type === 'rock_choice');
        const paperAction = actions.find(a => a.action_type === 'paper_choice');
        
        if (!rockAction || !paperAction) {
            console.log('❌ RPS actions not available');
            return false;
        }
        
        // Execute Rock
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
        
        // Execute Paper
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
        
        // Step 4: Execute turn choice
        console.log('4. Executing turn choice...');
        const [turnStateRes, turnActionsRes] = await Promise.all([
            makeRequest('GET', '/api/game-state'),
            makeRequest('GET', '/api/actions')
        ]);
        
        const turnState = JSON.parse(turnStateRes);
        const turnActionsData = JSON.parse(turnActionsRes);
        const turnActions = turnActionsData.actions || [];
        
        const goFirstAction = turnActions.find(a => a.action_type === 'choose_first_attacker');
        
        if (!goFirstAction) {
            console.log('❌ Turn choice actions not available');
            return false;
        }
        
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
        
        const turnChoiceState = JSON.parse(turnRes);
        console.log('After turn choice:', turnChoiceState.phase);
        
        if (turnChoiceState.phase !== 'MulliganP1Turn' && turnChoiceState.phase !== 'MulliganP2Turn') {
            console.log('❌ Phase did not advance to Mulligan');
            return false;
        }
        
        console.log('✓ Phase advanced to Mulligan');
        
        // Step 5: Test mulligan phase
        console.log('5. Testing mulligan phase...');
        const [mulliganStateRes, mulliganActionsRes] = await Promise.all([
            makeRequest('GET', '/api/game-state'),
            makeRequest('GET', '/api/actions')
        ]);
        
        const mulliganState = JSON.parse(mulliganStateRes);
        const mulliganActionsData = JSON.parse(mulliganActionsRes);
        const mulliganActions = mulliganActionsData.actions || [];
        
        console.log('Mulligan phase:', mulliganState.phase);
        console.log('Mulligan actions:', mulliganActions.length);
        
        const hasSkipMulligan = mulliganActions.some(a => a.action_type === 'skip_mulligan');
        const hasConfirmMulligan = mulliganActions.some(a => a.action_type === 'confirm_mulligan');
        
        if (!hasSkipMulligan && !hasConfirmMulligan) {
            console.log('❌ Mulligan actions not available');
            return false;
        }
        
        console.log('✓ Mulligan actions available');
        
        // Step 6: Execute mulligan (skip) for P1
        console.log('6. Executing mulligan (skip) for P1...');
        const skipAction = mulliganActions.find(a => a.action_type === 'skip_mulligan');
        
        const skipRes = await makeRequest('POST', '/api/execute-action', {
            action_index: skipAction.index,
            action_type: skipAction.action_type,
            card_id: skipAction.parameters?.card_id,
            card_index: skipAction.parameters?.card_index,
            card_indices: skipAction.parameters?.card_indices,
            card_no: skipAction.parameters?.card_no,
            stage_area: skipAction.parameters?.stage_area,
            use_baton_touch: skipAction.parameters?.use_baton_touch
        });
        
        const skipState = JSON.parse(skipRes);
        console.log('After P1 mulligan skip:', skipState.phase);
        
        if (skipState.phase !== 'MulliganP2Turn') {
            console.log('❌ Phase did not advance to MulliganP2Turn');
            return false;
        }
        
        console.log('✓ Phase advanced to MulliganP2Turn');
        
        // Step 7: Execute mulligan (skip) for P2
        console.log('7. Executing mulligan (skip) for P2...');
        const [p2StateRes, p2ActionsRes] = await Promise.all([
            makeRequest('GET', '/api/game-state'),
            makeRequest('GET', '/api/actions')
        ]);
        
        const p2State = JSON.parse(p2StateRes);
        const p2ActionsData = JSON.parse(p2ActionsRes);
        const p2Actions = p2ActionsData.actions || [];
        
        const p2SkipAction = p2Actions.find(a => a.action_type === 'skip_mulligan');
        
        const p2SkipRes = await makeRequest('POST', '/api/execute-action', {
            action_index: p2SkipAction.index,
            action_type: p2SkipAction.action_type,
            card_id: p2SkipAction.parameters?.card_id,
            card_index: p2SkipAction.parameters?.card_index,
            card_indices: p2SkipAction.parameters?.card_indices,
            card_no: p2SkipAction.parameters?.card_no,
            stage_area: p2SkipAction.parameters?.stage_area,
            use_baton_touch: p2SkipAction.parameters?.use_baton_touch
        });
        
        const p2SkipState = JSON.parse(p2SkipRes);
        console.log('After P2 mulligan skip:', p2SkipState.phase);
        
        if (p2SkipState.phase !== 'Main' && p2SkipState.phase !== 'Active') {
            console.log('❌ Phase did not advance to Main or Active');
            return false;
        }
        
        console.log('✓ Phase advanced to Main/Active (auto-advance working)');
        
        // Step 8: Check main game actions (Active should auto-advance to Main)
        console.log('8. Checking main game actions (Active should auto-advance)...');
        const [mainStateRes, mainActionsRes] = await Promise.all([
            makeRequest('GET', '/api/game-state'),
            makeRequest('GET', '/api/actions')
        ]);
        
        const mainState = JSON.parse(mainStateRes);
        const mainActionsData = JSON.parse(mainActionsRes);
        const mainActions = mainActionsData.actions || [];
        
        console.log('Main phase:', mainState.phase);
        console.log('Main actions:', mainActions.length);
        
        const hasPlayMemberAction = mainActions.some(a => a.action_type === 'play_member_to_stage');
        const hasPassAction = mainActions.some(a => a.action_type === 'pass');
        
        if (!hasPlayMemberAction && !hasPassAction) {
            console.log('❌ Main game actions not available');
            return false;
        }
        
        console.log('✅ Main game actions available');
        console.log('\n🎉 COMPLETE GAME FLOW TEST PASSED');
        console.log('✅ Turn choice works correctly');
        console.log('✅ Mulligan phase works correctly');
        console.log('✅ Main game phase works correctly');
        console.log('✅ Complete game flow verified');
        
        return true;
        
    } catch (error) {
        console.error('\n💥 COMPLETE GAME FLOW TEST FAILED');
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

testCompleteGame().then(success => {
    console.log(success ? '\n✅ COMPLETE GAME FLOW VERIFIED' : '\n❌ COMPLETE GAME FLOW FAILED');
    process.exit(success ? 0 : 1);
});
