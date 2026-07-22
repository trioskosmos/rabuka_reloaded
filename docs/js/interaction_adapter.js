/**
 * Interaction Adapter
 * Handles mapping of Rust backend actions to UI targets and validating legal actions.
 */
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

        state.legal_actions.forEach((action) => {
            const params = action.parameters || action.params || {};
            const cardIndex = params.card_index;
            const cardIndices = params.card_indices;
            const stageArea = params.stage_area;
            const cardId = params.card_id;
            const cardNo = params.card_no;
            const actionType = action.action_type || '';

            // select_card actions (ChoiceSelect) — map to both zone-specific and .selection
            if (actionType === 'select_card') {
                if (!valid.selection) valid.selection = {};
                if (cardIndex !== undefined) {
                    valid.selection[cardIndex] = action;
                    const zone = state.pending_choice?.zone;
                    if (zone === 'hand') valid.myHand[cardIndex] = action;
                    else if (zone === 'discard') valid.discard[cardIndex] = action;
                }
                if (cardIndices && cardIndices.length > 0) {
                    cardIndices.forEach(idx => {
                        const perCard = { ...action, parameters: { ...action.parameters, card_index: idx } };
                        delete perCard.parameters.card_indices;
                        valid.selection[idx] = perCard;
                        const zone = state.pending_choice?.zone;
                        if (zone === 'hand') valid.myHand[idx] = perCard;
                        else if (zone === 'discard') valid.discard[idx] = perCard;
                    });
                }
                return;
            }

            // Hand card actions (non-select_card fallback)
            if (cardIndex !== undefined) {
                valid.myHand[cardIndex] = action;
            }
            if (cardIndices && cardIndices.length > 0) {
                cardIndices.forEach(idx => {
                    const perCard = { ...action, parameters: { ...action.parameters, card_index: idx } };
                    delete perCard.parameters.card_indices;
                    valid.myHand[idx] = perCard;
                });
            }

            // Stage area actions
            if (stageArea) {
                const areaMap = { 'left': 0, 'left_side': 0, 'center': 1, 'right': 2, 'right_side': 2 };
                const stageIdx = areaMap[stageArea.toLowerCase()];
                if (stageIdx !== undefined) {
                    valid.myStage[stageIdx] = action;
                }
            }

            // Live zone actions — check both players, use the one with cards
            if (actionType.includes('Live') || actionType.includes('Performance')) {
                const pLive = state.player1?.live_zone?.cards?.length ? state.player1.live_zone.cards :
                             state.player2?.live_zone?.cards?.length ? state.player2.live_zone.cards : [];
                pLive.forEach((_, idx) => {
                    valid.myLive[idx] = action;
                });
            }

            // Energy zone actions — check both players
            if (actionType.includes('Energy') || actionType.includes('Activate')) {
                const pEnergy = state.player1?.energy?.cards?.length ? state.player1.energy.cards :
                               state.player2?.energy?.cards?.length ? state.player2.energy.cards : [];
                pEnergy.forEach((_, idx) => {
                    valid.myEnergy[idx] = action;
                });
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
