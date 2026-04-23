import { State } from './state.js';
import { RoomManager } from './services/RoomManager.js';
import { PlannerService } from './services/PlannerService.js';
import { GameService } from './services/GameService.js';
import { DebugService } from './services/DebugService.js';

/**
 * Network Facade
 * Orchestrates calls between specialized services while providing a unified API for the UI.
 */
export const Network = {
    // Shared State & Utils
    getHeaders: () => {
        console.log(`[Network] getHeaders called. roomCode: ${State.roomCode}`);
        return {
            'Content-Type': 'application/json',
            'X-Session-Token': State.sessionToken || '',
            'X-Room-Id': State.roomCode || ''
        };
    },

    setRenderCallback: () => {
        // Legacy callback placeholder to prevent initialization errors in main.js
        // The modern UI uses reactive state updates via state.js
    },

    setOpenDeckModalCallback: () => {
        // Placeholder to prevent initialization error in main.js
    },

    // --- Room Management (Delegated to RoomManager) ---
    saveSession: (room, sessionData) => RoomManager.saveSession(room, sessionData),
    loadSession: (room) => RoomManager.loadSession(room),
    createRoom: (mode) => RoomManager.createRoom(mode, Network),
    joinRoom: (code) => RoomManager.joinRoom(code, Network),
    leaveRoom: () => RoomManager.leaveRoom(Network),
    fetchPublicRooms: () => RoomManager.fetchPublicRooms(),
    triggerRoomUpdate: () => {
        // This is caught by UI elements that react to room state
        State.emit('roomUpdate', { roomCode: State.roomCode });
    },

    // --- Planner Service (Delegated to PlannerService) ---
    clearPlannerData: () => PlannerService.clearPlannerData(),
    getPlannerFetchKey: () => PlannerService.getPlannerFetchKey(),
    shouldAutoFetchPlanner: () => PlannerService.shouldAutoFetchPlanner(),
    fetchPlannerData: (options) => PlannerService.fetchPlannerData(options, Network),

    // --- Core Game Service (Delegated to GameService) ---
    checkSystemStatus: () => GameService.checkSystemStatus(),
    fetchState: () => GameService.fetchState(Network),
    sendAction: (action) => GameService.sendAction(action, Network),
    resetGame: () => GameService.resetGame(Network),
    startOffline: (userInitiated) => GameService.startOffline(userInitiated, Network),
    changeAI: (aiMode) => GameService.changeAI(aiMode, Network),

    // --- Debug Service (Delegated to DebugService) ---
    submitReport: (explanation) => DebugService.submitReport(explanation),
    applyState: (jsonStr) => DebugService.applyState(jsonStr),
    boardOverride: (jsonStr) => DebugService.boardOverride(jsonStr),
    toggleDebugMode: () => DebugService.toggleDebugMode(),
    rewind: () => DebugService.rewind(Network),
    redo: () => DebugService.redo(Network),
    exportGame: () => DebugService.exportGame(),
    importGame: (data) => DebugService.importGame(data, Network),
    forceAction: (id) => DebugService.forceAction(id, Network),
    forcedTurnEnd: () => DebugService.forcedTurnEnd(Network),
    execCode: (code) => DebugService.execCode(code, Network),
    fetchDebugSnapshot: () => DebugService.fetchDebugSnapshot(),
    fetchStandardizedState: () => DebugService.fetchStandardizedState(),
    buildDownloadReport: (explanation) => DebugService.buildDownloadReport(explanation),

    // UI Callback hooks (can be overridden by Controller/UI)
    onOpenDeckModal: (playerIdx) => {
        // Default implementation or placeholder
        if (window.Controller && window.Controller.openDeckModal) {
            window.Controller.openDeckModal(playerIdx);
        }
    }
};

// Expose to window for legacy support and debug console
window.Network = Network;
