import { ActionButtons } from './ActionButtons.js';
import { ModalManager } from '../utils/ModalManager.js';
import * as i18n from '../i18n/index.js';

export const RpsView = {
    render: (state, perspectivePlayer, container) => {
        const modalEl = document.getElementById('rps-modal');
        const body = document.getElementById('rps-modal-body');
        if (body && modalEl && window.__isMobile) {
            if (modalEl.dataset.dismissed === 'true') {
                RpsView._renderInline(state, container);
                return;
            }
            delete modalEl.dataset.dismissed;
            body.innerHTML = '';
            const signs = [
                { actionType: 'RockChoice', snakeType: 'rock_choice', name: i18n.t('rps_rock') },
                { actionType: 'PaperChoice', snakeType: 'paper_choice', name: i18n.t('rps_paper') },
                { actionType: 'ScissorsChoice', snakeType: 'scissors_choice', name: i18n.t('rps_scissors') }
            ];
            signs.forEach((sign, idx) => {
                const found = state.legal_actions && state.legal_actions.find(a => 
                    a.action_type === sign.actionType || a.action_type === sign.snakeType
                );
                const a = found || { action_type: sign.snakeType, description: sign.name, index: idx };
                if (found && found.index === undefined && state.legal_actions) {
                    found.index = state.legal_actions.indexOf(found);
                }
                const btn = ActionButtons.createActionButton(a, false, 'rps-btn', state);
                btn.style.width = '100%';
                btn.style.padding = '14px 20px';
                btn.style.fontSize = '1.1rem';
                btn.addEventListener('click', () => ModalManager.hide('rps-modal'));
                body.appendChild(btn);
            });
            ModalManager.show('rps-modal');
            return;
        }

        RpsView._renderInline(state, container);
    },

    _renderInline: (state, container) => {
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

        const signs = [
            { actionType: 'RockChoice', snakeType: 'rock_choice', name: i18n.t('rps_rock') },
            { actionType: 'PaperChoice', snakeType: 'paper_choice', name: i18n.t('rps_paper') },
            { actionType: 'ScissorsChoice', snakeType: 'scissors_choice', name: i18n.t('rps_scissors') }
        ];

        signs.forEach((sign, idx) => {
            const found = state.legal_actions && state.legal_actions.find(a => 
                a.action_type === sign.actionType || a.action_type === sign.snakeType
            );
            const a = found || { action_type: sign.snakeType, description: sign.name, index: idx };
            if (found && found.index === undefined && state.legal_actions) {
                found.index = state.legal_actions.indexOf(found);
            }
            const btn = ActionButtons.createActionButton(a, false, 'rps-btn', state);
            btn.style.width = '120px';
            btnContainer.appendChild(btn);
        });

        rpsDiv.appendChild(btnContainer);
        container.appendChild(rpsDiv);
    }
};
