// Test room persistence issue directly
const http = require('http');

async function testRoomPersistence() {
    console.log('=== ROOM PERSISTENCE TEST ===');
    
    try {
        // Test 1: Check if backend is running
        console.log('1. Testing backend connection...');
        try {
            const statusRes = await makeRequest('GET', '/api/status');
            const status = JSON.parse(statusRes);
            console.log('✓ Backend is running:', status.status);
        } catch (error) {
            console.log('❌ Backend not running:', error.message);
            return false;
        }
        
        // Test 2: Create room
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
        console.log('✓ Room created with ID:', roomData.room_id);
        
        // Test 3: Get state immediately
        console.log('3. Getting game state...');
        const stateRes = await makeRequest('GET', '/api/game-state');
        const state = JSON.parse(stateRes);
        console.log('Initial phase:', state.phase);
        
        // Test 4: Get actions
        console.log('4. Getting actions...');
        const actionsRes = await makeRequest('GET', '/api/actions');
        const actionsData = JSON.parse(actionsRes);
        console.log('Actions available:', actionsData.actions.length);
        
        // Test 5: Check if room persists
        console.log('5. Testing room persistence...');
        const stateRes2 = await makeRequest('GET', '/api/game-state');
        const state2 = JSON.parse(stateRes2);
        
        if (state2.phase !== state.phase) {
            console.log('❌ Room state changed unexpectedly');
            return false;
        }
        
        console.log('✅ ROOM PERSISTENCE TEST PASSED');
        console.log('Backend is working and room persists');
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

testRoomPersistence().then(success => {
    console.log(success ? '\n🎉 ROOM PERSISTENCE WORKS' : '\n💥 ROOM PERSISTENCE FAILED');
    process.exit(success ? 0 : 1);
});
