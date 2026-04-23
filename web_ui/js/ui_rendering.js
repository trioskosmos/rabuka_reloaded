/**
 * UI Rendering Module
 * Handles all board, card, and performance result rendering.
 */
import { State } from './state.js';
import { CardRenderer } from './components/CardRenderer.js';
import { BoardRenderer } from './components/BoardRenderer.js';
import { ActionMenu } from './components/ActionMenu.js';

import { Phase, fixImg } from './constants.js';
import * as i18n from './i18n/index.js';
import { Tooltips } from './ui_tooltips.js';
import { InteractionAdapter } from './interaction_adapter.js';
import { LogRenderer as Logs } from './components/LogRenderer.js';
import { PerformanceRenderer } from './components/PerformanceRenderer.js';

import { HeaderStats } from './components/HeaderStats.js';
import { ZoneViewer } from './components/ZoneViewer.js';
import { DOM_IDS, DISPLAY_VALUES } from './constants_dom.js';
import { DOMUtils } from './utils/DOMUtils.js';
import { ViewState } from './view_state.js';

// Cached DOM element references for performance
const DOM_CACHE = {
    myHand: null,
    oppHand: null,
    myStage: null,
    oppStage: null,
    myLive: null,
    oppLive: null,
    myEnergy: null,
    oppEnergy: null,
    myDiscard: null,
    oppDiscard: null,
    mySuccess: null,
    oppSuccess: null,
    actions: null,
    ruleLog: null,
    activeAbilitiesList: null,
    activeAbilitiesPanel: null,
};

// Initialize DOM cache on first use
let domCacheInitialized = false;
function initDomCache() {
    if (domCacheInitialized) return;
    domCacheInitialized = true;
    for (const [key, id] of Object.entries({
        myHand: 'my-hand',
        oppHand: 'opp-hand',
        myStage: 'my-stage',
        oppStage: 'opp-stage',
        myLive: 'my-live',
        oppLive: 'opp-live',
        myEnergy: 'my-energy',
        oppEnergy: 'opp-energy',
        myDiscard: 'my-discard-visual',
        oppDiscard: 'opp-discard-visual',
        mySuccess: 'my-success',
        oppSuccess: 'opp-success',
        actions: 'actions',
        ruleLog: 'rule-log',
        activeAbilitiesList: 'active-abilities-list',
        activeAbilitiesPanel: 'active-abilities-panel',
    })) {
        DOM_CACHE[key] = document.getElementById(id);
    }
}

export const Rendering = {
    render: () => {
        if (State.renderRequested) return;
        State.renderRequested = true;
        
        // Initial fade-in - REMOVED for snappiness
        State.firstRenderDone = true;

        requestAnimationFrame(() => {
            try {
                initDomCache();
                // initAccessibility(); // Missing in codebase, causes ReferenceError
                Rendering.renderInternal();
            } catch (error) {
                console.error('Fatal Rendering Error:', error);
            } finally {
                State.renderRequested = false;
            }
        });
    },

    renderHeaderStats: (state, p0) => {
        HeaderStats.render(state, p0, Rendering.getPhaseKey);
    },

    get_valid_targets: InteractionAdapter.get_valid_targets,

    renderInternal: () => {
        const state = State.data;
        // Rust backend format: state.player1, state.player2
        if (!state || (!state.player1 && !state.player2)) return;

        // --- Proactive Pre-loading ---
        const assetsToLoad = [];
        [state.player1, state.player2].forEach(p => {
            if (p?.hand?.cards) p.hand.cards.forEach(c => { if (c?.card_no) assetsToLoad.push(`img/cards_webp/${c.card_no}.webp`); });
            if (p?.stage) {
                if (p.stage.left_side?.card_no) assetsToLoad.push(`img/cards_webp/${p.stage.left_side.card_no}.webp`);
                if (p.stage.center?.card_no) assetsToLoad.push(`img/cards_webp/${p.stage.center.card_no}.webp`);
                if (p.stage.right_side?.card_no) assetsToLoad.push(`img/cards_webp/${p.stage.right_side.card_no}.webp`);
            }
        });

        const assetsHash = assetsToLoad.join('|');
        if (State.lastAssetsHash !== assetsHash) {
            if (window.preloadAssets) window.preloadAssets(assetsToLoad);
            State.lastAssetsHash = assetsHash;
        }

        const validTargets = Rendering.get_valid_targets(state);
        const viewState = ViewState.buildRenderModel(state, State, validTargets);

        if (State.hotseatMode && State.perspectivePlayer !== viewState.perspectivePlayer) {
            State.perspectivePlayer = viewState.perspectivePlayer;
        }

        const { p0, p1 } = viewState;

        if (p0) state.looked_cards = p0.looked_cards || [];
        if (!p0 || !p1) return;

        // Update UI Headers, Stats, etc. (Logic moved from main.js)
        Rendering.renderHeaderStats(state, p0);
        Rendering.renderBoard(state, p0, p1, validTargets);

        Rendering.renderMulliganReturn(viewState);

        if (viewState.isMulligan) {
            // Unselected cards in Hand - Rust backend: hand is { cards: [...] }
            Rendering.renderCards('my-hand', p0.hand?.cards || [], true, false, viewState.selectedIndices, validTargets.myHand, validTargets.hasSelection, viewState.handFilter);
            
            // Selected cards shown at bottom of deck during mulligan (visual representation)
            // Do not show in "Confirmed Cards" panel during mulligan
        } else {
            Rendering.renderCards('my-hand', p0.hand?.cards || [], true, false, viewState.selectedIndices, validTargets.myHand, validTargets.hasSelection);
            Rendering.renderLookedCards(validTargets.selection);
        }
        Rendering.renderSelectionModal(viewState.selectionModal);
        Rendering.renderRuleLog();
        Rendering.renderActiveEffects(state);
        DOMUtils.setVisible(DOM_IDS.ACTIVE_ABILITIES_PANEL, viewState.hasActiveEffects, DISPLAY_VALUES.BLOCK);
        if (state.game_over) {
            Rendering.renderGameOver(state);
        } else {
            Rendering.renderActions();
        }

        Tooltips.highlightPendingSource();
        Rendering.updateSettingsButtons(viewState.perspectivePlayer);
    },

    getPhaseKey: (phase) => {
        const perspectivePlayer = State.perspectivePlayer;
        if (phase === Phase.RPS) return 'rps';
        if (phase === Phase.SETUP) return 'setup';
        if (phase === Phase.MULLIGAN_P1) return (perspectivePlayer === 0) ? 'mulligan_you' : 'mulligan_opp';
        if (phase === Phase.MULLIGAN_P2) return (perspectivePlayer === 1) ? 'mulligan_you' : 'mulligan_opp';
        if (phase === Phase.ACTIVE) return 'active';
        if (phase === Phase.ENERGY) return 'energy';
        if (phase === Phase.DRAW) return 'draw';
        if (phase === Phase.MAIN) return 'main';
        if (phase === Phase.LIVE_SET) return 'live_set';
        if (phase === Phase.PERFORMANCE_P1) return (perspectivePlayer === 0) ? 'perf_p1' : 'perf_p2';
        if (phase === Phase.PERFORMANCE_P2) return (perspectivePlayer === 1) ? 'perf_p1' : 'perf_p2';
        if (phase === Phase.LIVE_RESULT) return 'live_result';
        return 'wait';
    },


    renderBoard: (state, p0, p1, validTargets = { stage: {}, discard: {}, hasSelection: false }) => {
        BoardRenderer.renderBoard(state, p0, p1, validTargets, Rendering.showDiscardModal);
    },

    renderDeckCounts: (p0, p1) => {
        BoardRenderer.renderDeckCounts(p0, p1);
    },

    renderCards: (containerId, cards, clickable = false, mini = false, selectedIndices = [], validActionMap = {}, hasGlobalSelection = false, filter = null) => {
        CardRenderer.renderCards(containerId, cards, clickable, mini, selectedIndices, validActionMap, hasGlobalSelection, filter);
    },

    renderStage: (containerId, stage, clickable, validActionMap = {}, hasGlobalSelection = false) => {
        CardRenderer.renderStage(containerId, stage, clickable, validActionMap, hasGlobalSelection);
    },

    renderEnergy: (containerId, energy, clickable = false, validActionMap = {}, hasGlobalSelection = false) => {
        BoardRenderer.renderEnergy(containerId, energy, clickable, validActionMap, hasGlobalSelection, State.data);
    },

    renderLiveZone: (containerId, liveCards, visible, validActionMap = {}, hasGlobalSelection = false) => {
        CardRenderer.renderLiveZone(containerId, liveCards, visible, validActionMap, hasGlobalSelection);
    },

    renderDiscardPile: (containerId, discard, playerIdx, validActionMap = {}, hasGlobalSelection = false) => {
        CardRenderer.renderDiscardPile(containerId, discard, playerIdx, validActionMap, hasGlobalSelection, Rendering.showDiscardModal);
    },

    renderActiveAbilities: (containerId, abilities) => Logs.renderActiveAbilities(containerId, abilities),

    renderMulliganReturn: (viewState) => {
        const shouldShowMulliganCards = viewState.showMulliganReturn || (viewState.isMulligan && viewState.mulliganSelectedCards.length > 0);
        DOMUtils.setVisible(DOM_IDS.MY_DECK_BOTTOM, shouldShowMulliganCards, DISPLAY_VALUES.FLEX);
        DOMUtils.setVisible(DOM_IDS.OPP_DECK_BOTTOM, false);

        if (shouldShowMulliganCards) {
            const cardsToShow = viewState.showMulliganReturn ? viewState.mulliganReturnCards : viewState.mulliganSelectedCards;
            Rendering.renderCards(DOM_IDS.MY_DECK_BOTTOM, cardsToShow, false, false);
        }
    },

    renderSelectionModal: (selectionModal = null) => {
        const modalState = selectionModal || { isVisible: false, cards: [], actions: [] };
        const panel = document.getElementById(DOM_IDS.SELECTION_MODAL);
        const content = document.getElementById(DOM_IDS.SELECTION_CONTENT);
        if (!panel || !content) return;

        const useSidebarCards = modalState.cards.length > 0 && window.innerWidth > 768;
        if (!modalState.isVisible || useSidebarCards) {
            panel.style.display = DISPLAY_VALUES.NONE;
            return;
        }

        panel.style.display = DISPLAY_VALUES.FLEX;

        content.innerHTML = '';
        modalState.cards.forEach((c, idx) => {
            const action = modalState.actions[idx];
            const viewModel = CardRenderer.getCardViewModel(c, {
                containerId: DOM_IDS.SELECTION_CONTENT,
                actionId: action?.index,
            });
            const cardEl = CardRenderer.createCardDOM(viewModel, c, () => {
                if (window.doAction) window.doAction(action);
            });
            cardEl.className = `selection-card-item ${viewModel.classes}`;
            content.appendChild(cardEl);
        });
    },

    renderGameOver: (state) => {
        ActionMenu.renderGameOver(state);
    },

    showDiscardModal: (playerIdx) => ZoneViewer.showDiscard(playerIdx),
    showZoneViewer: (playerIdx) => ZoneViewer.showZoneViewer(playerIdx),

    renderActions: () => {
        ActionMenu.renderActions();
    },

    renderPerformanceGuide: () => PerformanceRenderer.renderPerformanceGuide(Rendering.renderHeartProgress),


    renderRuleLog: () => Logs.renderRuleLog('rule-log'),

    renderActiveEffects: (state) => Logs.renderActiveEffects(state),


    renderLookedCards: (selectionTargets = {}, overrideCards = null, overrideTitle = null) => {
        CardRenderer.renderLookedCards(selectionTargets, overrideCards, overrideTitle);
    },

    renderPerformanceResult: (results = null) => PerformanceRenderer.renderPerformanceResult(results),
    renderHeartProgress: (filled, required) => PerformanceRenderer.renderHeartProgress(filled, required),

    renderHeartsCompact: (hearts) => PerformanceRenderer.renderHeartsCompact(hearts),
    renderBladeHeartsCompact: (hearts) => PerformanceRenderer.renderHeartsCompact(hearts),
    renderBladesCompact: (blades) => PerformanceRenderer.renderBladesCompact(blades),
    renderTotalHeartsBreakdown: (hearts) => PerformanceRenderer.renderHeartsCompact(hearts),

    renderModifiers: () => { /* Placeholder for future implementation */ },
    renderGameData: () => { /* Placeholder for future implementation */ },

    updateSettingsButtons: (perspectivePlayer = State.perspectivePlayer) => {
        const liveWatchBtn = document.getElementById('live-watch-btn');
        if (liveWatchBtn) {
            const label = i18n.t('live_watch');
            const stateLabel = State.isLiveWatchOn ? i18n.t('on') : i18n.t('off');
            liveWatchBtn.textContent = `${label}: ${stateLabel}`;
        }

        const hotseatBtn = document.getElementById('pvp-btn');
        if (hotseatBtn) {
            const label = i18n.t('shared_screen');
            const stateLabel = State.hotseatMode ? i18n.t('on') : i18n.t('off');
            hotseatBtn.textContent = `${label}: ${stateLabel}`;
        }

        const perspectiveBtn = document.getElementById('switch-btn');
        if (perspectiveBtn) {
            const label = i18n.t('view_persp');
            perspectiveBtn.textContent = `${label}: P${perspectivePlayer + 1}`;
        }

        const friendlyBtn = document.getElementById('friendly-abilities-btn');
        if (friendlyBtn) {
            const label = i18n.t('friendly_abilities');
            const stateLabel = State.showFriendlyAbilities ? i18n.t('on') : i18n.t('off');
            friendlyBtn.textContent = `${label}: ${stateLabel}`;
        }

        const langBtn = document.getElementById('lang-btn');
        if (langBtn) {
            langBtn.textContent = State.currentLang === 'jp' ? 'English' : '日本語';
        }
    },

    showPerfTab: (tab) => PerformanceRenderer.showPerfTab(tab),

    renderTurnHistory: () => PerformanceRenderer.renderTurnHistory(Rendering.getPhaseKey)
};

// Automatic rendering on state change
if (typeof window !== 'undefined') {
    State.on('change', () => Rendering.render());
}

// Global Highlighting Logic for Bidirectional Linkage
window.highlightActionBtn = (actionId, active) => {
    if (actionId === undefined) return;
    
    // Update global hover state for persistence across re-renders
    if (active) {
        State.hoveredActionId = actionId;
    } else if (State.hoveredActionId === actionId) {
        State.hoveredActionId = null;
    }

    // 1. Highlight the button(s)
    const btns = document.querySelectorAll(`.action-btn[data-action-id="${actionId}"]`);
    btns.forEach(btn => {
        if (active) btn.classList.add('hover-highlight');
        else btn.classList.remove('hover-highlight');
    });
    
    // 2. Highlight all linked components (Cards, Slots, etc.)
    const linked = document.querySelectorAll(`[data-action-id="${actionId}"]:not(.action-btn)`);
    linked.forEach(el => {
        if (active) el.classList.add('hover-highlight');
        else el.classList.remove('hover-highlight');
    });
};

window.highlightActionTarget = window.highlightActionBtn;

