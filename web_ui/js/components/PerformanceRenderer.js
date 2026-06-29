/**
 * Performance Renderer Component
 * Renders a player-facing explanation of the performance phase using the
 * snapshot emitted by the Rust engine.
 */
import { State } from '../state.js';
import { fixImg, Phase, isMulliganPhase, isLiveCardSetPhase } from '../constants.js';
import { resolveCardImagePath } from './CardRenderer.js';
import * as i18n from '../i18n/index.js';
import { Tooltips } from '../ui_tooltips.js';
import { TextEnricher } from '../utils/TextEnricher.js';

// heart types: index 0 = heart_00 (wildcard Any), index 7 = icon_all (counts as ALL colors)
const HEART_LABELS = ['Any', 'Pink', 'Red', 'Yellow', 'Green', 'Blue', 'Purple', 'All'];

const HEART_ICONS = [
    'img/texticon/heart_00.png', 'img/texticon/heart_01.png', 'img/texticon/heart_02.png',
    'img/texticon/heart_03.png', 'img/texticon/heart_04.png', 'img/texticon/heart_05.png',
    'img/texticon/heart_06.png', 'img/texticon/icon_all.png'
];

function tr(key, params) {
    return i18n.t(key, params);
}

function escapeHtml(value) {
    return String(value ?? '')
        .replace(/&/g, '&amp;')
        .replace(/</g, '&lt;')
        .replace(/>/g, '&gt;')
        .replace(/"/g, '&quot;')
        .replace(/'/g, '&#39;');
}

function enrichText(value) {
    if (!value) return '';
    return Tooltips.enrichAbilityText(value);
}

function sumHearts(hearts) {
    return hearts.reduce((total, value) => total + (value || 0), 0);
}

function countPassedLives(lives) {
    return lives.filter((live) => live && live.passed).length;
}

function sumPassedLiveScores(lives) {
    return lives
        .filter((live) => live && live.passed)
        .reduce((total, live) => total + (live.score || 0), 0);
}

function sumPassedBaseScores(lives) {
    return lives
        .filter((live) => live && live.passed)
        .reduce((total, live) => total + (live.base_score || 0), 0);
}

function findAbilitySource(triggered, sourceText) {
    if (!triggered || !sourceText) return null;
    for (const t of triggered) {
        if (t.effect_text && sourceText.includes(t.effect_text)) return t;
        if (t.name && sourceText.includes(t.name)) return t;
    }
    return null;
}

function getDisplayResults(results) {
    // If caller passed a structured object (not array), use it directly
    if (results && typeof results === 'object' && !Array.isArray(results)) {
        return arrayFromPerformanceMap(results);
    }

    // Prefer the ordered performance_history array — it's indexed [0]=P1, [1]=P2
    if (State.data?.performance_history && State.data.performance_history.length > 0) {
        return State.data.performance_history;
    }

    // Fallback: convert the HashMap to an ordered array
    const perfMap = State.data?.performance_results || {};
    return arrayFromPerformanceMap(perfMap);
}

/// Convert a player_id → snapshot HashMap to an ordered array [P1, P2].
function arrayFromPerformanceMap(perfMap) {
    const keys = Object.keys(perfMap);
    if (keys.length === 0) return [];
    // Try to order using known player_id patterns
    const knownOrder = ['player1', 'player2', 'p1', 'p2', '0', '1'];
    const ordered = [];
    for (const key of knownOrder) {
        if (perfMap[key] !== undefined) ordered.push(perfMap[key]);
    }
    // If we got nothing from known patterns, just use whatever keys exist
    if (ordered.length === 0) {
        for (const key of keys) ordered.push(perfMap[key]);
    }
    return ordered;
}

function getPlayerName(playerId) {
    return playerId === State.perspectivePlayer
        ? tr('you')
        : tr('opponent');
}

function getTurnLabel(turn) {
    if (turn === undefined || turn === null) {
        return tr('perf_breakdown_title');
    }
    return tr('perf_breakdown_turn', { turn });
}

function getOutcomeLabel(playerId, result) {
    if (!result) return tr('perf_outcome_no_result');
    const winsKey = playerId === 0 ? 'p0_wins' : 'p1_wins';
    const otherWinsKey = playerId === 0 ? 'p1_wins' : 'p0_wins';
    const selfWins = !!result[winsKey];
    const otherWins = !!result[otherWinsKey];

    if (selfWins && otherWins) return tr('perf_outcome_tie');
    if (selfWins) return tr('perf_outcome_won');
    if (otherWins) return tr('perf_outcome_lost');
    return result.success ? tr('perf_outcome_pass') : tr('perf_outcome_fail');
}

function renderIconMetric(iconPath, label, value, accentClass = '') {
    return `
        <div class="perf-metric-card ${accentClass}">
            <div class="perf-metric-label">
                <img src="${iconPath}" class="perf-inline-icon" alt="">
                <span>${escapeHtml(label)}</span>
            </div>
            <div class="perf-metric-value">${escapeHtml(value)}</div>
        </div>
    `;
}

function renderTextMetric(label, value, detail = '') {
    return `
        <div class="perf-metric-card perf-metric-card-text">
            <div class="perf-metric-label">${escapeHtml(label)}</div>
            <div class="perf-metric-value">${escapeHtml(value)}</div>
            ${detail ? `<div class="perf-metric-detail">${escapeHtml(detail)}</div>` : ''}
        </div>
    `;
}

function renderHeartsGrid(hearts) {
    const values = hearts;
    const filtered = HEART_LABELS.map((label, index) => ({
        label,
        count: values[index] || 0,
        icon: HEART_ICONS[index],
        index
    })).filter(h => h.count > 0);

    if (filtered.length === 0) return `<div class="perf-hearts-grid empty">${tr('perf_none')}</div>`;

    return `
        <div class="perf-hearts-grid">
            ${filtered.map(h => `
                <div class="heart-grid-cell${h.index === 0 ? ' color-any' : ' color-'+h.index}">
                    <img src="${h.icon}" class="heart-mini-icon" alt="${escapeHtml(h.label)}">
                    <span class="count-value">${h.count}</span>
                    ${h.index === 0 ? '<span class="heart-any-label">Any</span>' : ''}
                </div>
            `).join('')}
        </div>
    `;
}

function renderHeartsCompact(hearts) {
    if (!Array.isArray(hearts) || hearts.every((value) => !value)) {
        return `<span class="perf-empty-inline">${tr('perf_none')}</span>`;
    }

    const heartLabels = ['Any', 'Pink', 'Red', 'Yellow', 'Green', 'Blue', 'Purple', 'All'];

    return `<div class="hearts-compact">${hearts.map((count, index) => {
        if (!count) return '';
        const iconSrc = HEART_ICONS[index];
        return `
            <div class="heart-tag ${index === 0 ? 'color-any' : `color-${index}`}" title="${heartLabels[index]}">
                <img src="${iconSrc}" class="heart-mini-icon" alt="${heartLabels[index]}">
                <span>${count}</span>
            </div>
        `;
    }).join('')}</div>`;
}

function renderBladesCompact(blades) {
    if (!blades || blades <= 0) {
        return '<span class="perf-empty-inline">0</span>';
    }

    let html = '<div class="blades-compact">';
    for (let index = 0; index < blades; index += 1) {
        html += '<img src="img/texticon/icon_blade.png" class="heart-mini-icon" alt="Blade">';
    }
    html += '</div>';
    return html;
}

function renderHeartProgress(filled, required) {
    if (!required || !Array.isArray(required)) return '';
    const filledArr = Array.isArray(filled) ? filled : [];
    let html = '<div class="heart-progress-row">';
    for (let color = 0; color < 7; color += 1) {
        const requiredCount = required[color] || 0;
        const filledCount = filledArr[color] || 0;
        for (let slot = 0; slot < requiredCount; slot += 1) {
            const isFilled = slot < filledCount;
            html += `<div class="heart-pip color-${color} ${isFilled ? 'filled' : 'empty'}"></div>`;
        }
    }
    html += '</div>';
    return html;
}

function renderAggregateHeartSummary(result) {
    const lives = result?.lives || [];
    const totalHearts = result?.total_hearts || [0,0,0,0,0,0,0,0];
    const totalAvailable = sumHearts(totalHearts);
    const allocations = result?.breakdown?.allocations || [];
    // Show even with 0 lives — reveals the available pool / surplus.
    if (totalAvailable === 0 && lives.length === 0) return '';

    // Use the engine's own success determination (all passed + total_score > 0)
    const isSuccess = !!result.success;

    let html = `
        <div class="perf-agg-summary ${isSuccess ? 'success' : 'failure'}">
            <div class="perf-agg-header">
                <img src="img/texticon/heart_00.png" class="heart-mini-icon" alt="">
                ${tr('perf_agg_header')}
            </div>
            <div class="perf-agg-table">
                <div class="perf-agg-row">
                    <span class="perf-agg-label">${tr('perf_agg_available_pool')}</span>
                    ${renderHeartsCompact(totalHearts)}
                    <span class="perf-agg-sum">${totalAvailable}</span>
                </div>`;

    // Phase tags emitted by the engine (engine/src/turn/live.rs compute_allocations):
    //   1a_colored        — colored hearts → colored req (matching color, capped at req)
    //   1b_h00_wild       — Heart00 wildcard → remaining colored deficit
    //   2_wildcard        — remaining Heart00 wild → remaining colored deficit (second pass)
    //   3a_colored_surplus — leftover colored hearts → Heart00 req (demand-aware: prefers
    //                        colors with most surplus vs future cards' needs)
    //   3b_h00            — Heart00 wildcards → remaining Heart00 req
    //   4_all_cleanup     — icon_all → ANY remaining deficit (color deficits first,
    //                        then heart00). Uses texticon images to show conversion.

    if (lives.length > 0) {
        for (let liveIdx = 0; liveIdx < lives.length; liveIdx++) {
            const live = lives[liveIdx];
            const cd = live.card_no ? State.resolveCardData(live.card_no) : null;
            const liveName = cd?.name || `Live ${liveIdx + 1}`;
            const req = live.required || [0,0,0,0,0,0,0,0];
            const passed = live.passed;
            const reqSum = sumHearts(req);
            const colorReqSum = sumHearts(req.slice(1, 7));
            const wildReq = req[0] || 0;

            const liveAllocs = allocations.filter(a => a.target_idx === liveIdx);
            const phase1aAllocs = liveAllocs.filter(a => a.phase === '1a_colored');
            const phase1bAllocs = liveAllocs.filter(a => a.phase === '1b_h00_wild');
            const phase1cAllocs = liveAllocs.filter(a => a.phase === '1c_all_wild');
            const phase2Allocs = liveAllocs.filter(a => a.phase === '2_wildcard');
            const phase3aAllocs = liveAllocs.filter(a => a.phase === '3a_colored_surplus');
            const phase3bAllocs = liveAllocs.filter(a => a.phase === '3b_h00');
            const phase3cAllocs = liveAllocs.filter(a => a.phase === '3c_all');
            const phase4Allocs = liveAllocs.filter(a => a.phase === '4_all_cleanup');

            const sumAllocs = (arr) => arr.reduce((s, a) => s + a.amount, 0);
            const sumPhase1a = sumAllocs(phase1aAllocs);
            const sumPhase1b = sumAllocs(phase1bAllocs);
            const sumPhase1c = sumAllocs(phase1cAllocs);
            const sumPhase2 = sumAllocs(phase2Allocs);
            const sumPhase3a = sumAllocs(phase3aAllocs);
            const sumPhase3b = sumAllocs(phase3bAllocs);
            const sumPhase3c = sumAllocs(phase3cAllocs);
            const sumPhase4 = sumAllocs(phase4Allocs);
            const totalWildToColored = sumPhase1b + sumPhase1c + sumPhase2;
            const totalWildToH00 = sumPhase3b + sumPhase3c;

            const afterDisplay = Array.isArray(live.spare) ? live.spare : [0,0,0,0,0,0,0,0];
            const beforeDisplay = liveIdx === 0
                ? [...totalHearts]
                : (Array.isArray(lives[liveIdx - 1].spare) ? [...lives[liveIdx - 1].spare] : [...totalHearts]);

            const beforeSum = sumHearts(beforeDisplay);
            const afterSum = sumHearts(afterDisplay);
            const consumedArr = beforeDisplay.map((v, i) => v - afterDisplay[i]);
            const consumedSum = sumHearts(consumedArr);
            const totalShort = Math.max(0, reqSum - consumedSum);

            const detail1a = phase1aAllocs.map(a => `${a.amount}×${HEART_LABELS[a.color] || a.color}`).join(', ');
            const detail1b = phase1bAllocs.map(a => `${a.amount}×${HEART_LABELS[a.color] || a.color}`).join(', ');
            const detail1c = phase1cAllocs.map(a => `${a.amount}×${HEART_LABELS[a.color] || a.color}`).join(', ');
            const detail2 = phase2Allocs.map(a => `${a.amount}×${HEART_LABELS[a.color] || a.color}`).join(', ');
            const detail3a = phase3aAllocs.map(a => {
                const srcIcon = `img/texticon/heart_0${a.color}.png`;
                return `${a.amount}×<img src="${srcIcon}" class="heart-mini-icon"> → <img src="img/texticon/heart_00.png" class="heart-mini-icon"> Any`;
            }).join(', ');
            const detail3b = phase3bAllocs.map(a => `${a.amount}×${HEART_LABELS[a.color] || a.color}`).join(', ');
            const detail3c = phase3cAllocs.map(a => `${a.amount}×${HEART_LABELS[a.color] || a.color}`).join(', ');
            const detail4 = phase4Allocs.map(a => {
                if (a.wildcard) {
                    const targetIcon = `img/texticon/heart_0${a.color}.png`;
                    return `${a.amount}×<img src="img/texticon/icon_all.png" class="heart-mini-icon"> → <img src="${targetIcon}" class="heart-mini-icon"> ${HEART_LABELS[a.color]}`;
                }
                return `${a.amount}×<img src="img/texticon/icon_all.png" class="heart-mini-icon"> → <img src="img/texticon/heart_00.png" class="heart-mini-icon"> Any`;
            }).join(', ');

            const colDeficit = [1,2,3,4,5,6].reduce((sum, c) => sum + Math.max(0, (req[c] || 0) - (live.filled?.[c] || 0)), 0);

            html += `
                <div class="perf-agg-card ${passed ? 'success' : 'failure'}">
                    <div class="perf-agg-card-head">
                        <strong>${escapeHtml(liveName)}</strong>
                        <span class="perf-status-pill tiny ${passed ? 'success' : 'failure'}">${passed ? 'PASS' : 'FAIL'}</span>
                    </div>
                    <div class="perf-agg-card-require">${tr('perf_agg_need')} ${renderHeartsCompact(req)} = ${reqSum}</div>
                    <div class="perf-agg-card-pool">${tr('perf_agg_before')}: ${renderHeartsCompact(beforeDisplay)} = ${beforeSum}</div>
                    <div class="perf-agg-card-pool consumed">${tr('perf_agg_used')}: ${renderHeartsCompact(consumedArr)} = ${consumedSum}${totalShort > 0 ? ` <span class="perf-agg-fail">(${tr('perf_short_of', { short: totalShort, needed: reqSum })})</span>` : ''}</div>
                    <div class="perf-agg-steps">
                        ${colorReqSum > 0 ? `
                        <div class="perf-agg-step ${sumPhase1a >= colorReqSum ? (colDeficit === 0 ? 'done' : (totalWildToColored > 0 ? 'done' : 'fail')) : 'fail'}">
                            <span class="perf-agg-marker">①</span>
                            <span>${tr('perf_agg_colored_to_colored')}</span>
                            <span class="perf-agg-step-stat">${sumPhase1a}/${colorReqSum}${sumPhase1a < colorReqSum ? ` <span class="perf-agg-fail">(${colorReqSum - sumPhase1a} ${tr('perf_short')})</span>` : ''}</span>
                            ${detail1a ? `<div class="perf-agg-alloc-detail">${detail1a}</div>` : ''}
                        </div>` : ''}
                        ${totalWildToColored > 0 ? `
                        <div class="perf-agg-step done">
                            <span class="perf-agg-marker">①b</span>
                            <span>${tr('perf_agg_wildcards_to_color')}</span>
                            <span class="perf-agg-step-stat">+${totalWildToColored}</span>
                            ${detail1b ? `<div class="perf-agg-alloc-detail"><img src="img/texticon/heart_00.png" class="heart-mini-icon"> ${tr('perf_wild_any')}: ${detail1b}</div>` : ''}
                            ${detail1c ? `<div class="perf-agg-alloc-detail"><img src="img/texticon/icon_all.png" class="heart-mini-icon"> ${tr('perf_wild_all')}: ${detail1c}</div>` : ''}
                            ${detail2 ? `<div class="perf-agg-alloc-detail">${tr('perf_wild_pool')}: ${detail2}</div>` : ''}
                        </div>` : ''}
                        ${wildReq > 0 ? `
                        <div class="perf-agg-step ${(sumPhase3a + totalWildToH00) >= wildReq ? 'done' : 'fail'}">
                            <span class="perf-agg-marker">③a</span>
                            <span>${tr('perf_agg_colored_surplus_to_h00')}</span>
                            <span class="perf-agg-step-stat">${sumPhase3a}/${wildReq}${sumPhase3a < wildReq ? ` <span class="perf-agg-fail">(${wildReq - sumPhase3a} ${tr('perf_remaining')})</span>` : ''}</span>
                            ${detail3a ? `<div class="perf-agg-alloc-detail">${detail3a}</div>` : ''}
                        </div>` : ''}
                        ${wildReq > 0 && totalWildToH00 > 0 ? `
                        <div class="perf-agg-step ${(sumPhase3a + totalWildToH00) >= wildReq ? 'done' : 'fail'}">
                            <span class="perf-agg-marker">③b</span>
                            <span>${tr('perf_agg_wildcards_to_h00')}</span>
                            <span class="perf-agg-step-stat">+${totalWildToH00}</span>
                            ${detail3b ? `<div class="perf-agg-alloc-detail"><img src="img/texticon/heart_00.png" class="heart-mini-icon"> ${tr('perf_wild_any')}: ${detail3b}</div>` : ''}
                            ${detail3c ? `<div class="perf-agg-alloc-detail"><img src="img/texticon/icon_all.png" class="heart-mini-icon"> ${tr('perf_wild_all')}: ${detail3c}</div>` : ''}
                        </div>` : ''}
                        ${sumPhase4 > 0 ? `
                        <div class="perf-agg-step done">
                            <span class="perf-agg-marker">④</span>
                            <span><img src="img/texticon/icon_all.png" class="heart-mini-icon"> ${tr('perf_agg_icon_all_cleanup')}</span>
                            <span class="perf-agg-step-stat">${sumPhase4}</span>
                            ${detail4 ? `<div class="perf-agg-alloc-detail">${detail4}</div>` : ''}
                        </div>` : ''}
                    </div>
                    <div class="perf-agg-card-after">${tr('perf_agg_after')}: ${renderHeartsCompact(afterDisplay)} = ${afterSum}</div>
                </div>`;
        }
    }

    // Surplus = use snapshot surplus_hearts if available, fall back to spare
    const finalRemaining = (result.surplus_hearts && Array.isArray(result.surplus_hearts))
        ? result.surplus_hearts
        : (lives.length > 0
            ? (Array.isArray(lives[lives.length - 1].spare) ? lives[lives.length - 1].spare : [0,0,0,0,0,0,0,0])
            : [...totalHearts]);
    const surplusTotal = sumHearts(finalRemaining);
    html += `
                <div class="perf-agg-divider"></div>
                <div class="perf-agg-row surplus ${surplusTotal > 0 ? 'positive' : 'zero'}">
                    <span class="perf-agg-label">${tr('perf_surplus')}</span>
                    ${renderHeartsCompact(finalRemaining)}
                    <span class="perf-agg-surplus-value">${surplusTotal > 0 ? '+' : ''}${surplusTotal}</span>
                </div>`;

    // Check for surplus removal effects (LiveSuccess abilities like Kowareyasuki)
    const tempEffects = State.data?.temporary_effects || [];
    const surplusEffects = tempEffects.filter(te =>
        te.effect_type === 'gain_surplus_heart'
        && te.effect_data?.target
        && te.effect_data?.old_value > 0
    );
    for (const se of surplusEffects) {
        const lostAmount = se.effect_data.old_value;
        const target = se.effect_data.target; // "opponent" or "self"
        html += `
                <div class="perf-agg-row surplus-removed">
                    <span class="perf-agg-label">${tr('perf_surplus_removed', { target: tr(target) })}</span>
                    <span class="perf-agg-surplus-value negative">-${lostAmount}</span>
                </div>`;
    }

    html += `
            </div>
        </div>`;

    return html;
}

function renderTurnNavigation() {
    if (!State.performanceHistoryTurns || State.performanceHistoryTurns.length <= 1) {
        return '';
    }

    const turns = [...State.performanceHistoryTurns].sort((left, right) => left - right);
    return `
        <div class="perf-turn-nav">
            ${turns.map((turn) => {
                const latestTurn = turns[turns.length - 1];
                const isLatest = turn === latestTurn;
                const isSelected = State.selectedPerfTurn === turn || (State.selectedPerfTurn === -1 && isLatest);
                const label = isLatest
                    ? tr('current_turn', { turn })
                    : tr('turn_label', { turn });
                return `<button class="perf-nav-btn ${isSelected ? 'active' : ''}" data-action="show-performance-turn" data-value="${turn}">${escapeHtml(label)}</button>`;
            }).join('')}
        </div>
    `;
}

function renderPerfSteps(result) {
    if (!result) return `<div class="perf-empty-state">${tr('perf_no_data')}</div>`;

    const fmtH = (arr) => arr ? arr.map((v,i) => v > 0 ? `${HEART_LABELS[i]}:${v}` : null).filter(Boolean).join(' ') : 'none';
    const fmtHeartIcon = (i) => i === 0 ? 'img/texticon/heart_00.png' : i === 7 ? 'img/texticon/icon_all.png' : `img/texticon/heart_0${i}.png`;
    const fmtHShortReq = (arr) => arr ? arr.map((v,i) => v > 0 ? `<img src="${fmtHeartIcon(i)}" class="heart-mini-icon">${v}` : '').join('') : '';
    const fmtHShortSrc = (arr) => arr ? arr.map((v,i) => v > 0 ? `<img src="${fmtHeartIcon(i)}" class="heart-mini-icon">${v}` : '').join('') : '';

    const totalBlades = (result.member_contributions || []).reduce((s, m) => m.is_wait ? s : s + m.base_blades + m.bonus_blades, 0);
    const passedLives = (result.lives || []).filter(l => l.passed).length;
    const baseLiveScore = (result.lives || []).reduce((s, l) => l.passed ? s + l.score : s, 0);
    const baseRawScore = (result.lives || []).reduce((s, l) => l.passed ? s + (l.base_score || 0) : s, 0);

    return `
        <section class="perf-steps-all">
            <div class="perf-section-heading-row compact">
                <div>
                    <div class="perf-eyebrow">${tr('perf_engine_header')}</div>
                </div>
            </div>

            <!-- Step 1: Live Zone -->
            <details class="perf-step-detail" open>
                <summary class="perf-step-summary">${tr('perf_engine_live_zone', { count: result.lives?.length || 0 })}</summary>
                <div class="perf-step-body">
                    ${(result.lives || []).map((live, i) => {
                        const cardData = live.card_no ? State.resolveCardData(live.card_no) : null;
                        const imgSrc = cardData ? fixImg(cardData.img || '') : '';
                        return `
                            <div class="perf-step-live-card">
                                ${imgSrc ? `<div class="card card-micro md"><img src="${imgSrc}"></div>` : ''}
                                <div class="perf-step-live-info">
                                    <div>${tr('perf_require')}: ${fmtHShortReq(live.required)}</div>
                                    <div>${tr('perf_score')}: ${live.score}</div>
                                </div>
                            </div>
                        `;
                    }).join('') || `<div class="perf-empty-state small">${tr('perf_no_live_cards')}</div>`}
                    <div class="perf-step-note">${tr('perf_note_live_zone')}</div>
                </div>
            </details>

            <!-- Step 2: Live Start Triggers -->
            <details class="perf-step-detail" open>
                <summary class="perf-step-summary">${tr('perf_engine_triggers', { count: result.triggered_abilities?.length || 0 })}</summary>
                <div class="perf-step-body">
                    ${(result.triggered_abilities || []).map(t => {
                        const cd = State.resolveCardData(t.source_card_id);
                        return `<div class="perf-step-trigger">${escapeHtml(cd?.name || t.card_name || '?')}: ${escapeHtml(t.name || 'triggered')}</div>`;
                    }).join('') || `<div class="perf-empty-state small">${tr('perf_no_triggers_step')}</div>`}
                    <div class="perf-step-note">${tr('perf_note_triggers')}</div>
                </div>
            </details>

            <!-- Step 3: Blades + Yell -->
            <details class="perf-step-detail" open>
                <summary class="perf-step-summary">${tr('perf_engine_blades_yell', { blades: totalBlades, yell: result.yell_count || 0 })}</summary>
                <div class="perf-step-body">
                    <div class="perf-step-members">
                        ${(result.member_contributions || []).map(m => {
                            const cd = m.card_no ? State.resolveCardData(m.card_no) : null;
                            const imgSrc = cd ? fixImg(cd.img || '') : '';
                            const isWait = m.is_wait;
                            return `
                                <div class="perf-step-member${isWait ? ' perf-dimmed' : ''}">
                                    ${imgSrc ? `<img src="${imgSrc}" class="perf-step-member-img">` : ''}
                                    <div>${tr('perf_blade')}: ${isWait ? `0 ${tr('perf_negated')}` : `${m.base_blades}${m.bonus_blades > 0 ? '+' + m.bonus_blades : ''}`} ${isWait ? `<span class="perf-wait-badge">${tr('perf_wait')}</span>` : ''}</div>
                                    <div>${fmtHShortSrc(m.base_hearts)}</div>
                                </div>
                            `;
                        }).join('')}
                    </div>
                    <div class="perf-step-note">${tr('perf_note_blades')}</div>
                </div>
            </details>

            <!-- Step 4: Stage Hearts -->
            <details class="perf-step-detail" open>
                <summary class="perf-step-summary">${tr('perf_engine_stage_hearts', { hearts: fmtH(result.total_hearts) })}</summary>
                <div class="perf-step-body">
                    <div class="perf-step-hearts-row">
                          ${result.total_hearts ? HEART_LABELS.map((_, i) =>
                              result.total_hearts[i] > 0
                                 ? `<span class="perf-step-heart-cell"><img src="img/texticon/heart_0${i}.png" class="heart-mini-icon"> ${result.total_hearts[i]}</span>`
                                : ''
                        ).join('') : ''}
                    </div>
                    <div class="perf-step-note">${tr('perf_note_hearts')}</div>
                </div>
            </details>

            <!-- Step 5: Yell Cards -->
            <details class="perf-step-detail" open>
                <summary class="perf-step-summary">${tr('perf_engine_yell_cards', { count: result.yell_cards?.length || 0 })}</summary>
                <div class="perf-step-body">
                    <div class="perf-step-yells">
                        ${(result.yell_cards || []).map(y => {
                            const cd = y.card_no ? State.resolveCardData(y.card_no) : null;
                            const imgSrc = cd ? fixImg(cd.img || '') : '';
                            return `
                                <div class="perf-step-yell-card">
                                    ${imgSrc ? `<img src="${imgSrc}" class="perf-step-card-img-sm">` : ''}
                                    <div>${fmtHShortSrc(y.blade_hearts)}</div>
                                    <div><img src="img/texticon/icon_score.png" class="heart-mini-icon">${y.note_icons} <img src="img/texticon/icon_draw.png" class="heart-mini-icon">${y.draw_icons}</div>
                                </div>
                            `;
                        }).join('') || `<div class="perf-empty-state small">${tr('perf_no_yell_cards')}</div>`}
                    </div>
                    <div class="perf-step-note">${tr('perf_note_yell')}</div>
                </div>
            </details>

            <!-- Step 6: Color Transforms -->
            <details class="perf-step-detail">
                <summary class="perf-step-summary">${tr('perf_engine_transforms', { count: result.breakdown?.transforms?.length || 0 })}</summary>
                <div class="perf-step-body">
                    ${(result.breakdown?.transforms || []).map(t =>
                        `<div class="perf-step-transform">${escapeHtml(t.source)}: ${escapeHtml(t.desc)}</div>`
                    ).join('') || `<div class="perf-empty-state small">${tr('perf_no_transforms')}</div>`}
                    <div class="perf-step-note">${tr('perf_note_transforms')}</div>
                </div>
            </details>

            <!-- Step 7: Requirements Modifiers -->
            <details class="perf-step-detail">
                <summary class="perf-step-summary">${tr('perf_engine_requirements', { count: result.breakdown?.requirements?.length || 0 })}</summary>
                <div class="perf-step-body">
                    ${(result.breakdown?.requirements || []).map(r =>
                        `<div class="perf-step-req">${escapeHtml(r.source)}: ${escapeHtml(r.desc)}</div>`
                    ).join('') || `<div class="perf-empty-state small">${tr('perf_no_requirement_changes')}</div>`}
                    <div class="perf-step-note">${tr('perf_note_requirements')}</div>
                </div>
            </details>

            <!-- Step 8: Judge Each Live -->
            <details class="perf-step-detail" open>
                <summary class="perf-step-summary">${tr('perf_engine_judge', { passed: passedLives, total: result.lives?.length || 0 })}</summary>
                <div class="perf-step-body">
                    ${(result.lives || []).map((live, i) => {
                        const cardData = live.card_no ? State.resolveCardData(live.card_no) : null;
                        const imgSrc = cardData ? fixImg(cardData.img || '') : '';
                        const failedReason = live.adjustments?.filter(a => a.adjustment_type === 'failure') || [];
                        return `
                            <div class="perf-step-judge ${live.passed ? 'pass' : 'fail'}">
                                <div class="perf-step-judge-header">
                                    ${imgSrc ? `<div class="card card-micro sm"><img src="${imgSrc}"></div>` : ''}
                                    <span>${tr('perf_slot', { n: i })}: <b>${live.passed ? '✓ PASS' : '✗ FAIL'}</b> ${tr('perf_score')} +${live.score}</span>
                                </div>
                                <div class="perf-step-judge-detail">
                                    ${tr('perf_need')} ${fmtHShortReq(live.required)} / ${tr('perf_filled')} ${fmtHShortSrc(live.filled)} / ${tr('perf_spare')} ${fmtHShortSrc(live.spare)}
                                </div>
                                ${failedReason.map(a => `<div class="perf-step-fail-reason">${escapeHtml(a.desc)}</div>`).join('')}
                            </div>
                        `;
                    }).join('') || `<div class="perf-empty-state small">${tr('perf_no_live_cards')}</div>`}
                    <div class="perf-step-note">${tr('perf_note_judge')}</div>
                </div>
            </details>

            <!-- Step 9: Score + Winner -->
            <details class="perf-step-detail" open>
                <summary class="perf-step-summary">${tr('perf_engine_result', { score: result.total_score || 0 })} ${result.success ? '✓ PASS' : '✗ FAIL'}</summary>
                <div class="perf-step-body">
                    <div class="perf-step-result-row">
                        <div class="perf-step-result-item">
                            <img src="img/texticon/icon_score.png" class="heart-mini-icon">
                            ${tr('perf_base_score')}: ${baseRawScore}
                        </div>
                        <div class="perf-step-result-item">
                            <img src="img/texticon/icon_score.png" class="heart-mini-icon">
                            ${tr('perf_triggered_bonuses')}: ${baseLiveScore - baseRawScore > 0 ? '+' : ''}${baseLiveScore - baseRawScore}
                        </div>
                        <div class="perf-step-result-item total">
                            ${tr('perf_total')}: <b>${result.total_score || 0}</b>
                        </div>
                        <div class="perf-step-result-item outcome ${result.success ? 'success' : 'failure'}">
                            ${result.success ? '✓ PASS' : '✗ FAIL'}
                            ${result.p0_wins ? ` — ${tr('perf_p1_wins')}` : ''}
                            ${result.p1_wins ? ` — ${tr('perf_p2_wins')}` : ''}
                            ${result.p0_wins && result.p1_wins ? ` — ${tr('perf_draw')}` : ''}
                        </div>
                    </div>
                    <div class="perf-step-note">${tr('perf_note_result')}</div>
                </div>
            </details>
        </section>
    `;
}

function renderComparisonBanner(displayResults) {
    const p0 = displayResults?.[0];
    const p1 = displayResults?.[1];
    if (!p0 && !p1) return '';

    const p0Wins = !!p0?.p0_wins;
    const p1Wins = !!p0?.p1_wins || !!p1?.p1_wins;
    let summary;
    if (p0Wins && p1Wins) {
        summary = tr('perf_comp_tie');
    } else if (p0Wins) {
        summary = tr('perf_comp_won', { name: getPlayerName(0) });
    } else if (p1Wins) {
        summary = tr('perf_comp_won', { name: getPlayerName(1) });
    } else if (p0?.success || p1?.success) {
        summary = tr('perf_comp_no_winner');
    } else {
        summary = tr('perf_comp_no_check');
    }

    return `
        <section class="perf-comparison-banner" style="padding: 8px 12px; margin-bottom: 4px;">
            <div class="perf-comparison-copy" style="font-size: 0.9rem;"><b>${tr('perf_result_label')}:</b> ${escapeHtml(summary)}</div>
        </section>
    `;
}

function renderTotalSection(result) {
    if (!result) return '';

    const totalHearts = result.total_hearts || [0,0,0,0,0,0,0,0];

    return `
        <section class="perf-section-card">
            <div class="perf-section-heading-row compact">
                <div>
                    <div class="perf-eyebrow">${tr('perf_total_hearts_title')}</div>
                </div>
            </div>
            <div class="perf-total-breakdown">
                <div class="perf-breakdown-row grand">
                    <span class="perf-mini-heading">${tr('perf_stage_yell')}</span>
                    ${renderHeartsCompact(totalHearts)}
                    <span class="perf-breakdown-sum">${sumHearts(totalHearts)}</span>
                </div>
            </div>
        </section>
    `;
}

function renderLiveCards(result) {
    const lives = Array.isArray(result?.lives) ? result.lives : [];
    const triggered = result?.triggered_abilities || [];
    const noLives = lives.length === 0;

    // Collect triggered abilities already shown in per-member bonuses
    // so global-bonuses only shows the ones not claimed by a specific card.
    const claimedTexts = new Set();
    if (!noLives) {
        for (const live of lives) {
            for (const sLine of (result?.breakdown?.scores || []).filter(s => s.value > 0)) {
                const srcAbility = findAbilitySource(triggered, sLine.source);
                if (srcAbility?.effect_text) claimedTexts.add(srcAbility.effect_text);
            }
            for (const adj of (live.adjustments || [])) {
                const adjAbility = findAbilitySource(triggered, adj?.source || '');
                if (adjAbility?.effect_text) claimedTexts.add(adjAbility.effect_text);
            }
        }
        // Check per-member heart/blade bonuses (shown in renderContributionSection)
        for (const member of (result?.member_contributions || [])) {
            for (const hb of (member.ability_heart_bonuses || [])) {
                if (hb?.ability_text) claimedTexts.add(hb.ability_text);
            }
            for (const bb of (member.ability_blade_bonuses || [])) {
                if (bb?.ability_text) claimedTexts.add(bb.ability_text);
            }
        }
    }
    const globalTriggered = triggered.filter(t => !t.effect_text || !claimedTexts.has(t.effect_text));

    // Revealed member cards from yell (card images only)
    const revealedIds = Array.isArray(result?.revealed_ids) ? result.revealed_ids : [];
    const revealedMembers = revealedIds
        .map(id => State.resolveCardData(id))
        .filter(cd => cd && (cd.card_type === 'Member' || cd.type === 'member'));
    const memberImgSection = revealedMembers.length > 0 ? `
        <div class="perf-revealed-members">
            <div class="perf-mini-heading" style="margin: 6px 0 4px;">${tr('perf_revealed_members')}</div>
            <div class="perf-revealed-grid">
                ${revealedMembers.map(cd => `
                    <div class="perf-revealed-member-card">
                        ${cd.img ? `<img src="${fixImg(cd.img)}" class="perf-live-art" alt="${escapeHtml(cd.name || '')}">` : ''}
                    </div>
                `).join('')}
            </div>
        </div>
    ` : '';

    return `
        <section class="perf-section-card">
            <div class="perf-section-heading-row compact">
                <div>
                    <div class="perf-eyebrow">${tr('perf_live_checks')}</div>
                </div>
            </div>
            ${noLives ? `<div class="perf-empty-state">${tr('perf_no_live_snapshot')}</div>` : ''}
            ${renderAggregateHeartSummary(result)}
            ${memberImgSection}
            <div class="perf-live-grid">
                ${noLives ? '' : lives.map((live, index) => {
                    const cd = live.card_no ? State.resolveCardData(live.card_no) : null;
                    const required = live?.required || [0,0,0,0,0,0,0,0];
                    const filled = live?.filled || [0,0,0,0,0,0,0,0];
                    const spare = live?.spare || [0,0,0,0,0,0,0,0];
                    const adjustments = live.adjustments;
                    const baseScore = live?.base_score || 0;
                    const totalScore = live?.score || 0;
                    const bonusScore = totalScore > baseScore ? totalScore - baseScore : 0;
                    return `
                        <article class="perf-live-card ${live?.passed ? 'success' : 'failure'}">
                            <div class="perf-live-card-head">
                                <div class="perf-card-id-badge">Live ${index + 1}</div>
                                <div class="perf-live-card-title">
                                    ${cd?.img ? `<div class="card card-micro lg"><img src="${fixImg(cd.img)}" alt="${escapeHtml(cd?.name || 'Live')}"></div>` : ''}
                                    <div>
                                        <h4>${escapeHtml(cd?.name || 'Live')}</h4>
                                        <div class="perf-breakdown-row total">
                                            <span class="perf-mini-heading"><img src="img/texticon/icon_score.png" class="heart-mini-icon"> ${tr('perf_score')}</span>
                                            <span class="perf-breakdown-detail">${tr('perf_base')} ${baseScore}</span>
                                            ${bonusScore > 0 ? `<span class="perf-breakdown-detail">+${bonusScore} ${tr('perf_abilities')}</span>` : ''}
                                            <span class="perf-breakdown-sum">${totalScore}</span>
                                        </div>
                                        ${bonusScore > 0 ? `
                                        <div class="perf-breakdown-bonuses">
                                            ${(result?.breakdown?.scores || []).filter(s => s.value > 0).map((sLine) => {
                                                const srcAbility = findAbilitySource(triggered, sLine.source);
                                                const sourceLabel = srcAbility ? `${escapeHtml(srcAbility.card_name || '')}` : escapeHtml(sLine.source);
                                                return `
                                                    <div class="perf-bonus-item compact">
                                                        <div class="perf-bonus-title"><img src="img/texticon/icon_score.png" class="heart-mini-icon"> ${sourceLabel} +${sLine.value}</div>
                                                        ${srcAbility?.effect_text ? `<div class="perf-bonus-text">${enrichText(srcAbility.effect_text)}</div>` : ''}
                                                    </div>
                                                `;
                                            }).join('')}
                                        </div>
                                        ` : ''}
                                    </div>
                                </div>
                                <div class="perf-status-pill ${live?.passed ? 'success' : 'failure'}">${live?.passed ? 'PASS' : 'FAIL'}</div>
                            </div>
                            <div class="perf-live-breakdown">
                                <div class="perf-breakdown-row">
                                    <span class="perf-mini-heading">${tr('perf_required')}</span>
                                    ${renderHeartsCompact(required)}
                                    <span class="perf-breakdown-sum">${sumHearts(required)}</span>
                                </div>
                                <div class="perf-breakdown-row">
                                    <span class="perf-mini-heading">${tr('perf_filled')}</span>
                                    ${renderHeartsCompact(filled)}
                                    <span class="perf-breakdown-sum">${sumHearts(filled)}</span>
                                </div>
                                <div class="perf-breakdown-row">
                                    <span class="perf-mini-heading">${tr('perf_remaining')}</span>
                                    ${renderHeartsCompact(spare)}
                                    <span class="perf-breakdown-sum">${sumHearts(spare)}</span>
                                </div>
                            </div>
                            ${required[0] > 0 ? `<div class="perf-heart-legend" style="font-size:0.65rem;color:var(--text-muted);margin-top:2px;"><img src="img/texticon/heart_00.png" class="heart-mini-icon" style="width:12px;height:12px;"> ${tr('perf_heart_legend')}</div>` : ''}
                            ${adjustments && adjustments.length > 0 ? `
                                <div class="perf-pill-list">
                                    ${adjustments.map((adj) => {
                                        const isTransform = adj?.type === 'transform' || adj?.type === 'override';
                                        const src = adj?.source || '';
                                        const cardName = src.includes(' req modifier ') ? src.split(' req modifier ')[0] : src;
                                        const adjDesc = (adj?.desc || '').replace(/\s+\(add|sub|set\)$/, '');
                                        const adjText = adjDesc || `${adj?.value > 0 ? '+' : ''}${adj?.value || 0} ${HEART_LABELS[adj?.color ?? 0] || 'heart'}`;
                                        return `<div class="perf-adjustment-pill ${isTransform ? 'transform' : 'requirement'}">${escapeHtml(cardName || 'Effect')}: ${escapeHtml(adjText)}</div>`;
                                    }).join('')}
                                </div>
                            ` : ''}
                        </article>
                    `;
                }).join('')}
            </div>
        </section>
    `;
}


function renderContributionSection(result) {
    if (!result?.member_contributions) return '';
    const members = result.member_contributions;
    const triggered = result.triggered_abilities || [];

    // Filter triggered abilities to those not shown in per-member bonuses
    const claimedTexts = new Set();
    for (const member of members) {
        for (const hb of (member.ability_heart_bonuses || [])) {
            if (hb?.ability_text) claimedTexts.add(hb.ability_text);
        }
        for (const bb of (member.ability_blade_bonuses || [])) {
            if (bb?.ability_text) claimedTexts.add(bb.ability_text);
        }
    }
    const globalTriggered = triggered.filter(t => !t.effect_text || !claimedTexts.has(t.effect_text));

    if (members.length === 0 && triggered.length === 0) {
        return `
            <section class="perf-section-card">
                <div class="perf-section-heading-row compact">
                    <div>
                        <div class="perf-eyebrow">${tr('perf_stage_contributors')}</div>
                    </div>
                </div>
                <div class="perf-empty-state">${tr('perf_no_contributors')}</div>
            </section>
        `;
    }

    const slotLabels = [tr('area_left'), tr('area_center'), tr('area_right')];

    const rendered = members.map((member) => {
        const base = member.base_hearts || [0,0,0,0,0,0,0,0];
        const bonus = member.bonus_hearts || [0,0,0,0,0,0,0,0];
        const isWait = member.is_wait;
        const totalBlade = isWait ? 0 : (member.base_blades || 0) + (member.bonus_blades || 0);
        const heartBonuses = member.ability_heart_bonuses || [];
        const bladeBonuses = member.ability_blade_bonuses || [];
        const slot = member.slot !== undefined && member.slot >= 0 && member.slot < 3
            ? slotLabels[member.slot] : `Slot ${(member.slot ?? -1) + 1}`;
        const memberImg = member.card_no ? (() => { const cd = State.resolveCardData(member.card_no); return cd?.img ? fixImg(cd.img) : ''; })() : '';
        const memberName = member.card_no ? (() => { const cd = State.resolveCardData(member.card_no); return cd?.name || member?.source || 'Member'; })() : (member?.source || 'Member');

        // Step 1: base_hearts — the card's original hearts
        // Step 2: transform changes the color of all base hearts (no addition/subtraction)
        // Step 3: ability bonuses add hearts on top
        // bonus_hearts = transform_delta + ability_total per color
        // So: transform_delta = bonus_hearts - ability_total
        const abilityPerColor = [0,0,0,0,0,0,0,0];
        for (const ab of heartBonuses) {
            if (ab.color !== undefined && ab.color >= 0 && ab.color < 8) {
                abilityPerColor[ab.color] += ab.amount;
            }
        }
        const transformDelta = bonus.map((v, i) => v - abilityPerColor[i]);
        const afterTransform = base.map((v, i) => v + transformDelta[i]);
        const total = base.map((v, i) => v + bonus[i]);

        return `
            <article class="perf-contrib-card${isWait ? ' perf-contrib-wait' : ''}" data-member-id="${member?.source_id ?? ''}" data-member-slot="${member?.slot ?? ''}">
                <div class="perf-contrib-header">
                    ${memberImg ? `<img src="${memberImg}" class="perf-contrib-art" alt="${escapeHtml(memberName)}">` : ''}
                    <div>
                        <h4>${escapeHtml(memberName)}${isWait ? ` <span class="perf-wait-badge">${tr('perf_wait')}</span>` : ''}</h4>
                        <div class="perf-breakdown-row total">
                            <span class="perf-mini-heading">${tr('perf_total_hearts')}</span>
                            ${renderHeartsCompact(total)}
                            <span class="perf-breakdown-sum">${sumHearts(total)}</span>
                        </div>
                    </div>
                </div>
                <div class="perf-stage-breakdown">
                    <div class="perf-breakdown-subrows">
                        <div class="perf-breakdown-row sub">
                            <span class="perf-mini-heading">① ${tr('perf_base_hearts')}</span>
                            ${renderHeartsCompact(base)}
                            <span class="perf-breakdown-sum">${sumHearts(base)}</span>
                        </div>
                        ${base.some((v, i) => v !== afterTransform[i]) ? `
                        <div class="perf-breakdown-row sub">
                            <span class="perf-mini-heading">② ${tr('perf_after_transform')}</span>
                            ${renderHeartsCompact(afterTransform)}
                            <span class="perf-breakdown-sum">${sumHearts(afterTransform)}</span>
                        </div>
                        ` : ''}
                        ${transformDelta.some(v => v !== 0) ? `
                        <div class="perf-breakdown-row sub">
                            <span class="perf-mini-heading">③ ${tr('perf_transform_delta')}</span>
                            ${renderHeartsCompact(transformDelta)}
                        </div>
                        ` : ''}
                        ${heartBonuses.length > 0 ? `
                        <div class="perf-breakdown-bonuses">
                            ${heartBonuses.map((b) => `
                                <div class="perf-bonus-item compact">
                                    <div class="perf-bonus-title">${escapeHtml(b?.source || 'Effect')} +${b?.amount || 0} ${escapeHtml(HEART_LABELS[b?.color ?? 0] || 'heart')}</div>
                                    ${b?.ability_text ? `<div class="perf-bonus-text">${enrichText(b.ability_text)}</div>` : ''}
                                </div>
                            `).join('')}
                        </div>
                        ` : ''}
                    </div>
                    <div class="perf-breakdown-row${isWait ? ' perf-dimmed' : ''}">
                        <span class="perf-mini-heading">${tr('perf_blades')}</span>
                        ${renderBladesCompact(totalBlade)}
                        ${!isWait && (member.bonus_blades || 0) > 0 ? `<span class="perf-breakdown-detail">(+${member.bonus_blades} ${tr('perf_from_abilities')})</span>` : ''}
                        ${isWait ? `<span class="perf-breakdown-detail">${tr('perf_negated_wait')}</span>` : ''}
                        ${!isWait && bladeBonuses.length > 0 ? `
                        <div class="perf-breakdown-bonuses">
                            ${bladeBonuses.map((bonus) => `
                                <div class="perf-bonus-item compact">
                                    <div class="perf-bonus-title">${escapeHtml(bonus?.source || 'Effect')} +${bonus?.amount || bonus?.value || 0} blade</div>
                                    ${bonus?.ability_text ? `<div class="perf-bonus-text">${enrichText(bonus.ability_text)}</div>` : ''}
                                </div>
                            `).join('')}
                        </div>
                        ` : ''}
                    </div>
                    <div class="perf-breakdown-row minor">
                        <span class="perf-mini-heading"><img src="img/texticon/icon_score.png" class="heart-mini-icon"> ${tr('perf_notes')}</span>
                        <span class="perf-breakdown-value">${member?.base_notes || 0}${member?.bonus_notes ? ` (+${member.bonus_notes})` : ''}</span>
                        <span style="margin-left:12px;" class="perf-mini-heading"><img src="img/texticon/icon_draw.png" class="heart-mini-icon"> ${tr('perf_draw')}</span>
                        <span class="perf-breakdown-value">${member?.draw_icons || 0}</span>
                    </div>
                </div>
            </article>
        `;
    });

    // Compute totals across all 3 slots
    const grandTotal = [0,0,0,0,0,0,0,0];
    let grandBlade = 0, grandNotes = 0, grandDraw = 0;
    for (const m of members) {
        const b = m.base_hearts || [0,0,0,0,0,0,0,0];
        const bn = m.bonus_hearts || [0,0,0,0,0,0,0,0];
        for (let i = 0; i < 8; i++) grandTotal[i] += b[i] + bn[i];
        if (!m.is_wait) grandBlade += (m.base_blades || 0) + (m.bonus_blades || 0);
        grandNotes += (m.base_notes || 0) + (m.bonus_notes || 0);
        grandDraw += m.draw_icons || 0;
    }

    return `
        <section class="perf-section-card">
            <div class="perf-section-heading-row compact">
                <div>
                    <div class="perf-eyebrow">${tr('perf_stage_contributors')}</div>
                    ${members.length > 0 ? `<div class="perf-total-badge">${tr('perf_grand_total', { hearts: sumHearts(grandTotal), blades: grandBlade, notes: grandNotes, draw: grandDraw })}</div>` : ''}
                </div>
            </div>
            <div class="perf-contrib-grid">
                ${rendered.join('')}
                ${members.length > 1 ? `
                <article class="perf-contrib-card perf-total-row">
                    <div class="perf-contrib-header">
                        <div>
                            <h4>${tr('perf_total_all_slots')}</h4>
                            <div class="perf-breakdown-row total">
                                <span class="perf-mini-heading">${tr('perf_hearts')}</span>
                                ${renderHeartsCompact(grandTotal)}
                                <span class="perf-breakdown-sum">${sumHearts(grandTotal)}</span>
                            </div>
                        </div>
                    </div>
                    <div class="perf-stage-breakdown">
                        <div class="perf-breakdown-row">
                            <span class="perf-mini-heading">${tr('perf_blades')}</span>
                            <span class="perf-breakdown-value">${grandBlade}</span>
                        </div>
                        <div class="perf-breakdown-row minor">
                            <span class="perf-mini-heading">${tr('perf_notes')}</span>
                            <span class="perf-breakdown-value">${grandNotes}</span>
                            <span style="margin-left:12px;" class="perf-mini-heading">${tr('perf_draw')}</span>
                            <span class="perf-breakdown-value">${grandDraw}</span>
                        </div>
                    </div>
                </article>
                ` : ''}
                ${globalTriggered.length > 0 ? `
                <article class="perf-contrib-card global-bonuses">
                    <div class="perf-contrib-header">
                        <div>
                            <h4>${tr('global_bonuses')}</h4>
                            <div class="perf-breakdown-bonuses">
                                ${globalTriggered.map((ability) => {
                                    const effectText = ability?.effect_text || '';
                                    const condText = ability?.condition_text || '';
                                    const abilityDisplay = effectText ? enrichText(effectText) : '';
                                    const condDisplay = condText ? enrichText(condText) : '';
                                    const triggeredType = effectText.includes('ライブ開始時') || effectText.includes('live_start') ? 'live_start' :
                                        effectText.includes('ライブ成功時') || effectText.includes('live_success') ? 'live_success' :
                                        effectText.includes('常時') || effectText.includes('jyouji') ? 'jyouji' : '';
                                    const durationLabel = {
                                        'live_start': tr('perf_effect_duration_live_start'),
                                        'live_success': tr('perf_effect_duration_live_success'),
                                        'jyouji': tr('perf_effect_duration_jyouji'),
                                    }[triggeredType] || '';
                                    return `
                                        <div class="perf-bonus-item compact">
                                            <div class="perf-bonus-title">${escapeHtml(ability?.card_name || 'Ability')} ${durationLabel ? `<span class="effect-duration">[${escapeHtml(durationLabel)}]</span>` : ''}</div>
                                            ${abilityDisplay ? `<div class="perf-bonus-text">${abilityDisplay}</div>` : ''}
${condDisplay ? `<div class="perf-ability-condition">Cond: ${condDisplay}</div>` : ''}
                                        </div>
                                    `;
                                }).join('')}
                            </div>
                        </div>
                    </div>
                </article>
                ` : ''}
            </div>
        </section>
    `;
}

function renderYellSection(result) {
    const yellCards = result.yell_cards || [];
    const heartSources = result.breakdown?.hearts || [];

    if (yellCards.length === 0 && heartSources.length === 0) {
        return `
            <section class="perf-section-card">
                <div class="perf-section-heading-row compact">
                    <div>
                        <div class="perf-eyebrow">${tr('perf_yell_pool')}</div>
                    </div>
                </div>
                <div class="perf-empty-state">${tr('perf_no_yell_data')}</div>
            </section>
        `;
    }

    // Aggregate per-color total across all yell cards
    const totalYellHearts = [0,0,0,0,0,0,0,0];
    yellCards.forEach(c => {
        const bh = c.blade_hearts || [0,0,0,0,0,0,0,0];
        for (let i = 0; i < 7; i++) totalYellHearts[i] += bh[i];
    });

    // Per-color source: count how many yell cards contribute to each color
    const perColorCount = [0,0,0,0,0,0,0,0];
    yellCards.forEach(c => {
        const bh = c.blade_hearts || [0,0,0,0,0,0,0,0];
        for (let i = 1; i < 7; i++) {
            if (bh[i] > 0) perColorCount[i]++;
        }
    });

    return `
        <section class="perf-section-card">
            <div class="perf-section-heading-row compact">
                <div>
                    <div class="perf-eyebrow">Yell & Source Pool</div>
                </div>
            </div>
            <div class="perf-yell-summary">
                <div class="perf-breakdown-row total">
                    <span class="perf-mini-heading">${tr('perf_total_yell_hearts')}</span>
                    ${renderHeartsCompact(totalYellHearts)}
                    <span class="perf-breakdown-sum">${sumHearts(totalYellHearts)}</span>
                </div>
            </div>
            <div class="perf-yell-gallery">
                ${yellCards.length > 0 ? yellCards.map((card) => {
                    const rawText = Tooltips.getEffectiveRawText(card);
                    return `
                        <article class="perf-yell-card-modern" ${card?.id !== undefined ? `data-card-id="${card.id}"` : ''} ${rawText ? `data-text="${escapeHtml(rawText)}"` : ''}>
                            ${card?.card_no ? `<img src="${resolveCardImagePath(card.card_no)}" alt="Yell card">` : ''}
                            <div class="perf-yell-icons">
                                ${renderHeartsCompact(card?.blade_hearts || [])}
                                ${(card?.note_icons || 0) > 0 ? `<span class="perf-badge note"><img src="img/texticon/icon_score.png" class="heart-mini-icon"> ${card.note_icons}</span>` : ''}
                                ${(card?.draw_icons || 0) > 0 ? `<span class="perf-badge draw"><img src="img/texticon/icon_draw.png" class="heart-mini-icon"> ${card.draw_icons}</span>` : ''}
                            </div>
                        </article>
                    `;
                }).join('') : ''}
            </div>

            ${heartSources.length > 0 ? `
            <div class="perf-source-lists">
                <div>
                    <div class="perf-mini-heading">${tr('perf_heart_sources')}</div>
                    <div class="perf-chip-list">
                        ${heartSources.map((item) => `
                            <div class="perf-source-chip ${item?.source_type === 'yell' ? 'yell' : ''}">
                                <span>${escapeHtml(item?.source || 'Source')}</span>
                                ${renderHeartsCompact(item?.value || [])}
                            </div>
                        `).join('')}
                    </div>
                </div>
            </div>
            ` : ''}
        </section>
    `;
}

function renderEffectsSection(result) {
    const requirementEffects = result.breakdown.requirements;
    const transforms = result.breakdown.transforms;
    const scoreLines = result.breakdown.scores;
    const triggered = result.triggered_abilities;

    return `
        <section class="perf-section-card">
            <div class="perf-section-heading-row compact">
                <div>
                    <div class="perf-eyebrow">${tr('perf_effects_title')}</div>
                    <h3>${tr('perf_effects_subtitle')}</h3>
                </div>
            </div>
            <div class="perf-effects-grid">
                <div class="perf-effects-column">
                    <div class="perf-mini-heading">${tr('perf_requirements_title')}</div>
                    <div class="perf-list-block">
                        ${requirementEffects.length > 0 || transforms.length > 0 ? `
                            ${requirementEffects.map((effect) => `<div class="perf-list-row">${escapeHtml(effect?.source || 'Effect')}: ${escapeHtml(effect?.value || effect?.desc || 'adjustment')}</div>`).join('')}
                            ${transforms.map((effect) => `<div class="perf-list-row">${escapeHtml(effect?.source || 'Effect')}: ${escapeHtml(effect?.desc || 'transform')}</div>`).join('')}
                        ` : `<div class="perf-empty-state small">${tr('perf_no_effects')}</div>`}
                    </div>
                </div>
                <div class="perf-effects-column">
                    <div class="perf-mini-heading">${tr('perf_score_line_title')}</div>
                    <div class="perf-list-block">
                        ${scoreLines.length > 0 ? scoreLines.map((line) => `
                            <div class="perf-score-line">
                                <span>${escapeHtml(line?.source || 'Score source')}</span>
                                <strong>+${line?.value || 0}</strong>
                            </div>
                        `).join('') : `<div class="perf-empty-state small">${tr('perf_no_scores')}</div>`}
                    </div>
                </div>
                <div class="perf-effects-column">
                    <div class="perf-mini-heading">${tr('perf_triggered_title')}</div>
                    <div class="perf-list-block">
                        ${triggered.length > 0 ? triggered.map((ability) => {
                            const effectText = ability?.effect_text || '';
                            const condText = ability?.condition_text || '';
                            const abilityDisplay = effectText ? enrichText(effectText) : '';
                            const condDisplay = condText ? enrichText(condText) : '';
                            const triggeredType = effectText.includes('ライブ開始時') || effectText.includes('live_start') ? 'live_start' :
                                effectText.includes('ライブ成功時') || effectText.includes('live_success') ? 'live_success' :
                                effectText.includes('常時') || effectText.includes('jyouji') ? 'jyouji' : '';
                            const durationLabel = {
                                'live_start': tr('perf_effect_duration_live_start'),
                                'live_success': tr('perf_effect_duration_live_success'),
                                'jyouji': tr('perf_effect_duration_jyouji'),
                            }[triggeredType] || '';
                            return `
                                <div class="perf-list-row">
                                    <div class="effect-title-row">
                                        <strong>${escapeHtml(ability?.card_name || 'Unknown card')}</strong>
                                        ${durationLabel ? `<span class="effect-duration">${escapeHtml(durationLabel)}</span>` : ''}
                                    </div>
                                    ${abilityDisplay ? `<div class="perf-bonus-text" style="margin-top: 4px; margin-left: 0;">${abilityDisplay}</div>` : ''}
                                    ${condDisplay ? `<div class="perf-ability-condition">Cond: ${condDisplay}</div>` : ''}
                                </div>
                            `;
                        }).join('') : `<div class="perf-empty-state small">${tr('perf_no_triggers')}</div>`}
                    </div>
                </div>
            </div>
        </section>
    `;
}

function renderPlayerPanel(playerId, result) {
    if (!result) return '';
    const lives = result.lives || [];
    const passedLives = countPassedLives(lives);
    const totalLives = lives.length;
    
    const isSuccess = result.success;

    // Check cannot_live restriction
    const playerKey = playerId === 0 ? 'player1' : 'player2';
    const playerStrId = State.data?.[playerKey]?.id;
    const cannotLivePlayers = State.data?.cannot_live_players || [];
    const isCannotLive = playerStrId && cannotLivePlayers.includes(playerStrId);

    let cannotLiveCardName = '';
    if (isCannotLive) {
        const entry = (State.data?.prohibition_effects || []).find(e => e.startsWith('const_restriction:cannot_live:'));
        if (entry) {
            const nameMatch = entry.match(/cardname=([^,]+?)(?:,|:)/);
            if (nameMatch) cannotLiveCardName = nameMatch[1];
        }
    }

    // Comparative win flags from the engine
    const winsKey = playerId === 0 ? 'p0_wins' : 'p1_wins';
    const otherWinsKey = playerId === 0 ? 'p1_wins' : 'p0_wins';
    const selfWins = !!result[winsKey];
    const otherWins = !!result[otherWinsKey];

    const totalHearts = sumHearts(result.total_hearts);
    const baseLiveScore = sumPassedLiveScores(lives);
    
    // Outcome prioritizes comparative win/loss; heart-check pass/fail is fallback
    let outcome = tr('perf_outcome_fail');
    if (isCannotLive) {
        outcome = tr('perf_outcome_cannot_live');
    } else if (selfWins && otherWins) {
        outcome = tr('perf_outcome_tie');
    } else if (selfWins) {
        outcome = tr('perf_outcome_won');
    } else if (otherWins) {
        outcome = tr('perf_outcome_lost');
    } else if (isSuccess) {
        outcome = tr('perf_outcome_pass');
    }

    const members = result?.member_contributions || [];
    const totalBlades = members.reduce((s, m) => m.is_wait ? s : s + (m.base_blades || 0) + (m.bonus_blades || 0), 0);

    // Panel success if either heart check passed OR player won comparative
    const panelSuccess = isSuccess || selfWins;

    return `
        <article class="perf-panel ${panelSuccess ? 'success' : 'failure'}">
            <header class="perf-panel-header">
                <div class="perf-panel-header-main">
                    <div class="perf-eyebrow">${escapeHtml(getPlayerName(playerId))}</div>
                    <h2>${escapeHtml(outcome)}</h2>
                    ${isCannotLive
                        ? `<div class="perf-panel-subtitle cannot-live-subtitle">${cannotLiveCardName ? tr('perf_due_to_ability', { name: escapeHtml(cannotLiveCardName) }) : tr('perf_due_to_restriction')}</div>`
                        : `<div class="perf-panel-subtitle">${tr('perf_judge_score_subtitle', { score: result?.total_score || 0, passed: passedLives, total: totalLives })}</div>`
                    }
                </div>
                <div class="perf-panel-statuses">
                    ${isCannotLive
                        ? `<div class="perf-status-pill blocked">${tr('perf_blocked')}</div>`
                        : `<div class="perf-status-pill ${isSuccess ? 'success' : 'failure'}">${isSuccess ? tr('perf_pass') : tr('perf_fail')}</div>`
                    }
                    <div class="perf-outcome-pill">${escapeHtml(outcome)}</div>
                </div>
            </header>
 
            <section class="perf-score-hero" style="border-bottom: 1px solid var(--border); margin-bottom: 16px; padding-bottom: 12px;">
                <div class="perf-metric-grid">
                    <div class="perf-metric-card highlight">
                        <div class="perf-metric-label">${tr('perf_judge_score')}</div>
                        <div class="perf-metric-value" style="font-size: 1.8rem;">${result?.total_score || 0}</div>
                    </div>
                    <div class="perf-metric-card">
                        <div class="perf-metric-label">${tr('perf_heart_vector')}</div>
                        <div class="perf-metric-value">
                            ${renderHeartsCompact(result?.total_hearts || [])}
                            <span class="total-count-dim">(${totalHearts})</span>
                        </div>
                    </div>
                    ${renderTextMetric(tr('perf_lives_passed'), `${passedLives} / ${totalLives}`)}
                    ${renderIconMetric('img/texticon/icon_score.png', tr('perf_live_pts'), String(baseLiveScore), 'score')}
                    ${renderIconMetric('img/texticon/icon_score.png', tr('perf_notes'), `${result?.note_icons || 0}`, 'notes')}
                    ${renderIconMetric('img/texticon/icon_blade.png', tr('perf_stage_blades'), String(totalBlades), 'blades')}
                    ${renderIconMetric('img/texticon/icon_blade.png', tr('perf_yell_count'), String(result?.yell_count || 0), 'yells')}
                </div>
            </section>

            <div class="perf-panel-body-grid">
                <div class="perf-column left">
                    ${renderAggregateHeartSummary(result)}
                    ${renderTotalSection(result)}
                    ${renderYellSection(result)}
                    ${renderEffectsSection(result)}
                </div>
                <div class="perf-column right">
                    ${renderContributionSection(result)}
                    ${renderLiveCards(result)}
                </div>
            </div>
        </article>
    `;
}

let _lastDisplayResults = null;

export const PerformanceRenderer = {
    renderHeartProgress,

    renderPerformanceGuide: () => {
        const state = State.data;
        if (!state) return;
        const perspectivePlayer = State.perspectivePlayer;
        const player = perspectivePlayer === 0 ? state.player1 : state.player2;
        const guide = player?.performance_guide;
        const panel = document.getElementById('perf-guide-panel');
        const contentEl = document.getElementById('perf-guide-content');
        if (!panel || !contentEl) return;

        if (!guide?.lives || guide.lives.length === 0) {
            panel.style.display = 'none';
            return;
        }

        panel.style.display = 'block';

        // Collect member hearts breakdown from performance results
        const perfResults = state.performance_results || {};
        const perfResult = perfResults[perspectivePlayer] || perfResults[0];
        let memberHtml = '';
        if (perfResult && perfResult.member_contributions && perfResult.member_contributions.length > 0) {
            memberHtml = '<div class="perf-guide-members">';
            perfResult.member_contributions.forEach(mc => {
                const cardData = mc.card_no ? State.resolveCardData(mc.card_no) : null;
                const imgSrc = cardData ? fixImg(cardData.img) : null;
                const name = cardData?.name || mc.source || 'Member';
                const heartHtml = renderHeartsCompact(mc.base_hearts || []);
                memberHtml += `
                    <div class="perf-guide-member">
                        ${imgSrc ? `<img src="${imgSrc}" class="perf-guide-member-img" alt="${escapeHtml(name)}">` : ''}
                        <div class="perf-guide-member-info">
                            <div class="perf-guide-member-name">${escapeHtml(name)}</div>
                            <div class="perf-guide-member-hearts">${heartHtml}</div>
                        </div>
                    </div>
                `;
            });
            memberHtml += '</div>';
        }

        let html = `
            <div class="perf-guide-header">
                <span><img src="img/texticon/icon_blade.png" class="heart-mini-icon"> <b>${guide.total_blades}</b></span>
                <span>${renderHeartsCompact(guide.total_hearts)}</span>
            </div>
            ${memberHtml}
        `;

        guide.lives.forEach((live) => {
            if (!live || typeof live !== 'object') return;
            const liveImgSrc = live.img || live.img_path ? fixImg(live.img || live.img_path) : null;
            html += `
                <div class="perf-guide-entry" style="opacity:${live.passed ? 1 : 0.72}">
                    ${liveImgSrc ? `<div class="card card-micro md"><img src="${liveImgSrc}" alt="${escapeHtml(live.name || 'Live')}"></div>` : ''}
                    <div class="perf-guide-info">
                        <div class="perf-guide-name">${escapeHtml(live.name || 'Live')} <span class="perf-guide-score">(${live.score || 0} pts)</span></div>
                        <div class="perf-guide-pips">${renderHeartProgress(live.filled, live.required)}</div>
                        ${!live.passed && live.reason ? `<div class="perf-guide-reason">${escapeHtml(live.reason)}</div>` : ''}
                    </div>
                    <div class="perf-guide-status" style="color:${live.passed ? '#78d08b' : '#f26d6d'}">${live.passed ? tr('perf_guide_ready') : tr('perf_guide_risk')}</div>
                </div>
            `;
        });

        contentEl.innerHTML = html;
    },

    renderPerformanceResult: (results = null) => {
        const modal = document.getElementById('performance-modal');
        const content = document.getElementById('performance-result-content');
        const title = document.getElementById('perf-title');
        if (!modal || !content) return;

        const displayResults = getDisplayResults(results);
        _lastDisplayResults = displayResults;
        if (!displayResults || Object.keys(displayResults).length === 0) {
            content.innerHTML = `<div class="perf-empty-state">${escapeHtml(tr('no_perf_data', 'No performance data is available yet.'))}</div>`;
            if (title) title.textContent = 'Performance Breakdown';
            return;
        }

        const sampleResult = displayResults?.[0] || displayResults?.[1];
        const selectedTurn = State.selectedPerfTurn >= 0 ? State.selectedPerfTurn : null;
        if (title) {
            title.textContent = getTurnLabel(sampleResult?.turn ?? selectedTurn);
        }

        PerformanceRenderer.renderTurnHistory();
        PerformanceRenderer.showPerfTab('result');

        content.innerHTML = `
            <div class="perf-overview-shell">
                ${renderTurnNavigation()}
                ${renderComparisonBanner(displayResults)}
                <div class="perf-player-grid">
                    ${[0, 1].map((playerId) => renderPlayerPanel(playerId, displayResults[playerId])).join('')}
                </div>
            </div>
        `;
    },

    showPerfTab: (tab) => {
        const resultTab = document.getElementById('perf-tab-result');
        const engineTab = document.getElementById('perf-tab-engine');
        const historyTab = document.getElementById('perf-tab-history');

        const resultBtn = document.getElementById('tab-btn-result');
        const engineBtn = document.getElementById('tab-btn-engine');
        const historyBtn = document.getElementById('tab-btn-history');

        if (!resultTab || !historyTab || !engineTab) return;

        resultTab.style.display = 'none';
        engineTab.style.display = 'none';
        historyTab.style.display = 'none';

        resultBtn?.classList.remove('active');
        engineBtn?.classList.remove('active');
        historyBtn?.classList.remove('active');

        if (tab === 'result') {
            resultTab.style.display = 'block';
            resultBtn?.classList.add('active');
        } else if (tab === 'engine') {
            engineTab.style.display = 'block';
            engineBtn?.classList.add('active');
            const engineContent = document.getElementById('performance-engine-content');
            if (engineContent) {
                const firstResult = _lastDisplayResults?.[0] || _lastDisplayResults?.[1];
                engineContent.innerHTML = renderPerfSteps(firstResult);
            }
        } else {
            historyTab.style.display = 'block';
            historyBtn?.classList.add('active');
            PerformanceRenderer.renderTurnHistory();
        }
    },

    renderTurnHistory: () => {
        const container = document.getElementById('performance-history-content');
        if (!container) return;

        const history = State.data?.turn_history || State.data?.turn_events || [];
        if (!Array.isArray(history) || history.length === 0) {
            container.innerHTML = `<div class="perf-empty-state">${escapeHtml(tr('no_history', 'No turn history is available.'))}</div>`;
            return;
        }

        container.innerHTML = history.map((event) => {
            const phaseKey = PerformanceRenderer._getPhaseKey(event.phase);
            const playerLabel = event.player_id === State.perspectivePlayer ? tr('you', 'You') : tr('opponent', 'Opponent');
            const typeClass = event.event_type ? String(event.event_type).toLowerCase() : 'generic';
            return `
                <div class="turn-event-item ${escapeHtml(typeClass)}">
                    <div class="event-header">
                        <span>Turn ${event.turn} - <span class="event-phase-tag">${escapeHtml(tr(phaseKey, phaseKey))}</span></span>
                        <span>${escapeHtml(playerLabel)}</span>
                    </div>
                    <div class="event-source">${escapeHtml(event.event_type || 'Event')}</div>
                    <div class="event-description">${escapeHtml(event.description || '')}</div>
                </div>
            `;
        }).join('');
    },

    _getPhaseKey: (phase) => {
        const perspectivePlayer = State.perspectivePlayer;
        if (phase === Phase.ROCK_PAPER_SCISSORS) return 'rps';
        if (isMulliganPhase(phase)) return 'mulligan';
        if (phase === Phase.ACTIVE) return 'active';
        if (phase === Phase.ENERGY) return 'energy';
        if (phase === Phase.DRAW) return 'draw';
        if (phase === Phase.MAIN) return 'main';
        if (isLiveCardSetPhase(phase)) return 'live_set';
        if (phase === Phase.FIRST_ATTACKER_PERFORMANCE) return perspectivePlayer === 0 ? 'perf_p1' : 'perf_p2';
        if (phase === Phase.SECOND_ATTACKER_PERFORMANCE) return perspectivePlayer === 1 ? 'perf_p1' : 'perf_p2';
        if (phase === Phase.LIVE_VICTORY_DETERMINATION) return 'live_result';
        return 'wait';
    },

    renderHeartsGrid,
    renderHeartsCompact,
    renderBladesCompact,
    renderAggregateHeartSummary,
};
