import { State } from '../state.js';
import { TextEnricher } from '../utils/TextEnricher.js';
import { ModalManager } from '../utils/ModalManager.js';
import { resolveCardImagePath } from '../components/CardRenderer.js';
import { DOM_IDS } from '../constants_dom.js';
import { fixImg } from '../constants.js';
import * as i18n from '../i18n/index.js';

// Navigation state
let navCards = [];       // flat list of cards in the current zone
let navIndex = 0;        // index within navCards
let navZoneNames = [];   // ordered zone names
let navZoneCards = {};   // zoneName -> cards[]
let navCurrentZone = ''; // which zone we're viewing
let navPerspectivePlayer = 0;

function buildZoneMap() {
    const state = State.data;
    if (!state) return {};
    const pp = State.perspectivePlayer;
    navPerspectivePlayer = pp;
    const p = pp === 0 ? state.player1 : state.player2;
    const opp = pp === 0 ? state.player2 : state.player1;
    if (!p) return {};

    const zones = {};

    const hand = p.hand?.cards || [];
    zones['hand'] = hand.filter(c => c && c.card_no && c.card_no > 0);

    const stage = (p.stage || []).filter(s => s && s.card_no && s.card_no > 0);
    zones['stage'] = stage;

    const live = (p.live_zone?.cards || []).filter(c => c && c.card_no && c.card_no > 0);
    zones['live'] = live;

    const discard = (p.discard?.cards || []).filter(c => c && c.card_no && c.card_no > 0);
    zones['discard'] = discard;

    // Under-cards (energy/members under stage)
    const under = [];
    (p.stage || []).forEach(slot => {
        (slot.under || []).forEach(c => {
            if (c && c.card_no && c.card_no > 0) under.push(c);
        });
    });
    if (under.length) zones['under'] = under;

    // Opponent's hand (back side only) / stage — show briefly
    const oppStage = (opp.stage || []).filter(s => s && s.card_no && s.card_no > 0);
    if (oppStage.length) zones['opp_stage'] = oppStage;

    return zones;
}

const ZONE_LABELS = {
    hand: 'Hand',
    stage: 'Stage',
    live: 'Live',
    discard: 'Discard',
    under: 'Under',
    opp_stage: 'Opp Stage',
};

function switchToZone(zoneName) {
    const zones = buildZoneMap();
    if (!zones[zoneName] || zones[zoneName].length === 0) return;
    navCurrentZone = zoneName;
    navCards = zones[zoneName];
    navIndex = 0;
    renderCurrentCard();
}

function navigateWithinZone(delta) {
    if (!navCards.length) return;
    const newIdx = navIndex + delta;
    if (newIdx < 0 || newIdx >= navCards.length) return;
    navIndex = newIdx;
    renderCurrentCard();
}

function navigateZone(delta) {
    const zones = buildZoneMap();
    const names = Object.keys(zones);
    if (!names.length) return;
    const idx = names.indexOf(navCurrentZone);
    if (idx === -1) { switchToZone(names[0]); return; }
    const newIdx = (idx + delta + names.length) % names.length;
    switchToZone(names[newIdx]);
}

function renderCurrentCard() {
    const modal = document.getElementById(DOM_IDS.MODAL_CARD_DETAIL);
    if (!modal) return;

    const titleEl = document.getElementById('card-detail-title');
    const imageEl = document.getElementById('card-detail-image');
    const contentEl = document.getElementById('card-detail-text');
    const statsEl = document.getElementById('card-detail-stats');
    const footerEl = document.getElementById('card-detail-position');

    const card = navCards[navIndex];
    if (!card) return;

    const cardNo = card.card_no;
    const resolved = card.card_no ? State.resolveCardData(card.card_no) : card;
    const cardObj = resolved || card;

    const isHidden = card.hidden || card.is_hidden ||
        card.card_no === -2 || card.card_no === -1 ||
        card.card_no === '-2' || card.card_no === '-1';

    const translated = window.translateCard ? window.translateCard(cardObj) : { name: cardObj.name, groups: cardObj.groups, units: cardObj.units };

    // Title
    let titleText = translated.name || cardObj.name || 'Card';
    if (cardNo && cardNo !== '-1' && cardNo !== -1) {
        const productMatch = String(cardNo).match(/^(PL!-[A-Za-z0-9]+)/);
        const productLabel = productMatch ? productMatch[1] : '';
        if (productLabel) titleText = `${productLabel} — ${titleText}`;
        titleText += ` <span style="opacity:0.5;font-size:0.75em;font-family:monospace;">${cardNo}</span>`;
    }
    if (titleEl) titleEl.innerHTML = titleText;

    // Image
    if (imageEl) {
        imageEl.innerHTML = '';
        if (isHidden) {
            const img = document.createElement('img');
            img.src = fixImg('img/texticon/lltcg-back.png');
            img.alt = 'Hidden';
            img.style.maxWidth = '100%';
            img.style.height = 'auto';
            imageEl.appendChild(img);
        } else if (cardNo && cardNo !== '-1' && cardNo !== -1) {
            const imgPath = resolveCardImagePath(cardNo);
            if (imgPath) {
                const img = document.createElement('img');
                img.src = fixImg(imgPath);
                img.alt = cardObj.name || '';
                imageEl.appendChild(img);
            }
        }
    }

    // Content (stats + text side-by-side)
    if (contentEl) {
        contentEl.innerHTML = '';
        if (isHidden) {
            contentEl.innerHTML = '<p style="opacity:0.5;">Card is hidden</p>';
        } else {
            let html = '';
            if (cardObj.groups || cardObj.units) {
                const groups = (translated.groups || cardObj.groups || []).join(', ');
                const units = (translated.units || cardObj.units || []).join(', ');
                const parts = [];
                if (groups) parts.push(`<span data-i18n="groups">Groups</span>: ${groups}`);
                if (units) parts.push(`<span data-i18n="units">Units</span>: ${units}`);
                if (parts.length) html += `<div class="card-detail-meta">${parts.join(' | ')}</div>`;
            }

            let rawText = cardObj.ability_text || cardObj.text || cardObj.original_text || '';
            if (rawText) {
                html += `<div class="card-detail-ability">${TextEnricher.enrichAbilityText(rawText)}</div>`;
            }
            contentEl.innerHTML = html;
        }
    }

    // Stats
    if (statsEl) {
        statsEl.innerHTML = '';
        if (!isHidden) {
            const badges = [];
            if (cardObj.energy_cost !== undefined) badges.push(`<span class="stat-badge">Cost: ${cardObj.energy_cost}</span>`);
            if (cardObj.power !== undefined) badges.push(`<span class="stat-badge">Power: ${cardObj.power}</span>`);
            if (cardObj.soul !== undefined) badges.push(`<span class="stat-badge">Soul: ${cardObj.soul}</span>`);
            if (badges.length) statsEl.innerHTML = badges.join(' ');
        }
    }

    // Footer position
    if (footerEl) {
        const zoneLabel = ZONE_LABELS[navCurrentZone] || navCurrentZone;
        footerEl.textContent = `${zoneLabel}: ${navIndex + 1} / ${navCards.length}`;
    }
}

// Map containerId → zone name
const CONTAINER_TO_ZONE = {
    'my-hand': 'hand', 'opp-hand': 'hand',
    'my-stage': 'stage', 'opp-stage': 'opp_stage',
    'my-live': 'live', 'opp-live': 'live',
    'my-discard': 'discard', 'opp-discard': 'discard',
};

export const CardDetailModal = {
    open(card, zoneHint) {
        const modal = document.getElementById(DOM_IDS.MODAL_CARD_DETAIL);
        if (!modal) return;

        // Build zone map and find which zone this card belongs to
        const zones = buildZoneMap();
        const mappedHint = CONTAINER_TO_ZONE[zoneHint] || zoneHint;

        if (mappedHint && zones[mappedHint]) {
            navCurrentZone = mappedHint;
            navCards = zones[mappedHint];
            const foundIdx = navCards.findIndex(c => c.card_no === card.card_no || c.id === card.id);
            navIndex = foundIdx >= 0 ? foundIdx : 0;
        } else {
            // Auto-detect zone
            navCurrentZone = '';
            for (const [name, cards] of Object.entries(zones)) {
                const foundIdx = cards.findIndex(c => c.card_no === card.card_no || c.id === card.id);
                if (foundIdx >= 0) {
                    navCurrentZone = name;
                    navCards = cards;
                    navIndex = foundIdx;
                    break;
                }
            }
            if (!navCurrentZone) {
                // Card not found in any zone (e.g., from a search result), just show it
                navCards = [card];
                navIndex = 0;
                navCurrentZone = 'card';
            }
        }

        ModalManager.show(DOM_IDS.MODAL_CARD_DETAIL);
        renderCurrentCard();
    },

    navigatePrev() { navigateWithinZone(-1); },
    navigateNext() { navigateWithinZone(1); },
    navigateZonePrev() { navigateZone(-1); },
    navigateZoneNext() { navigateZone(1); },

    close() {
        ModalManager.hide(DOM_IDS.MODAL_CARD_DETAIL);
    }
};
