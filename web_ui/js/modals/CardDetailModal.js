import { State } from '../state.js';
import { TextEnricher } from '../utils/TextEnricher.js';
import { ModalManager } from '../utils/ModalManager.js';
import { resolveCardImagePath } from '../components/CardRenderer.js';
import { DOM_IDS } from '../constants_dom.js';
import { fixImg } from '../constants.js';
import * as i18n from '../i18n/index.js';

// Navigation state
let navCards = [];
let navIndex = 0;
let navCurrentZone = '';

function buildAllCards() {
    const state = State.data;
    if (!state) return {};
    const pp = State.perspectivePlayer;
    const p = pp === 0 ? state.player1 : state.player2;
    const opp = pp === 0 ? state.player2 : state.player1;
    if (!p) return {};

    const cards = {};

    const handArr = Array.isArray(p.hand?.cards) ? p.hand.cards : [];
    cards['hand'] = handArr.filter(c => c && c.card_no > 0);

    const stageArr = Array.isArray(p.stage) ? p.stage : [];
    cards['stage'] = stageArr.filter(s => s && s.card_no > 0);

    const liveArr = Array.isArray(p.live_zone?.cards) ? p.live_zone.cards : [];
    cards['live'] = liveArr.filter(c => c && c.card_no > 0);

    const discArr = Array.isArray(p.discard?.cards) ? p.discard.cards : [];
    cards['discard'] = discArr.filter(c => c && c.card_no > 0);

    const under = [];
    stageArr.forEach(slot => {
        const uArr = Array.isArray(slot.under) ? slot.under : [];
        uArr.forEach(c => { if (c && c.card_no > 0) under.push(c); });
    });
    if (under.length) cards['under'] = under;

    const oppStageArr = Array.isArray(opp?.stage) ? opp.stage : [];
    const oppStage = oppStageArr.filter(s => s && s.card_no > 0);
    if (oppStage.length) cards['opp_stage'] = oppStage;

    return cards;
}

function render() {
    const titleEl = document.getElementById('card-detail-title');
    const imageEl = document.getElementById('card-detail-image');
    const textEl = document.getElementById('card-detail-text');
    const statsEl = document.getElementById('card-detail-stats');
    const footerEl = document.getElementById('card-detail-position');

    const card = navCards[navIndex];
    if (!card) return;

    const cardNo = card.card_no;
    const resolved = card.card_no > 0 ? State.resolveCardData(card.card_no) : null;
    const cardObj = resolved || card;

    const isHidden = card.hidden || card.is_hidden || card.card_no <= 0;
    const translated = window.translateCard ? window.translateCard(cardObj) : null;

    // Title
    if (titleEl) {
        let t = translated?.name || cardObj.name || 'Card';
        if (cardNo > 0) {
            t += ` <span style="opacity:0.5;font-size:0.75em;font-family:monospace;">${cardNo}</span>`;
        }
        titleEl.innerHTML = t;
    }

    // Image
    if (imageEl) {
        imageEl.innerHTML = '';
        if (cardNo > 0) {
            const imgPath = resolveCardImagePath(cardNo);
            if (imgPath) {
                const img = document.createElement('img');
                img.src = fixImg(imgPath);
                img.alt = cardObj.name || '';
                imageEl.appendChild(img);
            }
        }
    }

    // Stats
    if (statsEl) {
        statsEl.innerHTML = '';
        if (!isHidden) {
            const b = [];
            if (cardObj.energy_cost !== undefined) b.push(`<span class="stat-badge">Cost: ${cardObj.energy_cost}</span>`);
            if (cardObj.power !== undefined) b.push(`<span class="stat-badge">Power: ${cardObj.power}</span>`);
            if (cardObj.soul !== undefined) b.push(`<span class="stat-badge">Soul: ${cardObj.soul}</span>`);
            if (b.length) statsEl.innerHTML = b.join(' ');
        }
    }

    // Text
    if (textEl) {
        textEl.innerHTML = '';
        if (isHidden) {
            textEl.innerHTML = '<p style="opacity:0.5;">Card is hidden</p>';
        } else {
            let html = '';
            const groups = (translated?.groups || cardObj.groups || []).join(', ');
            const units = (translated?.units || cardObj.units || []).join(', ');
            const parts = [];
            if (groups) parts.push(`Groups: ${groups}`);
            if (units) parts.push(`Units: ${units}`);
            if (parts.length) html += `<div style="font-size:0.75em;opacity:0.7;margin-bottom:4px;">${parts.join(' | ')}</div>`;

            const rawText = cardObj.ability_text || cardObj.text || cardObj.original_text || '';
            if (rawText) html += `<div class="card-detail-ability">${TextEnricher.enrichAbilityText(rawText)}</div>`;
            textEl.innerHTML = html;
        }
    }

    if (footerEl) {
        footerEl.textContent = navCards.length > 1 ? `${navIndex + 1}/${navCards.length}` : '';
    }
}

export const CardDetailModal = {
    open(card) {
        const cards = buildAllCards();
        navCards = [card];
        navIndex = 0;
        navCurrentZone = '';

        // Try to find which zone this card belongs to and set up navigation
        for (const [zone, list] of Object.entries(cards)) {
            const idx = list.findIndex(c => c.card_no === card.card_no || c.id === card.id);
            if (idx >= 0) {
                navCards = list;
                navIndex = idx;
                navCurrentZone = zone;
                break;
            }
        }

        ModalManager.show(DOM_IDS.MODAL_CARD_DETAIL);
        render();
    },

    navigatePrev() {
        if (navIndex > 0) { navIndex--; render(); }
    },
    navigateNext() {
        if (navIndex < navCards.length - 1) { navIndex++; render(); }
    },
    navigateZonePrev() {
        const cards = buildAllCards();
        const names = Object.keys(cards);
        if (!names.length) return;
        const idx = names.indexOf(navCurrentZone);
        if (idx < 0) { navCurrentZone = names[0]; navCards = cards[names[0]]; navIndex = 0; render(); return; }
        const prev = (idx - 1 + names.length) % names.length;
        navCurrentZone = names[prev];
        navCards = cards[prev];
        navIndex = 0;
        render();
    },
    navigateZoneNext() {
        const cards = buildAllCards();
        const names = Object.keys(cards);
        if (!names.length) return;
        const idx = names.indexOf(navCurrentZone);
        if (idx < 0) { navCurrentZone = names[0]; navCards = cards[names[0]]; navIndex = 0; render(); return; }
        const next = (idx + 1) % names.length;
        navCurrentZone = names[next];
        navCards = cards[next];
        navIndex = 0;
        render();
    },

    close() {
        ModalManager.hide(DOM_IDS.MODAL_CARD_DETAIL);
    }
};
