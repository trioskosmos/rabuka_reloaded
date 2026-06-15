import { StringUtils } from '../utils/StringUtils.js';
import { State } from '../state.js';
import * as i18n from '../i18n/index.js';
import { Tooltips } from '../ui_tooltips.js';
import { Phase } from '../constants.js';

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
        // web_old behavior: prioritize action.description over card name
        let name = a.description || "";
        // Debug logging for action text
        if (!name && a.action_type) {
            console.warn(`[ActionButtons] Action ${a.action_type} (index:${a.index}) has no description, falling back to card name or action_type`);
        }
        // If no description, fall back to card name (web_ui behavior)
        if (!name && displayCard) {
            name = i18n.translateCard(displayCard).name;
            name = StringUtils.cleanCardName(name);
            console.log(`[ActionButtons] Using card name fallback: ${name}`);
        }
        // Final fallback to action_type
        if (!name) name = a.action_type || "";
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
            const prefix = isSelected ? '✓ ' : '';
            const displayName = prefix + (name || '?');
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
            const isActivation = displayName.includes('(起動)') || displayName.includes('(Activate)');
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
