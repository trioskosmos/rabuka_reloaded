/**
 * ZoneViewer Component
 * Handles the display of deck, discard, and various "card list" viewports.
 */
import * as i18n from '../i18n/index.js';
import { fixImg } from '../constants.js';
import { Tooltips } from '../ui_tooltips.js';
import { State } from '../state.js';
import { ModalManager } from '../utils/ModalManager.js';
import { DOM_IDS } from '../constants_dom.js';
import { ImageLoader } from './CardRenderer.js';

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
    },

    showDiscard: (playerIdx) => {
        if (!ZoneViewer.cache.modal) ZoneViewer.init();
        const state = State.data;
        if (!state) return;

        // Rust backend format: player1, player2
        const player = playerIdx === 0 ? state.player1 : state.player2;
        const discard = player.discard || [];
        const isMe = playerIdx === State.perspectivePlayer;
        const count = discard.length;

        ZoneViewer.cache.title.textContent = isMe ? i18n.t('your_discard_title', { count }) : i18n.t('opp_discard_title', { count });
        ZoneViewer.cache.container.innerHTML = '';
        ZoneViewer.cache.container.className = 'selection-grid';

        if (discard.length === 0) {
            ZoneViewer.cache.container.innerHTML = `<div style="grid-column: 1/-1; text-align: center; opacity: 0.5; padding: 40px;">${i18n.t('no_cards_discard')}</div>`;
        } else {
            // Render in reverse order (most recent on top/first)
            [...discard].reverse().forEach((c) => {
                // Rust backend: card is { card_no, name, card_type }
                const card = (typeof c === 'number') ? State.resolveCardData(c) : c;
                const div = ZoneViewer._createCardElement(card);
                ZoneViewer.cache.container.appendChild(div);
            });
        }
        ModalManager.show(DOM_IDS.MODAL_DISCARD);
    },

    showZoneViewer: (playerIdx) => {
        if (!ZoneViewer.cache.modal) ZoneViewer.init();
        const state = State.data;
        if (!state) return;

        const isMe = playerIdx === State.perspectivePlayer;
        
        // PRIVACY: Enforce deck privacy as requested
        if (!isMe) {
            console.log("[ZoneViewer] Privacy block: Opponent's deck is hidden.");
            // Optional: User might want a "Deck is Hidden" modal instead of nothing.
            // For now, let's just show the modal with a "Private" title for clarity.
            ZoneViewer.cache.title.textContent = i18n.t('opp_viewer_title_private') || "Opponent's Deck (Hidden)";
            ZoneViewer.cache.container.innerHTML = `<div style="opacity:0.5; padding:40px; text-align:center;">${i18n.t('deck_is_private') || "This zone is private."}</div>`;
            ModalManager.show(DOM_IDS.MODAL_DISCARD);
            return;
        }

        // Rust backend format: player1, player2
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
                // IMPORTANT: Resolve ID into rich card data if it's just a number
                const card = (typeof c === 'number') ? State.resolveCardData(c) : c;
                const div = ZoneViewer._createCardElement(card, false);
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

        ModalManager.show(DOM_IDS.MODAL_DISCARD);
    },

    _createCardElement: (card, isMini = false) => {
        if (!card) return document.createElement('div');
        const div = document.createElement('div');
        div.className = isMini ? 'card card-mini' : 'card';
        
        // Resolve card data if card_no is present but no image path
        let imgPath = card.img || card.img_path || card.image || '';
        if (!imgPath && card.card_no) {
            const resolved = State.resolveCardData(card.card_no);
            imgPath = (State.cardImageMapping && State.cardImageMapping[card.card_no])
                ? State.cardImageMapping[card.card_no]
                : (card.card_no ? `img/cards_webp/${card.card_no}.webp` : '');
        }
        
        const img = document.createElement('img');
        img.draggable = false;
        ImageLoader.loadImage(img, fixImg(imgPath));
        div.appendChild(img);
        
        const rawText = Tooltips.getEffectiveRawText(card);
        if (rawText) div.setAttribute('data-text', rawText);
        if (card.id !== undefined) div.setAttribute('data-card-id', card.id);
        
        Tooltips.attachCardData(div, card);
        return div;
    }
};
