/**
 * HeaderStats Component
 * Handles rendering of the game header (Turn, Phase, Energy, Scores, Hearts Summary).
 */
import { State } from '../state.js';
import * as i18n from '../i18n/index.js';
import { PerformanceRenderer } from './PerformanceRenderer.js';
import { sourceName } from '../utils/Attribution.js';

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
        player1NeedHearts: null,
        player2NeedHearts: null,
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
        HeaderStats.cache.player1NeedHearts = document.getElementById('player1-need-hearts');
        HeaderStats.cache.player2NeedHearts = document.getElementById('player2-need-hearts');
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

    // Live-score preview for the perspective player's currently selected live
    // cards (during the live set phase). Returns the running total = the stage's
    // existing live bonuses (current_score) + the sum of each selected card's
    // own score. Returns current_score when nothing is selected.
    computeLocalLiveScore: (player) => {
        let selected = 0;
        State.localLiveCardSelection.forEach(idx => {
            const handCard = player?.hand?.cards?.[idx];
            if (!handCard) return;
            const cardData = State.resolveCardData(handCard.card_no || handCard.card_id);
            if (!cardData) return;
            selected += cardData.score || 0;
        });
        return (player.current_score || 0) + selected;
    },

    // ── Blade drill-down ────────────────────────────────────────────
    // Clicking a blades summary shows a popover breaking the total down by
    // granting card (from engine effect_attribution).
    toggleBladeBreakdown: (player, anchorEl) => {
        const existing = document.getElementById('blade-breakdown-popover');
        if (existing) { existing.remove(); return; }

        const members = [player.stage?.left_side, player.stage?.center, player.stage?.right_side]
            .filter(m => m && typeof m.id === 'number' && m.id >= 0);
        const bySource = new Map();
        const attr = State.data?.effect_attribution || {};
        members.forEach(m => {
            (attr[m.id] || []).forEach(e => {
                if (e.kind !== 'blade' || !e.amount) return;
                if (!bySource.has(e.source_card_id)) {
                    bySource.set(e.source_card_id, { name: sourceName(e.source_card_id), text: e.ability_text, total: 0, targets: [] });
                }
                const rec = bySource.get(e.source_card_id);
                rec.total += e.amount;
                rec.targets.push(m.name || '?');
            });
        });

        const pop = document.createElement('div');
        pop.id = 'blade-breakdown-popover';
        pop.className = 'blade-breakdown-popover';
        const title = document.createElement('div');
        title.className = 'attr-title';
        title.textContent = i18n.t('attr_active_effects');
        pop.appendChild(title);
        if (!bySource.size) {
            const empty = document.createElement('div');
            empty.style.cssText = 'font-size:0.72rem;opacity:0.6;';
            empty.textContent = i18n.t('attr_none');
            pop.appendChild(empty);
        } else {
            bySource.forEach(rec => {
                const row = document.createElement('div');
                row.className = 'attr-entry';
                const body = document.createElement('div');
                body.className = 'attr-entry-body';
                const line1 = document.createElement('div');
                line1.className = 'attr-line1';
                line1.innerHTML = `<b>${rec.total > 0 ? '+' : ''}${rec.total}</b> <span class="attr-from">${i18n.t('attr_from', { card: rec.name })}</span>`;
                body.appendChild(line1);
                if (rec.text) {
                    const line2 = document.createElement('div');
                    line2.className = 'attr-line2';
                    line2.textContent = rec.text;
                    body.appendChild(line2);
                }
                row.appendChild(body);
                pop.appendChild(row);
            });
        }
        document.body.appendChild(pop);

        const rect = anchorEl.getBoundingClientRect();
        pop.style.left = Math.min(rect.left, window.innerWidth - pop.offsetWidth - 8) + 'px';
        pop.style.top = (rect.bottom + 4) + 'px';

        setTimeout(() => {
            const close = (ev) => {
                if (pop.contains(ev.target)) return;
                pop.remove();
                document.removeEventListener('click', close, true);
            };
            document.addEventListener('click', close, true);
        }, 0);
    },

    render: (state, _p0, _p1, getPhaseKey) => {
        if (!HeaderStats.cache.turn) HeaderStats.init();

        // Column layout: col-meta | col-p1 = perspective player | col-p2 = opponent
        const p0 = state.player1 || {};
        const p1 = state.player2 || {};
        const perspective = State.perspectivePlayer;
        const pPerspective = perspective === 0 ? p0 : p1;
        const pOpponent   = perspective === 0 ? p1 : p0;

        // Update column labels: col-p1 = perspective player, col-p2 = opponent
        const col1Label = document.querySelector('.col-p1 .stat-label[data-i18n]');
        const col2Label = document.querySelector('.col-p2 .stat-label[data-i18n]');
        if (col1Label) col1Label.textContent = perspective === 0 ? i18n.t('player1') : i18n.t('player2');
        if (col2Label) col2Label.textContent = perspective === 0 ? i18n.t('player2') : i18n.t('player1');

        const phaseKey = getPhaseKey(state.phase);
        const isSetPhase = state.phase === 'LiveCardSetFirstAttacker' || state.phase === 'LiveCardSetSecondAttacker';

        if (HeaderStats.cache.turn) HeaderStats.cache.turn.textContent = state.turn || 1;
        if (HeaderStats.cache.phase) HeaderStats.cache.phase.textContent = i18n.t(phaseKey);
        if (HeaderStats.cache.activePlayer) {
            const ap = state.active_player;
            const apLabel = ap === 'player1' || ap === 'p1' || ap === '0' ? 'P1' : ap === 'player2' || ap === 'p2' || ap === '1' ? 'P2' : ap || 'P1';
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

        // Score display — col-p1 = perspective, col-p2 = opponent.
        // During the live set phase, the perspective player's score previews the
        // live total of their currently selected live cards (e.g. three score-2
        // cards => 6). Otherwise fall back to the engine's current_score.
        const scoreFor = (p, isPerspective) => {
            if (isSetPhase && isPerspective && State.localLiveCardSelection.size > 0) {
                return `${HeaderStats.computeLocalLiveScore(p)}`;
            }
            return `${p.current_score ?? 0}`;
        };
        if (HeaderStats.cache.player1Score) {
            HeaderStats.cache.player1Score.textContent = scoreFor(pPerspective, true);
        }
        if (HeaderStats.cache.player2Score) {
            HeaderStats.cache.player2Score.textContent = scoreFor(pOpponent, false);
        }

        // Energy — col-p1 = perspective, col-p2 = opponent
        if (HeaderStats.cache.player1Energy) {
            const active = (pPerspective.energy?.cards || []).filter(e => e && e.orientation === 'Active').length;
            const total = (pPerspective.energy?.cards || []).length;
            HeaderStats.cache.player1Energy.textContent = `${active}/${total}`;
        }
        if (HeaderStats.cache.player2Energy) {
            const active = (pOpponent.energy?.cards || []).filter(e => e && e.orientation === 'Active').length;
            const total = (pOpponent.energy?.cards || []).length;
            HeaderStats.cache.player2Energy.textContent = `${active}/${total}`;
        }

        // Hand Counts — col-p1 = perspective, col-p2 = opponent
        if (HeaderStats.cache.player1HandCount) {
            HeaderStats.cache.player1HandCount.textContent = (pPerspective.hand?.cards || []).length;
        }
        if (HeaderStats.cache.player2HandCount) {
            HeaderStats.cache.player2HandCount.textContent = (pOpponent.hand?.cards || []).length;
        }

        // Helper: compute stage hearts from player data
        const getStageHearts = (player) => {
            let hearts = player.total_hearts;
            if (!hearts || hearts.length === 0) {
                hearts = [0, 0, 0, 0, 0, 0, 0];
                if (player.stage) {
                    const members = [player.stage.left_side, player.stage.center, player.stage.right_side];
                    members.forEach(member => {
                        if (member && member.card_no) {
                            const card = State.resolveCardData(member.card_no);
                            const heartData = card.base_heart || card.hearts;
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
            return hearts;
        };

        // Helper: compute blades count
        const getBladesCount = (player) => {
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
            return bladesCount;
        };

        // Helper: get need hearts (backend or local preview)
        const getNeedHearts = (player, isPerspectivePlayer, isOpponent) => {
            if (isSetPhase && isPerspectivePlayer && State.localLiveCardSelection.size > 0) {
                return HeaderStats.computeLocalNeedHearts(player);
            }
            // Rule 8.2.x: live cards are face-down until performance.
            // Opponent's need hearts are hidden until their cards are revealed.
            if (isOpponent) {
                const isOppFirst = perspective === 0 ? !!p1.is_first_attacker : !!p0.is_first_attacker;
                const performed = state.phase === 'SecondAttackerPerformance'
                    || state.phase === 'LiveVictoryDetermination'
                    || (state.phase === 'FirstAttackerPerformance' && isOppFirst);
                if (!performed) return null;
            }
            if (player.live_need_hearts && player.live_need_hearts.some(v => v > 0)) {
                return player.live_need_hearts;
            }
            return null;
        };

        // --- Col-P1 (perspective player): stage hearts + blades + need hearts ---
        if (HeaderStats.cache.player1Hearts) {
            HeaderStats.cache.player1Hearts.innerHTML = PerformanceRenderer.renderHeartsCompact(getStageHearts(pPerspective));
        }
        if (HeaderStats.cache.player1Blades) {
            const b = getBladesCount(pPerspective);
            HeaderStats.cache.player1Blades.innerHTML = `<span class="stat-item stat-item-clickable" title="${i18n.t('attr_active_effects')}">
                <img src="img/texticon/icon_blade.png" class="heart-mini-icon">
                <span class="stat-value">${b}</span>
            </span>`;
            HeaderStats.cache.player1Blades.onclick = (ev) => HeaderStats.toggleBladeBreakdown(pPerspective, HeaderStats.cache.player1Blades);
        }
        if (HeaderStats.cache.player1NeedHearts) {
            const nh = getNeedHearts(pPerspective, true, false);
            HeaderStats.cache.player1NeedHearts.innerHTML = nh ? '<span class="stat-separator"></span>' + PerformanceRenderer.renderHeartsCompact(nh) : '';
        }

        // --- Col-P2 (opponent): stage hearts + blades + need hearts ---
        if (HeaderStats.cache.player2Hearts) {
            HeaderStats.cache.player2Hearts.innerHTML = PerformanceRenderer.renderHeartsCompact(getStageHearts(pOpponent));
        }
        if (HeaderStats.cache.player2Blades) {
            const b = getBladesCount(pOpponent);
            HeaderStats.cache.player2Blades.innerHTML = `<span class="stat-item stat-item-clickable" title="${i18n.t('attr_active_effects')}">
                <img src="img/texticon/icon_blade.png" class="heart-mini-icon">
                <span class="stat-value">${b}</span>
            </span>`;
            HeaderStats.cache.player2Blades.onclick = (ev) => HeaderStats.toggleBladeBreakdown(pOpponent, HeaderStats.cache.player2Blades);
        }
        if (HeaderStats.cache.player2NeedHearts) {
            const nh = getNeedHearts(pOpponent, false, true);
            HeaderStats.cache.player2NeedHearts.innerHTML = nh ? '<span class="stat-separator"></span>' + PerformanceRenderer.renderHeartsCompact(nh) : '';
        }

        // Deck / Energy / Discard counts — col-p1 = perspective, col-p2 = opponent
        if (HeaderStats.cache.p1.deck) HeaderStats.cache.p1.deck.textContent = pPerspective.main_deck_count ?? 0;
        if (HeaderStats.cache.p1.energy) HeaderStats.cache.p1.energy.textContent = pPerspective.energy_deck_count ?? 0;
        if (HeaderStats.cache.p1.discard) HeaderStats.cache.p1.discard.textContent = (pPerspective.waitroom?.cards?.length || pPerspective.discard?.cards?.length || 0);
        if (HeaderStats.cache.p2.deck) HeaderStats.cache.p2.deck.textContent = pOpponent.main_deck_count ?? 0;
        if (HeaderStats.cache.p2.energy) HeaderStats.cache.p2.energy.textContent = pOpponent.energy_deck_count ?? 0;
        if (HeaderStats.cache.p2.discard) HeaderStats.cache.p2.discard.textContent = (pOpponent.waitroom?.cards?.length || pOpponent.discard?.cards?.length || 0);
    }
};
