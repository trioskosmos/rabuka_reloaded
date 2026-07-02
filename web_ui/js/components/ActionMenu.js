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

let _sentRpsP1 = false;
let _sentRpsP2 = false;
let _sentTurn = false;

function resetSentFlags(state) {
    if (state.phase !== Phase.ROCK_PAPER_SCISSORS) {
        _sentRpsP1 = false;
        _sentRpsP2 = false;
    }
    if (state.phase !== Phase.ROCK_PAPER_SCISSORS && !state.legal_actions?.some(a => a.action_type === 'choose_first_attacker' || a.action_type === 'ChooseFirstAttacker')) {
        _sentTurn = false;
    }
}

function autoResolveSandbox(state, actionsDiv) {
    const isSandbox = state.mode && state.mode !== 'pvp' && state.mode !== 'pve';
    if (!isSandbox) return false;

    RpsView.hideIfOpen();
    resetSentFlags(state);

    const findAction = (types) => state.legal_actions?.find(a => types.includes(a.action_type));

    // RPS phase — send for any player that hasn't chosen yet
    if (state.phase === Phase.ROCK_PAPER_SCISSORS) {
        // P1 sends Rock
        if (state.player1_rps_choice == null && !_sentRpsP1) {
            const a = findAction(['rock_choice', 'RockChoice']);
            if (a) { _sentRpsP1 = true; window.doAction?.(a);
                actionsDiv.innerHTML = `<div style="padding:16px;text-align:center;color:var(--text-muted);font-size:0.9rem;">⚡ P1 Rock</div>`; return true; }
        }
        // P2 sends Paper
        if (state.player2_rps_choice == null && !_sentRpsP2) {
            const a = findAction(['paper_choice', 'PaperChoice']);
            if (a) { _sentRpsP2 = true; window.doAction?.(a);
                actionsDiv.innerHTML = `<div style="padding:16px;text-align:center;color:var(--text-muted);font-size:0.9rem;">⚡ P2 Paper</div>`; return true; }
        }
        return false;
    }

    // Turn choice — send once
    if (!_sentTurn) {
        const a = findAction(['choose_first_attacker', 'ChooseFirstAttacker']);
        if (a) { _sentTurn = true; window.doAction?.(a);
            actionsDiv.innerHTML = `<div style="padding:16px;text-align:center;color:var(--text-muted);font-size:0.9rem;">⚡ First attacker</div>`; return true; }
    }

    return false;
}

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

        // Sandbox: auto-resolve RPS and turn choice
        if (autoResolveSandbox(state, actionsDiv)) return;

        // Clear stale play selection
        if (window._playSel) {
            const stillValid = state.legal_actions?.some(a =>
                a.action_type === 'play_member_to_stage' &&
                (a.parameters?.card_index === window._playSel.cardIdx ||
                 a.parameters?.card_indices?.includes(window._playSel.cardIdx))
            );
            if (!stillValid) window._playSel = null;
        }

        // Show/hide floating PASS button
        ActionMenu.updatePassButton(state);

        // 1. RPS Phase — render before waiting gate so both players can choose
        if (state.phase === Phase.ROCK_PAPER_SCISSORS) {
            const mode = state.mode;
            if (mode === 'sandbox') {
                RpsView.hideIfOpen();
                actionsDiv.innerHTML = `<div style="padding:16px;text-align:center;color:var(--text-muted);font-size:0.9rem;">⚡ Resolving RPS...</div>`;
                return;
            }
            if (mode === 'pve') {
                RpsView.render(state, perspectivePlayer, actionsDiv);
                return;
            }
            RpsView.render(state, perspectivePlayer, actionsDiv);
            return;
        }

        // Hide RPS modal if phase ended
        RpsView.hideIfOpen();

        // PvE: show "AI is thinking..." when it's P2's turn (AI handles actions)
        if (state.mode === 'pve' && (state.active_player === 'player2' || state.active_player === '1' || state.active_player === 1)) {
            const aiDiv = document.createElement('div');
            aiDiv.className = 'ai-thinking-indicator';
            aiDiv.innerHTML = '<div style="font-weight:bold; color:#0096ff; padding:10px; border-left:4px solid #0096ff; background:rgba(0,150,255,0.1); border-radius:8px;">🤖 AI is thinking...</div>';
            actionsDiv.appendChild(aiDiv);
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

        // 2. Pending Choice — renders inline on desktop, modal on mobile
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
        // After RPS, only the winner gets routed to the 先行 choice
        const isMobileActions = typeof window.__isMobile === 'function' ? window.__isMobile() : false;
        if (isMobileActions && !State._sysActionsDismissed && state.legal_actions) {
            const systemOnly = state.legal_actions.filter(a =>
                a.action_type === 'choose_first_attacker' ||
                a.action_type === 'choose_second_attacker'
            );
            const isRpsWinnerChoice = state.rps_winner != null && systemOnly.some(a => a.action_type === 'choose_first_attacker');
            if (systemOnly.length > 0 && (!isRpsWinnerChoice || state.rps_winner === State.perspectivePlayer)) {
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
            'select_card', 'choose_option',
            'confirm_mulligan', 'skip_mulligan',
            'select_position',
            'finish_live_card_set', 'confirm_live_card_set', 'skip_live_card_set'];
        let foundAction = null;
        let foundLabel = '';
        if (state?.legal_actions) {
            for (const a of state.legal_actions) {
                const t = (a.action_type || '').toLowerCase();
                if (lowTypes.includes(t)) {
                    foundAction = a;
                    if (t === 'pass' || t === 'pass_remaining') foundLabel = i18n.t('pass_no') || 'PASS';
                    else if (t === 'decision' || t === 'select_skip' || t === 'select_card' || t === 'choose_option' || t === 'select_position') foundLabel = i18n.t('done') || 'DONE';
                    else if (t === 'confirm_mulligan' || t === 'confirm_live_card_set') foundLabel = i18n.t('confirm') || 'CONFIRM';
                    else if (t === 'skip_mulligan' || t === 'skip_live_card_set') foundLabel = i18n.t('skip') || 'SKIP';
                    else if (t === 'finish_live_card_set') foundLabel = i18n.t('finish_live_card_set') || 'DONE';
                    break;
                }
            }
        }
        const passLabel = document.getElementById('mobile-pass-label');
        passBtn.style.display = 'flex';
        if (foundAction) {
            passBtn.disabled = false;
            passBtn.style.opacity = '1';
            passBtn.style.cursor = 'pointer';
            passBtn.onclick = () => { if (window.doAction) window.doAction(foundAction); };
            if (passLabel) passLabel.textContent = foundLabel;
        } else {
            passBtn.disabled = true;
            passBtn.style.opacity = '0.4';
            passBtn.style.cursor = 'default';
            passBtn.onclick = null;
            if (passLabel) passLabel.textContent = 'PASS';
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
