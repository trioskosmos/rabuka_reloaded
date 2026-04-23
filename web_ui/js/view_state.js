import { Phase } from './constants.js';

function getSelectedIndices(state, uiState, perspectivePlayer) {
    const isMulligan = state.phase === Phase.MULLIGAN;
    if (isMulligan) {
        const player = perspectivePlayer === 0 ? state.player1 : state.player2;
        const serverSelection = player.mulligan_selection;
        const indices = new Set(uiState.localMulliganSelection);

        // Engine may not send mulligan_selection in PlayerDisplay
        if (serverSelection !== undefined) {
            if (typeof serverSelection === 'number') {
                // Rust backend: hand is { cards: [...] }
                const handCards = player.hand.cards;
                for (let i = 0; i < handCards.length; i++) {
                    if ((serverSelection >> i) & 1) indices.add(i);
                }
            } else if (Array.isArray(serverSelection)) {
                serverSelection.forEach(idx => indices.add(Number(idx)));
            }
        }

        return Array.from(indices);
    }
    return uiState.selectedHandIdx !== -1 ? [uiState.selectedHandIdx] : [];
}

function buildConfirmedActions(selectedIndices, validTargets) {
    const confirmedActions = {};
    selectedIndices.forEach((handIdx, internalIdx) => {
        if (validTargets.myHand[handIdx] !== undefined) {
            confirmedActions[internalIdx] = validTargets.myHand[handIdx];
        }
    });
    return confirmedActions;
}

function hasActiveEffects(state, p0, p1) {
    return Boolean(
        (state.triggered_abilities && state.triggered_abilities.length > 0) ||
        (p0.blade_buffs && p0.blade_buffs.some(v => v !== 0)) ||
        (p0.heart_buffs && p0.heart_buffs.some(hb => hb.some(v => v > 0))) ||
        (p1.blade_buffs && p1.blade_buffs.some(v => v !== 0)) ||
        (p1.heart_buffs && p1.heart_buffs.some(hb => hb.some(v => v > 0))) ||
        (p0.cost_reduction ?? 0) !== 0 ||
        (p1.cost_reduction ?? 0) !== 0 ||
        // Support both prevent_baton_touch and prevent_baton
        ((p0.prevent_baton_touch ?? p0.prevent_baton ?? 0) > 0) ||
        ((p1.prevent_baton_touch ?? p1.prevent_baton ?? 0) > 0)
    );
}

export const ViewState = {
    buildRenderModel(state, uiState, validTargets) {
        const perspectivePlayer = uiState.hotseatMode && state.active_player !== undefined
            ? state.active_player
            : uiState.perspectivePlayer;

        // Rust backend format: state.player1, state.player2
        const p0 = perspectivePlayer === 0 ? state.player1 : state.player2;
        const p1 = perspectivePlayer === 0 ? state.player2 : state.player1;

        const isMulligan = state.phase === Phase.MULLIGAN;
        const selectedIndices = getSelectedIndices(state, uiState, perspectivePlayer);
        const handFilter = (_, idx) => !isMulligan || !selectedIndices.some(s => Number(s) === Number(idx));
        // Rust backend: hand is { cards: [...] }
        const handCards = p0.hand.cards;
        const mulliganSelectedCards = isMulligan ? selectedIndices.map(idx => handCards[idx]).filter(card => card !== null && card !== undefined) : [];
        const confirmedCards = isMulligan ? [] : selectedIndices.map(idx => handCards[idx]).filter(card => card !== null && card !== undefined);

        const pendingChoice = state.pending_choice;
        const selectionCards = pendingChoice?.selection_cards || [];
        const selectionActions = selectionCards.map((_, idx) => validTargets.selection[idx]);

        return {
            perspectivePlayer,
            p0,
            p1,
            isMulligan,
            selectedIndices,
            handFilter,
            confirmedCards,
            mulliganSelectedCards,
            confirmedActions: buildConfirmedActions(selectedIndices, validTargets),
            showMulliganReturn: uiState.showMulliganReturn && uiState.lastMulliganCards.length > 0,
            mulliganReturnCards: uiState.lastMulliganCards,
            hasActiveEffects: hasActiveEffects(state, p0, p1),
            selectionModal: {
                isVisible: selectionCards.length > 0,
                cards: selectionCards,
                actions: selectionActions,
            },
        };
    },
};
