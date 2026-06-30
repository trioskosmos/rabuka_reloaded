import { State } from '../state.js';
import { Phase } from '../constants.js';
import * as i18n from '../i18n/index.js';
import { DOMUtils } from '../utils/DOMUtils.js';
import { DOM_IDS } from '../constants_dom.js';
import { ModalManager } from '../utils/ModalManager.js';
import { ActionButtons } from './ActionButtons.js';

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

        // Show/hide floating PASS button
        ActionMenu.updatePassButton(state);

        // 1. RPS Phase — render before waiting gate so both players can choose
        if (state.phase === Phase.ROCK_PAPER_SCISSORS) {
            RpsView.render(state, perspectivePlayer, actionsDiv);
            return;
        }

        // 0. PVP: Waiting for opponent (flag set by server via pvp_player_can_act)
        if (state.waiting_for_opponent) {
            const waitDiv = document.createElement('div');
            waitDiv.className = 'waiting-opponent';
            waitDiv.innerHTML = `<div style="font-weight:bold; color:#ffcc00; padding:20px; text-align:center; border:2px solid #ffcc00; border-radius:12px; background:rgba(255,204,0,0.08);">⏳ Waiting for opponent's turn...</div>`;
            actionsDiv.appendChild(waitDiv);
            return;
        }

        // 2. Pending Choice — always render via ChoiceView (handles options, selection_cards, legal_actions)
        if (state.pending_choice) {
            ChoiceView.render(state, actionsDiv, true);
            return;
        }

        // 3. AI Thinking
        if (state.is_ai_thinking) {
            const aiDiv = document.createElement('div');
            aiDiv.className = 'ai-thinking-indicator';
            aiDiv.innerHTML = `<div style="font-weight:bold; color:#0096ff; padding:10px; border-left:4px solid #0096ff; background:rgba(0,150,255,0.1); border-radius:8px;">${state.ai_status || i18n.t('ai_thinking')}</div>`;
            actionsDiv.appendChild(aiDiv);
        }

        // 4. System actions modal (choose first/second and similar) — mobile only
        if (window.__isMobile && !State._sysActionsDismissed && state.legal_actions) {
            const systemOnly = state.legal_actions.filter(a =>
                a.action_type === 'choose_first_attacker' ||
                a.action_type === 'choose_second_attacker'
            );
            if (systemOnly.length > 0) {
                const sysBody = document.getElementById('system-actions-body');
                if (sysBody) {
                    sysBody.innerHTML = '';
                    systemOnly.forEach(a => {
                        const btn = ActionButtons.createActionButton(a, false, '', state);
                        btn.style.width = '100%';
                        btn.style.padding = '12px 16px';
                        btn.style.fontSize = '1rem';
                        btn.addEventListener('click', () => {
                            ModalManager.hide('system-actions-modal');
                            State._sysActionsDismissed = false;
                        });
                        sysBody.appendChild(btn);
                    });
                    ModalManager.show('system-actions-modal');
                }
            }
        }

        // 5. Action List
        ActionListView.render(state, perspectivePlayer, actionsDiv);
    },

    updatePassButton: (state) => {
        const modeLabel = document.getElementById('mobile-mode-label');
        if (modeLabel) {
            modeLabel.textContent = i18n.t(State.uiMode === 'view' ? 'mobile_mode_view' : 'mobile_mode_play');
        }
        const passBtn = document.getElementById('mobile-pass-btn');
        if (!passBtn) return;
        const lowTypes = ['pass', 'pass_remaining', 'decision', 'select_skip',
            'confirm_mulligan', 'skip_mulligan',
            'finish_live_card_set', 'confirm_live_card_set', 'skip_live_card_set'];
        let foundAction = null;
        let foundLabel = '';
        if (state?.legal_actions) {
            for (const a of state.legal_actions) {
                const t = (a.action_type || '').toLowerCase();
                if (lowTypes.includes(t)) {
                    foundAction = a;
                    if (t === 'pass' || t === 'pass_remaining') foundLabel = i18n.t('pass_no') || 'PASS';
                    else if (t === 'decision' || t === 'select_skip') foundLabel = i18n.t('done') || 'DONE';
                    else if (t === 'confirm_mulligan' || t === 'confirm_live_card_set') foundLabel = i18n.t('confirm') || 'CONFIRM';
                    else if (t === 'skip_mulligan' || t === 'skip_live_card_set') foundLabel = i18n.t('skip') || 'SKIP';
                    else if (t === 'finish_live_card_set') foundLabel = i18n.t('finish_live_card_set') || 'DONE';
                    break;
                }
            }
        }
        const passLabel = document.getElementById('mobile-pass-label');
        if (foundAction) {
            passBtn.classList.remove('hidden');
            passBtn.onclick = () => { if (window.doAction) window.doAction(foundAction); };
            if (passLabel) passLabel.textContent = foundLabel;
        } else {
            passBtn.classList.add('hidden');
            passBtn.onclick = null;
        }
    },

    updateMobileActionBadge: () => {
        const btn = DOMUtils.getElement(DOM_IDS.MOBILE_TOGGLE_ACTIONS);
        if (!btn) return;
        const state = State.data;
        const count = state?.legal_actions?.length || 0;
        let badge = btn.querySelector('.action-badge');
        if (count > 0) {
            if (!badge) {
                badge = document.createElement('span');
                badge.className = 'action-badge';
                btn.appendChild(badge);
            }
            badge.textContent = count;
        } else {
            if (badge) badge.remove();
        }
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
