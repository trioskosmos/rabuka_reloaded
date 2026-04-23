import { State } from '../state.js';
import { log } from '../logger.js';

export const DebugService = {
    _buildSlimReport: (explanation) => {
        const raw = State.data;
        if (!raw) return { explanation, timestamp: new Date().toISOString(), error: "No game state available" };

        const slimCard = (c) => {
            if (!c) return null;
            const out = { ...c };
            delete out.img;
            delete out.valid_actions;
            delete out.is_new;
            if (out.description && out.desc) delete out.description;
            for (const key of ['hearts', 'blade_hearts', 'required_hearts', 'filled', 'required']) {
                if (Array.isArray(out[key])) {
                    let last = out[key].length - 1;
                    while (last > 0 && out[key][last] === 0) last--;
                    out[key] = out[key].slice(0, last + 1);
                    if (out[key].length === 1 && out[key][0] === 0) out[key] = [];
                }
            }
            return out;
        };

        const slimPlayer = (p) => {
            if (!p) return null;
            const out = {};
            [
                'player_id', 'score', 'is_active', 'deck_count', 'hand_count',
                'discard_count', 'energy_count', 'energy_untapped', 'energy_deck_count',
                'live_zone_count', 'total_blades', 'restrictions', 'flags',
                'activated_energy_group_mask', 'activated_member_group_mask',
                'cost_reduction', 'blade_buffs', 'heart_buffs', 'prevent_activate',
                'prevent_baton_touch', 'prevent_success_pile_set', 'played_group_mask',
                'yell_cards', 'heart_req_reductions', 'heart_req_additions'
            ].forEach(k => { if (p[k] !== undefined) out[k] = p[k]; });

            if (Array.isArray(p.hand)) {
                if (!p.hand.some(c => c && c.hidden)) out.hand = p.hand.map(slimCard).filter(Boolean);
            }
            if (Array.isArray(p.stage)) out.stage = p.stage.map(s => s ? slimCard(s) : null);
            if (Array.isArray(p.live_zone) && p.live_zone.some(l => l !== null)) out.live_zone = p.live_zone.map(l => l ? slimCard(l) : null);
            if (Array.isArray(p.success_lives) && p.success_lives.length > 0) out.success_lives = p.success_lives.map(slimCard);
            if (Array.isArray(p.discard) && p.discard.length > 0) out.discard = p.discard.map(slimCard);
            if (Array.isArray(p.energy)) {
                out.energy_summary = {
                    tapped: p.energy.filter(e => e && e.tapped).length,
                    untapped: p.energy.filter(e => e && !e.tapped).length
                };
            }
            return out;
        };

        const slimPerf = (perf) => {
            if (!perf) return null;
            const out = { ...perf };
            if (out.member_contributions) {
                out.member_contributions = out.member_contributions.map(mc => {
                    const s = { ...mc };
                    if (Array.isArray(s.hearts)) {
                        let last = s.hearts.length - 1;
                        while (last > 0 && s.hearts[last] === 0) last--;
                        s.hearts = s.hearts.slice(0, last + 1);
                        if (s.hearts.length === 1 && s.hearts[0] === 0) s.hearts = [];
                    }
                    return s;
                });
            }
            if (out.lives) out.lives = out.lives.map(slimCard);
            if (out.breakdown && out.breakdown.hearts) {
                out.breakdown.hearts = out.breakdown.hearts.map(h => {
                    const s = { ...h };
                    if (Array.isArray(s.value)) {
                        let last = s.value.length - 1;
                        while (last > 0 && s.value[last] === 0) last--;
                        s.value = s.value.slice(0, last + 1);
                        if (s.value.length === 1 && s.value[0] === 0) s.value = [];
                    }
                    return s;
                });
            }
            if (Array.isArray(out.total_hearts)) {
                let last = out.total_hearts.length - 1;
                while (last > 0 && out.total_hearts[last] === 0) last--;
                out.total_hearts = out.total_hearts.slice(0, last + 1);
                if (out.total_hearts.length === 1 && out.total_hearts[0] === 0) out.total_hearts = [];
            }
            return out;
        };

        const report = {
            timestamp: new Date().toISOString(),
            explanation: explanation || "",
            room_id: State.roomCode || null,
            mode: raw.mode || "unknown",
            turn: raw.turn,
            phase: raw.phase,
            active_player: raw.active_player,
            winner: raw.winner,
            game_over: raw.game_over,
            my_player_id: raw.my_player_id,
            pending_choice: raw.pending_choice,
            rule_log: raw.rule_log,
            bytecode_log: raw.bytecode_log,
            performance_history: (raw.performance_history || []).map(slimPerf),
            action_history: State.actionHistory || []
        };

        if (Array.isArray(raw.legal_actions)) {
            report.legal_actions = raw.legal_actions.map(a => ({
                id: a.id, name: a.name, type: a.type, hand_idx: a.hand_idx,
                area_idx: a.area_idx, slot_idx: a.slot_idx, cost: a.cost, cost_label: a.cost_label
            }));
        }
        if (Array.isArray(raw.players)) report.players = raw.players.map(slimPlayer);

        return report;
    },

    fetchStandardizedState: async () => {
        const roomCode = localStorage.getItem("lovelive_room_code");
        if (!roomCode) return null;
        try {
            const res = await fetch('api/debug/dump_state', { headers: { 'X-Room-Id': roomCode } });
            return res.ok ? await res.json() : null;
        } catch (e) { return null; }
    },

    submitReport: async (explanation) => {
        let reportData = await DebugService.fetchStandardizedState();
        if (reportData) {
            reportData.explanation = explanation;
            reportData.userAgent = navigator.userAgent;
        } else {
            reportData = DebugService._buildSlimReport(explanation);
        }

        try {
            const res = await fetch('api/report', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify(reportData)
            });
            return res.ok;
        } catch (e) { return false; }
    },

    applyState: async (jsonStr) => {
        const roomCode = localStorage.getItem("lovelive_room_code");
        if (!roomCode) return { ok: false, error: 'No room code' };
        try {
            const res = await fetch('/api/debug/apply_state', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json', 'X-Room-Id': roomCode },
                body: jsonStr
            });
            const data = await res.json().catch(() => null);
            return { ok: res.ok, error: data?.error || null, data };
        } catch (e) { return { ok: false, error: e.message }; }
    },

    fetchDebugSnapshot: async () => {
        const roomCode = localStorage.getItem("lovelive_room_code");
        if (!roomCode) return null;
        try {
            const res = await fetch('/api/debug/snapshot', { headers: { 'X-Room-Id': roomCode } });
            return res.ok ? await res.json() : null;
        } catch (e) { return null; }
    },

    boardOverride: async (jsonStr) => {
        const roomCode = localStorage.getItem("lovelive_room_code");
        if (!roomCode) return false;
        try {
            const res = await fetch('/api/debug/board_override', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json', 'X-Room-Id': roomCode },
                body: jsonStr
            });
            return res.ok;
        } catch (e) { return false; }
    },

    toggleDebugMode: async () => {
        const roomCode = localStorage.getItem("lovelive_room_code");
        if (!roomCode) return null;
        try {
            const res = await fetch('/api/debug/toggle', { method: 'POST', headers: { 'X-Room-ID': roomCode } });
            const data = await res.json();
            return data.success ? data.debug_mode : null;
        } catch (e) { return null; }
    },

    rewind: async (networkFacade) => {
        const roomCode = localStorage.getItem("lovelive_room_code");
        if (!roomCode) return false;
        try {
            const res = await fetch('/api/debug/rewind', { method: 'POST', headers: { 'X-Room-ID': roomCode } });
            const data = await res.json();
            if (data.success && networkFacade?.fetchState) await networkFacade.fetchState();
            return data.success;
        } catch (e) { return false; }
    },

    redo: async (networkFacade) => {
        const roomCode = localStorage.getItem("lovelive_room_code");
        if (!roomCode) return false;
        try {
            const res = await fetch('/api/debug/redo', { method: 'POST', headers: { 'X-Room-ID': roomCode } });
            const data = await res.json();
            if (data.success && networkFacade?.fetchState) await networkFacade.fetchState();
            return data.success;
        } catch (e) { return false; }
    },

    exportGame: async () => {
        const roomCode = localStorage.getItem("lovelive_room_code");
        if (!roomCode) return null;
        try {
            const res = await fetch('/api/export_game', { headers: { 'X-Room-ID': roomCode } });
            const data = await res.json();
            return data.success === false ? null : data;
        } catch (e) { return null; }
    },

    importGame: async (exportData, networkFacade) => {
        const roomCode = localStorage.getItem("lovelive_room_code");
        if (!roomCode) return false;
        try {
            const res = await fetch('/api/import_game', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json', 'X-Room-ID': roomCode },
                body: JSON.stringify(exportData)
            });
            const data = await res.json();
            if (data.success && networkFacade?.fetchState) await networkFacade.fetchState();
            return data.success;
        } catch (e) { return false; }
    },

    buildCheckpointStrings: (exportData) => {
        if (!exportData) return [];

        const history = Array.isArray(exportData.history) ? exportData.history : [];
        return history.map((state, index) => ({
            index,
            turn: state?.turn ?? '?',
            phase: state?.phase ?? '?',
            current_player: state?.current_player ?? 0,
            score: Array.isArray(state?.players) ? state.players.map((player) => player?.score ?? 0) : [],
            serialized: JSON.stringify(state),
            state,
        }));
    },

    buildDownloadReport: async (explanation) => {
        const [snapshot, exportData, standardized] = await Promise.all([
            DebugService.fetchDebugSnapshot(),
            DebugService.exportGame(),
            DebugService.fetchStandardizedState(),
        ]);

        return {
            timestamp: new Date().toISOString(),
            explanation: explanation || "",
            userAgent: navigator.userAgent,
            room_id: State.roomCode || null,
            debug_snapshot: snapshot?.success ? snapshot : null,
            standardized_state: standardized || null,
            export_bundle: exportData || null,
            checkpoint_strings: DebugService.buildCheckpointStrings(exportData).map((entry) => ({
                index: entry.index,
                turn: entry.turn,
                phase: entry.phase,
                current_player: entry.current_player,
                score: entry.score,
                state_string: entry.serialized,
            })),
            fallback_summary: DebugService._buildSlimReport(explanation),
        };
    },

    forceAction: async (id, networkFacade) => {
        try {
            const res = await fetch('api/action', {
                method: 'POST',
                headers: networkFacade?.getHeaders ? networkFacade.getHeaders() : {},
                body: JSON.stringify({ action_id: id, force: true })
            });
            const text = await res.text();
            State.lastStateJson = text;
            const data = JSON.parse(text);
            if (data.success) {
                import('../state.js').then(m => m.updateStateData(data.state));
                if (networkFacade?.clearPlannerData) networkFacade.clearPlannerData();
                return true;
            }
        } catch (e) { console.error(e); }
        return false;
    },

    forcedTurnEnd: async (networkFacade) => {
        try {
            await fetch('api/force_turn_end', { method: 'POST', headers: networkFacade?.getHeaders ? networkFacade.getHeaders() : {} });
            if (networkFacade?.fetchState) await networkFacade.fetchState();
        } catch (e) { console.error(e); }
    },

    execCode: async (code, networkFacade) => {
        try {
            const res = await fetch('api/exec', {
                method: 'POST',
                headers: networkFacade?.getHeaders ? networkFacade.getHeaders() : {},
                body: JSON.stringify({ code: code })
            });
            const text = await res.text();
            State.lastStateJson = text;
            const data = JSON.parse(text);
            if (data.success) {
                import('../state.js').then(m => m.updateStateData(data.state));
                if (networkFacade?.clearPlannerData) networkFacade.clearPlannerData();
                log('Code executed');
            } else {
                log('Error: ' + data.error);
            }
        } catch (e) { console.error(e); }
    }
};
