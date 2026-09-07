import { State } from './state.js';
import { RoomManager } from './services/RoomManager.js';
import { PlannerService } from './services/PlannerService.js';
import { GameService } from './services/GameService.js';
import { DebugService } from './services/DebugService.js';

function getInjectedBackendUrl() {
    const meta = document.querySelector('meta[name="rabuka-backend-url"]');
    return meta ? meta.getAttribute('content') : null;
}

// Wait for DOM to be ready before reading meta tag
function waitForMetaTag() {
    return new Promise(resolve => {
        if (document.readyState === 'loading') {
            document.addEventListener('DOMContentLoaded', () => resolve(getInjectedBackendUrl()));
        } else {
            resolve(getInjectedBackendUrl());
        }
    });
}

// Initialize backend URL asynchronously
let BACKEND_URL_PROMISE = waitForMetaTag().then(injectedBackendUrl => {
    const isGitHubPages = typeof window !== 'undefined' && window.location.hostname.includes('github.io');
    const url = injectedBackendUrl || (isGitHubPages
        ? (window.RABUKA_BACKEND_URL || 'https://rabuka-server.onrender.com')
        : '');
    console.log('[Network] Resolved backend URL:', url);
    return url;
});

// Synchronous getter that blocks until backend URL is ready
let _cachedBackendUrl = null;
export async function getBackendUrl() {
    if (_cachedBackendUrl !== null) return _cachedBackendUrl;
    _cachedBackendUrl = await BACKEND_URL_PROMISE;
    console.log('[Network] Backend URL ready:', _cachedBackendUrl);
    return _cachedBackendUrl;
}

function buildUrl(path) {
    // This will be called after getBackendUrl() in apiFetch
    const cleanPath = path.startsWith('/') ? path.slice(1) : path;
    return _cachedBackendUrl ? `${_cachedBackendUrl}/${cleanPath}` : path;
}

function buildSseUrl(roomId) {
    return _cachedBackendUrl ? `${_cachedBackendUrl}/api/events?room_id=${roomId}` : `/api/events?room_id=${roomId}`;
}

export function isCrossOrigin() {
    return typeof window !== 'undefined' && window.location.hostname.includes('github.io') && _cachedBackendUrl?.length > 0;
}

export function getSseUrl(roomId) {
    console.log('[Network] getSseUrl:', roomId, '->', buildSseUrl(roomId));
    return buildSseUrl(roomId);
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
export async function apiFetch(path, options = {}) {
    // Ensure backend URL is resolved before making request
    await getBackendUrl();
    console.log('[Network] apiFetch:', path, '->', buildUrl(path));
    const { headers, ...rest } = options;
    return fetch(buildUrl(path), {
        ...rest,
        headers: { ...apiHeaders(), ...(headers || {}) }
    });
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
    apiFetch: apiFetch,

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