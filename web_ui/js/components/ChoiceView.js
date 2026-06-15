import { State } from '../state.js';
import { Tooltips } from '../ui_tooltips.js';
import { CardRenderer, resolveCardImagePath } from './CardRenderer.js';
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

        let cardName = choice.card_name || choice.source_member;
        let cardId = choice.card_id !== undefined ? choice.card_id : (choice.source_card_id !== undefined ? choice.source_card_id : -1);

        if (!cardName || cardName === 'Unknown Source' || cardName === 'Unknown Card' || cardName.startsWith('Card ')) {
            const resolvedCard = State.resolveCardData(cardId);
            if (resolvedCard && resolvedCard.name) cardName = resolvedCard.name;
            else cardName = i18n.t('unknown_card');
        }

        let headerText = cardName;
        if (choice.card_no) headerText += ` <span style="opacity:0.5;font-size:0.8em;">[${choice.card_no}]</span>`;
        else if (cardId >= 0) headerText += ` <span style="opacity:0.6;font-size:0.8em;">(ID: ${cardId})</span>`;

        let content = `<div class="choice-header" style="color:${headerColor};">${headerText}</div>`;

        let abilityText = choice.ability_text || '';
        if (!abilityText || abilityText.length < 5) {
            if (cardId >= 0) {
                const card = State.resolveCardData(cardId);
                const naturalText = Tooltips.extractRelevantAbility(card, choice.trigger_label, choice.ability_index);
                if (naturalText && !Tooltips.isGenericInstruction(naturalText)) abilityText = naturalText;
            }
        }
        if (!abilityText || abilityText.length < 5) {
            const fallback = choice.source_ability || '';
            const isGenericChoice = Tooltips.isGenericInstruction(choice.choice_text);
            if (fallback && fallback.length > 5 && !Tooltips.isGenericInstruction(fallback) && !isGenericChoice) abilityText = fallback;
        }

        if (abilityText && abilityText.length > 5 && !Tooltips.isGenericInstruction(abilityText)) {
            const blocks = Tooltips.splitAbilities ? Tooltips.splitAbilities(abilityText) : [abilityText];
            blocks.forEach(block => content += `<div class="source-ability-text">${Tooltips.enrichAbilityText(block)}</div>`);
        }

        choiceDiv.innerHTML = content;
        let hasContent = false;

        if (choice.choice_type === 29) {
            const confirmBtn = document.createElement('button');
            confirmBtn.className = 'btn action-btn confirm';
            confirmBtn.style.cssText = 'width:100%;margin-top:10px;';
            confirmBtn.innerHTML = i18n.t('confirm_formation');
            confirmBtn.onclick = () => {
                const pIdx = State.perspectivePlayer;
                if (!State.rawData || (!State.rawData.player1 && !State.rawData.player2)) { console.warn('[ChoiceView] rawData not available'); return; }
                const oldPlayer = pIdx === 0 ? State.rawData.player1 : State.rawData.player2;
                const newPlayer = pIdx === 0 ? state.player1 : state.player2;
                const oldStage = [oldPlayer.stage.left_side, oldPlayer.stage.center, oldPlayer.stage.right_side];
                const newStage = [newPlayer.stage.left_side, newPlayer.stage.center, newPlayer.stage.right_side];
                const perms = [[0,1,2],[0,2,1],[1,0,2],[1,2,0],[2,0,1],[2,1,0]];
                let permIdx = 0;
                for (let i = 0; i < perms.length; i++) {
                    const p = perms[i];
                    if (newStage[0] === oldStage[p[0]] && newStage[1] === oldStage[p[1]] && newStage[2] === oldStage[p[2]]) { permIdx = i; break; }
                }
                if (window.doAction) window.doAction(permIdx);
            };
            choiceDiv.appendChild(confirmBtn);
            hasContent = true;
        } else {
            // Build options: from choice.options (WASM-style), selection_cards, or legal_actions
            const optContainer = document.createElement('div');
            optContainer.className = 'choice-cards-row';

            const optItems = [];

            // Check for SelectAutoAbility choice (Rule 9.5.3)
            if (choice.title && choice.title.includes('auto ability resolves first')) {
                state.legal_actions && state.legal_actions.forEach(a => {
                    const cardName = a.parameters?.card_no || a.parameters?.card_name || a.description || 'Ability';
                    const abilityText = a.parameters?.card_name ? '' : '';
                    optItems.push({
                        card: null,
                        name: cardName,
                        action: a,
                    });
                });
            } else if (choice.options && choice.options.length > 0) {
                // Check if options are plain strings (heart colors, etc.) vs WASM objects
                const isStringOptions = typeof choice.options[0] === 'string';
                if (isStringOptions) {
                    // String options: render as clickable buttons
                    state.legal_actions && state.legal_actions.forEach(a => {
                        const cardNo = a.parameters?.card_no;
                        const name = cardNo && cardNo.startsWith('heart') 
                            ? cardNo.replace('heart0', '♥').replace('heart', '♥') 
                            : (a.parameters?.card_name || a.description || '');
                        optItems.push({ card: null, name, action: a });
                    });
                } else {
                    // WASM-style options with action IDs
                    choice.options.forEach((opt, idx) => {
                        const actionId = choice.actions?.[idx];
                        if (actionId === undefined || actionId === null || actionId === 0) return;
                        const optCardId = opt.card_id !== undefined ? opt.card_id : null;
                        const fallbackName = opt.name || opt.text || `Option ${idx + 1}`;
                        const optCard = optCardId !== null ? (State.resolveCardData(optCardId) || Tooltips.findCardById(optCardId)) : null;
                        optItems.push({ card: optCard, name: fallbackName, action: { index: actionId } });
                    });
                }
            } else if (choice.selection_cards && choice.selection_cards.length > 0 && state.legal_actions) {
                // REST-style: selection_cards + legal_actions
                const cardByNo = {};
                choice.selection_cards.forEach(sc => { cardByNo[sc.card_no] = sc; });
                state.legal_actions.forEach(a => {
                    if (a.action_type !== 'select_card' && a.action_type !== 'select_skip') return;
                    const cardNo = a.parameters?.card_no;
                    const cardData = cardNo ? State.resolveCardData(cardNo) : null;
                    const name = cardData?.name || a.parameters?.card_name || a.description || '';
                    const isTextAction = cardNo && !cardData && (
                        cardNo === 'yes' || cardNo === 'no' || cardNo === 'skip' ||
                        cardNo === 'pay_optional_cost' || cardNo === 'skip_optional_cost'
                    );
                    optItems.push({ card: cardData, name, action: a, isText: isTextAction });
                });
            } else if (state.legal_actions && state.legal_actions.length > 0) {
                // Fallback: show legal_actions as text buttons or cards
                state.legal_actions.forEach(a => {
                    if (a.action_type === 'Pass') return;
                    const cardNo = a.parameters?.card_no;
                    const cardData = cardNo ? State.resolveCardData(cardNo) : null;
                    // Text-only action (yes/no/skip/digit) → render as text button, not empty card
                    const isTextAction = cardNo && !cardData && (
                        cardNo === 'yes' || cardNo === 'no' || cardNo === 'skip' ||
                        cardNo === 'pay_optional_cost' || cardNo === 'skip_optional_cost' ||
                        cardNo === 'primary' || cardNo === 'alternative' || /^\d+$/.test(cardNo)
                    );
                    const name = cardData?.name || a.parameters?.card_name || a.description || '';
                    optItems.push({ card: cardData, name, action: a, isText: isTextAction });
                });
            }

            // Deduplicate by action index
            const seen = new Set();
            const unique = optItems.filter(item => {
                const key = item.action.index || JSON.stringify(item.action);
                if (seen.has(key)) return false;
                seen.add(key);
                return true;
            });

            unique.forEach(item => {
                const cardData = item.card;
                const cardEl = document.createElement('div');
                cardEl.className = 'compact-choice-card';
                cardEl.title = item.name;

                // Text-only action: render as clickable button, not empty card
                if (item.isText) {
                    cardEl.className += ' text-option';
                    cardEl.style.cssText = `
                        display: flex;
                        align-items: center;
                        justify-content: center;
                        padding: 12px 16px;
                        min-width: 80px;
                        min-height: 48px;
                        cursor: pointer;
                        font-size: 0.95rem;
                        background: var(--input-bg, #2a2a3a);
                        border: 2px solid var(--border, #555);
                        border-radius: 8px;
                        color: var(--text, #eee);
                    `;
                    cardEl.textContent = item.name;
                } else if (item.name.startsWith('♥') || item.name.match(/heart\d{2}/)) {
                    const heartIdx = parseInt(item.name.replace(/\D/g, '')) || 1;
                    cardEl.className += ' has-image heart-option';
                    cardEl.style.cssText = `
                        background: none;
                        border: 2px solid var(--border);
                        display: flex;
                        align-items: center;
                        justify-content: center;
                        font-size: 2rem;
                        min-width: 64px;
                        min-height: 64px;
                        cursor: pointer;
                    `;
                    const heartImg = document.createElement('img');
                    heartImg.src = `img/texticon/heart_0${heartIdx}.png`;
                    heartImg.className = 'heart-mini-icon';
                    heartImg.style.width = '48px';
                    heartImg.style.height = '48px';
                    cardEl.innerHTML = '';
                    cardEl.appendChild(heartImg);
                } else if (cardData && cardData.card_no) {
                    const imgSrc = resolveCardImagePath(cardData.card_no);
                    if (imgSrc) {
                        cardEl.style.backgroundImage = `url(${imgSrc})`;
                        cardEl.className += ' has-image';
                    }
                }

                cardEl.onclick = () => {
                    if (window.doAction) window.doAction(item.action);
                };
                optContainer.appendChild(cardEl);
            });

            if (unique.length > 0) {
                choiceDiv.appendChild(optContainer);
                hasContent = true;
            }
        }

        if (hasContent) container.appendChild(choiceDiv);
    }
};
