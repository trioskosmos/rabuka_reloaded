import { State } from '../state.js';
import { Tooltips } from '../ui_tooltips.js';
import { CardRenderer, resolveCardImagePath, ImageLoader } from './CardRenderer.js';
import { fixImg } from '../constants.js';
import * as i18n from '../i18n/index.js';

export const ChoiceView = {
    render: (state, container) => {
        const choice = state.pending_choice;

        // G2: in PVP mode, if the choice is for the opponent, show a waiting indicator.
        // In sandbox mode, show all choices with a player label — one person makes all decisions.
        if (choice && state?.mode === 'pvp') {
            const viewerPlayerId = `p${State.perspectivePlayer + 1}`;
            if (choice.choice_player_id && choice.choice_player_id !== viewerPlayerId) {
                const waitDiv = document.createElement('div');
                waitDiv.className = 'pending-choice-indicator';
                waitDiv.innerHTML = `<div style="font-weight:bold; color:#ffcc00; padding:20px; text-align:center;">Waiting for opponent's choice...</div>`;
                container.appendChild(waitDiv);
                return;
            }
        }

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
            const resolvedCard = cardId >= 0 ? State.resolveCardData(cardId) : null;
            if (resolvedCard && resolvedCard.name) {
                cardName = resolvedCard.name;
            } else if (choice.options?.[0]?.card_name) {
                cardName = choice.options[0].card_name;
            } else {
                cardName = i18n.t('unknown_card');
            }
        }

        let headerText = cardName;
        if (state?.mode !== 'pvp' && choice.choice_player_id) {
            const label = choice.choice_player_id === 'p2' ? 'P2' : 'P1';
            headerText = `[${label}] ${headerText}`;
        }
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
            blocks.forEach(block => {
                let displayText = block;
                if (State.currentLang === 'en' && window.translateAbility) {
                    displayText = window.translateAbility(displayText, 'en');
                }
                content += `<div class="source-ability-text">${Tooltips.enrichAbilityText(displayText)}</div>`;
            });
        }

        // Render prompt/instruction between ability text and options
        const prompt = State.currentLang === 'en'
            ? (choice.prompt_en || choice.title || '')
            : (choice.prompt_ja || choice.title || '');
        if (prompt) {
            const displayPrompt = State.currentLang === 'en' && window.translateAbility
                ? window.translateAbility(prompt, 'en')
                : prompt;
            content += `<div class="choice-prompt">${Tooltips.enrichAbilityText(displayPrompt)}</div>`;
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
            // The choice's options array has ability_text fields, distinguishing
            // it from WASM-style card options.
            if (choice.options && choice.options.length > 0 && choice.options[0].ability_text) {
                choice.options.forEach((opt, idx) => {
                    const cardName = opt.card_name || `Ability ${idx + 1}`;
                    const abilityText = opt.ability_text || '';
                    const optCard = opt.card_id !== undefined ? (State.resolveCardData(opt.card_id) || Tooltips.findCardById(opt.card_id)) : (opt.card_no ? State.resolveCardData(opt.card_no) : null);
                    const action = state.legal_actions?.find(a => {
                        return a.parameters?.card_id === idx || a.description?.startsWith(cardName);
                    });
                    optItems.push({
                        card: optCard,
                        name: cardName,
                        desc: abilityText,
                        action: action || { index: idx },
                    });
                });
            } else if (choice.options && choice.options.length > 0 && choice.options[0].card_index !== undefined) {
                // SelectLiveSuccess: options have card_name + card_index, not ability_text
                choice.options.forEach((opt, idx) => {
                    optItems.push({
                        card: null,
                        name: opt.card_name || `Card ${idx + 1}`,
                        action: { action_type: 'select_card', parameters: { card_indices: [idx] } },
                        isText: true,
                    });
                });
            } else if (choice.options && choice.options.length > 0) {
                // Check if options are plain strings (heart colors, etc.) vs WASM objects
                const isStringOptions = typeof choice.options[0] === 'string';
                if (isStringOptions) {
                    state.legal_actions && state.legal_actions.forEach(a => {
                        const cardNo = a.parameters?.card_no;
                        const isHeart = cardNo && cardNo.startsWith('heart');
                        const optIdx = a.parameters?.card_id;
                        const optText = optIdx !== undefined && choice.options[optIdx] ? choice.options[optIdx] : null;
                        const name = isHeart
                            ? cardNo.replace('heart0', '♥').replace('heart', '♥')
                            : (optText || a.parameters?.card_name || a.description || '');
                        optItems.push({ card: null, name, action: a, isText: !isHeart });
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
                    const resolved = cardNo ? State.resolveCardData(cardNo) : null;
                    const cardData = cardNo ? (resolved || cardByNo[cardNo] || null) : null;
                    const name = cardData?.name || a.parameters?.card_name || a.description || '';
                     const isTextAction = cardNo === '-1' || (cardNo && !cardData);
                    optItems.push({ card: cardData, name, action: a, isText: isTextAction });
                });
            } else if (state.legal_actions && state.legal_actions.length > 0) {
                // Fallback: show legal_actions as text buttons or cards
                state.legal_actions.forEach(a => {
                    if (a.action_type === 'Pass') return;
                    const cardNo = a.parameters?.card_no;
                    const resolved = cardNo ? State.resolveCardData(cardNo) : null;
                    const cardData = cardNo ? resolved : null;
                    // Text-only action (yes/no/skip/digit) → render as text button, not empty card
                     const isTextAction = cardNo === '-1' || (cardNo && !cardData);
                    let name = cardData?.name || a.parameters?.card_name || a.description || '';
                    if (!name || name.startsWith('ACT_') || name.startsWith('CHOOSE_')) {
                        const ACTION_LABELS = {
                            'PlayMemberToStage': i18n.t('set_deck'),
                            'play_member_to_stage': i18n.t('set_deck'),
                            'UseAbility': i18n.t('act_ability'),
                            'use_ability': i18n.t('act_ability'),
                            'SetLiveCard': i18n.t('live_card_set'),
                            'set_live_card': i18n.t('live_card_set'),
                            'EnergyCharge': i18n.t('energy'),
                            'energy_charge': i18n.t('energy'),
                            'Pass': i18n.t('pass_no'),
                            'pass': i18n.t('pass_no'),
                            'pass_remaining': i18n.t('pass_no'),
                            'SkipMulligan': i18n.t('skip'),
                            'skip_mulligan': i18n.t('skip'),
                            'Decision': i18n.t('done'),
                            'decision': i18n.t('done'),
                            'ChooseOption': i18n.t('select'),
                            'choose_option': i18n.t('select'),
                            'SelectMulligan': i18n.t('mulligan'),
                            'select_mulligan': i18n.t('mulligan'),
                            'RockChoice': i18n.t('rps_rock'),
                            'rock_choice': i18n.t('rps_rock'),
                            'PaperChoice': i18n.t('rps_paper'),
                            'paper_choice': i18n.t('rps_paper'),
                            'ScissorsChoice': i18n.t('rps_scissors'),
                            'scissors_choice': i18n.t('rps_scissors'),
                            'ChooseFirstAttacker': i18n.t('go_first'),
                            'choose_first_attacker': i18n.t('go_first'),
                            'ChooseSecondAttacker': i18n.t('go_second'),
                            'choose_second_attacker': i18n.t('go_second'),
                            'ConfirmMulligan': i18n.t('confirm'),
                            'confirm_mulligan': i18n.t('confirm'),
                            'FinishLiveCardSet': i18n.t('finish_live_card_set'),
                            'finish_live_card_set': i18n.t('finish_live_card_set'),
                            'SelectPosition': i18n.t('select_position'),
                            'select_position': i18n.t('select_position'),
                            'SelectCard': i18n.t('select'),
                            'select_card': i18n.t('select'),
                            'SelectSkip': i18n.t('skip'),
                            'select_skip': i18n.t('skip'),
                        };
                        name = ACTION_LABELS[a.action_type] || a.action_type || '';
                    }
                    // Special card_no-based text actions — use locale
                    if (cardNo === 'pay_optional_cost') {
                        name = i18n.t('pay_optional_cost');
                    } else if (cardNo === 'skip_optional_cost') {
                        name = i18n.t('skip');
                    }
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

                // Auto-ability option: show card image + name + ability text
                if (item.desc) {
                    cardEl.className += ' text-option auto-ability';
                    cardEl.style.cssText = `
                        display: flex;
                        flex-direction: row;
                        align-items: center;
                        gap: 8px;
                        padding: 4px 8px;
                        width: 100%;
                        box-sizing: border-box;
                        cursor: pointer;
                        font-size: 0.9rem;
                        background: var(--input-bg, #2a2a3a);
                        border: 1px solid var(--accent-purple, #9966ff);
                        border-radius: 6px;
                        color: var(--text, #eee);
                        text-align: left;
                    `;
                    if (cardData && cardData.card_no) {
                        const resolvedForType = State.resolveCardData(cardData.card_no) || cardData;
                        const rawType = (resolvedForType.type || resolvedForType.card_type || '').toLowerCase();
                        const isLive = rawType === 'live' || rawType === 'ライブ' ||
                            (typeof cardData.card_no === 'string' && cardData.card_no.startsWith('live'));
                        const imgSrc = resolveCardImagePath(cardData.card_no);
                        if (imgSrc) {
                            const imgWrap = document.createElement('div');
                            imgWrap.className = 'compact-choice-card' + (isLive ? ' type-live orientation-landscape' : '');
                            if (isLive) {
                                imgWrap.style.setProperty('width', '84px', 'important');
                                imgWrap.style.setProperty('height', '60px', 'important');
                            }
                            const img = document.createElement('img');
                            img.draggable = false;
                            let fallbackEl = null;
                            const showFallback = () => {
                                if (fallbackEl) return;
                                img.style.display = 'none';
                                fallbackEl = document.createElement('div');
                                fallbackEl.style.cssText = 'display:flex;align-items:center;justify-content:center;width:100%;height:100%;font-size:0.55rem;color:var(--text-dim);text-align:center;padding:2px;overflow:hidden;word-break:break-all;';
                                fallbackEl.textContent = cardData.name || cardData.card_no || '?';
                                imgWrap.appendChild(fallbackEl);
                            };
                            const hideFallback = () => {
                                if (fallbackEl) { fallbackEl.remove(); fallbackEl = null; }
                                img.style.display = '';
                            };
                            img.addEventListener('imagePermanentFailure', showFallback);
                            img.addEventListener('imageRetrying', hideFallback);
                            img.addEventListener('load', hideFallback);
                            imgWrap.appendChild(img);
                            ImageLoader.loadImage(img, imgSrc);
                            cardEl.appendChild(imgWrap);
                        }
                        Tooltips.attachCardData(cardEl, cardData);
                    }
                    const textWrap = document.createElement('div');
                    textWrap.style.cssText = 'flex:1;min-width:0;display:flex;flex-direction:column;gap:2px;';
                    textWrap.innerHTML = `
                        <div style="font-weight:bold;color:#cc88ff;font-size:0.85rem;">${Tooltips.enrichAbilityText(item.name)}</div>
                        <div style="opacity:0.8;font-size:0.75rem;line-height:1.3;">${Tooltips.enrichAbilityText(item.desc)}</div>
                    `;
                    cardEl.appendChild(textWrap);
                } else if (item.isText) {
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
                    if (choice.blind) {
                        // Blind pick: show card back, not the actual card face
                        cardEl.style.cssText = `
                            display: flex;
                            align-items: center;
                            justify-content: center;
                            width: 60px;
                            height: 84px;
                            border-radius: 4px;
                            border: 2px solid var(--accent-gold, #d4a843);
                            background: linear-gradient(135deg, #2a2a3a 0%, #1a1a2e 50%, #2a2a3a 100%);
                            cursor: pointer;
                            font-size: 1.6rem;
                            color: var(--accent-gold, #d4a843);
                            flex-shrink: 0;
                        `;
                        cardEl.textContent = '?';
                        cardEl.className += ' has-image blind-card';
                        cardEl.title = '??? (blind pick)';
                    } else {
                        // Re-resolve by card_no for proper type (selection_cards skips it)
                        const resolvedForType = cardData.card_no ? (State.resolveCardData(cardData.card_no) || cardData) : cardData;
                        const rawType = (resolvedForType.type || resolvedForType.card_type || '').toLowerCase();
                        const isLive = rawType === 'live' || rawType === 'ライブ' ||
                            (typeof cardData.card_no === 'string' && cardData.card_no.startsWith('live'));
                        let imgSrc = resolveCardImagePath(cardData.card_no);
                        if (!imgSrc) {
                            imgSrc = fixImg(cardData.img || cardData.img_path || cardData.image || '');
                        }
                        if (imgSrc) {
                            cardEl.className += ' has-image';
                            const img = document.createElement('img');
                            img.draggable = false;
                            let fallbackEl = null;
                            const showFallback = () => {
                                if (fallbackEl) return;
                                img.style.display = 'none';
                                fallbackEl = document.createElement('div');
                                fallbackEl.style.cssText = 'display:flex;align-items:center;justify-content:center;width:100%;height:100%;font-size:0.55rem;color:var(--text-dim);text-align:center;padding:2px;overflow:hidden;word-break:break-all;';
                                fallbackEl.textContent = cardData.name || cardData.card_no || '?';
                                cardEl.appendChild(fallbackEl);
                            };
                            const hideFallback = () => {
                                if (fallbackEl) { fallbackEl.remove(); fallbackEl = null; }
                                img.style.display = '';
                            };
                            img.addEventListener('imagePermanentFailure', showFallback);
                            img.addEventListener('imageRetrying', hideFallback);
                            img.addEventListener('load', hideFallback);
                            cardEl.appendChild(img);
                            ImageLoader.loadImage(img, imgSrc);
                            if (isLive) {
                                cardEl.style.setProperty('width', '84px', 'important');
                                cardEl.style.setProperty('height', '60px', 'important');
                                cardEl.classList.add('type-live', 'orientation-landscape');
                            }
                        } else {
                            const fb = document.createElement('div');
                            fb.style.cssText = 'display:flex;align-items:center;justify-content:center;width:100%;height:100%;font-size:0.55rem;color:var(--text-dim);text-align:center;padding:2px;overflow:hidden;word-break:break-all;';
                            fb.textContent = cardData.name || cardData.card_no || '?';
                            cardEl.appendChild(fb);
                        }
                        Tooltips.attachCardData(cardEl, cardData);
                    }
                } else {
                    cardEl.className += ' text-option';
                    cardEl.style.cssText = `
                        display: flex;
                        align-items: center;
                        justify-content: center;
                        padding: 8px 12px;
                        width: 100%;
                        font-size: 0.85rem;
                        background: var(--input-bg, #2a2a3a);
                        border: 1px solid var(--border, #555);
                        border-radius: 6px;
                        color: var(--text, #eee);
                        box-sizing: border-box;
                        height: auto;
                    `;
                    cardEl.textContent = item.name;
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
