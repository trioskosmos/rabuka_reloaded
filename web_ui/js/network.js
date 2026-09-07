import { State } from './state.js';
import { RoomManager } from './services/RoomManager.js';
import { PlannerService } from './services/PlannerService.js';
import { GameService } from './services/GameService.js';
import { DebugService } from './services/DebugService.js';

function getInjectedBackendUrl() {
    const meta = document.querySelector('meta[name="rabuka-backend-url"]');
    return meta ? meta.getAttribute('content') : null;
}

const injectedBackendUrl = getInjectedBackendUrl();
const isGitHubPages = typeof window !== 'undefined' && window.location.hostname.includes('github.io');
const BACKEND_URL = injectedBackendUrl || (isGitHubPages
    ? (window.RABUKA_BACKEND_URL || 'https://your-rabuka-server.onrender.com')
    : '');

function buildUrl(path) {
    const cleanPath = path.startsWith('/') ? path.slice(1) : path;
    return BACKEND_URL ? `${BACKEND_URL}/${cleanPath}` : path;
}

function buildSseUrl(roomId) {
    return BACKEND_URL ? `${BACKEND_URL}/api/events?room_id=${roomId}` : `/api/events?room_id=${roomId}`;
}

export function getBackendUrl() {
    return BACKEND_URL;
}

export function isCrossOrigin() {
    return isGitHubPages && BACKEND_URL.length > 0;
}

/**
 * Build the standard API request headers from the current session state.
 * Header names are case-insensitive per HTTP, so a single casing is used.
 */
export function apiHeaders() {
    return {
        'Content-Type': 'application/json',
        'X-Session-Token': State.sessionToken || '',
        'X-Room-Id': State.roomCode || ''
    };
}

/**
 * Centralized fetch wrapper for the backend API. Merges the standard headers
 * with any caller-supplied options so every service talks to the server the
 * same way. Returns the raw Response; callers decide how to read it.
 */
export function apiFetch(path, options = {}) {
    const { headers, ...rest } = options;
    return fetch(buildUrl(path), {
        ...rest,
        headers: { ...apiHeaders(), ...(headers || {}) }
    });
}

export function getSseUrl(roomId) {
    return buildSseUrl(roomId);
}

/**
 * Network Facade
 * Orchestrates calls between specialized services while providing a unified API for the UI.
 */
export const Network = {
    // Shared State & Utils
    getHeaders: () => apiHeaders(),

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
    startOffline: null,
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
