import { State } from '../state.js';

const AI_ACTION_DELAY = 0;
const AI_IDLE_DELAY = 0;

function aiHeaders(token) {
    return {
        'Content-Type': 'application/json',
        'X-Session-Token': token || '',
        'X-Room-Id': State.roomCode || ''
    };
}

export const AiDriver = {
    _running: false,

    start() {
        if (this._running) return;
        this._running = true;
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
        const res = await fetch('api/game-state', {
            headers: aiHeaders(State._aiSessionToken)
        });
        if (!res.ok) return false;

        const state = await res.json();
        if (state.game_over) { this.stop(); return false; }

        const actions = state.legal_actions;
        if (!actions || actions.length === 0) return false;

        const action = actions[Math.floor(Math.random() * actions.length)];
        const p = action.parameters || {};

        const sendRes = await fetch('api/execute-action', {
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
