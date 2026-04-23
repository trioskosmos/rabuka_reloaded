import { State } from './state.js';
import { Network } from './network.js';
import { DragDrop } from './ui_drag_drop.js';
import { Modals } from './ui_modals.js';
import { Rendering } from './ui_rendering.js';
import { Replay } from './replay_system.js';
import { closeSidebar, toggleSidebar, switchBoard } from './layout.js';
import { loadTranslations } from './i18n/index.js';
import { DOMUtils } from './utils/DOMUtils.js';
import { ModalManager } from './utils/ModalManager.js';
import { DebugModal } from './modals/DebugModal.js';
import { LogViewerModal } from './modals/LogViewerModal.js';
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
let pollingTimeout = null;
let healthCheckInterval = null;
let heartbeat = 0;
let isTabActive = true;
let currentDelay = POLL_DELAYS.idle;
let burstCounter = 0;

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
    if (State.replayMode || State.offlineMode || !State.roomCode) return POLL_DELAYS.idle;
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

function schedulePoll(delay) {
    if (pollingTimeout) clearTimeout(pollingTimeout);
    currentDelay = delay;
    pollingTimeout = window.setTimeout(AppController.pollOnce, delay);
}

function syncRoomDisplay() {
    DOMUtils.setText(DOM_IDS.ROOM_CODE_HEADER, State.roomCode || '---');
    DOMUtils.setVisible(DOM_IDS.ROOM_DISPLAY, Boolean(State.roomCode), DISPLAY_VALUES.FLEX);
}

const actionHandlers = {
    'toggle-sidebar': toggleSidebar,
    'close-sidebar': closeSidebar,
    'save-state': Modals.saveState,
    'load-state': Modals.loadState,
    'rewind': Modals.rewind,
    'redo': Modals.redo,
    'open-debug-modal': Modals.openDebugModal,
    'open-report-modal': Modals.openReportModal,
    'open-settings-modal': Modals.openSettingsModal,
    'close-settings-modal': Modals.closeSettingsModal,
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
    'toggle-hotseat': () => window.Actions.toggleHotseat(),
    'toggle-perspective': () => window.Actions.togglePerspective(),
    'toggle-live-watch': () => window.Actions.toggleLiveWatch(),
    'toggle-friendly-abilities': Modals.toggleFriendlyAbilities,
    'toggle-lang': Modals.toggleLang,
    'toggle-replay-mode': Replay.toggleReplayMode,
    'toggle-debug-mode': Modals.toggleDebugMode,
    'close-setup-modal': Modals.closeSetupModal,
    'submit-game-setup': Modals.submitGameSetup,
    'open-setup-modal': ({ value }) => Modals.openSetupModal(value),
    'join-room': () => Network.joinRoom(document.getElementById('room-code-input')?.value || ''),
    'start-offline': Network.startOffline,
    'force-reset': () => window.App.forceReset(),
    'set-perspective': ({ value }) => window.Actions.setPerspective(value),
    'close-log-viewer': LogViewerModal.close,
    'open-log-viewer': ({ value, event }) => { event.stopPropagation(); LogViewerModal.open(value); },
    'debug-rewind': DebugModal.rewind,
    'debug-redo': DebugModal.redo,
    'debug-render-all': DebugModal.renderAll,
    'close-debug-modal': DebugModal.closeDebugModal,
    'debug-switch-tab': ({ value }) => DebugModal.switchTab(value),
    'debug-copy-state-string': DebugModal.copyStateString,
    'debug-load-state-string': DebugModal.loadStateString,
    'debug-trigger-file-load': DebugModal.triggerFileLoad,
    'debug-render-minimal-json': DebugModal.renderMinimalJSON,
    'debug-render-checkpoint-json': DebugModal.renderCheckpointJSON,
    'debug-render-rich-json': DebugModal.renderRichJSON,
    'debug-copy-json-state': DebugModal.copyJsonState,
    'debug-load-json-file': DebugModal.loadJsonFile,
    'debug-apply-json-state': DebugModal.applyJsonState,
    'debug-export-game': DebugModal.exportGameWithHistory,
    'debug-import-game': DebugModal.importGameWithHistory,
    'show-performance-turn': ({ value }) => Modals.showPerformanceForTurn(Number(value)),
    'close-discard-modal': () => ModalManager.hide(DOM_IDS.MODAL_DISCARD),
    'reload-page': () => window.location.reload(),
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

        await loadTranslations(State.currentLang);
        // Load static card database for fallback card resolution
        State.loadStaticCardDatabase();
        AppController.restartPolling();
        
        const syncRoomState = () => syncRoomDisplay();
        State.on('roomUpdate', syncRoomState);
        State.on('room-change', syncRoomState);
        
        // Adaptive Polling: Listen for state changes to accelerate
        State.on('change-detected', () => {
            console.log("[App] State change detected! Accelerating polling...");
            burstCounter = 5; // Fast poll for the next few cycles to catch follow-ups
            schedulePoll(POLL_DELAYS.burst);
        });

        document.addEventListener('click', handleDelegatedClick);
        document.addEventListener('visibilitychange', () => {
            isTabActive = !document.hidden;
            if (isTabActive) AppController.pollOnce();
        });

        window.onRoomUpdate = () => { syncRoomDisplay(); Network.triggerRoomUpdate(); };
        Network.onOpenDeckModal = (playerIdx) => {
            if (playerIdx === State.perspectivePlayer) Modals.openDeckModal();
        };

        Modals.updateLanguage();
        syncRoomDisplay();
        await Network.checkSystemStatus();
        await Network.fetchState();
        DragDrop.init();

        if (!healthCheckInterval) {
            healthCheckInterval = window.setInterval(() => {
                if (isTabActive) Network.checkSystemStatus();
            }, POLL_DELAYS.healthCheck);
        }

        const savedScale = localStorage.getItem('lovelive_board_scale');
        if (savedScale) Modals.updateBoardScale(savedScale);
        // Simplified: Auto-start game instead of showing room modal
        if (!State.roomCode && !State.offlineMode && !State.replayMode && !State.gameHasStarted) {
            if (window.Actions && window.Actions.startGame) {
                window.Actions.startGame('pve');
            }
        }
    },

    restartPolling() {
        heartbeat = 0;
        schedulePoll(0);
    },

    async pollOnce() {
        if (pollingTimeout) { clearTimeout(pollingTimeout); pollingTimeout = null; }
        heartbeat += 1;

        try {
            updateDebugOverlay();
            const shouldFetch = isTabActive || (heartbeat % 20 === 0);

            if (shouldFetch && !State.replayMode && !State.offlineMode && (State.roomCode || State.gameHasStarted)) {
                await Network.fetchState();
            }

            // Adaptive backoff logic: Use burst if counter > 0, else slow target
            let nextDelay = getTargetPollDelay();
            if (isTabActive && burstCounter > 0) {
                burstCounter--;
                nextDelay = Math.min(nextDelay, POLL_DELAYS.burst);
            }
            
            schedulePoll(nextDelay);
        } catch (error) {
            console.error('[Polling] Error:', error);
            schedulePoll(POLL_DELAYS.error);
        }
    },
};
