import { ActionButtons } from './ActionButtons.js';
import { ModalManager } from '../utils/ModalManager.js';
import { State } from '../state.js';
import * as i18n from '../i18n/index.js';

const SIGNS = [
    { actionType: 'RockChoice', snakeType: 'rock_choice', name: 'rps_rock', emoji: '✊' },
    { actionType: 'PaperChoice', snakeType: 'paper_choice', name: 'rps_paper', emoji: '✋' },
    { actionType: 'ScissorsChoice', snakeType: 'scissors_choice', name: 'rps_scissors', emoji: '✌️' },
];

function signByValue(v) {
    if (v === 0) return SIGNS[0];
    if (v === 1) return SIGNS[1];
    if (v === 2) return SIGNS[2];
    return null;
}

function getRpsDrawHistory(state) {
    if (!state.structured_log) return [];
    return state.structured_log
        .filter(e => e.category === 'rps')
        .map(e => {
            const m = e.metadata || {};
            return {
                p1: m.p1_choice || '—',
                p2: m.p2_choice || '—',
                winner: m.winner || '',
                p1_value: m.p1_value,
                p2_value: m.p2_value,
            };
        });
}

function renderFull(body, state, perspectivePlayer) {
    body.innerHTML = '';

    const mode = state.mode;
    const isSandbox = mode && mode !== 'pvp' && mode !== 'pve';
    const isPve = mode === 'pve';

    let myIdx;
    let showBothLabels;
    if (isPve) {
        myIdx = 0;
        showBothLabels = false;
    } else if (isSandbox && state.pending_rps_player_id != null) {
        myIdx = state.pending_rps_player_id;
        showBothLabels = false;
    } else if (isSandbox && state.player1_rps_choice != null && state.player2_rps_choice != null) {
        myIdx = 0;
        showBothLabels = true;
    } else {
        myIdx = perspectivePlayer;
        showBothLabels = isSandbox ? false : true;
    }

    const myLabel = `P${myIdx + 1}`;
    const oppIdx = myIdx === 0 ? 1 : 0;
    const oppLabel = `P${oppIdx + 1}`;

    const hasLegalRps = state.legal_actions?.some(a =>
        SIGNS.some(s => a.action_type === s.actionType || a.action_type === s.snakeType)
    );

    const myChoice = myIdx === 0 ? state.player1_rps_choice : state.player2_rps_choice;
    const oppChoice = myIdx === 0 ? state.player2_rps_choice : state.player1_rps_choice;
    const history = getRpsDrawHistory(state);
    const drawCount = history.length;

    // Header: player identity
    const header = document.createElement('div');
    header.className = 'rps-heading';
    if (showBothLabels) {
        header.innerHTML = `<div class="rps-title">RPS</div>
            <div class="rps-player-badge"><strong>${myLabel}</strong> · <strong>${oppLabel}</strong></div>`;
    } else {
        header.innerHTML = `<div class="rps-title">Choose for <strong>${myLabel}</strong></div>`;
    }
    body.appendChild(header);

    // Draw history (if any draws occurred)
    if (drawCount > 0) {
        const histDiv = document.createElement('div');
        histDiv.className = 'rps-history';
        histDiv.innerHTML = `<div class="rps-history-title">Draws: ${drawCount}</div>` +
            history.map((h, i) => {
                return `<div class="rps-history-row">
                    <span class="rps-history-num">#${i + 1}</span>
                    <span>${h.p1} vs ${h.p2}</span>
                    <span class="rps-history-draw">Draw</span>
                </div>`;
            }).join('');
        body.appendChild(histDiv);
    }

    // Both have chosen — show result
    if (myChoice != null && oppChoice != null) {
        const mySign = signByValue(myChoice);
        const oppSign = signByValue(oppChoice);
        const winner = state.rps_winner;
        const iWon = winner === myIdx;
        const isDraw = winner === -1 || winner === undefined || winner === null;

        let resultText;
        if (isDraw) {
            resultText = 'Draw! Choose again.';
        } else if (iWon) {
            resultText = `${myLabel} Wins!`;
        } else {
            resultText = `${oppLabel} Wins.`;
        }

        let statusHtml = `<div class="rps-waiting">
            <div class="rps-waiting-choice">
                <span class="rps-emoji rps-emoji-lg">${mySign ? mySign.emoji : '?'}</span>
                <span class="rps-waiting-label">${mySign ? i18n.t(mySign.name) : '?'} (${myLabel})</span>
            </div>
            <div class="rps-vs">vs</div>
            <div class="rps-waiting-choice">
                <span class="rps-emoji rps-emoji-lg">${oppSign ? oppSign.emoji : '?'}</span>
                <span class="rps-waiting-label">${oppSign ? i18n.t(oppSign.name) : '?'} (${oppLabel})</span>
            </div>
            <div class="rps-waiting-text" style="font-size:1rem;font-weight:bold;margin-top:8px;">${resultText}</div>
        </div>`;
        body.innerHTML += statusHtml;
        return;
    }

    // My choice is sent, waiting for opponent
    if (myChoice != null) {
        const mySign = signByValue(myChoice);
        let waitingText = isSandbox ? `Waiting for ${oppLabel}...` : i18n.t('waiting_for_opponent');
        let statusHtml = `<div class="rps-waiting">
            <div class="rps-waiting-choice">
                <span class="rps-emoji rps-emoji-lg">${mySign ? mySign.emoji : '?'}</span>
                <span class="rps-waiting-label">${mySign ? i18n.t(mySign.name) : '?'} (${myLabel})</span>
            </div>
            <div class="rps-vs">vs</div>
            <div class="rps-waiting-choice">
                <span class="rps-emoji rps-emoji-lg">❔</span>
                <span class="rps-waiting-label">???</span>
            </div>
            <div class="rps-waiting-text">${waitingText}</div>
            <div class="rps-spinner"></div>
        </div>`;
        body.innerHTML += statusHtml;
        return;
    }

    // Opponent chose, waiting for me
    if (oppChoice != null && myChoice == null && !hasLegalRps) {
        const oppSign = signByValue(oppChoice);
        body.innerHTML += `<div class="rps-waiting">
            <div class="rps-waiting-text">${oppLabel} chose — waiting for you...</div>
            <div class="rps-waiting-choice">
                <span class="rps-emoji rps-emoji-lg">${oppSign ? oppSign.emoji : '?'}</span>
                <span class="rps-waiting-label">${oppSign ? i18n.t(oppSign.name) : '?'}</span>
            </div>
            <div class="rps-spinner"></div>
        </div>`;
        return;
    }

    // No choices made yet — show buttons
    const grid = document.createElement('div');
    grid.className = 'rps-grid';

    SIGNS.forEach((sign, idx) => {
        const found = state.legal_actions?.find(a =>
            a.action_type === sign.actionType || a.action_type === sign.snakeType
        );
        if (!found) return;
        if (found.index === undefined && state.legal_actions) {
            found.index = state.legal_actions.indexOf(found);
        }

        const btn = document.createElement('button');
        btn.className = 'rps-choice-btn';
        btn.innerHTML = `
            <span class="rps-emoji">${sign.emoji}</span>
            <span class="rps-label">${i18n.t(sign.name)}</span>
        `;
        btn.onclick = () => {
            if (found.index !== undefined) {
                window.doAction?.(found);
                renderFull(body, {
                    ...state,
                    legal_actions: [],
                    [myIdx === 0 ? 'player1_rps_choice' : 'player2_rps_choice']: idx,
                }, perspectivePlayer);
            }
        };
        grid.appendChild(btn);
    });
    body.appendChild(grid);
}

export const RpsView = {
    render: (state, perspectivePlayer, container) => {
        const modalEl = document.getElementById('rps-modal');
        const body = document.getElementById('rps-modal-body');
        if (!modalEl || !body) {
            RpsView._renderFallback(state, container);
            return;
        }

        renderFull(body, state, perspectivePlayer);
        ModalManager.show('rps-modal');
    },

    hideIfOpen: () => {
        const modalEl = document.getElementById('rps-modal');
        if (modalEl && modalEl.style.display !== 'none') {
            ModalManager.hide('rps-modal');
        }
    },

    _renderFallback: (state, container) => {
        const wrapper = document.createElement('div');
        wrapper.style.cssText = 'display:flex;justify-content:center;gap:12px;padding:16px;';

        SIGNS.forEach((sign, idx) => {
            const found = state.legal_actions?.find(a =>
                a.action_type === sign.actionType || a.action_type === sign.snakeType
            );
            const action = found || { action_type: sign.snakeType, description: i18n.t(sign.name), index: idx };
            if (found && found.index === undefined && state.legal_actions) {
                found.index = state.legal_actions.indexOf(found);
            }
            const btn = ActionButtons.createActionButton(action, false, 'rps-btn', state);
            wrapper.appendChild(btn);
        });
        container.appendChild(wrapper);
    }
};
