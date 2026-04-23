import { State } from '../state.js';
import { Rendering } from '../ui_rendering.js';
import { ModalManager } from '../utils/ModalManager.js';
import { DOM_IDS } from '../constants_dom.js';

export const PerformanceModal = {
    showLastPerformance: () => {
        ModalManager.show(DOM_IDS.MODAL_PERFORMANCE);
        if (Rendering && Rendering.renderPerformanceResult) {
            const latestTurn = State.performanceHistoryTurns && State.performanceHistoryTurns.length > 0
                ? Math.max(...State.performanceHistoryTurns)
                : -1;
            State.selectedPerfTurn = latestTurn;
            const dataToUse = latestTurn !== -1 ? State.performanceHistory[latestTurn] : State.lastPerformanceData;
            Rendering.renderPerformanceResult(dataToUse);
        }
    },

    showPerformanceForTurn: (turn) => {
        State.selectedPerfTurn = turn;
        const data = State.performanceHistory[turn];
        if (data && Rendering && Rendering.renderPerformanceResult) {
            Rendering.renderPerformanceResult(data);
        }
    },

    closePerformanceModal: () => {
        ModalManager.hide(DOM_IDS.MODAL_PERFORMANCE);
    }
};
