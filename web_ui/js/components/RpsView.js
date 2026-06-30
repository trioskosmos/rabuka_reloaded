import { ActionButtons } from './ActionButtons.js';
import { ModalManager } from '../utils/ModalManager.js';
import * as i18n from '../i18n/index.js';

// Track local RPS choice for display while waiting
let _localRpsName = '';

export const RpsView = {
    resetLocalChoice: () => { _localRpsName = ''; },
    render: (state, perspectivePlayer, container) => {
        const isMobile = typeof window.__isMobile === 'function' ? window.__isMobile() : false;
        const modalEl = document.getElementById('rps-modal');
        const body = document.getElementById('rps-modal-body');
        if (body && modalEl && isMobile) {
            body.innerHTML = '';

            // If a choice was already made locally, show waiting state
            // (unless RPS buttons are available again = new RPS session, reset)
            const hasRpsBtns = state.legal_actions?.some(a =>
                ['RockChoice','rock_choice','PaperChoice','paper_choice','ScissorsChoice','scissors_choice'].includes(a.action_type)
            );
            if (_localRpsName && !hasRpsBtns) {
                const waitDiv = document.createElement('div');
                waitDiv.style.cssText = 'text-align:center;padding:24px;';
                waitDiv.innerHTML = `
                    <div style="font-size:1.1rem;font-weight:700;color:var(--accent-gold);margin-bottom:10px;">
                        You chose ${_localRpsName}
                    </div>
                    <div style="font-size:0.85rem;opacity:0.6;">Waiting for opponent...</div>
                `;
                body.appendChild(waitDiv);
                ModalManager.show('rps-modal');
                return;
            }
            // Reset if this is a new RPS phase
            _localRpsName = '';

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

                const origOnclick = btn.onclick;
                btn.onclick = (e) => {
                    _localRpsName = sign.name;
                    if (origOnclick) origOnclick.call(btn, e);
                    body.innerHTML = `
                        <div style="text-align:center;padding:24px;">
                            <div style="font-size:1.1rem;font-weight:700;color:var(--accent-gold);margin-bottom:10px;">
                                You chose ${sign.name}
                            </div>
                            <div style="font-size:0.85rem;opacity:0.6;">Waiting for opponent...</div>
                        </div>
                    `;
                };
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
