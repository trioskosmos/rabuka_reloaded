/**
 * Interaction Adapter
 * Handles mapping of Rust backend actions to UI targets and validating legal actions.
 */
import { State } from './state.js';

export const InteractionAdapter = {
    /**
     * Calculates which UI elements are valid targets for the current list of legal actions.
     * @param {Object} state The current game state
     * @returns {Object} Mapping of zone names to valid action objects
     */
    get_valid_targets: (state) => {
        const valid = {
            myHand: {},
            oppHand: {},
            myStage: {},
            oppStage: {},
            myLive: {},
            oppLive: {},
            myEnergy: {},
            oppEnergy: {},
            discard: {},
            hasSelection: false
        };

        if (!state.legal_actions) return valid;

        state.legal_actions.forEach((action, index) => {
            // Support both parameters and params field names
            const params = action.parameters || action.params || {};
            const cardIndex = params.card_index;
            const cardIndices = params.card_indices;
            const stageArea = params.stage_area;
            const cardId = params.card_id;
            const cardNo = params.card_no;

            // Hand card actions
            if (cardIndex !== undefined) {
                valid.myHand[cardIndex] = { ...action, index };
            }
            if (cardIndices && cardIndices.length > 0) {
                cardIndices.forEach(idx => {
                    valid.myHand[idx] = { ...action, index };
                });
            }

            // Stage area actions
            if (stageArea) {
                // Rust engine MemberArea serializes as lowercase without underscores: "left", "center", "right"
                // Support both formats for compatibility
                const areaMap = { 'left': 0, 'left_side': 0, 'center': 1, 'right': 2, 'right_side': 2 };
                const stageIdx = areaMap[stageArea.toLowerCase()];
                if (stageIdx !== undefined) {
                    valid.myStage[stageIdx] = { ...action, index };
                }
            }

            // Live zone actions
            if (action.action_type.includes('Live') || action.action_type.includes('Performance')) {
                // For now, mark all live cards as valid targets
                const liveCards = state.player1 ? state.player1.live_zone.cards : [];
                if (liveCards.length > 0) {
                    liveCards.forEach((_, idx) => {
                        valid.myLive[idx] = { ...action, index };
                    });
                }
            }

            // Energy zone actions
            if (action.action_type.includes('Energy') || action.action_type.includes('Activate')) {
                const energyCards = state.player1 ? state.player1.energy.cards : [];
                if (energyCards.length > 0) {
                    energyCards.forEach((_, idx) => {
                        valid.myEnergy[idx] = { ...action, index };
                    });
                }
            }
        });

        valid.hasSelection = Object.keys(valid.myHand).length > 0 ||
                           Object.keys(valid.myStage).length > 0 ||
                           Object.keys(valid.myLive).length > 0 ||
                           Object.keys(valid.myEnergy).length > 0;

        return valid;
    },

    /**
     * Gets the action object for a given zone and index
     */
    get_action_for_target: (zone, index, state) => {
        const valid = InteractionAdapter.get_valid_targets(state);
        return valid[zone]?.[index];
    }
};
