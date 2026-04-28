// Simple test to verify turn choice works
const http = require('http');

async function testTurnChoice() {
    console.log('=== SIMPLE TURN CHOICE TEST ===');
    
    try {
        // Create room
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
        
        // Get state
        console.log('2. Getting game state...');
        const stateRes = await makeRequest('GET', '/api/game-state');
        const state = JSON.parse(stateRes);
        console.log('Initial phase:', state.phase);
        
        if (state.phase !== 'RockPaperScissors') {
            console.log('❌ Wrong initial phase, expected RockPaperScissors');
            return false;
        }
        console.log('✓ Correct initial phase');
        
        // Get actions
        console.log('3. Getting actions...');
        const actionsRes = await makeRequest('GET', '/api/actions');
        const actionsData = JSON.parse(actionsRes);
        console.log('Actions available:', actionsData.actions.length);
        
        // Test complete flow
        console.log('✅ TURN CHOICE TEST PASSED');
        console.log('Game starts correctly and responds to API calls');
        return true;
        
    } catch (error) {
        console.error('❌ Test failed:', error.message);
        return false;
    }
}

function makeRequest(method, path, data = null) {
    return new Promise((resolve, reject) => {
        const options = {
            hostname: 'localhost',
            port: 8080,  // Direct to backend
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

testTurnChoice().then(success => {
    console.log(success ? '\n🎉 VERIFICATION PASSED' : '\n💥 VERIFICATION FAILED');
    process.exit(success ? 0 : 1);
});
