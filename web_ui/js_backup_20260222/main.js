import { ICON_DATA_URIs } from './assets_registry.js';
import { State } from './state.js';
import { Phase, fixImg } from './constants.js';
import { translations } from './translations_data.js';
import { Network } from './network.js';
import { Tooltips } from './ui_tooltips.js';
import { Rendering } from './ui_rendering.js';
import { DragDrop } from './ui_drag_drop.js';
import { Modals } from './ui_modals.js';
import { Replay } from './replay_system.js';
import { toggleSidebar, switchBoard } from './layout.js';

// Global UI object for layout/switching
window.UI = {
    toggleSidebar,
    switchBoard,
    showPerformanceForTurn: Modals.showPerformanceForTurn,
    showLastPerformance: Modals.showLastPerformance,
    closePerformanceModal: Modals.closePerformanceModal,
    showDiscardModal: Rendering.showDiscardModal,
    showPerfTab: Rendering.showPerfTab
};

// Network Aliases (for legacy compatibility if needed)
const fetchState = Network.fetchState;

// Polling Logic
let pollingTimeout = null;

function updatePolling() {
    if (pollingTimeout) clearTimeout(pollingTimeout);

    let heartbeat = 0;
    const poll = async () => {
        try {
            heartbeat++;

            // Phase 5: SYNC Check
            const syncEl = document.getElementById('debug-sync');
            if (syncEl) {
                const isSynced = (window.StateMaster === State);
                syncEl.textContent = isSynced ? 'OK' : 'MISMATCH';
                syncEl.style.color = isSynced ? '#00ff00' : '#ff0000';
            }

            // Phase 4: Update Debug Overlay
            const roomEl = document.getElementById('debug-room');
            if (roomEl) roomEl.textContent = String(State.roomCode) || 'NULL';
            const sessEl = document.getElementById('debug-session');
            if (sessEl) sessEl.textContent = State.sessionToken ? 'VALID' : 'MISSING';
            const viewEl = document.getElementById('debug-view');
            if (viewEl) viewEl.textContent = `P${State.perspectivePlayer + 1}`;
            const pollEl = document.getElementById('debug-poll');
            if (pollEl) pollEl.textContent = heartbeat;

            // Phase 6: Expanded Visibility
            const delayEl = document.getElementById('debug-delay');
            if (delayEl) delayEl.textContent = (State.offlineMode ? 'OFFLINE' : (State.replayMode ? 'REPLAY' : 'LIVE'));
            if (delayEl) delayEl.style.color = (State.offlineMode || State.replayMode) ? '#f1c40f' : '#00ff00';

            // Verbose logging for Phase 6
            console.log(`[Poll#${heartbeat}] room="${State.roomCode}" | offline=${State.offlineMode} | replay=${State.replayMode} | sync=${window.StateMaster === State}`);

            if (State.replayMode || State.offlineMode || !State.roomCode) {
                // Not in a game, keep checking slowly
                pollingTimeout = setTimeout(poll, 1000);
                return;
            }

            const isThinking = State.data && State.data.is_ai_thinking;
            // 250ms if thinking/live, 500ms if just live, 1000ms fallback
            let delay = 1000;
            if (isThinking) delay = 250;
            else if (State.isLiveWatchOn) delay = 500;

            await fetchState();
            pollingTimeout = setTimeout(poll, delay);
        } catch (e) {
            console.error("[Polling] Critical Error in Loop:", e);
            // Always reschedule even on error to prevent loop death
            pollingTimeout = setTimeout(poll, 2000);
        }
    };

    poll();
}

// --- Error Handling ---
window.onerror = function (msg, url, line, col, error) {
    console.error("[CRITICAL] Global Error Caught:", msg, "at", url, ":", line);
    const logEl = document.getElementById('rule-log');
    if (logEl) {
        const div = document.createElement('div');
        div.className = 'log-item error';
        div.innerHTML = `<span style="color:#ff5555;font-weight:bold;">[ERROR]</span> UI Crash: ${msg}`;
        logEl.prepend(div);
    }
    return false;
};

// --- Global Initialization ---
export async function initialize() {
    console.log("[Init] Starting Rabukasim UI Initialization...");


    try {
        // 0. Start Adaptive Polling (PROMOTED to first step)
        console.log("[Init] Starting polling (immediate)...");
        updatePolling();

        // 1. Register Callbacks & Initialize UI
        console.log("[Init] Registering callbacks...");
        Network.setRenderCallback(Rendering.render);
        Replay.setRenderCallback(Rendering.render);

        Network.setRoomUpdateCallback(() => {
            const roomCodeEl = document.getElementById('room-code-header');
            const roomDisplay = document.getElementById('room-display');
            if (roomCodeEl) roomCodeEl.textContent = State.roomCode || '---';
            if (roomDisplay) roomDisplay.style.display = State.roomCode ? 'flex' : 'none';
            Rendering.render();
        });

        // Legacy alias for ui_modals.js
        window.onRoomUpdate = () => Network.triggerRoomUpdate ? Network.triggerRoomUpdate() : Rendering.render();

        Network.setOpenDeckModalCallback(Modals.openDeckModal);
        Modals.updateLanguage();

        // 2. Initial State Check (Important for re-joining)
        console.log("[Init] Checking system status...");
        await Network.checkSystemStatus();

        console.log("[Init] Fetching initial state...");
        await Network.fetchState();

        // 3. Initialize Drag & Drop
        console.log("[Init] Wiring drag & drop...");
        DragDrop.init();

        // Start Periodic Health Check
        setInterval(() => Network.checkSystemStatus(), 10000);

        // 6. Apply Saved Zoom
        const savedScale = localStorage.getItem('lovelive_board_scale');
        if (savedScale) {
            Modals.updateBoardScale(savedScale);
        }

        // Initial render
        console.log("[Init] Final rendering...");
        window.render();

        // Show Lobby if not in a room and not in offline mode
        if (!State.roomCode && !State.offlineMode && !State.replayMode) {
            console.log("[Init] No room detected, showing lobby.");
            const roomModal = document.getElementById('room-modal');
            if (roomModal) roomModal.style.display = 'flex';
        }
    } catch (e) {
        console.error("[Init] Initialization Failed:", e);
        const roomModal = document.getElementById('room-modal');
        if (roomModal) roomModal.style.display = 'flex';
    }

    console.log("[Init] Done.");
}

// Start Initialization
initialize();

// --- Re-attach Globals (For legacy onclick handlers in index.html) ---
window.render = Rendering.render;
window.fetchState = Network.fetchState;

// Modal Actions
window.openDeckModal = Modals.openDeckModal;
window.closeDeckModal = Modals.closeDeckModal;
window.submitDeck = Modals.submitDeck;
window.loadTestDeck = Modals.loadTestDeck;
window.loadRandomDeck = Modals.loadRandomDeck;

window.openSetupModal = Modals.openSetupModal;
window.openGameSetup = Modals.openSetupModal;
window.closeSetupModal = Modals.closeSetupModal;
window.submitGameSetup = Modals.submitGameSetup;
window.onDeckSelectChange = Modals.onDeckSelectChange;

window.openSettingsModal = Modals.openSettingsModal;
window.closeSettingsModal = Modals.closeSettingsModal;
window.toggleLang = Modals.toggleLang;
window.toggleFriendlyAbilities = Modals.toggleFriendlyAbilities;
window.updateBoardScale = Modals.updateBoardScale;

window.showLastPerformance = Modals.showLastPerformance;
window.closePerformanceModal = Modals.closePerformanceModal;
window.dismissPerformanceModal = Modals.closePerformanceModal; // Alias for index.html

window.forceReset = () => {
    console.log("[Lobby] Force Reset triggered. Clearing all data...");
    localStorage.removeItem('lovelive_room_code');
    localStorage.removeItem('lovelive_sessions');
    localStorage.removeItem('lovelive_board_scale');
    // Clear all keys starting with lovelive_
    for (let i = 0; i < localStorage.length; i++) {
        const key = localStorage.key(i);
        if (key && key.startsWith('lovelive_')) {
            localStorage.removeItem(key);
            i--;
        }
    }
    window.location.reload();
};

window.openHelpModal = Modals.openHelpModal;
window.closeHelpModal = Modals.closeHelpModal;

window.openLobby = Modals.openLobby;
window.closeLobby = Modals.closeLobby;
window.showLastPerformance = Modals.showLastPerformance;
window.showPerformanceForTurn = Modals.showPerformanceForTurn;
window.closePerformanceModal = Modals.closePerformanceModal;
window.fetchPublicRooms = Network.fetchPublicRooms;

window.openReportModal = Modals.openReportModal;
window.closeReportModal = Modals.closeReportModal;
window.submitReport = Modals.submitReport;
window.downloadReport = Modals.downloadReport;

// Game Actions
window.sendAction = (actionId, target = null) => {
    Network.sendAction(actionId, target);
};
window.doAction = window.sendAction;

window.toggleHotseat = () => {
    State.hotseatMode = !State.hotseatMode;
    window.render();
};

window.toggleLiveWatch = () => {
    State.isLiveWatchOn = !State.isLiveWatchOn;
    updatePolling();
    window.render();
};

window.togglePerspective = () => {
    State.perspectivePlayer = 1 - State.perspectivePlayer;
    window.render();
};

window.setPerspective = (id) => {
    State.perspectivePlayer = parseInt(id);
    document.getElementById('perspective-modal').style.display = 'none';
    const btn = document.getElementById('switch-btn');
    if (btn) btn.textContent = `View: P${State.perspectivePlayer + 1}`;
    fetchState();
};

window.leaveRoom = () => Network.leaveRoom();
window.joinRoom = (code) => Network.joinRoom(code);
window.resetGame = () => Network.resetGame();
window.forceAdvance = () => Network.forceAction(-1);
window.changeAI = (m) => Network.changeAI(m);
window.forceAction = (id) => Network.forceAction(id);
window.execCode = (c) => Network.execCode(c);
window.startOffline = (u) => Network.startOffline(u);

// Layout Actions
window.toggleSidebar = toggleSidebar;

// Replay System Actions
window.toggleReplayMode = Replay.toggleReplayMode;
window.loadReplay = Replay.loadReplay;
window.loadReplayFromFile = Replay.loadReplayFromFile;
window.openPasteReplayModal = Replay.openPasteReplayModal;
window.closePasteReplayModal = Replay.closePasteReplayModal;
window.submitPasteReplay = Replay.submitPasteReplay;
window.jumpToFrame = Replay.jumpToFrame;
window.replayPrev = Replay.replayPrev;
window.replayNext = Replay.replayNext;
window.replayPrevTurn = Replay.replayPrevTurn;
window.replayNextTurn = Replay.replayNextTurn;
window.replayPrevPhase = Replay.replayPrevPhase;
window.replayNextPhase = Replay.replayNextPhase;
window.togglePlay = Replay.togglePlay;

// Legacy Aliases
window.startReplay = Replay.loadReplay;
window.stopReplay = Replay.toggleReplayMode;

// Utility exports
window.State = State;
window.Network = Network;
window.Tooltips = Tooltips;

console.log("Rabukasim modular main.js loaded and globals wired.");
