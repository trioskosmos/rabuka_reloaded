import { ActionButtons } from './ActionButtons.js';
import { ActionBases } from '../generated_constants.js';
import * as i18n from '../i18n/index.js';

export const RpsView = {
    render: (state, perspectivePlayer, container) => {
        const rpsDiv = document.createElement('div');
        rpsDiv.className = 'rps-selector';
        rpsDiv.style.textAlign = 'center';
        rpsDiv.style.padding = '15px';
        rpsDiv.style.background = 'rgba(255, 255, 255, 0.05)';
        rpsDiv.style.borderRadius = '12px';
        rpsDiv.style.marginBottom = '20px';

        const title = i18n.t('choose_sign');
        rpsDiv.innerHTML = `<h3 style="margin-top:0; color:var(--accent-gold);">${title}</h3>`;

        const btnContainer = document.createElement('div');
        btnContainer.style.display = 'flex';
        btnContainer.style.flexDirection = 'column';
        btnContainer.style.alignItems = 'center';
        btnContainer.style.gap = '10px';

        const baseId = (perspectivePlayer === 1) ? ActionBases.RPS_P2 : ActionBases.RPS;
        const signs = [
            { actionType: 'RockChoice', snakeType: 'rock_choice', name: i18n.t('rps_rock') },
            { actionType: 'PaperChoice', snakeType: 'paper_choice', name: i18n.t('rps_paper') },
            { actionType: 'ScissorsChoice', snakeType: 'scissors_choice', name: i18n.t('rps_scissors') }
        ];

        signs.forEach((sign, idx) => {
            // Rust engine sends snake_case action_type (e.g., "rock_choice")
            const legalAction = state.legal_actions && state.legal_actions.find(a => 
                a.action_type === sign.actionType || a.action_type === sign.snakeType
            );
            const hasAction = !!legalAction;
            
            // Debug: log what we're checking against
            console.log('RPS sign:', sign.actionType, 'Legal actions:', state.legal_actions, 'Found:', hasAction);
            const a = legalAction || { action_type: sign.actionType, description: sign.name, index: idx };
            // Ensure the action has an index for execution
            if (legalAction && legalAction.index === undefined) {
                const actionIndex = state.legal_actions.indexOf(legalAction);
                legalAction.index = actionIndex;
            }
            const btn = ActionButtons.createActionButton(a, false, 'rps-btn', state);
            btn.style.width = '120px';
            btn.style.opacity = hasAction ? '1' : '0.4';
            btn.style.pointerEvents = hasAction ? 'auto' : 'none';
            btnContainer.appendChild(btn);
        });

        rpsDiv.appendChild(btnContainer);
        container.appendChild(rpsDiv);
    }
};
