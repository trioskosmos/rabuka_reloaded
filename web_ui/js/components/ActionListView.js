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
        const liveCardActions = {};
        const abilityActions = [];
        const systemActions = [];

        // Clear stale button references before re-render
        State.mulliganButtons.clear();
        State.liveCardButtons.clear();

        state.legal_actions.forEach(a => {
            const category = a.category || a.type;
            const cardNo = a.parameters?.card_no;
            const handIdx = a.parameters?.card_index;

            if (a.action_type === 'decision' ||
                a.action_type === 'select_card' ||
                a.action_type === 'select_skip' ||
                a.action_type === 'choose_option' ||
                a.action_type === 'select_position' ||
                a.action_type === 'pass' ||
                a.action_type === 'confirm_mulligan' ||
                a.action_type === 'finish_live_card_set' ||
                a.action_type === 'confirm_live_card_set' ||
                a.action_type === 'choose_first_attacker' ||
                a.action_type === 'choose_second_attacker') {
                systemActions.push(a);
            } else if (a.action_type === 'play_member_to_stage' && cardNo !== undefined) {
                if (!playActionsByHand[cardNo]) playActionsByHand[cardNo] = [];
                playActionsByHand[cardNo].push(a);
            } else if (category === 'MULLIGAN' || a.action_type === 'select_mulligan' || a.action_type === 'mulligan_header') {
                if (handIdx !== undefined) {
                    if (!mulliganActions[handIdx]) mulliganActions[handIdx] = [];
                    mulliganActions[handIdx].push(a);
                }
            } else if (a.action_type === 'select_live_card' || a.action_type === 'live_card_header') {
                if (handIdx !== undefined) {
                    if (!liveCardActions[handIdx]) liveCardActions[handIdx] = [];
                    liveCardActions[handIdx].push(a);
                }
            } else if (category === 'ABILITY' || a.action_type === 'use_ability') {
                abilityActions.push(a);
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
            systemActions.forEach(a => {
                listDiv.appendChild(ActionButtons.createActionButton(a, false, a.action_type === 'Pass' ? 'confirm system' : 'system', state));
            });
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

        const allLiveCardActions = Object.values(liveCardActions).flat();
        if (allLiveCardActions.length > 0) {
            addHeader(i18n.t('live_card_set').toUpperCase(), 'var(--accent-cyan)');
            allLiveCardActions.forEach(a => listDiv.appendChild(ActionButtons.createActionButton(a, false, '', state)));
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
                let cleanName = firstA.parameters?.card_name ?? firstA.description ?? "Unknown";
                if (State.currentLang === 'en' && firstA.parameters?.card_id !== undefined) {
                    const card = Tooltips.findCardById(firstA.parameters.card_id);
                    if (card) cleanName = i18n.translateCard(card).name;
                }
                header.innerHTML = `<span class="truncate-name" style="max-width: 180px;">${cleanName}</span> <span class="header-base-cost">${energyIcon}${displayCost}</span>`;
                groupDiv.appendChild(header);

                const availableAreas = firstA.parameters?.available_areas;

                const doubleBatonPairs = firstA.parameters?.double_baton_pairs;
                const hasDoubleBaton = doubleBatonPairs && doubleBatonPairs.length > 0;
                const anySingleAvailable = availableAreas && availableAreas.some(a => a.available);

                if (anySingleAvailable || hasDoubleBaton) {
                    const areaLabels = { 'left': i18n.t('area_left'), 'center': i18n.t('area_center'), 'right': i18n.t('area_right') };
                    const areaOrder = ['left', 'center', 'right'];
                    const areaIndexMap = { 'left': 0, 'center': 1, 'right': 2 };
                    
                    if (anySingleAvailable) {
                        const areasDiv = document.createElement('div');
                        areasDiv.className = 'action-group-buttons';
                        
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
                                const costText = isBaton ? `${label} ${cost} Baton` : `${label} ${cost}`;
                                btn.innerHTML = `<span style="display:flex;flex-direction:column;align-items:center;gap:1px;font-weight:600;"><span style="font-size:0.8rem;">${label}</span><span style="font-size:0.65rem;opacity:0.7;">${cost}</span></span>`;
                                btn.dataset.zoneArea = areaName;
                                btn.style.cssText = '';
                                btn.className = btn.className + ' action-btn';
                                // Replace hover handlers: isolate to this button only (stage-slot hover removed — was buggy due to shared data-action-id)
                                btn.onmouseenter = () => {
                                    document.querySelectorAll('.action-btn.hover-highlight').forEach(s => s.classList.remove('hover-highlight'));
                                    btn.classList.add('hover-highlight');
                                };
                                btn.onmouseleave = () => {
                                    btn.classList.remove('hover-highlight');
                                };
                                areasDiv.appendChild(btn);
                            } else {
                                const spacer = document.createElement('div');
                                spacer.style.cssText = 'flex:1;min-height:36px;border:1px solid transparent;border-right:none;';
                                areasDiv.appendChild(spacer);
                            }
                        });
                        groupDiv.appendChild(areasDiv);
                    }

                    // Double Baton grid: render pair+placement buttons if available
                    if (hasDoubleBaton) {
                        const dbDiv = document.createElement('div');
                        dbDiv.style.cssText = 'margin-top: 6px; border-top: 1px dashed rgba(255, 215, 0, 0.3); background: rgba(0,0,0,0.15); padding: 6px; border-radius: 4px;';
                        
                        const dbLabel = document.createElement('div');
                        dbLabel.style.cssText = 'font-size: 0.7em; color: #ffda79; margin-bottom: 4px; font-weight: bold;';
                        dbLabel.textContent = i18n.t('double_baton') || 'DOUBLE BATON';
                        dbDiv.appendChild(dbLabel);

                        // Group pairs by their 2 replacement areas
                        const pairGroups = {};
                        doubleBatonPairs.forEach(pair => {
                            const key = pair.areas.sort().join('&');
                            if (!pairGroups[key]) { pairGroups[key] = []; }
                            pairGroups[key].push(pair);
                        });

                        Object.keys(pairGroups).forEach(key => {
                            const row = document.createElement('div');
                            row.className = 'action-group-buttons grid-3';
                            row.style.cssText = 'padding: 2px; border-radius: 4px; margin-top: 2px;';

                            const areas = key.split('&');
                            areaOrder.forEach(expectedArea => {
                                const pairForPlacement = pairGroups[key].find(p => p.placement === expectedArea);
                                if (pairForPlacement) {
                                    const labelA = areaLabels[areas[0]] || areas[0];
                                    const labelB = areaLabels[areas[1]] || areas[1];
                                    const placeLabel = areaLabels[expectedArea] || expectedArea;
                                    
                                    // Build action params
                                    const replaceIndices = areas.map(a => areaIndexMap[a]);
                                    const placement = areaIndexMap[expectedArea];
                                    
                                    const dbActionParams = {
                                        card_id: firstA.parameters?.card_id,
                                        card_index: firstA.parameters?.card_index,
                                        card_indices: replaceIndices,
                                        stage_area: expectedArea,
                                        use_baton_touch: true,
                                        card_name: firstA.parameters?.card_name,
                                        card_no: firstA.parameters?.card_no,
                                    };
                                    
                                    const btn = ActionButtons.createActionButton(
                                        { action_type: 'play_member_to_stage', parameters: dbActionParams },
                                        true, '', state
                                    );
                                    const costText = `${labelA}&${labelB} → ${placeLabel} (${pairForPlacement.cost} - Double)`;
                                    btn.innerHTML = `<span>${costText}</span>`;
                                    btn.style.width = '100%';
                                    btn.onclick = () => {
                                        if (window.doAction) window.doAction({ action_type: 'play_member_to_stage', parameters: dbActionParams });
                                    };
                                    row.appendChild(btn);
                                } else {
                                    const spacer = document.createElement('div');
                                    spacer.style.cssText = 'min-height: 30px; display: flex; align-items: center; justify-content: center; opacity: 0.2; font-size: 0.6em; border: 1px dashed rgba(255,255,255,0.1);';
                                    spacer.textContent = '--';
                                    row.appendChild(spacer);
                                }
                            });
                            dbDiv.appendChild(row);
                        });

                        groupDiv.appendChild(dbDiv);
                    }
                } else {
                    if (firstA) {
                        console.warn('[ActionListView] No available options for action:', firstA);
                    }
                }

                listDiv.appendChild(groupDiv);
            });
        }

        container.innerHTML = '';
        container.appendChild(listDiv);
    }
};
