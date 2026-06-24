import { State } from '../state.js';
import { ModalManager } from '../utils/ModalManager.js';
import { Tooltips } from '../ui_tooltips.js';
import * as i18n from '../i18n/index.js';

const HEART_NAMES = ['Smile', 'Pure', 'Cool', 'Green', 'Blue', 'Purple', 'Wildcard', 'All'];

function esc(v) {
    return String(v ?? '').replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;');
}

function cardName(id) {
    const c = State.resolveCardData(id);
    return c ? c.name || c.card_no || `#${id}` : `#${id}`;
}

function cardTypeClass(t) {
    const m = { Member: 'member', Live: 'live', Energy: 'energy' };
    return m[t] || 'member';
}

function bool(v) { return v ? 'Yes' : 'No'; }

function section(title, content) {
    const s = document.createElement('div'); s.className = 'gs-section';
    const h = document.createElement('div'); h.className = 'gs-section-header'; h.textContent = title;
    s.appendChild(h);
    if (typeof content === 'string') s.insertAdjacentHTML('beforeend', content);
    else s.appendChild(content);
    return s;
}

function grid(items) {
    return `<div class="gs-grid">${items.map(([l, v, c]) => {
        const cls = c ? `gs-value ${c}` : 'gs-value';
        return `<div class="gs-grid-item"><span class="gs-label">${esc(l)}</span><span class="${cls}">${esc(String(v ?? ''))}</span></div>`;
    }).join('')}</div>`;
}

function trackKV(items) {
    return items.map(([k, v]) =>
        `<div class="gs-track-item"><div class="track-kv"><span>${esc(k)}</span><span class="tv">${esc(String(v ?? ''))}</span></div></div>`
    ).join('');
}

function mkBox(title, html) {
    return `<div class="gs-tracking-box"><h4>${esc(title)}</h4>${html}</div>`;
}

function chip(name, cls) {
    return `<span class="zone-card-chip ${cls || ''}">${esc(name)}</span>`;
}

function renderCardSlot(slot, prefix, state) {
    if (!slot) return `<div class="gs-card-slot"><span class="card-sub">${prefix}: empty</span></div>`;
    const orient = slot.orientation || 'Active';
    const orientCls = orient === 'Wait' ? 'badge wait-state' : 'badge active-state';
    const baseCost = slot.cost ?? '?';
    const bonusCost = slot.bonus_cost ?? 0;
    const effCost = baseCost !== '?' ? Math.max(0, (baseCost + bonusCost)) : '?';
    const moved = (state.cards_moved_this_turn || []).includes(slot.id);
    const negated = (state.negated_abilities || []).includes(slot.id);
    const typeCls = cardTypeClass(slot.type);
    let badges = `<span class="${orientCls}">${orient}</span>`;
    if (moved) badges += ` <span class="badge moved">moved</span>`;
    if (negated) badges += ` <span class="badge negated">negated</span>`;
    const heartStr = slot.base_heart && typeof slot.base_heart === 'object'
        ? Object.entries(slot.base_heart).filter(([_, c]) => c > 0).map(([col, c]) => `${col}:${c}`).join(' ')
        : '';
    return `<div class="gs-card-slot">
        <div><span class="card-name">${esc(slot.name || `#${slot.id}`)}</span> ${badges} <span class="badge ${typeCls}">${esc(slot.type)}</span></div>
        <div class="card-sub">${prefix} · Cost: ${baseCost}${bonusCost !== 0 ? ` (${bonusCost > 0 ? '+' : ''}${bonusCost})` : ''} → ${effCost}</div>
        <div class="card-sub">Blade: ${slot.total_blade ?? slot.blade ?? 0}${(slot.bonus_blade ?? 0) !== 0 ? ` (${(slot.bonus_blade ?? 0) > 0 ? '+' : ''}${slot.bonus_blade ?? 0})` : ''}</div>
        ${heartStr ? `<div class="card-hearts">${esc(heartStr)}</div>` : ''}
        ${(slot.bonus_score ?? 0) !== 0 ? `<div class="card-sub">Score: +${slot.bonus_score}</div>` : ''}
        ${(slot.bonus_hearts || []).some(h => h !== 0) ? `<div class="card-sub">Heart bonus: [${(slot.bonus_hearts || []).join(', ')}]</div>` : ''}
        ${slot.heart_transform ? `<div class="card-sub">Heart → ${slot.heart_transform}</div>` : ''}
    </div>`;
}

let _conditionsCache = null;
let _conditionsError = null;

export const GameStateModal = {
    _currentTab: 'global',

    open: () => {
        ModalManager.show('game-state-modal');
        GameStateModal.renderAll();
    },
    close: () => { ModalManager.hide('game-state-modal'); },

    showTab: (tab) => {
        ['global', 'player', 'zones', 'tracking', 'conditions'].forEach(t => {
            const p = document.getElementById(`gs-tab-${t}`);
            const b = document.querySelector(`.gs-modal-tabs [data-tab="${t}"]`);
            if (p) p.style.display = t === tab ? 'block' : 'none';
            if (b) b.classList.toggle('active', t === tab);
        });
        GameStateModal._currentTab = tab;
        GameStateModal.renderAll();
        if (tab === 'conditions') GameStateModal.renderConditionsTab();
    },

    renderAll: () => {
        const s = State.data;
        if (!s) return;
        GameStateModal.renderGlobalTab(s);
        GameStateModal.renderPlayerTab(s);
        GameStateModal.renderZonesTab(s);
        GameStateModal.renderTrackingTab(s);
        if (GameStateModal._currentTab === 'conditions') GameStateModal.renderConditionsTab();
    },

    fetchAndCacheConditions: async () => {
        _conditionsError = null;
        _conditionsCache = null;
        try {
            const controller = new AbortController();
            const timeout = setTimeout(() => controller.abort(), 5000);
            const res = await fetch('/api/debug/conditions', { signal: controller.signal });
            clearTimeout(timeout);
            if (!res.ok) throw new Error(`HTTP ${res.status}`);
            _conditionsCache = await res.json();
        } catch (e) {
            _conditionsError = e.name === 'AbortError' ? 'Request timed out' : e.message;
            _conditionsCache = [];
        }
        return _conditionsCache;
    },

    renderConditionsTab: () => {
        const c = document.getElementById('gs-tab-conditions');
        if (!c) return;

        c.innerHTML = '<div style="padding:12px;font-size:0.85rem;opacity:0.6;">Fetching conditions...</div>';

        GameStateModal.fetchAndCacheConditions().then(conditions => {
            if (_conditionsError) {
                c.innerHTML = `<div style="padding:12px;color:var(--accent-pink);"><b>Error:</b> ${_conditionsError}</div>`;
                return;
            }
            if (!conditions || conditions.length === 0) {
                c.innerHTML = '<div style="padding:12px;opacity:0.6;">Engine returned no conditions for any card in any zone. This iterates all abilities on all cards currently in play (stage, hand, energy, waitroom, live_zone, success_live_zone). If cards with ability conditions exist, check that the condition is parsed into one of: <code>activation_condition_parsed</code>, <code>condition</code>, <code>alternative_condition</code>, <code>result_condition</code>.</div>';
                return;
            }

            const trueCount = conditions.filter(c => c.result).length;
            const falseCount = conditions.filter(c => !c.result).length;

            const rows = conditions.map((cond, i) => {
                const rCls = cond.result ? 'color:#4ade80;background:rgba(34,197,94,0.15);' : 'color:#f87171;background:rgba(239,68,68,0.12);';
                const rLbl = cond.result ? 'PASS' : 'FAIL';
                const av = cond.actual_value || {};
                const actualStr = av.measure ? `${esc(av.measure)}` : '-';
                const thresh = av.threshold != null ? ` [need ≥ ${av.threshold}]` : '';
                return `<tr style="vertical-align:top;${i%2===1?'background:rgba(255,255,255,0.015);':''}">
                    <td style="padding:4px 6px;border-bottom:1px solid rgba(255,255,255,0.06);font-size:0.65rem;"><span style="display:inline-block;padding:1px 5px;border-radius:3px;font-weight:bold;font-size:0.6rem;${rCls}">${rLbl}</span></td>
                    <td style="padding:4px 6px;border-bottom:1px solid rgba(255,255,255,0.06);font-size:0.65rem;white-space:nowrap;">P${cond.player+1}</td>
                    <td style="padding:4px 6px;border-bottom:1px solid rgba(255,255,255,0.06);font-size:0.65rem;white-space:nowrap;">${esc(cond.zone)}</td>
                    <td style="padding:4px 6px;border-bottom:1px solid rgba(255,255,255,0.06);font-size:0.65rem;max-width:100px;overflow:hidden;text-overflow:ellipsis;white-space:nowrap;" title="${esc(cond.card_name)}">${esc(cond.card_name)}</td>
                    <td style="padding:4px 6px;border-bottom:1px solid rgba(255,255,255,0.06);font-size:0.65rem;white-space:nowrap;">${esc(cond.condition_type||cond.field||'')}</td>
                    <td style="padding:4px 6px;border-bottom:1px solid rgba(255,255,255,0.06);font-size:0.65rem;max-width:130px;overflow:hidden;text-overflow:ellipsis;white-space:nowrap;" title="${esc(cond.condition_text||'')}">${esc(cond.condition_text||'-')}</td>
                    <td style="padding:4px 6px;border-bottom:1px solid rgba(255,255,255,0.06);font-size:0.65rem;white-space:nowrap;">${actualStr}${thresh}</td>
                </tr>`;
            }).join('');

            c.innerHTML = `
                <div style="margin:6px;font-size:0.65rem;opacity:0.5;">All conditions are evaluated read-only — no game state modification.</div>
                <div style="margin:6px;display:flex;justify-content:space-between;align-items:center;gap:8px;">
                    <strong style="font-size:0.75rem;">Condition Evaluation (${conditions.length} total)</strong>
                    <span style="font-size:0.7rem;">
                        <span style="color:#4ade80;">${trueCount} PASS</span>
                        <span style="opacity:0.4;"> / </span>
                        <span style="color:#f87171;">${falseCount} FAIL</span>
                    </span>
                    <button class="btn btn-sm btn-secondary" id="gs-reeval-conds" style="font-size:0.65rem;">Re-evaluate</button>
                </div>
                <div style="overflow:auto;border:1px solid rgba(255,255,255,0.06);border-radius:4px;max-height:55vh;">
                    <table style="width:100%;border-collapse:collapse;font-size:0.65rem;min-width:700px;">
                        <thead><tr style="background:rgba(15,23,42,0.95);text-transform:uppercase;position:sticky;top:0;">
                            <th style="padding:6px;text-align:left;border-bottom:1px solid rgba(255,255,255,0.08);">Result</th>
                            <th style="padding:6px;text-align:left;border-bottom:1px solid rgba(255,255,255,0.08);">P</th>
                            <th style="padding:6px;text-align:left;border-bottom:1px solid rgba(255,255,255,0.08);">Zone</th>
                            <th style="padding:6px;text-align:left;border-bottom:1px solid rgba(255,255,255,0.08);">Card</th>
                            <th style="padding:6px;text-align:left;border-bottom:1px solid rgba(255,255,255,0.08);">Type</th>
                            <th style="padding:6px;text-align:left;border-bottom:1px solid rgba(255,255,255,0.08);">Condition</th>
                            <th style="padding:6px;text-align:left;border-bottom:1px solid rgba(255,255,255,0.08);">Actual</th>
                        </tr></thead>
                        <tbody>${rows}</tbody>
                    </table>
                </div>
                <details style="margin:6px;">
                    <summary style="cursor:pointer;opacity:0.5;font-size:0.65rem;">Raw JSON</summary>
                    <pre style="margin:4px 0 0;padding:6px;background:#05070d;border-radius:3px;font-size:0.55rem;line-height:1.3;color:#dbeafe;white-space:pre-wrap;word-break:break-word;max-height:200px;overflow:auto;">${esc(JSON.stringify(conditions,null,2))}</pre>
                </details>`;

            document.getElementById('gs-reeval-conds')?.addEventListener('click', () => {
                _conditionsCache = null;
                _conditionsError = null;
                GameStateModal.renderConditionsTab();
            });
        });
    },

    // ─── Global Tab ──────────────────────────────────────────
    renderGlobalTab: (s) => {
        const c = document.getElementById('gs-tab-global');
        if (!c) return;
        c.innerHTML = '';

        const turnLines = [
            ['Turn', s.turn], ['Phase', s.phase], ['Turn Phase', s.current_turn_phase],
            ['Active Player', s.active_player], ['Game Result', s.game_result],
            ['Is First Turn', bool(s.is_first_turn)], ['Turn Order Changed', bool(s.turn_order_changed)],
            ['Heart Color Decision', s.heart_color_decision_phase || 'none'],
        ];
        c.appendChild(section('Turn', grid(turnLines)));

        // RPS
        const rpsLines = [
            ['RPS Winner', s.rps_winner != null ? `P${s.rps_winner}` : 'none'],
            ['P1 RPS Choice', s.player1_rps_choice != null ? s.player1_rps_choice : 'none'],
            ['P2 RPS Choice', s.player2_rps_choice != null ? s.player2_rps_choice : 'none'],
            ['Pending RPS Player', s.pending_rps_player_id != null ? `P${s.pending_rps_player_id}` : 'none'],
        ];
        if (s.rps_winner != null || s.player1_rps_choice != null) {
            c.appendChild(section('RPS', grid(rpsLines)));
        }

        // Ability Queue state summary
        const hasQueue = s.ability_queue_entries && s.ability_queue_entries.length > 0;
        const qLines = [
            ['Queue State', s.ability_queue_state || 'Idle'],
            ['Current Index', s.ability_queue_current_index ?? 0],
            ['Pending Entries', hasQueue ? s.ability_queue_entries.length : 0],
        ];
        c.appendChild(section('Ability Queue', grid(qLines)));

        // Activating card
        const actLines = [
            ['Activating Card', s.activating_card != null ? cardName(s.activating_card) : 'none'],
            ['Activating Ability Index', s.activating_ability_index != null ? s.activating_ability_index : 'none'],
            ['Just Completed Key', s.just_completed_ability_key || 'none'],
        ];
        c.appendChild(section('Active Ability', grid(actLines)));

        // Baton touch
        const btLines = [
            ['Baton Touch Count', s.baton_touch_count ?? 0],
            ['Zero Cost', bool(s.baton_touch_zero_cost)],
            ['Replaced', s.baton_touch_replaced_member_id != null ? cardName(s.baton_touch_replaced_member_id) : 'none'],
            ['Replaced Cost', s.baton_touch_replaced_member_cost != null ? s.baton_touch_replaced_member_cost : 'none'],
            ['Arriving', s.baton_touch_arriving_card_id != null ? cardName(s.baton_touch_arriving_card_id) : 'none'],
            ['Last Vacated', s.last_vacated_stage_area || 'none'],
        ];
        c.appendChild(section('Baton Touch', grid(btLines)));

        // Card tracking
        const ctLines = [
            ['Cards Moved', (s.cards_moved_this_turn || []).map(id => cardName(id)).join(', ') || 'none'],
            ['Cards Appeared', (s.cards_appeared_this_turn || []).map(id => cardName(id)).join(', ') || 'none'],
            ['Areas Placed', (s.areas_placed_this_turn || []).join(', ') || 'none'],
            ['Recently Moved', (s.recently_moved_cards || []).map(id => cardName(id)).join(', ') || 'none'],
            ['Recently From', s.recently_moved_from_zone || 'none'],
            ['Last Move', s.last_area_move_card_id != null ? `${cardName(s.last_area_move_card_id)} (by ${s.last_area_move_by_player || '?'})` : 'none'],
            ['Energy By Effect', bool(s.last_energy_placed_by_effect)],
        ];
        c.appendChild(section('Card Movement', grid(ctLines)));

        // Flags
        const flagLines = [
            ['Position Changed', bool(s.position_change_occurred_this_turn)],
            ['Formation Changed', bool(s.formation_change_occurred_this_turn)],
            ['Opponent Live Success', bool(s.opponent_live_success_this_turn)],
            ['Opponent No Excess', bool(s.opponent_live_no_excess_heart_this_turn)],
            ['Self No Excess', bool(s.self_no_excess_heart_this_turn)],
            ['Opponent Surplus', s.opponent_live_surplus_count ?? 0],
            ['Self Surplus', s.self_live_surplus_count ?? 0],
            ['Live Success Triggered', bool(s.live_success_triggered_this_turn)],
            ['Live Surplus Ready', bool(s.live_surplus_ready_this_turn)],
            ['Live Being Performed', bool(s.live_being_performed)],
            ['Deck Refresh', bool(s.deck_refresh_pending)],
            ['Loop Detected', bool(s.loop_detected)],
            ['Draw State', bool(s.draw_state)],
            ['Opponent Choice Declined', bool(s.opponent_choice_declined)],
            ['Cheer Checks', `${s.cheer_checks_done ?? 0}/${s.cheer_checks_required ?? 0}`],
            ['Cheer Completed', bool(s.cheer_check_completed)],
            ['Live Cheer Count', s.live_cheer_count ?? 0],
        ];
        c.appendChild(section('Flags & Live', grid(flagLines)));

        // Cheer blade hearts
        const cheerLines = [
            ['P1 Cheer Blade Hearts', s.player1_cheer_blade_heart_count ?? 0],
            ['P2 Cheer Blade Hearts', s.player2_cheer_blade_heart_count ?? 0],
        ];
        c.appendChild(section('Cheer Stats', grid(cheerLines)));

        // Resolution zone
        const resCards = s.resolution_zone_cards || [];
        if (resCards.length > 0) {
            c.appendChild(section('Resolution Zone',
                `<div style="display:flex;flex-wrap:wrap;gap:4px;">${resCards.map(id => chip(cardName(id))).join('')}</div>`));
        }

        // Mulligan
        if (s.mulligan_selected_indices && s.mulligan_selected_indices.length > 0) {
            c.appendChild(section('Mulligan Selected', `<div class="gs-track-item">${s.mulligan_selected_indices.join(', ')}</div>`));
        }
    },

    // ─── Player Tab ──────────────────────────────────────────
    renderPlayerTab: (s) => {
        const c = document.getElementById('gs-tab-player');
        if (!c) return;
        c.innerHTML = '';
        if (!s.player1 && !s.player2) { c.textContent = 'No player data'; return; }

        const cols = document.createElement('div');
        cols.className = 'gs-player-columns';

        [s.player1, s.player2].forEach((p, idx) => {
            if (!p) return;
            const panel = document.createElement('div');
            panel.className = `gs-player-panel p${idx}`;
            const isMe = idx === State.perspectivePlayer;
            const title = document.createElement('div');
            title.className = 'gs-player-title';
            title.innerHTML = `<span>${isMe ? 'You' : 'Opponent'} ${esc(p.id || `P${idx + 1}`)}${p.is_first_attacker ? ' ★' : ''}</span> <span class="gs-badge">${p.main_deck_count ?? '?'} deck</span>`;
            panel.appendChild(title);

            // Stats grid — includes all values conditions may check
            const tLines = [
                ['Cost Reduction', p.cost_reduction ?? 0],
                ['Prevent Baton', bool(!!(p.prevent_baton_touch || p.prevent_baton))],
                ['Debut Count', p.debut_count_this_turn ?? 0],
                ['Areas Locked', (p.areas_locked_this_turn || []).join(', ') || 'none'],
                ['Energy Active', `${p.energy_active_count ?? 0}/${(p.energy?.cards || []).length}`],
                ['Blade Buffs', (p.blade_buffs || []).join(', ') || 'none'],
                ['Heart Buffs', (p.heart_buffs || []).map(h => `[${h.join(',')}]`).join(' ') || 'none'],
            ];
            if (p.total_hearts) {
                p.total_hearts.forEach((h, ci) => { if (h > 0) tLines.push([`Hearts ${HEART_NAMES[ci] || ci}`, h]); });
            }
            if (p.live_card_scores) {
                Object.entries(p.live_card_scores).forEach(([no, sc]) => tLines.push([`Score ${no}`, sc]));
            }
            if (p.score_modifiers) {
                Object.entries(p.score_modifiers).forEach(([cid, v]) => tLines.push([`Score mod #${cid}`, v]));
            }
            // Stage hearts
            if (p.stage_hearts) {
                Object.entries(p.stage_hearts).forEach(([col, h]) => { if (h > 0) tLines.push([`Stage ${col}`, h]); });
            }
            // Global game state values indexed by this player
            const playerId = p.id || (idx === 0 ? s.player1?.id : s.player2?.id);
            const isP1 = playerId === s.player1?.id;
            if (s.live_success_total_score != null) tLines.push(['Last Live Total Score', s.live_success_total_score]);
            if (s.last_cost_discard_count != null) tLines.push(['Last Cost Discard', s.last_cost_discard_count]);
            if (s.last_cost_energy_count != null) tLines.push(['Last Cost Energy', s.last_cost_energy_count]);
            tLines.push(['Self No Excess Heart', bool(s.self_no_excess_heart_this_turn)]);
            const mySurplus = isP1 ? s.self_live_surplus_count : s.opponent_live_surplus_count;
            const oppSurplus = isP1 ? s.opponent_live_surplus_count : s.self_live_surplus_count;
            tLines.push(['My Surplus Count', mySurplus ?? 0]);
            tLines.push(['Opp Surplus Count', oppSurplus ?? 0]);
            tLines.push(['Live Success Triggered', bool(s.live_success_triggered_this_turn)]);
            tLines.push(['Surplus Ready', bool(s.live_surplus_ready_this_turn)]);
            tLines.push(['Cheer Checks', `${s.cheer_checks_done ?? 0}/${s.cheer_checks_required ?? 0}`]);
            panel.insertAdjacentHTML('beforeend', grid(tLines));
            panel.appendChild(document.createElement('hr'));

            // Stage cards with per-card gained abilities
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
                    const slotDiv = document.createElement('div');
                    if (!card) {
                        slotDiv.innerHTML = `<div class="gs-card-slot"><span class="card-sub">${label}: empty</span></div>`;
                    } else {
                        slotDiv.innerHTML = renderCardSlot(card, label, s);
                        // Append undercards
                        if (under.length > 0) {
                            const underDiv = document.createElement('div');
                            underDiv.style.cssText = 'margin-top:2px;font-size:0.6rem;color:var(--text-muted);';
                            underDiv.textContent = `Under: ${under.map(u => u.name || `#${u.id}`).join(', ')}`;
                            slotDiv.appendChild(underDiv);
                        }
                        // Per-card gained abilities
                        const gainedOnThis = (p.gained_abilities || []).filter(a => a.startsWith(`Card#${card.id}:`));
                        if (gainedOnThis.length > 0) {
                            const gDiv = document.createElement('div');
                            gDiv.style.cssText = 'margin-top:2px;font-size:0.6rem;';
                            gDiv.innerHTML = `<span style="color:var(--accent-gold);">Abilities gained:</span> ${gainedOnThis.map(a => a.replace(/^Card#\d+:\s*/, '')).join(', ')}`;
                            slotDiv.appendChild(gDiv);
                        }
                        // Ability text
                        if (card.ability_text) {
                            const aDiv = document.createElement('div');
                            aDiv.style.cssText = 'margin-top:2px;font-size:0.6rem;color:var(--text-muted);font-style:italic;overflow:hidden;text-overflow:ellipsis;max-height:2.4em;';
                            aDiv.textContent = card.ability_text;
                            slotDiv.appendChild(aDiv);
                        }
                    }
                    cardRow.appendChild(slotDiv);
                });
                panel.appendChild(cardRow);
            }

            // Restrictions
            if (p.active_restrictions && p.active_restrictions.length > 0) {
                const rDiv = document.createElement('div');
                rDiv.style.cssText = 'margin-top:6px;font-size:0.7rem;';
                rDiv.innerHTML = `<strong style="color:var(--accent-pink);">Restrictions:</strong> ${p.active_restrictions.join(', ')}`;
                panel.appendChild(rDiv);
            }

            // Exclusion zone count
            const excCount = p.exclusion_zone?.cards?.length || 0;
            if (excCount > 0) {
                const eDiv = document.createElement('div');
                eDiv.style.cssText = 'margin-top:4px;font-size:0.7rem;color:var(--text-muted);';
                eDiv.textContent = `Exclusion zone: ${excCount} cards`;
                panel.appendChild(eDiv);
            }

            cols.appendChild(panel);
        });
        c.appendChild(cols);
    },

    // ─── Zones Tab ───────────────────────────────────────────
    renderZonesTab: (s) => {
        const c = document.getElementById('gs-tab-zones');
        if (!c) return;
        c.innerHTML = '';
        if (!s.player1 && !s.player2) { c.textContent = 'No zone data'; return; }

        [s.player1, s.player2].forEach((p, idx) => {
            if (!p) return;
            const isMe = idx === State.perspectivePlayer;
            const label = `${isMe ? 'You' : 'Opponent'} (${p.id || `P${idx + 1}`})`;

            const zoneRow = document.createElement('div');
            zoneRow.className = 'gs-zone-row';

            const mkZone = (name, cards, extra) => {
                const box = document.createElement('div');
                box.className = 'gs-zone-box';
                box.innerHTML = `<div class="zone-name"><span>${esc(name)}</span><span>${extra ?? (cards ? cards.length : 0)}</span></div>`;
                if (cards && cards.length > 0) {
                    const chipsDiv = document.createElement('div');
                    chipsDiv.className = 'zone-cards';
                    cards.forEach(c => {
                        const t = c.type || 'Member';
                        const chipType = cardTypeClass(t);
                        const cost = c.cost != null ? ` (cost ${c.cost})` : '';
                        const orient = c.orientation === 'Wait' ? ' ⏸' : c.orientation === 'Active' ? ' ▶' : '';
                        chip(c.name || `#${c.id}`, chipType);
                        const sp = document.createElement('span');
                        sp.className = `zone-card-chip ${chipType}`;
                        sp.textContent = `${c.name || `#${c.id}`}${orient}${cost}`;
                        sp.title = `${t} · ID: ${c.id}`;
                        chipsDiv.appendChild(sp);
                    });
                    box.appendChild(chipsDiv);
                }
                zoneRow.appendChild(box);
            };

            mkZone('Main Deck', [], `${p.main_deck_count ?? 0} cards`);
            mkZone('Energy Deck', [], `${p.energy_deck_count ?? 0} cards`);
            // Energy with active/wait split
            const activeE = (p.energy?.cards || []).filter(c => c.orientation === 'Active');
            const waitE = (p.energy?.cards || []).filter(c => c.orientation !== 'Active');
            mkZone('Energy (Active)', activeE, `${activeE.length} active`);
            mkZone('Energy (Wait)', waitE, `${waitE.length} wait`);
            mkZone('Hand', p.hand?.cards || []);
            mkZone('Waitroom', p.waitroom?.cards || []);
            mkZone('Live Zone', p.live_zone?.cards || []);
            mkZone('Success Zone', p.success_live_card_zone?.cards || []);
            mkZone('Exclusion', p.exclusion_zone?.cards || []);

            c.appendChild(section(label, zoneRow));
        });

        // Global zones
        const globalSections = [];
        if (s.looked_cards && s.looked_cards.cards && s.looked_cards.cards.length > 0) {
            const row = document.createElement('div'); row.className = 'gs-zone-row';
            const box = document.createElement('div'); box.className = 'gs-zone-box';
            box.innerHTML = `<div class="zone-name"><span>Looked / Revealed</span><span>${s.looked_cards.cards.length}</span></div>`;
            const chips = document.createElement('div'); chips.className = 'zone-cards';
            s.looked_cards.cards.forEach(c => {
                const sp = document.createElement('span'); sp.className = 'zone-card-chip';
                sp.textContent = c.name || `#${c.id}`; chips.appendChild(sp);
            });
            box.appendChild(chips); row.appendChild(box);
            globalSections.push(section('Global Zones', row));
        }
        // Revealed cost cards
        if (s.revealed_cost_cards && s.revealed_cost_cards.length > 0) {
            globalSections.push(section('Revealed Cost Cards',
                `<div style="display:flex;flex-wrap:wrap;gap:4px;">${s.revealed_cost_cards.map(id => chip(cardName(id))).join('')}</div>`));
        }
        // Cheer revealed cards
        if (s.player1_cheer_revealed_cards && s.player1_cheer_revealed_cards.length > 0) {
            globalSections.push(section('P1 Cheer Revealed',
                `<div style="display:flex;flex-wrap:wrap;gap:4px;">${s.player1_cheer_revealed_cards.map(id => chip(cardName(id))).join('')}</div>`));
        }
        if (s.player2_cheer_revealed_cards && s.player2_cheer_revealed_cards.length > 0) {
            globalSections.push(section('P2 Cheer Revealed',
                `<div style="display:flex;flex-wrap:wrap;gap:4px;">${s.player2_cheer_revealed_cards.map(id => chip(cardName(id))).join('')}</div>`));
        }
        globalSections.forEach(el => c.appendChild(el));
    },

    // ─── Tracking Tab ────────────────────────────────────────
    renderTrackingTab: (s) => {
        const c = document.getElementById('gs-tab-tracking');
        if (!c) return;
        c.innerHTML = '';

        const gridDiv = document.createElement('div');
        gridDiv.className = 'gs-tracking-grid';

        // Ability Queue
        const entries = s.ability_queue_entries || [];
        let qHtml = `<div class="gs-track-item"><b>State:</b> ${esc(s.ability_queue_state || 'Idle')}, idx=${s.ability_queue_current_index ?? 0}</div>`;
        if (entries.length > 0) {
            entries.forEach((e, i) => {
                const active = i === (s.ability_queue_current_index ?? -1);
                qHtml += `<div class="gs-track-item" style="${active ? 'background:rgba(245,158,11,0.1);border-left:2px solid var(--accent-gold);' : ''}padding:2px 4px;margin:2px 0;">
                    <div class="track-kv"><span>#${i} ${esc(e.card_no)}</span><span class="tv">${esc(e.player_id)} · ${esc(e.trigger_type)}</span></div>
                    <div style="font-size:0.65rem;color:var(--text-dim);">${esc(e.ability_text || '(no text)')}</div>
                    <div style="font-size:0.6rem;">${['completed','cost_paid','effect_started'].map(f => `${f}:${e[f] ? '✓' : '✗'}`).join(' · ')}${e.choice_player_id ? ` · choice: ${e.choice_player_id}` : ''}</div>
                </div>`;
            });
        } else {
            qHtml += '<div class="gs-track-item">(empty)</div>';
        }
        gridDiv.insertAdjacentHTML('beforeend', mkBox('Ability Queue', qHtml));

        // Debut triggers
        const debuts = s.debut_ability_triggers || [];
        gridDiv.insertAdjacentHTML('beforeend', mkBox('Debut Triggers',
            debuts.length > 0 ? debuts.map(d => `<div class="gs-track-item">${esc(d.ability_key)} → ${esc(cardName(d.card_id))}</div>`).join('') : '<div class="gs-track-item">none</div>'));

        // Turn-limited abilities
        const tla = s.turn_limited_abilities_used || [];
        const tla2 = s.turn2_abilities_played || {};
        let tlaHtml = '';
        if (tla.length > 0) tlaHtml += `<div class="gs-track-item">Turn-limited: ${tla.join(', ')}</div>`;
        if (Object.keys(tla2).length > 0) tlaHtml += trackKV(Object.entries(tla2).map(([k, v]) => [`Turn2: ${k}`, v]));
        if (!tlaHtml) tlaHtml = '<div class="gs-track-item">none</div>';
        gridDiv.insertAdjacentHTML('beforeend', mkBox('Turn-Limited Abilities', tlaHtml));

        // Turn1 abilities played
        const t1ap = s.turn1_abilities_played || [];
        gridDiv.insertAdjacentHTML('beforeend', mkBox('Turn1 Abilities',
            t1ap.length > 0 ? t1ap.map(a => `<div class="gs-track-item">${esc(a)}</div>`).join('') : '<div class="gs-track-item">none</div>'));

        // Turn limit usage
        const tlu = s.turn_limit_usage || {};
        gridDiv.insertAdjacentHTML('beforeend', mkBox('Turn Limit Usage',
            Object.keys(tlu).length > 0 ? trackKV(Object.entries(tlu)) : '<div class="gs-track-item">none</div>'));

        // Auto ability triggers
        const aatc = s.auto_ability_trigger_counts || {};
        gridDiv.insertAdjacentHTML('beforeend', mkBox('Auto Ability Triggers',
            Object.keys(aatc).length > 0 ? trackKV(Object.entries(aatc)) : '<div class="gs-track-item">none</div>'));

        // Prohibitions
        const proh = s.prohibition_effects || [];
        const dproh = s.delayed_prohibition_effects || [];
        let prohHtml = proh.map(p => `<div class="gs-track-item">${esc(p)}</div>`).join('');
        if (dproh.length > 0) prohHtml += dproh.map(p => `<div class="gs-track-item">[delayed] ${esc(p)}</div>`).join('');
        if (!prohHtml) prohHtml = '<div class="gs-track-item">none</div>';
        gridDiv.insertAdjacentHTML('beforeend', mkBox('Prohibition Effects', prohHtml));

        // Cannot activate
        const ca = s.cannot_activate_members || [];
        const cca = s.constant_cannot_activate_members || [];
        let caHtml = ca.map(p => `<div class="gs-track-item">${esc(p)}</div>`).join('');
        if (cca.length > 0) caHtml += cca.map(p => `<div class="gs-track-item">[constant] ${esc(p)}</div>`).join('');
        if (!caHtml) caHtml = '<div class="gs-track-item">none</div>';
        gridDiv.insertAdjacentHTML('beforeend', mkBox('Cannot Activate Members', caHtml));

        // Negated
        const neg = s.negated_abilities || [];
        gridDiv.insertAdjacentHTML('beforeend', mkBox('Negated Abilities',
            neg.length > 0 ? neg.map(id => `<div class="gs-track-item">#${id} (${esc(cardName(id))})</div>`).join('') : '<div class="gs-track-item">none</div>'));

        // Non-stackable
        const ns = s.non_stackable_effects || [];
        gridDiv.insertAdjacentHTML('beforeend', mkBox('Non-Stackable Effects',
            ns.length > 0 ? ns.map(e => `<div class="gs-track-item">${esc(e)}</div>`).join('') : '<div class="gs-track-item">none</div>'));

        // Temporary effects
        const te = s.temporary_effects || [];
        let teHtml = te.length > 0 ? te.map(e => `
            <div class="gs-track-item" style="border-bottom:1px solid rgba(255,255,255,0.05);padding:2px 0;">
                <div class="track-kv"><span>Type</span><span class="tv">${esc(e.effect_type)}</span></div>
                <div class="track-kv"><span>Duration</span><span class="tv">${esc(e.duration)}</span></div>
                <div class="track-kv"><span>Turn</span><span class="tv">${e.created_turn ?? '?'}</span></div>
                <div class="track-kv"><span>Target</span><span class="tv">${esc(e.target_player_id || '?')}</span></div>
                <div style="color:var(--text-dim);font-size:0.65rem;white-space:normal;word-break:break-all;">${esc(e.description || '')}</div>
            </div>`).join('') : '<div class="gs-track-item">none</div>';
        gridDiv.insertAdjacentHTML('beforeend', mkBox('Temporary Effects', teHtml));

        // Replacement effects
        const re = s.replacement_effects || [];
        let reHtml = re.length > 0 ? re.map(e => `
            <div class="gs-track-item" style="border-bottom:1px solid rgba(255,255,255,0.05);padding:2px 0;">
                <div class="track-kv"><span>Card</span><span class="tv">${e.card_id != null ? esc(cardName(e.card_id)) : '?'}</span></div>
                <div class="track-kv"><span>Player</span><span class="tv">${esc(e.player_id || '?')}</span></div>
                <div class="track-kv"><span>Event</span><span class="tv">${esc(e.original_event || '?')}</span></div>
                <div class="track-kv"><span>Choice</span><span class="tv">${e.is_choice_based ? 'Yes' : 'No'}</span></div>
            </div>`).join('') : '<div class="gs-track-item">none</div>';
        gridDiv.insertAdjacentHTML('beforeend', mkBox('Replacement Effects', reHtml));

        // Ability applications
        const apps = s.ability_applications || [];
        let appHtml = apps.length > 0 ? apps.map(a => `
            <div class="gs-track-item" style="border-bottom:1px solid rgba(255,255,255,0.05);padding:1px 0;">
                <div class="track-kv"><span>Source</span><span class="tv">${esc(cardName(a.source_card_id))}</span></div>
                <div class="track-kv"><span>Effect</span><span class="tv">${esc(a.effect_type)}</span></div>
                <div class="track-kv"><span>Target</span><span class="tv">${esc(cardName(a.target_card_id))}</span></div>
                <div class="track-kv"><span>Amount</span><span class="tv">${a.amount ?? 0}</span></div>
            </div>`).join('') : '<div class="gs-track-item">none</div>';
        gridDiv.insertAdjacentHTML('beforeend', mkBox('Ability Applications', appHtml));

        // Card instance mapping
        const cim = s.card_instance_mapping || {};
        gridDiv.insertAdjacentHTML('beforeend', mkBox('Card Instance Mapping',
            Object.keys(cim).length > 0 ? trackKV(Object.entries(cim).map(([id, inst]) => [`#${id} (${cardName(parseInt(id))})`, inst])) : '<div class="gs-track-item">none</div>'));

        // Constant ability statuses
        const cas = s.constant_ability_statuses || [];
        if (cas.length > 0) {
            gridDiv.insertAdjacentHTML('beforeend', mkBox('Constant Ability Statuses',
                cas.map(a => `<div class="gs-track-item">${esc(a.card_no || '?')} — ${esc(a.ability_text || '')} → ${a.enabled ? '✓ ENABLED' : '✗ DISABLED'}</div>`).join('')));
        }

        // Live owned hearts
        const loh = s.live_owned_hearts || {};
        let lohHtml = Object.keys(loh).length > 0
            ? Object.entries(loh).map(([pid, pairs]) =>
                `<div class="gs-track-item"><b>${esc(pid)}</b>: ${pairs.map(([c, v]) => `${c}:${v}`).join(', ')}</div>`).join('')
            : '<div class="gs-track-item">none</div>';
        gridDiv.insertAdjacentHTML('beforeend', mkBox('Live Owned Hearts', lohHtml));

        // Card movement tracking
        const moved = s.recently_moved_cards || [];
        if (moved.length > 0) {
            gridDiv.insertAdjacentHTML('beforeend', mkBox('Recent Card Movement', trackKV([
                ['From Zone', s.recently_moved_from_zone || '?'],
                ['Last Vacated Stage', s.last_vacated_stage_area || '?'],
                ['Cards', moved.map(c => cardName(c)).join(', ')],
            ])));
        }

        // Pending success replacement
        const psr = [];
        if (s.pending_success_replacement_card_id != null) psr.push(['Card', cardName(s.pending_success_replacement_card_id)]);
        if (s.pending_success_replacement_player_id) psr.push(['Player', s.pending_success_replacement_player_id]);
        if (psr.length > 0) gridDiv.insertAdjacentHTML('beforeend', mkBox('Pending Success Replacement', trackKV(psr)));

        // Card instance counter + effect creation counter
        gridDiv.insertAdjacentHTML('beforeend', mkBox('Counters', trackKV([
            ['Card Instance Counter', s.card_instance_counter ?? 0],
            ['Effect Creation Counter', s.effect_creation_counter ?? 0],
            ['Last State Change Wait→Active', s.last_state_change_wait_to_active_count ?? 0],
        ])));

        // --- Modifier Constants ---
        const constBlade = s.constant_blade_bonuses || {};
        gridDiv.insertAdjacentHTML('beforeend', mkBox('Constant Blade Bonuses',
            Object.keys(constBlade).length > 0 ? trackKV(Object.entries(constBlade).map(([id, v]) => [`#${id} (${cardName(parseInt(id))})`, v])) : '<div class="gs-track-item">none</div>'));

        const constCost = s.constant_cost_bonuses || {};
        const constCostTotal = Object.values(constCost).reduce((s, v) => s + v, 0);
        gridDiv.insertAdjacentHTML('beforeend', mkBox('Constant Cost Bonuses',
            `<div class="gs-track-item"><b>Total cost bonus:</b> ${constCostTotal}</div>` + 
            (Object.keys(constCost).length > 0 ? trackKV(Object.entries(constCost).map(([id, v]) => [`#${id} (${cardName(parseInt(id))})`, v])) : '')));

        const constScore = s.constant_score_bonuses || {};
        gridDiv.insertAdjacentHTML('beforeend', mkBox('Constant Score Bonuses',
            Object.keys(constScore).length > 0 ? trackKV(Object.entries(constScore).map(([id, v]) => [`#${id} (${cardName(parseInt(id))})`, v])) : '<div class="gs-track-item">none</div>'));

        const constHeart = s.constant_heart_bonuses || {};
        let chHtml = Object.keys(constHeart).length > 0
            ? Object.entries(constHeart).map(([id, cols]) =>
                `<div class="gs-track-item">#${id} (${cardName(parseInt(id))}): ${Object.entries(cols).map(([c, v]) => `${c}:${v}`).join(', ')}</div>`).join('')
            : '<div class="gs-track-item">none</div>';
        gridDiv.insertAdjacentHTML('beforeend', mkBox('Constant Heart Bonuses', chHtml));

        const cgnh = s.constant_global_need_heart || [];
        gridDiv.insertAdjacentHTML('beforeend', mkBox('Constant Global Need Heart',
            cgnh.length > 0 ? cgnh.map(([cid, sname, v]) => `<div class="gs-track-item">#${cid} (${cardName(parseInt(cid))}) · ${esc(sname)}: ${v}</div>`).join('') : '<div class="gs-track-item">none</div>'));

        const css = s.constant_score_sources || [];
        gridDiv.insertAdjacentHTML('beforeend', mkBox('Constant Score Sources',
            css.length > 0 ? css.map(([cid, sname, v]) => `<div class="gs-track-item">#${cid} (${cardName(parseInt(cid))}) · ${esc(sname)}: ${v}</div>`).join('') : '<div class="gs-track-item">none</div>'));

        const btMod = s.blade_type_modifiers || {};
        gridDiv.insertAdjacentHTML('beforeend', mkBox('Blade Type Modifiers',
            Object.keys(btMod).length > 0 ? trackKV(Object.entries(btMod).map(([id, v]) => [`#${id} (${cardName(parseInt(id))})`, v])) : '<div class="gs-track-item">none</div>'));

        const ho = s.heart_override || {};
        let hoHtml = Object.keys(ho).length > 0
            ? Object.entries(ho).map(([id, arr]) => `<div class="gs-track-item">#${id} (${cardName(parseInt(id))}): ${arr[0]} × ${arr[1]}</div>`).join('')
            : '<div class="gs-track-item">none</div>';
        gridDiv.insertAdjacentHTML('beforeend', mkBox('Heart Override', hoHtml));

        // Heart color decision phase
        if (s.heart_color_decision_phase) {
            gridDiv.insertAdjacentHTML('beforeend', mkBox('Heart Color Phase',
                `<div class="gs-track-item">${esc(s.heart_color_decision_phase)}</div>`));
        }

        // Opponent choice declined
        if (s.opponent_choice_declined) {
            gridDiv.insertAdjacentHTML('beforeend', mkBox('Opponent Choice',
                `<div class="gs-track-item" style="color:var(--accent-pink);">Opponent choice was declined</div>`));
        }

        const dca = s.delayed_cannot_active || {};
        gridDiv.insertAdjacentHTML('beforeend', mkBox('Delayed Cannot Activate',
            Object.keys(dca).length > 0 ? trackKV(Object.entries(dca).map(([id, v]) => [`#${id} (${cardName(parseInt(id))})`, v])) : '<div class="gs-track-item">none</div>'));

        gridDiv.insertAdjacentHTML('beforeend', mkBox('Cost Payment', trackKV([
            ['Last Cost Discard Count', s.last_cost_discard_count ?? 0],
            ['Last Cost Energy Count', s.last_cost_energy_count ?? 0],
        ])));

        c.appendChild(gridDiv);
    },
};
