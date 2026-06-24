/**
 * Performance Renderer Component
 * Renders a player-facing explanation of the performance phase using the
 * snapshot emitted by the Rust engine.
 */
import { State } from '../state.js';
import { fixImg, Phase, isMulliganPhase } from '../constants.js';
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
        return 'Performance Breakdown';
    }
    return `Performance Breakdown - Turn ${turn}`;
}

function getOutcomeLabel(playerId, result) {
    if (!result) return 'No result';
    const winsKey = playerId === 0 ? 'p0_wins' : 'p1_wins';
    const otherWinsKey = playerId === 0 ? 'p1_wins' : 'p0_wins';
    const selfWins = !!result[winsKey];
    const otherWins = !!result[otherWinsKey];

    if (selfWins && otherWins) return 'Comparative tie';
    if (selfWins) return 'Won live result';
    if (otherWins) return 'Lost live result';
    return result.success ? 'Passed performance' : 'Failed performance';
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

    if (filtered.length === 0) return '<div class="perf-hearts-grid empty">None</div>';

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
        return '<span class="perf-empty-inline">none</span>';
    }

    const heartNames = ['Any (heart_00 wildcard)', 'Pink', 'Red', 'Yellow', 'Green', 'Blue', 'Purple', 'All (icon_all blade/ability)'];

    return `<div class="hearts-compact">${hearts.map((count, index) => {
        if (!count) return '';
        const iconSrc = HEART_ICONS[index];
        return `
            <div class="heart-tag ${index === 0 ? 'color-any' : `color-${index}`}" title="${heartNames[index]}">
                <img src="${iconSrc}" class="heart-mini-icon" alt="${heartNames[index]}">
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
    if (lives.length === 0) return '';

    const totalHearts = result.total_hearts || [0,0,0,0,0,0,0,0];
    const totalAvailable = sumHearts(totalHearts);
    const allPassed = lives.every(l => l.passed);
    const allocations = result?.breakdown?.allocations || [];

    const fmtPool = (arr, label) => `
        <div class="perf-agg-row">
            <span class="perf-agg-label"></span>
            ${renderHeartsCompact(arr)}
            <span class="perf-agg-sum">${sumHearts(arr)}</span>
            ${label ? `<span class="perf-agg-desc">${label}</span>` : ''}
        </div>`;

    // Reconstruct remaining pool at each step using allocations
    let html = `
        <div class="perf-agg-summary ${allPassed ? 'success' : 'failure'}">
            <div class="perf-agg-header">
                <img src="img/texticon/heart_00.png" class="heart-mini-icon" alt="">
                Heart Allocation — sequential per live card
            </div>
            <div class="perf-agg-table">
                <div class="perf-agg-row">
                    <span class="perf-agg-label">Available pool</span>
                    ${renderHeartsCompact(totalHearts)}
                    <span class="perf-agg-sum">${totalAvailable}</span>
                </div>`;

    // The engine allocates hearts from the shared pool to each live card sequentially.
    // Allocation phases (engine/src/turn/live.rs):
    //
    //   Phase ① — Colored hearts → colored req (by matching color, up to req)
    //     e.g. 4 Red hearts with req 2 Red: 2 go to Phase ①, 2 remain as excess
    //
    //   Phase ② — Wild (Any / heart_00) hearts → remaining colored deficits
    //     Any heart (index 0) is a wildcard that fills any color's unmet req.
    //
    //   Phase ③ step 1 — Leftover colored hearts → Heart00 req
    //     Colored hearts beyond what Phase ① used go to the Any/Heart00 requirement.
    //     The engine records these under their original color index (1-6), NOT as index 0.
    //
    //   Phase ③ step 2 — Remaining Any (heart_00) hearts → Heart00 req
    //     Recorded with color index 0.
    //
    // Heart type reference:
    //   heart_00  (index 0, img/texticon/heart_00.png)
    //     Rule 2.1.1.2: "heart icon with no specific color" — wildcard.
    //     In pool: a wildcard heart that can be any single color.
    //     In need_heart: accepts hearts of ANY color to satisfy this slot.
    //
    //   icon_all  (index 7, img/texticon/icon_all.png)
    //     Rule 2.1.1.3: "can be treated as any ONE of Pink/Red/Yellow/Green/Blue/Purple"
    //     Rule 8.3.15.1.1: during performance check, each icon_all = any ONE color.
    //     QA Q46: icon_all's color assignment is decided at perf check time.
    //     QA Q67: icon_all is ONLY treated as any color during perf check, NOT live start.
    //     NOTE: icon_all can fill heart00 req by being treated as one of the other colors.
    //     If there are no specific color deficits, which color you pick doesn't matter —
    //     h00_satisfied = sum(filled[1..7]) counts all colored hearts equally. The choice
    //     only matters when a specific colored slot still has a deficit.
    //
    //   ALLブレード (BAll, img/texticon/icon_b_all.png)
    //     QA Q45: during perf check, each ALLブレード counts as any ONE color.
    //     QA Q112: ALLブレード is classified as a blade heart.
    //     Engine: BladeColor::All → HeartColor::Heart00 (converted during blade processing).
    //
    //   heart_01-06  — Pink, Red, Yellow, Green, Blue, Purple specific colors.
    //
    // PASS/FAIL logic (engine check_heart_requirement, engine/src/core/card.rs):
    //   Rule 2.11.3 — Requires BOTH:
    //     (a) For each non-heart_00 slot: enough hearts of that color exist.
    //     (b) Total provided hearts >= total required hearts.
    //   Engine steps:
    //     1. total_filled < total_required → FAIL (2.11.3 bullet 2)
    //     2. Heart00: sum(filled[1..7]) + filled[0] < req[0] → FAIL
    //        All colored hearts consumed count toward Heart00 (h00_satisfied).
    //     3. Each colored slot: filled[c] < req[c] AND wildcard (heart00+all) can't cover → FAIL
    //   QA Q115: need_heart modifications apply "set" then "add/subtract".
    //   QA Q127, Q110: heart_00 increases from multiple sources stack.
    //   QA Q148, Q114, Q99, Q98: heart_00 reductions per condition.
    //
    // filled[c] includes ALL allocations to color c (Phases ① + ② + ③ step 1).
    // We split Phase ① and Phase ③ step 1 by capping at per-color req.

    const remaining = totalHearts.map(v => v);
    for (let liveIdx = 0; liveIdx < lives.length; liveIdx++) {
        const live = lives[liveIdx];
        const cd = live.card_no ? State.resolveCardData(live.card_no) : null;
        const liveName = cd?.name || `Live ${liveIdx + 1}`;
        const req = live.required || [0,0,0,0,0,0,0,0];
        const filled = live.filled || [0,0,0,0,0,0,0,0];
        const reqSum = sumHearts(req);
        const fillSum = sumHearts(filled);
        const passed = live.passed;
        const colorReq = req.slice(1, 7);
        const colorFill = filled.slice(1, 7);
        const colorReqSum = sumHearts(colorReq);
        const colorFillSum = sumHearts(colorFill);
        const wildReq = req[0] || 0;
        const wildFill = filled[0] || 0;

        // Per-color deficit for colored req (sum of shortfalls per color, not total)
        const colDeficit = [1,2,3,4,5,6].reduce((sum, c) => sum + Math.max(0, (req[c] || 0) - (filled[c] || 0)), 0);

        // Phase 1: colored hearts → colored req (capped at req per color)
        // Phase 3 step 1: excess colored hearts → Heart00 (colored beyond req)
        const phase1PerColor = [0,0,0,0,0,0,0,0];
        const p3ColorPerColor = [0,0,0,0,0,0,0,0];
        for (let c = 1; c <= 6; c++) {
            const fc = filled[c] || 0;
            const rc = req[c] || 0;
            phase1PerColor[c] = Math.min(fc, rc);
            p3ColorPerColor[c] = Math.max(0, fc - rc);
        }
        const phase1Total = sumHearts(phase1PerColor);
        const p3ColorTotal = sumHearts(p3ColorPerColor);
        const totalWildFill = wildFill + p3ColorTotal;

        // Phase 2: wildcard (Any index-0) hearts filling colored deficit
        const p2Allocs = allocations.filter(a => a.target_idx === liveIdx && a.wildcard);
        // Phase 3 step 2: wildcard Any hearts (color 0) filling remaining Heart00
        const p3Allocs = allocations.filter(a => a.target_idx === liveIdx && !a.wildcard && a.color === 0);

        // Compute what was left before this card
        const beforeDisplay = remaining.map(v => v);

        // Deduct what this card consumed
        for (const a of allocations) {
            if (a.target_idx === liveIdx) {
                remaining[a.color] = Math.max(0, remaining[a.color] - a.amount);
            }
        }

        const afterDisplay = remaining.map(v => v);
        const beforeSum = sumHearts(beforeDisplay);
        const afterSum = sumHearts(afterDisplay);

        const consumedArr = beforeDisplay.map((v, i) => v - afterDisplay[i]);
        const consumedSum = sumHearts(consumedArr);
        const totalShort = Math.max(0, reqSum - consumedSum);

        html += `
            <div class="perf-agg-card ${passed ? 'success' : 'failure'}">
                <div class="perf-agg-card-head">
                    <strong>${escapeHtml(liveName)}</strong>
                    <span class="perf-status-pill tiny ${passed ? 'success' : 'failure'}">${passed ? 'PASS' : 'FAIL'}</span>
                </div>
                <div class="perf-agg-card-require">Need ${renderHeartsCompact(req)} = ${reqSum}</div>
                <div class="perf-agg-card-pool">Before: ${renderHeartsCompact(beforeDisplay)} = ${beforeSum}</div>
                <div class="perf-agg-card-pool consumed">Used: ${renderHeartsCompact(consumedArr)} = ${consumedSum}${totalShort > 0 ? ` <span class="perf-agg-fail">(${totalShort} short of ${reqSum} needed)</span>` : ''}</div>
                <div class="perf-agg-steps">
                    ${colorReqSum > 0 ? `
                    <div class="perf-agg-step ${phase1Total >= colorReqSum ? 'done' : 'fail'}">
                        <span class="perf-agg-marker">①</span>
                        <span>Colored hearts → colored req</span>
                        <span class="perf-agg-step-stat">${phase1Total}/${colorReqSum}${phase1Total < colorReqSum ? `<span class="perf-agg-fail"> (${colorReqSum - phase1Total} short)</span>` : ''}</span>
                        ${phase1Total > 0 ? `<div class="perf-agg-alloc-detail">${[1,2,3,4,5,6].filter(c => phase1PerColor[c] > 0).map(c => `${phase1PerColor[c]}×${HEART_LABELS[c]}`).join(', ')}</div>` : ''}
                    </div>` : ''}
                    ${colDeficit > 0 ? `
                    <div class="perf-agg-step ${p2Allocs.reduce((s, a) => s + a.amount, 0) >= colDeficit ? 'done' : 'fail'}">
                        <span class="perf-agg-marker">②</span>
                        <span>Wild (Any) → colored deficit</span>
                        <span class="perf-agg-step-stat">${p2Allocs.reduce((s, a) => s + a.amount, 0)}/${colDeficit}${p2Allocs.reduce((s, a) => s + a.amount, 0) < colDeficit ? `<span class="perf-agg-fail"> (${colDeficit - p2Allocs.reduce((s, a) => s + a.amount, 0)} short)</span>` : ''}</span>
                        ${p2Allocs.length > 0 ? `<div class="perf-agg-alloc-detail">${p2Allocs.map(a => `${a.amount}×${HEART_LABELS[a.color] || a.color}`).join(', ')}</div>` : ''}
                        ${p2Allocs.length === 0 ? '<div class="perf-agg-alloc-detail">No wild (Any) hearts available</div>' : ''}
                    </div>` : ''}
                    ${wildReq > 0 ? `
                    <div class="perf-agg-step ${totalWildFill >= wildReq ? 'done' : 'fail'}">
                        <span class="perf-agg-marker">③</span>
                        <span>Heart00 (any heart color accepted)</span>
                        <span class="perf-agg-step-stat">${totalWildFill}/${wildReq}${totalWildFill < wildReq ? `<span class="perf-agg-fail"> (${wildReq - totalWildFill} short)</span>` : ''}</span>
                        ${totalWildFill > 0 ? `<div class="perf-agg-alloc-detail">${
                            [1,2,3,4,5,6].filter(c => p3ColorPerColor[c] > 0).map(c => `${p3ColorPerColor[c]}×${HEART_LABELS[c]}`).concat(
                                p3Allocs.map(a => `${a.amount}×${HEART_LABELS[a.color] || a.color}`)
                            ).join(', ')
                        }</div>` : ''}
                    </div>` : ''}
                </div>
                <div class="perf-agg-card-after">After: ${renderHeartsCompact(afterDisplay)} = ${afterSum}</div>
            </div>`;
    }

    // Final surplus (always show)
    const finalRemaining = remaining.map(v => v);
    const surplusTotal = sumHearts(finalRemaining);
    html += `
                <div class="perf-agg-divider"></div>
                <div class="perf-agg-row surplus ${surplusTotal > 0 ? 'positive' : 'zero'}">
                    <span class="perf-agg-label">Surplus</span>
                    ${renderHeartsCompact(finalRemaining)}
                    <span class="perf-agg-surplus-value">${surplusTotal > 0 ? '+' : ''}${surplusTotal}</span>
                </div>
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
    if (!result) return '<div class="perf-empty-state">No performance data available.</div>';

    const H = ['h00','h01','h02','h03','h04','h05','h06'];
    const fmtH = (arr) => arr ? H.map((h,i) => arr[i] > 0 ? `${h}:${arr[i]}` : null).filter(Boolean).join(' ') : 'none';
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
                    <div class="perf-eyebrow">Live Phase — Step by Step</div>
                </div>
            </div>

            <!-- Step 1: Live Zone -->
            <details class="perf-step-detail" open>
                <summary class="perf-step-summary">1. Live Zone — ${result.lives?.length || 0} card(s)</summary>
                <div class="perf-step-body">
                    ${(result.lives || []).map((live, i) => {
                        const cardData = live.card_no ? State.resolveCardData(live.card_no) : null;
                        const imgSrc = cardData ? fixImg(cardData.img || '') : '';
                        return `
                            <div class="perf-step-live-card">
                                ${imgSrc ? `<div class="perf-live-art-wrapper md"><img src="${imgSrc}"></div>` : ''}
                                <div class="perf-step-live-info">
                                    <div>Require: ${fmtHShortReq(live.required)}</div>
                                    <div>Score: ${live.score}</div>
                                </div>
                            </div>
                        `;
                    }).join('') || '<div class="perf-empty-state small">No live cards</div>'}
                    <div class="perf-step-note">§8.3.1: Live card zone 확인. Non-live cards는 제거됨.</div>
                </div>
            </details>

            <!-- Step 2: Live Start Triggers -->
            <details class="perf-step-detail" open>
                <summary class="perf-step-summary">2. Live Start — ${result.triggered_abilities?.length || 0} trigger(s)</summary>
                <div class="perf-step-body">
                    ${(result.triggered_abilities || []).map(t => {
                        const cd = State.resolveCardData(t.source_card_id);
                        return `<div class="perf-step-trigger">${cd?.name || t.card_name || '?'}: ${t.name || 'triggered'}</div>`;
                    }).join('') || '<div class="perf-empty-state small">No live-start triggers</div>'}
                    <div class="perf-step-note">§8.3.2: Live start triggers resolve before yell.</div>
                </div>
            </details>

            <!-- Step 3: Blades + Yell -->
            <details class="perf-step-detail" open>
                <summary class="perf-step-summary">3. Blades → Yell — ${totalBlades} blades → ${result.yell_count || 0} yell</summary>
                <div class="perf-step-body">
                    <div class="perf-step-members">
                        ${(result.member_contributions || []).map(m => {
                            const cd = m.card_no ? State.resolveCardData(m.card_no) : null;
                            const imgSrc = cd ? fixImg(cd.img || '') : '';
                            const isWait = m.is_wait;
                            return `
                                <div class="perf-step-member${isWait ? ' perf-dimmed' : ''}">
                                    ${imgSrc ? `<img src="${imgSrc}" class="perf-step-member-img">` : ''}
                                    <div>Blade: ${isWait ? '0 (negated)' : `${m.base_blades}${m.bonus_blades > 0 ? '+' + m.bonus_blades : ''}`} ${isWait ? '<span class="perf-wait-badge">(wait)</span>' : ''}</div>
                                    <div>${fmtHShortSrc(m.base_hearts)}</div>
                                </div>
                            `;
                        }).join('')}
                    </div>
                    <div class="perf-step-note">§8.3.3: Total blades × cheer modifier = yell count.</div>
                </div>
            </details>

            <!-- Step 4: Stage Hearts -->
            <details class="perf-step-detail" open>
                <summary class="perf-step-summary">4. Stage Hearts — ${fmtH(result.total_hearts)}</summary>
                <div class="perf-step-body">
                    <div class="perf-step-hearts-row">
                          ${result.total_hearts ? H.map((h,i) =>
                              result.total_hearts[i] > 0
                                 ? `<span class="perf-step-heart-cell"><img src="img/texticon/heart_0${i}.png" class="heart-mini-icon"> ${result.total_hearts[i]}</span>`
                                : ''
                        ).join('') : ''}
                    </div>
                    <div class="perf-step-note">§8.3.4: Sum of stage member hearts + yell blade hearts.</div>
                </div>
            </details>

            <!-- Step 5: Yell Cards -->
            <details class="perf-step-detail" open>
                <summary class="perf-step-summary">5. Yell Cards — ${result.yell_cards?.length || 0} card(s)</summary>
                <div class="perf-step-body">
                    <div class="perf-step-yells">
                        ${(result.yell_cards || []).map(y => {
                            const cd = y.card_no ? State.resolveCardData(y.card_no) : null;
                            const imgSrc = cd ? fixImg(cd.img || '') : '';
                            return `
                                <div class="perf-step-yell-card">
                                    ${imgSrc ? `<img src="${imgSrc}" class="perf-step-card-img-sm">` : ''}
                                    <div>♥ ${fmtHShortSrc(y.blade_hearts)}</div>
                                    <div><img src="img/texticon/icon_score.png" class="heart-mini-icon">${y.note_icons} <img src="img/texticon/icon_draw.png" class="heart-mini-icon">${y.draw_icons}</div>
                                </div>
                            `;
                        }).join('') || '<div class="perf-empty-state small">No yell cards</div>'}
                    </div>
                    <div class="perf-step-note">§8.3.5: Yell cards provide blade hearts + note/draw icons.</div>
                </div>
            </details>

            <!-- Step 6: Color Transforms -->
            <details class="perf-step-detail">
                <summary class="perf-step-summary">6. Color Transforms — ${result.breakdown?.transforms?.length || 0} change(s)</summary>
                <div class="perf-step-body">
                    ${(result.breakdown?.transforms || []).map(t =>
                        `<div class="perf-step-transform">${t.source}: ${t.desc}</div>`
                    ).join('') || '<div class="perf-empty-state small">No color transforms</div>'}
                    <div class="perf-step-note">§8.3.7: Heart color conversion effects apply.</div>
                </div>
            </details>

            <!-- Step 7: Requirements Modifiers -->
            <details class="perf-step-detail">
                <summary class="perf-step-summary">7. Requirement Mods — ${result.breakdown?.requirements?.length || 0} change(s)</summary>
                <div class="perf-step-body">
                    ${(result.breakdown?.requirements || []).map(r =>
                        `<div class="perf-step-req">${r.source}: ${r.desc}</div>`
                    ).join('') || '<div class="perf-empty-state small">No requirement changes</div>'}
                    <div class="perf-step-note">§8.3.6–8.3.7: Effects modify required hearts.</div>
                </div>
            </details>

            <!-- Step 8: Judge Each Live -->
            <details class="perf-step-detail" open>
                <summary class="perf-step-summary">8. Judge Lives — ${passedLives}/${result.lives?.length || 0} passed</summary>
                <div class="perf-step-body">
                    ${(result.lives || []).map((live, i) => {
                        const cardData = live.card_no ? State.resolveCardData(live.card_no) : null;
                        const imgSrc = cardData ? fixImg(cardData.img || '') : '';
                        const failedReason = live.adjustments?.filter(a => a.adjustment_type === 'failure') || [];
                        return `
                            <div class="perf-step-judge ${live.passed ? 'pass' : 'fail'}">
                                <div class="perf-step-judge-header">
                                    ${imgSrc ? `<div class="perf-live-art-wrapper sm"><img src="${imgSrc}"></div>` : ''}
                                    <span>Slot ${i}: <b>${live.passed ? '✓ PASS' : '✗ FAIL'}</b> score +${live.score}</span>
                                </div>
                                <div class="perf-step-judge-detail">
                                    need ${fmtHShortReq(live.required)} / filled ${fmtHShortSrc(live.filled)} / spare ${fmtHShortSrc(live.spare)}
                                </div>
                                ${failedReason.map(a => `<div class="perf-step-fail-reason">${a.desc}</div>`).join('')}
                            </div>
                        `;
                    }).join('') || '<div class="perf-empty-state small">No live cards</div>'}
                    <div class="perf-step-note">§8.3.8: Lives judged in slot order 0→1→2. One failure = whole zone fails.</div>
                </div>
            </details>

            <!-- Step 9: Score + Winner -->
            <details class="perf-step-detail" open>
                <summary class="perf-step-summary">9. Result — Score ${result.total_score || 0} ${result.success ? '✓ PASS' : '✗ FAIL'}</summary>
                <div class="perf-step-body">
                    <div class="perf-step-result-row">
                        <div class="perf-step-result-item">
                            <img src="img/texticon/icon_score.png" class="heart-mini-icon">
                            Base live score: ${baseRawScore}
                        </div>
                        <div class="perf-step-result-item">
                            <img src="img/texticon/icon_score.png" class="heart-mini-icon">
                            Triggered bonuses: ${baseLiveScore - baseRawScore > 0 ? '+' : ''}${baseLiveScore - baseRawScore}
                        </div>
                        <div class="perf-step-result-item total">
                            Total: <b>${result.total_score || 0}</b>
                        </div>
                        <div class="perf-step-result-item outcome ${result.success ? 'success' : 'failure'}">
                            ${result.success ? '✓ PASS' : '✗ FAIL'}
                            ${result.p0_wins ? ' — P1 wins!' : ''}
                            ${result.p1_wins ? ' — P2 wins!' : ''}
                            ${result.p0_wins && result.p1_wins ? ' — Draw' : ''}
                        </div>
                    </div>
                    <div class="perf-step-note">§8.3.9: Compare scores. Top live card moved to success if passed. §1.2: Win at 3+ success.</div>
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
    let summary = 'This snapshot has not reached the comparative winner check yet.';

    if (p0Wins && p1Wins) {
        summary = 'Both players are marked as winners in Live Result, so this performance snapshot is a comparative tie.';
    } else if (p0Wins) {
        summary = `${getPlayerName(0)} won the live result comparison for this turn.`;
    } else if (p1Wins) {
        summary = `${getPlayerName(1)} won the live result comparison for this turn.`;
    } else if (p0?.success || p1?.success) {
        summary = 'At least one player passed their performance, but no winner flag is stored on this snapshot.';
    }

    return `
        <section class="perf-comparison-banner" style="padding: 8px 12px; margin-bottom: 4px;">
            <div class="perf-comparison-copy" style="font-size: 0.9rem;"><b>Result:</b> ${escapeHtml(summary)}</div>
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
                    <div class="perf-eyebrow">Total Hearts Available</div>
                </div>
            </div>
            <div class="perf-total-breakdown">
                <div class="perf-breakdown-row grand">
                    <span class="perf-mini-heading">Stage + Yell</span>
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

    return `
        <section class="perf-section-card">
            <div class="perf-section-heading-row compact">
                <div>
                    <div class="perf-eyebrow">Live Checks</div>
                </div>
            </div>
            ${noLives ? '<div class="perf-empty-state">No live cards were stored in this snapshot.</div>' : ''}
            <div class="perf-live-grid">
                ${noLives ? renderAggregateHeartSummary(lives, result?.total_hearts || [0,0,0,0,0,0,0,0], result?.breakdown?.allocations || [], 0)
                : lives.map((live, index) => {
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
                                    ${cd?.img ? `<div class="perf-live-art-wrapper lg"><img src="${fixImg(cd.img)}" alt="${escapeHtml(cd?.name || 'Live')}"></div>` : ''}
                                    <div>
                                        <h4>${escapeHtml(cd?.name || 'Live')}</h4>
                                        <div class="perf-breakdown-row total">
                                            <span class="perf-mini-heading"><img src="img/texticon/icon_score.png" class="heart-mini-icon"> Score</span>
                                            <span class="perf-breakdown-detail">Base ${baseScore}</span>
                                            ${bonusScore > 0 ? `<span class="perf-breakdown-detail">+${bonusScore} abilities</span>` : ''}
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
                                    <span class="perf-mini-heading">Required</span>
                                    ${renderHeartsCompact(required)}
                                    <span class="perf-breakdown-sum">${sumHearts(required)}</span>
                                </div>
                                <div class="perf-breakdown-row">
                                    <span class="perf-mini-heading">Filled</span>
                                    ${renderHeartsCompact(filled)}
                                    <span class="perf-breakdown-sum">${sumHearts(filled)}</span>
                                </div>
                                <div class="perf-breakdown-row">
                                    <span class="perf-mini-heading">Remaining</span>
                                    ${renderHeartsCompact(spare)}
                                    <span class="perf-breakdown-sum">${sumHearts(spare)}</span>
                                </div>
                            </div>
                            ${required[0] > 0 ? `<div class="perf-heart-legend" style="font-size:0.65rem;color:var(--text-muted);margin-top:2px;"><img src="img/texticon/heart_00.png" class="heart-mini-icon" style="width:12px;height:12px;"> Any hearts fill deficits of any color</div>` : ''}
                            ${adjustments && adjustments.length > 0 ? `
                                <div class="perf-pill-list">
                                    ${adjustments.map((adj) => {
                                        const isTransform = adj?.type === 'transform' || adj?.type === 'override';
                                        const adjText = adj?.desc || `${adj?.value > 0 ? '+' : ''}${adj?.value || 0} ${HEART_LABELS[adj?.color ?? 0] || 'heart'}`;
                                        const adjAbility = findAbilitySource(triggered, adj?.source || '');
                                        const sourceLabel = adjAbility ? `${escapeHtml(adjAbility.card_name || adj?.source || 'Effect')}` : escapeHtml(adj?.source || 'Effect');
                                        return `<div class="perf-adjustment-pill ${isTransform ? 'transform' : 'requirement'}">${sourceLabel}: ${escapeHtml(adjText)}</div>`;
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
                        <div class="perf-eyebrow">Stage Contributors</div>
                    </div>
                </div>
                <div class="perf-empty-state">No stage member contribution breakdown is stored for this snapshot.</div>
            </section>
        `;
    }

    const slotLabels = ['Left', 'Center', 'Right'];

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
                        <h4>${escapeHtml(memberName)}${isWait ? ' <span class="perf-wait-badge">(wait)</span>' : ''}</h4>
                        <div class="perf-breakdown-row total">
                            <span class="perf-mini-heading">Total hearts</span>
                            ${renderHeartsCompact(total)}
                            <span class="perf-breakdown-sum">${sumHearts(total)}</span>
                        </div>
                    </div>
                </div>
                <div class="perf-stage-breakdown">
                    <div class="perf-breakdown-subrows">
                        <div class="perf-breakdown-row sub">
                            <span class="perf-mini-heading">① Base hearts</span>
                            ${renderHeartsCompact(base)}
                            <span class="perf-breakdown-sum">${sumHearts(base)}</span>
                        </div>
                        ${base.some((v, i) => v !== afterTransform[i]) ? `
                        <div class="perf-breakdown-row sub">
                            <span class="perf-mini-heading">② After transform</span>
                            ${renderHeartsCompact(afterTransform)}
                            <span class="perf-breakdown-sum">${sumHearts(afterTransform)}</span>
                        </div>
                        ` : ''}
                        ${transformDelta.some(v => v !== 0) ? `
                        <div class="perf-breakdown-row sub">
                            <span class="perf-mini-heading">③ Transform Δ</span>
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
                        <span class="perf-mini-heading">Blades</span>
                        ${renderBladesCompact(totalBlade)}
                        ${!isWait && (member.bonus_blades || 0) > 0 ? `<span class="perf-breakdown-detail">(+${member.bonus_blades} from abilities)</span>` : ''}
                        ${isWait ? `<span class="perf-breakdown-detail">(negated — card is in wait)</span>` : ''}
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
                        <span class="perf-mini-heading"><img src="img/texticon/icon_score.png" class="heart-mini-icon"> Notes</span>
                        <span class="perf-breakdown-value">${member?.base_notes || 0}${member?.bonus_notes ? ` (+${member.bonus_notes})` : ''}</span>
                        <span style="margin-left:12px;" class="perf-mini-heading"><img src="img/texticon/icon_draw.png" class="heart-mini-icon"> Draw</span>
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
                    <div class="perf-eyebrow">Stage Contributors</div>
                    ${members.length > 0 ? `<div class="perf-total-badge">${sumHearts(grandTotal)} hearts · ${grandBlade} blades · ${grandNotes} notes · ${grandDraw} draw</div>` : ''}
                </div>
            </div>
            <div class="perf-contrib-grid">
                ${rendered.join('')}
                ${members.length > 1 ? `
                <article class="perf-contrib-card perf-total-row">
                    <div class="perf-contrib-header">
                        <div>
                            <h4>Total (all slots)</h4>
                            <div class="perf-breakdown-row total">
                                <span class="perf-mini-heading">Hearts</span>
                                ${renderHeartsCompact(grandTotal)}
                                <span class="perf-breakdown-sum">${sumHearts(grandTotal)}</span>
                            </div>
                        </div>
                    </div>
                    <div class="perf-stage-breakdown">
                        <div class="perf-breakdown-row">
                            <span class="perf-mini-heading">Blades</span>
                            <span class="perf-breakdown-value">${grandBlade}</span>
                        </div>
                        <div class="perf-breakdown-row minor">
                            <span class="perf-mini-heading">Notes</span>
                            <span class="perf-breakdown-value">${grandNotes}</span>
                            <span style="margin-left:12px;" class="perf-mini-heading">Draw</span>
                            <span class="perf-breakdown-value">${grandDraw}</span>
                        </div>
                    </div>
                </article>
                ` : ''}
                ${globalTriggered.length > 0 ? `
                <article class="perf-contrib-card global-bonuses">
                    <div class="perf-contrib-header">
                        <div>
                            <h4>Global Bonuses</h4>
                            <div class="perf-breakdown-bonuses">
                                ${globalTriggered.map((ability) => {
                                    const effectText = ability?.effect_text || '';
                                    const condText = ability?.condition_text || '';
                                    const abilityDisplay = effectText ? enrichText(effectText) : '';
                                    const condDisplay = condText ? enrichText(condText) : '';
                                    const triggeredType = effectText.includes('ライブ開始時') || effectText.includes('live_start') ? 'live_start' :
                                        effectText.includes('ライブ成功時') || effectText.includes('live_success') ? 'live_success' :
                                        effectText.includes('常時') || effectText.includes('jyouji') ? 'jyouji' : '';
                                    return `
                                        <div class="perf-bonus-item compact">
                                            <div class="perf-bonus-title">${escapeHtml(ability?.card_name || 'Ability')} ${triggeredType ? `<span class="effect-duration">[${escapeHtml(triggeredType)}]</span>` : ''}</div>
                                            ${abilityDisplay ? `<div class="perf-bonus-text">${abilityDisplay}</div>` : ''}
                                            ${condDisplay ? `<div class="perf-ability-condition">Condition: ${condDisplay}</div>` : ''}
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
                        <div class="perf-eyebrow">Yell & Source Pool</div>
                    </div>
                </div>
                <div class="perf-empty-state">No yell cards or source data recorded.</div>
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
                    <span class="perf-mini-heading">Total yell hearts</span>
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
                    <div class="perf-mini-heading">Heart sources</div>
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
                    <div class="perf-eyebrow">Effects and Score Lines</div>
                    <h3>Everything else the engine explicitly logged</h3>
                </div>
            </div>
            <div class="perf-effects-grid">
                <div class="perf-effects-column">
                    <div class="perf-mini-heading">Requirement and color effects</div>
                    <div class="perf-list-block">
                        ${requirementEffects.length > 0 || transforms.length > 0 ? `
                            ${requirementEffects.map((effect) => `<div class="perf-list-row">${escapeHtml(effect?.source || 'Effect')}: ${escapeHtml(effect?.value || effect?.desc || 'adjustment')}</div>`).join('')}
                            ${transforms.map((effect) => `<div class="perf-list-row">${escapeHtml(effect?.source || 'Effect')}: ${escapeHtml(effect?.desc || 'transform')}</div>`).join('')}
                        ` : '<div class="perf-empty-state small">No additional requirement or color transforms were stored.</div>'}
                    </div>
                </div>
                <div class="perf-effects-column">
                    <div class="perf-mini-heading">Score lines</div>
                    <div class="perf-list-block">
                        ${scoreLines.length > 0 ? scoreLines.map((line) => `
                            <div class="perf-score-line">
                                <span>${escapeHtml(line?.source || 'Score source')}</span>
                                <strong>+${line?.value || 0}</strong>
                            </div>
                        `).join('') : '<div class="perf-empty-state small">No score breakdown lines were stored.</div>'}
                    </div>
                </div>
                <div class="perf-effects-column">
                    <div class="perf-mini-heading">Triggered abilities carried into Live Result</div>
                    <div class="perf-list-block">
                        ${triggered.length > 0 ? triggered.map((ability) => {
                            const effectText = ability?.effect_text || '';
                            const condText = ability?.condition_text || '';
                            const abilityDisplay = effectText ? enrichText(effectText) : '';
                            const condDisplay = condText ? enrichText(condText) : '';
                            const triggeredType = effectText.includes('ライブ開始時') || effectText.includes('live_start') ? 'live_start' :
                                effectText.includes('ライブ成功時') || effectText.includes('live_success') ? 'live_success' :
                                effectText.includes('常時') || effectText.includes('jyouji') ? 'jyouji' : '';
                            return `
                                <div class="perf-list-row">
                                    <div class="effect-title-row">
                                        <strong>${escapeHtml(ability?.card_name || 'Unknown card')}</strong>
                                        ${triggeredType ? `<span class="effect-duration">${escapeHtml(triggeredType)}</span>` : ''}
                                    </div>
                                    ${abilityDisplay ? `<div class="perf-bonus-text" style="margin-top: 4px; margin-left: 0;">${abilityDisplay}</div>` : ''}
                                    ${condDisplay ? `<div class="perf-ability-condition">Condition: ${condDisplay}</div>` : ''}
                                </div>
                            `;
                        }).join('') : '<div class="perf-empty-state small">No triggered abilities were recorded.</div>'}
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

    const totalHearts = sumHearts(result.total_hearts);
    const baseLiveScore = sumPassedLiveScores(lives);
    
    let outcome = 'Failed performance';
    if (isCannotLive) {
        outcome = 'Live cannot happen';
    } else if (isSuccess) {
        const winsKey = playerId === 0 ? 'p0_wins' : 'p1_wins';
        const otherWinsKey = playerId === 0 ? 'p1_wins' : 'p0_wins';
        const selfWins = !!result[winsKey];
        const otherWins = !!result[otherWinsKey];
        if (selfWins && otherWins) outcome = 'Comparative tie';
        else if (selfWins) outcome = 'Won live result';
        else if (otherWins) outcome = 'Lost live result';
        else outcome = 'Passed performance';
    }

    const members = result?.member_contributions || [];
    const totalBlades = members.reduce((s, m) => m.is_wait ? s : s + (m.base_blades || 0) + (m.bonus_blades || 0), 0);

    return `
        <article class="perf-panel ${isSuccess ? 'success' : 'failure'}">
            <header class="perf-panel-header">
                <div class="perf-panel-header-main">
                    <div class="perf-eyebrow">${escapeHtml(getPlayerName(playerId))}</div>
                    <h2>${escapeHtml(outcome)}</h2>
                    ${isCannotLive
                        ? `<div class="perf-panel-subtitle cannot-live-subtitle">${cannotLiveCardName ? `Due to ${escapeHtml(cannotLiveCardName)}'s ability` : 'Due to a restriction effect'}</div>`
                        : `<div class="perf-panel-subtitle">Judge score ${result?.total_score || 0} with ${passedLives}/${totalLives} live cards passing.</div>`
                    }
                </div>
                <div class="perf-panel-statuses">
                    ${isCannotLive
                        ? '<div class="perf-status-pill blocked">BLOCKED</div>'
                        : `<div class="perf-status-pill ${isSuccess ? 'success' : 'failure'}">${isSuccess ? 'PASS' : 'FAIL'}</div>`
                    }
                    <div class="perf-outcome-pill">${escapeHtml(outcome)}</div>
                </div>
            </header>
 
            <section class="perf-score-hero" style="border-bottom: 1px solid var(--border); margin-bottom: 16px; padding-bottom: 12px;">
                <div class="perf-metric-grid">
                    <div class="perf-metric-card highlight">
                        <div class="perf-metric-label">JUDGE SCORE</div>
                        <div class="perf-metric-value" style="font-size: 1.8rem;">${result?.total_score || 0}</div>
                    </div>
                    <div class="perf-metric-card">
                        <div class="perf-metric-label">HEART VECTOR</div>
                        <div class="perf-metric-value">
                            ${renderHeartsCompact(result?.total_hearts || [])}
                            <span class="total-count-dim">(${totalHearts})</span>
                        </div>
                    </div>
                    ${renderTextMetric('Lives Passed', `${passedLives} / ${totalLives}`)}
                    ${renderIconMetric('img/texticon/icon_score.png', 'Live Pts', String(baseLiveScore), 'score')}
                    ${renderIconMetric('img/texticon/icon_score.png', 'Notes', `${result?.note_icons || 0}`, 'notes')}
                    ${renderIconMetric('img/texticon/icon_blade.png', 'Stage Blades', String(totalBlades), 'blades')}
                    ${renderIconMetric('img/texticon/icon_blade.png', 'Yell Count', String(result?.yell_count || 0), 'yells')}
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
                    ${liveImgSrc ? `<div class="perf-live-art-wrapper md"><img src="${liveImgSrc}" alt="${escapeHtml(live.name || 'Live')}"></div>` : ''}
                    <div class="perf-guide-info">
                        <div class="perf-guide-name">${escapeHtml(live.name || 'Live')} <span class="perf-guide-score">(${live.score || 0} pts)</span></div>
                        <div class="perf-guide-pips">${renderHeartProgress(live.filled, live.required)}</div>
                        ${!live.passed && live.reason ? `<div class="perf-guide-reason">${escapeHtml(live.reason)}</div>` : ''}
                    </div>
                    <div class="perf-guide-status" style="color:${live.passed ? '#78d08b' : '#f26d6d'}">${live.passed ? '✓ READY' : '✗ RISK'}</div>
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
        if (phase === Phase.LIVE_CARD_SET) return 'live_set';
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
