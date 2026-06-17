import { State } from '../state.js';
import { ModalManager } from '../utils/ModalManager.js';
import { Tooltips } from '../ui_tooltips.js';
import * as i18n from '../i18n/index.js';

const HEART_COLORS = ['Smile', 'Pure', 'Cool', 'Green', 'Blue', 'Purple', 'Wildcard'];

function escapeHtml(v) {
    return String(v ?? '').replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;');
}

function cardName(id) {
    const card = State.resolveCardData(id);
    return card ? card.name || card.card_no || `#${id}` : `#${id}`;
}

function cardNo(id) {
    const card = State.resolveCardData(id);
    return card ? card.card_no || '' : '';
}

function boolYes(v) { return v ? 'Yes' : 'No'; }
function boolVal(v) { return v ? 'true' : 'false'; }

function section(title, content, badge) {
    const sec = document.createElement('div');
    sec.className = 'gs-section';
    const hdr = document.createElement('div');
    hdr.className = 'gs-section-header';
    hdr.textContent = title;
    if (badge !== undefined) {
        const b = document.createElement('span');
        b.className = 'gs-badge';
        b.textContent = badge;
        hdr.appendChild(b);
    }
    sec.appendChild(hdr);
    if (typeof content === 'string') {
        sec.insertAdjacentHTML('beforeend', content);
    } else {
        sec.appendChild(content);
    }
    return sec;
}

function gridItem(label, value, valueClass) {
    const cls = valueClass ? `gs-value ${valueClass}` : 'gs-value';
    return `<div class="gs-grid-item"><span class="gs-label">${escapeHtml(label)}</span><span class="${cls}">${escapeHtml(String(value))}</span></div>`;
}

function kvRows(map) {
    return map.map(([k, v]) => `<div class="gs-track-item"><div class="track-kv"><span>${escapeHtml(k)}</span><span class="tv">${escapeHtml(String(v ?? ''))}</span></div></div>`).join('');
}

export const GameStateModal = {
    _currentTab: 'global',

    open: () => {
        ModalManager.show('game-state-modal');
        GameStateModal.renderAll();
    },

    close: () => {
        ModalManager.hide('game-state-modal');
    },

    showTab: (tab) => {
        const tabs = ['global', 'player', 'zones', 'tracking'];
        tabs.forEach(t => {
            const panel = document.getElementById(`gs-tab-${t}`);
            const btn = document.querySelector(`.gs-modal-tabs [data-tab="${t}"]`);
            if (panel) panel.style.display = t === tab ? 'block' : 'none';
            if (btn) btn.classList.toggle('active', t === tab);
        });
        GameStateModal._currentTab = tab;
        GameStateModal.renderAll();
    },

    renderAll: () => {
        const state = State.data;
        if (!state) return;
        GameStateModal.renderGlobalTab(state);
        GameStateModal.renderPlayerTab(state);
        GameStateModal.renderZonesTab(state);
        GameStateModal.renderTrackingTab(state);
    },

    renderGlobalTab: (state) => {
        const container = document.getElementById('gs-tab-global');
        if (!container) return;
        container.innerHTML = '';

        const items = [];

        // Turn info
        const turnLines = [
            ['Turn', state.turn ?? '?'],
            ['Phase', state.phase ?? '?'],
            ['Turn Phase', state.current_turn_phase ?? '?'],
            ['Active Player', state.active_player ?? '?'],
            ['Game Result', state.game_result ?? 'Ongoing'],
            ['Is First Turn', boolYes(state.is_first_turn)],
            ['Turn Order Changed', boolYes(state.turn_order_changed)],
        ];
        const turnGrid = turnLines.map(([k, v]) => gridItem(k, v)).join('');
        items.push(section('Turn', `<div class="gs-grid">${turnGrid}</div>`));

        // Baton touch
        const bt = state;
        const btLines = [
            ['Baton Touch Count', bt.baton_touch_count ?? 0],
            ['Zero Cost', boolYes(bt.baton_touch_zero_cost)],
            ['Replaced Card', bt.baton_touch_replaced_member_id != null ? cardName(bt.baton_touch_replaced_member_id) : 'none'],
            ['Replaced Cost', bt.baton_touch_replaced_member_cost != null ? bt.baton_touch_replaced_member_cost : 'none'],
            ['Arriving Card', bt.baton_touch_arriving_card_id != null ? cardName(bt.baton_touch_arriving_card_id) : 'none'],
        ];
        const btGrid = btLines.map(([k, v]) => gridItem(k, v)).join('');
        items.push(section('Baton Touch', `<div class="gs-grid">${btGrid}</div>`));

        // Card tracking
        const movedNames = (state.cards_moved_this_turn || []).map(id => cardName(id)).join(', ') || 'none';
        const appearedNames = (state.cards_appeared_this_turn || []).map(id => cardName(id)).join(', ') || 'none';
        const areasStr = (state.areas_placed_this_turn || []).join(', ') || 'none';
        const lastMove = state.last_area_move_card_id != null ? `${cardName(state.last_area_move_card_id)} (by ${state.last_area_move_by_player || '?'})` : 'none';

        const ctLines = [
            ['Cards Moved', movedNames],
            ['Cards Appeared', appearedNames],
            ['Areas Placed', areasStr],
            ['Last Area Move', lastMove],
            ['Last Energy By Effect', boolYes(state.last_energy_placed_by_effect)],
        ];
        const ctGrid = ctLines.map(([k, v]) => gridItem(k, v)).join('');
        items.push(section('Card Movement', `<div class="gs-grid">${ctGrid}</div>`));

        // Flags
        const flags = [
            ['Position Changed', boolYes(state.position_change_occurred_this_turn)],
            ['Formation Changed', boolYes(state.formation_change_occurred_this_turn)],
            ['Opponent Live Success', boolYes(state.opponent_live_success_this_turn)],
            ['Opponent No Excess Heart', boolYes(state.opponent_live_no_excess_heart_this_turn)],
            ['Self No Excess Heart', boolYes(state.self_no_excess_heart_this_turn)],
            ['Opponent Surplus', state.opponent_live_surplus_count ?? 0],
            ['Self Surplus', state.self_live_surplus_count ?? 0],
            ['Live Success Triggered', boolYes(state.live_success_triggered_this_turn)],
            ['Live Surplus Ready', boolYes(state.live_surplus_ready_this_turn)],
            ['Live Being Performed', boolYes(state.live_being_performed)],
            ['Deck Refresh Pending', boolYes(state.deck_refresh_pending)],
            ['Loop Detected', boolYes(state.loop_detected)],
            ['Draw State', boolYes(state.draw_state)],
            ['Cheer Checks', `${state.cheer_checks_done ?? 0}/${state.cheer_checks_required ?? 0}`],
        ];
        const flagGrid = flags.map(([k, v]) => gridItem(k, v)).join('');
        items.push(section('Flags & Live', `<div class="gs-grid">${flagGrid}</div>`));

        items.forEach(el => container.appendChild(el));
    },

    renderPlayerTab: (state) => {
        const container = document.getElementById('gs-tab-player');
        if (!container) return;
        container.innerHTML = '';

        if (!state.player1 && !state.player2) {
            container.textContent = 'No player data';
            return;
        }

        const cols = document.createElement('div');
        cols.className = 'gs-player-columns';

        [state.player1, state.player2].forEach((p, idx) => {
            if (!p) return;
            const panel = document.createElement('div');
            panel.className = `gs-player-panel p${idx}`;

            const title = document.createElement('div');
            title.className = 'gs-player-title';
            const isMe = idx === State.perspectivePlayer;
            title.innerHTML = `<span>${isMe ? 'You' : 'Opponent'} (P${idx + 1})</span> <span class="gs-badge">${escapeHtml(p.main_deck_count ?? '?')} deck</span>`;
            panel.appendChild(title);

            // Totals grid
            const tLines = [
                ['Cost Reduction', p.cost_reduction ?? 0],
                ['Prevent Baton', boolYes(!!(p.prevent_baton_touch || p.prevent_baton))],
                ['Debut Count This Turn', p.debut_count_this_turn ?? 0],
                ['Areas Locked', (p.areas_locked_this_turn || []).join(', ') || 'none'],
            ];
            if (p.total_hearts) {
                p.total_hearts.forEach((h, ci) => { if (h > 0) tLines.push([`Hearts ${HEART_COLORS[ci] || ci}`, h]); });
            }
            if (p.live_card_scores) {
                Object.entries(p.live_card_scores).forEach(([no, sc]) => tLines.push([`Score ${no}`, sc]));
            }
            const tGrid = tLines.map(([k, v]) => gridItem(k, v)).join('');
            panel.insertAdjacentHTML('beforeend', `<div class="gs-grid" style="margin-bottom:6px;">${tGrid}</div>`);
            panel.appendChild(document.createElement('hr'));

            // Stage cards — like board card spacing
            const stage = p.stage;
            if (stage) {
                const slots = [
                    { label: 'Left', card: stage.left_side, under: stage.left_under || [] },
                    { label: 'Center', card: stage.center, under: stage.center_under || [] },
                    { label: 'Right', card: stage.right_side, under: stage.right_under || [] },
                ];

                const cardRow = document.createElement('div');
                cardRow.className = 'gs-card-row';

                slots.forEach(({ label, card, under }) => {
                    const slot = document.createElement('div');
                    slot.className = 'gs-card-slot';
                    if (!card) {
                        slot.innerHTML = `<span class="card-sub">${label}: empty</span>`;
                    } else {
                        const orient = card.orientation || 'Active';
                        const orientBadge = orient === 'Wait' ? 'wait-state' : 'active-state';
                        const baseCost = card.cost ?? '?';
                        const bonusCost = card.bonus_cost ?? 0;
                        const effectiveCost = baseCost !== '?' ? Math.max(0, (baseCost + bonusCost)) : '?';
                        const moved = (state.cards_moved_this_turn || []).includes(card.id);
                        const negated = (state.negated_abilities || []).includes(card.id);

                        let badges = `<span class="card-badge ${orientBadge}">${orient}</span>`;
                        if (moved) badges += ` <span class="card-badge moved">moved</span>`;
                        if (negated) badges += ` <span class="card-badge negated">negated</span>`;

                        const heartStr = card.base_heart && typeof card.base_heart === 'object'
                            ? Object.entries(card.base_heart).filter(([_, c]) => c > 0).map(([col, c]) => `${col}:${c}`).join(' ')
                            : '';

                        slot.innerHTML = `
                            <div><span class="card-name">${escapeHtml(card.name || `#${card.id}`)}</span> ${badges}</div>
                            <div class="card-sub">${label} · Cost: ${baseCost}${bonusCost !== 0 ? ` (${bonusCost > 0 ? '+' : ''}${bonusCost})` : ''} → ${effectiveCost}</div>
                            <div class="card-sub">Blade: ${card.total_blade ?? card.blade ?? 0}${(card.bonus_blade ?? 0) !== 0 ? ` (${(card.bonus_blade ?? 0) > 0 ? '+' : ''}${card.bonus_blade ?? 0})` : ''}</div>
                            ${heartStr ? `<div class="card-hearts">${escapeHtml(heartStr)}</div>` : ''}
                            ${(card.bonus_score ?? 0) !== 0 ? `<div class="card-sub">Score bonus: ${card.bonus_score > 0 ? '+' : ''}${card.bonus_score}</div>` : ''}
                            ${(card.bonus_hearts || []).some(h => h !== 0) ? `<div class="card-sub">Heart bonus: [${(card.bonus_hearts || []).join(', ')}]</div>` : ''}
                            ${card.heart_transform ? `<div class="card-sub">Heart → ${card.heart_transform}</div>` : ''}
                            ${under.length > 0 ? `<div class="card-sub">Under: ${under.map(u => escapeHtml(u.name || `#${u.id}`)).join(', ')}</div>` : ''}
                        `;
                    }
                    cardRow.appendChild(slot);
                });
                panel.appendChild(cardRow);
            } else {
                panel.insertAdjacentHTML('beforeend', '<div class="card-sub">No stage data</div>');
            }

            // Gained abilities on this player
            if (p.gained_abilities && p.gained_abilities.length > 0) {
                const abDiv = document.createElement('div');
                abDiv.className = 'gs-section';
                abDiv.style.marginTop = '8px';
                abDiv.style.padding = '6px 8px';
                abDiv.style.background = 'rgba(255,255,255,0.03)';
                abDiv.style.borderRadius = '6px';
                const abTitle = document.createElement('div');
                abTitle.style.cssText = 'font-size:0.75rem;font-weight:600;color:var(--accent-gold);margin-bottom:2px;';
                abTitle.textContent = 'Gained Abilities';
                abDiv.appendChild(abTitle);
                p.gained_abilities.forEach(a => {
                    const el = document.createElement('div');
                    el.style.cssText = 'font-size:0.65rem;color:var(--text-dim);padding:1px 0;font-family:monospace;';
                    el.textContent = a;
                    abDiv.appendChild(el);
                });
                panel.appendChild(abDiv);
            }

            // Active restrictions
            if (p.active_restrictions && p.active_restrictions.length > 0) {
                const rDiv = document.createElement('div');
                rDiv.style.cssText = 'margin-top:6px;font-size:0.7rem;';
                rDiv.innerHTML = `<strong style="color:var(--accent-pink);">Restrictions:</strong> ${p.active_restrictions.join(', ')}`;
                panel.appendChild(rDiv);
            }

            cols.appendChild(panel);
        });

        container.appendChild(cols);
    },

    renderZonesTab: (state) => {
        const container = document.getElementById('gs-tab-zones');
        if (!container) return;
        container.innerHTML = '';

        if (!state.player1 && !state.player2) {
            container.textContent = 'No zone data';
            return;
        }

        [state.player1, state.player2].forEach((p, idx) => {
            if (!p) return;
            const isMe = idx === State.perspectivePlayer;
            const label = `${isMe ? 'You' : 'Opponent'} (P${idx + 1})`;

            const zoneRow = document.createElement('div');
            zoneRow.className = 'gs-zone-row';

            const mkZone = (name, cards, extra) => {
                const box = document.createElement('div');
                box.className = 'gs-zone-box';
                box.innerHTML = `<div class="zone-name"><span>${escapeHtml(name)}</span><span>${extra ?? (cards ? cards.length : 0)}</span></div>`;
                if (cards && cards.length > 0) {
                    const chips = document.createElement('div');
                    chips.className = 'zone-cards';
                    cards.forEach(c => {
                        const chip = document.createElement('span');
                        chip.className = 'zone-card-chip';
                        chip.textContent = c.name || `#${c.id}`;
                        chips.appendChild(chip);
                    });
                    box.appendChild(chips);
                }
                zoneRow.appendChild(box);
            };

            mkZone('Main Deck', [], `${p.main_deck_count ?? 0} cards`);
            mkZone('Energy Deck', [], `${p.energy_deck_count ?? 0} cards`);
            mkZone('Energy', p.energy?.cards || []);
            mkZone('Hand', p.hand?.cards || []);
            mkZone('Waitroom', p.waitroom?.cards || []);
            mkZone('Live Zone', p.live_zone?.cards || []);
            mkZone('Success Zone', p.success_live_card_zone?.cards || []);

            container.appendChild(section(label, zoneRow));
        });

        // Resolution zone and looked cards
        if (state.looked_cards && state.looked_cards.cards && state.looked_cards.cards.length > 0) {
            const rzRow = document.createElement('div');
            rzRow.className = 'gs-zone-row';
            const box = document.createElement('div');
            box.className = 'gs-zone-box';
            box.innerHTML = `<div class="zone-name"><span>Looked / Revealed</span><span>${state.looked_cards.cards.length}</span></div>`;
            const chips = document.createElement('div');
            chips.className = 'zone-cards';
            state.looked_cards.cards.forEach(c => {
                const chip = document.createElement('span');
                chip.className = 'zone-card-chip';
                chip.textContent = c.name || `#${c.id}`;
                chips.appendChild(chip);
            });
            box.appendChild(chips);
            rzRow.appendChild(box);
            container.appendChild(section('Global Zones', rzRow));
        }
    },

    renderTrackingTab: (state) => {
        const container = document.getElementById('gs-tab-tracking');
        if (!container) return;
        container.innerHTML = '';

        const grid = document.createElement('div');
        grid.className = 'gs-tracking-grid';

        const mkBox = (title, rows) => {
            const box = document.createElement('div');
            box.className = 'gs-tracking-box';
            box.insertAdjacentHTML('beforeend', `<h4>${escapeHtml(title)}</h4>`);
            box.insertAdjacentHTML('beforeend', rows);
            return box;
        };

        // Turn-limited abilities
        const turn1 = (state.turn_limited_abilities_used || []);
        const turn2 = state.turn2_abilities_played || {};
        let tlaHtml = '';
        if (turn1.length > 0) tlaHtml += `<div class="gs-track-item">Turn1: ${turn1.join(', ')}</div>`;
        if (Object.keys(turn2).length > 0) tlaHtml += kvRows(Object.entries(turn2));
        if (!tlaHtml) tlaHtml = '<div class="gs-track-item">none</div>';
        grid.appendChild(mkBox('Turn-Limited Abilities Used', tlaHtml));

        // Turn limit usage
        const tlu = state.turn_limit_usage || {};
        grid.appendChild(mkBox('Turn Limit Usage', Object.keys(tlu).length > 0 ? kvRows(Object.entries(tlu)) : '<div class="gs-track-item">none</div>'));

        // Auto ability trigger counts
        const aatc = state.auto_ability_trigger_counts || {};
        grid.appendChild(mkBox('Auto Ability Trigger Counts', Object.keys(aatc).length > 0 ? kvRows(Object.entries(aatc)) : '<div class="gs-track-item">none</div>'));

        // Prohibition effects
        const proh = state.prohibition_effects || [];
        const dproh = state.delayed_prohibition_effects || [];
        let prohHtml = '';
        if (proh.length > 0) prohHtml += proh.map(p => `<div class="gs-track-item">${escapeHtml(p)}</div>`).join('');
        if (dproh.length > 0) prohHtml += dproh.map(p => `<div class="gs-track-item">[delayed] ${escapeHtml(p)}</div>`).join('');
        if (!prohHtml) prohHtml = '<div class="gs-track-item">none</div>';
        grid.appendChild(mkBox('Prohibition Effects', prohHtml));

        // Cannot activate members
        const ca = state.cannot_activate_members || [];
        const cca = state.constant_cannot_activate_members || [];
        let caHtml = '';
        if (ca.length > 0) caHtml += ca.map(p => `<div class="gs-track-item">${escapeHtml(p)}</div>`).join('');
        if (cca.length > 0) caHtml += cca.map(p => `<div class="gs-track-item">[constant] ${escapeHtml(p)}</div>`).join('');
        if (!caHtml) caHtml = '<div class="gs-track-item">none</div>';
        grid.appendChild(mkBox('Cannot Activate Members', caHtml));

        // Negated abilities
        const neg = state.negated_abilities || [];
        grid.appendChild(mkBox('Negated Card IDs', neg.length > 0 ? neg.map(id => `<div class="gs-track-item">#${id} (${escapeHtml(cardName(id))})</div>`).join('') : '<div class="gs-track-item">none</div>'));

        // Non-stackable effects
        const ns = state.non_stackable_effects || [];
        grid.appendChild(mkBox('Non-Stackable Effects', ns.length > 0 ? ns.map(e => `<div class="gs-track-item">${escapeHtml(e)}</div>`).join('') : '<div class="gs-track-item">none</div>'));

        // Temporary effects
        const te = state.temporary_effects || [];
        let teHtml = '';
        if (te.length > 0) {
            te.forEach(e => {
                teHtml += `<div class="gs-track-item" style="border-bottom:1px solid rgba(255,255,255,0.05);padding:2px 0;">
                    <div class="track-kv"><span>Type</span><span class="tv">${escapeHtml(e.effect_type)}</span></div>
                    <div class="track-kv"><span>Duration</span><span class="tv">${escapeHtml(e.duration)}</span></div>
                    <div class="track-kv"><span>Turn</span><span class="tv">${e.created_turn ?? '?'}</span></div>
                    <div class="track-kv"><span>Target</span><span class="tv">${escapeHtml(e.target_player_id || '?')}</span></div>
                    <div class="track-kv" style="color:var(--text-dim);font-size:0.65rem;white-space:normal;word-break:break-all;"><span>${escapeHtml(e.description || '')}</span></div>
                </div>`;
            });
        } else {
            teHtml = '<div class="gs-track-item">none</div>';
        }
        grid.appendChild(mkBox('Temporary Effects', teHtml));

        // Replacement effects
        const re = state.replacement_effects || [];
        let reHtml = '';
        if (re.length > 0) {
            re.forEach(e => {
                reHtml += `<div class="gs-track-item" style="border-bottom:1px solid rgba(255,255,255,0.05);padding:2px 0;">
                    <div class="track-kv"><span>Card</span><span class="tv">${e.card_id != null ? escapeHtml(cardName(e.card_id)) : '?'}</span></div>
                    <div class="track-kv"><span>Player</span><span class="tv">${escapeHtml(e.player_id || '?')}</span></div>
                    <div class="track-kv"><span>Event</span><span class="tv">${escapeHtml(e.original_event || '?')}</span></div>
                    <div class="track-kv"><span>Choice</span><span class="tv">${e.is_choice_based ? 'Yes' : 'No'}</span></div>
                </div>`;
            });
        } else {
            reHtml = '<div class="gs-track-item">none</div>';
        }
        grid.appendChild(mkBox('Replacement Effects', reHtml));

        container.appendChild(grid);
    },
};
