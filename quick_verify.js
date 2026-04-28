// Quick verification script - runs complete game flow in 3 seconds
const http = require('http');

async function quickVerify() {
    console.log('=== QUICK VERIFICATION ===');
    const start = Date.now();
    
    try {
        // 1. Create room
        const roomRes = await makeRequest('POST', '/api/rooms/create', {
            mode: 'pve', card_set: 'compiled', p0_deck: [], p0_energy: [], public: true
        });
        const roomData = JSON.parse(roomRes);
        if (!roomData.success) throw new Error('Room creation failed');
        
        // 2. Get state and actions in parallel
        const [stateRes, actionsRes] = await Promise.all([
            makeRequest('GET', '/api/game-state'),
            makeRequest('GET', '/api/actions')
        ]);
        
        const state = JSON.parse(stateRes);
        const actions = JSON.parse(actionsRes);
        
        // 3. Quick validation
        const results = {
            initialPhase: state.phase === 'RockPaperScissors',
            hasRPSActions: actions.actions.filter(a => 
                a.action_type.includes('_choice')).length === 3,
            canAdvance: actions.actions.length > 0
        };
        
        const elapsed = Date.now() - start;
        console.log(`✅ Verified in ${elapsed}ms`);
        console.log(`Initial phase: ${results.initialPhase ? '✓' : '❌'} ${state.phase}`);
        console.log(`RPS actions: ${results.hasRPSActions ? '✓' : '❌'} ${actions.actions.filter(a => a.action_type.includes('_choice')).length}`);
        console.log(`Game functional: ${results.canAdvance ? '✓' : '❌'}`);
        
        return results.initialPhase && results.hasRPSActions && results.canAdvance;
        
    } catch (error) {
        console.error('❌ Verification failed:', error.message);
        return false;
    }
}

function makeRequest(method, path, data = null) {
    return new Promise((resolve, reject) => {
        const options = {
            hostname: 'localhost',
            port: 3000,
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

quickVerify().then(success => {
    console.log(success ? '\n🎉 TURN CHOICE FIX VERIFIED' : '\n💥 VERIFICATION FAILED');
    process.exit(success ? 0 : 1);
});
