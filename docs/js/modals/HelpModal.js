export const HelpModal = {
    openHelpModal: () => {
        const modal = document.getElementById('help-modal');
        if (modal) modal.style.display = 'flex';
    },

    closeHelpModal: () => {
        const modal = document.getElementById('help-modal');
        if (modal) modal.style.display = 'none';
    }
};
