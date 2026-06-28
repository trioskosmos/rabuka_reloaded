/**
 * HeaderStats Component
 * Handles rendering of the game header (Turn, Phase, Energy, Scores, Hearts Summary).
 */
import { State } from '../state.js';
import * as i18n from '../i18n/index.js';
import { PerformanceRenderer } from './PerformanceRenderer.js';

export const HeaderStats = {
    cache: {
        turn: null,
        phase: null,
        activePlayer: null,
        frameCounter: null,
        actionLatency: null,
        player1Score: null,
        player2Score: null,
        energy: null,
        hearts: null,
        blades: null,
        player1Hearts: null,
        player2Hearts: null,
        player1Blades: null,
        player2Blades: null,
        player1Energy: null,
        player2Energy: null,
        player1HandCount: null,
        player2HandCount: null,
        p1: { deck: null, energy: null, discard: null },
        p2: { deck: null, energy: null, discard: null }
    },

    init: () => {
        HeaderStats.cache.turn = document.getElementById('turn');
        HeaderStats.cache.phase = document.getElementById('phase');
        HeaderStats.cache.activePlayer = document.getElementById('active-player');
        HeaderStats.cache.frameCounter = document.getElementById('frame-counter');
        HeaderStats.cache.actionLatency = document.getElementById('action-latency');
        HeaderStats.cache.player1Score = document.getElementById('player1-score');
        HeaderStats.cache.player2Score = document.getElementById('player2-score');
        HeaderStats.cache.energy = document.getElementById('header-energy');
        HeaderStats.cache.hearts = document.getElementById('total-hearts-summary');
        HeaderStats.cache.blades = document.getElementById('total-blades-summary');
        HeaderStats.cache.player1Hearts = document.getElementById('player1-hearts-summary');
        HeaderStats.cache.player2Hearts = document.getElementById('player2-hearts-summary');
        HeaderStats.cache.player1Blades = document.getElementById('player1-blades-summary');
        HeaderStats.cache.player2Blades = document.getElementById('player2-blades-summary');
        HeaderStats.cache.player1Energy = document.getElementById('player1-energy');
        HeaderStats.cache.player2Energy = document.getElementById('player2-energy');
        HeaderStats.cache.player1HandCount = document.getElementById('player1-hand-count');
        HeaderStats.cache.player2HandCount = document.getElementById('player2-hand-count');
        HeaderStats.cache.p1.deck = document.getElementById('h-p1-deck');
        HeaderStats.cache.p1.energy = document.getElementById('h-p1-energy');
        HeaderStats.cache.p1.discard = document.getElementById('h-p1-discard');
        HeaderStats.cache.p2.deck = document.getElementById('h-p2-deck');
        HeaderStats.cache.p2.energy = document.getElementById('h-p2-energy');
        HeaderStats.cache.p2.discard = document.getElementById('h-p2-discard');
    },

    // Compute need hearts for a set of selected hand card indices from local data
    computeLocalNeedHearts: (player) => {
        const hearts = [0, 0, 0, 0, 0, 0, 0];
        State.localLiveCardSelection.forEach(idx => {
            const handCard = player?.hand?.cards?.[idx];
            if (!handCard) return;
            const cardData = State.resolveCardData(handCard.card_no || handCard.card_id);
            if (!cardData) return;
            const needHeart = cardData.need_heart || cardData.required_hearts;
            if (needHeart) {
                for (const [heartType, count] of Object.entries(needHeart)) {
                    if (count && !heartType.startsWith('b_heart')) {
                        const ci = parseInt(heartType.replace('heart', '')) || 0;
                        if (ci < hearts.length) hearts[ci] += count;
                    }
                }
            }
        });
        return hearts;
    },

    render: (state, _p0, _p1, getPhaseKey) => {
        if (!HeaderStats.cache.turn) HeaderStats.init();

        // Use player1/player2 directly — P1 always top, P2 always bottom
        const p0 = state.player1 || {};
        const p1 = state.player2 || {};

        const perspective = State.perspectivePlayer;
        const selfLabel = perspective === 0 ? 'P1' : 'P2';
        const oppLabel = perspective === 0 ? 'P2' : 'P1';
        const p1LabelEls = document.querySelectorAll('[data-i18n="player1"]');
        const p2LabelEls = document.querySelectorAll('[data-i18n="player2"]');
        p1LabelEls.forEach(el => el.textContent = selfLabel);
        p2LabelEls.forEach(el => el.textContent = oppLabel);

        const phaseKey = getPhaseKey(state.phase);
        const isSetPhase = state.phase === 'LiveCardSetFirstAttacker' || state.phase === 'LiveCardSetSecondAttacker';

        if (HeaderStats.cache.turn) HeaderStats.cache.turn.textContent = state.turn || 1;
        if (HeaderStats.cache.phase) HeaderStats.cache.phase.textContent = i18n.t(phaseKey);
        if (HeaderStats.cache.activePlayer) {
            const ap = state.active_player;
            const apLabel = ap === 'player1' || ap === '0' ? 'P1' : ap === 'player2' || ap === '1' ? 'P2' : ap || 'P1';
            HeaderStats.cache.activePlayer.textContent = apLabel;
        }
        if (HeaderStats.cache.frameCounter) {
            HeaderStats.cache.frameCounter.textContent = state._frameCounter ?? 0;
        }
        if (HeaderStats.cache.actionLatency) {
            const lat = State._actionLatency;
            if (lat < 0) {
                HeaderStats.cache.actionLatency.textContent = '---';
            } else {
                HeaderStats.cache.actionLatency.textContent = `${lat}ms`;
                HeaderStats.cache.actionLatency.style.color = lat > 2000 ? 'var(--accent-pink)' : lat > 800 ? 'var(--accent-yellow)' : '';
            }
        }

        // Score display — show current_score from backend
        if (HeaderStats.cache.player1Score) {
            const sc = p0.current_score ?? 0;
            HeaderStats.cache.player1Score.textContent = `${sc}`;
            HeaderStats.cache.player1Score.title = `Current score: ${sc} (base + ability modifiers)`;
        }
        if (HeaderStats.cache.player2Score) {
            const sc = p1.current_score ?? 0;
            HeaderStats.cache.player2Score.textContent = `${sc}`;
            HeaderStats.cache.player2Score.title = `Current score: ${sc} (base + ability modifiers)`;
        }

        // P1 Energy
        if (HeaderStats.cache.player1Energy) {
            const active = (p0.energy?.cards || []).filter(e => e && e.orientation === 'Active').length;
            const total = (p0.energy?.cards || []).length;
            HeaderStats.cache.player1Energy.textContent = `${active}/${total}`;
        }

        // P2 Energy
        if (HeaderStats.cache.player2Energy) {
            const active = (p1.energy?.cards || []).filter(e => e && e.orientation === 'Active').length;
            const total = (p1.energy?.cards || []).length;
            HeaderStats.cache.player2Energy.textContent = `${active}/${total}`;
        }

        // Hand Counts
        if (HeaderStats.cache.player1HandCount) {
            HeaderStats.cache.player1HandCount.textContent = (p0.hand?.cards || []).length;
        }
        if (HeaderStats.cache.player2HandCount) {
            HeaderStats.cache.player2HandCount.textContent = (p1.hand?.cards || []).length;
        }

        // Helper: render hearts section with stage + need hearts
        const renderHeartsSection = (player, heartsEl, bladesEl, label) => {
            if (!heartsEl) return;

            // Stage hearts
            let hearts = player.total_hearts;
            if (!hearts || hearts.length === 0) {
                hearts = [0, 0, 0, 0, 0, 0, 0];
                if (player.stage) {
                    const members = [player.stage.left_side, player.stage.center, player.stage.right_side];
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

            heartsEl.style.flexDirection = 'column';

            let html = '<div class="summary-hearts-row">' + PerformanceRenderer.renderHeartsCompact(hearts) + '</div>';

            // Need hearts: backend live_need_hearts (after confirm) or local preview (during set phase)
            let needHearts = null;
            if (isSetPhase && State.localLiveCardSelection.size > 0) {
                needHearts = HeaderStats.computeLocalNeedHearts(player);
            } else if (player.live_need_hearts && player.live_need_hearts.some(v => v > 0)) {
                needHearts = player.live_need_hearts;
            }

            if (needHearts) {
                html += '<div class="summary-hearts-row summary-need-hearts">' + PerformanceRenderer.renderHeartsCompact(needHearts) + '</div>';
            }

            heartsEl.innerHTML = html;

            // Blades
            if (bladesEl) {
                let bladesCount = player.total_blades;
                if (bladesCount === undefined) {
                    bladesCount = 0;
                    if (player.stage) {
                        const members = [player.stage.left_side, player.stage.center, player.stage.right_side];
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
                bladesEl.innerHTML = `<span class="stat-item" title="${label} Blades">
                    <img src="img/texticon/icon_blade.png" class="heart-mini-icon">
                    <span class="stat-value">${bladesCount}</span>
                </span>`;
            }
        };

        // P1 Hearts and Blades
        renderHeartsSection(p0, HeaderStats.cache.player1Hearts, HeaderStats.cache.player1Blades, 'P1');

        // P2 Hearts and Blades
        renderHeartsSection(p1, HeaderStats.cache.player2Hearts, HeaderStats.cache.player2Blades, 'P2');

        // Deck / Energy / Discard counts
        if (state.player1) {
            if (HeaderStats.cache.p1.deck) HeaderStats.cache.p1.deck.textContent = state.player1.main_deck_count;
            if (HeaderStats.cache.p1.energy) HeaderStats.cache.p1.energy.textContent = state.player1.energy_deck_count;
            if (HeaderStats.cache.p1.discard) HeaderStats.cache.p1.discard.textContent = (state.player1.waitroom?.cards?.length || state.player1.discard?.cards?.length || 0);
        }
        if (state.player2) {
            if (HeaderStats.cache.p2.deck) HeaderStats.cache.p2.deck.textContent = state.player2.main_deck_count;
            if (HeaderStats.cache.p2.energy) HeaderStats.cache.p2.energy.textContent = state.player2.energy_deck_count;
            if (HeaderStats.cache.p2.discard) HeaderStats.cache.p2.discard.textContent = (state.player2.waitroom?.cards?.length || state.player2.discard?.cards?.length || 0);
        }
    }
};
