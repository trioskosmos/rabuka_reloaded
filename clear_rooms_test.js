// Clear rooms and test fresh room creation
const http = require('http');

async function clearRoomsAndTest() {
    console.log('=== CLEAR ROOMS AND TEST ===');
    
    try {
        // Step 1: Clear any existing rooms (if possible)
        console.log('1. Attempting to clear rooms...');
        try {
            const clearRes = await makeRequest('POST', '/api/debug/clear-rooms', {}, 8080);
            console.log('Clear response:', clearRes);
        } catch (error) {
            console.log('Clear endpoint not available:', error.message);
        }
        
        // Step 2: Create fresh room
        console.log('2. Creating fresh room...');
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
        console.log('✓ Room created:', roomData.room_id);
        
        // Step 3: Check state
        console.log('3. Checking state...');
        const stateRes = await makeRequest('GET', '/api/game-state', null, 3000);
        const state = JSON.parse(stateRes);
        console.log('Initial phase:', state.phase);
        
        if (state.phase === 'RockPaperScissors') {
            console.log('✅ SUCCESS: Fresh room starts with RockPaperScissors');
            return true;
        } else {
            console.log('❌ FAILED: Room starts with:', state.phase);
            console.log('Expected: RockPaperScissors');
            return false;
        }
        
    } catch (error) {
        console.error('❌ Test failed:', error.message);
        return false;
    }
}

function makeRequest(method, path, data = null, port = 3000) {
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

clearRoomsAndTest().then(success => {
    console.log(success ? '\n🎉 FRESH ROOM TEST PASSED' : '\n💥 FRESH ROOM TEST FAILED');
    process.exit(success ? 0 : 1);
});
