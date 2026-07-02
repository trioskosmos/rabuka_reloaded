import { isMulliganPhase, isLiveCardSetPhase } from './constants.js';

function getSelectedIndices(state, uiState, perspectivePlayer) {
    if (isMulliganPhase(state.phase)) {
        const player = perspectivePlayer === 0 ? state.player1 : state.player2;
        const serverSelection = player.mulligan_selection;
        const indices = new Set(uiState.localMulliganSelection);
        if (serverSelection !== undefined) {
            if (typeof serverSelection === 'number') {
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
    if (isLiveCardSetPhase(state.phase)) {
        return Array.from(uiState.localLiveCardSelection);
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

export const ViewState = {
    buildRenderModel(state, uiState, validTargets) {
        const activePlayerNum = state.active_player === 'player2' || state.active_player === 'p2' || state.active_player === '1' || state.active_player === 1 ? 1 : 0;
        const gameMode = state.mode || State.data?.mode;
        const perspectivePlayer = gameMode !== 'pvp' && gameMode !== 'pve' && state.active_player !== undefined
            ? activePlayerNum
            : uiState.perspectivePlayer;

        // Rust backend format: state.player1, state.player2
        const p0 = perspectivePlayer === 0 ? state.player1 : state.player2;
        const p1 = perspectivePlayer === 0 ? state.player2 : state.player1;

        const isMulligan = isMulliganPhase(state.phase);
        const selectedIndices = getSelectedIndices(state, uiState, perspectivePlayer);
        const handFilter = null; // Render all hand cards during mulligan so selected cards remain visible in-place
        // Rust backend: hand is { cards: [...] }
        const handCards = p0.hand.cards;
        const mulliganSelectedCards = isMulligan ? selectedIndices.map(idx => handCards[idx]).filter(card => card !== null && card !== undefined) : [];
        const confirmedCards = isMulligan ? [] : selectedIndices.map(idx => handCards[idx]).filter(card => card !== null && card !== undefined);

        const pendingChoice = state.pending_choice;
        const rawSelectionCards = pendingChoice?.selection_cards || [];
        // Only include cards that have a matching legal action
        // (backend may send all zone cards; legal_actions define which are valid choices)
        const selectionPairs = [];
        rawSelectionCards.forEach(c => {
            const cardId = c.id !== undefined ? c.id : c.card_id;
            const action = state.legal_actions?.find(a => {
                const params = a.parameters || {};
                return params.card_id === cardId || params.card_id === c.card_id;
            });
            if (action) selectionPairs.push({ card: c, action });
        });
        const selectionCards = selectionPairs.map(p => p.card);
        const selectionActions = selectionPairs.map(p => p.action);

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
            selectionModal: {
                isVisible: selectionCards.length > 0,
                cards: selectionCards,
                actions: selectionActions,
            },
        };
    },
};
