import { StringUtils } from '../utils/StringUtils.js';
import { State } from '../state.js';
import * as i18n from '../i18n/index.js';
import { Tooltips } from '../ui_tooltips.js';
import { Phase } from '../constants.js';

export const ActionButtons = {
    getActionLabel: (a, isMini = false, state) => {
        const currentLang = State.currentLang;
        const params = a.parameters || {};
        const sourceCard = params.card_id !== undefined ? Tooltips.findCardById(params.card_id) : null;
        const displayCard = sourceCard;

        const energyIcon = `<img src="img/texticon/icon_energy.png" class="inline-icon">`;
        const heartIcon = `<img src="img/texticon/icon_heart.png" class="inline-icon">`;

        let cost = params.final_cost ?? params.base_cost ?? null;
        let name = a.description || a.action_type || "";
        const isBaton = params.use_baton_touch || (name && (name.includes('Baton') || name.includes('バトン')));

        if (isMini) {
            if (a.action_type === 'PlayMemberToStage') return `<span>${cost !== null ? cost : 0}</span>${isBaton ? ' [B]' : ''}`;
            if (a.action_type === 'SelectMulligan') {
                return `<span class="truncate-name">${name || '?'}</span>`;
            }
            let label = `${energyIcon}${cost !== null ? cost : 0}`;
            if (isBaton) label += ' [B]';
            return Tooltips.enrichAbilityText(label);
        } else {
            let displayName = name;

            // If we have a card, use its name
            if (displayCard) {
                displayName = i18n.translateCard(displayCard).name;
                displayName = StringUtils.cleanCardName(displayName);
            }

            displayName = Tooltips.enrichAbilityText(displayName);

            let label = `<div class="action-title" style="${(displayName.includes('&') || displayName.includes('＆')) ? 'font-size:0.85em;' : ''}">${displayName}</div>`;
            if (cost !== null) label += `<div class="action-cost">${energyIcon}${cost}</div>`;
            if (isBaton) label += ' [B]';
            return label;
        }
    },

    createActionButton: (a, isMini = false, extraClass = '', state) => {
        const btn = document.createElement('button');
        const isHovered = (a.index !== undefined && a.index === State.hoveredActionId);
        const hoverClass = isHovered ? ' hover-highlight' : '';
        btn.className = `btn action-btn ${isMini ? 'mini' : ''} ${extraClass}${hoverClass}`.trim();

        const params = a.parameters || {};
        const displayCard = params.card_id !== undefined ? Tooltips.findCardById(params.card_id) : null;

        Tooltips.attachCardData(btn, displayCard, a.index);

        btn.innerHTML = ActionButtons.getActionLabel(a, isMini, state);
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
