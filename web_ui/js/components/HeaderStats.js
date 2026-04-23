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
        if (HeaderStats.cache.phase) HeaderStats.cache.phase.textContent = i18n.t(phaseKey);

        if (HeaderStats.cache.score) {
            const p0Success = state.player1.success_live_card_zone.cards;
            const p1Success = state.player2.success_live_card_zone.cards;
            const p0Score = p0Success.length || 0;
            const p1Score = p1Success.length || 0;
            HeaderStats.cache.score.textContent = `${p0Score} - ${p1Score}`;
        }

        if (HeaderStats.cache.energy && p0) {
            const active = p0.energy.active_energy_count || 0;
            const total = p0.energy.cards.length;
            HeaderStats.cache.energy.textContent = `${active}/${total}`;
        }

        if (HeaderStats.cache.hearts && p0) {
            // Calculate hearts from stage if total_hearts not provided
            let hearts = p0.total_hearts;
            if (!hearts || hearts.length === 0) {
                // Calculate from stage members
                hearts = [0, 0, 0, 0, 0, 0, 0];
                if (p0.stage) {
                    const members = [p0.stage.left_side, p0.stage.center, p0.stage.right_side];
                    members.forEach(member => {
                        if (member && member.card_no) {
                            const card = State.resolveCardData(member.card_no);
                            // Support both base_heart and hearts field names
                            const heartData = card.base_heart || card.hearts || card.required_hearts;
                            if (heartData) {
                                for (const [heartType, count] of Object.entries(heartData)) {
                                    if (count && !heartType.startsWith('b_heart')) {
                                        const idx = parseInt(heartType.replace('heart', '')) || 0;
                                        if (idx < hearts.length) hearts[idx] += count;
                                    }
                                }
                            }
                        }
                    });
                }
            }
            HeaderStats.cache.hearts.innerHTML = PerformanceRenderer.renderHeartsCompact(hearts);
        }

        if (HeaderStats.cache.blades && p0) {
            // Calculate blades from stage if total_blades not provided
            let bladesCount = p0.total_blades;
            if (bladesCount === undefined) {
                bladesCount = 0;
                if (p0.stage) {
                    const members = [p0.stage.left_side, p0.stage.center, p0.stage.right_side];
                    members.forEach(member => {
                        if (member && member.card_no) {
                            const card = State.resolveCardData(member.card_no);
                            // Support both blade and blades field names
                            if (card && (card.blade || card.blades)) {
                                bladesCount += card.blade || card.blades || 0;
                            }
                        }
                    });
                }
            }
            HeaderStats.cache.blades.innerHTML = `<span class="stat-item" title="Total Blades">
                <img src="img/texticon/icon_blade.png" class="heart-mini-icon">
                <span class="stat-value">${bladesCount}</span>
            </span>`;
        }

        if (state.player1) {
            if (HeaderStats.cache.p1.deck) HeaderStats.cache.p1.deck.textContent = state.player1.main_deck_count;
            if (HeaderStats.cache.p1.energy) HeaderStats.cache.p1.energy.textContent = state.player1.energy_deck_count;
            if (HeaderStats.cache.p1.discard) HeaderStats.cache.p1.discard.textContent = state.player1.waitroom_count;
        }
        if (state.player2) {
            if (HeaderStats.cache.p2.deck) HeaderStats.cache.p2.deck.textContent = state.player2.main_deck_count;
            if (HeaderStats.cache.p2.energy) HeaderStats.cache.p2.energy.textContent = state.player2.energy_deck_count;
            if (HeaderStats.cache.p2.discard) HeaderStats.cache.p2.discard.textContent = state.player2.waitroom_count;
        }
    }
};
