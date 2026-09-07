import { getSseUrl, getBackendUrl } from '../network.js';

let eventSource = null;

export const SSEClient = {
    connect: async (roomCode, onUpdate) => {
        if (eventSource) {
            eventSource.close();
        }
        // Ensure backend URL is resolved before connecting SSE
        await getBackendUrl();
        eventSource = new EventSource(getSseUrl(roomCode));
        eventSource.onmessage = (e) => {
            console.log('[SSE] onmessage:', e.data);
            if (e.data === 'update' && onUpdate) {
                onUpdate();
            } else if (e.data === 'closed' && onUpdate) {
                console.log('[SSE] room closed by opponent');
                if (window.handleRoomClosed) {
                    window.handleRoomClosed();
                }
            }
        };
        eventSource.onopen = () => {
            console.log('[SSE] connected to room', roomCode);
        };
        eventSource.onerror = (err) => {
            console.error('[SSE] error:', err);
        };
        return eventSource;
    },

    disconnect: () => {
        if (eventSource) {
            eventSource.close();
            eventSource = null;
        }
    }
};
