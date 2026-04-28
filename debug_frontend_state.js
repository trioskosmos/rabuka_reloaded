// Debug frontend state to see what's happening
const http = require('http');

async function debugFrontendState() {
    console.log('=== FRONTEND STATE DEBUG ===');
    
    try {
        // Test 1: Check if backend is accessible
        console.log('1. Testing backend connection...');
        try {
            const statusRes = await makeRequest('GET', '/api/status', null, 8080);
            const status = JSON.parse(statusRes);
            console.log('✓ Backend status:', status.status);
        } catch (error) {
            console.log('❌ Backend not accessible:', error.message);
            return false;
        }
        
        // Test 2: Check what state frontend proxy returns
        console.log('2. Checking frontend proxy state...');
        const proxyStateRes = await makeRequest('GET', '/api/game-state', null, 3000);
        const proxyState = JSON.parse(proxyStateRes);
        console.log('Frontend proxy phase:', proxyState.phase);
        
        // Test 3: Check what backend directly returns
        console.log('3. Checking backend direct state...');
        const backendStateRes = await makeRequest('GET', '/api/game-state', null, 8080);
        const backendState = JSON.parse(backendStateRes);
        console.log('Backend direct phase:', backendState.phase);
        
        // Test 4: Create new room and compare
        console.log('4. Creating new room through frontend...');
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
        
        // Test 5: Check state after room creation
        console.log('5. Checking state after room creation...');
        const newStateRes = await makeRequest('GET', '/api/game-state', null, 3000);
        const newState = JSON.parse(newStateRes);
        console.log('New state phase:', newState.phase);
        
        // Test 6: Compare states
        console.log('6. Comparing states...');
        console.log('Proxy vs Backend difference:', proxyState.phase !== backendState.phase);
        console.log('New vs Backend difference:', newState.phase !== backendState.phase);
        
        if (newState.phase === 'RockPaperScissors') {
            console.log('✅ Frontend correctly returns RockPaperScissors');
            return true;
        } else {
            console.log('❌ Frontend returns wrong phase:', newState.phase);
            console.log('Expected: RockPaperScissors');
            console.log('Actual:', newState.phase);
            return false;
        }
        
    } catch (error) {
        console.error('❌ Debug failed:', error.message);
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

debugFrontendState().then(success => {
    console.log(success ? '\n✅ FRONTEND STATE DEBUG PASSED' : '\n❌ FRONTEND STATE DEBUG FAILED');
    process.exit(success ? 0 : 1);
});
