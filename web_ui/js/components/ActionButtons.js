import { StringUtils } from '../utils/StringUtils.js';
import { State } from '../state.js';
import * as i18n from '../i18n/index.js';
import { Tooltips } from '../ui_tooltips.js';
import { Phase } from '../constants.js';
import { resolveCardImagePath } from './CardRenderer.js';

export const ActionButtons = {
    getActionLabel: (a, isMini = false, state) => {
        const currentLang = State.currentLang;
        // Support both parameters and params field names
        const params = a.parameters || a.params || {};
        const sourceCard = params.card_id !== undefined ? Tooltips.findCardById(params.card_id) : null;
        const displayCard = sourceCard;

        const energyIcon = `<img src="img/texticon/icon_energy.png" class="inline-icon">`;
        const heartIcon = `<img src="img/texticon/icon_blade.png" class="inline-icon">`;

        let cost = params.final_cost ?? params.base_cost ?? null;

        // Get name: description from engine, then card name, then action_type
        let name = a.description || "";
        if (!name && displayCard) {
            name = displayCard.name || displayCard.card_no || "";
        }
        if (!name) {
            name = a.action_type || "";
        }
        // For ability actions, show only the specific ability being activated
        if (a.action_type === 'use_ability' && displayCard) {
            const abilityText = Tooltips.extractRelevantAbility(displayCard, null, params.ability_index);
            if (abilityText && abilityText.length >= 5) {
                name = abilityText;
            } else {
                const fallback = Tooltips.getEffectiveRawText(displayCard);
                if (fallback && fallback.length >= 5) name = fallback;
            }
        }
        // For EN mode, translate Japanese card names to English
        if (currentLang === 'en' && displayCard && a.action_type !== 'use_ability') {
            name = i18n.translateCard(displayCard).name;
        }
        // For system actions without a card, translate via locale (both languages)
        if (!displayCard) {
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
            name = ACTION_LABELS[a.action_type] || name;
        }
        // For JP mode with a card, engine description is English — use card name instead
        if (currentLang === 'jp' && displayCard && a.description && a.action_type !== 'use_ability') {
            name = displayCard.name || displayCard.card_no || name;
        }
        // Special text-only actions — override generic labels with locale text
        if (params.card_no === 'pay_optional_cost') {
            name = i18n.t('pay_optional_cost');
        } else if (params.card_no === 'skip_optional_cost') {
            name = i18n.t('skip');
        }
        name = StringUtils.cleanCardName(name);
        const isBaton = params.use_baton_touch || (name && (name.includes('Baton') || name.includes('バトン')));

        if (a.action_type === 'select_mulligan') {
            const ci = params.card_index;
            let isSelected = false;
            if (state && ci !== undefined) {
                const p = State.perspectivePlayer;
                const player = p === 0 ? state.player1 : state.player2;
                if (player) {
                    const sel = player.mulligan_selection;
                    if (typeof sel === 'number') {
                        isSelected = ((sel >> ci) & 1) === 1;
                    } else if (Array.isArray(sel)) {
                        isSelected = sel.includes(ci);
                    }
                }
            }
            // Use card name directly (skip English description from engine)
            const cardName = displayCard
                ? (currentLang === 'en' ? i18n.translateCard(displayCard).name : StringUtils.cleanCardName(displayCard.name))
                : (name || '?');
            const prefix = isSelected ? '✓ ' : '';
            const displayName = prefix + cardName;
            if (isMini) return `<span class="truncate-name">${displayName}</span>`;
            return `<div class="action-title">${Tooltips.enrichAbilityText(displayName)}</div>`;
        }

        if (isMini) {
            if (a.action_type === 'play_member_to_stage') return `<span>${cost !== null ? cost : 0}</span>${isBaton ? ' [B]' : ''}`;
            let label = `${energyIcon}${cost !== null ? cost : 0}`;
            if (isBaton) label += ' [B]';
            return Tooltips.enrichAbilityText(label);
        } else {
            let displayName = name;
            const isActivation = displayName.includes('(起動)') || displayName.includes('(Activate)') || a.action_type === 'use_ability';
            if (isActivation) {
                displayName = displayName.split(' (起動)')[0].split(' (Activate)')[0];
            }
            displayName = Tooltips.enrichAbilityText(displayName);

            let label = `<div class="action-title" style="${(displayName.includes('&') || displayName.includes('＆')) ? 'font-size:0.85em;' : ''}">${displayName}</div>`;
            if (cost !== null && !isActivation) label += `<div class="action-cost">${energyIcon}${cost}</div>`;
            if (isBaton) label += ' [B]';
            return label;
        }
    },

    createActionButton: (a, isMini = false, extraClass = '', state) => {
        const btn = document.createElement('button');
        const isHovered = (a.index !== undefined && a.index === State.hoveredActionId);
        const hoverClass = isHovered ? ' hover-highlight' : '';
        btn.className = `btn action-btn ${isMini ? 'mini' : ''} ${extraClass}${hoverClass}`.trim();

        // Support both parameters and params field names
        const params = a.parameters || a.params || {};
        const displayCard = params.card_id !== undefined ? Tooltips.findCardById(params.card_id) : null;

        Tooltips.attachCardData(btn, displayCard, a.index);

        btn.innerHTML = ActionButtons.getActionLabel(a, isMini, state);

        // Show a small card thumbnail to the left for action types that have an associated card
        const thumbTypes = ['play_member_to_stage', 'use_ability', 'select_mulligan', 'set_live_card'];
        if (!isMini && thumbTypes.includes(a.action_type) && displayCard?.card_no) {
            const thumbSrc = resolveCardImagePath(displayCard.card_no);
            if (thumbSrc) {
                const img = document.createElement('img');
                img.className = 'action-card-thumb';
                img.draggable = false;
                img.src = thumbSrc;
                btn.prepend(img);
            }
        }

        // For mulligan actions: mark card thumbnail selected so it goes grayscale like hand cards
        if (a.action_type === 'select_mulligan') {
            const ci = (a.parameters || a.params || {}).card_index;
            if (ci !== undefined) {
                const isSelected = State.localMulliganSelection.has(ci);
                if (isSelected) {
                    const thumb = btn.querySelector('.action-card-thumb');
                    if (thumb) {
                        thumb.classList.add('mulligan-selected');
                    }
                }
            }
        }

        btn.onclick = () => { if (window.doAction && a.index !== undefined) window.doAction(a); };

        btn.onmouseenter = () => {
            if (window.highlightActionBtn && a.index !== undefined) {
                window.highlightActionBtn(a.index, true);
            }
        };
        btn.onmouseleave = () => {
            if (window.highlightActionBtn && a.index !== undefined) {
                window.highlightActionBtn(a.index, false);
            }
        };

        if (a.index !== undefined) {
            btn.setAttribute('data-action-id', a.index);
        }

        return btn;
    }
};
