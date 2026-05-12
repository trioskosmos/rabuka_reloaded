import { State } from '../state.js';
import { ActionButtons } from './ActionButtons.js';
import { Tooltips } from '../ui_tooltips.js';
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

        // Show card/ability context when a choice is pending
        const pc = state.pending_choice;
        if (pc) {
            const ctxDiv = document.createElement('div');
            ctxDiv.className = 'pending-choice-context';
            ctxDiv.style.cssText = 'padding:6px 8px; margin-bottom:8px; border-left:3px solid #9966ff; background:rgba(153,102,255,0.08); border-radius:4px; font-size:0.85em;';

            let ctxHTML = '';
            if (pc.card_name) {
                ctxHTML += `<strong style="color:#cc88ff;">${pc.card_name}</strong>`;
                if (pc.card_no) ctxHTML += ` <span style="opacity:0.5; font-size:0.85em;">[${pc.card_no}]</span>`;
                if (pc.trigger_type) ctxHTML += ` <span style="opacity:0.6; font-size:0.8em;">(${pc.trigger_type})</span>`;
            }
            if (pc.ability_text) {
                const enrichedAbility = Tooltips.enrichAbilityText(pc.ability_text);
                ctxHTML += `<div style="color:rgba(255,255,255,0.7); margin-top:3px; line-height:1.5; font-size:0.9em;">${enrichedAbility}</div>`;
            }
            if (ctxHTML) {
                ctxDiv.innerHTML = ctxHTML;
                listDiv.appendChild(ctxDiv);
            }
        }

        const playActionsByHand = {};
        const mulliganActions = {};
        const abilityActions = [];
        const systemActions = [];

        console.log('[ActionListView] All legal_actions:', state.legal_actions);

        state.legal_actions.forEach(a => {
            const category = a.category || a.type;
            // Engine sends card_no in parameters for play_member_to_stage
            const cardNo = a.parameters?.card_no;
            const handIdx = a.parameters?.card_index;

            if (a.action_type === 'play_member_to_stage') {
                console.log('[ActionListView] PLAY action FULL object:', JSON.stringify(a, null, 2));
                console.log('[ActionListView] PLAY action details:', {
                    action_type: a.action_type,
                    card_no: cardNo,
                    card_index: handIdx,
                    card_id: a.parameters?.card_id,
                    parameters: a.parameters,
                    hasParameters: !!a.parameters,
                    parametersKeys: a.parameters ? Object.keys(a.parameters) : []
                });
            }

            if (a.action_type === 'decision' ||
                a.action_type === 'select_card' ||
                a.action_type === 'select_skip' ||
                a.action_type === 'choose_option' ||
                a.action_type === 'select_position' ||
                a.action_type === 'pass' ||
                a.action_type === 'confirm_mulligan' ||
                a.action_type === 'finish_live_card_set' ||
                a.action_type === 'choose_first_attacker' ||
                a.action_type === 'choose_second_attacker' ||
                a.action_type === 'set_live_card') {
                systemActions.push(a);
            } else if (a.action_type === 'play_member_to_stage' && cardNo !== undefined) {
                // Group by card_no - engine's primary identifier for cards
                if (!playActionsByHand[cardNo]) playActionsByHand[cardNo] = [];
                playActionsByHand[cardNo].push(a);
                console.log('[ActionListView] Grouped PLAY action for card_no', cardNo, ':', a);
            } else if (category === 'MULLIGAN' || a.action_type === 'select_mulligan' || a.action_type === 'mulligan_header') {
                if (handIdx !== undefined) {
                    if (!mulliganActions[handIdx]) mulliganActions[handIdx] = [];
                    mulliganActions[handIdx].push(a);
                }
            } else if (category === 'ABILITY' || a.action_type === 'use_ability') {
                abilityActions.push(a);
            }
        });

        console.log('[ActionListView] playActionsByHand:', playActionsByHand);

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

            // Select All button — dispatches all unselected select_mulligan actions
            const selectAllBtn = document.createElement('button');
            selectAllBtn.className = 'btn action-btn system';
            selectAllBtn.textContent = i18n.t('select_all');
            selectAllBtn.onclick = async () => {
                const cur = State.data;
                if (!cur) return;
                const player = perspectivePlayer === 0 ? cur.player1 : cur.player2;
                if (!player) return;
                const handCards = player.hand.cards;

                // Build set of already-selected indices from current state
                const selectedSet = new Set();
                if (player.mulligan_selection) {
                    if (typeof player.mulligan_selection === 'number') {
                        for (let i = 0; i < handCards.length; i++) {
                            if ((player.mulligan_selection >> i) & 1) selectedSet.add(i);
                        }
                    } else if (Array.isArray(player.mulligan_selection)) {
                        player.mulligan_selection.forEach(i => selectedSet.add(Number(i)));
                    }
                }

                // Find select_mulligan actions for cards NOT yet selected
                const actionsToSend = cur.legal_actions.filter(a =>
                    a.action_type === 'select_mulligan' &&
                    a.parameters?.card_index !== undefined &&
                    !selectedSet.has(a.parameters.card_index)
                );

                if (actionsToSend.length === 0) return;

                selectAllBtn.disabled = true;
                selectAllBtn.textContent = i18n.t('loading');
                try {
                    for (const action of actionsToSend) {
                        if (window.Network?.sendAction) {
                            await window.Network.sendAction(action);
                        } else {
                            window.doAction(action);
                        }
                    }
                } finally {
                    selectAllBtn.disabled = false;
                    selectAllBtn.textContent = i18n.t('select_all');
                }
            };
            listDiv.appendChild(selectAllBtn);
        }

        if (Object.keys(playActionsByHand).length > 0) {
            addHeader(i18n.t('event_play').toUpperCase(), 'var(--accent-gold)');
            Object.keys(playActionsByHand).forEach(cardNo => {
                const actions = playActionsByHand[cardNo];
                const firstA = actions[0];
                const groupDiv = document.createElement('div');
                groupDiv.className = 'action-group-card';

                const header = document.createElement('div');
                header.className = 'action-group-header';
                const energyIcon = `<img src="img/texticon/icon_energy.png" style="height:14px; vertical-align:middle; margin-left: 5px;">`;
                const displayCost = firstA.parameters?.base_cost ?? 0;
                const cleanName = firstA.parameters?.card_name ?? firstA.description ?? "Unknown";
                header.innerHTML = `<span class="truncate-name" style="max-width: 180px;">${cleanName}</span> <span class="header-base-cost">${energyIcon}${displayCost}</span>`;
                groupDiv.appendChild(header);

                const availableAreas = firstA.parameters?.available_areas;

                if (availableAreas && availableAreas.length > 0) {
                    const areasDiv = document.createElement('div');
                    areasDiv.className = 'action-group-buttons grid-3';
                    
                    const areaLabels = { 'left': 'Left', 'center': 'Center', 'right': 'Right' };
                    
                    // Always render 3 slots (left, center, right)
                    const areaOrder = ['left', 'center', 'right'];
                    areaOrder.forEach((expectedArea) => {
                        const areaInfo = availableAreas.find(a => a.area === expectedArea);
                        if (areaInfo && areaInfo.available) {
                            const areaName = areaInfo.area;
                            const label = areaLabels[areaName] || areaName;
                            const cost = areaInfo.cost;
                            const isBaton = areaInfo.is_baton_touch;
                            
                            const areaActionCopy = { ...firstA };
                            areaActionCopy.parameters = { ...firstA.parameters, stage_area: areaName };
                            
                            const btn = ActionButtons.createActionButton(areaActionCopy, true, '', state);
                            const costText = isBaton ? `${label} (${cost} - Baton)` : `${label} (${cost})`;
                            btn.innerHTML = `<span>${costText}</span>`;
                            btn.style.width = '100%';
                            areasDiv.appendChild(btn);
                        } else {
                            const spacer = document.createElement('div');
                            spacer.style.visibility = 'hidden';
                            spacer.style.minHeight = '36px';
                            areasDiv.appendChild(spacer);
                        }
                    });
                    groupDiv.appendChild(areasDiv);
                } else {
                    console.warn('[ActionListView] No available_areas found for action:', firstA);
                }

                listDiv.appendChild(groupDiv);
            });
        }

        container.innerHTML = '';
        container.appendChild(listDiv);
    }
};
