let eventSource = null;

export const SSEClient = {
    connect: (roomCode, onUpdate) => {
        if (eventSource) {
            eventSource.close();
        }
        eventSource = new EventSource(`/api/events?room_id=${roomCode}`);
        eventSource.onmessage = (e) => {
            console.log('[SSE] message:', e.data);
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
