/**
 * UI Rendering Module
 * Handles all board, card, and performance result rendering.
 */
import { State } from './state.js';
import { CardRenderer, ImageLoader, resolveCardImagePath } from './components/CardRenderer.js';
import { BoardRenderer } from './components/BoardRenderer.js';
import { ActionMenu } from './components/ActionMenu.js';

import { Phase, isMulliganPhase, isLiveCardSetPhase } from './constants.js';
import { AiDriver } from './components/AiDriver.js';
import * as i18n from './i18n/index.js';
import { Tooltips } from './ui_tooltips.js';
import { InteractionAdapter } from './interaction_adapter.js';
import { LogRenderer as Logs } from './components/LogRenderer.js';
import { PerformanceRenderer } from './components/PerformanceRenderer.js';
import { switchBoard } from './layout.js';

import { HeaderStats } from './components/HeaderStats.js';
import { ZoneViewer } from './components/ZoneViewer.js';
import { DOM_IDS, DISPLAY_VALUES } from './constants_dom.js';
import { DOMUtils } from './utils/DOMUtils.js';
import { ViewState } from './view_state.js';

let _lastActivePlayer = -1;
let _viewInitialized = false;

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

        mySuccess: 'my-success',
        oppSuccess: 'opp-success',
        actions: 'actions',
        ruleLog: 'rule-log',
    })) {
        DOM_CACHE[key] = document.getElementById(id);
    }
}

export const Rendering = {
    render: () => {
        if (State.renderRequested) return;

        State._forceRender = false;

        State.renderRequested = true;
        State.firstRenderDone = true;

        requestAnimationFrame(() => {
            try {
                initDomCache();
                Rendering.renderInternal();
            } catch (error) {
                console.error('Fatal Rendering Error:', error);
            } finally {
                State.renderRequested = false;
            }
        });
    },

    renderHeaderStats: (state, p0, p1) => {
        HeaderStats.render(state, p0, p1, Rendering.getPhaseKey);
    },

    get_valid_targets: InteractionAdapter.get_valid_targets,

    renderInternal: () => {
        const state = State.data;
        // Rust backend format: state.player1, state.player2
        if (!state || (!state.player1 && !state.player2)) {
            AiDriver.stop();
            return;
        }

        const assetsToLoad = [];
        [state.player1, state.player2].forEach(p => {
            const handCards = p?.hand?.cards;
            if (!handCards) return;
            handCards.forEach(c => {
                if (c?.card_no) {
                    const path = resolveCardImagePath(c.card_no);
                    if (path) assetsToLoad.push(path);
                }
            });
            if (p?.stage) {
                [p.stage.left_side, p.stage.center, p.stage.right_side].forEach(slot => {
                    if (slot?.card_no) {
                        const path = resolveCardImagePath(slot.card_no);
                        if (path) assetsToLoad.push(path);
                    }
                });
            }
        });

        const assetsHash = assetsToLoad.join('|');
        if (State.lastAssetsHash !== assetsHash) {
            if (window.preloadAssets) window.preloadAssets(assetsToLoad);
            State.lastAssetsHash = assetsHash;
        }

        const validTargets = Rendering.get_valid_targets(state);
        const viewState = ViewState.buildRenderModel(state, State, validTargets);

        if (State.data?.mode !== 'pvp' && State.data?.mode !== 'pve' && State.perspectivePlayer !== viewState.perspectivePlayer) {
            State.updateUiConfig({ perspective_player: viewState.perspectivePlayer });
        }

        const { p0, p1 } = viewState;

        if (p0) state.looked_cards = p0.looked_cards;
        if (!p0 || !p1) return;

        // Update UI Headers, Stats, etc. (Logic moved from main.js)
        Rendering.renderHeaderStats(state, p0, p1);
        Rendering.renderBoard(state, p0, p1, validTargets);
        // Adjust stage scroll centering: center when content fits, align to start edge when it overflows
        const myStage = document.getElementById('my-stage');
        if (myStage) myStage.style.justifyContent = myStage.scrollWidth > myStage.clientWidth ? 'flex-start' : 'center';
        const oppStage = document.getElementById('opp-stage');
        if (oppStage) oppStage.style.justifyContent = oppStage.scrollWidth > oppStage.clientWidth ? 'flex-start' : 'center';

        Rendering.renderMulliganReturn(viewState);

        // Always render both hands through CardRenderer. When opponent cards
        // are hidden, CardRenderer.getCardViewModel handles per-card hiding
        // via the `hidden` flag → card-back class. The old `innerHTML = ''`
        // approach caused DOM destruction and prevented smooth updates.
        if (viewState.isMulligan) {
            Rendering.renderCards('my-hand', p0.hand.cards, true, false, viewState.selectedIndices, validTargets.myHand, validTargets.hasSelection, viewState.handFilter);
        } else {
            Rendering.renderCards('my-hand', p0.hand.cards, true, false, viewState.selectedIndices, validTargets.myHand, validTargets.hasSelection);
        }
        Rendering.renderCards('opp-hand', p1.hand.cards, false, false);
        Rendering.renderSelectionModal(viewState.selectionModal);
        Rendering.renderRuleLog();
        if (state.game_over) {
            Rendering.renderGameOver(state);
        } else {
            Rendering.renderActions();
        }

        // Update board toggle labels based on perspective
        const selfLabel = viewState.perspectivePlayer === 0 ? 'P1' : 'P2';
        const oppLabel = viewState.perspectivePlayer === 0 ? 'P2' : 'P1';
        const activePlayerNum = state.active_player === 'player2' || state.active_player === 'p2' || state.active_player === '1' || state.active_player === 1 ? 1 : 0;
        const isSelfActive = state.active_player !== undefined && viewState.perspectivePlayer === activePlayerNum;
        const selfRole = isSelfActive ? 'Attacker' : 'Defender';
        const oppRole = isSelfActive ? 'Defender' : 'Attacker';
        const selfSuffix = state.active_player !== undefined ? ` — ${selfLabel} (${selfRole})` : ` (${selfLabel})`;
        const oppSuffix = state.active_player !== undefined ? ` — ${oppLabel} (${oppRole})` : ` (${oppLabel})`;
        const ui = i18n.getCurrentTranslations()?.ui || {};
        const playerBtn = document.getElementById('btn-show-player');
        const oppBtn = document.getElementById('btn-show-opponent');
        const bothBtn = document.getElementById('btn-show-both');
        if (playerBtn) playerBtn.textContent = `${ui.my_board || 'My Board'}${selfSuffix}`;
        if (oppBtn) oppBtn.textContent = `${ui.opponent || 'Opponent'}${oppSuffix}`;
        if (bothBtn) bothBtn.textContent = `Both (${selfLabel} + ${oppLabel})`;

        // Default to both-mode view on first render
        if (!_viewInitialized && bothBtn && !bothBtn.classList.contains('active')) {
            switchBoard('both');
        }

        // Track active player for reference (no auto-switch)
        if (_lastActivePlayer === -1 && state.active_player !== undefined) {
            _lastActivePlayer = activePlayerNum;
        }
        _viewInitialized = true;
        _lastActivePlayer = activePlayerNum;

        Tooltips.highlightPendingSource();

        // Update language button labels
        document.querySelectorAll('[data-action="toggle-lang"]').forEach(btn => {
            btn.textContent = State.currentLang === 'jp' ? 'English' : 'Japanese';
        });

        AiDriver.think();
    },

    getPhaseKey: (phase) => {
        const perspectivePlayer = State.perspectivePlayer;
        if (!phase) return 'wait';
        
        // Handle string phase names from backend directly
        if (typeof phase === 'string') {
            if (phase === 'RockPaperScissors') return 'RockPaperScissors';
            if (phase === 'ChooseFirstAttacker') return 'ChooseFirstAttacker';
            if (phase === 'MulliganFirstAttacker' || phase === 'MulliganSecondAttacker') return 'MulliganFirstAttacker';
            if (phase === 'Active') return 'Active';
            if (phase === 'Energy') return 'Energy';
            if (phase === 'Draw') return 'Draw';
            if (phase === 'Main') return 'Main';
            if (phase === 'LiveCardSetFirstAttacker' || phase === 'LiveCardSetSecondAttacker') return 'LiveCardSetFirstAttacker';
            if (phase === 'FirstAttackerPerformance') return (perspectivePlayer === 0) ? 'perf_p1' : 'perf_p2';
            if (phase === 'SecondAttackerPerformance') return (perspectivePlayer === 1) ? 'perf_p1' : 'perf_p2';
            if (phase === 'LiveVictoryDetermination') return 'LiveVictoryDetermination';
            return phase;
        }
        
        // Fallback: numeric Phase constants
        if (phase === Phase.ROCK_PAPER_SCISSORS) return 'RockPaperScissors';
        if (phase === Phase.CHOOSE_FIRST_ATTACKER) return 'ChooseFirstAttacker';
        if (isMulliganPhase(phase)) return 'MulliganFirstAttacker';
        if (phase === Phase.ACTIVE) return 'Active';
        if (phase === Phase.ENERGY) return 'Energy';
        if (phase === Phase.DRAW) return 'Draw';
        if (phase === Phase.MAIN) return 'Main';
        if (isLiveCardSetPhase(phase)) return 'LiveCardSetFirstAttacker';
        if (phase === Phase.FIRST_ATTACKER_PERFORMANCE) return (perspectivePlayer === 0) ? 'perf_p1' : 'perf_p2';
        if (phase === Phase.SECOND_ATTACKER_PERFORMANCE) return (perspectivePlayer === 1) ? 'perf_p1' : 'perf_p2';
        if (phase === Phase.LIVE_VICTORY_DETERMINATION) return 'LiveVictoryDetermination';
        
        return String(phase);
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
        const shouldShowMulliganCards = viewState.showMulliganReturn;
        DOMUtils.setVisible(DOM_IDS.MY_DECK_BOTTOM, shouldShowMulliganCards, DISPLAY_VALUES.FLEX);
        DOMUtils.setVisible(DOM_IDS.OPP_DECK_BOTTOM, false);

        if (shouldShowMulliganCards) {
            Rendering.renderCards(DOM_IDS.MY_DECK_BOTTOM, viewState.mulliganReturnCards, false, false);
        }
    },

    renderSelectionModal: (selectionModal = null) => {
        const modalState = selectionModal || { isVisible: false, cards: [], actions: [] };
        const panel = document.getElementById(DOM_IDS.SELECTION_MODAL);
        const content = document.getElementById(DOM_IDS.SELECTION_CONTENT);
        if (!panel || !content) return;

        // Filter to only cards that have a valid action
        const validPairs = [];
        modalState.cards.forEach((c, idx) => {
            const action = modalState.actions[idx];
            if (action) validPairs.push({ card: c, action });
        });
        const filteredCards = validPairs.map(p => p.card);
        const filteredActions = validPairs.map(p => p.action);

        // Hide selection modal when there's no pending choice (choice was resolved)
        if (!State.data?.pending_choice) {
            panel.style.display = DISPLAY_VALUES.NONE;
            return;
        }

        // ChoiceView already populates the modal — don't override
        return;

        panel.style.display = DISPLAY_VALUES.FLEX;

        content.innerHTML = '';
        filteredCards.forEach((c, idx) => {
            const action = filteredActions[idx];
            const viewModel = CardRenderer.getCardViewModel(c, {
                containerId: DOM_IDS.SELECTION_CONTENT,
                actionId: action?.index,
            });
            const onClick = () => {
                if (State.uiMode === 'view') {
                    const m = window.__modals?.CardDetailModal;
                    if (m) m.open(c);
                } else if (window.doAction) {
                    window.doAction(action);
                }
            };
            const cardEl = CardRenderer.createCardDOM(viewModel, c, onClick);
            CardRenderer.renderCardBonuses(cardEl, c, true);
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
        ActionMenu.updateMobileActionBadge();
    },

    renderPerformanceGuide: () => PerformanceRenderer.renderPerformanceGuide(Rendering.renderHeartProgress),


    renderRuleLog: () => Logs.renderRuleLog('rule-log'),

    renderLookedCards: (selectionTargets = {}, overrideCards = null, overrideTitle = null) => {
        CardRenderer.renderLookedCards(selectionTargets, overrideCards, overrideTitle);
    },

    renderPerformanceResult: (results = null) => PerformanceRenderer.renderPerformanceResult(results),
    renderHeartProgress: (filled, required) => PerformanceRenderer.renderHeartProgress(filled, required),

    renderHeartsCompact: (hearts) => PerformanceRenderer.renderHeartsCompact(hearts),
    renderBladesCompact: (blades) => PerformanceRenderer.renderBladesCompact(blades),

    showPerfTab: (tab) => PerformanceRenderer.showPerfTab(tab),

    renderTurnHistory: () => PerformanceRenderer.renderTurnHistory(Rendering.getPhaseKey)
};

// Automatic rendering on state change
if (typeof window !== 'undefined') {
    State.on('change', () => Rendering.render());
}

// Image preloading — eagerly start downloads so cards show instantly
window.preloadAssets = (assets) => {
    if (!assets || !assets.length) return;
    assets.forEach(src => {
        if (!src || ImageLoader.loadedImages.has(src)) return;
        const img = new Image();
        img.onload = () => { ImageLoader.loadedImages.add(src); };
        img.onerror = () => {};
        img.src = src;
    });
};

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

