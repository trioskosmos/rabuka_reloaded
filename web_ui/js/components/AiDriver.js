import { State, updateStateData } from '../state.js';
import { Phase } from '../constants.js';

const RPS_TYPES = ['RockChoice', 'rock_choice', 'PaperChoice', 'paper_choice', 'ScissorsChoice', 'scissors_choice'];
const TURN_TYPES = ['ChooseFirstAttacker', 'choose_first_attacker'];

function rpsPhase(p) { return p === Phase.ROCK_PAPER_SCISSORS; }

function rpsWinner(p1, p2) {
    if (p1 == null || p2 == null || p1 === p2) return 0;
    if ((p1 === 0 && p2 === 1) || (p1 === 1 && p2 === 2) || (p1 === 2 && p2 === 0)) return 1;
    return 2;
}

export const AiDriver = {
    _running: false,
    _processing: false,

    start() { this._running = true; },
    stop() { this._running = false; this._processing = false; },
    isPve() { return State.data?.mode === 'pve'; },

    async _send(action) {
        const token = State._aiSessionToken;
        if (!token || !State.roomCode) return;
        try {
            const res = await fetch('api/execute-action', {
                method: 'POST',
                headers: {
                    'Content-Type': 'application/json',
                    'X-Session-Token': token,
                    'X-Room-Id': State.roomCode
                },
                body: JSON.stringify({ action })
            });
            if (res.ok) {
                const data = await res.json();
                if (data) updateStateData(data);
            }
        } catch (_) {}
    },

    think() {
        if (this._processing) return;
        if (!this.isPve()) { if (this._running) this.stop(); return; }
        if (!this._running) this.start();
        if (!State._aiSessionToken) return;

        this._processing = true;
        try {
            const state = State.data;
            if (!state || state.game_over) { this.stop(); return; }

            const actions = state.legal_actions;
            if (!actions || actions.length === 0) return;

            // RPS: wait for P1 to choose, then send any remaining RPS action for P2
            if (rpsPhase(state.phase)) {
                if (state.player1_rps_choice != null && state.player2_rps_choice == null) {
                    const a = actions.find(a => RPS_TYPES.includes(a.action_type));
                    if (a) this._send(a);
                }
                return;
            }

            // Turn choice: auto-send if P2 won
            if (actions.some(a => TURN_TYPES.includes(a.action_type))) {
                const winner = rpsWinner(state.player1_rps_choice, state.player2_rps_choice);
                if (winner === 2) {
                    const a = actions.find(a => TURN_TYPES.includes(a.action_type));
                    if (a) this._send(a);
                }
                return;
            }

            // Pending choice for P2
            if (state.pending_choice) {
                const pid = state.pending_choice.choice_player_id;
                if (pid === 'p2' || pid == null) {
                    const valid = actions.filter(a => !a.parameters?.disabled);
                    if (valid.length) this._send(valid[Math.random() * valid.length | 0]);
                }
                return;
            }

            // General: act on P2's turn
            if (state.active_player === 'player2' || state.active_player === '1' || state.active_player === 1) {
                const valid = actions.filter(a => !a.parameters?.disabled);
                if (valid.length) this._send(valid[Math.random() * valid.length | 0]);
            }
        } finally {
            this._processing = false;
        }
    }
};
