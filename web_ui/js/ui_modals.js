import { State } from './state.js';
import { Network } from './network.js';
import { Rendering } from './ui_rendering.js';

import { DeckSetupModal } from './modals/DeckSetupModal.js';
import { GameSetupModal } from './modals/GameSetupModal.js';
import { SettingsModal } from './modals/SettingsModal.js';
import { PerformanceModal } from './modals/PerformanceModal.js';
import { HelpModal } from './modals/HelpModal.js';
import { LobbyModal } from './modals/LobbyModal.js';
import { ReportModal } from './modals/ReportModal.js';
import { DebugModal } from './modals/DebugModal.js';

export const Modals = {
    deckPresets: [],
    setupMode: 'pve',
    pvpJoinPid: 1,

    // --- Core Deck/Setup/Settings ---
    openDeckModal: () => DeckSetupModal.openDeckModal(),
    closeDeckModal: () => DeckSetupModal.closeDeckModal(),
    fetchAndPopulateDecks: () => DeckSetupModal.fetchAndPopulateDecks(),
    populateDeckSelect: (el, decks) => DeckSetupModal.populateDeckSelect(el, decks),
    submitDeck: () => DeckSetupModal.submitDeck(),
    loadTestDeck: () => DeckSetupModal.loadTestDeck(),
    loadRandomDeck: () => DeckSetupModal.loadRandomDeck(),

    openSetupModal: (mode) => GameSetupModal.openSetupModal(mode),
    closeSetupModal: () => GameSetupModal.closeSetupModal(),
    getDeckConfig: (pid) => GameSetupModal.getDeckConfig(pid),
    resolveDeck: (config) => GameSetupModal.resolveDeck(config),
    submitGameSetup: () => GameSetupModal.submitGameSetup(),

    openDeckSelectionForPvP: (pid) => GameSetupModal.openDeckSelectionForPvP(pid),
    submitPvPDeck: () => GameSetupModal.submitPvPDeck(),
    onDeckSelectChange: (pid, val) => GameSetupModal.onDeckSelectChange(pid, val),

    openSettingsModal: () => SettingsModal.openSettingsModal(),
    closeSettingsModal: () => SettingsModal.closeSettingsModal(),
    updateBoardScale: (val) => SettingsModal.updateBoardScale(val),
    toggleLang: () => SettingsModal.toggleLang(),
    toggleFriendlyAbilities: () => SettingsModal.toggleFriendlyAbilities(),
    updateLanguage: () => SettingsModal.updateLanguage(),
    toggleDebugMode: () => SettingsModal.toggleDebugMode(),

    // --- Performance ---
    showLastPerformance: () => PerformanceModal.showLastPerformance(),
    showPerformanceForTurn: (turn) => PerformanceModal.showPerformanceForTurn(turn),
    closePerformanceModal: () => PerformanceModal.closePerformanceModal(),

    // --- Help ---
    openHelpModal: () => HelpModal.openHelpModal(),
    closeHelpModal: () => HelpModal.closeHelpModal(),

    // --- Lobby ---
    openLobby: () => LobbyModal.openLobby(),
    closeLobby: () => LobbyModal.closeLobby(),

    // --- Report ---
    openReportModal: () => ReportModal.openReportModal(),
    closeReportModal: () => ReportModal.closeReportModal(),
    submitReport: () => ReportModal.submitReport(),
    downloadReport: () => ReportModal.downloadReport(),

    // --- Debug ---
    openDebugModal: (tab) => {
        DebugModal.openDebugModal();
        if (tab) DebugModal.switchTab(tab);
    },
    closeDebugModal: () => DebugModal.closeDebugModal(),
    saveState: () => DebugModal.saveState(),
    copyState: () => {
        DebugModal.openDebugModal();
        DebugModal.switchTab('string');
    },
    loadState: () => {
        DebugModal.openDebugModal();
        DebugModal.switchTab('string');
        setTimeout(() => DebugModal.triggerFileLoad(), 50);
    },
    applyState: () => DebugModal.applyCustomState(),
    rewind: () => DebugModal.rewind(),
    redo: () => DebugModal.redo(),
    switchDebugTab: (tab) => DebugModal.switchTab(tab)
};
