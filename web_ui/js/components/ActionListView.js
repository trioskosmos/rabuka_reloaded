import { State } from '../state.js';
import { ActionButtons } from './ActionButtons.js';
import * as i18n from '../i18n/index.js';
import { StringUtils } from '../utils/StringUtils.js';

export const ActionListView = {
    render: (state, perspectivePlayer, container) => {
        if (!state.legal_actions || state.legal_actions.length === 0) {
            container.innerHTML = `<div class="no-actions">${i18n.t('wait')}</div>`;
            return;
        }

        const listDiv = document.createElement('div');
        listDiv.className = 'action-list';
        container.appendChild(listDiv);

        const playActionsByHand = {};
        const mulliganActions = {};
        const abilityActions = [];
        const systemActions = [];
        const otherActions = [];

        state.legal_actions.forEach(a => {
            const category = a.category || a.type;
            const hIdx = a.hand_idx;
            const sIdx = a.slot_idx;

            if (a.source_card_id === undefined && a.card_id !== undefined) {
                a.source_card_id = a.card_id;
            }

            if (a.source_card_id === undefined) {
                // Rust backend format: player1, player2
                const p = perspectivePlayer === 0 ? state.player1 : state.player2;
                const params = a.parameters || {};
                if (params.card_index !== undefined && p?.hand?.cards) {
                    const card = p.hand.cards[params.card_index];
                    if (card) a.source_card_id = card.card_no;
                } else if (category === 'ABILITY' && params.stage_area !== undefined && p?.stage) {
                    const areaMap = { 'left_side': p.stage.left_side, 'center': p.stage.center, 'right_side': p.stage.right_side };
                    const card = areaMap[params.stage_area.toLowerCase()];
                    if (card) a.source_card_id = card.card_no;
                }
            }

            if (a.action_type === 'Pass' || a.action_type === 'SkipMulligan' || a.action_type === 'ConfirmMulligan' || a.action_type === 'FinishLiveCardSet' || a.name?.includes('End') || a.name?.includes('終了')) {
                systemActions.push(a);
            } else if (category === 'PLAY' && hIdx !== undefined) {
                if (!playActionsByHand[hIdx]) playActionsByHand[hIdx] = [];
                playActionsByHand[hIdx].push(a);
            } else if ((a.action_type === 'SelectMulligan' || a.action_type === 'MulliganHeader') && hIdx !== undefined) {
                if (!mulliganActions[hIdx]) mulliganActions[hIdx] = [];
                mulliganActions[hIdx].push(a);
            } else if (category === 'ABILITY') {
                abilityActions.push(a);
            } else {
                otherActions.push(a);
            }
        });

        const addHeader = (text, color) => {
            const header = document.createElement('div');
            header.className = 'category-header';
            header.style.color = color || 'rgba(255,255,255,0.4)';
            header.innerText = text;
            listDiv.appendChild(header);
        };

        if (systemActions.length > 0) {
            addHeader(i18n.t('system'));
            systemActions.forEach(a => listDiv.appendChild(ActionButtons.createActionButton(a, false, a.action_type === 'Pass' ? 'confirm system' : 'system', state)));
        }

        if (abilityActions.length > 0) {
            addHeader(i18n.t('act_ability').toUpperCase(), '#9966ff');
            abilityActions.forEach(a => listDiv.appendChild(ActionButtons.createActionButton(a, false, '', state)));
        }

        const allMulliganActions = Object.values(mulliganActions).flat();
        if (allMulliganActions.length > 0) {
            addHeader(i18n.t('mulligan').toUpperCase(), 'var(--accent-pink)');
            allMulliganActions.forEach(a => listDiv.appendChild(ActionButtons.createActionButton(a, false, '', state)));
        }

        if (Object.keys(playActionsByHand).length > 0) {
            addHeader(i18n.t('event_play').toUpperCase(), 'var(--accent-gold)');
            Object.keys(playActionsByHand).sort((a, b) => parseInt(a) - parseInt(b)).forEach(hIdx => {
                const actions = playActionsByHand[hIdx];
                const firstA = actions[0];
                const groupDiv = document.createElement('div');
                groupDiv.className = 'action-group-card';

                const header = document.createElement('div');
                header.className = 'action-group-header';
                const energyIcon = `<img src="img/texticon/icon_energy.png" style="height:14px; vertical-align:middle; margin-left: 5px;">`;
                const displayCost = firstA.cost ?? firstA.base_cost ?? 0;
                const sourceCard = firstA.source_card_id !== undefined ? State.resolveCardData(firstA.source_card_id) : null;
                let cleanName = sourceCard ? i18n.translateCard(sourceCard).name : (firstA.name ?? "");
                cleanName = StringUtils.cleanCardName(cleanName);
                header.innerHTML = `<span class="truncate-name" style="max-width: 180px;">${cleanName}</span> <span class="header-base-cost">${energyIcon}${displayCost}</span>`;
                groupDiv.appendChild(header);

                const btnsDiv = document.createElement('div');
                btnsDiv.className = 'action-group-buttons grid-3';
                for (let slotIdx = 0; slotIdx < 3; slotIdx++) {
                    const a = actions.find(act => (act.slot_idx === slotIdx) && act.secondary_slot_idx === undefined);
                    if (a) {
                        btnsDiv.appendChild(ActionButtons.createActionButton(a, true, '', state));
                    } else {
                        const spacer = document.createElement('div');
                        spacer.style.visibility = 'hidden';
                        spacer.style.minHeight = '36px';
                        btnsDiv.appendChild(spacer);
                    }
                }
                groupDiv.appendChild(btnsDiv);

                const doubleActions = actions.filter(act => act.secondary_slot_idx !== undefined);
                if (doubleActions.length > 0) {
                    const pairs = {};
                    doubleActions.forEach(a => {
                        const s1 = a.slot_idx;
                        const s2 = a.secondary_slot_idx;
                        const key = [s1, s2].sort().join('-');
                        if (!pairs[key]) pairs[key] = [];
                        pairs[key].push(a);
                    });

                    Object.values(pairs).forEach(pairActions => {
                        const doubleDiv = document.createElement('div');
                        doubleDiv.className = 'action-group-buttons double-baton-row grid-3';

                        const pairSlots = new Set();
                        pairActions.forEach(a => pairSlots.add(a.slot_idx));
                        pairActions.forEach(a => pairSlots.add(a.secondary_slot_idx));

                        for (let i = 0; i < 3; i++) {
                            const a = pairActions.find(act => act.slot_idx === i);
                            if (a) {
                                const btn = ActionButtons.createActionButton(a, true, 'double-baton-btn', state);
                                btn.style.width = '100%';
                                doubleDiv.appendChild(btn);
                            } else if (pairSlots.has(i)) {
                                const spacer = document.createElement('div');
                                spacer.className = 'pair-spacer';
                                spacer.innerText = i18n.t('gap');
                                doubleDiv.appendChild(spacer);
                            } else {
                                const spacer = document.createElement('div');
                                spacer.style.visibility = 'hidden';
                                doubleDiv.appendChild(spacer);
                            }
                        }
                        groupDiv.appendChild(doubleDiv);
                    });
                }
                listDiv.appendChild(groupDiv);
            });
        }

        if (otherActions.length > 0) {
            addHeader(i18n.t('actions').toUpperCase());
            otherActions.forEach(a => listDiv.appendChild(ActionButtons.createActionButton(a, false, '', state)));
        }
    }
};
