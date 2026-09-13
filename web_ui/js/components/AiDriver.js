import { State } from '../state.js';
import { apiFetch } from '../network.js';
import { GameService } from '../services/GameService.js';

const AI_ACTION_DELAY = 0;
const AI_IDLE_DELAY = 100;

function aiHeaders(token) {
    return {
        'Content-Type': 'application/json',
        'X-Session-Token': token || '',
        'X-Room-Id': State.roomCode || ''
    };
}

export const AiDriver = {
    _running: false,
    // Last frame version seen (via the cheap version endpoint or response
    // counters). Lets the idle loop skip the full state GET when nothing
    // moved — previously 10 full serializations/sec even on the human turn.
    _lastVersion: null,
    _haveState: false,
    // Full post-action state returned by execute-action in AI rooms.
    // The next decision chains off it with no extra GET.
    _pendingState: null,

    start() {
        if (this._running) return;
        this._running = true;
        this._lastVersion = null;
        this._haveState = false;
        this._pendingState = null;
        this._loop();
    },

    stop() {
        this._running = false;
    },

    async _loop() {
        while (this._running) {
            if (!State._aiSessionToken || !State.roomCode) {
                this.stop();
                return;
            }

            try {
                const acted = await this._step();
                if (!this._running) return;
                await this._delay(acted ? AI_ACTION_DELAY : AI_IDLE_DELAY);
            } catch (e) {
                console.error('[AI]', e);
                if (!this._running) return;
                await this._delay(0);
            }
        }
    },

    async _step() {
        // Reuse the full state the server returns for AI-room actions —
        // no GET needed after our own move.
        let state = this._pendingState;
        this._pendingState = null;

        if (!state) {
            // Cheap version gate: full state GET only when the frame moved.
            try {
                const vres = await apiFetch('api/game-state/version', {
                    headers: aiHeaders(State._aiSessionToken)
                });
                if (vres.ok) {
                    const vdata = await vres.json();
                    if (vdata.version !== undefined) {
                        if (this._haveState && vdata.version === this._lastVersion) {
                            return false;
                        }
                        this._lastVersion = vdata.version;
                    }
                }
            } catch (e) {
                // Version check failed — fall through to a full fetch.
            }

            const res = await apiFetch('api/game-state', {
                headers: aiHeaders(State._aiSessionToken)
            });
            if (!res.ok) return false;

            state = await res.json();
            if (state.frame_counter !== undefined) this._lastVersion = state.frame_counter;
            this._haveState = true;
        }

        if (state.game_over) { this.stop(); return false; }

        const actions = state.legal_actions;
        if (!actions || actions.length === 0) return false;

        const action = actions[Math.floor(Math.random() * actions.length)];
        const p = action.parameters || {};

        const sendRes = await apiFetch('api/execute-action', {
            method: 'POST',
            headers: aiHeaders(State._aiSessionToken),
            body: JSON.stringify({
                action_index: action.index ?? 0,
                action_type: action.action_type,
                card_id: p.card_id,
                card_index: p.card_index ?? p.card_indices?.[0],
                card_indices: p.card_indices,
                card_no: p.card_no,
                stage_area: p.stage_area,
                use_baton_touch: p.use_baton_touch
            })
        });

        if (!sendRes.ok) {
            console.warn('[AI] action rejected:', sendRes.status);
            return false;
        }

        // AI rooms return the full post-action state: chain the next
        // decision off it instead of re-fetching.
        try {
            const result = await sendRes.json();
            if (result && result.frame_counter !== undefined && result.player1) {
                this._pendingState = result;
                this._lastVersion = result.frame_counter;
                this._haveState = true;
            } else if (result && result.frame_id !== undefined) {
                this._lastVersion = result.frame_id;
            }
        } catch (e) {
            console.warn('[AI] action response unreadable:', e);
        }

        // The AI is driven by this same browser tab, so push its move to the
        // human view directly — no reason to wait for the poll loop or SSE.
        // fetchState coalesces overlapping calls internally.
        GameService.fetchState(null);

        return true;
    },

    _delay(ms) {
        return new Promise(r => setTimeout(r, ms));
    },

    think() {
        if (!State._aiSessionToken) {
            this.stop();
            return;
        }
        if (!this._running) this.start();
    }
};
