import { State } from './state.js';
import { Network } from './network.js';
import { DragDrop } from './ui_drag_drop.js';
import { Modals } from './ui_modals.js';
import { Rendering } from './ui_rendering.js';
import { Replay } from './replay_system.js';
import { closeSidebar, toggleLogSidebar, toggleActionsSidebar, switchBoard } from './layout.js';
import { loadTranslations } from './i18n/index.js';
import { DOMUtils } from './utils/DOMUtils.js';
import { ModalManager } from './utils/ModalManager.js';
import { DebugModal } from './modals/DebugModal.js';
import { LogViewerModal } from './modals/LogViewerModal.js';
import { LogDetailModal } from './modals/LogDetailModal.js';
import { GameStateModal } from './modals/GameStateModal.js';
import { DOM_IDS, COLORS, DISPLAY_VALUES } from './constants_dom.js';

const POLL_DELAYS = {
    idle: 3000,         // Normal slow polling
    thinking: 1500,     // Poll faster when AI is thinking
    liveWatch: 1200,    // Poll faster when watching live
    burst: 200,         // Immediate follow-up after change
    error: 5000,
    healthCheck: 30000,
};

let initialized = false;
let healthCheckInterval = null;
let heartbeat = 0;
let isTabActive = true;

const debugElements = {
    sync: null, room: null, session: null, view: null, poll: null, delay: null,
};

function initializeDebugElementCache() {
    if (debugElements.sync) return;
    debugElements.sync = DOMUtils.getElement(DOM_IDS.DEBUG_SYNC);
    debugElements.room = DOMUtils.getElement(DOM_IDS.DEBUG_ROOM);
    debugElements.session = DOMUtils.getElement(DOM_IDS.DEBUG_SESSION);
    debugElements.view = DOMUtils.getElement(DOM_IDS.DEBUG_VIEW);
    debugElements.poll = DOMUtils.getElement(DOM_IDS.DEBUG_POLL);
    debugElements.delay = DOMUtils.getElement(DOM_IDS.DEBUG_DELAY);
}

function getPollingMode() {
    if (!isTabActive) return 'SLEEP';
    if (State.offlineMode) return 'OFFLINE';
    if (State.replayMode) return 'REPLAY';
    return 'LIVE';
}

function getTargetPollDelay() {
    if (!isTabActive) return 10000;
    if (State.replayMode || State.offlineMode || (!State.roomCode && !State.gameHasStarted)) return POLL_DELAYS.idle;
    if (State.data?.is_ai_thinking) return POLL_DELAYS.thinking;
    if (State.isLiveWatchOn) return POLL_DELAYS.liveWatch;
    return POLL_DELAYS.idle;
}

function updateDebugOverlay() {
    initializeDebugElementCache();
    const isSynced = window.StateMaster === State;
    DOMUtils.updateText({
        [DOM_IDS.DEBUG_SYNC]: isSynced ? 'OK' : 'MISMATCH',
        [DOM_IDS.DEBUG_ROOM]: String(State.roomCode || 'NULL'),
        [DOM_IDS.DEBUG_SESSION]: State.sessionToken ? 'VALID' : 'MISSING',
        [DOM_IDS.DEBUG_VIEW]: `P${State.perspectivePlayer + 1}`,
        [DOM_IDS.DEBUG_POLL]: heartbeat,
        [DOM_IDS.DEBUG_DELAY]: `${getPollingMode()} (${currentDelay}ms)`,
    });
    if (debugElements.sync) debugElements.sync.style.color = isSynced ? '#00ff00' : COLORS.ACCENT_RED;
}

function syncRoomDisplay() {
    DOMUtils.setText(DOM_IDS.ROOM_CODE_HEADER, State.roomCode || '---');
    DOMUtils.setVisible(DOM_IDS.ROOM_DISPLAY, Boolean(State.roomCode), DISPLAY_VALUES.FLEX);
}

const actionHandlers = {
    'toggle-log-sidebar': toggleLogSidebar,
    'toggle-actions-sidebar': toggleActionsSidebar,
    'close-sidebar': closeSidebar,
    'save-state': Modals.saveState,
    'load-state': Modals.loadState,
    'rewind': Modals.rewind,
    'redo': Modals.redo,
    'open-debug-modal': Modals.openDebugModal,
    'open-report-modal': Modals.openReportModal,

    'leave-room': Network.leaveRoom,
    'click-target': ({ targetId }) => document.getElementById(targetId)?.click(),
    'open-paste-replay-modal': Replay.openPasteReplayModal,
    'close-paste-replay-modal': Replay.closePasteReplayModal,
    'submit-paste-replay': Replay.submitPasteReplay,
    'load-replay': Replay.loadReplay,
    'replay-prev-turn': Replay.replayPrevTurn,
    'replay-prev-phase': Replay.replayPrevPhase,
    'replay-prev': Replay.replayPrev,
    'toggle-play': Replay.togglePlay,
    'replay-next': Replay.replayNext,
    'replay-next-phase': Replay.replayNextPhase,
    'replay-next-turn': Replay.replayNextTurn,
    'switch-board': ({ value }) => switchBoard(value),
    'show-zone-viewer': ({ owner }) => Rendering.showZoneViewer(owner === 'opponent' ? 1 - State.perspectivePlayer : State.perspectivePlayer),
    'show-discard': ({ owner }) => Rendering.showDiscardModal(owner === 'opponent' ? 1 - State.perspectivePlayer : State.perspectivePlayer),
    'show-last-performance': Modals.showLastPerformance,
    'close-performance-modal': Modals.closePerformanceModal,
    'show-performance-tab': ({ value }) => Rendering.showPerfTab(value),
    'close-selection-modal': () => ModalManager.hide(DOM_IDS.SELECTION_MODAL),
    'close-report-modal': Modals.closeReportModal,
    'download-report': Modals.downloadReport,
    'submit-report': Modals.submitReport,
    'open-help-modal': Modals.openHelpModal,
    'close-help-modal': Modals.closeHelpModal,
    'fetch-state': Network.fetchState,
    'reset-game': Network.resetGame,
    'navigate': ({ href }) => { if (href) window.location.href = href; },
    'open-deck-modal': Modals.openDeckModal,
    'close-deck-modal': Modals.closeDeckModal,
    'submit-deck': Modals.submitDeck,
    'load-test-deck': Modals.loadTestDeck,
    'load-random-deck': Modals.loadRandomDeck,

    'toggle-perspective': () => window.Actions.togglePerspective(),
    'toggle-lang': Modals.toggleLang,
    'close-setup-modal': Modals.closeSetupModal,
    'submit-game-setup': Modals.submitGameSetup,
    'open-setup-modal': ({ value }) => Modals.openSetupModal(value),
    'create-room': () => Network.createRoom('pvp'),
    'join-room': () => Network.joinRoom(document.getElementById('room-code-input')?.value || ''),
    'start-offline': () => { console.warn('Offline mode removed. Use Rust backend via Express proxy.'); },
    'force-reset': () => window.App.forceReset(),
    'set-perspective': ({ value }) => window.Actions.setPerspective(value),
    'close-log-viewer': LogViewerModal.close,
    'open-log-viewer': ({ value, event }) => { event.stopPropagation(); LogViewerModal.open(value); },
    'debug-rewind': DebugModal.rewind,
    'debug-redo': DebugModal.redo,
    'debug-render-all': DebugModal.renderAll,
    'close-debug-modal': DebugModal.closeDebugModal,
    'show-performance-turn': ({ value }) => Modals.showPerformanceForTurn(Number(value)),
    'open-game-state-modal': () => GameStateModal.open(),
    'close-game-state-modal': () => GameStateModal.close(),
    'switch-game-state-tab': ({ value }) => GameStateModal.showTab(value),
    'close-discard-modal': () => ModalManager.hide(DOM_IDS.MODAL_DISCARD),
    'close-revealed-modal': () => ModalManager.hide(DOM_IDS.MODAL_REVEALED),
    'close-log-detail-modal': LogDetailModal.close,
    'reload-page': () => window.location.reload(),
    'cheat-add-energy': ({ player }) => {
        const amount = document.getElementById('cheat-energy-amount')?.value || '1';
        window.Actions.execCode(`player_idx=${player}; amount=${amount}; draw_energy`);
    },
    'cheat-max-energy': () => {
        window.Actions.execCode(`player_idx=0; amount=15; draw_energy`);
        window.Actions.execCode(`player_idx=1; amount=15; draw_energy`);
    },
    'cheat-add-card': ({ player }) => {
        const cardId = document.getElementById('cheat-card-id')?.value || '';
        if (!cardId) { alert('Enter a card ID'); return; }
        window.Actions.execCode(`player_idx=${player}; card_no=${cardId}; add_card`);
    },
    'cheat-draw-card': ({ player }) => {
        const amount = document.getElementById('cheat-draw-amount')?.value || '1';
        window.Actions.execCode(`player_idx=${player}; amount=${amount}; draw_card`);
    },
    'cheat-clear-hand': ({ player }) => {
        window.Actions.execCode(`player_idx=${player}; clear_hand`);
    },
    'cheat-add-stage': ({ player }) => {
        const cardId = document.getElementById('cheat-stage-id')?.value || '';
        if (!cardId) { alert('Enter a card ID'); return; }
        window.Actions.execCode(`player_idx=${player}; card_no=${cardId}; add_stage`);
    },
    'cheat-add-live': ({ player }) => {
        const cardId = document.getElementById('cheat-live-id')?.value || '';
        if (!cardId) { alert('Enter a card ID'); return; }
        window.Actions.execCode(`player_idx=${player}; card_no=${cardId}; add_live_to_zone`);
    },
    'cheat-force-win': () => {
        window.Actions.execCode(`force_win`);
    },
    'cheat-reshuffle': ({ player }) => {
        window.Actions.execCode(`player_idx=${player}; reshuffle_deck`);
    },
    'cheat-negative-energy': ({ player }) => {
        window.Actions.execCode(`player_idx=${player}; negative_energy`);
    },
    'cheat-to-success': ({ player }) => {
        const cardId = document.getElementById('cheat-util-id')?.value || '';
        if (!cardId) { alert('Enter a card ID'); return; }
        window.Actions.execCode(`player_idx=${player}; card_no=${cardId}; to_success`);
    },
    'cheat-to-discard': ({ player }) => {
        const cardId = document.getElementById('cheat-util-id')?.value || '';
        if (!cardId) { alert('Enter a card ID'); return; }
        window.Actions.execCode(`player_idx=${player}; card_no=${cardId}; to_discard`);
    },
};

function handleDelegatedClick(event) {
    const button = event.target.closest('[data-action]');
    if (!button) return;
    const action = button.getAttribute('data-action');
    if (action === 'send-action') {
        window.sendAction(button.getAttribute('data-id')); return;
    }
    if (action === 'close-modal') {
        const modal = button.closest('.modal') || button.closest('.modal-overlay');
        if (modal) ModalManager.hideElement(modal); return;
    }
    const handler = actionHandlers[action];
    if (handler) {
        const params = {
            button, event,
            id: button.getAttribute('data-id'),
            value: button.getAttribute('data-value'),
            owner: button.getAttribute('data-owner'),
            targetId: button.getAttribute('data-target-id'),
            href: button.getAttribute('data-href'),
            player: button.getAttribute('data-player'),
        };
        handler(params);
    }
}

export const AppController = {
    async initialize() {
        if (initialized) return;
        initialized = true;

        window.onerror = (msg, url, line) => {
            console.error('[CRITICAL] Global Error:', msg, 'at', url, ':', line);
            const logEl = document.getElementById(DOM_IDS.CONTAINER_RULE_LOG);
            if (logEl) {
                const div = document.createElement('div');
                div.className = 'log-item error';
                div.innerHTML = `<span style="color:#ff5555;font-weight:bold;">[ERROR]</span> UI Crash: ${msg}`;
                logEl.prepend(div);
            }
            return false;
        };

        // Show lobby immediately so the user has something to interact with
        // while slower async init (card DB, translations, health checks) completes.
        if (!State.replayMode) {
            Modals.openLobby();
        }

        await Promise.all([
            loadTranslations(State.currentLang),
            State.loadStaticCardDatabase()
        ]);
        
        const syncRoomState = () => syncRoomDisplay();
        State.on('roomUpdate', syncRoomState);
        State.on('room-change', syncRoomState);
        
        // Adaptive Polling: Listen for state changes to accelerate
        document.addEventListener('click', handleDelegatedClick);
        document.addEventListener('visibilitychange', () => {
            isTabActive = !document.hidden;
        });

        // Clear card selection when clicking outside cards
        document.addEventListener('click', (e) => {
            if (window.selectedAction && !e.target.closest('.card, .member-slot, .member-area')) {
                window.selectedAction = null;
                document.querySelectorAll('.card.selected').forEach(c => c.classList.remove('selected'));
                if (window.highlightActionBtn) window.highlightActionBtn(null, false);
            }
        });

        // Custom events for log entry clicks
        document.addEventListener('opencode:show-performance', (e) => {
            const { turn } = e.detail;
            if (turn > 0 && State.performanceHistory && State.performanceHistory[turn]) {
                Modals.showPerformanceForTurn(turn);
            } else {
                Modals.showLastPerformance();
            }
        });
        document.addEventListener('opencode:show-log-detail', (e) => {
            LogDetailModal.open(e.detail.entryType, e.detail.body, e.detail.groupId);
        });

        window.onRoomUpdate = () => { syncRoomDisplay(); Network.triggerRoomUpdate(); };
        Network.onOpenDeckModal = (playerIdx) => {
            if (playerIdx === State.perspectivePlayer) Modals.openDeckModal();
        };

        Modals.updateLanguage();
        syncRoomDisplay();
        await Network.checkSystemStatus();

        DragDrop.init();
        LogDetailModal.init();

        if (!healthCheckInterval) {
            healthCheckInterval = window.setInterval(() => {
                if (isTabActive) Network.checkSystemStatus();
            }, POLL_DELAYS.healthCheck);
            window.addEventListener('beforeunload', () => {
                clearInterval(healthCheckInterval);
                healthCheckInterval = null;
                // Use sendBeacon to notify server on tab close (works during unload)
                if (State.roomCode && State.sessionToken) {
                    const payload = JSON.stringify({
                        room_id: State.roomCode,
                        session_id: State.sessionToken
                    });
                    navigator.sendBeacon('api/rooms/leave', payload);
                }
            });
        }

        // Global handler for when the other player leaves the room
        window._roomClosedHandled = false;
        window.handleRoomClosed = () => {
            if (window._roomClosedHandled) return;
            window._roomClosedHandled = true;
            if (window._pvpPollInterval) {
                clearInterval(window._pvpPollInterval);
                window._pvpPollInterval = null;
            }
            alert('Opponent has left the game. Returning to lobby.');
            if (window.Network?.leaveRoom) {
                window.Network.leaveRoom();
            }
        };

        const savedScale = localStorage.getItem('lovelive_board_scale');
        if (savedScale) Modals.updateBoardScale(savedScale);
    },

    restartPolling() {
        heartbeat = 0;
        Network.fetchState();
    },
};
