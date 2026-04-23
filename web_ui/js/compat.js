/**
 * compat.js - Single point of entry for legacy window global wiring
 * 
 * This module prevents multiple scripts from overwriting the same globals.
 * All window.* assignments happen here, reducing collisions and making APIs explicit.
 * 
 * Import this ONCE at the end of your main entry point after all modules are loaded.
 */

import { State } from './state.js';
import { Network } from './network.js';
import { Rendering } from './ui_rendering.js';
import { Tooltips } from './ui_tooltips.js';
import { DragDrop } from './ui_drag_drop.js';
import { Modals } from './ui_modals.js';
import { Replay } from './replay_system.js';
import { toggleSidebar, switchBoard } from './layout.js';
import { ModalManager } from './utils/ModalManager.js';
import { DOMUtils } from './utils/DOMUtils.js';
import { DOM_IDS } from './constants_dom.js';

/**
 * Wire all legacy global APIs to window.
 * Call this function once during app initialization.
 */
export function initializeGlobals(controller = {}) {
    if (window.__compatGlobalsInitialized) {
        return;
    }

    window.__compatGlobalsInitialized = true;

    // ============================================================
    // UI Object (for layout/board switching)
    // ============================================================
    window.UI = {
        toggleSidebar,
        switchBoard,
        showPerformanceForTurn: Modals.showPerformanceForTurn,
        showLastPerformance: Modals.showLastPerformance,
        closePerformanceModal: Modals.closePerformanceModal,
        showDiscardModal: Rendering.showDiscardModal,
        showZoneViewer: Rendering.showZoneViewer,
        showPerfTab: Rendering.showPerfTab,
        
        // Perspective-aware wrappers
        showMyZoneViewer: () => Rendering.showZoneViewer(State.perspectivePlayer),
        showOpponentZoneViewer: () => Rendering.showZoneViewer(1 - State.perspectivePlayer),
        showMyDiscard: () => Rendering.showDiscardModal(State.perspectivePlayer),
        showOpponentDiscard: () => Rendering.showDiscardModal(1 - State.perspectivePlayer)
    };

    // ============================================================
    // App Object (main rendering/fetching)
    // ============================================================
    window.App = {
        render: Rendering.render,
        fetchState: Network.fetchState,
        refreshTurnPlanner: () => Network.fetchPlannerData(),
        scoreTurnPlanner: () => Network.fetchPlannerData({ score: true }),
        forceReset: () => {
            console.log("[Lobby] Force Reset triggered. Clearing all data...");
            localStorage.removeItem('lovelive_room_code');
            localStorage.removeItem('lovelive_sessions');
            localStorage.removeItem('lovelive_board_scale');
            for (let i = 0; i < localStorage.length; i++) {
                const key = localStorage.key(i);
                if (key && key.startsWith('lovelive_')) {
                    localStorage.removeItem(key);
                    i--;
                }
            }
            window.location.reload();
        }
    };

    // ============================================================
    // Actions Object (game actions & controls)
    // ============================================================
    window.Actions = {
        sendAction: (actionId, target = null) => Network.sendAction(actionId, target),
        doAction: (actionId, target = null) => Network.sendAction(actionId, target),
        toggleHotseat: () => { State.hotseatMode = !State.hotseatMode; window.render(); },
        toggleLiveWatch: () => { State.isLiveWatchOn = !State.isLiveWatchOn; controller.restartPolling?.(); window.render(); },
        togglePerspective: () => { State.perspectivePlayer = 1 - State.perspectivePlayer; window.render(); },
        setPerspective: (id) => {
            State.perspectivePlayer = parseInt(id, 10);
            ModalManager.hide(DOM_IDS.MODAL_PERSPECTIVE);
            DOMUtils.setText(DOM_IDS.SWITCH_BTN, `View: P${State.perspectivePlayer + 1}`);
            Network.fetchState();
        },
        leaveRoom: () => Network.leaveRoom(),
        joinRoom: (code) => Network.joinRoom(code),
        resetGame: () => Network.resetGame(),
        forceAdvance: () => Network.forceAction(-1),
        changeAI: (m) => Network.changeAI(m),
        forceAction: (id) => Network.forceAction(id),
        execCode: (c) => Network.execCode(c),
        startOffline: (u) => Network.startOffline(u)
    };

    // ============================================================
    // Top-level function aliases (for HTML onclick handlers)
    // ============================================================
    window.render = window.App.render;
    window.fetchState = window.App.fetchState;
    window.refreshTurnPlanner = window.App.refreshTurnPlanner;
    window.scoreTurnPlanner = window.App.scoreTurnPlanner;
    window.forceReset = window.App.forceReset;
    window.fetchPublicRooms = Network.fetchPublicRooms;

    window.sendAction = (action) => window.Actions.sendAction(action);
    window.doAction = (action) => window.Actions.sendAction(action);
    window.toggleHotseat = window.Actions.toggleHotseat;
    window.toggleLiveWatch = window.Actions.toggleLiveWatch;
    window.togglePerspective = window.Actions.togglePerspective;
    window.setPerspective = window.Actions.setPerspective;
    window.leaveRoom = window.Actions.leaveRoom;
    window.joinRoom = window.Actions.joinRoom;
    window.resetGame = window.Actions.resetGame;
    window.forceAdvance = window.Actions.forceAdvance;
    window.changeAI = window.Actions.changeAI;
    window.forceAction = window.Actions.forceAction;
    window.execCode = window.Actions.execCode;
    window.startOffline = window.Actions.startOffline;

    // ============================================================
    // Modals Object & Methods
    // ============================================================
    window.Modals = Modals;

    window.openDeckModal = Modals.openDeckModal;
    window.closeDeckModal = Modals.closeDeckModal;
    window.submitDeck = Modals.submitDeck;
    window.loadTestDeck = Modals.loadTestDeck;
    window.loadRandomDeck = Modals.loadRandomDeck;
    window.openSetupModal = Modals.openSetupModal;
    window.closeSetupModal = Modals.closeSetupModal;
    window.submitGameSetup = Modals.submitGameSetup;
    window.onDeckSelectChange = Modals.onDeckSelectChange;
    window.openSettingsModal = Modals.openSettingsModal;
    window.closeSettingsModal = Modals.closeSettingsModal;
    window.toggleLang = Modals.toggleLang;
    window.toggleFriendlyAbilities = Modals.toggleFriendlyAbilities;
    window.updateBoardScale = Modals.updateBoardScale;
    window.openHelpModal = Modals.openHelpModal;
    window.closeHelpModal = Modals.closeHelpModal;
    window.openLobby = Modals.openLobby;
    window.closeLobby = Modals.closeLobby;
    window.openReportModal = Modals.openReportModal;
    window.closeReportModal = Modals.closeReportModal;
    window.submitReport = Modals.submitReport;
    window.downloadReport = Modals.downloadReport;
    window.showLastPerformance = Modals.showLastPerformance;
    window.showPerformanceForTurn = Modals.showPerformanceForTurn;
    window.closePerformanceModal = Modals.closePerformanceModal;
    window.dismissPerformanceModal = Modals.closePerformanceModal;

    // ============================================================
    // Replay System
    // ============================================================
    window.Replay = { ...Replay };
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
    window.startReplay = Replay.loadReplay;
    window.stopReplay = Replay.toggleReplayMode;

    // ============================================================
    // Utility Exports
    // ============================================================
    window.State = State;
    window.Network = Network;
    window.Rendering = Rendering;
    window.Tooltips = Tooltips;
    window.DragDrop = DragDrop;

    console.log("[Compat] All legacy globals wired successfully. No overwrites.");
}

export default { initializeGlobals };
