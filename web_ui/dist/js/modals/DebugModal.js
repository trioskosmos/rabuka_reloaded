import { State } from '../state.js';
import { Network } from '../network.js';

const buildReverseLookup = (source) => Object.fromEntries(
    Object.entries(source)
        .filter(([key, value]) => typeof value === 'number' && !/^[-]?\d+$/.test(key))
        .map(([key, value]) => [value, key])
);

import {
    ChoiceTypes,
    ConditionTypes,
    CostTypes,
    ExtraConstants,
    Opcodes,
    Phases,
    TargetType,
    TriggerType,
} from '../generated_constants.js';

const TRIGGER_NAMES = buildReverseLookup(TriggerType);
const CONDITION_NAMES = buildReverseLookup(ConditionTypes);
const EFFECT_NAMES = buildReverseLookup(Opcodes);
const COST_NAMES = buildReverseLookup(CostTypes);
const CHOICE_NAMES = buildReverseLookup(ChoiceTypes);
const TARGET_NAMES = buildReverseLookup(TargetType);
const PHASE_NAMES = buildReverseLookup(Phases);

const pickBits = (names) => names
    .filter((name) => Number.isSafeInteger(ExtraConstants[name]))
    .map((name) => ({ name, value: ExtraConstants[name] }));

const ABILITY_FLAG_BITS = pickBits([
    'FLAG_DRAW',
    'FLAG_SEARCH',
    'FLAG_RECOVER',
    'FLAG_BUFF',
    'FLAG_CHARGE',
    'FLAG_TEMPO',
    'FLAG_REDUCE',
    'FLAG_BOOST',
    'FLAG_TRANSFORM',
    'FLAG_WIN_COND',
    'FLAG_MOVE',
    'FLAG_TAP',
]);

const COST_FLAG_BITS = pickBits(['COST_FLAG_DISCARD', 'COST_FLAG_TAP']);
const CHOICE_FLAG_BITS = pickBits(['CHOICE_FLAG_LOOK', 'CHOICE_FLAG_DISCARD', 'CHOICE_FLAG_MODE', 'CHOICE_FLAG_COLOR', 'CHOICE_FLAG_ORDER']);
const SYNERGY_FLAG_BITS = pickBits(['SYN_FLAG_GROUP', 'SYN_FLAG_COLOR', 'SYN_FLAG_BATON', 'SYN_FLAG_CENTER', 'SYN_FLAG_LIFE_LEAD']);
const FILTER_FLAG_BITS = pickBits([
    'FILTER_TYPE_MEMBER',
    'FILTER_TYPE_LIVE',
    'FILTER_GROUP_ENABLE',
    'FILTER_TAPPED',
    'FILTER_HAS_BLADE_HEART',
    'FILTER_NOT_HAS_BLADE_HEART',
    'FILTER_UNIQUE_NAMES',
    'FILTER_UNIT_ENABLE',
    'FILTER_COST_ENABLE',
    'FILTER_COST_LE',
    'FILTER_BLADE_FILTER_FLAG',
    'FILTER_ANY_STAGE',
    'FILTER_OPPONENT',
    'FILTER_REVEALED_CONTEXT',
    'FILTER_TOTAL_COST',
    'FILTER_COST_TYPE_FLAG',
    'FILTER_IS_OPTIONAL',
]);

const escapeHtml = (value) => String(value ?? '')
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
    .replace(/'/g, '&#39;');

const summarizeObject = (value) => {
    if (value === null) return 'null';
    if (value === undefined) return 'undefined';
    if (typeof value === 'string') return value;
    if (typeof value === 'number' || typeof value === 'boolean') return String(value);
    return JSON.stringify(value);
};

const formatPacked = (value) => {
    if (value === undefined || value === null) return '0';
    if (typeof value !== 'number' || !Number.isFinite(value)) return summarizeObject(value);
    return `0x${value.toString(16)}`;
};

const decodeBitmask = (value, bits) => {
    if (!Number.isSafeInteger(value) || value <= 0) return [];
    return bits
        .filter((bit) => bit.value !== 0 && (value & bit.value) === bit.value)
        .map((bit) => bit.name);
};

const renderChips = (items, accent = '#7dd3fc') => {
    if (!items || items.length === 0) {
        return '<span style="opacity:0.45; font-size:10px;">none</span>';
    }
    return items.map((item) => `
        <span class="debug-badge" style="--accent: ${accent}">${escapeHtml(item)}</span>
    `).join('');
};

const zoneDefinitions = (player) => [
    { key: 'stage', label: 'Stage', cards: player?.stage || [] },
    { key: 'live', label: 'Live', cards: player?.live_zone || [] },
    { key: 'hand', label: 'Hand', cards: player?.hand || [] },
    { key: 'success', label: 'Success', cards: player?.success_live_card_zone || [] },
    { key: 'energy', label: 'Energy', cards: player?.energy || [] },
    { key: 'discard', label: 'Discard', cards: player?.discard || [] },
    { key: 'looked', label: 'Looked', cards: player?.looked_cards || [] },
];

const extractScalarEntries = (value) => Object.entries(value || {})
    .filter(([, item]) => item === null || ['string', 'number', 'boolean'].includes(typeof item));

const describeNumber = (key, value, itemType) => {
    if (key === 'trigger') return `${TRIGGER_NAMES[value] || value} (${value})`;
    if (key === 'condition_type') return `${CONDITION_NAMES[value] || value} (${value})`;
    if (key === 'effect_type') return `${EFFECT_NAMES[value] || value} (${value})`;
    if (key === 'cost_type') return `${COST_NAMES[value] || value} (${value})`;
    if (key === 'choice_type' || key === 'choice') return `${CHOICE_NAMES[value] || value} (${value})`;
    if (key === 'target') return `${TARGET_NAMES[value] || value} (${value})`;
    if (key === 'phase') return `${PHASE_NAMES[value] || value} (${value})`;
    if (key.includes('flags') || key.includes('filter') || key === 'attr') {
        const bits = key.startsWith('choice')
            ? CHOICE_FLAG_BITS
            : (itemType === 'cost' && key.includes('flag'))
                ? COST_FLAG_BITS
                : FILTER_FLAG_BITS;
        const decoded = decodeBitmask(value, bits);
        return decoded.length > 0 ? `${value} [${decoded.join(', ')}]` : String(value);
    }
    return String(value);
};

const renderScalarCell = (label, value) => {
    const display = typeof value === 'number' ? value : String(value);
    return `
    <div class="debug-scalar-cell">
        <div class="debug-scalar-label">${escapeHtml(label)}</div>
        <div class="debug-scalar-value">${escapeHtml(display)}</div>
    </div>
`;
};

const renderStatusBanner = (status) => {
    if (!status?.message) return '';
    const bannerClass = status.kind === 'error' ? 'debug-status-error' : 'debug-status-success';
    return `
        <div class="debug-status-banner ${bannerClass}">
            ${escapeHtml(status.message)}
        </div>
    `;
};

export const DebugModal = {
    _filters: {
        selectedPlayer: 0,
        selectedZone: 'all',
        abilitySearch: '',
    },

    _status: null,
    _conditions: null,
    _conditionError: null,

    init: () => {},

    openDebugModal: async () => {
        const modal = document.getElementById('debug-modal');
        if (!modal) return;

        modal.style.display = 'flex';
        await DebugModal.renderAll();
    },

    closeDebugModal: () => {
        const modal = document.getElementById('debug-modal');
        if (modal) modal.style.display = 'none';
    },

    _setStatus: (kind, message) => {
        DebugModal._status = message ? { kind, message } : null;
    },

    _clearStatus: () => {
        DebugModal._status = null;
    },

    renderAll: async () => {
        if (State.roomCode) await Network.fetchState();
        await DebugModal._fetchConditions();

        if (!State.data) {
            const container = document.getElementById('debug-inspector-content');
            if (container) {
                container.innerHTML = '<div style="padding:24px; opacity:0.6; text-align:center; font-size:12px;">Waiting for game state...</div>';
            }
            return;
        }

        DebugModal.renderInspector();
    },

    _fetchConditions: async () => {
        DebugModal._conditions = null;
        DebugModal._conditionError = null;
        try {
            const res = await fetch('/api/debug/conditions', {
                headers: State.roomCode ? { 'X-Room-ID': State.roomCode } : {}
            });
            const data = await res.json();
            if (data.success && Array.isArray(data.conditions)) {
                DebugModal._conditions = data.conditions;
            } else {
                DebugModal._conditionError = data.error || 'Unknown error';
            }
        } catch (e) {
            DebugModal._conditionError = e.message;
        }
    },

    renderInspector: () => {
        const container = document.getElementById('debug-inspector-content');
        if (!container || !State.data) return;

        const players = State.data.players || [];
        const playerIdx = DebugModal._filters.selectedPlayer;
        const currentPlayer = players[playerIdx] || players[0] || null;
        const zone = DebugModal._filters.selectedZone;
        const search = DebugModal._filters.abilitySearch.trim();
        const visibleCards = DebugModal._collectVisibleCards(currentPlayer, zone)
            .filter((entry) => entry.card && entry.card.id !== -1 && entry.card.id !== -2)
            .filter((entry) => DebugModal._matchesSearch(entry, search));

        container.innerHTML = `
            <div style="display:flex; flex-direction:column; gap:10px;">
                ${renderStatusBanner(DebugModal._status)}
                <div style="display:grid; grid-template-columns:minmax(140px, 0.9fr) minmax(140px, 0.9fr) minmax(220px, 1.4fr); gap:8px;">
                    <div>
                        <label class="form-label-xs">Player</label>
                        <select onchange="DebugModal.onPlayerChange(this.value)" class="form-select form-select-sm">
                            ${players.map((player, index) => `<option value="${index}" ${index === playerIdx ? 'selected' : ''}>Player ${index + 1}${State.data.active_player === index ? ' [active]' : ''}</option>`).join('')}
                        </select>
                    </div>
                    <div>
                        <label class="form-label-xs">Zone</label>
                        <select onchange="DebugModal.onZoneChange(this.value)" class="form-select form-select-sm">
                            <option value="all" ${zone === 'all' ? 'selected' : ''}>All Zones</option>
                            <option value="stage" ${zone === 'stage' ? 'selected' : ''}>Stage</option>
                            <option value="live" ${zone === 'live' ? 'selected' : ''}>Live</option>
                            <option value="hand" ${zone === 'hand' ? 'selected' : ''}>Hand</option>
                            <option value="success" ${zone === 'success' ? 'selected' : ''}>Success</option>
                            <option value="energy" ${zone === 'energy' ? 'selected' : ''}>Energy</option>
                            <option value="discard" ${zone === 'discard' ? 'selected' : ''}>Discard</option>
                            <option value="looked" ${zone === 'looked' ? 'selected' : ''}>Looked</option>
                        </select>
                    </div>
                    <div>
                        <label class="form-label-xs">Search</label>
                        <input type="text" placeholder="card, trigger, condition, pseudocode" value="${escapeHtml(DebugModal._filters.abilitySearch)}" oninput="DebugModal.onSearchChange(this.value)" class="form-input form-input-sm">
                    </div>
                </div>

                ${DebugModal._renderSummaryCards(players, visibleCards)}
                ${currentPlayer ? DebugModal._renderZoneDiagnostics(currentPlayer) : ''}
                ${DebugModal._renderAbilityMatrix(visibleCards)}
                ${DebugModal._renderConditionTable()}

                <div style="display:flex; flex-direction:column; gap:10px;">
                    <strong style="font-size:12px;">Card Detail</strong>
                    ${visibleCards.length === 0
                        ? '<div style="opacity:0.55; text-align:center; padding:24px; font-size:11px; background:rgba(255,255,255,0.03); border:1px solid rgba(255,255,255,0.08); border-radius:8px;">No cards match the current zone/search filters.</div>'
                        : visibleCards.map((entry, index) => DebugModal._renderCardInspector(entry, index)).join('')}
                </div>
            </div>
        `;
    },

    _renderConditionTable: () => {
        const conditions = DebugModal._conditions;
        if (DebugModal._conditionError) {
            return `
                <div style="background:rgba(239,68,68,0.08); border:1px solid rgba(239,68,68,0.3); border-radius:8px; padding:12px;">
                    <strong style="font-size:11px; color:#ef4444;">Condition fetch error:</strong>
                    <div style="font-size:10px; margin-top:4px;">${escapeHtml(DebugModal._conditionError)}</div>
                </div>
            `;
        }
        if (!conditions) {
            return `
                <div style="background:rgba(255,255,255,0.03); border:1px solid rgba(255,255,255,0.08); border-radius:8px; padding:12px;">
                    <div style="font-size:11px; opacity:0.6;">Loading conditions...</div>
                </div>
            `;
        }
        if (conditions.length === 0) {
            return `
                <div style="background:rgba(255,255,255,0.03); border:1px solid rgba(255,255,255,0.08); border-radius:8px; padding:12px;">
                    <div style="font-size:11px; opacity:0.6;">No conditions found on any card.</div>
                </div>
            `;
        }

        const trueCount = conditions.filter(c => c.result).length;
        const falseCount = conditions.filter(c => !c.result).length;

        const rows = conditions.map((c, i) => {
            const resultClass = c.result ? 'condition-true' : 'condition-false';
            const resultLabel = c.result ? 'TRUE' : 'FALSE';
            return `
                <tr style="vertical-align:top; ${i % 2 === 0 ? 'background:rgba(255,255,255,0.015);' : ''}">
                    <td style="padding:6px 8px; border-bottom:1px solid rgba(255,255,255,0.06); font-size:10px;">
                        <span class="${resultClass}" style="display:inline-block; padding:1px 6px; border-radius:4px; font-weight:bold; font-size:10px; ${c.result ? 'background:rgba(34,197,94,0.2); color:#4ade80;' : 'background:rgba(239,68,68,0.15); color:#f87171;'}">${resultLabel}</span>
                    </td>
                    <td style="padding:6px 8px; border-bottom:1px solid rgba(255,255,255,0.06); font-size:10px; white-space:nowrap;">P${c.player + 1}</td>
                    <td style="padding:6px 8px; border-bottom:1px solid rgba(255,255,255,0.06); font-size:10px; white-space:nowrap;">${escapeHtml(c.zone)}</td>
                    <td style="padding:6px 8px; border-bottom:1px solid rgba(255,255,255,0.06); font-size:10px; max-width:160px; overflow:hidden; text-overflow:ellipsis; white-space:nowrap;" title="${escapeHtml(c.card_name)}">${escapeHtml(c.card_name)}</td>
                    <td style="padding:6px 8px; border-bottom:1px solid rgba(255,255,255,0.06); font-size:10px; white-space:nowrap;">${escapeHtml(c.condition_type || '(none)')}</td>
                    <td style="padding:6px 8px; border-bottom:1px solid rgba(255,255,255,0.06); font-size:10px; max-width:200px; overflow:hidden; text-overflow:ellipsis; white-space:nowrap;" title="${escapeHtml(c.condition_text)}">${escapeHtml(c.condition_text || '-')}</td>
                </tr>
            `;
        }).join('');

        return `
            <div style="background:rgba(255,255,255,0.03); border:1px solid rgba(255,255,255,0.08); border-radius:8px; padding:12px; display:flex; flex-direction:column; gap:10px;">
                <div style="display:flex; justify-content:space-between; align-items:center; gap:8px;">
                    <strong style="font-size:12px;">Condition Evaluation (${conditions.length} total)</strong>
                    <span style="font-size:10px;">
                        <span style="color:#4ade80;">${trueCount} true</span>
                        <span style="opacity:0.5;"> / </span>
                        <span style="color:#f87171;">${falseCount} false</span>
                    </span>
                </div>
                <div style="overflow:auto; border:1px solid rgba(255,255,255,0.06); border-radius:6px; max-height:400px;">
                    <table style="width:100%; border-collapse:collapse; min-width:800px; font-size:10px;">
                        <thead>
                            <tr style="background:rgba(15,23,42,0.95); text-transform:uppercase; letter-spacing:0.04em; position:sticky; top:0;">
                                <th style="padding:8px; text-align:left; border-bottom:1px solid rgba(255,255,255,0.08);">Result</th>
                                <th style="padding:8px; text-align:left; border-bottom:1px solid rgba(255,255,255,0.08);">P</th>
                                <th style="padding:8px; text-align:left; border-bottom:1px solid rgba(255,255,255,0.08);">Zone</th>
                                <th style="padding:8px; text-align:left; border-bottom:1px solid rgba(255,255,255,0.08);">Card</th>
                                <th style="padding:8px; text-align:left; border-bottom:1px solid rgba(255,255,255,0.08);">Type</th>
                                <th style="padding:8px; text-align:left; border-bottom:1px solid rgba(255,255,255,0.08);">Text</th>
                            </tr>
                        </thead>
                        <tbody>
                            ${rows}
                        </tbody>
                    </table>
                </div>
                <details>
                    <summary style="cursor:pointer; opacity:0.65; font-size:10px;">Raw JSON</summary>
                    <pre style="margin:6px 0 0 0; padding:8px; background:#05070d; border-radius:4px; font-size:9px; line-height:1.35; color:#dbeafe; white-space:pre-wrap; word-break:break-word; max-height:300px; overflow:auto;">${escapeHtml(JSON.stringify(conditions, null, 2))}</pre>
                </details>
            </div>
        `;
    },

    _renderSummaryCards: (players, visibleCards) => {
        const phaseName = PHASE_NAMES[State.data.phase] || String(State.data.phase ?? '?');

        return `
            <div style="display:grid; grid-template-columns:repeat(auto-fit, minmax(180px, 1fr)); gap:10px;">
                <div style="background:rgba(255,255,255,0.04); border:1px solid rgba(255,255,255,0.08); border-radius:8px; padding:12px; display:flex; flex-direction:column; gap:6px;">
                    <div style="font-size:10px; letter-spacing:0.08em; text-transform:uppercase; opacity:0.6;">State</div>
                    <div style="display:grid; grid-template-columns:repeat(2, minmax(60px, 1fr)); gap:8px; font-size:11px;">
                        ${renderScalarCell('turn', State.data.turn ?? '?')}
                        ${renderScalarCell('phase', phaseName)}
                        ${renderScalarCell('active', `P${(State.data.active_player ?? 0) + 1}`)}
                        ${renderScalarCell('visible cards', visibleCards.length)}
                    </div>
                </div>
                ${players.map((player, index) => `
                    <div style="background:${index === DebugModal._filters.selectedPlayer ? 'rgba(56,189,248,0.12)' : 'rgba(255,255,255,0.04)'}; border:1px solid ${index === DebugModal._filters.selectedPlayer ? 'rgba(56,189,248,0.4)' : 'rgba(255,255,255,0.08)'}; border-radius:8px; padding:12px; display:flex; flex-direction:column; gap:6px;">
                        <div style="display:flex; justify-content:space-between; gap:8px; align-items:center;">
                            <strong style="font-size:11px; color:${index === DebugModal._filters.selectedPlayer ? '#7dd3fc' : '#fff'};">Player ${index + 1}${State.data.active_player === index ? ' [active]' : ''}</strong>
                            <span style="font-size:10px; opacity:0.72;">Score ${escapeHtml(player?.success_live_card_zone?.cards?.length ?? 0)}</span>
                        </div>
                        <div style="display:grid; grid-template-columns:repeat(2, minmax(70px, 1fr)); gap:6px;">
                            ${zoneDefinitions(player).map((zone) => renderScalarCell(zone.label, zone.cards.length)).join('')}
                        </div>
                    </div>
                `).join('')}
            </div>
        `;
    },

    _renderPendingChoice: () => {
        if (!State.data?.pending_choice) return '';

        const pending = State.data.pending_choice;
        const choiceType = CHOICE_NAMES[pending.choice_type] || CHOICE_NAMES[pending.type] || 'PENDING_CHOICE';

        return `
            <div style="background:rgba(251,191,36,0.08); border-left:3px solid #fbbf24; padding:10px 12px; border-radius:6px; display:flex; flex-direction:column; gap:8px;">
                <div style="display:flex; justify-content:space-between; align-items:center; gap:8px;">
                    <strong style="font-size:11px; color:#fbbf24; letter-spacing:0.06em; text-transform:uppercase;">Pending Choice</strong>
                    <span style="font-size:10px; opacity:0.85;">${escapeHtml(choiceType)}</span>
                </div>
                <div style="display:grid; grid-template-columns:repeat(auto-fit, minmax(120px, 1fr)); gap:6px;">
                    ${extractScalarEntries(pending).map(([key, value]) => renderScalarCell(key, summarizeObject(value))).join('')}
                </div>
            </div>
        `;
    },

    _buildZoneDiagnostic: (player, zone) => {
        const entries = zone.cards.map((entry, index) => ({
            index,
            card: DebugModal._normalizeCard(entry),
        })).filter((entry) => entry.card && entry.card.id !== -1 && entry.card.id !== -2);

        const triggers = new Set();
        const conditions = new Set();
        const effects = new Set();
        const costs = new Set();
        const abilityFlags = new Set();
        const synergyFlags = new Set();
        const semanticFlags = new Set();

        let tapped = 0;
        let moved = 0;
        let revealed = 0;
        let totalAbilities = 0;
        let totalNotes = 0;

        entries.forEach(({ card }) => {
            if (card.orientation === 'Wait') tapped += 1;
            if (card.moved) moved += 1;
            if (card.revealed) revealed += 1;
            totalNotes += Number(card.note_icons || 0);
            decodeBitmask(card.ability_flags || 0, ABILITY_FLAG_BITS).forEach((item) => abilityFlags.add(item));
            decodeBitmask(card.synergy_flags || 0, SYNERGY_FLAG_BITS).forEach((item) => synergyFlags.add(item));
            if (Number.isFinite(card.semantic_flags)) semanticFlags.add(formatPacked(card.semantic_flags));

            (card.abilities || []).forEach((ability) => {
                totalAbilities += 1;
                triggers.add(TRIGGER_NAMES[ability.trigger] || `TRIGGER_${ability.trigger ?? '?'}`);
                (ability.conditions || []).forEach((condition) => {
                    conditions.add(CONDITION_NAMES[condition.condition_type] || `COND_${condition.condition_type ?? '?'}`);
                });
                (ability.effects || []).forEach((effect) => {
                    effects.add(EFFECT_NAMES[effect.effect_type] || `EFFECT_${effect.effect_type ?? '?'}`);
                });
                (ability.costs || []).forEach((cost) => {
                    costs.add(COST_NAMES[cost.cost_type] || `COST_${cost.cost_type ?? '?'}`);
                });
            });
        });

        return {
            cards: entries.length,
            tapped,
            moved,
            revealed,
            totalAbilities,
            totalNotes,
            triggers: Array.from(triggers).sort(),
            conditions: Array.from(conditions).sort(),
            effects: Array.from(effects).sort(),
            costs: Array.from(costs).sort(),
            abilityFlags: Array.from(abilityFlags).sort(),
            synergyFlags: Array.from(synergyFlags).sort(),
            semanticFlags: Array.from(semanticFlags).sort(),
        };
    },

    _renderZoneDiagnostics: (player) => {
        const zones = zoneDefinitions(player);
        return `
            <div style="background:rgba(255,255,255,0.03); border:1px solid rgba(255,255,255,0.08); border-radius:8px; padding:12px; display:flex; flex-direction:column; gap:10px; overflow:hidden;">
                <div style="display:flex; justify-content:space-between; align-items:center; gap:8px;">
                    <strong style="font-size:12px;">Zone Diagnostics</strong>
                    <span style="font-size:10px; opacity:0.7;">All visible trigger, condition, cost, effect, and flag surfaces per zone</span>
                </div>
                <div style="overflow:auto; border:1px solid rgba(255,255,255,0.06); border-radius:6px;">
                    <table style="width:100%; border-collapse:collapse; min-width:1180px; font-size:10px;">
                        <thead>
                            <tr style="background:rgba(15,23,42,0.95); text-transform:uppercase; letter-spacing:0.04em;">
                                <th style="padding:8px; text-align:left; border-bottom:1px solid rgba(255,255,255,0.08);">Zone</th>
                                <th style="padding:8px; text-align:left; border-bottom:1px solid rgba(255,255,255,0.08);">Counts</th>
                                <th style="padding:8px; text-align:left; border-bottom:1px solid rgba(255,255,255,0.08);">Triggers</th>
                                <th style="padding:8px; text-align:left; border-bottom:1px solid rgba(255,255,255,0.08);">Conditions</th>
                                <th style="padding:8px; text-align:left; border-bottom:1px solid rgba(255,255,255,0.08);">Costs</th>
                                <th style="padding:8px; text-align:left; border-bottom:1px solid rgba(255,255,255,0.08);">Effects</th>
                                <th style="padding:8px; text-align:left; border-bottom:1px solid rgba(255,255,255,0.08);">Flags</th>
                            </tr>
                        </thead>
                        <tbody>
                            ${zones.map((zone) => {
                                const diag = DebugModal._buildZoneDiagnostic(player, zone);
                                return `
                                    <tr style="vertical-align:top; background:${DebugModal._filters.selectedZone === zone.key ? 'rgba(56,189,248,0.06)' : 'transparent'};">
                                        <td style="padding:8px; border-bottom:1px solid rgba(255,255,255,0.06); font-weight:700;">${escapeHtml(zone.label)}</td>
                                        <td style="padding:8px; border-bottom:1px solid rgba(255,255,255,0.06); min-width:180px;">
                                            <div style="display:grid; grid-template-columns:repeat(2, minmax(80px, 1fr)); gap:6px;">
                                                ${renderScalarCell('cards', diag.cards)}
                                                ${renderScalarCell('abilities', diag.totalAbilities)}
                                                ${renderScalarCell('tapped', diag.tapped)}
                                                ${renderScalarCell('revealed', diag.revealed)}
                                                ${renderScalarCell('moved', diag.moved)}
                                                ${renderScalarCell('notes', diag.totalNotes)}
                                            </div>
                                        </td>
                                        <td style="padding:8px; border-bottom:1px solid rgba(255,255,255,0.06); min-width:180px;"><div style="display:flex; flex-wrap:wrap; gap:4px;">${renderChips(diag.triggers, '#4ade80')}</div></td>
                                        <td style="padding:8px; border-bottom:1px solid rgba(255,255,255,0.06); min-width:240px;"><div style="display:flex; flex-wrap:wrap; gap:4px;">${renderChips(diag.conditions, '#38bdf8')}</div></td>
                                        <td style="padding:8px; border-bottom:1px solid rgba(255,255,255,0.06); min-width:180px;"><div style="display:flex; flex-wrap:wrap; gap:4px;">${renderChips(diag.costs, '#fb923c')}</div></td>
                                        <td style="padding:8px; border-bottom:1px solid rgba(255,255,255,0.06); min-width:220px;"><div style="display:flex; flex-wrap:wrap; gap:4px;">${renderChips(diag.effects, '#facc15')}</div></td>
                                        <td style="padding:8px; border-bottom:1px solid rgba(255,255,255,0.06); min-width:260px;">
                                            <div style="display:flex; flex-direction:column; gap:6px;">
                                                <div><span style="opacity:0.65;">ability</span><div style="display:flex; flex-wrap:wrap; gap:4px; margin-top:4px;">${renderChips(diag.abilityFlags, '#22c55e')}</div></div>
                                                <div><span style="opacity:0.65;">synergy</span><div style="display:flex; flex-wrap:wrap; gap:4px; margin-top:4px;">${renderChips(diag.synergyFlags, '#eab308')}</div></div>
                                                <div><span style="opacity:0.65;">semantic</span><div style="display:flex; flex-wrap:wrap; gap:4px; margin-top:4px;">${renderChips(diag.semanticFlags, '#c084fc')}</div></div>
                                            </div>
                                        </td>
                                    </tr>
                                `;
                            }).join('')}
                        </tbody>
                    </table>
                </div>
            </div>
        `;
    },

    _collectAbilityRows: (visibleCards) => visibleCards.flatMap((entry) => {
        const card = entry.card;
        let cardName = card.name;
        if (!cardName && card.id !== undefined) {
            const resolved = State.resolveCardData(card.id);
            if (resolved && resolved.name) {
                cardName = resolved.name;
            }
        }
        return (card.abilities || []).map((ability, abilityIndex) => ({
            cardName: cardName || `Card ${card.id}`,
            cardId: card.id ?? card.card_id ?? '?',
            slotLabel: entry.slotLabel,
            abilityIndex,
            trigger: TRIGGER_NAMES[ability.trigger] || `TRIGGER_${ability.trigger ?? '?'}`,
            conditions: (ability.conditions || []).map((condition) => CONDITION_NAMES[condition.condition_type] || `COND_${condition.condition_type ?? '?'}`),
            costs: (ability.costs || []).map((cost) => COST_NAMES[cost.cost_type] || `COST_${cost.cost_type ?? '?'}`),
            effects: (ability.effects || []).map((effect) => EFFECT_NAMES[effect.effect_type] || `EFFECT_${effect.effect_type ?? '?'}`),
            flags: [
                ...(ability.choice_flags !== undefined ? decodeBitmask(ability.choice_flags, CHOICE_FLAG_BITS) : []),
                ...(ability.filter_flags !== undefined ? decodeBitmask(ability.filter_flags, FILTER_FLAG_BITS) : []),
                ...(ability.cost_flags !== undefined ? decodeBitmask(ability.cost_flags, COST_FLAG_BITS) : []),
            ],
            pseudocode: ability.pseudocode || '',
        }));
    }),

    _renderAbilityMatrix: (visibleCards) => {
        const rows = DebugModal._collectAbilityRows(visibleCards);
        return `
            <div style="background:rgba(255,255,255,0.03); border:1px solid rgba(255,255,255,0.08); border-radius:8px; padding:12px; display:flex; flex-direction:column; gap:10px; overflow:hidden;">
                <div style="display:flex; justify-content:space-between; align-items:center; gap:8px;">
                    <strong style="font-size:12px;">Ability Matrix</strong>
                    <span style="font-size:10px; opacity:0.7;">Every visible ability in the current filter window</span>
                </div>
                ${rows.length === 0 ? '<div style="opacity:0.55; text-align:center; padding:20px; font-size:11px;">No abilities match the current zone/search filter.</div>' : `
                    <div style="overflow:auto; border:1px solid rgba(255,255,255,0.06); border-radius:6px;">
                        <table style="width:100%; border-collapse:collapse; min-width:1350px; font-size:10px;">
                            <thead>
                                <tr style="background:rgba(15,23,42,0.95); text-transform:uppercase; letter-spacing:0.04em;">
                                    <th style="padding:8px; text-align:left; border-bottom:1px solid rgba(255,255,255,0.08);">Card</th>
                                    <th style="padding:8px; text-align:left; border-bottom:1px solid rgba(255,255,255,0.08);">Zone</th>
                                    <th style="padding:8px; text-align:left; border-bottom:1px solid rgba(255,255,255,0.08);">Trigger</th>
                                    <th style="padding:8px; text-align:left; border-bottom:1px solid rgba(255,255,255,0.08);">Conditions</th>
                                    <th style="padding:8px; text-align:left; border-bottom:1px solid rgba(255,255,255,0.08);">Costs</th>
                                    <th style="padding:8px; text-align:left; border-bottom:1px solid rgba(255,255,255,0.08);">Effects</th>
                                    <th style="padding:8px; text-align:left; border-bottom:1px solid rgba(255,255,255,0.08);">Flags</th>
                                    <th style="padding:8px; text-align:left; border-bottom:1px solid rgba(255,255,255,0.08);">Pseudocode</th>
                                </tr>
                            </thead>
                            <tbody>
                                ${rows.map((row) => `
                                    <tr style="vertical-align:top;">
                                        <td style="padding:8px; border-bottom:1px solid rgba(255,255,255,0.06); min-width:180px;">
                                            <strong>${escapeHtml(row.cardName)}</strong><br/>
                                            <span style="opacity:0.6; font-family:'Cascadia Code', monospace;">id=${escapeHtml(row.cardId)} a${row.abilityIndex + 1}</span>
                                        </td>
                                        <td style="padding:8px; border-bottom:1px solid rgba(255,255,255,0.06); min-width:120px;">${escapeHtml(row.slotLabel)}</td>
                                        <td style="padding:8px; border-bottom:1px solid rgba(255,255,255,0.06); min-width:140px;">${escapeHtml(row.trigger)}</td>
                                        <td style="padding:8px; border-bottom:1px solid rgba(255,255,255,0.06); min-width:220px;"><div style="display:flex; flex-wrap:wrap; gap:4px;">${renderChips(row.conditions, '#38bdf8')}</div></td>
                                        <td style="padding:8px; border-bottom:1px solid rgba(255,255,255,0.06); min-width:180px;"><div style="display:flex; flex-wrap:wrap; gap:4px;">${renderChips(row.costs, '#fb923c')}</div></td>
                                        <td style="padding:8px; border-bottom:1px solid rgba(255,255,255,0.06); min-width:220px;"><div style="display:flex; flex-wrap:wrap; gap:4px;">${renderChips(row.effects, '#facc15')}</div></td>
                                        <td style="padding:8px; border-bottom:1px solid rgba(255,255,255,0.06); min-width:190px;"><div style="display:flex; flex-wrap:wrap; gap:4px;">${renderChips(row.flags, '#22c55e')}</div></td>
                                        <td style="padding:8px; border-bottom:1px solid rgba(255,255,255,0.06); min-width:280px; line-height:1.4;">${escapeHtml(row.pseudocode || '-')}</td>
                                    </tr>
                                `).join('')}
                            </tbody>
                        </table>
                    </div>
                `}
            </div>
        `;
    },

    _renderFlagRow: (label, value, bits, accent) => {
        if (value === undefined || value === null) return '';
        const decoded = bits.length > 0 ? decodeBitmask(value, bits) : [];

        return `
            <div style="border:1px solid rgba(255,255,255,0.08); border-radius:6px; padding:8px; background:rgba(255,255,255,0.025); display:flex; flex-direction:column; gap:6px;">
                <div style="display:flex; justify-content:space-between; align-items:center; gap:8px; font-size:10px;">
                    <strong style="color:${accent};">${escapeHtml(label)}</strong>
                    <span style="font-family:'Cascadia Code', monospace; opacity:0.7;">${escapeHtml(formatPacked(value))}</span>
                </div>
                <div style="display:flex; flex-wrap:wrap; gap:4px;">${renderChips(decoded.length > 0 ? decoded : [formatPacked(value)], accent)}</div>
            </div>
        `;
    },

    _renderLogicItem: (item, itemType, accent) => {
        const typeField = itemType === 'condition' ? 'condition_type' : itemType === 'effect' ? 'effect_type' : 'cost_type';
        const labelMap = itemType === 'condition' ? CONDITION_NAMES : itemType === 'effect' ? EFFECT_NAMES : COST_NAMES;
        const typeValue = item[typeField];
        const itemLabel = labelMap[typeValue] || `${itemType.toUpperCase()}_${typeValue ?? '?'}`;
        const scalarEntries = extractScalarEntries(item);

        return `
            <div style="border:1px solid rgba(255,255,255,0.08); border-left:3px solid ${accent}; border-radius:6px; padding:8px; background:rgba(255,255,255,0.025); display:flex; flex-direction:column; gap:8px;">
                <div style="display:flex; justify-content:space-between; align-items:center; gap:8px;">
                    <strong style="font-size:10px; color:${accent};">${escapeHtml(itemLabel)}</strong>
                    <span style="font-size:9px; opacity:0.65; font-family:'Cascadia Code', monospace;">${escapeHtml(typeField)}=${escapeHtml(typeValue ?? '?')}</span>
                </div>
                <div style="display:grid; grid-template-columns:repeat(auto-fit, minmax(110px, 1fr)); gap:6px;">
                    ${scalarEntries.map(([key, value]) => renderScalarCell(key, typeof value === 'number' ? describeNumber(key, value, itemType) : summarizeObject(value))).join('')}
                </div>
                <details>
                    <summary style="cursor:pointer; opacity:0.65; font-size:9px;">Raw JSON</summary>
                    <pre style="margin:6px 0 0 0; padding:8px; background:#05070d; border-radius:4px; font-size:9px; line-height:1.35; color:#dbeafe; white-space:pre-wrap; word-break:break-word;">${escapeHtml(JSON.stringify(item, null, 2))}</pre>
                </details>
            </div>
        `;
    },

    _renderLogicGroup: (title, items, itemType, accent) => {
        if (!items || items.length === 0) return '';
        return `
            <div style="display:flex; flex-direction:column; gap:6px;">
                <div style="font-size:10px; text-transform:uppercase; letter-spacing:0.06em; color:${accent};">${title} (${items.length})</div>
                ${items.map((item) => DebugModal._renderLogicItem(item, itemType, accent)).join('')}
            </div>
        `;
    },

    _renderAbilityBlock: (ability, abilityIndex) => {
        const triggerLabel = TRIGGER_NAMES[ability.trigger] || `TRIGGER_${ability.trigger ?? '?'}`;
        const abilityTags = [
            ability.is_once_per_turn ? 'ONCE_PER_TURN' : null,
            ability.requires_selection ? 'REQUIRES_SELECTION' : null,
            ability.choice_count ? `CHOICE_COUNT=${ability.choice_count}` : null,
        ].filter(Boolean);

        return `
            <div style="display:flex; flex-direction:column; gap:8px; padding:10px; border-radius:8px; background:rgba(0,0,0,0.18); border:1px solid rgba(255,255,255,0.08);">
                <div style="display:flex; justify-content:space-between; gap:8px; align-items:flex-start;">
                    <div style="display:flex; flex-direction:column; gap:4px; min-width:0;">
                        <div style="display:flex; flex-wrap:wrap; gap:6px; align-items:center;">
                            <strong style="font-size:11px; color:#fbbf24;">Ability ${abilityIndex + 1}</strong>
                            <span style="font-size:9px; padding:2px 6px; border-radius:999px; background:rgba(251,191,36,0.12); border:1px solid rgba(251,191,36,0.35); color:#fbbf24;">${escapeHtml(triggerLabel)}</span>
                            ${abilityTags.map((tag) => `<span style="font-size:9px; padding:2px 6px; border-radius:999px; background:rgba(255,255,255,0.08);">${escapeHtml(tag)}</span>`).join('')}
                        </div>
                        <div style="font-size:10px; line-height:1.45; opacity:0.88;">${escapeHtml(ability.pseudocode || 'No pseudocode')}</div>
                    </div>
                    <div style="font-size:9px; opacity:0.65; font-family:'Cascadia Code', monospace;">trigger=${escapeHtml(ability.trigger ?? '?')}</div>
                </div>

                <div style="display:grid; grid-template-columns:repeat(auto-fit, minmax(110px, 1fr)); gap:6px;">
                    ${ability.choice_type !== undefined ? renderScalarCell('choice_type', describeNumber('choice_type', ability.choice_type)) : ''}
                    ${ability.choice_flags !== undefined ? renderScalarCell('choice_flags', describeNumber('choice_flags', ability.choice_flags)) : ''}
                    ${ability.filter_flags !== undefined ? renderScalarCell('filter_flags', describeNumber('filter_flags', ability.filter_flags)) : ''}
                </div>

                ${DebugModal._renderLogicGroup('Conditions', ability.conditions, 'condition', '#38bdf8')}
                ${DebugModal._renderLogicGroup('Costs', ability.costs, 'cost', '#fb923c')}
                ${DebugModal._renderLogicGroup('Effects', ability.effects, 'effect', '#22c55e')}

                ${(ability.decoded_bytecode && ability.decoded_bytecode.length > 0) ? `
                    <details>
                        <summary style="cursor:pointer; opacity:0.65; font-size:9px;">Decoded Bytecode (${ability.decoded_bytecode.length})</summary>
                        <pre style="margin:6px 0 0 0; padding:8px; background:#05070d; border-radius:4px; font-size:9px; line-height:1.3; color:#8df58d; white-space:pre-wrap; word-break:break-word;">${escapeHtml(ability.decoded_bytecode.join('\n'))}</pre>
                    </details>
                ` : ''}
            </div>
        `;
    },

    _renderMetadataRows: (card) => {
        const metadataFields = [
            'card_no',
            'attribute',
            'group',
            'group_mask',
            'unit',
            'unit_mask',
            'school',
            'year',
            'character',
            'traits',
            'keywords',
            'required_member',
            'required_group',
            'required_unit',
            'required_color',
            'activation_limit',
            'activation_count',
            'prevent_activate',
            'prevent_baton_touch',
            'prevent_success_pile_set',
        ];

        const present = metadataFields
            .filter((key) => card[key] !== undefined && card[key] !== null && card[key] !== '')
            .map((key) => renderScalarCell(key, summarizeObject(card[key])));

        if (present.length === 0) {
            return '<div style="opacity:0.5; font-size:10px;">No extra metadata surfaced on this card snapshot.</div>';
        }

        return `
            <div style="display:grid; grid-template-columns:repeat(auto-fit, minmax(120px, 1fr)); gap:6px;">
                ${present.join('')}
            </div>
        `;
    },

    _renderCardInspector: (entry, index) => {
        const card = entry.card;
        if (!card) return '';

        const abilities = card.abilities || [];
        const cardType = card.card_type || (card.score !== undefined ? 'Live' : 'Member');
        const statusBits = [
            card.orientation === 'Wait' ? 'TAPPED' : null,
            card.moved ? 'MOVED' : null,
            card.revealed ? 'REVEALED' : null,
            card.is_active ? 'ACTIVE' : null,
            card.waiting ? 'WAIT' : null,
        ].filter(Boolean);

        let displayName = card.name;
        if (!displayName && card.id !== undefined) {
            const resolved = State.resolveCardData(card.id);
            if (resolved && resolved.name) {
                displayName = resolved.name;
            }
        }

        return `
            <div style="background:rgba(255,255,255,0.045); border:1px solid #334155; border-radius:8px; padding:12px; display:flex; flex-direction:column; gap:10px;">
                <div style="display:flex; justify-content:space-between; align-items:flex-start; gap:10px; padding-bottom:8px; border-bottom:1px solid rgba(255,255,255,0.08);">
                    <div style="display:flex; flex-direction:column; gap:4px; min-width:0;">
                        <div style="display:flex; flex-wrap:wrap; gap:6px; align-items:center;">
                            <strong style="font-size:13px; color:${cardType === 'live' ? '#f87171' : '#7dd3fc'};">${escapeHtml(displayName || `Card ${card.id}`)}</strong>
                            <span style="font-size:9px; padding:2px 6px; border-radius:999px; background:rgba(255,255,255,0.08); opacity:0.75;">${escapeHtml(entry.zoneLabel)}</span>
                            <span style="font-size:9px; padding:2px 6px; border-radius:999px; background:rgba(255,255,255,0.08); opacity:0.75;">${escapeHtml(cardType.toUpperCase())}</span>
                            <span style="font-size:9px; opacity:0.55;">#${index + 1}</span>
                        </div>
                        <div style="font-size:10px; opacity:0.72;">${escapeHtml(entry.slotLabel)}</div>
                    </div>
                    <div style="font-size:10px; opacity:0.72; font-family:'Cascadia Code', monospace; text-align:right;">ID ${escapeHtml(card.id ?? card.card_id ?? '?')}</div>
                </div>

                <div style="display:grid; grid-template-columns:repeat(auto-fit, minmax(115px, 1fr)); gap:6px;">
                    ${renderScalarCell('type', cardType)}
                    ${renderScalarCell(cardType === 'live' ? 'score' : 'cost', (cardType === 'live' ? (card.score ?? 0) : (card.cost ?? 0)))}
                    ${renderScalarCell('blades', (card.blades ?? 0))}
                    ${renderScalarCell('hearts', summarizeObject(card.hearts ?? card.required_hearts ?? []))}
                    ${renderScalarCell('notes', (card.note_icons ?? 0))}
                    ${renderScalarCell('status', statusBits.join(', ') || 'none')}
                </div>

                <div style="display:grid; grid-template-columns:repeat(auto-fit, minmax(180px, 1fr)); gap:8px;">
                    ${DebugModal._renderFlagRow('Semantic Flags', card.semantic_flags ?? 0, [], '#c084fc')}
                    ${DebugModal._renderFlagRow('Ability Flags', card.ability_flags ?? 0, ABILITY_FLAG_BITS, '#22c55e')}
                    ${DebugModal._renderFlagRow('Synergy Flags', card.synergy_flags ?? 0, SYNERGY_FLAG_BITS, '#eab308')}
                    ${DebugModal._renderFlagRow('Cost Flags', card.cost_flags ?? 0, COST_FLAG_BITS, '#f97316')}
                </div>

                <div style="display:flex; flex-direction:column; gap:6px;">
                    <strong style="font-size:11px;">Metadata Surface</strong>
                    ${DebugModal._renderMetadataRows(card)}
                </div>

                ${abilities.length === 0 ? '<div style="opacity:0.5; font-size:10px;">No abilities on this card.</div>' : `
                    <div style="display:flex; flex-direction:column; gap:10px;">
                        ${abilities.map((ability, abilityIndex) => DebugModal._renderAbilityBlock(ability, abilityIndex)).join('')}
                    </div>
                `}
            </div>
        `;
    },

    _collectVisibleCards: (player, zoneKey) => {
        const defs = zoneDefinitions(player);
        const selectedDefs = zoneKey === 'all' ? defs : defs.filter((zone) => zone.key === zoneKey);
        return selectedDefs.flatMap((zone) => zone.cards.map((rawEntry, index) => ({
            zoneKey: zone.key,
            zoneLabel: zone.label,
            slotLabel: `${zone.label} ${index + 1}`,
            slotIndex: index,
            card: DebugModal._normalizeCard(rawEntry),
        })));
    },

    _matchesSearch: (entry, search) => {
        if (!search) return true;
        const card = entry.card;
        if (!card) return false;

        const needle = search.toLowerCase();
        if ((card.name || '').toLowerCase().includes(needle)) return true;
        if (String(card.id || '').includes(needle)) return true;
        if ((entry.zoneLabel || '').toLowerCase().includes(needle)) return true;

        return (card.abilities || []).some((ability) => {
            if ((ability.pseudocode || '').toLowerCase().includes(needle)) return true;
            const triggerName = TRIGGER_NAMES[ability.trigger] || '';
            if (triggerName.toLowerCase().includes(needle)) return true;
            return (ability.conditions || []).some((condition) => {
                const conditionName = CONDITION_NAMES[condition.condition_type] || '';
                return conditionName.toLowerCase().includes(needle);
            });
        });
    },

    _normalizeCard: (rawEntry) => {
        if (rawEntry === null || rawEntry === undefined) return null;
        if (typeof rawEntry === 'number') return State.resolveCardData(rawEntry);

        if (typeof rawEntry === 'object' && rawEntry.card) {
            const { card, ...rest } = rawEntry;
            const resolvedCard = DebugModal._normalizeCard(card);
            if (!resolvedCard) return null;
            return { ...resolvedCard, ...rest };
        }

        if (typeof rawEntry === 'object') {
            if (rawEntry.id === undefined && rawEntry.card_id !== undefined) {
                return { ...rawEntry, id: rawEntry.card_id };
            }
            return rawEntry;
        }

        return null;
    },

    onPlayerChange: (value) => {
        DebugModal._filters.selectedPlayer = parseInt(value, 10);
        DebugModal.renderInspector();
    },

    onZoneChange: (value) => {
        DebugModal._filters.selectedZone = value;
        DebugModal.renderInspector();
    },

    onSearchChange: (value) => {
        DebugModal._filters.abilitySearch = value;
        DebugModal.renderInspector();
    },

    rewind: async () => {
        const ok = await Network.rewind();
        if (!ok) {
            DebugModal._setStatus('error', 'Undo failed.');
            await DebugModal.renderAll();
            return;
        }
        DebugModal._clearStatus();
        await DebugModal.renderAll();
        if (window.Rendering) window.Rendering.render();
    },

    redo: async () => {
        const ok = await Network.redo();
        if (!ok) {
            DebugModal._setStatus('error', 'Redo failed.');
            await DebugModal.renderAll();
            return;
        }
        DebugModal._clearStatus();
        await DebugModal.renderAll();
        if (window.Rendering) window.Rendering.render();
    },

    toggleDebugMode: async () => {
        const ok = await Network.toggleDebugMode();
        DebugModal._setStatus(ok ? 'success' : 'error', ok ? 'Debug mode toggled.' : 'Toggle debug mode failed.');
        await DebugModal.renderAll();
    },
};

window.DebugModal = DebugModal;
window.openDebugModal = () => DebugModal.openDebugModal();
window.closeDebugModal = () => DebugModal.closeDebugModal();

window.Modals = window.Modals || {};
window.Modals.openDebugModal = () => DebugModal.openDebugModal();
window.Modals.closeDebugModal = () => DebugModal.closeDebugModal();
window.Modals.toggleDebugMode = () => DebugModal.toggleDebugMode();
window.Modals.rewind = () => DebugModal.rewind();
window.Modals.redo = () => DebugModal.redo();