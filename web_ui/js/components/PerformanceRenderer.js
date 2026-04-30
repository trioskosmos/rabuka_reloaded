/**
 * Performance Renderer Component
 * Renders a player-facing explanation of the performance phase using the
 * snapshot emitted by the Rust engine.
 */
import { State } from '../state.js';
import { fixImg, Phase, isMulliganPhase } from '../constants.js';
import * as i18n from '../i18n/index.js';
import { Tooltips } from '../ui_tooltips.js';
import { TextEnricher } from '../utils/TextEnricher.js';

const HEART_LABELS = ['Any', 'Pink', 'Red', 'Yellow', 'Green', 'Blue', 'Purple'];

const HEART_ICONS = HEART_LABELS.map((_, index) => `img/texticon/heart_0${index}.png`);

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

function getDisplayResults(results) {
    if (results && typeof results === 'object' && !Array.isArray(results)) {
        return results;
    }

    if (State.data?.performance_results && Object.keys(State.data.performance_results).length > 0) {
        return State.data.performance_results;
    }

    return State.data?.last_performance_results || {};
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
                <div class="heart-grid-cell color-${h.index}">
                    <img src="${h.icon}" class="heart-mini-icon" alt="${escapeHtml(h.label)}">
                    <span class="count-value">${h.count}</span>
                </div>
            `).join('')}
        </div>
    `;
}

function renderHeartsCompact(hearts) {
    if (!Array.isArray(hearts) || hearts.every((value) => !value)) {
        return '<span class="perf-empty-inline">none</span>';
    }

    return `<div class="hearts-compact">${hearts.map((count, index) => {
        if (!count) return '';
        return `
            <div class="heart-tag ${index === 0 ? 'color-any' : `color-${index}`}" title="${escapeHtml(HEART_LABELS[index])}">
                <img src="${HEART_ICONS[index]}" class="heart-mini-icon" alt="${escapeHtml(HEART_LABELS[index])}">
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

function renderSuccessEquation(have, need) {
    const haveValues = [...have];
    const needValues = need;
    const totalHave = sumHearts(haveValues);
    const totalNeed = sumHearts(needValues);

    let satisfied = 0;
    let wildcards = haveValues[0] || 0;
    let requiredTotal = 0;

    for (let color = 1; color < 7; color += 1) {
        const colorNeed = needValues[color] || 0;
        const colorHave = haveValues[color] || 0;
        requiredTotal += colorNeed;
        const directMatch = Math.min(colorHave, colorNeed);
        satisfied += directMatch;
        const deficit = colorNeed - directMatch;
        if (deficit > 0) {
            const wildcardMatch = Math.min(wildcards, deficit);
            satisfied += wildcardMatch;
            wildcards -= wildcardMatch;
        }
    }

    const anyNeed = needValues[6] || 0;
    requiredTotal += anyNeed;
    const anyMatch = Math.min(wildcards, anyNeed);
    satisfied += anyMatch;

    const success = satisfied >= requiredTotal;

    return `
        <div class="perf-match-summary ${success ? 'success' : 'failure'}">
            <div class="perf-math-sum">
                <div class="perf-math-operand">
                    <div class="perf-eyebrow">Goal</div>
                    <div class="perf-math-value">${totalNeed}</div>
                </div>
                <div class="perf-math-operator"> vs </div>
                <div class="perf-math-operand">
                    <div class="perf-eyebrow">Have</div>
                    <div class="perf-math-value">${totalHave}</div>
                </div>
                <div class="perf-math-operator"> = </div>
                <div class="perf-math-result">
                    <div class="perf-math-value">${success ? '✓' : '✗'}</div>
                </div>
            </div>
        </div>
    `;
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

function renderEngineFlow() {
    const steps = [
        {
            title: 'Reveal Live Zone',
            body: 'All three live slots flip first. Non-live cards in the live zone are discarded before the phase continues.'
        },
        {
            title: 'Check Live Start',
            body: 'FLAG_CANNOT_LIVE skips the whole performance. Otherwise, On Live Start effects resolve before any yell happens.'
        },
        {
            title: 'Count Blades and Yell',
            body: 'Stage members and cheer modifiers determine blade total. That many cards are yelled from deck into stage energy.'
        },
        {
            title: 'Total Hearts and Notes',
            body: 'The engine totals member hearts, yell hearts, note icons, color transforms, and requirement modifiers into one snapshot.'
        },
        {
            title: 'Judge Lives in Slot Order',
            body: 'Live 0, then 1, then 2. Hearts are consumed sequentially. One failed live discards the entire live zone.'
        },
        {
            title: 'Lock In Live Result',
            body: 'Performers rest, score lines are stored, then Live Result compares both players and marks who actually wins the phase.'
        }
    ];

    return `
        <section class="perf-engine-flow">
            <div class="perf-section-heading-row">
                <div>
                    <div class="perf-eyebrow">Engine Order</div>
                    <h3>How this phase resolves in code</h3>
                </div>
                <p>The copy below follows the Rust phase order in performance.rs, not a simplified mock flow.</p>
            </div>
            <div class="perf-flow-grid">
                ${steps.map((step, index) => `
                    <article class="perf-flow-step">
                        <div class="perf-flow-index">${index + 1}</div>
                        <div class="perf-flow-copy">
                            <h4>${escapeHtml(step.title)}</h4>
                            <p>${escapeHtml(step.body)}</p>
                        </div>
                    </article>
                `).join('')}
            </div>
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

function renderStoryCards(result) {
    const lives = result?.lives || [];
    const passedLives = countPassedLives(lives);
    const totalLives = lives.length;
    const totalHearts = sumHearts(result?.total_hearts || []);
    const yellCards = result?.yell_cards?.length || 0;
    const scoreLines = result?.breakdown?.scores?.length || 0;
    const bonusLines = result?.triggered_abilities?.length || 0;

    const cards = [
        {
            title: 'Board setup',
            body: `${totalHearts} total hearts and ${result?.note_icons || 0} note icons were available when the engine judged this player.`
        },
        {
            title: 'Yell window',
            body: `${result?.yell_count || 0} blades produced ${yellCards} yelled cards after reductions, and those cards immediately became stage energy.`
        },
        {
            title: 'Live check',
            body: `${passedLives} of ${totalLives} live cards passed. Because the engine checks lives sequentially, spare hearts only matter if earlier lives succeeded.`
        },
        {
            title: 'Score lock-in',
            body: `${scoreLines} explicit score lines and ${bonusLines} triggered abilities are recorded on this result snapshot.`
        }
    ];

    return `
        <section class="perf-section-card">
            <div class="perf-section-heading-row compact">
                <div>
                    <div class="perf-eyebrow">Phase Story</div>
                </div>
            </div>
            <div class="perf-story-grid">
                ${cards.map((card) => `
                    <article class="perf-story-card">
                        <h4>${escapeHtml(card.title)}</h4>
                        <p>${escapeHtml(card.body)}</p>
                    </article>
                `).join('')}
            </div>
        </section>
    `;
}

function renderLiveCards(result) {
    const lives = Array.isArray(result?.lives) ? result.lives : [];
    if (lives.length === 0) {
        return `
            <section class="perf-section-card">
                <div class="perf-section-heading-row compact">
                    <div>
                        <div class="perf-eyebrow">Live Checks</div>
                        <h3>Target lives</h3>
                    </div>
                </div>
                <div class="perf-empty-state">No live cards were stored in this snapshot.</div>
            </section>
        `;
    }

    return `
        <section class="perf-section-card">
            <div class="perf-section-heading-row compact">
                <div>
                    <div class="perf-eyebrow">Live Checks</div>
                </div>
            </div>
            <div class="perf-live-grid">
                ${lives.map((live, index) => {
                    const spare = live?.spare || [0, 0, 0, 0, 0, 0, 0];
                    const adjustments = live.adjustments;
                    const spareTotal = sumHearts(spare);
                    return `
                        <article class="perf-live-card ${live?.passed ? 'success' : 'failure'}">
                            <div class="perf-live-card-head">
                                <div class="perf-card-id-badge">Live ${index + 1}</div>
                                <div class="perf-live-card-title">
                                    // Support both img and img_path field names
                                    ${live?.img || live?.img_path ? `<img src="${fixImg(live.img || live.img_path)}" class="perf-live-art" alt="${escapeHtml(live.name || 'Live')}">` : ''}
                                    <div>
                                        <h4>${escapeHtml(live?.name || 'Live')}</h4>
                                        <div class="perf-live-card-meta">Printed score ${live?.score || 0}</div>
                                    </div>
                                </div>
                                <div class="perf-status-pill ${live?.passed ? 'success' : 'failure'}">${live?.passed ? 'PASS' : 'FAIL'}</div>
                            </div>
                            <div class="perf-live-grid-rows" style="gap: 4px;">
                                <div style="display: flex; gap: 8px; align-items: center;">
                                    <div class="perf-mini-heading" style="margin-bottom: 0;">Goal:</div>
                                    <div class="perf-hearts-grid">${renderHeartsGrid(live?.required || [])}</div>
                                </div>
                                <div style="display: flex; gap: 8px; align-items: center;">
                                    <div class="perf-mini-heading" style="margin-bottom: 0;">Hit:</div>
                                    <div class="perf-hearts-grid">${renderHeartsGrid(live?.filled || [])}</div>
                                </div>
                            </div>
                            ${renderSuccessEquation(live?.filled || [], live?.required || [])}
                            ${spareTotal > 0 ? `
                                <div class="perf-live-footnote">
                                    ${spareTotal} spare hearts remained after this card's simulated slot-order check.
                                </div>
                            ` : ''}
                            ${adjustments.length > 0 ? `
                                <div class="perf-pill-list">
                                    ${adjustments.map((adjustment) => {
                                        const isTransform = adjustment?.type === 'transform' || adjustment?.type === 'override';
                                        const value = adjustment?.desc || `${adjustment?.value > 0 ? '+' : ''}${adjustment?.value || 0} ${HEART_LABELS[adjustment?.color ?? 0] || 'heart'}`;
                                        return `<div class="perf-adjustment-pill ${isTransform ? 'transform' : 'requirement'}">${escapeHtml(adjustment?.source || 'Effect')}: ${escapeHtml(value)}</div>`;
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

function renderAllocationSection(result) {
    const allocations = result.breakdown.allocations;
    if (allocations.length === 0) {
        return `
            <section class="perf-section-card">
                <div class="perf-section-heading-row compact">
                    <div>
                        <div class="perf-eyebrow">Heart Routing</div>
                        <h3>Which sources paid which live</h3>
                    </div>
                </div>
                <div class="perf-empty-state">No source-to-live allocation map was stored for this result.</div>
            </section>
        `;
    }

    const groups = new Map();
    allocations.forEach((allocation) => {
        const key = `${allocation?.target_idx ?? -1}`;
        if (!groups.has(key)) {
            groups.set(key, []);
        }
        groups.get(key).push(allocation);
    });

    return `
        <section class="perf-section-card">
            <div class="perf-section-heading-row compact">
                <div>
                    <div class="perf-eyebrow">Heart Routing</div>
                </div>
            </div>
            <div class="perf-route-grid">
                ${[...groups.entries()].map((entry) => {
                    const [targetIndex, rows] = entry;
                    const title = rows[0]?.target_name || `Live ${Number(targetIndex) + 1}`;
                    return `
                        <article class="perf-route-card">
                            <h4>${escapeHtml(title)}</h4>
                            <div class="perf-route-list">
                                ${rows.map((row) => `
                                    <div class="perf-route-row">
                                        <div class="perf-route-source ${row?.source_type === 'yell' ? 'yell' : ''}">
                                            <span class="perf-route-source-name">${escapeHtml(row?.source_name || 'Source')}</span>
                                            <span class="perf-route-source-meta">${row?.source_type === 'yell' ? 'Yell card' : `Stage slot ${Number(row?.source_slot ?? -1) + 1}`}</span>
                                        </div>
                                        <div class="perf-route-payment">
                                            <img src="${HEART_ICONS[row?.wildcard ? 0 : (row?.color ?? 0)]}" class="heart-mini-icon" alt="">
                                            <strong>${row?.amount || 0}</strong>
                                            ${row?.is_bonus ? '<span class="perf-badge bonus">bonus</span>' : '<span class="perf-badge base">printed</span>'}
                                            ${row?.wildcard ? '<span class="perf-badge wildcard">wildcard</span>' : ''}
                                        </div>
                                    </div>
                                `).join('')}
                            </div>
                        </article>
                    `;
                }).join('')}
            </div>
        </section>
    `;
}

function renderContributionSection(result) {
    const members = result.member_contributions;
    if (members.length === 0) {
        return `
            <section class="perf-section-card">
                <div class="perf-section-heading-row compact">
                    <div>
                        <div class="perf-eyebrow">Stage Contributors</div>
                        <h3>Members on stage</h3>
                    </div>
                </div>
                <div class="perf-empty-state">No stage member contribution breakdown is stored for this snapshot.</div>
            </section>
        `;
    }

    return `
        <section class="perf-section-card">
            <div class="perf-section-heading-row compact">
                <div>
                    <div class="perf-eyebrow">Stage Contributors</div>
                </div>
            </div>
            <div class="perf-contrib-grid">
                ${members.map((member) => {
                    const heartBonuses = member.ability_heart_bonuses;
                    const bladeBonuses = member.ability_blade_bonuses;
                    return `
                        <article class="perf-contrib-card" data-member-id="${member?.source_id ?? ''}" data-member-slot="${member?.slot ?? ''}">
                            <div class="perf-contrib-header">
                                ${member?.img ? `<img src="${fixImg(member.img)}" class="perf-contrib-art" alt="${escapeHtml(member?.source || 'Member')}">` : ''}
                                <div>
                                    <h4>${escapeHtml(member?.source || 'Member')}</h4>
                                    <div class="perf-live-card-meta">Stage slot ${Number(member?.slot ?? -1) + 1}</div>
                                </div>
                            </div>
                            <div class="perf-contrib-stats">
                                <div>
                                    <div class="perf-mini-heading">Printed hearts</div>
                                    ${renderHeartsCompact(member?.base_hearts || [])}
                                </div>
                                <div>
                                    <div class="perf-mini-heading">Bonus hearts</div>
                                    ${renderHeartsCompact(member?.bonus_hearts || [])}
                                </div>
                                <div>
                                    <div class="perf-mini-heading">Printed blades</div>
                                    ${renderBladesCompact(member?.base_blades || 0)}
                                </div>
                                <div>
                                    <div class="perf-mini-heading">Bonus blades</div>
                                    ${member?.bonus_blades > 0 ? renderBladesCompact(member.bonus_blades) : '<span class="perf-empty-inline">0</span>'}
                                </div>
                                <div>
                                    <div class="perf-mini-heading">Notes</div>
                                    <div class="perf-stat-pill">${member?.base_notes || 0}${member?.bonus_notes ? ` (+${member.bonus_notes})` : ''}</div>
                                </div>
                                <div>
                                    <div class="perf-mini-heading">Draw icons</div>
                                    <div class="perf-stat-pill">${member?.draw_icons || 0}</div>
                                </div>
                            </div>
                            ${(heartBonuses.length > 0 || bladeBonuses.length > 0) ? `
                                <div class="perf-bonus-block">
                                    ${heartBonuses.map((bonus) => `
                                        <div class="perf-bonus-item">
                                            <div class="perf-bonus-title">${escapeHtml(bonus?.source || 'Effect')} +${bonus?.amount || 0} ${escapeHtml(HEART_LABELS[bonus?.color ?? 0] || 'heart')}</div>
                                            ${bonus?.ability_text ? `<div class="perf-bonus-text">${enrichText(bonus.ability_text)}</div>` : ''}
                                        </div>
                                    `).join('')}
                                    ${bladeBonuses.map((bonus) => `
                                        <div class="perf-bonus-item">
                                            <div class="perf-bonus-title">${escapeHtml(bonus?.source || 'Effect')} +${bonus?.amount || bonus?.value || 0} blade</div>
                                            ${bonus?.ability_text ? `<div class="perf-bonus-text">${enrichText(bonus.ability_text)}</div>` : ''}
                                        </div>
                                    `).join('')}
                                </div>
                            ` : ''}
                        </article>
                    `;
                }).join('')}
            </div>
        </section>
    `;
}

function renderYellSection(result) {
    const yellCards = result.yell_cards;
    const heartSources = result.breakdown.hearts;
    const bladeSources = result.breakdown.blades;

    return `
        <section class="perf-section-card">
            <div class="perf-section-heading-row compact">
                <div>
                    <div class="perf-eyebrow">Yell and Source Pool</div>
                    <h3>Cards and values that fed this performance</h3>
                </div>
                <p>Stage cards, yelled cards, and blade-note sources are shown separately because the engine records them separately.</p>
            </div>
            <div class="perf-source-lists">
                <div>
                    <div class="perf-mini-heading">Heart sources</div>
                    <div class="perf-chip-list">
                        ${heartSources.length > 0 ? heartSources.map((item) => `
                            <div class="perf-source-chip ${item?.type === 'yell' ? 'yell' : ''}">
                                <span>${escapeHtml(item?.source || 'Source')}</span>
                                ${renderHeartsCompact(item?.value || [])}
                            </div>
                        `).join('') : '<div class="perf-empty-state small">No heart source breakdown saved.</div>'}
                    </div>
                </div>
                <div>
                    <div class="perf-mini-heading">Blade and note sources</div>
                    <div class="perf-chip-list">
                        ${bladeSources.length > 0 ? bladeSources.map((item) => `
                            <div class="perf-source-chip ${item?.type === 'yell' ? 'yell' : ''}">
                                <span>${escapeHtml(item?.source || 'Source')}</span>
                                <strong>+${item?.value || 0}</strong>
                            </div>
                        `).join('') : '<div class="perf-empty-state small">No blade breakdown saved.</div>'}
                    </div>
                </div>
            </div>
            <div class="perf-yell-gallery">
                ${yellCards.length > 0 ? yellCards.map((card) => {
                    const rawText = Tooltips.getEffectiveRawText(card);
                    return `
                        <article class="perf-yell-card-modern" ${card?.id !== undefined ? `data-card-id="${card.id}"` : ''} ${rawText ? `data-text="${escapeHtml(rawText)}"` : ''}>
                            <div class="perf-yell-card-art-wrap">
                                ${card?.img ? `<img src="${fixImg(card.img)}" alt="Yell card">` : ''}
                            </div>
                            <div class="perf-yell-icons">
                                ${renderHeartsCompact(card?.blade_hearts || [])}
                                ${(card?.note_icons || 0) > 0 ? `<span class="perf-badge note">${card.note_icons}♪</span>` : ''}
                                ${(card?.draw_icons || 0) > 0 ? `<span class="perf-badge draw">${card.draw_icons}⎋</span>` : ''}
                            </div>
                        </article>
                    `;
                }).join('') : '<div class="perf-empty-state">No yelled cards were recorded for this snapshot.</div>'}
            </div>
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
                            let abilityText = '';
                            try {
                                const card = Tooltips.findCardById(ability?.source_card_id);
                                if (card) {
                                    const rawText = TextEnricher.getEffectiveRawText(card);
                                    if (rawText) {
                                        abilityText = enrichText(rawText);
                                    }
                                }
                            } catch (e) {
                                // Fallback if card lookup fails
                            }
                            return `
                                <div class="perf-list-row">
                                    <strong>${escapeHtml(ability?.name || 'Triggered ability')}</strong>
                                    <span>${escapeHtml(ability?.card_name || 'Unknown card')}</span>
                                    ${abilityText ? `<div class="perf-bonus-text" style="margin-top: 4px; margin-left: 0;">${abilityText}</div>` : ''}
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
    const lives = result.lives;
    const passedLives = countPassedLives(lives);
    const totalLives = lives.length;
    const totalHearts = sumHearts(result.total_hearts);
    const baseLiveScore = sumPassedLiveScores(lives);
    const outcome = getOutcomeLabel(playerId, result);

    return `
        <article class="perf-panel ${result?.success ? 'success' : 'failure'}">
            <header class="perf-panel-header">
                <div class="perf-panel-header-main">
                    <div class="perf-eyebrow">${escapeHtml(getPlayerName(playerId))}</div>
                    <h2>${escapeHtml(outcome)}</h2>
                    <div class="perf-panel-subtitle">Judge score ${result?.total_score || 0} with ${passedLives}/${totalLives} live cards passing.</div>
                </div>
                <div class="perf-panel-statuses">
                    <div class="perf-status-pill ${result?.success ? 'success' : 'failure'}">${result?.success ? 'PASS' : 'FAIL'}</div>
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
                    ${renderIconMetric('img/texticon/icon_score.png', 'Notes', String(result?.note_icons || 0), 'notes')}
                    ${renderIconMetric('img/texticon/icon_blade.png', 'Blades', String(result?.yell_count || 0), 'blades')}
                </div>
            </section>

            <div class="perf-panel-body-grid">
                <div class="perf-body-column-left">
                    ${renderStoryCards(result)}
                    ${renderLiveCards(result)}
                </div>
                <div class="perf-body-column-right">
                    ${renderContributionSection(result)}
                    ${renderYellSection(result)}
                    ${renderAllocationSection(result)}
                    ${renderEffectsSection(result)}
                </div>
            </div>
        </article>
    `;
}

export const PerformanceRenderer = {
    renderHeartProgress,

    renderPerformanceGuide: () => {
        const state = State.data;
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
        let html = `
            <div class="perf-guide-header">
                <span>Blades: <b>${guide.total_blades}</b></span>
                <span>Hearts: ${renderHeartsCompact(guide.total_hearts)}</span>
            </div>
        `;

        guide.lives.forEach((live) => {
            if (!live || typeof live !== 'object') return;
            html += `
                <div class="perf-guide-entry" style="opacity:${live.passed ? 1 : 0.72}">
                    // Support both img and img_path field names
                    ${live.img || live.img_path ? `<img src="${fixImg(live.img || live.img_path)}" class="perf-guide-img" alt="${escapeHtml(live.name || 'Live')}">` : ''}
                    <div class="perf-guide-info">
                        <div class="perf-guide-name">${escapeHtml(live.name || 'Live')} <span class="perf-guide-score">(${live.score || 0} pts)</span></div>
                        <div class="perf-guide-pips">${renderHeartProgress(live.filled, live.required)}</div>
                        ${!live.passed && live.reason ? `<div class="perf-guide-reason">${escapeHtml(live.reason)}</div>` : ''}
                    </div>
                    <div class="perf-guide-status" style="color:${live.passed ? '#78d08b' : '#f26d6d'}">${live.passed ? 'READY' : 'RISK'}</div>
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
                engineContent.innerHTML = renderEngineFlow();
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
    renderSuccessEquation,
};
