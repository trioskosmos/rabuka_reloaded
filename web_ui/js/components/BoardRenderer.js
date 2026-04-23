import { State } from '../state.js';
import { fixImg } from '../constants.js';
import { Tooltips } from '../ui_tooltips.js';
import { CardRenderer } from './CardRenderer.js';
import { DOMUtils } from '../utils/DOMUtils.js';

export const BoardRenderer = {
    renderBoard: (state, p0, p1, validTargets, showDiscardModalCallback) => {
        // Rust backend format: stage is { left_side, center, right_side }, live_zone is { cards }
        const myStage = p0.stage ? [p0.stage.left_side, p0.stage.center, p0.stage.right_side].filter(c => c) : [];
        const oppStage = p1.stage ? [p1.stage.left_side, p1.stage.center, p1.stage.right_side].filter(c => c) : [];
        
        CardRenderer.renderStage('my-stage', myStage, true, validTargets.myStage, validTargets.hasSelection);
        CardRenderer.renderStage('opp-stage', oppStage, true, validTargets.oppStage, validTargets.hasSelection);
        
        const myLive = p0.live_zone ? p0.live_zone.cards : [];
        const oppLive = p1.live_zone ? p1.live_zone.cards : [];
        CardRenderer.renderLiveZone('my-live', myLive, true, validTargets.myLive, validTargets.hasSelection);
        CardRenderer.renderLiveZone('opp-live', oppLive, true, validTargets.oppLive, validTargets.hasSelection);
        
        CardRenderer.renderDiscardPile('my-discard-visual', p0.discard || [], 0, validTargets.discard, validTargets.hasSelection, showDiscardModalCallback);
        CardRenderer.renderDiscardPile('opp-discard-visual', p1.discard || [], 1, validTargets.discard, validTargets.hasSelection, showDiscardModalCallback);

        const myEnergy = p0.energy ? p0.energy.cards : [];
        const oppEnergy = p1.energy ? p1.energy.cards : [];
        BoardRenderer.renderEnergy('my-energy', myEnergy, true, validTargets.myEnergy, validTargets.hasSelection, state);
        BoardRenderer.renderEnergy('opp-energy', oppEnergy, true, validTargets.oppEnergy, validTargets.hasSelection, state);

        const mySuccess = p0.success_live_card_zone ? p0.success_live_card_zone.cards : [];
        const oppSuccess = p1.success_live_card_zone ? p1.success_live_card_zone.cards : [];
        CardRenderer.renderCards('my-success', mySuccess, true, true);
        CardRenderer.renderCards('opp-success', oppSuccess, false, true);

        BoardRenderer.renderDeckCounts(p0, p1);
    },

    renderDeckCounts: (p0, p1) => {
        const updateCount = (id, count) => {
            const el = document.getElementById(id);
            if (el) {
                el.textContent = count !== undefined ? count : 0;
            } else {
                console.warn('[BoardRenderer] Element not found:', id);
            }
        };

        console.log('[BoardRenderer] renderDeckCounts - p0.hand:', p0?.hand, 'p0.energy:', p0?.energy);

        // Rust backend format
        updateCount('my-deck-count', p0.main_deck_count);
        updateCount('opp-deck-count', p1.main_deck_count);
        updateCount('my-energy-deck-count', p0.energy_deck_count);
        updateCount('opp-energy-deck-count', p1.energy_deck_count);
        updateCount('my-discard-count', p0.waitroom_count || 0);
        updateCount('opp-discard-count', p1.waitroom_count || 0);

        // Update hand and energy counts
        updateCount('my-hand-count', p0.hand ? p0.hand.cards.length : 0);
        updateCount('my-energy-count', p0.energy ? p0.energy.cards.length : 0);
        updateCount('opp-hand-count', p1.hand ? p1.hand.cards.length : 0);
        updateCount('opp-energy-count', p1.energy ? p1.energy.cards.length : 0);
    },

    renderEnergy: (containerId, energy, clickable = false, validActionMap = {}, hasGlobalSelection = false, state = null) => {
        const el = document.getElementById(containerId);
        if (!el) return;
        if (!energy) {
            el.innerHTML = '';
            return;
        }

        const existingPips = Array.from(el.children);
        const energyCount = energy.length;

        // Synchronize pip count
        while (el.children.length > energyCount) {
            el.removeChild(el.lastChild);
        }

        energy.forEach((e, i) => {
            const action = validActionMap[i];
            const isValid = action !== undefined;
            const highlightClass = isValid ? ' valid-target' : '';
            // Rust backend: orientation is 'Active' or 'Wait'
            const isWait = e.orientation === 'Wait';
            const tappedClass = isWait ? ' tapped' : '';
            const existingPip = existingPips[i];

            let div;
            if (existingPip) {
                div = existingPip;
            } else {
                div = document.createElement('div');
                el.appendChild(div);
            }

            const newClassName = 'energy-pip' + tappedClass + highlightClass;
            if (div.className !== newClassName) div.className = newClassName;
            div.id = `${containerId}-slot-${i}`;

            const imgPath = fixImg('img/texticon/icon_energy.png');
            let img = div.querySelector('img');
            if (!img) {
                img = document.createElement('img');
                div.appendChild(img);
            }
            if (img.getAttribute('src') !== imgPath) {
                img.setAttribute('src', imgPath);
            }

            let numberEl = div.querySelector('.energy-num');
            if (!numberEl) {
                numberEl = document.createElement('div');
                numberEl.className = 'energy-num';
                div.appendChild(numberEl);
            }
            const numberText = String(i + 1);
            if (numberEl.textContent !== numberText) numberEl.textContent = numberText;

            if (e) {
                Tooltips.attachCardData(div, e, isValid ? action : undefined);
            } else if (isValid) {
                DOMUtils.patchAttributes(div, { 'data-action-id': action.index });
            }

            if (clickable && isValid) {
                div.style.cursor = 'pointer';
                div.onclick = () => { if (window.doAction) window.doAction(action); };

                div.onmouseenter = () => {
                    if (window.highlightActionBtn) window.highlightActionBtn(action.index, true);
                };
                div.onmouseleave = () => {
                    if (window.highlightActionBtn) window.highlightActionBtn(action.index, false);
                };
            } else {
                div.style.cursor = '';
                div.onclick = null;
                div.onmouseenter = null;
                div.onmouseleave = null;
            }
        });
    }
};
