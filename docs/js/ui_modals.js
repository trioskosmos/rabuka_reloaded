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
import { GameStateModal } from './modals/GameStateModal.js';
import { CardDetailModal } from './modals/CardDetailModal.js';
import { PlayActionModal } from './modals/PlayActionModal.js';
import { StageAbilityModal } from './modals/StageAbilityModal.js';
import { AbilityQueueModal } from './modals/AbilityQueueModal.js';

export const Modals = {
    deckPresets: [],
    pvpJoinPid: null,

    // --- Core Deck/Setup/Settings ---
    openDeckModal: () => DeckSetupModal.openDeckModal(),
    closeDeckModal: () => DeckSetupModal.closeDeckModal(),
    fetchAndPopulateDecks: () => DeckSetupModal.fetchAndPopulateDecks(),
    populateDeckSelect: (el, decks) => DeckSetupModal.populateDeckSelect(el, decks),
    submitDeck: () => DeckSetupModal.submitDeck(),
    loadTestDeck: () => DeckSetupModal.loadTestDeck(),


    openSetupModal: (mode) => GameSetupModal.openSetupModal(mode),
    closeSetupModal: () => GameSetupModal.closeSetupModal(),
    getDeckConfig: (pid) => GameSetupModal.getDeckConfig(pid),
    resolveDeck: (config) => GameSetupModal.resolveDeck(config),
    submitGameSetup: () => GameSetupModal.submitGameSetup(),
    startGame: (mode) => GameSetupModal.startGame(mode),

    openDeckSelectionForPvP: (pid) => GameSetupModal.openDeckSelectionForPvP(pid),
    submitPvPDeck: () => GameSetupModal.submitPvPDeck(),
    onDeckSelectChange: (pid, val) => GameSetupModal.onDeckSelectChange(pid, val),


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

    // --- Game State ---
    openGameStateModal: () => GameStateModal.open(),
    closeGameStateModal: () => GameStateModal.close(),

    // --- Debug ---
    openDebugModal: () => DebugModal.openDebugModal(),
    closeDebugModal: () => DebugModal.closeDebugModal(),
    rewind: () => DebugModal.rewind(),
    redo: () => DebugModal.redo(),

    // --- Mobile Modals ---
    openCardDetail: (card) => CardDetailModal.open(card),
    closeCardDetail: () => CardDetailModal.close(),
    openPlayAction: (card, actions) => PlayActionModal.open(card, actions),
    closePlayAction: () => PlayActionModal.close(),
    openStageAbility: (card, actions) => StageAbilityModal.open(card, actions),
    closeStageAbility: () => StageAbilityModal.close(),
    openAbilityQueue: () => AbilityQueueModal.open(),
    closeAbilityQueue: () => AbilityQueueModal.close(),
};
