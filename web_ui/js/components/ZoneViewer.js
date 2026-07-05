/**
 * ZoneViewer Component
 * Handles the display of deck, discard, and various "card list" viewports.
 */
import * as i18n from '../i18n/index.js';
import { State } from '../state.js';
import { ModalManager } from '../utils/ModalManager.js';
import { DOM_IDS } from '../constants_dom.js';
import { CardRenderer } from './CardRenderer.js';

let _cards = [];
let _selectedIndex = -1;
let _viewBtn = null;
let _prevBtn = null;
let _nextBtn = null;

function _syncNav() {
    if (!_viewBtn) _viewBtn = document.getElementById('discard-view-card-btn');
    if (!_prevBtn) _prevBtn = document.getElementById('discard-nav-prev');
    if (!_nextBtn) _nextBtn = document.getElementById('discard-nav-next');
    const hasCard = _selectedIndex >= 0 && _selectedIndex < _cards.length;
    if (_viewBtn) _viewBtn.disabled = !hasCard;
    if (_prevBtn) _prevBtn.disabled = _selectedIndex <= 0;
    if (_nextBtn) _nextBtn.disabled = _selectedIndex >= _cards.length - 1;
}

function _highlightCard() {
    const grid = document.getElementById('discard-modal-cards');
    if (grid) grid.querySelectorAll('.card.selected, .selection-card-item.selected').forEach(e => e.classList.remove('selected'));
    if (_selectedIndex >= 0 && _selectedIndex < _cards.length) {
        const slots = grid?.children;
        if (slots && slots[_selectedIndex]) {
            slots[_selectedIndex].classList.add('selected');
            slots[_selectedIndex].scrollIntoView({ block: 'nearest', behavior: 'smooth' });
        }
    }
    _syncNav();
}

function _selectCard(index) {
    if (index < 0 || index >= _cards.length) return;
    _selectedIndex = index;
    _highlightCard();
}

function _openDetail() {
    if (_selectedIndex < 0 || _selectedIndex >= _cards.length) return;
    const m = window.__modals?.CardDetailModal;
    if (m) m.open(_cards[_selectedIndex]);
}

export const ZoneViewer = {
    cache: {
        modal: null,
        title: null,
        container: null
    },

    init: () => {
        ZoneViewer.cache.modal = document.getElementById(DOM_IDS.MODAL_DISCARD);
        ZoneViewer.cache.title = document.getElementById('discard-modal-title');
        ZoneViewer.cache.container = document.getElementById('discard-modal-cards');

        _viewBtn = document.getElementById('discard-view-card-btn');
        if (_viewBtn) _viewBtn.addEventListener('click', _openDetail);
        _prevBtn = document.getElementById('discard-nav-prev');
        _nextBtn = document.getElementById('discard-nav-next');
        if (_prevBtn) _prevBtn.addEventListener('click', () => _selectCard(_selectedIndex - 1));
        if (_nextBtn) _nextBtn.addEventListener('click', () => _selectCard(_selectedIndex + 1));
    },

    showDiscard: (playerIdx) => {
        if (!ZoneViewer.cache.modal) ZoneViewer.init();
        const state = State.data;
        if (!state) return;

        _cards = [];
        _selectedIndex = -1;

        const player = playerIdx === 0 ? state.player1 : state.player2;
        const discard = (player.waitroom?.cards || player.discard?.cards || player.waitroom || player.discard || []);
        const isMe = playerIdx === State.perspectivePlayer;
        const count = discard.length;

        ZoneViewer.cache.title.textContent = isMe ? i18n.t('your_discard_title', { count }) : i18n.t('opp_discard_title', { count });
        ZoneViewer.cache.container.innerHTML = '';
        ZoneViewer.cache.container.className = 'selection-grid';

        if (discard.length === 0) {
            ZoneViewer.cache.container.innerHTML = `<div style="grid-column: 1/-1; text-align: center; opacity: 0.5; padding: 40px;">${i18n.t('no_cards_discard')}</div>`;
        } else {
            [...discard].reverse().forEach((c) => {
                const card = (typeof c === 'number') ? State.resolveCardData(c) : c;
                const div = ZoneViewer._createCardElement(card);
                div.style.cursor = 'pointer';
                const idx = _cards.length;
                _cards.push(card);
                div.addEventListener('click', (e) => {
                    e.stopPropagation();
                    _selectCard(idx);
                });
                ZoneViewer.cache.container.appendChild(div);
            });
        }
        _syncNav();
        ModalManager.show(DOM_IDS.MODAL_DISCARD);
    },

    showZoneViewer: (playerIdx) => {
        if (!ZoneViewer.cache.modal) ZoneViewer.init();
        const state = State.data;
        if (!state) return;

        _cards = [];
        _selectedIndex = -1;

        const isMe = playerIdx === State.perspectivePlayer;

        if (!isMe) {
            console.log("[ZoneViewer] Privacy block: Opponent's deck is hidden.");
            ZoneViewer.cache.title.textContent = i18n.t('opp_viewer_title_private') || "Opponent's Deck (Hidden)";
            ZoneViewer.cache.container.innerHTML = `<div style="opacity:0.5; padding:40px; text-align:center;">${i18n.t('deck_is_private') || "This zone is private."}</div>`;
            _syncNav();
            ModalManager.show(DOM_IDS.MODAL_DISCARD);
            return;
        }

        const player = playerIdx === 0 ? state.player1 : state.player2;
        ZoneViewer.cache.title.textContent = i18n.t('your_viewer_title');
        ZoneViewer.cache.container.innerHTML = '';
        ZoneViewer.cache.container.className = 'zone-viewer-grid visual-only';

        const addSection = (label, cards) => {
            if (!cards || cards.length === 0) return;

            const section = document.createElement('div');
            section.className = 'zone-viewer-section';
            section.innerHTML = `<h3>${label} (${cards.length})</h3>`;

            const grid = document.createElement('div');
            grid.className = 'selection-grid';

            cards.forEach(c => {
                const card = (typeof c === 'number') ? State.resolveCardData(c) : c;
                const div = ZoneViewer._createCardElement(card);
                div.style.cursor = 'pointer';
                const idx = _cards.length;
                _cards.push(card);
                div.addEventListener('click', (e) => {
                    e.stopPropagation();
                    _selectCard(idx);
                });
                grid.appendChild(div);
            });

            section.appendChild(grid);
            ZoneViewer.cache.container.appendChild(section);
        };

        const initialDeck = player.initial_deck || [];
        const deck = player.deck_cards || player.deck || player.full_deck || [];
        const energyDeck = player.energy_deck_cards || player.energy_deck || [];

        if (initialDeck.length > 0) addSection(i18n.t('initial_deck'), initialDeck);
        if (deck.length > 0) addSection(i18n.t('member_deck_rem'), deck);
        if (energyDeck.length > 0) addSection(i18n.t('energy_deck_rem'), energyDeck);

        if (initialDeck.length === 0 && deck.length === 0 && energyDeck.length === 0) {
            ZoneViewer.cache.container.innerHTML = `<div style="opacity:0.5; padding:40px; text-align:center;">${i18n.t('no_cards_zone')}</div>`;
        }

        _syncNav();
        ModalManager.show(DOM_IDS.MODAL_DISCARD);
    },

    _createCardElement: (card) => {
        if (!card) return document.createElement('div');
        const vm = CardRenderer.getCardViewModel(card, { mini: true });
        const el = CardRenderer.createCardDOM(vm, card);
        return el;
    }
};
