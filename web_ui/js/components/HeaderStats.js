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
        player1Score: null,
        player2Score: null,
        energy: null,
        hearts: null,
        blades: null,
        player1Hearts: null,
        player2Hearts: null,
        player1Blades: null,
        player2Blades: null,
        p1: { deck: null, energy: null, discard: null },
        p2: { deck: null, energy: null, discard: null }
    },

    init: () => {
        HeaderStats.cache.turn = document.getElementById('turn');
        HeaderStats.cache.phase = document.getElementById('phase');
        HeaderStats.cache.player1Score = document.getElementById('player1-score');
        HeaderStats.cache.player2Score = document.getElementById('player2-score');
        HeaderStats.cache.energy = document.getElementById('header-energy');
        HeaderStats.cache.hearts = document.getElementById('total-hearts-summary');
        HeaderStats.cache.blades = document.getElementById('total-blades-summary');
        HeaderStats.cache.player1Hearts = document.getElementById('player1-hearts-summary');
        HeaderStats.cache.player2Hearts = document.getElementById('player2-hearts-summary');
        HeaderStats.cache.player1Blades = document.getElementById('player1-blades-summary');
        HeaderStats.cache.player2Blades = document.getElementById('player2-blades-summary');
        HeaderStats.cache.p1.deck = document.getElementById('h-p1-deck');
        HeaderStats.cache.p1.energy = document.getElementById('h-p1-energy');
        HeaderStats.cache.p1.discard = document.getElementById('h-p1-discard');
        HeaderStats.cache.p2.deck = document.getElementById('h-p2-deck');
        HeaderStats.cache.p2.energy = document.getElementById('h-p2-energy');
        HeaderStats.cache.p2.discard = document.getElementById('h-p2-discard');
    },

    render: (state, p0, p1, getPhaseKey) => {
        if (!HeaderStats.cache.turn) HeaderStats.init();

        const phaseKey = getPhaseKey(state.phase);
        
        if (HeaderStats.cache.turn) HeaderStats.cache.turn.textContent = state.turn || 1;
        if (HeaderStats.cache.phase) HeaderStats.cache.phase.textContent = i18n.t(phaseKey);

        if (HeaderStats.cache.player1Score && HeaderStats.cache.player2Score) {
            // Calculate base scores from success_live_card_zone
            const p0Success = p0.success_live_card_zone?.cards || [];
            const p1Success = p1.success_live_card_zone?.cards || [];
            const p0BaseScore = p0Success.length || 0;
            const p1BaseScore = p1Success.length || 0;

            // Calculate ability bonuses from score modifiers (handle missing data gracefully)
            let p0Bonus = 0;
            let p1Bonus = 0;

            // Only calculate bonuses if score_modifiers data exists
            if (p0.score_modifiers) {
                for (const cardId of p0Success) {
                    if (p0.score_modifiers[cardId]) {
                        p0Bonus += p0.score_modifiers[cardId];
                    }
                }
            }
            if (p1.score_modifiers) {
                for (const cardId of p1Success) {
                    if (p1.score_modifiers[cardId]) {
                        p1Bonus += p1.score_modifiers[cardId];
                    }
                }
            }

            // Display in "base +bonus" format
            HeaderStats.cache.player1Score.textContent = `${p0BaseScore} +${p0Bonus}`;
            HeaderStats.cache.player2Score.textContent = `${p1BaseScore} +${p1Bonus}`;
        }

        if (HeaderStats.cache.energy && p0) {
            // Engine sends orientation in each energy card - calculate active count
            const active = p0.energy.cards.filter(e => e && e.orientation === 'Active').length;
            const total = p0.energy.cards.length;
            HeaderStats.cache.energy.textContent = `${active}/${total}`;
        }

        // P1 Hearts and Blades
        if (HeaderStats.cache.player1Hearts && p0) {
            let hearts = p0.total_hearts;
            if (!hearts || hearts.length === 0) {
                hearts = [0, 0, 0, 0, 0, 0, 0];
                if (p0.stage) {
                    const members = [p0.stage.left_side, p0.stage.center, p0.stage.right_side];
                    members.forEach(member => {
                        if (member && member.card_no) {
                            const card = State.resolveCardData(member.card_no);
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
            HeaderStats.cache.player1Hearts.innerHTML = PerformanceRenderer.renderHeartsCompact(hearts);
        }

        if (HeaderStats.cache.player1Blades && p0) {
            let bladesCount = p0.total_blades;
            if (bladesCount === undefined) {
                bladesCount = 0;
                if (p0.stage) {
                    const members = [p0.stage.left_side, p0.stage.center, p0.stage.right_side];
                    members.forEach(member => {
                        if (member) {
                            if (member.total_blade !== undefined) {
                                bladesCount += member.total_blade;
                            } else if (member.card_no) {
                                const card = State.resolveCardData(member.card_no);
                                if (card && (card.blade || card.blades)) {
                                    bladesCount += card.blade || card.blades || 0;
                                }
                            }
                        }
                    });
                }
            }
            HeaderStats.cache.player1Blades.innerHTML = `<span class="stat-item" title="P1 Blades">
                <img src="img/texticon/icon_blade.png" class="heart-mini-icon">
                <span class="stat-value">${bladesCount}</span>
            </span>`;
        }

        // P2 Hearts and Blades
        if (HeaderStats.cache.player2Hearts && p1) {
            let hearts = p1.total_hearts;
            if (!hearts || hearts.length === 0) {
                hearts = [0, 0, 0, 0, 0, 0, 0];
                if (p1.stage) {
                    const members = [p1.stage.left_side, p1.stage.center, p1.stage.right_side];
                    members.forEach(member => {
                        if (member && member.card_no) {
                            const card = State.resolveCardData(member.card_no);
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
            HeaderStats.cache.player2Hearts.innerHTML = PerformanceRenderer.renderHeartsCompact(hearts);
        }

        if (HeaderStats.cache.player2Blades && p1) {
            let bladesCount = p1.total_blades;
            if (bladesCount === undefined) {
                bladesCount = 0;
                if (p1.stage) {
                    const members = [p1.stage.left_side, p1.stage.center, p1.stage.right_side];
                    members.forEach(member => {
                        if (member) {
                            if (member.total_blade !== undefined) {
                                bladesCount += member.total_blade;
                            } else if (member.card_no) {
                                const card = State.resolveCardData(member.card_no);
                                if (card && (card.blade || card.blades)) {
                                    bladesCount += card.blade || card.blades || 0;
                                }
                            }
                        }
                    });
                }
            }
            HeaderStats.cache.player2Blades.innerHTML = `<span class="stat-item" title="P2 Blades">
                <img src="img/texticon/icon_blade.png" class="heart-mini-icon">
                <span class="stat-value">${bladesCount}</span>
            </span>`;
        }

        if (state.player1) {
            if (HeaderStats.cache.p1.deck) HeaderStats.cache.p1.deck.textContent = state.player1.main_deck_count;
            if (HeaderStats.cache.p1.energy) HeaderStats.cache.p1.energy.textContent = state.player1.energy_deck_count;
            // Engine sends waitroom zone, calculate count from cards
            if (HeaderStats.cache.p1.discard) HeaderStats.cache.p1.discard.textContent = (state.player1.waitroom?.cards?.length || state.player1.discard?.cards?.length || 0);
        }
        if (state.player2) {
            if (HeaderStats.cache.p2.deck) HeaderStats.cache.p2.deck.textContent = state.player2.main_deck_count;
            if (HeaderStats.cache.p2.energy) HeaderStats.cache.p2.energy.textContent = state.player2.energy_deck_count;
            // Engine sends waitroom zone, calculate count from cards
            if (HeaderStats.cache.p2.discard) HeaderStats.cache.p2.discard.textContent = (state.player2.waitroom?.cards?.length || state.player2.discard?.cards?.length || 0);
        }
    }
};
