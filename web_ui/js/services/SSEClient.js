import { getSseUrl, getBackendUrl } from '../network.js';

let eventSource = null;
let reconnectAttempt = 0;
let reconnectTimeout = null;
let currentRoomCode = null;
let currentOnUpdate = null;

function scheduleReconnect() {
    if (reconnectTimeout) clearTimeout(reconnectTimeout);
    const delay = Math.min(1000 * Math.pow(2, reconnectAttempt), 30000);
    console.log(`[SSE] Reconnecting in ${delay}ms (attempt ${reconnectAttempt + 1})`);
    reconnectTimeout = setTimeout(() => {
        reconnectAttempt++;
        SSEClient.connect(currentRoomCode, currentOnUpdate);
    }, delay);
}

export const SSEClient = {
    connect: async (roomCode, onUpdate) => {
        currentRoomCode = roomCode;
        currentOnUpdate = onUpdate;
        reconnectAttempt = 0;

        if (eventSource) {
            eventSource.close();
        }
        // Ensure backend URL is resolved before connecting SSE
        await getBackendUrl();
        eventSource = new EventSource(getSseUrl(roomCode));
        eventSource.onmessage = (e) => {
            console.log('[SSE] onmessage:', e.data);
            // Message format: "update <frame_id>" or "closed"
            if (e.data.startsWith('update')) {
                const parts = e.data.split(' ');
                const frameId = parts[1] ? parseInt(parts[1], 10) : null;
                if (onUpdate) onUpdate(frameId);
            } else if (e.data === 'closed' && onUpdate) {
                console.log('[SSE] room closed by opponent');
                if (window.handleRoomClosed) {
                    window.handleRoomClosed();
                }
            }
        };
        eventSource.onopen = () => {
            console.log('[SSE] connected to room', roomCode);
            reconnectAttempt = 0; // Reset on successful connection
        };
        eventSource.onerror = (err) => {
            console.error('[SSE] error:', err);
            if (eventSource.readyState === EventSource.CLOSED) {
                scheduleReconnect();
            }
        };
        return eventSource;
    },

    disconnect: () => {
        if (reconnectTimeout) clearTimeout(reconnectTimeout);
        reconnectTimeout = null;
        if (eventSource) {
            eventSource.close();
            eventSource = null;
        }
        currentRoomCode = null;
        currentOnUpdate = null;
    }
};
