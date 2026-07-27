import { State } from '../state.js';
import { fixImg } from '../constants.js';
import { Tooltips } from '../ui_tooltips.js';
import { CardRenderer } from './CardRenderer.js';
import { ModalManager } from '../utils/ModalManager.js';
import { DOM_IDS } from '../constants_dom.js';
import * as i18n from '../i18n/index.js';

let _selViewBtn = null;
let _selSelectBtn = null;
let _choiceCards = [];
let _choiceIndex = -1;

function _syncSelNav() {
    if (!_selViewBtn) _selViewBtn = document.getElementById('selection-view-card-btn');
    if (!_selSelectBtn) _selSelectBtn = document.getElementById('selection-select-btn');
    const hasCard = _choiceIndex >= 0 && _choiceIndex < _choiceCards.length;
    if (_selViewBtn) _selViewBtn.disabled = !hasCard;
    if (_selSelectBtn) _selSelectBtn.disabled = !hasCard;
}

function _highlightChoice() {
    const content = document.getElementById(DOM_IDS.SELECTION_CONTENT);
    if (content) content.querySelectorAll('.choice-item.selected, .card.selected, .card-choice.selected').forEach(el => el.classList.remove('selected'));
    if (_choiceIndex >= 0 && _choiceIndex < _choiceCards.length) {
        const entry = _choiceCards[_choiceIndex];
        if (entry?.el) {
            entry.el.classList.add('selected');
            entry.el.scrollIntoView({ block: 'nearest', behavior: 'smooth' });
        }
    }
    _syncSelNav();
}

function _selectByIndex(index) {
    if (index < 0 || index >= _choiceCards.length) return;
    _choiceIndex = index;
    _highlightChoice();
}

function _openChoiceDetail() {
    if (_choiceIndex < 0 || _choiceIndex >= _choiceCards.length) return;
    const entry = _choiceCards[_choiceIndex];
    const card = entry?.card;
    if (!card) return;
    const m = window.__modals?.CardDetailModal;
    if (m) m.open(card, _selectCurrentChoice);
}

function _selectCurrentChoice() {
    if (_choiceIndex < 0 || _choiceIndex >= _choiceCards.length) return;
    const entry = _choiceCards[_choiceIndex];
    const action = entry?.action;
    ModalManager.hide(DOM_IDS.SELECTION_MODAL);
    if (action && window.doAction) window.doAction(action);
}

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
    'SelectCard': i18n.t('select'),
    'select_card': i18n.t('select'),
    'SelectSkip': i18n.t('skip'),
    'select_skip': i18n.t('skip'),
};

function _resolveActionLabel(action) {
    const name = action.parameters?.card_name || action.description || '';
    if (!name || name.startsWith('ACT_') || name.startsWith('CHOOSE_')) {
        return ACTION_LABELS[action.action_type] || action.action_type || '';
    }
    return name;
}

function _resolveSpecialLabel(cardNo, name) {
    if (cardNo === 'pay_optional_cost') return i18n.t('pay_optional_cost');
    if (cardNo === 'skip_optional_cost') return i18n.t('skip');
    if (cardNo === 'primary') return i18n.t('primary_option');
    if (cardNo === 'alternative') return i18n.t('alternative_option');
    if (cardNo === 'yes') return i18n.t('yes_label');
    if (cardNo === 'no') return i18n.t('no_label');
    if (name.startsWith('Draw ')) {
        const m = name.match(/^Draw (\d+)/);
        if (m) return parseInt(m[1]) === 0 ? i18n.t('draw_skip') : i18n.t('draw_count', { count: parseInt(m[1]) });
    }
    if (name.startsWith('Move card ')) {
        const m = name.match(/^Move card (\d+) to top/);
        if (m) return i18n.t('move_to_top', { n: m[1] });
    }
    if (name === "Skip (don't change position)") return i18n.t('skip');
    if (name === 'Apply replacement') return i18n.t('apply_replacement');
    if (name === "Don't apply") return i18n.t('dont_apply');
    return name;
}

function _localizePositionLabel(action, name) {
    if ((action.action_type === 'select_position' || action.action_type === 'SelectPosition') && name && State.currentLang === 'jp') {
        return name
            .replace('Left', i18n.t('area_left'))
            .replace('Center', i18n.t('area_center'))
            .replace('Right', i18n.t('area_right'))
            .replace('Select ', '')
            .replace('Move to ', '');
    }
    return name;
}

const STAGE_POSITIONS = [
    { key: 'left', areaKey: 'left_side', labelEn: 'Left', labelJp: '左' },
    { key: 'center', areaKey: 'center', labelEn: 'Center', labelJp: '中央' },
    { key: 'right', areaKey: 'right_side', labelEn: 'Right', labelJp: '右' },
];

function _posLabel(pos) {
    return State.currentLang === 'jp' ? pos.labelJp : pos.labelEn;
}

function _isPositionItem(item) {
    const at = item.action?.action_type;
    return at === 'select_position' || at === 'SelectPosition';
}

function _renderStageChoice(items, choice) {
    const container = document.createElement('div');
    container.className = 'stage-choice-container';

    const allItems = items.filter(_isPositionItem);
    if (allItems.length === 0) return null;

    let sourceCard = null;
    const srcId = choice?.source_card_id ?? choice?.card_id;
    if (srcId !== undefined && srcId >= 0) {
        sourceCard = State.resolveCardData(srcId);
    }
    if (!sourceCard && choice?.card_no) {
        sourceCard = State.resolveCardData(choice.card_no);
    }
    if (!sourceCard && choice?.source_member && State.resolveCardDataByName) {
        sourceCard = State.resolveCardDataByName(choice.source_member);
    }
    if (sourceCard) {
        const srcRow = document.createElement('div');
        srcRow.className = 'stage-choice-source';
        const thumb = CardRenderer.createCardDOM(
            CardRenderer.getCardViewModel(sourceCard, { mini: true }),
            sourceCard
        );
        thumb.classList.add('card-mini');
        srcRow.appendChild(thumb);
        const arrow = document.createElement('span');
        arrow.className = 'stage-choice-arrow';
        arrow.textContent = '↓';
        srcRow.appendChild(arrow);
        container.appendChild(srcRow);
    }

    // Determine player targeting from choice-level context
    const choiceText = (choice?.prompt_en || choice?.description || choice?.title || '');
    const allOpponent = /opponent/i.test(choiceText) && !/your|self/i.test(choiceText);

    function parsePlayerAndPos(item) {
        const params = item.action?.parameters || {};
        const area = params.stage_area || '';
        const raw = area.replace('_side', '');
        const idx = STAGE_POSITIONS.findIndex(p => p.key === raw);
        if (idx < 0) return null;
        const optIdx = params.card_id;
        const optText = (optIdx !== undefined && choice?.options?.[optIdx]) ? String(choice.options[optIdx]) : '';
        const prefixed = optText.startsWith('opponent:') || optText.startsWith('self:');
        const isOpp = prefixed ? optText.startsWith('opponent:') : allOpponent;
        return { player: isOpp ? 1 : 0, posKey: STAGE_POSITIONS[idx].key, slotIdx: idx, item };
    }

    const parsed = allItems.map(parsePlayerAndPos).filter(p => p !== null);
    const playerGroups = {};
    for (const p of parsed) {
        if (!playerGroups[p.player]) playerGroups[p.player] = {};
        playerGroups[p.player][p.posKey] = p.item;
    }

    const pp = State.perspectivePlayer;
    const state = State.data;

    for (const [playerStr, posMap] of Object.entries(playerGroups)) {
        const playerIdx = parseInt(playerStr);
        const player = playerIdx === 0 ? state?.player1 : state?.player2;
        const stage = player?.stage;
        if (!stage) continue;

        const stageBlock = document.createElement('div');
        stageBlock.className = 'stage-choice-block';

        const isSelf = (pp === 0 && playerIdx === 0) || (pp === 1 && playerIdx === 1);
        const titleEl = document.createElement('div');
        titleEl.className = 'stage-choice-player-title';
        titleEl.textContent = isSelf ? 'Your Stage' : 'Opponent Stage';
        stageBlock.appendChild(titleEl);

        const row = document.createElement('div');
        row.className = 'stage-choice-row';

        for (const pos of STAGE_POSITIONS) {
            const slotData = stage[pos.areaKey];
            const actionItem = posMap[pos.key];
            const isValid = !!actionItem;
            const occupied = slotData && slotData.card_no && slotData.card_no !== -1;

            const slot = document.createElement('div');
            slot.className = 'stage-choice-slot' + (isValid ? ' valid' : '') + (occupied ? ' occupied' : '');

            if (occupied) {
                const vm = CardRenderer.getCardViewModel(slotData, { mini: true });
                const cardEl = CardRenderer.createCardDOM(vm, slotData);
                cardEl.classList.add('card-mini');
                cardEl.onclick = (e) => {
                    e.stopPropagation();
                    const m = window.__modals?.CardDetailModal;
                    if (m) m.open(slotData);
                };
                slot.appendChild(cardEl);
            } else {
                const empty = document.createElement('div');
                empty.className = 'stage-choice-empty';
                empty.textContent = '—';
                slot.appendChild(empty);
            }

            const label = document.createElement('div');
            label.className = 'stage-choice-label';
            label.textContent = _posLabel(pos);
            slot.appendChild(label);

            if (isValid) {
                slot.tabIndex = 0;
                slot.setAttribute('role', 'button');
                slot.addEventListener('click', (e) => {
                    if (e.target.closest('.card-mini')) return;
                    ModalManager.hide(DOM_IDS.SELECTION_MODAL);
                    if (window.doAction) window.doAction(actionItem.action);
                });
            }

            row.appendChild(slot);
        }

        stageBlock.appendChild(row);
        container.appendChild(stageBlock);
    }

    if (container.children.length === 0) return null;
    return container;
}

function _buildCardItemFromAction(a, cardByNo) {
    const cardNo = a.parameters?.card_no;
    if (a.action_type === 'Pass') return null;
    const resolved = cardNo ? State.resolveCardData(cardNo) : null;
    const cardData = cardNo ? (resolved || (cardByNo ? cardByNo[cardNo] : null) || null) : null;
    const isTextAction = cardNo === '-1' || (cardNo && !cardData);
    let name = cardData?.name || a.parameters?.card_name || a.description || '';
    if (!name || name.startsWith('ACT_') || name.startsWith('CHOOSE_')) {
        name = _resolveActionLabel(a);
    }
    name = _localizePositionLabel(a, name);
    name = _resolveSpecialLabel(cardNo, name);
    return { card: cardData, name, action: a, isText: isTextAction };
}

export const ChoiceView = {
    render: (state, container, useModal = true) => {
        const choice = state.pending_choice;

        // G2: in PVP mode, if the choice is for the opponent, show a waiting indicator.
        // In sandbox mode, show all choices with a player label — one person makes all decisions.
        // In pve (vs AI) mode, choices for AI (P2) are auto-answered, not shown.
        if (choice && state?.mode === 'pve' && choice.choice_player_id === 'p2') {
            return;
        }
        if (choice && state?.mode === 'pvp') {
            const viewerPlayerId = `p${State.perspectivePlayer + 1}`;
            if (choice.choice_player_id && choice.choice_player_id !== viewerPlayerId) {
                const _isMobile = typeof window.__isMobile === 'function' ? window.__isMobile() : false;
                if (!_isMobile) {
                    const waitDiv = document.createElement('div');
                    waitDiv.className = 'pending-choice-indicator';
                    waitDiv.innerHTML = `<div style="font-weight:bold; color:#ffcc00; padding:20px; text-align:center;">Waiting for opponent's choice...</div>`;
                    container.appendChild(waitDiv);
                }
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
            : (choice.prompt_ja || choice.prompt_en || choice.title || '');
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
            const optContainer = document.createElement('div');
            optContainer.className = 'choice-cards-row';

            // ── Phase 1: Build unified items from available data sources ──
            const optItems = ChoiceView._buildItems(choice, state);

            // ── Phase 2: Deduplicate by action identity ──
            const seen = new Set();
            const unique = optItems.filter(item => {
                const key = item.action.index || JSON.stringify(item.action);
                if (seen.has(key)) return false;
                seen.add(key);
                return true;
            });

            // ── Phase 3: Render items ──
            const hasPositionItems = unique.some(_isPositionItem);
            unique.forEach(item => {
                if (hasPositionItems && _isPositionItem(item)) return;
                const el = ChoiceView._renderItem(item, choice);
                if (!el) return;
                const isDisabled = item.action?.parameters?.disabled === true;
                if (isDisabled) {
                    el.style.opacity = '0.35';
                    el.style.filter = 'grayscale(1)';
                    el.style.cursor = 'not-allowed';
                } else {
                    el.onclick = () => { if (window.doAction) window.doAction(item.action); };
                }
                el._choiceCard = item.card || null;
                el._choiceAction = item.action || null;
                optContainer.appendChild(el);
            });

            if (hasPositionItems) {
                const stageEl = _renderStageChoice(unique, choice);
                if (stageEl) {
                    optContainer.className = '';
                    optContainer.appendChild(stageEl);
                }
            }

            if (optContainer.children.length > 0) {
                choiceDiv.appendChild(optContainer);
                hasContent = true;
            }
        }

        if (hasContent) {
            const isMobileChoice = typeof window.__isMobile === 'function' ? window.__isMobile() : false;
            if (useModal && isMobileChoice) {
                const choiceStateId = state.state_id || 0;
                if (State._choiceModalDismissed && State._choiceStateId === choiceStateId) {
                    const cb = document.getElementById('mobile-choice-btn');
                    if (cb) cb.style.display = 'flex';
                    return;
                }
                State._choiceModalDismissed = false;
                State._choiceStateId = choiceStateId;

                const selModal = document.getElementById(DOM_IDS.SELECTION_MODAL);
                const selContent = document.getElementById(DOM_IDS.SELECTION_CONTENT);
                const selTitle = document.getElementById('selection-title');
                if (selModal && selContent) {
                    selContent.innerHTML = '';
                    _choiceCards = [];
                    _choiceIndex = -1;

                    _selViewBtn = document.getElementById('selection-view-card-btn');
                    _selSelectBtn = document.getElementById('selection-select-btn');
                    const selCloseBtn = document.getElementById('selection-close-btn');
                    if (_selViewBtn) { _selViewBtn.style.display = ''; _selViewBtn.onclick = _openChoiceDetail; }
                    if (_selSelectBtn) { _selSelectBtn.style.display = ''; _selSelectBtn.onclick = _selectCurrentChoice; }
                    if (selCloseBtn) selCloseBtn.style.display = 'none';

                    while (choiceDiv.children.length > 0) {
                        selContent.appendChild(choiceDiv.children[0]);
                    }
                    if (selTitle) {
                        const cardName = choice?.card_name || choice?.source_member || '';
                        selTitle.textContent = cardName ? `${i18n.t('sel_title') || 'Select'}: ${cardName}` : (i18n.t('sel_title') || 'Select');
                    }

                    selContent.querySelectorAll('.choice-item, .choice-cards-row > *').forEach(el => {
                        if (el.onclick) {
                            const orig = el.onclick;
                            el.onclick = function(e) {
                                ModalManager.hide(DOM_IDS.SELECTION_MODAL);
                                State._choiceModalDismissed = false;
                                return orig.call(this, e);
                            };
                        }
                    });

                    selContent.querySelectorAll('.choice-item, .card-choice, .card').forEach(el => {
                        const existingOnclick = el.onclick;
                        el.onclick = null;
                        el.style.cursor = 'pointer';
                        const idx = _choiceCards.length;
                        const entry = { card: el._choiceCard || null, action: el._choiceAction || null, el };
                        if (entry.card) _choiceCards.push(entry);
                        el.addEventListener('click', (e) => {
                            e.stopPropagation();
                            if (!entry.card && existingOnclick) {
                                existingOnclick.call(el, e);
                                return;
                            }
                            if (entry.card) {
                                _selectByIndex(idx);
                            }
                        });
                    });

                    ModalManager.show(DOM_IDS.SELECTION_MODAL);
                    const cb = document.getElementById('mobile-choice-btn');
                    if (cb) cb.style.display = 'none';
                    choiceDiv.style.display = 'none';
                }

                container.appendChild(choiceDiv);
            } else {
                container.appendChild(choiceDiv);
            }
        }
    },

    /// Build a unified optItems array from whatever data source the choice provides.
    _buildItems(choice, state) {
        const items = [];
        const selByNo = {};
        if (choice.selection_cards) {
            choice.selection_cards.forEach(sc => { if (sc.card_no) selByNo[sc.card_no] = sc; });
        }
        const hasOptions = choice.options && choice.options.length > 0;
        const firstOpt = hasOptions ? choice.options[0] : null;

        // 1) Auto-ability options (Rule 9.5.3)
        if (hasOptions && firstOpt.ability_text) {
            choice.options.forEach((opt, idx) => {
                const cardName = opt.card_name || `Ability ${idx + 1}`;
                let abilityText = opt.ability_text || '';
                if (State.currentLang === 'en' && window.translateAbility) {
                    abilityText = window.translateAbility(abilityText, 'en');
                }
                const optCard = opt.card_id !== undefined
                    ? (State.resolveCardData(opt.card_id) || Tooltips.findCardById(opt.card_id))
                    : (opt.card_no ? State.resolveCardData(opt.card_no) : null);
                const action = state.legal_actions?.find(a =>
                    a.parameters?.card_id === idx || a.description?.startsWith(cardName)
                );
                items.push({ card: optCard, name: cardName, desc: abilityText, action: action || { index: idx } });
            });
            return items;
        }

        // 2) Live success options
        if (hasOptions && firstOpt.card_index !== undefined) {
            choice.options.forEach((opt, idx) => {
                const resolved = opt.card_no
                    ? (selByNo[opt.card_no] || State.resolveCardData(opt.card_no) || State.resolveCardDataByName(opt.card_name))
                    : null;
                const action = state.legal_actions?.find(a =>
                    (a.action_type === 'select_card' || a.action_type === 'select_live_card') &&
                    (a.parameters?.card_id === idx || a.parameters?.card_indices?.includes(idx) || a.parameters?.card_no === opt.card_no)
                );
                items.push({
                    card: resolved,
                    name: opt.card_name || `Card ${idx + 1}`,
                    action: action || { action_type: 'select_card', parameters: { card_indices: [idx] } },
                    isText: !resolved,
                });
            });
            return items;
        }

        // 3) String options (heart colors, yes/no, etc.)
        if (hasOptions && typeof firstOpt === 'string') {
            (state.legal_actions || []).forEach(a => {
                const cardNo = a.parameters?.card_no;
                const isHeart = cardNo && cardNo.startsWith('heart');
                const optIdx = a.parameters?.card_id;
                const optText = optIdx !== undefined && choice.options[optIdx] ? choice.options[optIdx] : null;
                const name = isHeart
                    ? cardNo.replace('heart0', '♥').replace('heart', '♥')
                    : (a.description || optText || a.parameters?.card_name || '');
                items.push({ card: null, name, action: a, isText: !isHeart });
            });
            return items;
        }

        // 4) WASM-style options with action IDs
        if (hasOptions && choice.actions) {
            choice.options.forEach((opt, idx) => {
                const actionId = choice.actions?.[idx];
                if (actionId === undefined || actionId === null || actionId === 0) return;
                const optCardId = opt.card_id !== undefined ? opt.card_id : null;
                const fallbackName = opt.name || opt.text || `Option ${idx + 1}`;
                const optCard = optCardId !== null
                    ? (State.resolveCardData(optCardId) || Tooltips.findCardById(optCardId))
                    : null;
                items.push({ card: optCard, name: fallbackName, action: { index: actionId } });
            });
            return items;
        }

        // 5) REST-style: selection_cards + legal_actions
        // Only render cards present in selection_cards — non-matching cards that
        // exist in legal_actions (as disabled) are for look-and-select (all cards
        // shown, some greyed out) and already have full selection_cards data.
        // For non-look choices (hand/discard/stage), selection_cards only includes
        // matching cards, so non-matching legal_actions are silently skipped.
        if (choice.selection_cards && choice.selection_cards.length > 0 && state.legal_actions) {
            state.legal_actions.forEach(a => {
                if (a.action_type !== 'select_card' && a.action_type !== 'select_skip') return;
                const cardNo = a.parameters?.card_no;
                // select_skip has card_no="skip" which isn't in selByNo — let it through
                if (!cardNo || (a.action_type !== 'select_skip' && !selByNo[cardNo])) return;
                const item = _buildCardItemFromAction(a, selByNo);
                if (item) items.push(item);
            });
            return items;
        }

        // 6) Fallback: legal_actions only
        if (state.legal_actions && state.legal_actions.length > 0) {
            state.legal_actions.forEach(a => {
                if (a.action_type === 'Pass') return;
                const item = _buildCardItemFromAction(a, null);
                if (item) items.push(item);
            });
            return items;
        }

        return items;
    },

    /// Render a single item to a DOM element based on its visual type.
    _renderItem(item, choice) {
        const cardData = item.card;

        if (item.desc) {
            // Auto-ability option: show card thumbnail + name + ability text
            const el = document.createElement('div');
            el.className = 'choice-item text-option auto-ability';
            el.style.cssText = `
                display: flex; flex-direction: row; align-items: center; gap: 8px;
                padding: 4px 8px; width: 100%; height: auto; min-height: 0;
                flex-shrink: 1; box-sizing: border-box; cursor: pointer;
                font-size: 0.9rem; background: var(--input-bg, #2a2a3a);
                border: 1px solid var(--accent-purple, #9966ff); border-radius: 6px;
                color: var(--text, #eee); text-align: left;
            `;
            if (cardData && cardData.card_no) {
                const vm = CardRenderer.getCardViewModel(cardData, { mini: true });
                el.appendChild(CardRenderer.createCardDOM(vm, cardData));
            }
            const wrap = document.createElement('div');
            wrap.style.cssText = 'flex:1;min-width:0;display:flex;flex-direction:column;gap:2px;';
            wrap.innerHTML = `
                <div style="font-weight:bold;color:#cc88ff;font-size:0.85rem;">${Tooltips.enrichAbilityText(item.name)}</div>
                <div style="opacity:0.8;font-size:0.75rem;line-height:1.3;">${Tooltips.enrichAbilityText(item.desc)}</div>
            `;
            el.appendChild(wrap);
            return el;
        }

        if (item.isText) {
            const el = document.createElement('div');
            el.className = 'choice-item text-option';
            el.style.cssText = `
                display: flex; align-items: center; justify-content: center;
                padding: 12px 16px; width: auto; height: auto; min-width: 80px;
                min-height: 48px; flex-shrink: 1; cursor: pointer; font-size: 0.95rem;
                background: var(--input-bg, #2a2a3a); border: 2px solid var(--border, #555);
                border-radius: 8px; color: var(--text, #eee);
            `;
            if (item.name.includes('{{')) {
                el.innerHTML = Tooltips.enrichAbilityText(item.name);
            } else {
                el.textContent = item.name;
            }
            return el;
        }

        if (item.name.startsWith('♥') || item.name.match(/heart\d{2}/)) {
            const heartIdx = parseInt(item.name.replace(/\D/g, '')) || 1;
            const el = document.createElement('div');
            el.className = 'choice-item heart-option';
            el.style.cssText = `
                background: none; border: 2px solid var(--border);
                display: flex; align-items: center; justify-content: center;
                font-size: 2rem; min-width: 64px; min-height: 64px; cursor: pointer;
            `;
            const img = document.createElement('img');
            img.src = `img/texticon/heart_0${heartIdx}.png`;
            img.className = 'heart-mini-icon';
            img.style.width = '48px';
            img.style.height = '48px';
            el.appendChild(img);
            return el;
        }

        if (cardData && cardData.card_no) {
            if (choice.blind) {
                const el = document.createElement('div');
                el.className = 'card card-compact card-back';
                const img = document.createElement('img');
                img.src = fixImg('img/texticon/lltcg-back.png');
                img.style.width = '100%';
                img.style.height = '100%';
                img.style.objectFit = 'cover';
                img.draggable = false;
                el.appendChild(img);
                el.title = '??? (blind pick)';
                return el;
            }
            const vm = CardRenderer.getCardViewModel(cardData, { mini: true, actionId: item.action?.index });
            const el = CardRenderer.createCardDOM(vm, cardData);
            el.classList.add('card-choice');
            return el;
        }

        // Fallback text element
        const el = document.createElement('div');
        el.className = 'choice-item text-option';
        el.style.cssText = `
            display: flex; align-items: center; justify-content: center;
            padding: 8px 12px; width: 100%; height: auto; flex-shrink: 1;
            font-size: 0.85rem; background: var(--input-bg, #2a2a3a);
            border: 1px solid var(--border, #555); border-radius: 6px;
            color: var(--text, #eee); box-sizing: border-box;
        `;
        if (item.name.includes('{{')) {
            el.innerHTML = Tooltips.enrichAbilityText(item.name);
        } else {
            el.textContent = item.name;
        }
        return el;
    },
};
