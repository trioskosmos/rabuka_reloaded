import { State } from '../state.js';
import { ActionButtons } from './ActionButtons.js';
import { Tooltips } from '../ui_tooltips.js';
import * as i18n from '../i18n/index.js';
export const ChoiceView = {
    render: (state, container) => {
        const choice = state.pending_choice;
        const choiceDiv = document.createElement('div');
        choiceDiv.className = 'pending-choice-indicator';

        const opcode = choice.opcode || (state.legal_actions && state.legal_actions[0] && state.legal_actions[0].opcode);
        let headerColor = 'var(--accent-gold)';
        if (opcode === 58) headerColor = '#ff4d4d';
        else if (opcode === 15 || opcode === 17 || opcode === 63 || opcode === 30) headerColor = '#4da6ff';
        else if (opcode === 45) headerColor = '#ffcc00';
        else if (opcode === 41 || opcode === 74) headerColor = '#9966ff';

        choiceDiv.style.borderLeft = `4px solid ${headerColor}`;

        const cardId = choice.card_id !== undefined ? choice.card_id : (choice.source_card_id !== undefined ? choice.source_card_id : -1);
        let cardName = choice.source_member;

        if (!cardName || cardName === 'Unknown Source' || cardName === 'Unknown Card' || cardName.startsWith('Card ')) {
            const resolvedCard = State.resolveCardData(cardId);
            if (resolvedCard && resolvedCard.name) {
                cardName = resolvedCard.name;
            } else {
                cardName = i18n.t('unknown_card');
            }
        }

        let headerText = cardName;
        if (cardId >= 0) {
            headerText += ` <span style="opacity:0.6; font-size:0.8em;">(ID: ${cardId})</span>`;
        }

        let content = `<div class="choice-header" style="color:${headerColor};">${headerText}</div>`;

        let abilityText = "";
        if (cardId >= 0) {
            const card = State.resolveCardData(cardId);
            const naturalText = Tooltips.extractRelevantAbility(card, choice.trigger_label, choice.ability_index);
            if (naturalText && !Tooltips.isGenericInstruction(naturalText)) {
                abilityText = naturalText;
            }
        }

        if (!abilityText || abilityText.length < 5) {
            const fallback = choice.source_ability || "";
            const isGenericChoice = Tooltips.isGenericInstruction(choice.choice_text);
            if (fallback && fallback.length > 5 && !Tooltips.isGenericInstruction(fallback) && !isGenericChoice) {
                abilityText = fallback;
            }
        }

        if (abilityText && abilityText.length > 5 && !Tooltips.isGenericInstruction(abilityText)) {
            const blocks = Tooltips.splitAbilities ? Tooltips.splitAbilities(abilityText) : [abilityText];
            blocks.forEach(block => {
                const enriched = Tooltips.enrichAbilityText(block);
                content += `<div class="source-ability-text">${enriched}</div>`;
            });
        }

        choiceDiv.innerHTML = content;

        let hasContent = false;

        if (choice.choice_type === 29) { // REARRANGE_FORMATION
            const confirmBtn = document.createElement('button');
            confirmBtn.className = 'btn action-btn confirm';
            confirmBtn.style.width = '100%';
            confirmBtn.style.marginTop = '10px';
            confirmBtn.innerHTML = i18n.t('confirm_formation');

            confirmBtn.onclick = () => {
                const pIdx = State.perspectivePlayer;
                // Rust backend format: player1, player2
                if (!State.rawData || (!State.rawData.player1 && !State.rawData.player2)) {
                    console.warn('[ChoiceView] rawData not available for REARRANGE_FORMATION');
                    return;
                }
                const oldPlayer = pIdx === 0 ? State.rawData.player1 : State.rawData.player2;
                const newPlayer = pIdx === 0 ? state.player1 : state.player2;
                
                // Rust backend: stage is { left_side, center, right_side }
                const oldStage = [oldPlayer.stage.left_side, oldPlayer.stage.center, oldPlayer.stage.right_side];
                const newStage = [newPlayer.stage.left_side, newPlayer.stage.center, newPlayer.stage.right_side];

                const perms = [
                    [0, 1, 2], [0, 2, 1], [1, 0, 2], [1, 2, 0], [2, 0, 1], [2, 1, 0]
                ];
                let permIdx = 0;
                for (let i = 0; i < perms.length; i++) {
                    const p = perms[i];
                    if (newStage[0] === oldStage[p[0]] &&
                        newStage[1] === oldStage[p[1]] &&
                        newStage[2] === oldStage[p[2]]) {
                        permIdx = i;
                        break;
                    }
                }
                if (window.doAction) window.doAction(permIdx);
            };
            choiceDiv.appendChild(confirmBtn);
            hasContent = true;
        } else if (choice.options && choice.options.length > 0) {
            const optContainer = document.createElement('div');
            optContainer.className = 'action-list choice-options-container';
            const sourceCardId = choice.source_card_id !== undefined ? choice.source_card_id : cardId;

            choice.options.forEach((opt, idx) => {
                const actionId = choice.actions?.[idx];
                if (actionId === undefined || actionId === null || actionId === 0) {
                    return;
                }
                const optCardId = opt.card_id !== undefined ? opt.card_id : cardId;
                const fallbackName = opt.name || opt.text || `Option ${idx + 1}`;
                const a = {
                    index: actionId,
                    source_card_id: sourceCardId,
                    card_id: optCardId,
                    name: fallbackName,
                    text: opt.text,
                    metadata: {
                        category: 'CHOICE',
                        choice_type: choice.choice_type,
                        opcode: choice.opcode,
                        choice_idx: idx,
                    }
                };
                const btn = ActionButtons.createActionButton(a, false, 'confirm', state);
                btn.style.width = '100%';
                optContainer.appendChild(btn);
            });
            choiceDiv.appendChild(optContainer);
            hasContent = true;
        }
        
        if (hasContent) {
            container.appendChild(choiceDiv);
        }
    }
};
