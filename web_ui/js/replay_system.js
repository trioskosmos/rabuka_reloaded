/**
 * Replay System Module
 * Handles loading, playback, and navigation of game replays.
 */
import { State, updateStateData } from './state.js';
import { log } from './logger.js';
import { fixImg } from './constants.js';
import { DOM_IDS, DISPLAY_VALUES } from './constants_dom.js';
import { ModalManager } from './utils/ModalManager.js';

let onRender = () => { console.warn("Replay: No render callback set"); };

export const Replay = {
    setRenderCallback: (cb) => { onRender = cb; },

    toggleReplayMode: () => {
        State.replayMode = !State.replayMode;
        const controls = document.getElementById(DOM_IDS.REPLAY_CONTROLS);
        if (controls) controls.style.display = State.replayMode ? DISPLAY_VALUES.FLEX : DISPLAY_VALUES.NONE;
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
        ModalManager.show(DOM_IDS.MODAL_PASTE_REPLAY);
        const input = document.getElementById('paste-replay-input');
        input.value = '';
        input.focus();
    },

    closePasteReplayModal: () => {
        ModalManager.hide(DOM_IDS.MODAL_PASTE_REPLAY);
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
        if (btn) btn.textContent = '|| Pause';
        State.playInterval = setInterval(Replay.replayNext, 500);
    },

    stopPlay: () => {
        if (State.playInterval) {
            clearInterval(State.playInterval);
            State.playInterval = null;
            const btn = document.getElementById('play-btn');
            if (btn) btn.textContent = '> Play';
        }
    }
};

/**
 * Export current game state with undo/redo history in minimal JSON format.
 * Generates a minimal state JSON suitable for board editing and sharing.
 */
export const GameExport = {
    exportCurrentGame: async () => {
        try {
            const { Network } = await import('./network.js');
            const data = await Network.exportGame();
            if (!data) {
                log('Failed to export game');
                return null;
            }
            
            // Create minimal export format
            const minimal = {
                timestamp: data.export_timestamp,
                mode: data.game_mode,
                state: data.current_state,
                history: data.history || [],
                history_index: data.history_index || 0,
            };
            
            log(`Game exported: ${Object.keys(minimal.state).length} state fields`);
            return minimal;
        } catch (e) {
            log(`Export error: ${e.message}`);
            return null;
        }
    },

    downloadGameAsJSON: async () => {
        const data = await GameExport.exportCurrentGame();
        if (!data) return;
        
        const json = JSON.stringify(data, null, 2);
        const blob = new Blob([json], { type: 'application/json' });
        const url = URL.createObjectURL(blob);
        const a = document.createElement('a');
        a.href = url;
        a.download = `game_export_${new Date().toISOString().slice(0, 19).replace(/:/g, '-')}.json`;
        document.body.appendChild(a);
        a.click();
        document.body.removeChild(a);
        URL.revokeObjectURL(url);
        
        log('Game exported to file');
    },

    copyGameToClipboard: async () => {
        const data = await GameExport.exportCurrentGame();
        if (!data) return;
        
        const json = JSON.stringify(data);
        try {
            await navigator.clipboard.writeText(json);
            log('Game copied to clipboard');
        } catch (e) {
            log(`Failed to copy: ${e.message}`);
        }
    },

    importGameFromPaste: async (jsonText) => {
        try {
            const data = JSON.parse(jsonText);
            if (!data.state) {
                throw new Error('Invalid export format: missing state');
            }
            
            const { Network } = await import('./network.js');
            const success = await Network.importGame(data);
            if (success) {
                log('Game imported successfully');
            } else {
                log('Failed to import game');
            }
            return success;
        } catch (e) {
            log(`Import error: ${e.message}`);
            return false;
        }
    },

    importGameFromFile: async (input) => {
        if (!input.files || !input.files[0]) return;
        const file = input.files[0];
        const reader = new FileReader();
        reader.onload = async (e) => {
            try {
                const success = await GameExport.importGameFromPaste(e.target.result);
            } catch(err) {
                log(`Error reading file: ${err.message}`);
            }
        };
        reader.readAsText(file);
        input.value = '';
    },
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
