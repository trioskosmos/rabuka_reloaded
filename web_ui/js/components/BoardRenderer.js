import { fixImg } from '../constants.js';
import { resolveCardImagePath, CardRenderer } from './CardRenderer.js';

export const BoardRenderer = {
    renderBoard: (state, p0, p1, validTargets, showDiscardModalCallback) => {
        // Engine sends correct data - don't skip render for missing zones
        // Just log warnings for debugging
        if (!p0?.hand?.cards) console.warn('[BoardRenderer] p0.hand.cards missing');
        if (!p1?.hand?.cards) console.warn('[BoardRenderer] p1.hand.cards missing');
        if (!p0?.energy?.cards) console.warn('[BoardRenderer] p0.energy.cards missing');
        if (!p1?.energy?.cards) console.warn('[BoardRenderer] p1.energy.cards missing');
        if (!p0?.live_zone?.cards) console.warn('[BoardRenderer] p0.live_zone.cards missing');
        if (!p1?.live_zone?.cards) console.warn('[BoardRenderer] p1.live_zone.cards missing');
        if (!p0?.discard?.cards) console.warn('[BoardRenderer] p0.discard.cards missing');
        if (!p1?.discard?.cards) console.warn('[BoardRenderer] p1.discard.cards missing');
        if (!p0?.success_live_card_zone?.cards) console.warn('[BoardRenderer] p0.success_live_card_zone.cards missing');
        if (!p1?.success_live_card_zone?.cards) console.warn('[BoardRenderer] p1.success_live_card_zone.cards missing');

        // Rust backend format: stage is { left_side, center, right_side, left_under, center_under, right_under }
        const myStage = p0.stage ? [p0.stage.left_side, p0.stage.center, p0.stage.right_side] : [];
        const myUnderCards = p0.stage ? [p0.stage.left_under || [], p0.stage.center_under || [], p0.stage.right_under || []] : [[], [], []];
        const oppStage = p1.stage ? [p1.stage.left_side, p1.stage.center, p1.stage.right_side] : [];
        const oppUnderCards = p1.stage ? [p1.stage.left_under || [], p1.stage.center_under || [], p1.stage.right_under || []] : [[], [], []];
        
        CardRenderer.renderStage('my-stage', myStage, myUnderCards, true, validTargets.myStage, validTargets.hasSelection);
        CardRenderer.renderStage('opp-stage', oppStage, oppUnderCards, true, validTargets.oppStage, validTargets.hasSelection);
        
        CardRenderer.renderLiveZone('my-live', p0.live_zone.cards, true, validTargets.myLive, validTargets.hasSelection);
        CardRenderer.renderLiveZone('opp-live', p1.live_zone.cards, true, validTargets.oppLive, validTargets.hasSelection);

        BoardRenderer.renderEnergy('my-energy', p0.energy.cards, true, validTargets.myEnergy, validTargets.hasSelection, state);
        BoardRenderer.renderEnergy('opp-energy', p1.energy.cards, true, validTargets.oppEnergy, validTargets.hasSelection, state);

        CardRenderer.renderCards('my-success', p0.success_live_card_zone.cards, true, true);
        CardRenderer.renderCards('opp-success', p1.success_live_card_zone.cards, false, true);

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

        updateCount('my-deck-count', p0.main_deck_count);
        updateCount('opp-deck-count', p1.main_deck_count);
        updateCount('my-energy-deck-count', p0.energy_deck_count);
        updateCount('opp-energy-deck-count', p1.energy_deck_count);
        // Engine sends waitroom zone, calculate count from cards
        updateCount('my-discard-count', (p0.waitroom?.cards?.length || p0.discard?.cards?.length || 0));
        updateCount('opp-discard-count', (p1.waitroom?.cards?.length || p1.discard?.cards?.length || 0));

        const myHandCount = p0.hand.cards.length;
        const oppHandCount = p1.hand.cards.length;
        
        updateCount('my-hand-count', myHandCount);
        updateCount('opp-hand-count', oppHandCount);
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
        const activeCount = energy.filter(e => e && e.orientation === 'Active').length;

        const countEl = el.parentElement?.querySelector('.area-count');
        if (countEl) countEl.textContent = `${activeCount}/${energyCount}`;

        while (el.children.length > energyCount) {
            el.removeChild(el.lastChild);
        }

        energy.forEach((e, i) => {
            const action = validActionMap[i];
            const isValid = action !== undefined;
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

            const newClassName = 'energy-pip' + tappedClass;
            if (div.className !== newClassName) div.className = newClassName;
            div.id = `${containerId}-slot-${i}`;

            const imgPath = e.card_no ? resolveCardImagePath(e.card_no) : fixImg('img/texticon/icon_energy.png');
            let img = div.querySelector('img');
            if (!img) {
                img = document.createElement('img');
                img.draggable = false;
                div.appendChild(img);
            }
            if (img.getAttribute('src') !== imgPath) {
                img.setAttribute('src', imgPath);
            }

            if (isValid) {
                div.style.cursor = 'pointer';
                div.onclick = () => { if (window.doAction) window.doAction(action); };
            } else {
                div.style.cursor = '';
                div.onclick = null;
            }
        });
    }
};
