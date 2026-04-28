// Test backend directly without card database issues
const http = require('http');

async function testBackendDirect() {
    console.log('=== BACKEND DIRECT TEST ===');
    
    try {
        // Test if backend is running
        console.log('1. Testing backend connection...');
        const statusRes = await makeRequest('GET', '/api/status');
        const status = JSON.parse(statusRes);
        console.log('Backend status:', status);
        
        // Create room with minimal data
        console.log('2. Creating room...');
        const roomRes = await makeRequest('POST', '/api/rooms/create', {
            mode: 'pve',
            card_set: 'compiled',
            p0_deck: [],
            p0_energy: [],
            public: true
        });
        
        console.log('Room response:', roomRes);
        const roomData = JSON.parse(roomRes);
        
        if (!roomData.success) {
            console.log('❌ Room creation failed');
            return false;
        }
        console.log('✓ Room created');
        
        // Get state immediately after room creation
        console.log('3. Getting state...');
        const stateRes = await makeRequest('GET', '/api/game-state');
        const state = JSON.parse(stateRes);
        console.log('Initial phase:', state.phase);
        
        // Test if we can get actions
        console.log('4. Getting actions...');
        const actionsRes = await makeRequest('GET', '/api/actions');
        const actionsData = JSON.parse(actionsRes);
        console.log('Actions available:', actionsData.actions.length);
        
        console.log('✅ BACKEND DIRECT TEST PASSED');
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

testBackendDirect().then(success => {
    console.log(success ? '\n🎉 BACKEND WORKS' : '\n💥 BACKEND FAILED');
    process.exit(success ? 0 : 1);
});
