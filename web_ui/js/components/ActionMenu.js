import { State } from '../state.js';
import { Phase } from '../constants.js';
import * as i18n from '../i18n/index.js';
import { DOMUtils } from '../utils/DOMUtils.js';
import { DOM_IDS } from '../constants_dom.js';

import { RpsView } from './RpsView.js';
import { ChoiceView } from './ChoiceView.js';
import { ActionListView } from './ActionListView.js';

export const ActionMenu = {
    renderActions: () => {
        const state = State.data;
        if (!state || state.game_over) return;

        const perspectivePlayer = State.perspectivePlayer;

        // Clear action containers
        DOMUtils.clear(DOM_IDS.CONTAINER_ACTIONS);
        DOMUtils.clear(DOM_IDS.CONTAINER_MOBILE_ACTION_BAR);

        const actionsDiv = DOMUtils.getElement(DOM_IDS.CONTAINER_ACTIONS);
        if (!actionsDiv) return;

        // 1. RPS Phase
        if (state.phase === Phase.ROCK_PAPER_SCISSORS) {
            RpsView.render(state, perspectivePlayer, actionsDiv);
            return;
        }

        // 2. Pending Choice
        if (state.pending_choice) {
            ChoiceView.render(state, actionsDiv);
            return;
        }

        // 3. AI Thinking
        if (state.is_ai_thinking) {
            const aiDiv = document.createElement('div');
            aiDiv.className = 'ai-thinking-indicator';
            aiDiv.innerHTML = `<div style="font-weight:bold; color:#0096ff; padding:10px; border-left:4px solid #0096ff; background:rgba(0,150,255,0.1); border-radius:8px;">${state.ai_status || i18n.t('ai_thinking')}</div>`;
            actionsDiv.appendChild(aiDiv);
        }

        // 4. Action List
        ActionListView.render(state, perspectivePlayer, actionsDiv);
    },

    renderGameOver: (state) => {
        const winnerName = state.winner === State.perspectivePlayer ? "YOU" : `Player ${state.winner + 1}`;
        const gameOverHTML = `
                <div class="game-over-banner">
                    <h2>GAME OVER</h2>
                    <div class="winner-announcement">Winner: ${winnerName}</div>
                    <button class="btn btn-primary" data-action="reload-page">New Game</button>
                </div>
            `;
        DOMUtils.setHTML(DOM_IDS.CONTAINER_ACTIONS, gameOverHTML);
    }
};
