// Debug script to check what the frontend is actually doing
const http = require('http');

// Simulate exactly what the frontend should be doing
async function debugFrontend() {
    console.log('=== DEBUGGING FRONTEND BEHAVIOR ===');
    
    try {
        // 1. Initialize game through the frontend's room system
        console.log('1. Creating room through proxy...');
        const roomResponse = await makeRequest('POST', '/api/rooms/create', {
            mode: 'pve',
            card_set: 'compiled',
            p0_deck: [],
            p0_energy: [],
            public: true
        });
        const roomData = JSON.parse(roomResponse);
        console.log('Room created:', roomData.success ? 'Success' : 'Failed');
        
        // 2. Check what the frontend gets on initial load
        console.log('2. Checking initial game state...');
        const initialState = await makeRequest('GET', '/api/game-state');
        console.log('Initial state:', JSON.parse(initialState).phase);
        
        // 3. Check what actions the frontend gets
        console.log('3. Checking initial actions...');
        const initialActions = await makeRequest('GET', '/api/actions');
        const actionsData = JSON.parse(initialActions);
        console.log('Initial actions:', actionsData.actions.map(a => `${a.description} (${a.action_type})`));
        
        // 3. Simulate what happens when frontend sends RPS actions
        console.log('3. Simulating RPS choices like frontend would...');
        
        // First RPS choice
        const rps1Response = await makeRequest('POST', '/api/execute-action', {
            action_index: 0,
            action_type: 'rock_choice',
            card_id: null,
            card_index: null,
            card_indices: null,
            card_no: null,
            stage_area: null,
            use_baton_touch: null
        });
        console.log('After first RPS choice:', JSON.parse(rps1Response).phase);
        
        // Second RPS choice
        const rps2Response = await makeRequest('POST', '/api/execute-action', {
            action_index: 0,
            action_type: 'paper_choice',
            card_id: null,
            card_index: null,
            card_indices: null,
            card_no: null,
            stage_area: null,
            use_baton_touch: null
        });
        console.log('After second RPS choice:', JSON.parse(rps2Response).phase);
        
        // 4. Check what actions are available after RPS
        console.log('4. Checking actions after RPS...');
        const actionsAfterRPS = await makeRequest('GET', '/api/actions');
        const actionsAfterRPSData = JSON.parse(actionsAfterRPS);
        console.log('Actions after RPS:', actionsAfterRPSData.actions.map(a => `${a.description} (${a.action_type})`));
        
        // 5. Simulate turn choice like frontend would
        console.log('5. Simulating turn choice like frontend...');
        const turnChoiceAction = actionsAfterRPSData.actions.find(a => a.action_type === 'choose_first_attacker');
        if (turnChoiceAction) {
            console.log('Found turn choice action:', turnChoiceAction);
            
            const turnResponse = await makeRequest('POST', '/api/execute-action', {
                action_index: 0,
                action_type: turnChoiceAction.action_type,
                card_id: turnChoiceAction.parameters?.card_id,
                card_index: turnChoiceAction.parameters?.card_index,
                card_indices: turnChoiceAction.parameters?.card_indices,
                card_no: turnChoiceAction.parameters?.card_no,
                stage_area: turnChoiceAction.parameters?.stage_area,
                use_baton_touch: turnChoiceAction.parameters?.use_baton_touch
            });
            
            const turnState = JSON.parse(turnResponse);
            console.log('After turn choice:', turnState.phase);
            console.log('Legal actions after turn choice:', turnState.legal_actions?.map(a => a.description) || 'No actions');
            
            return turnState.phase;
        } else {
            console.log('❌ ERROR: No turn choice action found!');
            console.log('Available actions:', actionsAfterRPSData.actions);
            return null;
        }
        
    } catch (error) {
        console.error('Debug failed:', error);
        return null;
    }
}

function makeRequest(method, path, data = null) {
    return new Promise((resolve, reject) => {
        const options = {
            hostname: 'localhost',
            port: 3000,  // Use frontend proxy instead of backend directly
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

debugFrontend().then(finalPhase => {
    console.log('\n=== FRONTEND DEBUG RESULT ===');
    if (finalPhase && (finalPhase === 'MulliganP1Turn' || finalPhase === 'MulliganP2Turn')) {
        console.log('✅ Frontend API calls work correctly');
        console.log('❌ The issue is in the frontend JavaScript, not the backend');
    } else {
        console.log('❌ Backend API calls are broken');
        console.log('Final phase:', finalPhase);
    }
}).catch(error => {
    console.error('Debug error:', error);
});
