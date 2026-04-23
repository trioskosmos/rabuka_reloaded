import { State } from '../state.js';
import { Phase } from '../constants.js';

export const PlannerService = {
    clearPlannerData: () => {
        State.plannerData = null;
        State.lastPlannerFetchKey = null;
        State.plannerLoading = false;
    },

    getPlannerFetchKey: () => {
        const state = State.data;
        if (!state || !State.roomCode) return null;
        return `${State.roomCode}:${state.turn}:${state.active_player}:${state.phase}`;
    },

    shouldAutoFetchPlanner: () => {
        const state = State.data;
        if (!state || !State.roomCode || State.offlineMode || State.replayMode || State.hotseatMode) {
            return false;
        }

        const trackedPhases = [Phase.MAIN, Phase.LIVE_SET, Phase.RESPONSE];
        const isRelevantTurn = trackedPhases.includes(state.phase) && state.active_player === State.perspectivePlayer && !state.game_over;
        const needsCompletionRefresh = State.plannerData?.your_sequence?.status === 'in_progress' && !isRelevantTurn;
        return isRelevantTurn || needsCompletionRefresh;
    },

    fetchPlannerData: async ({ score = false, silent = false } = {}, networkFacade) => {
        if (State.offlineMode || State.replayMode || !State.roomCode) {
            PlannerService.clearPlannerData();
            return null;
        }

        if (State.plannerLoading && !score) {
            return State.plannerData;
        }

        State.plannerLoading = true;

        try {
            const endpoint = score ? 'api/planner/score' : 'api/planner';
            const headers = networkFacade?.getHeaders ? networkFacade.getHeaders() : {};
            
            const res = await fetch(endpoint, {
                method: score ? 'POST' : 'GET',
                headers: headers,
                body: score ? JSON.stringify({}) : undefined
            });
            const data = await res.json();

            if (data.success) {
                State.plannerData = data.planner;
                State.lastPlannerFetchKey = PlannerService.getPlannerFetchKey();
                State.emit('change', State.data); 
            } else if (!score) {
                State.plannerData = null;
            }

            return data;
        } catch (e) {
            console.error('[Planner] Failed to fetch planner data:', e);
            return null;
        } finally {
            State.plannerLoading = false;
        }
    }
};
