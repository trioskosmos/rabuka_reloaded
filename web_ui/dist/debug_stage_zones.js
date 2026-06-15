// Debug script to check stage zone mapping
import { State } from './js/state.js';

window.debugStageZones = function() {
    const state = State.data;
    if (!state.player1?.stage) {
        console.log('No stage data found');
        return;
    }
    
    console.log('=== STAGE ZONE DEBUG ===');
    console.log('Backend stage data:', state.player1.stage);
    
    const stageArray = [state.player1.stage.left_side, state.player1.stage.center, state.player1.stage.right_side];
    console.log('Frontend stage array:', stageArray);
    
    console.log('Legal actions:', state.legal_actions);
    
    // Check area mapping
    const areaMap = { 'left': 0, 'left_side': 0, 'center': 1, 'right': 2, 'right_side': 2 };
    console.log('Area mapping:', areaMap);
    
    // Check valid targets
    if (window.InteractionAdapter) {
        const validTargets = window.InteractionAdapter.get_valid_targets(state);
        console.log('Valid stage targets:', validTargets.myStage);
    }
    
    // Check DOM elements
    console.log('DOM stage slots:');
    for (let i = 0; i < 3; i++) {
        const slot = document.getElementById(`my-stage-slot-${i}`);
        if (slot) {
            console.log(`Slot ${i}:`, slot.innerHTML);
        }
    }
};

console.log('Stage zone debug loaded. Call window.debugStageZones() to debug.');
