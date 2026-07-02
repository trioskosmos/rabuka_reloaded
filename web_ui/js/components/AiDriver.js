import { State } from '../state.js';
import { Phase } from '../constants.js';

const ROCK_TYPES = ['RockChoice', 'rock_choice'];
const TURN_TYPES = ['ChooseFirstAttacker', 'choose_first_attacker'];

function rpsPhase(p) { return p === Phase.ROCK_PAPER_SCISSORS; }

// value mapping: 0=Rock, 1=Scissors, 2=Paper
function rpsWinner(p1, p2) {
    if (p1 == null || p2 == null || p1 === p2) return 0;
    if ((p1 === 0 && p2 === 1) || (p1 === 1 && p2 === 2) || (p1 === 2 && p2 === 0)) return 1;
    return 2;
}

export const AiDriver = {
    _running: false,
    _processing: false,

    start() {
        this._running = true;
    },

    stop() {
        this._running = false;
        this._processing = false;
    },

    isPve() {
        return State.data?.mode === 'pve';
    },

    think() {
        if (this._processing) return;
        if (!this.isPve()) {
            if (this._running) this.stop();
            return;
        }
        if (!this._running) this.start();

        this._processing = true;
        try {
            const state = State.data;
            if (!state || state.game_over) {
                this.stop();
                return;
            }

            const actions = state.legal_actions;
            if (!actions || actions.length === 0) return;

            // RPS phase: only auto-send P2's choice AFTER P1 has chosen.
            // In sandbox/pve the legal_actions are NOT player-specific —
            // if we send before P1, the server applies our action to P1 instead.
            if (rpsPhase(state.phase)) {
                if (state.player1_rps_choice != null && state.player2_rps_choice == null) {
                    const rock = this._findAction(actions, ROCK_TYPES);
                    if (rock) { this._do(rock); return; }
                }
                return;
            }

            // Turn choice: only auto-send if P2 (AI) won RPS.
            // If P1 won, let the human decide.
            const first = this._findAction(actions, TURN_TYPES);
            if (first) {
                const p1c = state.player1_rps_choice;
                const p2c = state.player2_rps_choice;
                const winner = rpsWinner(p1c, p2c);
                if (winner === 2) { this._do(first); return; }
                // P1 won (or draw) — stop here so the human sees the UI
                return;
            }

            if (state.pending_choice) {
                const pid = state.pending_choice.choice_player_id;
                if (pid === 'p2' || pid == null) {
                    const valid = actions.filter(a => !a.parameters?.disabled);
                    if (valid.length) { this._do(valid[Math.random() * valid.length | 0]); return; }
                }
            }

            const isAiTurn = state.active_player === 'player2' || state.active_player === 'p2' || state.active_player === '1' || state.active_player === 1;
            if (isAiTurn) {
                const valid = actions.filter(a => !a.parameters?.disabled);
                if (valid.length) { this._do(valid[Math.random() * valid.length | 0]); }
            }
        } finally {
            this._processing = false;
        }
    },

    _findAction(actions, types) {
        return actions.find(a => types.includes(a.action_type));
    },

    _do(action) {
        if (window.doAction) window.doAction(action);
    }
};
