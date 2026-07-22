import { State } from '../state.js';
import { ModalManager } from '../utils/ModalManager.js';
import { DOM_IDS } from '../constants_dom.js';

export const LobbyModal = {
    openLobby: () => {
        ModalManager.show(DOM_IDS.MODAL_ROOM);

        const waitingHint = document.getElementById('room-waiting-hint');
        if (waitingHint) {
            waitingHint.textContent = State.roomCode
                ? `Room: ${State.roomCode}`
                : 'Select Sandbox or enter a room code to join.';
        }
    },

    closeLobby: () => {
        ModalManager.hide(DOM_IDS.MODAL_ROOM);
    }
};
