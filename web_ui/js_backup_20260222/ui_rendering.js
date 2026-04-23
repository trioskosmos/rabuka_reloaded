/**
 * UI Rendering Module
 * Handles all board, card, and performance result rendering.
 */
import { State } from './state.js';
import { Phase, fixImg } from './constants.js';
import { translations } from './translations_data.js';
import { Tooltips } from './ui_tooltips.js';

export const Rendering = {
    render: () => {
        if (State.renderRequested) return;
        State.renderRequested = true;
        requestAnimationFrame(() => {
            Rendering.renderInternal();
            State.renderRequested = false;
        });
    },

    renderHeaderStats: (state, p0, p1, t) => {
        // RPS, Setup, etc. phase names
        let phaseKey = Rendering.getPhaseKey(state.phase);

        const turnEl = document.getElementById('turn');
        if (turnEl) turnEl.textContent = state.turn_number || state.turn || 1;

        const phaseEl = document.getElementById('phase');
        if (phaseEl) phaseEl.textContent = t[phaseKey] || state.phase;

        const scoreEl = document.getElementById('score');
        if (scoreEl) {
            const p0Score = state.players[0].success_lives ? state.players[0].success_lives.length : 0;
            const p1Score = state.players[1].success_lives ? state.players[1].success_lives.length : 0;
            scoreEl.textContent = `${p0Score} - ${p1Score}`;
        }

        // Energy and Hearts
        const energyEl = document.getElementById('header-energy');
        if (energyEl && p0) {
            energyEl.textContent = `${p0.energy_untapped || 0}/${p0.energy_count || 0}`;
        }

        // Hearts summary
        const totalHearts = document.getElementById('total-hearts-summary');
        if (totalHearts && p0) {
            const hearts = p0.total_hearts || [0, 0, 0, 0, 0, 0, 0];
            totalHearts.innerHTML = Rendering.renderHeartsCompact(hearts);
        }

        // Blades summary
        const totalBladesSummary = document.getElementById('total-blades-summary');
        if (totalBladesSummary && p0) {
            const bladesCount = p0.total_blades !== undefined ? p0.total_blades : 0;
            totalBladesSummary.innerHTML = `<span class="stat-item" title="Total Blades">
                <img src="img/texticon/icon_blade.png" class="heart-mini-icon">
                <span class="stat-value">${bladesCount}</span>
             </span>`;
        }
    },

    get_valid_targets: (state) => {
        const valid = {
            myHand: {},
            oppHand: {},
            myStage: {},
            oppStage: {},
            myLive: {},
            oppLive: {},
            myEnergy: {},
            oppEnergy: {},
            discard: {},
            hasSelection: false
        };

        if (!state.legal_actions) return valid;

        state.legal_actions.forEach(a => {
            const m = a.metadata || {};
            const hIdx = a.hand_idx ?? m.hand_idx;
            const sIdx = a.slot_idx ?? m.slot_idx;
            const srcIdx = a.source_idx ?? m.source_idx;
            const eIdx = m.energy_idx;
            const tPlayer = m.target_player !== undefined ? m.target_player : State.perspectivePlayer;
            const isMe = (tPlayer === State.perspectivePlayer);

            if (hIdx !== undefined) {
                if (isMe) valid.myHand[hIdx] = a.id;
                else valid.oppHand[hIdx] = a.id;
            }

            if (sIdx !== undefined) {
                // Determine if it's a stage target or live target
                if (a.type !== 'PLAY' && a.type !== 'LIVE_SET' && m.category !== 'LIVE') {
                    if (isMe) valid.myStage[sIdx] = a.id;
                    else valid.oppStage[sIdx] = a.id;
                }
            }
            if (srcIdx !== undefined) {
                if (isMe) valid.myStage[srcIdx] = a.id;
                else valid.oppStage[srcIdx] = a.id;
            }

            if (a.type === 'LIVE_PERFORM' || m.category === 'LIVE') {
                const liveIdx = sIdx !== undefined ? sIdx : (a.id >= 600 && a.id < 610 ? a.id - 600 : (a.id >= 900 && a.id <= 902 ? a.id - 900 : undefined));
                if (liveIdx !== undefined) {
                    if (isMe) valid.myLive[liveIdx] = a.id;
                    else valid.oppLive[liveIdx] = a.id;
                }
            }

            if (a.type === 'SELECT_DISCARD' || m.from_discard || m.category === 'DISCARD') {
                valid.discard['all'] = a.id;
            }

            if (eIdx !== undefined) {
                if (isMe) valid.myEnergy[eIdx] = a.id;
                else valid.oppEnergy[eIdx] = a.id;
            }
        });

        const hasCardActions = (Object.keys(valid.myHand).length + Object.keys(valid.myStage).length + Object.keys(valid.myLive).length +
            Object.keys(valid.oppHand).length + Object.keys(valid.oppStage).length + Object.keys(valid.oppLive).length) > 0;
        valid.hasSelection = hasCardActions;

        if (state.pending_choice && state.pending_choice.options) {
            valid.hasSelection = true;
            valid.myHand = {}; valid.oppHand = {};
            valid.myStage = {}; valid.oppStage = {};
            valid.myLive = {}; valid.oppLive = {};
            valid.myEnergy = {}; valid.oppEnergy = {};

            state.pending_choice.options.forEach((opt, idx) => {
                const actionId = state.pending_choice.actions[idx];
                const tPlayer = opt.target_player !== undefined ? opt.target_player : State.perspectivePlayer;
                const isMe = (tPlayer === State.perspectivePlayer);

                if (opt.hand_idx !== undefined) { if (isMe) valid.myHand[opt.hand_idx] = actionId; else valid.oppHand[opt.hand_idx] = actionId; }
                if (opt.slot_idx !== undefined) { if (isMe) valid.myStage[opt.slot_idx] = actionId; else valid.oppStage[opt.slot_idx] = actionId; }
                if (opt.energy_idx !== undefined) { if (isMe) valid.myEnergy[opt.energy_idx] = actionId; else valid.oppEnergy[opt.energy_idx] = actionId; }
            });
        }

        return valid;
    },

    renderInternal: () => {
        const state = State.data;
        if (!state || !state.players) return;

        const perspectivePlayer = State.perspectivePlayer;
        const currentLang = State.currentLang;
        const t = translations[currentLang];

        // --- Proactive Pre-loading ---
        const assetsToLoad = [];
        state.players.forEach(p => {
            if (p.hand) p.hand.forEach(c => { if (c && c.img) assetsToLoad.push(fixImg(c.img)); });
            if (p.stage) p.stage.forEach(c => { if (c && c.img) assetsToLoad.push(fixImg(c.img)); });
        });
        if (state.legal_actions) {
            state.legal_actions.forEach(a => { if (a && a.img) assetsToLoad.push(fixImg(a.img)); });
        }

        const assetsHash = assetsToLoad.join('|');
        if (State.lastAssetsHash !== assetsHash) {
            if (window.preloadAssets) window.preloadAssets(assetsToLoad);
            State.lastAssetsHash = assetsHash;
        }

        if (State.hotseatMode && state.active_player !== undefined) {
            State.perspectivePlayer = state.active_player;
        }

        const p0 = state.players[State.perspectivePlayer] || state.players[0];
        const p1 = state.players[1 - State.perspectivePlayer] || state.players[1];

        if (p0) state.looked_cards = p0.looked_cards || [];
        if (!p0 || !p1) return;

        // Calculate Valid Targets for Highlighting
        const validTargets = Rendering.get_valid_targets(state);

        // Update UI Headers, Stats, etc. (Logic moved from main.js)
        Rendering.renderHeaderStats(state, p0, p1, t);
        Rendering.renderBoard(state, p0, p1, validTargets);

        let selectedIndices = [];
        if (state.phase === Phase.MULLIGAN_P1 || state.phase === Phase.MULLIGAN_P2) {
            selectedIndices = Array.from(p0.mulligan_selection || []);
        } else {
            if (State.selectedHandIdx !== -1) selectedIndices = [State.selectedHandIdx];
        }

        Rendering.renderCards('my-hand', p0.hand, true, false, selectedIndices, validTargets.myHand, validTargets.hasSelection);
        Rendering.renderCards('opp-hand', p1.hand, false, true, [], validTargets.oppHand, validTargets.hasSelection);
        Rendering.renderLiveZone('my-live', p0.live_zone, p0.live_zone_revealed, validTargets.myLive, validTargets.hasSelection);
        Rendering.renderPerformanceGuide();
        Rendering.renderLookedCards();
        Rendering.renderSelectionModal();
        Rendering.renderRuleLog();
        Rendering.renderActiveEffects(state, p0, p1, t);
        Rendering.renderActiveAbilities('active-abilities-list', state.triggered_abilities || []);
        // Toggle the panel visibility based on content
        const abPanel = document.getElementById('active-abilities-panel');
        if (abPanel) abPanel.style.display = (state.triggered_abilities && state.triggered_abilities.length > 0) ? 'block' : 'none';
        if (state.game_over) {
            Rendering.renderGameOver(state);
        } else {
            Rendering.renderActions();
        }

        Tooltips.highlightPendingSource();
        Rendering.updateSettingsButtons();
    },

    getPhaseKey: (phase) => {
        const perspectivePlayer = State.perspectivePlayer;
        if (phase === Phase.RPS) return 'rps';
        if (phase === Phase.SETUP) return 'setup';
        if (phase === Phase.MULLIGAN_P1) return (perspectivePlayer === 0) ? 'mulligan_you' : 'mulligan_opp';
        if (phase === Phase.MULLIGAN_P2) return (perspectivePlayer === 1) ? 'mulligan_you' : 'mulligan_opp';
        if (phase === Phase.ACTIVE) return 'active';
        if (phase === Phase.ENERGY) return 'energy';
        if (phase === Phase.DRAW) return 'draw';
        if (phase === Phase.MAIN) return 'main';
        if (phase === Phase.LIVE_SET) return 'live_set';
        if (phase === Phase.PERFORMANCE_P1) return (perspectivePlayer === 0) ? 'perf_p1' : 'perf_p2';
        if (phase === Phase.PERFORMANCE_P2) return (perspectivePlayer === 1) ? 'perf_p1' : 'perf_p2';
        if (phase === Phase.LIVE_RESULT) return 'live_result';
        return 'wait';
    },

    renderBoard: (state, p0, p1, validTargets = { stage: {}, discard: {}, hasSelection: false }) => {
        Rendering.renderStage('my-stage', p0.stage, true, validTargets.myStage, validTargets.hasSelection);
        Rendering.renderStage('opp-stage', p1.stage, true, validTargets.oppStage, validTargets.hasSelection);
        Rendering.renderLiveZone('my-live', p0.live_zone, true, validTargets.myLive, validTargets.hasSelection);
        Rendering.renderLiveZone('opp-live', p1.live_zone, true, validTargets.oppLive, validTargets.hasSelection);
        Rendering.renderEnergy('my-energy', p0.energy, true, validTargets.myEnergy, validTargets.hasSelection);
        Rendering.renderEnergy('opp-energy', p1.energy, true, validTargets.oppEnergy, validTargets.hasSelection);

        // Discard Pile (Visual Mini-Card)
        Rendering.renderDiscardPile('my-discard-visual', p0.discard, 0, validTargets.discard, validTargets.hasSelection);
        Rendering.renderDiscardPile('opp-discard-visual', p1.discard, 1);

        // Success Piles (Mini, Portrait)
        Rendering.renderCards('my-success', p0.success_pile || p0.success_lives, true, true);
        Rendering.renderCards('opp-success', p1.success_pile || p1.success_lives, false, true);

        Rendering.renderDeckCounts(p0, p1);
    },

    renderDeckCounts: (p0, p1) => {
        const updateCount = (id, count) => {
            const el = document.getElementById(id);
            if (el) el.textContent = count !== undefined ? count : 0;
        };

        // Main Decks
        updateCount('my-deck-count', p0.deck_count);
        updateCount('opp-deck-count', p1.deck_count);

        // Energy Decks
        updateCount('my-energy-deck-count', p0.energy_deck_count);
        updateCount('opp-energy-deck-count', p1.energy_deck_count);

        // Discard Counts
        updateCount('my-discard-count', p0.discard ? p0.discard.length : 0);
        updateCount('opp-discard-count', p1.discard ? p1.discard.length : 0);
    },

    renderCards: (containerId, cards, clickable = false, mini = false, selectedIndices = [], validActionMap = {}, hasGlobalSelection = false) => {
        const el = document.getElementById(containerId);
        if (!el) return;
        el.innerHTML = '';
        if (!cards) return;

        const state = State.data;
        cards.forEach((card, idx) => {
            const div = document.createElement('div');

            // Handle placeholders (null cards from engine tombstones)
            if (card === null) {
                div.className = 'card placeholder' + (mini ? ' card-mini' : '');
                div.style.visibility = 'hidden';
                div.style.pointerEvents = 'none';
                el.appendChild(div);
                return;
            }

            const isSelected = selectedIndices.includes(idx);

            // Validation Logic
            const actionId = validActionMap[idx];
            const isValid = actionId !== undefined;

            let highlightClass = '';
            if (isSelected) {
                highlightClass = (state.phase === Phase.MULLIGAN_P1 || state.phase === Phase.MULLIGAN_P2) ? ' mulligan-selected' : ' selected';
            }
            if (card.is_new) highlightClass += ' new-card';
            if (isValid) highlightClass += ' valid-target';

            const isHidden = card.hidden || card.id === -2;
            const isLive = card.type === 'live';

            // Apply rotation logic based on zone context and card type
            let rotationClass = '';
            if (isLive) {
                // Live Cards (Naturally Landscape)
                if (containerId.includes('hand')) {
                    // Stay horizontal in hand, or we could rotate to portrait.
                    // User requested Hand cards stop overlapping, and "already in correct orientation" for live zone.
                    // For hand, we keep them native (landscape) but sized to fit.
                }
            } else {
                // Member Cards (Naturally Portrait)
                if (containerId.includes('live') || containerId.includes('success')) {
                    // Member entering Landscape zone rotates 90deg to Lay Down
                    rotationClass = ' rotated-90';
                }
            }

            div.className = 'card' + (isHidden ? ' hidden' : '') +
                (isLive ? ' type-live' : '') +
                (mini ? ' card-mini' : '') + rotationClass + highlightClass;

            div.id = `${containerId}-card-${idx}`;
            div.setAttribute('data-card-id', card.id);

            if (!isHidden) {
                let imgPath = card.img || card.img_path || '';
                const imgHtml = imgPath ? `<img src="${fixImg(imgPath)}" draggable="false" onerror="this.style.display='none'">` : '';

                div.innerHTML = `${imgHtml}${card.cost !== undefined ? `<span class="cost">${card.cost}</span>` : ''}<div class="name">${card.name || ''}</div>`;
            } else {
                div.classList.add('card-back');
                div.innerHTML = ''; // Ensure no "undefined" text
            }

            if (clickable) {
                if (isValid || !hasGlobalSelection) {
                    div.style.cursor = 'pointer';
                    div.onclick = () => {
                        if (isValid && window.doAction) {
                            window.doAction(actionId);
                        } else if (window.playCard) {
                            window.playCard(idx);
                        }
                    };
                } else {
                    div.onclick = null;
                }
            }
            el.appendChild(div);

            if (!card.hidden) {
                const rawText = Tooltips.getEffectiveRawText(card);
                if (rawText) div.setAttribute('data-text', rawText);
                if (card.id !== undefined) div.setAttribute('data-card-id', card.id);
            }
        });
    },

    renderStage: (containerId, stage, clickable, validActionMap = {}, hasGlobalSelection = false) => {
        const el = document.getElementById(containerId);
        if (!el) return;
        el.innerHTML = '';
        for (let i = 0; i < 3; i++) {
            const slot = stage[i];
            const area = document.createElement('div');
            area.className = 'member-area board-slot-container';

            // Validation
            const actionId = validActionMap[i];
            const isValid = actionId !== undefined;

            let highlightClass = '';
            if (isValid) highlightClass += ' valid-target';

            const slotDiv = document.createElement('div');
            slotDiv.className = 'member-slot' + (slot && slot !== -1 ? ' filled' : '') + highlightClass;
            slotDiv.id = `${containerId}-slot-${i}`;

            if (slot && typeof slot === 'object' && slot.id !== undefined && slot.id !== -1) {
                let imgPath = slot.img || slot.img_path || '';
                let modifiersHtml = '';
                if (slot.modifiers && slot.modifiers.length > 0) {
                    modifiersHtml = `<div class="member-modifiers">${slot.modifiers.map(m => `<div class="modifier-tag ${m.type}">${m.label || (m.type === 'heart' ? '+' : m.value)}</div>`).join('')}</div>`;
                }

                slotDiv.innerHTML = imgPath ? `<img src="${fixImg(imgPath)}">${modifiersHtml}` : modifiersHtml;

                const rawText = Tooltips.getEffectiveRawText(slot);
                if (rawText) slotDiv.setAttribute('data-text', rawText);
                slotDiv.setAttribute('data-card-id', slot.id);
            }

            area.appendChild(slotDiv);
            el.appendChild(area);

            if (clickable) {
                if (isValid || !hasGlobalSelection) {
                    area.onclick = () => {
                        if (isValid && window.doAction) {
                            window.doAction(actionId);
                        } else if (window.onStageSlotClick) {
                            window.onStageSlotClick(i);
                        }
                    };
                    // Also clickable on slotDiv just in case
                    slotDiv.onclick = area.onclick;
                    area.style.cursor = 'pointer';
                } else {
                    area.onclick = null;
                }
            }
        }
    },

    renderEnergy: (containerId, energy, clickable = false, validActionMap = {}, hasGlobalSelection = false) => {
        const el = document.getElementById(containerId);
        if (!el) return;
        el.innerHTML = '';
        if (!energy) return;

        energy.forEach((e, i) => {
            const div = document.createElement('div');

            // Validation
            const actionId = validActionMap[i];
            const isValid = actionId !== undefined;
            let highlightClass = isValid ? ' valid-target' : '';

            div.className = 'energy-pip' + (e.tapped ? ' tapped' : '') + highlightClass;
            div.id = `${containerId}-slot-${i}`;

            let imgPath = e.img || e.img_path || 'img/texticon/icon_energy.png';
            div.innerHTML = `
                <img src="${fixImg(imgPath)}" onerror="this.src='img/texticon/icon_energy.png'">
                <div class="energy-num">${i + 1}</div>
            `;

            if (e.card) {
                const rawText = Tooltips.getEffectiveRawText(e.card);
                if (rawText) div.setAttribute('data-text', rawText);
                div.setAttribute('data-card-id', e.card.id);
            }

            if (clickable && isValid) {
                div.style.cursor = 'pointer';
                div.onclick = () => { if (window.doAction) window.doAction(actionId); };
            }

            el.appendChild(div);
        });
    },

    renderLiveZone: (containerId, liveCards, visible, validActionMap = {}, hasGlobalSelection = false) => {
        const state = State.data;
        const el = document.getElementById(containerId);
        if (!el) return;
        el.innerHTML = '';
        for (let i = 0; i < 3; i++) {
            const card = liveCards[i];

            // Validation
            const actionId = validActionMap[i];
            const isValid = actionId !== undefined;
            let highlightClass = '';
            if (isValid) highlightClass += ' valid-target';

            const slot = document.createElement('div');
            const isLiveCard = card && card.type === 'live';

            slot.className = 'card card-mini' + (card ? (isLiveCard ? ' type-live' : '') : ' empty') + highlightClass;
            slot.id = `${containerId}-slot-${i}`;
            if (card && typeof card === 'object' && card.id !== undefined && card.id !== -1) {
                const isPerfLegal = card.is_perf_legal;
                const imgPath = card.img || card.img_path || '';
                slot.innerHTML = `
                    <div class="live-card-inner ${isPerfLegal ? 'perf-legal' : ''}">
                        ${imgPath ? `<img src="${fixImg(imgPath)}">` : ''}
                        <div class="cost">${card.score || (card.cost !== undefined ? card.cost : 0)}</div>
                        ${isPerfLegal ? '<div class="perf-badge">LIVE!</div>' : ''}
                    </div>
                `;
                const rawText = Tooltips.getEffectiveRawText(card);
                if (rawText) slot.setAttribute('data-text', rawText);
                slot.setAttribute('data-card-id', card.id);

                if (isValid) {
                    slot.style.cursor = 'pointer';
                    slot.onclick = () => { if (window.doAction) window.doAction(actionId); };
                } else if (isPerfLegal) {
                    // Fallback highlight for performance selection if validActionMap didn't catch it
                    // Search for 600+ or 900+ IDs in the top-level legal_actions for this card
                    const fallbackId = state.legal_actions.find(a => (a.id === 600 + i || a.id === 900 + i || (a.metadata && a.metadata.slot_idx === i && a.metadata.category === 'LIVE')))?.id;
                    if (fallbackId !== undefined) {
                        slot.style.cursor = 'pointer';
                        slot.onclick = () => { if (window.doAction) window.doAction(fallbackId); };
                    } else {
                        slot.onclick = null;
                    }
                } else {
                    slot.onclick = null;
                }
            } else {
                slot.innerHTML = ``;
            }
            el.appendChild(slot);
        }
    },

    renderDiscardPile: (containerId, discard, playerIdx, validActionMap = {}, hasGlobalSelection = false) => {
        const el = document.getElementById(containerId);
        if (!el) return;

        // Validation for the PILE itself
        const actionId = validActionMap && validActionMap['all'];
        const isValid = actionId !== undefined;
        let highlightClass = isValid ? ' valid-target' : '';

        el.innerHTML = '';
        el.className = 'discard-pile-visual ' + highlightClass;

        if (!discard || discard.length === 0) {
            el.classList.add('empty');
            el.innerHTML = '<span style="opacity:0.3; font-size:0.8rem;">Discard</span>';
        } else {
            // Show up to 3 cards in a stack (Visual representation)
            const showCount = Math.min(3, discard.length);
            for (let i = 0; i < showCount; i++) {
                const card = discard[discard.length - 1 - i];
                const div = document.createElement('div');
                div.className = 'card card-mini';
                div.innerHTML = `<img src="${fixImg(card.img || '')}">`;
                div.style.transform = `translate(${i * 2}px, ${i * 2}px)`;
                div.style.zIndex = 10 - i;
                el.appendChild(div);
            }
        }

        if (isValid) {
            el.style.cursor = 'pointer';
            el.onclick = (e) => {
                e.stopPropagation();
                if (window.doAction) window.doAction(actionId);
            };
        } else if (!hasGlobalSelection && discard && discard.length > 0) {
            el.style.cursor = 'pointer';
            el.onclick = () => Rendering.showDiscardModal(playerIdx);
        } else {
            el.onclick = null;
        }
    },

    renderActiveEffects: (state, p0, p1, t) => {
        const container = document.getElementById('active-effects-list');
        if (!container) return;

        let html = '';

        const renderPlayerEffects = (p, pIdx) => {
            if (!p) return '';
            let effects = [];
            const isMe = pIdx === State.perspectivePlayer;
            const badgeClass = isMe ? 'badge-p1' : 'badge-p2';
            const badgeLabel = isMe ? (t['you'] || 'You') : (t['opponent'] || 'Opponent');

            // Cost Reduction
            if (p.cost_reduction && p.cost_reduction !== 0) {
                effects.push({
                    title: t['cost_reduction'] || 'Cost Reduction',
                    desc: `${t['cost'] || 'Cost'} ${p.cost_reduction > 0 ? '-' : '+'}${Math.abs(p.cost_reduction)}`,
                    duration: t['until_end_of_turn'] || 'Until End of Turn',
                    type: 'buff'
                });
            }

            // Blade Buffs
            if (p.blade_buffs) {
                p.blade_buffs.forEach((val, idx) => {
                    if (val !== 0) {
                        effects.push({
                            title: `${t['slot'] || 'Slot'} ${idx + 1}: ${t['blade_buff'] || 'Blade Buff'}`,
                            desc: `Appeal ${val > 0 ? '+' : ''}${val}`,
                            duration: t['until_end_of_turn'] || 'Until End of Turn',
                            type: val > 0 ? 'buff-blade' : 'debuff'
                        });
                    }
                });
            }

            // Heart Buffs
            if (p.heart_buffs) {
                p.heart_buffs.forEach((hb, idx) => {
                    const colors = ['Smile', 'Pure', 'Cool', 'Green', 'Blue', 'Purple', 'Wildcard'];
                    let heartDesc = [];
                    if (hb && Array.isArray(hb)) {
                        hb.forEach((count, cIdx) => {
                            if (count > 0) {
                                heartDesc.push(`${colors[cIdx] || cIdx} +${count}`);
                            }
                        });
                    }
                    if (heartDesc.length > 0) {
                        effects.push({
                            title: `${t['slot'] || 'Slot'} ${idx + 1}: ${t['heart_buff'] || 'Heart Buff'}`,
                            desc: heartDesc.join(', '),
                            duration: t['until_end_of_turn'] || 'Until End of Turn',
                            type: 'buff-heart'
                        });
                    }
                });
            }

            // Game Restrictions
            if (p.prevent_baton_touch > 0) {
                effects.push({
                    title: t['restriction'] || 'Restriction',
                    desc: t['cannot_baton_touch'] || 'Cannot Baton Touch',
                    duration: t['until_end_of_turn'] || 'Until End of Turn',
                    type: 'restriction'
                });
            }
            if (p.prevent_activate > 0) {
                effects.push({
                    title: t['restriction'] || 'Restriction',
                    desc: t['cannot_activate_member'] || 'Cannot Activate Member Abilities',
                    duration: t['until_end_of_turn'] || 'Until End of Turn',
                    type: 'restriction'
                });
            }

            // Yell Cards removed from Active Effects UI per user feedback.
            // They are tracked via the Rule Log / Turn History now.

            if (effects.length === 0) return '';

            let pStats = `<div class="effect-player-badge ${badgeClass}">${badgeLabel}</div>`;
            return pStats + effects.map(e => `
                <div class="effect-item ${e.type || ''}">
                    <div class="effect-title-row">
                        <span class="effect-title">${e.title}</span>
                        <span class="effect-duration">${e.duration}</span>
                    </div>
                    <div class="effect-desc">${e.desc}</div>
                </div>
            `).join('');
        };

        html += renderPlayerEffects(p0, State.perspectivePlayer);
        html += renderPlayerEffects(p1, 1 - State.perspectivePlayer);

        if (!html) {
            container.innerHTML = `<div style="font-size: 0.75rem; color: var(--text-dim); text-align: center; padding: 10px;">${t['no_active_effects'] || 'No active effects'}</div>`;
        } else {
            container.innerHTML = html;
        }
    },

    renderRuleLog: (containerId = 'rule-log') => {
        const ruleLogEl = document.getElementById(containerId);
        if (!ruleLogEl) return;

        const state = State.data;
        const currentLang = State.currentLang;
        const showFriendlyAbilities = State.showFriendlyAbilities;
        const selectedTurn = State.selectedTurn || -1;
        const showingFullLog = State.showingFullLog;

        let logData = state.rule_log || [];

        // Apply filtering
        if (selectedTurn !== -1) {
            const turnStr = `[Turn ${selectedTurn}]`;
            logData = logData.filter(entry => entry.includes(turnStr));
        }

        ruleLogEl.innerHTML = '';

        // Log Consolidation: Deduplicate consecutive redundant logs
        let filteredLogData = [];
        for (let i = 0; i < logData.length; i++) {
            const entry = logData[i];
            const nextEntry = logData[i + 1];

            const normEntry = entry.replace(/^\[Turn \d+\]\s*/, '');
            const normNext = nextEntry ? nextEntry.replace(/^\[Turn \d+\]\s*/, '') : null;

            if (normEntry.includes("sets live card") && normNext && (normNext.includes("sets live card") || normNext.includes("Live Set End"))) {
                continue;
            }
            if (normEntry === normNext) continue;

            filteredLogData.push(entry);
        }

        const fragment = document.createDocumentFragment();
        filteredLogData.forEach(entry => {
            const div = document.createElement('div');
            div.className = 'log-entry';

            let displayText = entry;
            let isAbility = false;

            const abilityMatch = entry.match(/(?:\[Turn \d+\] )?\[TRIGGER:(\d+)\](.*?): (.*)/);
            const rustAbilityMatch = entry.match(/(?:\[Turn \d+\] )?(\[Rule .*?\]|\[Activated\]|\[Turn Start\]|\[Turn End\]|\[Triggered\])(.*?): (.*)/);

            if (abilityMatch || rustAbilityMatch) {
                isAbility = true;
                const match = abilityMatch || rustAbilityMatch;
                let triggerLabel = "";
                let cardName = "";
                let pseudocode = "";

                if (abilityMatch) {
                    const triggerId = parseInt(match[1]);
                    cardName = match[2].trim();
                    pseudocode = match[3].trim();
                    triggerLabel = `[${triggerId}]`;
                    if (translations[currentLang] && translations[currentLang].triggers && translations[currentLang].triggers[triggerId]) {
                        triggerLabel = translations[currentLang].triggers[triggerId];
                    }
                } else {
                    triggerLabel = match[1].trim();
                    cardName = match[2].trim();
                    pseudocode = match[3].trim();
                }

                let translatedEffect = pseudocode;
                const shouldTranslate = (currentLang === 'en' || showFriendlyAbilities);

                if (shouldTranslate && window.translateAbility) {
                    translatedEffect = window.translateAbility("EFFECT: " + pseudocode, currentLang);
                    translatedEffect = translatedEffect.replace(/^.*?: /, '').replace(/^→ /, '');
                } else if (currentLang === 'jp' && !showFriendlyAbilities) {
                    const srcCard = State.resolveCardDataByName(cardName);
                    if (srcCard && srcCard.original_text) {
                        translatedEffect = srcCard.original_text;
                    }
                }

                let displayCardName = cardName;
                if (currentLang === 'en' && window.NAME_MAP && window.NAME_MAP[cardName]) {
                    displayCardName = window.NAME_MAP[cardName];
                }

                const turnMatch = entry.match(/^\[Turn \d+\]/);
                const turnPrefix = turnMatch ? turnMatch[0] + " " : "";
                displayText = `${turnPrefix}${triggerLabel} ${displayCardName}: ${translatedEffect}`;
            }

            const mulliganMatch = entry.match(/(?:\[Turn \d+\] )?(Mulligan): (.*)/i);
            if (mulliganMatch) {
                const rawPhase = mulliganMatch[1];
                const cardName = mulliganMatch[2].trim();
                let displayPhase = currentLang === 'jp' ? "マリガン" : "Mulligan";
                let displayCardName = cardName;
                if (currentLang === 'en' && window.NAME_MAP && window.NAME_MAP[cardName]) {
                    displayCardName = window.NAME_MAP[cardName];
                }
                const turnMatch = entry.match(/^\[Turn \d+\]/);
                const turnPrefix = turnMatch ? turnMatch[0] + " " : "";
                displayText = `${turnPrefix}${displayPhase}: ${displayCardName}`;
            }

            const entryUpper = entry.toUpperCase();
            if (entryUpper.includes("---") && entryUpper.includes("PHASE") || entryUpper.includes("[ACTIVE PHASE]")) div.classList.add('phase');
            else if (isAbility) div.classList.add('ability');
            else if (entryUpper.includes('PLAYS') || entryUpper.includes('MULLIGAN') || entryUpper.includes('SELECTED')) div.classList.add('action');
            else if (entryUpper.includes('EFFECT:') || entryUpper.includes('RULE')) div.classList.add('effect');
            else if (entryUpper.includes('SCORE') || entryUpper.includes('SUCCESS LIVE')) div.classList.add('score');
            else if (entry.includes('===')) div.classList.add('turn');

            div.innerHTML = Tooltips.enrichAbilityText(displayText);
            fragment.appendChild(div);
        });

        ruleLogEl.appendChild(fragment);
        if (!showingFullLog) ruleLogEl.scrollTop = ruleLogEl.scrollHeight;
    },

    renderActiveAbilities: (containerId, abilities) => {
        const el = document.getElementById(containerId);
        if (!el || !abilities) return;
        el.innerHTML = abilities.map(a => `
            <div class="active-ability-tag" data-text="${a.text || a.description || ''}">
                ${Tooltips.enrichAbilityText(a.name || 'Ability')}
            </div>
        `).join('');
    },

    renderSelectionModal: () => {
        // Disabled as per user request - all selections are now in the sidebar
        const modal = document.getElementById('selection-modal');
        if (modal) modal.style.display = 'none';
        return;
    },

    renderGameOver: (state) => {
        const actionsDiv = document.getElementById('actions');
        if (actionsDiv) {
            const winnerName = state.winner === State.perspectivePlayer ? "YOU" : `Player ${state.winner + 1}`;
            actionsDiv.innerHTML = `
                <div class="game-over-banner">
                    <h2>GAME OVER</h2>
                    <div class="winner-announcement">Winner: ${winnerName}</div>
                    <button class="btn btn-primary" onclick="location.reload()">New Game</button>
                </div>
            `;
        }
    },

    showDiscardModal: (playerIdx) => {
        console.log("[Rendering] showDiscardModal triggered for player:", playerIdx);
        const state = State.data;
        if (!state || !state.players) {
            console.error("[Rendering] showDiscardModal: No state data found!");
            return;
        }
        const player = state.players[playerIdx];
        console.log("[Rendering] player discard length:", player.discard ? player.discard.length : 0);
        const discard = player.discard || [];

        const modal = document.getElementById('discard-modal');
        const title = document.getElementById('discard-modal-title');
        const container = document.getElementById('discard-modal-cards');

        if (!modal || !container) return;

        title.textContent = (playerIdx === State.perspectivePlayer ? "Your" : "Opponent's") + " Discard Pile (" + discard.length + ")";
        container.innerHTML = '';

        if (discard.length === 0) {
            container.innerHTML = '<div style="grid-column: 1/-1; text-align: center; opacity: 0.5; padding: 40px;">No cards in discard pile.</div>';
        } else {
            discard.forEach((card, idx) => {
                if (!card) return; // Added null check
                const div = document.createElement('div');
                div.className = 'card'; // Original class name
                let imgPath = card.img || card.img_path || '';
                div.innerHTML = `<img src="${fixImg(imgPath)}">`;
                const rawText = Tooltips.getEffectiveRawText(card);
                if (rawText) div.setAttribute('data-text', rawText);
                container.appendChild(div);
            });
        }

        modal.style.display = 'flex';
    },

    renderActions: () => {
        const state = State.data;
        const actionsDiv = document.getElementById('actions');
        if (!state || !actionsDiv || state.game_over) return;

        const currentLang = State.currentLang;
        const t = translations[currentLang];
        const perspectivePlayer = State.perspectivePlayer;
        const mobileActionBar = document.getElementById('mobile-action-bar');

        if (mobileActionBar) mobileActionBar.innerHTML = '';
        actionsDiv.innerHTML = '';

        // Helper for consistent action labels
        const getActionLabel = (a, isMini = false) => {
            const energyIcon = `<img src="img/texticon/icon_energy.png" style="height:14px; vertical-align:middle; margin:0 2px;">`;
            const heartIcon = `<img src="img/texticon/icon_heart.png" style="height:14px; vertical-align:middle; margin:0 2px;">`;

            let cost = a.metadata?.cost ?? a.cost ?? a.base_cost ?? null;
            const isBaton = (a.name && (a.name.includes('Baton') || a.name.includes('バトン')));
            let name = a.metadata?.name ?? a.name ?? "";

            // User Request: If name is "Action 30X", try to resolve the card name
            if (name.match(/^Action\s+30\d$/)) {
                const liveIdx = parseInt(name.replace("Action 30", ""), 10);
                const state = State.data;
                const liveCard = state?.live_zone && state.live_zone[liveIdx];
                if (liveCard && liveCard.name) {
                    name = liveCard.name;
                    // Add heart icon for live cards
                    return `<div class="action-title">${heartIcon} ${name}</div>`;
                }
            }

            // Clean name: remove verbose bracketed prefixes
            name = name.replace(/[【\[].*?[】\]]/g, "").trim();

            if (isMini) {
                // For PLAY actions, show just the cost number
                if (a.type === 'PLAY') return `<span>${cost !== null ? cost : 0}</span>${isBaton ? ' [B]' : ''}`;
                // For MULLIGAN, show the card name (truncated if needed)
                if (a.type === 'MULLIGAN') {
                    const shortName = name.length > 10 ? name.substring(0, 10) + '…' : name;
                    return `<span style="font-size:0.65rem">${shortName || '?'}</span>`;
                }
                // Default mini: energy + cost
                let label = `${energyIcon}${cost !== null ? cost : 0}`;
                if (isBaton) label += ' [B]';
                return Tooltips.enrichAbilityText(label);
            } else {
                let displayName = name;
                if (a.metadata?.secondary_slot_idx !== undefined && a.metadata?.areas_desc) {
                    displayName = (currentLang === 'jp')
                        ? a.metadata.areas_desc.replace(' & ', '＆')
                        : a.metadata.areas_desc;
                }

                // Enrich the name with icons/highlights
                displayName = Tooltips.enrichAbilityText(displayName);

                let label = `<div class="action-title" style="${(displayName.includes('&') || displayName.includes('＆')) ? 'font-size:0.85em;' : ''}">${displayName}</div>`;
                if (cost !== null) label += `<div class="action-cost">${energyIcon}${cost}</div>`;
                if (isBaton && a.metadata?.secondary_slot_idx === undefined) label += ' [B]';
                return label;
            }
        };

        // Unified Button Creator to reduce repeated code
        const createActionButton = (a, isMini = false, extraClass = '') => {
            const btn = document.createElement('button');
            btn.className = `action-btn ${isMini ? 'mini' : ''} ${extraClass}`.trim();
            if (a.id !== undefined) btn.setAttribute('data-action-id', a.id);
            if (a.raw_text || a.text) btn.dataset.text = a.raw_text || a.text;
            btn.innerHTML = getActionLabel(a, isMini);
            btn.onclick = () => { if (window.doAction && a.id !== undefined) window.doAction(a.id); };
            return btn;
        };


        // RPS Phase Handler
        if (state.phase === Phase.RPS) {
            const rpsDiv = document.createElement('div');
            rpsDiv.className = 'rps-selector';
            rpsDiv.style.textAlign = 'center';
            rpsDiv.style.padding = '15px';
            rpsDiv.style.background = 'rgba(255, 255, 255, 0.05)';
            rpsDiv.style.borderRadius = '12px';
            rpsDiv.style.marginBottom = '20px';

            const title = currentLang === 'en' ? 'Choose Your Sign' : '手を選んでください';
            rpsDiv.innerHTML = `<h3 style="margin-top:0; color:var(--accent-gold);">${title}</h3>`;

            const btnContainer = document.createElement('div');
            btnContainer.style.display = 'flex';
            btnContainer.style.flexDirection = 'column';
            btnContainer.style.alignItems = 'center';
            btnContainer.style.gap = '10px';

            const baseId = (perspectivePlayer === 1) ? 11000 : 10000;
            const signs = [
                { id: baseId + 0, name: 'Rock', jp: 'グー' },
                { id: baseId + 1, name: 'Paper', jp: 'パー' },
                { id: baseId + 2, name: 'Scissors', jp: 'チョキ' }
            ];

            signs.forEach(sign => {
                const hasAction = state.legal_actions && state.legal_actions.some(a => a.id === sign.id);
                const a = { id: sign.id, name: currentLang === 'en' ? sign.name : sign.jp };
                const btn = createActionButton(a, false, 'rps-btn');
                btn.style.width = '120px';
                btn.style.opacity = hasAction ? '1' : '0.4';
                btn.style.pointerEvents = hasAction ? 'auto' : 'none';
                btnContainer.appendChild(btn);
            });

            rpsDiv.appendChild(btnContainer);
            actionsDiv.appendChild(rpsDiv);
            return;
        }

        // Choice Indicator & Inline Selection
        if (state.pending_choice) {
            const choice = state.pending_choice;
            const choiceDiv = document.createElement('div');
            choiceDiv.className = 'pending-choice-indicator';

            const opcode = choice.opcode || (state.legal_actions && state.legal_actions[0] && state.legal_actions[0].metadata && state.legal_actions[0].metadata.opcode);
            let headerColor = 'var(--accent-gold)';
            if (opcode === 58) headerColor = '#ff4d4d';
            else if (opcode === 15 || opcode === 17 || opcode === 63) headerColor = '#4da6ff';
            else if (opcode === 45) headerColor = '#ffcc00';
            else if (opcode === 41 || opcode === 74) headerColor = '#9966ff';

            choiceDiv.style.marginBottom = '15px';
            choiceDiv.style.padding = '12px';
            choiceDiv.style.background = 'rgba(255,255,255,0.05)';
            choiceDiv.style.borderRadius = '10px';
            choiceDiv.style.borderLeft = `4px solid ${headerColor}`;

            const title = choice.title || choice.description || (currentLang === 'jp' ? '選択してください' : 'Please Select');
            let content = `<div style="font-weight:bold; color:${headerColor}; margin-bottom: 10px; font-size: 1.1rem;">${title}</div>`;
            if (choice.text && choice.text !== title) {
                content += `<div style="font-size: 0.9rem; margin-bottom: 10px; opacity: 0.8; line-height: 1.4;">${choice.text}</div>`;
            }
            choiceDiv.innerHTML = content;

            if (choice.options && choice.options.length > 0) {
                const optContainer = document.createElement('div');
                optContainer.className = 'action-list';
                optContainer.style.maxHeight = '300px';
                optContainer.style.overflowY = 'auto';

                choice.options.forEach((opt, idx) => {
                    const a = {
                        id: choice.actions[idx],
                        name: opt.name || opt.text || `Option ${idx + 1}`,
                        text: opt.text
                    };
                    const btn = createActionButton(a, false, 'confirm');
                    btn.style.width = '100%';
                    optContainer.appendChild(btn);
                });
                choiceDiv.appendChild(optContainer);
            }
            actionsDiv.appendChild(choiceDiv);
            return;
        }

        if (state.is_ai_thinking) {
            const aiDiv = document.createElement('div');
            aiDiv.className = 'ai-thinking-indicator';
            aiDiv.innerHTML = `<div style="font-weight:bold; color:#0096ff; padding:10px; border-left:4px solid #0096ff; background:rgba(0,150,255,0.1); border-radius:8px;">${state.ai_status || 'AI is thinking...'}</div>`;
            actionsDiv.appendChild(aiDiv);
        }

        if (!state.legal_actions || state.legal_actions.length === 0) {
            actionsDiv.innerHTML = `<div class="no-actions">${t['wait'] || 'Waiting...'}</div>`;
            return;
        }

        const listDiv = document.createElement('div');
        listDiv.className = 'action-list';
        actionsDiv.appendChild(listDiv);

        // Grouping Actions — ONLY positional for MULLIGAN and PLAY
        const playActionsByHand = {};
        const mulliganActions = {};
        const abilityActions = [];
        const systemActions = [];
        const otherActions = [];

        state.legal_actions.forEach(a => {
            const category = a.metadata?.category || a.type;
            const hIdx = a.metadata?.hand_idx ?? a.hand_idx;

            if (a.id === 0 || a.type === 'SYSTEM' || a.id < 10 || a.name?.includes('End') || a.name?.includes('終了')) {
                systemActions.push(a);
            } else if (category === 'PLAY' && hIdx !== undefined) {
                if (!playActionsByHand[hIdx]) playActionsByHand[hIdx] = [];
                playActionsByHand[hIdx].push(a);
            } else if (a.type === 'MULLIGAN' && hIdx !== undefined) {
                if (!mulliganActions[hIdx]) mulliganActions[hIdx] = [];
                mulliganActions[hIdx].push(a);
            } else if (category === 'ABILITY') {
                abilityActions.push(a);
            } else {
                otherActions.push(a);
            }
        });

        const addHeader = (text, color) => {
            const header = document.createElement('div');
            header.className = 'category-header';
            header.style.color = color || 'rgba(255,255,255,0.4)';
            header.innerText = text;
            listDiv.appendChild(header);
        };

        // 1. SYSTEM (Pass, Confirm)
        if (systemActions.length > 0) {
            addHeader(currentLang === 'jp' ? 'システム' : 'SYSTEM');
            systemActions.forEach(a => listDiv.appendChild(createActionButton(a, false, a.id === 0 ? 'confirm system' : 'system')));
        }

        // 2. ABILITIES
        if (abilityActions.length > 0) {
            addHeader(currentLang === 'jp' ? 'アビリティ' : 'ABILITIES', '#9966ff');
            abilityActions.forEach(a => listDiv.appendChild(createActionButton(a)));
        }

        // 3. MULLIGAN (Hand Grid with stable positions)
        const perspectivePlayerHand = state.players[perspectivePlayer]?.hand || [];
        if (Object.keys(mulliganActions).length > 0) {
            addHeader(currentLang === 'jp' ? 'マリガン' : 'MULLIGAN', 'var(--accent-pink)');

            perspectivePlayerHand.forEach((_, idx) => {
                const actions = mulliganActions[idx] || [];
                if (actions.length > 0) {
                    actions.forEach(a => listDiv.appendChild(createActionButton(a)));
                } else {
                    // Invisible placeholder keeps layout stable
                    const spacer = document.createElement('button');
                    spacer.className = 'action-btn';
                    spacer.style.visibility = 'hidden';
                    listDiv.appendChild(spacer);
                }
            });
        }

        // 4. PLAY MEMBER (Grouped by hand card, 3-slot sub-grid)
        if (Object.keys(playActionsByHand).length > 0) {
            addHeader(currentLang === 'jp' ? 'メンバー登場' : 'PLAY MEMBER', 'var(--accent-gold)');
            Object.keys(playActionsByHand).sort((a, b) => parseInt(a) - parseInt(b)).forEach(hIdx => {
                const actions = playActionsByHand[hIdx];
                const firstA = actions[0];
                const groupDiv = document.createElement('div');
                groupDiv.className = 'action-group-card';

                const header = document.createElement('div');
                header.className = 'action-group-header';
                const energyIcon = `<img src="img/texticon/icon_energy.png" style="height:14px; vertical-align:middle; margin-left: 5px;">`;
                const displayCost = firstA.metadata?.cost ?? firstA.cost ?? firstA.base_cost ?? 0;
                let cleanName = (firstA.metadata?.name ?? firstA.name ?? "").replace(/[【\[].*?[】\]]/g, "").replace(/\(.*?\)/g, "").trim();
                header.innerHTML = `<span>${cleanName}</span> <span class="header-base-cost">${energyIcon}${displayCost}</span>`;
                groupDiv.appendChild(header);

                const btnsDiv = document.createElement('div');
                btnsDiv.className = 'action-group-buttons grid-3';
                for (let slotIdx = 0; slotIdx < 3; slotIdx++) {
                    const a = actions.find(act => (act.slot_idx === slotIdx || act.metadata?.slot_idx === slotIdx) && act.metadata?.secondary_slot_idx === undefined);
                    if (a) {
                        btnsDiv.appendChild(createActionButton(a, true));
                    } else {
                        const spacer = document.createElement('div');
                        spacer.style.visibility = 'hidden';
                        spacer.style.minHeight = '36px';
                        btnsDiv.appendChild(spacer);
                    }
                }
                groupDiv.appendChild(btnsDiv);

                const doubleActions = actions.filter(act => act.metadata?.secondary_slot_idx !== undefined);
                if (doubleActions.length > 0) {
                    // Group double actions by unique pair (sorted indices)
                    const pairs = {};
                    doubleActions.forEach(a => {
                        const s1 = a.metadata.slot_idx;
                        const s2 = a.metadata.secondary_slot_idx;
                        const key = [s1, s2].sort().join('-');
                        if (!pairs[key]) pairs[key] = [];
                        pairs[key].push(a);
                    });

                    Object.values(pairs).forEach(pairActions => {
                        const doubleDiv = document.createElement('div');
                        doubleDiv.className = 'action-group-buttons double-baton-row grid-3';

                        // Show buttons in their actual target columns
                        const pairSlots = new Set();
                        pairActions.forEach(a => pairSlots.add(a.metadata.slot_idx));
                        pairActions.forEach(a => pairSlots.add(a.metadata.secondary_slot_idx));

                        for (let i = 0; i < 3; i++) {
                            const a = pairActions.find(act => act.metadata.slot_idx === i);
                            if (a) {
                                const btn = createActionButton(a, true, 'double-baton-btn');
                                btn.style.width = '100%';
                                doubleDiv.appendChild(btn);
                            } else if (pairSlots.has(i)) {
                                // Slot is part of the pair but this specific action doesn't target it
                                // (e.g. the other half of the pair's choice)
                                const spacer = document.createElement('div');
                                spacer.className = 'pair-spacer';
                                spacer.innerText = currentLang === 'jp' ? '間' : 'GAP';
                                doubleDiv.appendChild(spacer);
                            } else {
                                // Slot is not part of this pair at all
                                const spacer = document.createElement('div');
                                spacer.style.visibility = 'hidden';
                                doubleDiv.appendChild(spacer);
                            }
                        }
                        groupDiv.appendChild(doubleDiv);
                    });
                }

                listDiv.appendChild(groupDiv);
            });
        }

        // 5. OTHER ACTIONS (Choices, Selects, Responses — full-size buttons)
        if (otherActions.length > 0) {
            addHeader(currentLang === 'jp' ? 'アクション' : 'ACTIONS');
            otherActions.forEach(a => listDiv.appendChild(createActionButton(a)));
        }
    },

    renderPerformanceGuide: () => {
        const state = State.data;
        const perspectivePlayer = State.perspectivePlayer;
        const p0 = state.players[perspectivePlayer] || state.players[0];
        const guide = p0.performance_guide;
        const panel = document.getElementById('perf-guide-panel');
        const contentEl = document.getElementById('perf-guide-content');
        if (!panel || !contentEl) return;

        if (!guide || !guide.lives || guide.lives.length === 0) {
            panel.style.display = 'none';
            return;
        }
        panel.style.display = 'block';

        let html = `<div class="perf-guide-header">
            <span>Blades: <b>${guide.total_blades}</b></span>
            <span>Hearts: ${Rendering.renderHeartsCompact(guide.total_hearts)}</span>
        </div>`;

        guide.lives.forEach(l => {
            if (!l || typeof l !== 'object') return;
            const color = l.passed ? '#4f4' : '#f44';
            const imgPath = l.img || '';
            const imgHtml = imgPath ? `<img src="${fixImg(imgPath)}" class="perf-guide-img" style="border-color:${color}">` : '';

            let entryHtml = `<div class="perf-guide-entry" style="opacity: ${l.passed ? 1 : 0.7}" ${l.text ? `data-text="${l.text.replace(/"/g, '&quot;')}"` : ''}>
                ${imgHtml}
                <div class="perf-guide-info">
                    <div class="perf-guide-name">${l.name || 'Live'} <span class="perf-guide-score">(${l.score || 0}pts)</span></div>
                    <div class="perf-guide-pips">
                        ${Rendering.renderHeartProgress(l.filled, l.required)}
                    </div>
                    ${!l.passed ? `<div class="perf-guide-reason">${l.reason || ''}</div>` : ''}
                </div>
                <div class="perf-guide-status" style="color:${color}">${l.passed ? '✓' : 'x'}</div>
            </div>`;

            html += entryHtml;
        });

        contentEl.innerHTML = html;
    },

    renderLookedCards: () => {
        const state = State.data;
        const panel = document.getElementById('looked-cards-panel');
        const content = document.getElementById('looked-cards-content');
        if (!panel || !content) return;

        const cards = state.looked_cards || [];
        if (cards.length === 0) {
            panel.style.display = 'none';
            return;
        }
        panel.style.display = 'block';

        let html = "";
        if (state.pending_choice && (state.pending_choice.title || state.pending_choice.text)) {
            const title = state.pending_choice.title || state.pending_choice.text;
            html += `<div class="looked-cards-header" style="width:100%; color: var(--accent-gold); font-size: 0.8rem; padding: 5px; margin-bottom: 4px; border-bottom: 1px solid rgba(255,255,255,0.1); font-weight: bold;">${title}</div>`;
        }

        if (state.pending_choice && state.pending_choice.choose_count > 1) {
            const total = state.pending_choice.choose_count;
            const v_rem = state.pending_choice.v_remaining;
            const remaining = (v_rem === -1) ? total : (v_rem + 1);
            const t = translations ? translations[State.currentLang] : null;

            if (remaining > 1) {
                const label = t ? (t['pick_more'] || `Pick ${remaining} more cards`).replace('{count}', remaining) : `Pick ${remaining} more cards`;
                html += `<div style="padding: 0 5px 8px 5px; font-size: 0.75rem; color: var(--accent-pink); font-style: italic;">${label}</div>`;
            } else {
                const label = t ? (t['pick_last'] || 'Pick the last card') : 'Pick the last card';
                html += `<div style="padding: 0 5px 8px 5px; font-size: 0.75rem; color: var(--accent-green); font-style: italic;">${label}</div>`;
            }
        }

        html += cards.map((c, idx) => {
            if (c === null) {
                return `<div class="looked-card-item placeholder" style="visibility: hidden; pointer-events: none;"></div>`;
            }
            const tooltip = Tooltips.getEffectiveAbilityText(c);
            const aid = (state.pending_choice && state.pending_choice.actions && state.pending_choice.actions.length > idx)
                ? state.pending_choice.actions[idx]
                : undefined;

            // Only make clickable if action ID is valid
            const isClickable = (aid !== undefined && aid !== 0);
            const clickHandler = isClickable ? `if(window.doAction) window.doAction(${aid})` : '';
            const cursorStyle = isClickable ? 'cursor: pointer;' : '';

            return `
                <div class="looked-card-item" ${isClickable ? `onclick="${clickHandler}"` : ''} style="${cursorStyle}" title="${tooltip.replace(/"/g, '&quot;')}">
                    <img src="${fixImg(c.img)}" class="looked-card-img">
                    <div class="looked-card-name">${c.name}</div>
                </div>
            `;
        }).join('');
        content.innerHTML = html;
    },

    renderPerformanceResult: (results = null) => {
        const modal = document.getElementById('performance-modal');
        const content = document.getElementById('performance-result-content');
        if (!modal || !content) return;

        let displayResults = results ||
            (State.data.performance_results && Object.keys(State.data.performance_results).length > 0 ? State.data.performance_results : State.data.last_performance_results);

        const currentLang = State.currentLang;
        const t = translations ? translations[currentLang] : null;

        if (!displayResults || Object.keys(displayResults).length === 0) {
            const label = t ? (t['no_perf_data'] || 'No performance data available for this turn.') : 'No performance data available for this turn.';
            content.innerHTML = `<div style="text-align:center; padding: 20px; opacity:0.6;">${label}</div>`;
            return;
        }

        content.innerHTML = '';
        Rendering.renderTurnHistory(); // Render history in background tab
        Rendering.showPerfTab('result'); // Ensure we start on result tab

        let html = '';
        // Added turn history navigation
        if (State.performanceHistoryTurns && State.performanceHistoryTurns.length > 1) {
            html += `<div class="perf-turn-nav">`;
            const turns = [...State.performanceHistoryTurns].sort((a, b) => a - b);
            turns.forEach((turn) => {
                const turnNum = parseInt(turn);
                const isLatest = turnNum === turns[turns.length - 1];
                let turnLabel = isLatest ? `Current (T${turnNum})` : `Turn ${turnNum}`;
                if (t) {
                    turnLabel = isLatest ? (t['current_turn'] || 'Current (T{turn})').replace('{turn}', turnNum) : (t['turn_label'] || 'Turn {turn}').replace('{turn}', turnNum);
                }

                const isSelected = (State.selectedPerfTurn === turnNum) || (State.selectedPerfTurn === -1 && isLatest);
                const activeClass = isSelected ? 'active' : '';

                html += `<button class="perf-nav-btn ${activeClass}" onclick="window.showPerformanceForTurn(${turnNum})">
                            ${turnLabel}
                         </button>`;
            });
            html += `</div>`;
        }

        html += '<div class="perf-result-container">';
        [0, 1].forEach(pid => {
            const res = displayResults[pid];
            if (!res) return;

            const playerName = pid === State.perspectivePlayer ? (t ? (t['you'] || 'You') : 'You') : (t ? (t['opp'] || 'Opponent') : 'Opponent');
            const statusLabel = res.success ? (t ? (t['success'] || 'SUCCESS') : 'SUCCESS') : (t ? (t['failure'] || 'FAILURE') : 'FAILURE');
            const statusClass = res.success ? 'success' : 'failure';

            html += `
    <div class="perf-player-box ${statusClass}">
                    <div style="display:flex; justify-content:space-between; align-items:center; margin-bottom:10px;">
                        <h3 style="margin:0;">${playerName}: ${statusLabel}</h3>
                        <div style="text-align:right;">
                            <span style="font-size:0.75rem; opacity:0.6; text-transform:uppercase;">${t ? (t['judge_score'] || 'Judge Score') : 'Judge Score'}</span>
                            <div style="font-size:1.25rem; font-weight:bold; color:var(--accent-gold);">${res.total_score || 0}</div>
                        </div>
                    </div>
                    <div class="perf-breakdown">
                        <div class="perf-section">
                            <h4>${t ? (t['target_lives'] || 'Target Lives') : 'Target Lives'}</h4>
                            ${res.lives && res.lives.length > 0 ? res.lives.map(l => {
                if (!l) return ''; // Added null check
                const filledSum = (l.filled || [0, 0, 0, 0, 0, 0, 0]).reduce((a, b) => a + b, 0);
                const reqSum = (l.required || [0, 0, 0, 0, 0, 0, 0]).reduce((a, b) => a + b, 0);
                const spareSum = (l.spare || [0, 0, 0, 0, 0, 0, 0]).reduce((a, b) => a + b, 0);
                const extraHearts = Math.max(0, filledSum - reqSum);

                return `
                                <div class="perf-line" style="flex-direction: column; align-items: flex-start; gap: 4px; border-bottom: 1px solid rgba(255,255,255,0.05); padding-bottom: 8px; margin-bottom: 8px;">
                                    <div style="display:flex; justify-content: space-between; width: 100%; align-items:center;">
                                        <div style="display:flex; align-items:center; gap:5px;">
                                            ${l.img ? `<img src="${fixImg(l.img)}" style="width:24px; border-radius:3px;">` : ''}
                                            <div style="display:flex; flex-direction:column;">
                                                <span style="font-weight:bold; font-size:0.9rem;">${l.name || 'Live'}</span>
                                                <span style="font-size:0.7rem; color:var(--accent-gold); opacity:0.9;">${t ? (t['score'] || 'Score') : 'Score'}: <b>${l.score || 0}</b></span>
                                            </div>
                                        </div>
                                         <div style="display:flex; align-items:center; gap:10px;">
                                            ${extraHearts > 0 ? `<span style="font-size:0.75rem; color:var(--accent-gold);">${t ? (t['extra_hearts'] || '+{count} Extra').replace('{count}', extraHearts) : `+${extraHearts} Extra`}</span>` : ''}
                                            <span style="color:${l.passed ? '#4f4' : '#f44'}; font-weight:bold; font-size:0.8rem;">${l.passed ? '✓ PASS' : '✗ FAIL'}</span>
                                        </div>
                                    </div>
                                    <div class="perf-heart-progress">
                                        ${Rendering.renderHeartProgress(l.filled, l.required)}
                                    </div>
                                    <div style="display:flex; flex-wrap:wrap; gap:15px; margin-top:5px; font-size:0.75rem;">
                                        <div style="display:flex; align-items:center; gap:8px;">
                                            ${Rendering.renderHeartsCompact(l.filled)}
                                        </div>
                                    </div>
                                </div>
                                `;
            }).join('') : 'None'}
                        </div>

                        <div class="perf-section">
                            <h4>${t ? (t['blades_breakdown'] || 'Blades Breakdown (Total: {total})').replace('{total}', res.yell_count || 0) : `Blades Breakdown (Total: ${res.yell_count || 0})`}</h4>
                            ${res.breakdown && res.breakdown.blades ? res.breakdown.blades.map(b => `
                                <div class="perf-line">
                                    <span>${Tooltips.enrichAbilityText(b.source)}</span>
                                    <span class="value">+${b.value}</span>
                                </div>
                            `).join('') : ''}
                            ${res.volume_icons ? `
                                <div class="perf-line" style="border-top:1px dashed rgba(255,255,255,0.1); padding-top:2px;">
                                    <span>${t ? (t['volume'] || 'Volume Icons') : 'Volume Icons'}</span>
                                    <span class="value">+${res.volume_icons}</span>
                                </div>
                            ` : ''}
                        </div>



                        ${res.member_contributions && res.member_contributions.length > 0 ? `
                        <div class="perf-section">
                            <h4>${t ? (t['member_contrib'] || 'Member Contributions') : 'Member Contributions'}</h4>
                            ${res.member_contributions.map(m => {
                if (!m) return '';
                return `
                                <div class="perf-member-contribution">
                                    ${m.img ? `<img src="${fixImg(m.img)}" class="perf-member-img">` : ''}
                                    <div class="perf-member-info">
                                        <div class="perf-member-name">${Tooltips.enrichAbilityText(m.source || "Member")}</div>
                                        <div class="perf-member-stats">
                                            <div class="contrib-row">${Rendering.renderHeartsCompact(m.hearts)}</div>
                                            <div class="contrib-row">${Rendering.renderBladesCompact(m.blades)}</div>
                                             ${m.volume_icons ? `<span>${t ? (t['volume'] || 'Vol') : 'Vol'}: <b>${m.volume_icons}</b></span>` : ''}
                                            ${m.draw_icons ? `<span>${t ? (t['cards_draw'] || 'Drw') : 'Drw'}: <b>${m.draw_icons}</b></span>` : ''}
                                        </div>
                                    </div>
                                </div>
                            `;
            }).join('')}
                        </div>
                        ` : ''}


                        ${(res.breakdown && ((res.breakdown.requirements && res.breakdown.requirements.length > 0) || (res.breakdown.transforms && res.breakdown.transforms.length > 0))) ? `
                        <div class="perf-section">
                            <h4>${t ? (t['adjustments'] || 'Adjustments') : 'Adjustments'}</h4>
                            ${res.breakdown.requirements ? res.breakdown.requirements.map(req => {
                const colors = ['Pink', 'Red', 'Yellow', 'Green', 'Blue', 'Purple', 'Any'];
                return `
                                <div class="perf-line" style="color: #4f4; font-size: 0.8rem; gap: 4px;">
                                    <span style="opacity:0.7;">${Tooltips.enrichAbilityText(req.source)}:</span>
                                    <span>-${req.value} ${colors[req.color] || 'Any'} Req</span>
                                </div>
                                `;
            }).join('') : ''}
                            ${res.breakdown.transforms ? res.breakdown.transforms.map(tr => `
                                <div class="perf-line" style="color: #aaf; font-size: 0.8rem; gap: 4px;">
                                    <span style="opacity:0.7;">${tr.source}:</span>
                                    <span>${tr.desc}</span>
                                </div>
                            `).join('') : ''}
                        </div>
                        ` : ''}

                        ${res.yell_cards && res.yell_cards.length > 0 ? `
                        <div class="perf-section">
                            <h4>${t ? (t['yelled_cards'] || 'Yelled Cards') : 'Yelled Cards'} (${res.yell_cards.length} Total)</h4>
                            <div class="perf-yell-grid">
                                ${res.yell_cards.map(c => {
                if (!c) return '';
                const rawText = Tooltips.getEffectiveRawText(c);
                return `
                                    <div class="perf-yell-card" title="${c ? (c.name || 'Card') : 'Card'}" ${rawText ? `data-text="${rawText.replace(/"/g, '&quot;')}"` : ''}>
                                        ${c && c.img ? `<img src="${fixImg(c.img)}">` : ''}
                                        <div class="perf-card-icons">
                                            ${(c && c.blade_hearts && c.blade_hearts.some(v => v > 0)) ? c.blade_hearts.map((v, hIdx) => {
                    if (v <= 0) return '';
                    const icon = hIdx === 6 ? 'img/texticon/icon_all.png' : `img/texticon/heart_0${hIdx + 1}.png`;
                    return `<img src="${icon}" class="perf-mini-icon">`;
                }).join('') : ''}
                                             ${(c && c.volume_icons > 0) ? `<img src="img/texticon/icon_score.png" class="perf-mini-icon" title="${t ? (t['volume'] || 'Volume') : 'Volume'}">` : ''}
                                            ${(c && c.draw_icons > 0) ? `<img src="img/texticon/icon_draw.png" class="perf-mini-icon" title="${t ? (t['cards_draw'] || 'Draw') : 'Draw'}">` : ''}
                                        </div>
                                    </div>
                                `;
            }).join('')}
                            </div>
                        </div>
                        ` : ''}

                    </div>
                </div>
    `;
        });
        html += '</div>';
        content.innerHTML = html;
    },

    renderHeartProgress: (filled, required) => {
        if (!required || !Array.isArray(required)) return '';
        const filledArr = (Array.isArray(filled) ? filled : []);
        let html = '<div class="heart-progress-row">';
        for (let i = 0; i < 7; i++) {
            const reqCount = required[i] || 0;
            const filledCount = filledArr[i] || 0;
            for (let j = 0; j < reqCount; j++) {
                const isFilled = j < filledCount;
                html += `<div class="heart-pip color-${i} ${isFilled ? 'filled' : 'empty'}"></div>`;
            }
        }
        html += '</div>';
        return html;
    },

    renderHeartsCompact: (hearts) => {
        if (!hearts) return '';
        let html = '<div class="hearts-compact">';
        hearts.forEach((count, idx) => {
            if (count > 0) {
                const isAny = idx === 6;
                const colorClass = isAny ? 'color-any' : `color-${idx}`;
                const icon = isAny ? 'img/texticon/icon_all.png' : `img/texticon/heart_0${idx + 1}.png`; // fallback
                html += `<div class="heart-tag ${colorClass}"><img src="${icon}" class="heart-mini-icon"><span>${count}</span></div>`;
            }
        });
        html += '</div>';
        return (html === '<div class="hearts-compact"></div>') ? '-' : html;
    },

    renderBladeHeartsCompact: (hearts) => {
        return Rendering.renderHeartsCompact(hearts);
    },

    renderBladesCompact: (blades) => {
        if (!blades || blades <= 0) return '';
        let html = '<div class="blades-compact">';
        for (let i = 0; i < blades; i++) {
            html += `<img src="img/texticon/icon_blade.png" class="heart-mini-icon">`;
        }
        html += '</div>';
        return html;
    },

    renderTotalHeartsBreakdown: (hearts) => {
        return Rendering.renderHeartsCompact(hearts);
    },

    renderModifiers: () => { /* Placeholder for future implementation */ },
    renderGameData: () => { /* Placeholder for future implementation */ },

    updateSettingsButtons: () => {
        const t = (window.translations && State.currentLang) ? window.translations[State.currentLang] : null;

        const liveWatchBtn = document.getElementById('live-watch-btn');
        if (liveWatchBtn) {
            const label = t ? (t['live_watch'] || 'Live Watch') : 'Live Watch';
            liveWatchBtn.textContent = `${label}: ${State.isLiveWatchOn ? 'ON' : 'OFF'}`;
        }

        const hotseatBtn = document.getElementById('pvp-btn');
        if (hotseatBtn) {
            hotseatBtn.textContent = `Shared Screen: ${State.hotseatMode ? 'ON' : 'OFF'}`;
        }

        const perspectiveBtn = document.getElementById('switch-btn');
        if (perspectiveBtn) {
            perspectiveBtn.textContent = `View: P${State.perspectivePlayer + 1}`;
        }

        const friendlyBtn = document.getElementById('friendly-abilities-btn');
        if (friendlyBtn) {
            const label = t ? (t['friendly_abilities'] || 'Friendly Abilities') : 'Friendly Abilities';
            friendlyBtn.textContent = `${label}: ${State.showFriendlyAbilities ? 'ON' : 'OFF'}`;
        }

        const langBtn = document.getElementById('lang-btn');
        if (langBtn) {
            langBtn.textContent = State.currentLang === 'jp' ? 'English' : '日本語';
        }
    },

    showPerfTab: (tab) => {
        const resultTab = document.getElementById('perf-tab-result');
        const historyTab = document.getElementById('perf-tab-history');
        const resultBtn = document.getElementById('tab-btn-result');
        const historyBtn = document.getElementById('tab-btn-history');

        if (!resultTab || !historyTab) return;

        if (tab === 'result') {
            resultTab.style.display = 'block';
            historyTab.style.display = 'none';
            resultBtn.classList.add('active');
            historyBtn.classList.remove('active');
        } else {
            resultTab.style.display = 'none';
            historyTab.style.display = 'block';
            resultBtn.classList.remove('active');
            historyBtn.classList.add('active');
            Rendering.renderTurnHistory();
        }
    },

    renderTurnHistory: () => {
        const container = document.getElementById('performance-history-content');
        if (!container) return;

        const state = State.data;
        const history = state.turn_history || state.turn_events || [];

        const currentLang = State.currentLang;
        const t = translations ? translations[currentLang] : {};

        if (history.length === 0) {
            const label = t['no_history'] || 'No history available for this turn.';
            container.innerHTML = `<div style="text-align:center; padding:20px; opacity:0.6;">${label}</div>`;
            return;
        }

        let html = '';
        history.forEach((event) => {
            const phaseKey = Rendering.getPhaseKey(event.phase);
            const playerLabel = event.player_id === State.perspectivePlayer ? (t['you'] || 'You') : (t['opp'] || 'Opponent');
            const typeClass = event.event_type.toLowerCase();

            html += `
                <div class="turn-event-item ${typeClass}">
                    <div class="event-header">
                        <span>Turn ${event.turn} - <span class="event-phase-tag">${t[phaseKey] || event.phase}</span></span>
                        <span>${playerLabel}</span>
                    </div>
                    <div class="event-source">${event.event_type}</div>
                    <div class="event-description">${event.description}</div>
                </div>
            `;
        });
        container.innerHTML = html;
    }
};
