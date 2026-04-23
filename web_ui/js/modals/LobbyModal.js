import { validator } from '../components/DeckValidator.js';
import { State } from '../state.js';
import { ModalManager } from '../utils/ModalManager.js';
import { DOM_IDS } from '../constants_dom.js';

export const LobbyModal = {
    openLobby: async () => {
        ModalManager.show(DOM_IDS.MODAL_ROOM);
        validator.init();
        if (window.fetchPublicRooms) window.fetchPublicRooms();

        // Populate join deck select with saved decks
        if (window.Modals && window.Modals.fetchAndPopulateDecks) {
            await window.Modals.fetchAndPopulateDecks();
            window.Modals.populateDeckSelect('pjoin-deck-select', window.Modals.deckPresets);
        }

        const waitingHint = document.getElementById('room-waiting-hint');
        if (waitingHint) {
            waitingHint.textContent = State.roomCode ? `Room ${State.roomCode}` : 'Join a private room or create a new one.';
        }

        const input = document.getElementById('pjoin-deck-paste');
        if (input) {
            input.addEventListener('input', () => LobbyModal.validateInline());
        }
    },

    validateInline: () => {
        const input = document.getElementById('pjoin-deck-paste');
        const preview = document.getElementById('pjoin-deck-preview');
        if (!input || !preview) return;

        const results = validator.validateDeckString(input.value);
        validator.renderPreview(results, preview);
    },

    closeLobby: () => {
        ModalManager.hide(DOM_IDS.MODAL_ROOM);
    }
};
