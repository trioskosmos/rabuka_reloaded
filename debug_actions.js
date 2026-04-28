// Debug action execution to see why RPS actions don't work
const http = require('http');

async function debugActions() {
    console.log('=== DEBUG ACTION EXECUTION ===');
    
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
        
        // Step 2: Get state and actions
        console.log('2. Getting state and actions...');
        const [stateRes, actionsRes] = await Promise.all([
            makeRequest('GET', '/api/game-state'),
            makeRequest('GET', '/api/actions')
        ]);
        
        const state = JSON.parse(stateRes);
        const actionsData = JSON.parse(actionsRes);
        const actions = actionsData.actions || [];
        
        console.log('State:', state.phase);
        console.log('Actions:', actions.length);
        actions.forEach((a, i) => {
            console.log(`  Action ${i}: ${a.action_type} - ${a.description}`);
            console.log(`    Index: ${a.index}, Parameters:`, a.parameters);
        });
        
        // Step 3: Try to execute Rock action
        console.log('3. Trying to execute Rock action...');
        const rockAction = actions.find(a => a.action_type === 'rock_choice');
        if (!rockAction) {
            console.log('❌ No rock action found');
            return false;
        }
        
        console.log('Rock action found:', rockAction);
        
        const executeRes = await makeRequest('POST', '/api/execute-action', {
            action_index: rockAction.index,
            action_type: rockAction.action_type,
            card_id: rockAction.parameters?.card_id,
            card_index: rockAction.parameters?.card_index,
            card_indices: rockAction.parameters?.card_indices,
            card_no: rockAction.parameters?.card_no,
            stage_area: rockAction.parameters?.stage_area,
            use_baton_touch: rockAction.parameters?.use_baton_touch
        });
        
        console.log('Execute response:', executeRes);
        
        if (executeRes.startsWith('{')) {
            const result = JSON.parse(executeRes);
            console.log('✅ Action executed successfully');
            console.log('New phase:', result.phase);
            console.log('New actions:', result.legal_actions?.length || 0);
            return true;
        } else {
            console.log('❌ Action execution failed');
            console.log('Response:', executeRes);
            return false;
        }
        
    } catch (error) {
        console.error('❌ Debug failed:', error.message);
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

debugActions().then(success => {
    console.log(success ? '\n✅ ACTION DEBUG COMPLETE' : '\n❌ ACTION DEBUG FAILED');
    process.exit(success ? 0 : 1);
});
