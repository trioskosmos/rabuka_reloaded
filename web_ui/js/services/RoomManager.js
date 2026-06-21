import { State, updateStateData } from '../state.js';
import { log } from '../logger.js';
import { ModalManager } from '../utils/ModalManager.js';
import { DOMUtils } from '../utils/DOMUtils.js';
import { DOM_IDS } from '../constants_dom.js';
import { SSEClient } from './SSEClient.js';
import { GameService } from './GameService.js';

export const RoomManager = {
    // Session Management
    normalizeSession: (sessionData) => {
        if (!sessionData) return null;
        if (typeof sessionData === 'string') {
            return { token: sessionData };
        }
        if (typeof sessionData === 'object') {
            return {
                token: sessionData.token || sessionData.session_id || sessionData.session || null,
                playerId: sessionData.playerId ?? sessionData.player_id ?? sessionData.player_idx
            };
        }
        return null;
    },

    saveSession: (room, sessionData) => {
        if (!room) return;
        const normalized = RoomManager.normalizeSession(sessionData);
        if (!normalized?.token) return;
        const key = `lovelive_session_${room}`;
        localStorage.setItem(key, JSON.stringify(normalized));
        State.sessionToken = normalized.token;
        if (normalized.playerId !== undefined) State.updateUiConfig({ perspective_player: normalized.playerId });
    },

    loadSession: (room) => {
        if (!room) return null;
        const key = `lovelive_session_${room}`;
        const saved = localStorage.getItem(key);
        if (saved) {
            try {
                const data = JSON.parse(saved);
                const normalized = RoomManager.normalizeSession(data);
                if (normalized?.token) {
                    State.sessionToken = normalized.token;
                    if (normalized.playerId !== undefined) State.updateUiConfig({ perspective_player: normalized.playerId });
                    return normalized;
                }
            } catch (e) {
                console.error("Failed to load session", e);
            }
        }
        return null;
    },

    // Room Management
    createRoom: async (mode = 'sandbox', networkFacade) => {
        try {
            const previousRoomCode = State.roomCode;
            State.resetForNewGame();

            const res = await fetch('api/rooms/create', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ mode: mode })
            });
            const data = await res.json();
            if (data.success) {
                State.roomCode = data.room_id;
                State.offlineMode = false;
                if (networkFacade?.clearPlannerData) networkFacade.clearPlannerData();
                localStorage.setItem('lovelive_room_code', State.roomCode);

                if (previousRoomCode && previousRoomCode !== State.roomCode) {
                    localStorage.removeItem(`lovelive_session_${previousRoomCode}`);
                }
                if (data.session) {
                    RoomManager.saveSession(State.roomCode, data.session);
                } else {
                    State.sessionToken = null;
                    localStorage.removeItem(`lovelive_session_${State.roomCode}`);
                }

                const waitingHint = document.getElementById('room-waiting-hint');
                if (waitingHint && mode === 'pvp') {
                    waitingHint.textContent = `Waiting for an opponent. Room code: ${State.roomCode}`;
                }

                if (mode !== 'pvp') {
                    ModalManager.hide(DOM_IDS.MODAL_ROOM);
                }
                log(`Created Room: ${State.roomCode} (${mode})`);

                if (networkFacade?.fetchState) await networkFacade.fetchState();
                if (networkFacade?.triggerRoomUpdate) networkFacade.triggerRoomUpdate();

                if (mode === 'pvp') {
                    // Open deck selection for host
                    if (window.Modals?.openDeckSelectionForPvP) {
                        window.Modals.openDeckSelectionForPvP(0);
                    }
                    if (networkFacade?.fetchState) {
                        setTimeout(() => networkFacade.fetchState(), 1000);
                    }
                }

                RoomManager.connectSSE();
            } else {
                alert('Failed to create room: ' + data.error);
            }
        } catch (e) {
            console.error(e);
            alert('Network error creating room');
        }
    },

    joinRoom: async (code = null, networkFacade) => {
        if (!code) {
            const input = DOMUtils.getElement(DOM_IDS.ROOM_CODE_INPUT);
            if (input) code = input.value.toUpperCase();
        }
        if (!code || code.length < 3 || code.length > 6) {
            alert('Please enter a valid room code (4 letters).');
            return;
        }
        
        State.resetForNewGame();
        State.roomCode = code;
        State.offlineMode = false;
        if (networkFacade?.clearPlannerData) networkFacade.clearPlannerData();
        localStorage.setItem('lovelive_room_code', State.roomCode);

        RoomManager.loadSession(code);

        try {
            const res = await fetch('api/rooms/join', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ room_id: code })
            });
            const data = await res.json();
            if (data.success) {
                State.cardSet = data.card_set || 'compiled';
                if (data.session) {
                    RoomManager.saveSession(code, data.session);
                }

                // Open deck selection for joiner
                const joinerPid = data.session?.player_id !== undefined ? data.session.player_id : 1;
                if (window.Modals?.openDeckSelectionForPvP) {
                    await window.Modals.fetchAndPopulateDecks();
                    window.Modals.openDeckSelectionForPvP(joinerPid);
                }
            }
        } catch (e) {
            console.error("Join API error", e);
        }

        ModalManager.hide(DOM_IDS.MODAL_ROOM);
        log(`Joining Room: ${State.roomCode}...`);

        if (networkFacade?.triggerRoomUpdate) networkFacade.triggerRoomUpdate();
        if (networkFacade?.fetchState) await networkFacade.fetchState();

        GameService.startGameplayPolling();
        RoomManager.connectSSE();
    },

    connectSSE: () => {
        if (!State.roomCode) return;
        SSEClient.connect(State.roomCode, () => {
            GameService.triggerVersionCheck();
        });
    },

    leaveRoom: async (networkFacade) => {
        try {
            await fetch('api/rooms/leave', {
                method: 'POST',
                headers: {
                    'Content-Type': 'application/json',
                    'X-Session-Token': State.sessionToken || ''
                },
                body: JSON.stringify({
                    room_id: State.roomCode,
                    session_id: State.sessionToken
                })
            });
        } catch (e) {
            console.warn('Failed to notify server of leaving room:', e);
        }

        SSEClient.disconnect();
        // Clear polling intervals
        if (window._pvpPollInterval) {
            clearInterval(window._pvpPollInterval);
            window._pvpPollInterval = null;
        }
        if (window._gameplayPollInterval) {
            clearInterval(window._gameplayPollInterval);
            window._gameplayPollInterval = null;
        }
        State.resetForNewGame();
        State.roomCode = null;
        State.sessionToken = null;
        updateStateData(null);
        
        localStorage.removeItem('lovelive_room_code');
        ModalManager.show(DOM_IDS.MODAL_ROOM);
        ModalManager.hide('performance-modal');
        ModalManager.hide(DOM_IDS.MODAL_SETUP);
        history.pushState({}, document.title, window.location.pathname);
        if (networkFacade?.triggerRoomUpdate) networkFacade.triggerRoomUpdate();
    },

    fetchPublicRooms: async () => {
        const list = DOMUtils.getElement(DOM_IDS.PUBLIC_ROOMS_LIST);
        if (!list) return;

        DOMUtils.setHTML(DOM_IDS.PUBLIC_ROOMS_LIST, '<div style="color:#666;text-align:center;padding-top:20px;">Loading...</div>');

        try {
            const res = await fetch('api/rooms/list');
            const data = await res.json();

            if (!data.rooms || data.rooms.length === 0) {
                DOMUtils.setHTML(DOM_IDS.PUBLIC_ROOMS_LIST, '<div style="color:#666;text-align:center;padding-top:20px;">No active public rooms.</div>');
                return;
            }

            DOMUtils.clear(DOM_IDS.PUBLIC_ROOMS_LIST);
            data.rooms.forEach(r => {
                const roomId = r.room_id || r.id;
                const players = r.players ?? r.players_count ?? 0;
                const div = document.createElement('div');
                div.className = 'public-room-item';
                div.style.padding = '5px';
                div.style.borderBottom = '1px solid #444';
                div.style.cursor = 'pointer';
                div.style.display = 'flex';
                div.style.justifyContent = 'space-between';
                div.innerHTML = `
                    <span>Room <b>${roomId}</b> (${r.mode})</span>
                    <span style="color:#aaa; font-size:0.8rem;">${players}/2</span>
                `;
                div.onclick = () => {
                    const input = DOMUtils.getElement(DOM_IDS.ROOM_CODE_INPUT);
                    if (input) input.value = roomId;
                    // Note: joinRoom needs the network facade, so we'll let the UI call the facade version
                    if (window.Network && window.Network.joinRoom) {
                        window.Network.joinRoom(roomId);
                    }
                };
                list.appendChild(div);
            });
        } catch (e) {
            DOMUtils.setHTML(DOM_IDS.PUBLIC_ROOMS_LIST, '<div style="color:#e74c3c;text-align:center;padding-top:20px;">Failed to load rooms.</div>');
        }
    }
};
