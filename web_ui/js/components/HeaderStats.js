/**
 * HeaderStats Component
 * Handles rendering of the game header (Turn, Phase, Energy, Scores, Hearts Summary).
 */
import * as i18n from '../i18n/index.js';
import { PerformanceRenderer } from './PerformanceRenderer.js';

export const HeaderStats = {
    cache: {
        turn: null,
        phase: null,
        score: null,
        energy: null,
        hearts: null,
        blades: null,
        p1: { deck: null, energy: null, discard: null },
        p2: { deck: null, energy: null, discard: null }
    },

    init: () => {
        HeaderStats.cache.turn = document.getElementById('turn');
        HeaderStats.cache.phase = document.getElementById('phase');
        HeaderStats.cache.score = document.getElementById('score');
        HeaderStats.cache.energy = document.getElementById('header-energy');
        HeaderStats.cache.hearts = document.getElementById('total-hearts-summary');
        HeaderStats.cache.blades = document.getElementById('total-blades-summary');
        HeaderStats.cache.p1.deck = document.getElementById('h-p1-deck');
        HeaderStats.cache.p1.energy = document.getElementById('h-p1-energy');
        HeaderStats.cache.p1.discard = document.getElementById('h-p1-discard');
        HeaderStats.cache.p2.deck = document.getElementById('h-p2-deck');
        HeaderStats.cache.p2.energy = document.getElementById('h-p2-energy');
        HeaderStats.cache.p2.discard = document.getElementById('h-p2-discard');
    },

    render: (state, p0, getPhaseKey) => {
        if (!HeaderStats.cache.turn) HeaderStats.init();

        const phaseKey = getPhaseKey(state.phase);
        
        if (HeaderStats.cache.turn) HeaderStats.cache.turn.textContent = state.turn || 1;

        // Update App Title with Total Card Count
        const appTitle = document.querySelector('[data-i18n="app_title"]');
        if (appTitle) {
            // Rust backend format: player1, player2
            const totalCards = [state.player1, state.player2].reduce((sum, p) => {
                if (!p) return sum;
                const handCards = p.hand?.cards?.length || 0;
                const energyCards = p.energy?.cards?.length || 0;
                const liveCards = p.live_zone?.cards?.length || 0;
                const successCards = p.success_live_card_zone?.cards?.length || 0;
                const stageCards = p.stage ? (p.stage.left_side ? 1 : 0) + (p.stage.center ? 1 : 0) + (p.stage.right_side ? 1 : 0) : 0;
                const deckCards = p.main_deck_count || 0;
                const energyDeckCards = p.energy_deck_count || 0;
                const discardCards = p.waitroom_count || 0;
                return sum + handCards + energyCards + liveCards + successCards + stageCards + deckCards + energyDeckCards + discardCards;
            }, 0);
            const baseTitle = i18n.t('app_title');
            appTitle.textContent = `${baseTitle} (${totalCards}) v3`;
        }

        if (HeaderStats.cache.phase) HeaderStats.cache.phase.textContent = i18n.t(phaseKey);

        if (HeaderStats.cache.score) {
            const p0Score = state.player1?.success_live_card_zone?.cards?.length || 0;
            const p1Score = state.player2?.success_live_card_zone?.cards?.length || 0;
            HeaderStats.cache.score.textContent = `${p0Score} - ${p1Score}`;
        }

        if (HeaderStats.cache.energy && p0) {
            // Rust backend: energy zone has active_energy_count
            const active = p0.energy?.active_energy_count || 0;
            const total = p0.energy?.cards?.length || 0;
            HeaderStats.cache.energy.textContent = `${active}/${total}`;
        }

        if (HeaderStats.cache.hearts && p0) {
            const hearts = p0.total_hearts || [0, 0, 0, 0, 0, 0, 0];
            HeaderStats.cache.hearts.innerHTML = PerformanceRenderer.renderHeartsCompact(hearts);
        }

        if (HeaderStats.cache.blades && p0) {
            const bladesCount = p0.total_blades !== undefined ? p0.total_blades : 0;
            HeaderStats.cache.blades.innerHTML = `<span class="stat-item" title="Total Blades">
                <img src="img/texticon/icon_blade.png" class="heart-mini-icon">
                <span class="stat-value">${bladesCount}</span>
            </span>`;
        }

        // Rust backend format
        if (state.player1) {
            if (HeaderStats.cache.p1.deck) HeaderStats.cache.p1.deck.textContent = state.player1.main_deck_count || 0;
            if (HeaderStats.cache.p1.energy) HeaderStats.cache.p1.energy.textContent = state.player1.energy_deck_count || 0;
            if (HeaderStats.cache.p1.discard) HeaderStats.cache.p1.discard.textContent = state.player1.waitroom_count || 0;
        }
        if (state.player2) {
            if (HeaderStats.cache.p2.deck) HeaderStats.cache.p2.deck.textContent = state.player2.main_deck_count || 0;
            if (HeaderStats.cache.p2.energy) HeaderStats.cache.p2.energy.textContent = state.player2.energy_deck_count || 0;
            if (HeaderStats.cache.p2.discard) HeaderStats.cache.p2.discard.textContent = state.player2.waitroom_count || 0;
        }
    }
};
