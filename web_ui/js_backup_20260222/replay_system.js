/**
 * Replay System Module
 * Handles loading, playback, and navigation of game replays.
 */
import { State, updateStateData } from './state.js';
import { log } from './logger.js';
import { fixImg } from './constants.js';

let onRender = () => { console.warn("Replay: No render callback set"); };

export const Replay = {
    setRenderCallback: (cb) => { onRender = cb; },

    toggleReplayMode: () => {
        State.replayMode = !State.replayMode;
        const controls = document.getElementById('replay-controls');
        if (controls) controls.style.display = State.replayMode ? 'flex' : 'none';
        if (!State.replayMode && State.playInterval) Replay.stopPlay();
    },

    loadReplay: async () => {
        const filename = (document.getElementById('replay-file')?.value || 'ai_match.json').trim();
        try {
            const res = await fetch(`api/replay/${filename}?t=${Date.now()}`);
            State.replayData = await res.json();
            State.currentFrame = 0;
            const totalEl = document.getElementById('total-frames');
            if (totalEl) totalEl.textContent = State.replayData.states.length;
            Replay.displayReplayFrame();
            log(`Loaded: Game ${State.replayData.game_id + 1}, Winner: ${State.replayData.winner}, Phase: ${State.replayData.states[0]?.phase}, Frames: ${State.replayData.states.length}`);
        } catch (e) {
            log('Failed to load replay: ' + e.message);
        }
    },

    loadReplayFromFile: (input) => {
        if (!input.files || !input.files[0]) return;
        const file = input.files[0];
        const reader = new FileReader();
        reader.onload = (e) => {
            try {
                const json = JSON.parse(e.target.result);
                if (!json.states || !Array.isArray(json.states)) {
                    throw new Error("Invalid replay format: missing 'states' array");
                }

                State.replayData = json;
                State.currentFrame = 0;

                const totalEl = document.getElementById('total-frames');
                if (totalEl) totalEl.textContent = State.replayData.states.length;

                Replay.displayReplayFrame();
                log(`Loaded File: ${file.name} (Phase: ${State.replayData.states[0]?.phase}, Frames: ${State.replayData.states.length})`);

                if (State.playInterval) Replay.stopPlay();
                if (!State.replayMode) Replay.toggleReplayMode();

            } catch (err) {
                log('Error reading replay file: ' + err.message);
                console.error(err);
                alert('Error loading replay: ' + err.message);
            }
        };
        reader.readAsText(file);
        input.value = '';
    },

    openPasteReplayModal: () => {
        document.getElementById('paste-replay-modal').style.display = 'flex';
        const input = document.getElementById('paste-replay-input');
        input.value = '';
        input.focus();
    },

    closePasteReplayModal: () => {
        document.getElementById('paste-replay-modal').style.display = 'none';
    },

    submitPasteReplay: () => {
        const text = document.getElementById('paste-replay-input').value;
        if (!text.trim()) return;
        try {
            const json = JSON.parse(text);
            if (!json.states || !Array.isArray(json.states)) throw new Error("Invalid replay format: missing 'states'");

            State.replayData = json;
            State.currentFrame = 0;
            const totalEl = document.getElementById('total-frames');
            if (totalEl) totalEl.textContent = State.replayData.states.length;

            Replay.displayReplayFrame();
            log(`Loaded from Clipboard(Phase: ${State.replayData.states[0]?.phase}, Frames: ${State.replayData.states.length})`);

            if (State.playInterval) Replay.stopPlay();
            if (!State.replayMode) Replay.toggleReplayMode();

            Replay.closePasteReplayModal();
        } catch (e) {
            alert("Invalid JSON: " + e.message);
        }
    },

    jumpToFrame: (val) => {
        const n = parseInt(val);
        if (State.replayData && !isNaN(n) && n >= 0 && n < State.replayData.states.length) {
            State.currentFrame = n;
            Replay.displayReplayFrame();
        }
    },

    displayReplayFrame: () => {
        if (!State.replayData || State.currentFrame >= State.replayData.states.length) return;
        const frame = State.replayData.states[State.currentFrame];
        const frameEl = document.getElementById('frame-num');
        if (frameEl) frameEl.textContent = State.currentFrame;

        const jumpInput = document.getElementById('jump-frame');
        if (jumpInput) jumpInput.value = State.currentFrame;

        if (frame.players && frame.players.length >= 2) {
            updateStateData(frame);
            onRender();
            log(`Frame ${State.currentFrame}: T${frame.turn} ${frame.phase} P${frame.current_player} (Act: ${frame.action_taken || 0})`);
        } else {
            // Fallback to minimal display
            const turnEl = document.getElementById('turn');
            if (turnEl) turnEl.textContent = frame.turn;
            const phaseEl = document.getElementById('phase');
            if (phaseEl) phaseEl.textContent = frame.phase_name || frame.phase;
            const scoreEl = document.getElementById('score');
            if (scoreEl) scoreEl.textContent = `${frame.p0_score} - ${frame.p1_score}`;
            log(`Frame ${State.currentFrame}: T${frame.turn} ${frame.phase_name} ${frame.p0_score} - ${frame.p1_score}`);
        }
    },

    replayPrev: () => {
        if (State.currentFrame > 0) { State.currentFrame--; Replay.displayReplayFrame(); }
    },

    replayNext: () => {
        if (State.replayData && State.currentFrame < State.replayData.states.length - 1) {
            State.currentFrame++; Replay.displayReplayFrame();
        } else if (State.playInterval) Replay.stopPlay();
    },

    replayPrevTurn: () => {
        if (!State.replayData || State.currentFrame <= 0) return;
        const currentTurn = State.replayData.states[State.currentFrame].turn;
        let i = State.currentFrame - 1;
        while (i > 0 && State.replayData.states[i].turn === currentTurn) i--;
        State.currentFrame = i;
        Replay.displayReplayFrame();
    },

    replayNextTurn: () => {
        if (!State.replayData || State.currentFrame >= State.replayData.states.length - 1) return;
        const currentTurn = State.replayData.states[State.currentFrame].turn;
        let i = State.currentFrame + 1;
        while (i < State.replayData.states.length && State.replayData.states[i].turn === currentTurn) i++;
        if (i < State.replayData.states.length) State.currentFrame = i;
        Replay.displayReplayFrame();
    },

    replayPrevPhase: () => {
        if (!State.replayData || State.currentFrame <= 0) return;
        const currentPhase = State.replayData.states[State.currentFrame].phase;
        const currentTurn = State.replayData.states[State.currentFrame].turn;
        let i = State.currentFrame - 1;
        while (i > 0 && State.replayData.states[i].phase === currentPhase && State.replayData.states[i].turn === currentTurn) i--;
        State.currentFrame = i;
        Replay.displayReplayFrame();
    },

    replayNextPhase: () => {
        if (!State.replayData || State.currentFrame >= State.replayData.states.length - 1) return;
        const currentPhase = State.replayData.states[State.currentFrame].phase;
        const currentTurn = State.replayData.states[State.currentFrame].turn;
        let i = State.currentFrame + 1;
        while (i < State.replayData.states.length && State.replayData.states[i].phase === currentPhase && State.replayData.states[i].turn === currentTurn) i++;
        if (i < State.replayData.states.length) State.currentFrame = i;
        Replay.displayReplayFrame();
    },

    togglePlay: () => {
        State.playInterval ? Replay.stopPlay() : Replay.startPlay();
    },

    startPlay: () => {
        const btn = document.getElementById('play-btn');
        if (btn) btn.textContent = '⏸ Pause';
        State.playInterval = setInterval(Replay.replayNext, 500);
    },

    stopPlay: () => {
        if (State.playInterval) {
            clearInterval(State.playInterval);
            State.playInterval = null;
            const btn = document.getElementById('play-btn');
            if (btn) btn.textContent = '▶ Play';
        }
    }
};

// Keyboard listener
if (typeof window !== 'undefined') {
    window.addEventListener('keydown', (e) => {
        if (!State.replayMode) return;
        if (document.activeElement.tagName === 'INPUT') return;

        if (e.key === 'ArrowLeft') {
            Replay.replayPrev();
        } else if (e.key === 'ArrowRight') {
            Replay.replayNext();
        }
    });
}
