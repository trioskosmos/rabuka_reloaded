// Debug room creation issue
const http = require('http');

async function debugRoomIssue() {
    console.log('=== DEBUG ROOM ISSUE ===');
    
    try {
        // Step 1: Check current state without creating room
        console.log('1. Checking current state...');
        try {
            const currentStateRes = await makeRequest('GET', '/api/game-state');
            const currentState = JSON.parse(currentStateRes);
            console.log('Current state before room creation:', currentState.phase);
        } catch (error) {
            console.log('❌ Failed to get current state:', error.message);
        }
        
        // Step 2: Create new room
        console.log('2. Creating new room...');
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
        console.log('✓ Room created with ID:', roomData.room_id);
        
        // Step 3: Check state after room creation
        console.log('3. Checking state after room creation...');
        const newStateRes = await makeRequest('GET', '/api/game-state');
        const newState = JSON.parse(newStateRes);
        console.log('State after room creation:', newState.phase);
        
        // Step 4: Check if we can clear rooms
        console.log('4. Trying to clear rooms...');
        try {
            const clearRes = await makeRequest('POST', '/api/debug/clear-rooms', {});
            console.log('Clear response:', clearRes);
        } catch (error) {
            console.log('❌ Clear endpoint not available:', error.message);
        }
        
        // Step 5: Create another room to test
        console.log('5. Creating another room...');
        const room2Res = await makeRequest('POST', '/api/rooms/create', {
            mode: 'pve',
            card_set: 'compiled',
            p0_deck: [],
            p0_energy: [],
            public: true
        });
        
        const room2Data = JSON.parse(room2Res);
        if (!room2Data.success) {
            console.log('❌ Second room creation failed:', room2Data);
            return false;
        }
        console.log('✓ Second room created with ID:', room2Data.room_id);
        
        // Step 6: Check state after second room creation
        console.log('6. Checking state after second room creation...');
        const finalStateRes = await makeRequest('GET', '/api/game-state');
        const finalState = JSON.parse(finalStateRes);
        console.log('Final state:', finalState.phase);
        
        if (finalState.phase === 'RockPaperScissors') {
            console.log('✅ ROOM ISSUE FIXED - Fresh room starts with RockPaperScissors');
            return true;
        } else {
            console.log('❌ ROOM ISSUE PERSISTS - Still getting wrong phase');
            console.log('Expected: RockPaperScissors');
            console.log('Actual:', finalState.phase);
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

debugRoomIssue().then(success => {
    console.log(success ? '\n✅ ROOM ISSUE FIXED' : '\n❌ ROOM ISSUE PERSISTS');
    process.exit(success ? 0 : 1);
});
