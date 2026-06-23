import { State } from '../state.js';
import * as i18n from '../i18n/index.js';
import { Tooltips } from '../ui_tooltips.js';
import { LogFilter } from '../utils/LogFilter.js';
import { PerformanceMonitor } from '../utils/PerformanceMonitor.js';
import { LogViewerModal } from '../modals/LogViewerModal.js';
import { ModalManager } from '../utils/ModalManager.js';
import { DOM_IDS } from '../constants_dom.js';
import { fixImg } from '../constants.js';
import { resolveCardImagePath, CardRenderer } from './CardRenderer.js';

const Phase = {
    RPS: 0, SETUP: 1, MULLIGAN_P1: 2, MULLIGAN_P2: 3,
    ACTIVE: 4, ENERGY: 5, DRAW: 6, MAIN: 7,
    LIVE_SET: 8, PERFORMANCE_P1: 9, PERFORMANCE_P2: 10, LIVE_RESULT: 11
};

export const LogRenderer = {
    renderRuleLog: (containerId = 'rule-log') => {
        const ruleLogEl = document.getElementById(containerId);
        if (!ruleLogEl) return;

        const state = State.data;
        if (!state) return;

        const logData = state.rule_log || [];
        const logHash = logData.length + '|' + (logData.length > 0 ? logData[logData.length - 1] : '') + '|' + (State.selectedTurn || -1);
        if (logHash === LogRenderer._lastLogHash && !State.showingFullLog) return;
        LogRenderer._lastLogHash = logHash;

        PerformanceMonitor.startPerfMeasure();

        const currentLang = State.currentLang;
        const showFriendlyAbilities = State.showFriendlyAbilities;
        const selectedTurn = State.selectedTurn || -1;
        const showingFullLog = State.showingFullLog;

        ruleLogEl.innerHTML = '';
        const fragment = document.createDocumentFragment();

        // === SECTION 1: Turn History ===
        const turnHistorySection = LogRenderer.renderTurnHistorySection(state, selectedTurn);
        if (turnHistorySection) {
            fragment.appendChild(turnHistorySection);
        }

        // === SECTION 3: Rule Log ===
        const ruleLogSection = LogRenderer.renderRuleLogSection(state, currentLang, showFriendlyAbilities, selectedTurn);
        if (ruleLogSection) {
            fragment.appendChild(ruleLogSection);
        }

        ruleLogEl.appendChild(fragment);
        if (!showingFullLog) ruleLogEl.scrollTop = 0;

        PerformanceMonitor.endPerfMeasure();
    },

    renderTurnHistorySection: (state, selectedTurn) => {
        if (!state) return null;
        const history = state.turn_history || state.turn_events || [];
        if (!history || history.length === 0) return null;

        const filteredHistory = LogFilter.applyFilters(history);

        if (filteredHistory.length === 0) return null;

        const section = document.createElement('div');
        section.className = 'log-section turn-history-section';

        const header = document.createElement('div');
        header.className = 'log-section-header';
        header.textContent = i18n.t('turn_history');
        section.appendChild(header);

        filteredHistory.forEach(event => {
            const entry = LogRenderer.createTurnEventElement(event);
            section.appendChild(entry);
        });

        PerformanceMonitor.recordEntryCount(filteredHistory.length);
        return section;
    },

    createTurnEventElement: (event) => {
        const entry = document.createElement('div');
        const typeClass = event.event_type ? event.event_type.toLowerCase() : 'generic';
        entry.className = `log-entry turn-event ${typeClass}`;

        const playerLabel = event.player_id === State.perspectivePlayer
            ? i18n.t('you')
            : i18n.t('opponent');

        const phaseKey = LogRenderer.getPhaseKey(event.phase);
        const phaseLabel = i18n.t(phaseKey);
        const eventIcon = LogRenderer.getEventIcon(event.event_type);

        entry.setAttribute('role', 'logentry');
        entry.setAttribute('aria-live', 'polite');
        entry.setAttribute('aria-label', `Turn ${event.turn}, ${phaseLabel}, ${playerLabel}: ${event.event_type} - ${event.description || ''}`);

        const card = event.card_id !== undefined ? Tooltips.findCardById(event.card_id) : null;
        let eventDesc = event.description || '';
        if ((event.event_type === 'TRIGGER' || event.event_type === 'EFFECT') && card && !State.showFriendlyAbilities) {
            const rawText = Tooltips.getEffectiveRawText(card);
            if (rawText) {
                eventDesc = rawText;
            }
        }

        const container = document.createElement('div');
        container.className = 'turn-event-hover-container';
        container.style.display = 'contents';
        Tooltips.attachCardData(container, card);

        if (eventDesc) container.setAttribute('data-text', eventDesc);
        if (event.card_name) container.setAttribute('data-card-name', event.card_name);

        const enrichedDesc = Tooltips.enrichAbilityText(eventDesc);

        container.innerHTML = `
            <span class="turn-badge" aria-label="Turn ${event.turn}">T${event.turn}</span>
            <span class="phase-badge" aria-label="Phase: ${phaseLabel}">${phaseLabel}</span>
            <span class="player-badge p${event.player_id}" aria-label="Player: ${playerLabel}">${playerLabel}</span>
            <span class="event-type" aria-label="Event type: ${event.event_type || 'Event'}">${eventIcon} ${event.event_type || 'Event'}</span>
            <span class="event-desc">${enrichedDesc}</span>
        `;
        entry.appendChild(container);

        return entry;
    },

    getPhaseKey: (phase) => {
        const perspectivePlayer = State.perspectivePlayer;
        switch (phase) {
            case Phase.ROCK_PAPER_SCISSORS: return 'rps';
            case Phase.MULLIGAN: return 'mulligan';
            case Phase.ACTIVE: return 'active';
            case Phase.ENERGY: return 'energy';
            case Phase.DRAW: return 'draw';
            case Phase.MAIN: return 'main';
            case Phase.LIVE_CARD_SET: return 'live_set';
            case Phase.FIRST_ATTACKER_PERFORMANCE: return perspectivePlayer === 0 ? 'perf_p1' : 'perf_p2';
            case Phase.SECOND_ATTACKER_PERFORMANCE: return perspectivePlayer === 1 ? 'perf_p1' : 'perf_p2';
            case Phase.LIVE_VICTORY_DETERMINATION: return 'live_result';
            default: return String(phase);
        }
    },

    getEventIcon: (eventType) => {
        const icons = {
            'PLAY': '🃏', 'ACTIVATE': '⚡', 'TRIGGER': '🎯', 'EFFECT': '✨', 'RULE': '📜', 'YELL': '📣', 'PERFORMANCE': '🎤',
            'PHASE': '🔄', 'DRAW': '📥', 'SCORE': '📊', 'HEART': '💖', 'BATON': ' Baton', 'LIVE': '🎵'
        };
        return icons[eventType] || '•';
    },

    _lastLogHash: '',

    renderRuleLogSection: (state, currentLang, showFriendlyAbilities, selectedTurn) => {
        let logData = state.rule_log || [];
        let structData = state.structured_log || [];
        if (logData.length > 200) logData = logData.slice(-200);
        if (structData.length > 200) structData = structData.slice(-200);

        if (selectedTurn !== -1) {
            const turnStr = `[Turn ${selectedTurn}]`;
            logData = logData.filter(entry => entry.includes(turnStr));
            structData = structData.filter(e => e.turn === selectedTurn);
        }

        // Build lookup from log text to structured entry
        const structLookup = new Map();
        structData.forEach(s => structLookup.set(s.text, s));

        const section = document.createElement('div');
        section.className = 'log-section rule-log-section';

        const header = document.createElement('div');
        header.className = 'log-section-header';
        header.textContent = i18n.t('rule_log');
        section.appendChild(header);

        // Jyouji (constant/常時) ability status bar at the top of the log
        const constantStatuses = state.constant_ability_statuses || [];
        if (constantStatuses.length > 0) {
            const jyoujiBar = document.createElement('div');
            jyoujiBar.className = 'jyouji-status-bar';
            jyoujiBar.innerHTML = `<span class="jyouji-bar-label">⚡ 常時:</span> `;
            constantStatuses.forEach((cs, idx) => {
                const allMet = cs.all_conditions_met;
                const icon = allMet ? '🟢' : '🔴';
                const zoneLabel = cs.zone === 'stage' ? 'S' : cs.zone === 'live_card_zone' ? 'L' : '?';
                jyoujiBar.innerHTML += `<span class="jyouji-pill ${allMet ? 'active' : 'inactive'}" title="${Tooltips.enrichAbilityText(cs.ability_text)}">
                    ${icon} ${cs.card_name} [${zoneLabel}]
                </span>`;
                if (idx < constantStatuses.length - 1) jyoujiBar.innerHTML += ' ';
            });
            section.appendChild(jyoujiBar);
        }

        if (logData.length === 0) {
            const emptyMsg = document.createElement('div');
            emptyMsg.className = 'log-empty-message';
            emptyMsg.textContent = i18n.t('no_logs') || 'No log entries yet';
            section.appendChild(emptyMsg);
            return section;
        }

        let groupedLogs = [];
        let currentGroup = null;
        let currentAbGroup = null;
        let currentSnapshot = null;

        const flushAbGroup = () => {
            if (currentAbGroup) {
                groupedLogs.push({ type: 'ability_debug', ...currentAbGroup });
                currentAbGroup = null;
            }
        };

        const flushSnapshot = () => {
            if (currentSnapshot) {
                groupedLogs.push({ type: 'snapshot', ...currentSnapshot });
                currentSnapshot = null;
            }
        };

        const logArray = [...logData];
        logArray.reverse();
        logArray.forEach((entry, revIdx) => {
            const idMatch = entry.match(/\[Turn \d+\] \[ID: (\d+)\] (.*)/);
            const executionId = idMatch ? idMatch[1] : null;
            const body = idMatch ? idMatch[2] : entry.replace(/^\[Turn \d+\]\s*/, '');
            const turnMatch = entry.match(/^\[Turn \d+\]/);
            const turnPrefix = turnMatch ? turnMatch[0] : "";

            if (executionId) {
                flushAbGroup();
                flushSnapshot();
                if (!currentGroup || currentGroup.id !== executionId) {
                    currentGroup = { id: executionId, entries: [], turnPrefix };
                    groupedLogs.push(currentGroup);
                }
                currentGroup.entries.push(body);
            } else if (body.startsWith('[AB]')) {
                flushAbGroup();
                flushSnapshot();
                currentGroup = null;
                const abLine = body.replace(/^\[AB\]\s*/, '');
                if (/^ABILITY\s/.test(abLine)) {
                    flushAbGroup();
                    const cardMatch = abLine.match(/^ABILITY\s+"([^"]+)"/);
                    currentAbGroup = {
                        cardName: cardMatch ? cardMatch[1] : '',
                        header: abLine,
                        entries: [],
                        turnPrefix
                    };
                } else if (currentAbGroup) {
                    currentAbGroup.entries.push(abLine);
                }
            } else if (body.includes('Performance ──') || (currentSnapshot && body.startsWith(' '))) {
                // Snapshot block: header starts a new block, indented lines continue it
                flushAbGroup();
                currentGroup = null;
                if (body.includes('Performance ──')) {
                    flushSnapshot();
                    currentSnapshot = { header: body, entries: [], turnPrefix };
                } else if (currentSnapshot) {
                    currentSnapshot.entries.push(body);
                } else {
                    groupedLogs.push({ entry, body, turnPrefix, structEntry: structLookup.get(entry) || null });
                }
            } else {
                flushAbGroup();
                flushSnapshot();
                currentGroup = null;
                groupedLogs.push({ entry, body, turnPrefix, structEntry: structLookup.get(entry) || null });
            }
        });
        flushAbGroup();
        flushSnapshot();

        groupedLogs.forEach(group => {
            if (group.type === 'snapshot') {
                const block = LogRenderer.createSnapshotBlock(group, currentLang, showFriendlyAbilities);
                section.appendChild(block);
            } else if (group.type === 'ability_debug') {
                const block = LogRenderer.createAbilityDebugBlock(group, currentLang, showFriendlyAbilities);
                section.appendChild(block);
            } else if (group.entries) {
                const block = LogRenderer.createGroupedLogBlock(group, currentLang, showFriendlyAbilities);
                section.appendChild(block);
            } else {
                const entry = LogRenderer.createStandaloneLogEntry(group, currentLang, showFriendlyAbilities);
                section.appendChild(entry);
            }
        });

        // Render structured ability resolution entries (from structured_log, category=ability_resolution)
        const abilityResolutions = (state.structured_log || []).filter(
            e => e.category === 'ability_resolution' && e.metadata
        );
        abilityResolutions.forEach(entry => {
            const block = LogRenderer.createAbilityResolutionBlock(entry, currentLang, showFriendlyAbilities);
            if (block) section.appendChild(block);
        });

        // Render trigger evaluation entries (from structured_log, category=trigger_evaluation)
        const triggerEvals = (state.structured_log || []).filter(
            e => e.category === 'trigger_evaluation' && e.metadata
        );
        triggerEvals.forEach(entry => {
            const block = LogRenderer.createTriggerEvaluationBlock(entry, currentLang, showFriendlyAbilities);
            if (block) section.appendChild(block);
        });

        PerformanceMonitor.recordEntryCount(groupedLogs.length + abilityResolutions.length);
        return section;
    },

    createAbilityResolutionBlock: (entry, currentLang, showFriendlyAbilities) => {
        const meta = entry.metadata;
        if (!meta || !meta.items) return null;

        const blockDiv = document.createElement('div');
        blockDiv.className = 'log-group-block ability-resolution-block';

        // Header
        const headerDiv = document.createElement('div');
        headerDiv.className = 'log-entry ability group-header';
        const resultClass = meta.result === 'success' ? 'ability-pass' : 'ability-fail';
        const resultIcon = meta.result === 'success' ? '✓' : '✗';
        const triggerText = meta.trigger || '?';
        const zoneLabel = meta.zone === 'stage' ? 'ステージ' : meta.zone === 'live_card_zone' ? 'ライブ置場' : meta.zone === 'success_live_card_zone' ? '成功ライブ置場' : meta.zone || '';
        const cardName = entry.source_card_name || meta.card_name || '';
        headerDiv.innerHTML = `
            <div class="log-entry-icon"> </div>
            <div class="log-entry-content">
                <span class="${resultClass}">${resultIcon}</span>
                <strong>${cardName}</strong>
                ${zoneLabel ? `<span class="ability-zone">[${zoneLabel}]</span>` : ''}:
                <span class="ability-trigger">${triggerText}</span>
            </div>
            <div class="log-group-toggle">▼</div>
        `;
        blockDiv.appendChild(headerDiv);

        // Details
        const detailsContainer = document.createElement('div');
        detailsContainer.className = 'log-group-details';
        detailsContainer.style.display = 'block';

        meta.items.forEach(item => {
            LogRenderer._renderAbilityLogItem(item, detailsContainer);
        });

        blockDiv.appendChild(detailsContainer);

        // Toggle on click
        headerDiv.style.cursor = 'pointer';
        headerDiv.onclick = () => {
            const isHidden = detailsContainer.style.display === 'none';
            detailsContainer.style.display = isHidden ? 'block' : 'none';
            headerDiv.querySelector('.log-group-toggle').textContent = isHidden ? '▼' : '▶';
        };

        return blockDiv;
    },

    createTriggerEvaluationBlock: (entry, currentLang, showFriendlyAbilities) => {
        const meta = entry.metadata;
        if (!meta) return null;

        const blockDiv = document.createElement('div');
        blockDiv.className = 'log-group-block trigger-evaluation-block';

        const headerDiv = document.createElement('div');
        headerDiv.className = 'log-entry ability group-header';
        const triggerText = meta.trigger || '?';
        const zoneLabel = meta.zone === 'stage' ? 'ステージ' : meta.zone === 'live_card_zone' ? 'ライブ置場' : meta.zone || '?';
        headerDiv.innerHTML = `
            <div class="log-entry-icon"> </div>
            <div class="log-entry-content">
                <span class="ability-scan">🔍</span>
                <strong>${entry.source_card_name || ''}</strong>
                <span class="ability-zone">[${zoneLabel}]</span>:
                能力確認 [${triggerText}]
            </div>
            <div class="log-group-toggle">▼</div>
        `;
        blockDiv.appendChild(headerDiv);

        const detailsContainer = document.createElement('div');
        detailsContainer.className = 'log-group-details';
        detailsContainer.style.display = 'block';
        detailsContainer.innerHTML = `<div class="log-entry effect detail">
            <span class="ability-cond-text">結果: ${meta.result === 'pending' ? '条件評価待ち' : meta.result}</span>
        </div>`;
        blockDiv.appendChild(detailsContainer);

        headerDiv.style.cursor = 'pointer';
        headerDiv.onclick = () => {
            const isHidden = detailsContainer.style.display === 'none';
            detailsContainer.style.display = isHidden ? 'block' : 'none';
            headerDiv.querySelector('.log-group-toggle').textContent = isHidden ? '▼' : '▶';
        };

        return blockDiv;
    },

    _renderAbilityLogItem: (item, container) => {
        if (!item) return;
        const div = document.createElement('div');
        div.className = 'log-entry effect detail ability-log-item';

        switch (item.kind || 'Condition') {
            case 'Condition': {
                const sub = item.children && item.children.length > 0;
                const iconChar = item.passed ? '✓' : '✗';
                let html = `<div class="ability-cond-row">
                    <span class="ability-cond-icon">${iconChar}</span>
                    <span class="ability-cond-text">${item.text || ''}</span>
                    <span class="ability-cond-type">[${item.type || ''}]</span>
                </div>`;
                if (item.expectation || item.actual) {
                    html += `<div class="ability-cond-detail">
                        <span class="ability-label">期待:</span>
                        <span class="ability-value">${item.expectation || '—'}</span>
                        <span class="ability-label">実際:</span>
                        <span class="ability-value">${item.actual || '—'}</span>
                        <span class="ability-result ${item.passed ? 'pass' : 'fail'}">
                            ${iconChar}
                        </span>
                    </div>`;
                }
                if (sub) {
                    html += `<div class="ability-sub-items">`;
                    div.innerHTML = html;
                    container.appendChild(div);
                    item.children.forEach(child => {
                        LogRenderer._renderAbilityLogItem(child, container.lastChild || container);
                    });
                    const closeDiv = document.createElement('div');
                    closeDiv.className = 'ability-sub-close';
                    container.appendChild(closeDiv);
                    return;
                }
                div.innerHTML = html;
                break;
            }
            case 'Cost': {
                const iconChar = item.passed ? '✓' : '✗';
                div.innerHTML = `<div class="ability-cost-row">
                    <span class="ability-cond-icon"> </span>
                    <span class="ability-cond-text">${item.text || ''}</span>
                    <span class="ability-label">期待:</span>
                    <span class="ability-value">${item.expectation || '—'}</span>
                    <span class="ability-label">実際:</span>
                    <span class="ability-value">${item.actual || '—'}</span>
                    <span class="ability-result ${item.passed ? 'pass' : 'fail'}">${iconChar}</span>
                </div>`;
                break;
            }
            case 'Effect': {
                div.innerHTML = `<div class="ability-effect-row">
                    <span class="ability-cond-icon"> </span>
                    <span class="ability-cond-text">${item.text || ''}</span>
                    <span class="ability-effect-detail">${item.details || item.action || ''}</span>
                </div>`;
                break;
            }
            case 'KeyValue': {
                const iconChar = item.passed ? '✓' : '✗';
                div.innerHTML = `<div class="ability-kv-row">
                    <span class="ability-cond-icon"> </span>
                    <span class="ability-cond-text">${item.key}: ${item.value}</span>
                    <span class="ability-result ${item.passed ? 'pass' : 'fail'}">${iconChar}</span>
                </div>`;
                break;
            }
            default: {
                div.textContent = JSON.stringify(item);
            }
        }
        container.appendChild(div);
    },

    createGroupedLogBlock: (group, currentLang, showFriendlyAbilities) => {
        const blockDiv = document.createElement('div');
        blockDiv.className = 'log-group-block';

        let headerEntry = group.entries[0];
        let detailEntries = group.entries.slice(1);

        const headerDiv = document.createElement('div');
        headerDiv.className = 'log-entry ability group-header clickable-log';
        headerDiv.setAttribute('data-group-id', group.id);
        headerDiv.setAttribute('data-log-type', 'ability_effect');

        const headerContent = LogRenderer.formatLogEntry(headerEntry, group.turnPrefix, currentLang, showFriendlyAbilities);
        const enrichedHeader = Tooltips.enrichAbilityText(headerContent);

        // Add modal viewer button for expanded reading
        const modalButton = `<button class="log-modal-btn" title="${i18n.t('view_expanded')}" data-action="open-log-viewer" data-value="${group.id}">◻</button>`;

        headerDiv.innerHTML = `
            <div class="log-entry-icon"></div>
            <div class="log-entry-content">${enrichedHeader}</div>
            ${detailEntries.length > 0 ? '<div class="log-group-toggle">▼</div>' : ''}
            ${modalButton}
        `;

        headerDiv.onclick = (e) => {
            if (e.target.closest('.log-modal-btn')) return;
            LogRenderer.onLogEntryClick('ability_effect', { body: group.entries.join('\n'), entry: headerEntry, id: group.id });
        };

        LogRenderer.enrichLogEntryWithCard(headerDiv, headerEntry, currentLang, showFriendlyAbilities);

        blockDiv.appendChild(headerDiv);

        // Headers are now non-clickable - use modal button for expanded view
        if (detailEntries.length > 0) {
            const detailsContainer = document.createElement('div');
            detailsContainer.className = 'log-group-details';

            detailEntries.forEach(detail => {
                const detailDiv = document.createElement('div');
                detailDiv.className = 'log-entry effect detail';
                const detailContent = LogRenderer.formatLogEntry(detail, "", currentLang, showFriendlyAbilities);
                const enrichedDetail = Tooltips.enrichAbilityText(detailContent);
                detailDiv.innerHTML = `
                    <div class="log-entry-icon"></div>
                    <div class="log-entry-content">${enrichedDetail}</div>
                `;
                detailsContainer.appendChild(detailDiv);
            });

            // Add full card ability text in details section
            const cardData = LogRenderer.resolveCardFromBody(headerEntry);
            if (cardData) {
                LogRenderer.appendFullAbility(detailsContainer, cardData);
            }

            blockDiv.appendChild(detailsContainer);
        }

        return blockDiv;
    },

    createStandaloneLogEntry: (group, currentLang, showFriendlyAbilities) => {
        const div = document.createElement('div');
        div.className = 'log-entry';

        const bodyContent = LogRenderer.formatLogEntry(group.body, group.turnPrefix, currentLang, showFriendlyAbilities);
        const enrichedBody = Tooltips.enrichAbilityText(bodyContent);

        const entryUpper = group.entry.toUpperCase();
        let entryType = 'generic';

        if ((entryUpper.includes("---") && entryUpper.includes("PHASE")) || entryUpper.includes("[ACTIVE PHASE]")) {
            div.classList.add('phase');
            entryType = 'phase';
        } else if (entryUpper.includes('PLAYS') || entryUpper.includes('MULLIGAN') || entryUpper.includes('SELECTED')) {
            div.classList.add('action');
            entryType = 'action';
        } else if (entryUpper.includes('EFFECT:') || entryUpper.includes('RULE')) {
            div.classList.add('effect');
            entryType = 'effect';
        } else if (entryUpper.includes('SCORE') || entryUpper.includes('PASS') || entryUpper.includes('FAIL')) {
            div.classList.add('score');
            entryType = 'score';
        } else if (entryUpper.includes('PERFORMANCE')) {
            div.classList.add('performance');
            entryType = 'performance';
        } else if (group.entry.includes('===')) {
            div.classList.add('turn');
            entryType = 'turn';
        } else if (entryUpper.includes('ハート') || entryUpper.includes('heart') || entryUpper.includes('ブレード') || entryUpper.includes('blade')) {
            div.classList.add('effect');
            entryType = 'heart_effect';
        } else if (entryUpper.includes('能力') || entryUpper.includes('ability') || entryUpper.includes('スコア') || entryUpper.includes('コスト')) {
            div.classList.add('effect');
            entryType = 'ability_effect';
        } else if (entryUpper.includes('[ACTIVATED]') || entryUpper.includes('[TRIGGERED]')) {
            div.classList.add('activated');
            entryType = 'activated';
        }

        div.setAttribute('data-log-type', entryType);
        div.setAttribute('data-log-body', group.body || '');
        div.classList.add('clickable-log');

        div.innerHTML = `
            <div class="log-entry-icon"></div>
            <div class="log-entry-content">${enrichedBody}</div>
        `;

        div.onclick = () => LogRenderer.onLogEntryClick(entryType, group);

        LogRenderer.enrichLogEntryWithCard(div, group.body, currentLang, showFriendlyAbilities);

        const cardData = LogRenderer.resolveCardFromBody(group.body);
        if (cardData && !group.entry.includes('Mulligan') && !group.entry.includes('PLAYS')) {
            LogRenderer.appendFullAbility(div, cardData);
        }

        // Use structured log entry for richer card lookup
        if (!cardData && group.structEntry && group.structEntry.source_card_id != null) {
            const structuredCard = Tooltips.findCardById(group.structEntry.source_card_id);
            if (structuredCard) {
                LogRenderer.appendFullAbility(div, structuredCard);
            }
        }

        return div;
    },

    onLogEntryClick: (entryType, group) => {
        const body = group.body || group.entry || '';
        const turnMatch = body.match(/Turn (\d+)/i);
        const turn = turnMatch ? parseInt(turnMatch[1]) : -1;

        if (entryType === 'score' || entryType === 'performance' || body.includes('Score:') || body.includes('PASS') || body.includes('FAIL')) {
            document.dispatchEvent(new CustomEvent('opencode:show-performance', {
                detail: { turn, entry: body }
            }));
            return;
        }
        if (entryType === 'effect' || entryType === 'ability_effect' || entryType === 'heart_effect' || entryType === 'generic') {
            document.dispatchEvent(new CustomEvent('opencode:show-log-detail', {
                detail: { entryType, body, groupId: group.id }
            }));
            return;
        }
    },

    createAbilityDebugBlock: (group, currentLang, showFriendlyAbilities) => {
        const blockDiv = document.createElement('div');
        blockDiv.className = 'log-group-block ability-debug-group';

        const cardName = group.cardName || '';
        const headerText = cardName
            ? `${group.turnPrefix} P1 activates ${cardName}'s ability`
            : `${group.turnPrefix} Ability evaluation`;

        const headerDiv = document.createElement('div');
        headerDiv.className = 'log-entry ability group-header ability-debug-header clickable-log';
        headerDiv.setAttribute('data-log-type', 'ability_debug');
        const hasDetails = group.entries.length > 0;
        const fullBody = group.entries.join('\n');
        headerDiv.innerHTML = `
            <div class="log-entry-icon">⚡</div>
            <div class="log-entry-content">${headerText}</div>
            ${hasDetails ? '<div class="log-group-toggle">▼</div>' : ''}
        `;
        headerDiv.onclick = () => LogRenderer.onLogEntryClick('ability_effect', { body: fullBody, entry: headerText, id: group.id });
        blockDiv.appendChild(headerDiv);

        const cardData = State.resolveCardDataByName(cardName);
        if (cardData && cardData.card_no) {
            const imgPath = resolveCardImagePath(cardData.card_no);
            if (imgPath) {
                const img = document.createElement('img');
                img.src = imgPath;
                img.className = 'log-card-thumb ability-debug-thumb';
                img.alt = cardName;
                img.loading = 'lazy';
                headerDiv.insertBefore(img, headerDiv.firstChild);
            }
            Tooltips.attachCardData(headerDiv, cardData);
        }

        if (hasDetails) {
            const detailsContainer = document.createElement('div');
            detailsContainer.className = 'log-group-details ability-debug-details';

            group.entries.forEach(line => {
                const trimmed = line.trim();
                if (!trimmed) return;
                const div = document.createElement('div');
                div.className = 'log-entry effect detail';

                let icon = '•';
                let cls = '';

                if (/^TRIGGER\s/.test(trimmed)) {
                    icon = '⚡';
                    cls = 'ab-trigger';
                } else if (/^COND\s/.test(trimmed)) {
                    icon = '🔍';
                    cls = 'ab-cond';
                } else if (/^COST\s/.test(trimmed)) {
                    icon = '💠';
                    cls = 'ab-cost';
                } else if (/^EFFECT\s/.test(trimmed)) {
                    icon = '➜';
                    cls = 'ab-effect';
                } else {
                    icon = '•';
                }

                const text = trimmed.replace(/^(TRIGGER|COND|COST|EFFECT|ABILITY|TEXT)\s*/, '');
                const displayText = text || trimmed;

                div.innerHTML = `
                    <div class="log-entry-icon ${cls}">${icon}</div>
                    <div class="log-entry-content">${displayText}</div>
                `;
                detailsContainer.appendChild(div);
            });

            blockDiv.appendChild(detailsContainer);
        }

        return blockDiv;
    },

    createSnapshotBlock: (group, currentLang, showFriendlyAbilities) => {
        const blockDiv = document.createElement('div');
        blockDiv.className = 'log-group-block snapshot-block';

        const headerFormatted = LogRenderer.formatLogEntry(group.header, group.turnPrefix, currentLang, showFriendlyAbilities);
        const enriched = Tooltips.enrichAbilityText(headerFormatted);

        const headerDiv = document.createElement('div');
        headerDiv.className = 'log-entry snapshot-header clickable-log';
        headerDiv.setAttribute('data-log-type', 'performance');
        const allText = [group.header, ...(group.entries || [])].join('\n');
        headerDiv.innerHTML = `
            <div class="log-entry-icon">📊</div>
            <div class="log-entry-content">${enriched}</div>
            <div class="log-group-toggle">▼</div>
        `;
        headerDiv.onclick = () => LogRenderer.onLogEntryClick('performance', { body: allText, entry: group.header });
        blockDiv.appendChild(headerDiv);

        const detailsContainer = document.createElement('div');
        detailsContainer.className = 'log-group-details snapshot-details';
        detailsContainer.style.display = 'block';

        (group.entries || []).forEach((line, i) => {
            const formatted = LogRenderer.formatLogEntry(line, '', currentLang, showFriendlyAbilities);
            if (!formatted.trim()) return;
            const div = document.createElement('div');
            div.className = 'log-entry effect detail snapshot-line';

            const isSubhead = line.match(/Yell\s*\(\d+\s*cards?\):/) || line.match(/Hearts breakdown|Total hearts|Base hearts|Yell hearts/);
            if (isSubhead) {
                div.classList.add('snapshot-subhead-line');
            }
            const isHeader = line.match(/Score:/);
            if (isHeader) {
                div.classList.add('snapshot-score-line');
            }

            div.innerHTML = `<div class="log-entry-icon"></div><div class="log-entry-content">${formatted}</div>`;
            detailsContainer.appendChild(div);
        });

        blockDiv.appendChild(detailsContainer);
        return blockDiv;
    },

    formatLogEntry: (body, turnPrefix, currentLang, showFriendlyAbilities) => {
        if (!body) return "";

        // Handle translatable markers [[key:p1=v1:p2=v2]]
        if (body.startsWith("[[") && body.endsWith("]]")) {
            const content = body.slice(2, -2);
            const parts = content.split(":");
            const key = parts[0];
            const params = {};
            for (let i = 1; i < parts.length; i++) {
                const [k, v] = parts[i].split("=");
                if (k && v !== undefined) {
                    // If the value itself is an i18n key (like rps_rock), translate it
                    if (v.startsWith("rps_")) {
                        params[k] = i18n.t(v);
                    } else {
                        params[k] = v;
                    }
                }
            }
            return i18n.t(key, params);
        }

        let displayText = body;
        let playerTag = "";

        if (body.startsWith("P1 ") || body.startsWith("[P1]")) {
            playerTag = `<span class="log-p-badge p1">P1</span>`;
            displayText = displayText.replace(/^\[?P1\]?\s?/, '');
        } else if (body.startsWith("P2 ") || body.startsWith("[P2]")) {
            playerTag = `<span class="log-p-badge p2">P2</span>`;
            displayText = displayText.replace(/^\[?P2\]?\s?/, '');
        }

        const abilityMatch = body.match(/\[TRIGGER:(\d+)\](.*?): (.*)/);
        const rustAbilityMatch = body.match(/(\[Rule .*?\]|\[Activated\]|\[Turn Start\]|\[Turn End\]|\[Triggered\])(.*?): (.*)/);

        if (abilityMatch || rustAbilityMatch) {
            const match = abilityMatch || rustAbilityMatch;
            let triggerLabel = "";
            let cardName = "";
            let pseudocode = "";

            if (abilityMatch) {
                const triggerId = parseInt(match[1]);
                cardName = match[2].trim();
                pseudocode = match[3].trim();
                triggerLabel = `[${triggerId}]`;
                if (translations[currentLang]?.triggers?.[triggerId]) {
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
                if (srcCard && (srcCard.original_text || srcCard.ability)) {
                    // Try to match the trigger label from the log to the correct ability block
                    const block = Tooltips.extractRelevantAbility(srcCard, triggerLabel);
                    translatedEffect = block || srcCard.original_text || srcCard.ability;
                }
            }

            let displayCardName = cardName;
            if (currentLang === 'en' && window.NAME_MAP && window.NAME_MAP[cardName]) {
                displayCardName = window.NAME_MAP[cardName];
            }

            displayText = `${triggerLabel} <strong>${displayCardName}</strong>: ${translatedEffect}`;
        }

        const mulliganMatch = body.match(/(Mulligan): (.*)/i);
        if (mulliganMatch) {
            const cardName = mulliganMatch[2].trim();
            let displayPhase = i18n.t('mulligan');
            let displayCardName = cardName;
            if (currentLang === 'en' && window.NAME_MAP && window.NAME_MAP[cardName]) {
                displayCardName = window.NAME_MAP[cardName];
            }
            displayText = `${displayPhase}: <strong>${displayCardName}</strong>`;
        }

        displayText = displayText.replace(/HEART_RED/g, '[Red]')
            .replace(/HEART_YELLOW/g, '[Yellow]')
            .replace(/HEART_GREEN/g, '[Green]')
            .replace(/HEART_BLUE/g, '[Blue]')
            .replace(/HEART_PURPLE/g, '[Purple]')
            .replace(/HEART_PINK/g, '[Pink]')
            .replace(/HEART_WILD/g, '[Wild]');

        // Enrich snapshot debug output (performance/snapshot format)
        displayText = LogRenderer.enrichSnapshotLine(displayText);

        return (turnPrefix ? `<span class="log-turn-prefix">${turnPrefix}</span> ` : "") + playerTag + displayText;
    },

    enrichSnapshotLine: (text) => {
        if (!text) return text;

        // Hearts breakdown formatting: [h00:N h01:N ...]
        if (text.match(/\[(h\d{2}:\d+(?:\s+h\d{2}:\d+)*)\]/)) {
            text = text.replace(/\[(h\d{2}:\d+(?:\s+h\d{2}:\d+)*)\]/g, (match, inner) => {
                const parts = inner.split(/\s+/);
                const icons = parts.map(p => {
                    const m = p.match(/h(\d{2}):(\d+)/);
                    if (!m) return p;
                    return `<img src="img/texticon/heart_0${parseInt(m[1])}.png" class="heart-mini-icon" title="${p}">${m[2]}`;
                }).join(' ');
                return `[${icons}]`;
            });
        }

        // Performance phase header: "── P1 Performance ──"
        if (text.includes('Performance ──') || text.includes('Performance──')) {
            const pMatch = text.match(/(P[12])/);
            const playerClass = pMatch ? pMatch[1].toLowerCase() : '';
            text = text.replace(/──\s*(P[12])\s*Performance\s*──/,
                `<span class="snapshot-section ${playerClass}">⚡ $1 Performance ⚡</span>`);
        }

        // "Score: N PASS" or "Score: N FAIL"
        if (text.match(/Score:\s*\d+\s*(PASS|FAIL)/)) {
            text = text.replace(/Score:\s*(\d+)\s*(PASS|FAIL)/,
                (m, score, result) => {
                    const cls = result === 'PASS' ? 'snapshot-score-pass' : 'snapshot-score-fail';
                    const icon = result === 'PASS' ? '✓' : '✗';
                    return `<span class="${cls}"><img src="img/texticon/icon_score.png" class="heart-mini-icon"> Score: <b>${score}</b> ${icon} ${result}</span>`;
                });
        }

        // "Live: Name need[...] filled[...] score +N → PASS/FAIL"
        if (text.startsWith('Live:')) {
            text = text.replace(/Live:\s+(.*?)\s+need\[(.*?)\]\s+filled\[(.*?)\]\s+spare\[(.*?)\]\s+score\s+\+?(\d+)\s*→\s*(PASS|FAIL)/,
                (m, name, needStr, filledStr, spareStr, score, result) => {
                    const cls = result === 'PASS' ? 'snapshot-live-pass' : 'snapshot-live-fail';
                    const icon = result === 'PASS' ? '✓' : '✗';
                    const need = needStr.replace(/(h\d{2}):(\d+)/g,
                        (_, h, n) => `<img src="img/texticon/heart_0${parseInt(h.substring(1))}.png" class="heart-mini-icon">${n}`);
                    const filled = filledStr.replace(/(h\d{2}):(\d+)/g,
                        (_, h, n) => `<img src="img/texticon/heart_0${parseInt(h.substring(1))}.png" class="heart-mini-icon">${n}`);
                    const spare = spareStr.replace(/(h\d{2}):(\d+)/g,
                        (_, h, n) => `<img src="img/texticon/heart_0${parseInt(h.substring(1))}.png" class="heart-mini-icon">${n}`);
                    return `<span class="${cls}"><img src="img/texticon/icon_score.png" class="heart-mini-icon"> ${name} need[${need}] filled[${filled}] score+${score} ${icon}${result}</span>`;
                });
        }

        // "Stage: Name ★N ♥[h00:N ...]"
        if (text.match(/Stage:\s/)) {
            text = text.replace(/Stage:\s+(.*?)\s+(★\d+(?:\s*\(.*?\))?)\s*♥\[(.*?)\]/g,
                (m, name, starStr, heartStr) => {
                    const h = heartStr.replace(/(h\d{2}):(\d+)/g,
                        (_, hh, n) => `<img src="img/texticon/heart_0${parseInt(hh.substring(1))}.png" class="heart-mini-icon">${n}`);
                    return `<span class="snapshot-stage"><img src="img/texticon/icon_blade.png" class="heart-mini-icon"> ${name} ${starStr} ♥[${h}]</span>`;
                });
            // Also handle Stage without hearts
            text = text.replace(/Stage:\s+(.*?)\s+(★\d+(?:\s*\(.*?\))?)\s*$/g,
                (m, name, starStr) => {
                    return `<span class="snapshot-stage"><img src="img/texticon/icon_blade.png" class="heart-mini-icon"> ${name} ${starStr}</span>`;
                });
        }

        // "Yell (N cards):" heading
        if (text.match(/Yell\s*\(\d+\s*cards?\):/)) {
            text = text.replace(/(Yell\s*\(\d+\s*cards?\):)/, '<span class="snapshot-subhead">📋 $1</span>');
        }

        // Indented yell card line: "Name ♥[h00:N] ♪N ⎋N"
        if (text.match(/^\s{4}\S.*♥\[/) || text.match(/^\s{4}\S.*♪\d/)) {
            text = text.replace(/^\s{4}(.*?)\s*♥\[(.*?)\]\s*♪(\d+)\s*⎋(\d+)/,
                (m, name, heartStr, noteIcons, drawIcons) => {
                    const h = heartStr.replace(/(h\d{2}):(\d+)/g,
                        (_, hh, n) => `<img src="img/texticon/heart_0${parseInt(hh.substring(1))}.png" class="heart-mini-icon">${n}`);
                    return `<span class="snapshot-yell-card"><img src="img/texticon/icon_blade.png" class="heart-mini-icon"> ${name} ♥[${h}] ♪${noteIcons} ⎋${drawIcons}</span>`;
                });
            // Without heart
            text = text.replace(/^\s{4}(.*?)\s*♪(\d+)\s*⎋(\d+)/,
                (m, name, noteIcons, drawIcons) => {
                    return `<span class="snapshot-yell-card"><img src="img/texticon/icon_blade.png" class="heart-mini-icon"> ${name} ♪${noteIcons} ⎋${drawIcons}</span>`;
                });
        }

        // "Hearts breakdown:", "Total hearts:", "Base hearts:", "Yell hearts:"
        if (text.match(/^\s*(Hearts breakdown|Total hearts|Base hearts|Yell hearts):/)) {
            text = text.replace(/^\s*(Hearts breakdown|Total hearts|Base hearts|Yell hearts):/,
                '<span class="snapshot-label">$1:</span>');
        }

        // Ability bonus lines
        if (text.match(/^\s{4}Ability:/)) {
            text = text.replace(/^\s{4}Ability:\s+(.*?)\s+♥(\w+)\+(\d+)/,
                (m, source, colorStr, amount) => {
                    const hIdx = parseInt(colorStr.replace('heart', ''));
                    const icon = hIdx >= 0 && hIdx < 7
                        ? `<img src="img/texticon/heart_0${hIdx}.png" class="heart-mini-icon">`
                        : '♥';
                    return `<span class="snapshot-ability-bonus">↳ ${source} ${icon}+${amount}</span>`;
                });
            text = text.replace(/^\s{4}Ability:\s+(.*?)\s*★\+(\d+)/,
                (m, source, amount) => {
                    return `<span class="snapshot-ability-bonus">↳ ${source} <img src="img/texticon/icon_blade.png" class="heart-mini-icon">+${amount}</span>`;
                });
        }

        return text;
    },

    resolveCardFromBody: (body) => {
        if (!body) return null;
        const abilityMatch = body.match(/\[TRIGGER:\d+\]\s*(.*?):\s/);
        const rustAbilityMatch = body.match(/(\[Rule .*?\]|\[Activated\]|\[Turn Start\]|\[Turn End\]|\[Triggered\])\s*(.*?):\s/);
        const match = abilityMatch || rustAbilityMatch;
        let cardName = null;
        if (abilityMatch) {
            cardName = abilityMatch[1].trim();
        } else if (rustAbilityMatch) {
            cardName = rustAbilityMatch[2].trim();
        }
        if (!cardName) {
            const revealMatch = body.match(/reveals\s+(.+?)\s+from\s/i);
            if (revealMatch) {
                const names = revealMatch[1].split(',').map(n => n.trim()).filter(n => n);
                cardName = names[0] || null;
            }
        }
        if (!cardName) return null;
        const cardData = State.resolveCardDataByName(cardName) || State.resolveCardDataByName(cardName.replace(/^["']|["']$/g, ''));
        if (!cardData || !cardData.card_no) return null;
        return cardData;
    },

    getAllCardNamesFromBody: (body) => {
        if (!body) return [];
        const abilityMatch = body.match(/\[TRIGGER:\d+\]\s*(.*?):\s/);
        const rustAbilityMatch = body.match(/(\[Rule .*?\]|\[Activated\]|\[Turn Start\]|\[Turn End\]|\[Triggered\])\s*(.*?):\s/);
        if (abilityMatch || rustAbilityMatch) {
            const match = abilityMatch || rustAbilityMatch;
            const name = (abilityMatch ? match[1] : match[2]).trim();
            return name ? [name] : [];
        }
        const revealMatch = body.match(/reveals\s+(.+?)\s+from\s/i);
        if (revealMatch) {
            return revealMatch[1].split(',').map(n => n.trim()).filter(n => n);
        }
        return [];
    },

    enrichLogEntryWithCard: (entryEl, body, currentLang, showFriendlyAbilities) => {
        if (!entryEl || !body) return;
        const cardNames = LogRenderer.getAllCardNamesFromBody(body);
        if (cardNames.length === 0) return;

        const fragment = document.createDocumentFragment();
        cardNames.forEach(name => {
            const cardData = State.resolveCardDataByName(name) || State.resolveCardDataByName(name.replace(/^["']|["']$/g, ''));
            if (cardData && cardData.card_no) {
                const imgPath = resolveCardImagePath(cardData.card_no);
                if (imgPath) {
                    const img = document.createElement('img');
                    img.src = imgPath;
                    img.className = 'log-card-thumb log-revealed-thumb';
                    img.alt = cardData.name || name;
                    img.loading = 'lazy';
                    img.onclick = (e) => {
                        e.stopPropagation();
                        LogRenderer.showRevealedCardsModal();
                    };
                    fragment.appendChild(img);
                }
                if (!entryEl.dataset.cardId && cardData.id !== undefined) {
                    Tooltips.attachCardData(entryEl, cardData);
                }
            }
        });
        if (fragment.childNodes.length > 0) {
            entryEl.insertBefore(fragment, entryEl.firstChild);
        }
    },

    showRevealedCardsModal: () => {
        const s = State.data;
        if (!s) return;
        const title = document.getElementById(DOM_IDS.REVEALED_TITLE);
        const content = document.getElementById(DOM_IDS.REVEALED_CONTENT);
        if (!title || !content) return;

        const allRevealed = s.revealed_cards || [];
        const p1Cheer = s.player1_cheer_revealed_cards || [];
        const p2Cheer = s.player2_cheer_revealed_cards || [];
        const costRevealed = s.revealed_cost_cards || [];

        // Live zone cards (公開領域)
        const p1Live = s.player1?.live_zone?.cards || [];
        const p2Live = s.player2?.live_zone?.cards || [];

        const cardToHtml = (idOrCard) => {
            const card = typeof idOrCard === 'number' ? State.resolveCardData(idOrCard) : idOrCard;
            if (!card) return `<div class="revealed-card chip">ID:${typeof idOrCard === 'number' ? idOrCard : '?'}</div>`;
            const imgPath = resolveCardImagePath(card.card_no);
            const img = imgPath ? `<img src="${fixImg(imgPath)}" class="revealed-card-img" alt="${card.name}">` : '';
            return `<div class="revealed-card">${img}<span class="revealed-card-name">${card.name}</span></div>`;
        };

        const section = (label, ids) => ids.length > 0
            ? `<div class="revealed-section"><h4>${label} (${ids.length})</h4><div class="revealed-grid">${ids.map(cardToHtml).join('')}</div></div>`
            : '';

        const p1Label = State.perspectivePlayer === 0 ? i18n.t('you') : i18n.t('opponent');
        const p2Label = State.perspectivePlayer === 1 ? i18n.t('you') : i18n.t('opponent');

        title.textContent = 'Revealed Cards';
        content.innerHTML =
            section('All Revealed Cards', allRevealed) +
            section(p1Label + ' (yell)', p1Cheer) +
            section(p2Label + ' (yell)', p2Cheer) +
            section('Cost Revealed', costRevealed) +
            section(p1Label + ' (live set)', p1Live) +
            section(p2Label + ' (live set)', p2Live);

        if (!allRevealed.length && !p1Cheer.length && !p2Cheer.length && !costRevealed.length && !p1Live.length && !p2Live.length) {
            content.innerHTML = '<div style="opacity:0.5;padding:40px;text-align:center;">No cards have been revealed this game.</div>';
        }

        ModalManager.show(DOM_IDS.MODAL_REVEALED);
    },

    appendFullAbility: (containerEl, cardData) => {
        if (!containerEl || !cardData) return;
        const rawAbility = Tooltips.getEffectiveRawText(cardData);
        if (!rawAbility || rawAbility.length < 5) return;

        const abilityDiv = document.createElement('div');
        abilityDiv.className = 'log-full-ability';
        const enriched = Tooltips.enrichAbilityText(rawAbility);
        abilityDiv.innerHTML = enriched;
        containerEl.appendChild(abilityDiv);
    },

    renderActiveAbilities: (containerId, abilities) => {
        const el = document.getElementById(containerId);
        if (!el || !abilities) return;
        el.innerHTML = abilities.map(a => {
            const cardIdAttr = a.source_card_id !== undefined ? `data-card-id="${a.source_card_id}"` : '';
            const dataTextAttr = a.text || a.description ? `data-text="${a.text || a.description}"` : '';
            return `
                <div class="active-ability-tag" ${cardIdAttr} ${dataTextAttr}>
                    ${Tooltips.enrichAbilityText(a.name || 'Ability')}
                </div>
            `;
        }).join('');
    },

    updateLogDifferential: (containerId = 'rule-log') => {
        const ruleLogEl = document.getElementById(containerId);
        if (!ruleLogEl) return;

        const state = State.data;
        const currentLang = State.currentLang;
        const showFriendlyAbilities = State.showFriendlyAbilities;

        const currentLogCount = (state.rule_log || []).length;
        const currentHistoryCount = (state.turn_history || []).length;

        if (currentLogCount < PerformanceMonitor._lastLogCount || currentHistoryCount < PerformanceMonitor._lastHistoryCount) {
            PerformanceMonitor._lastLogCount = currentLogCount;
            PerformanceMonitor._lastHistoryCount = currentHistoryCount;
            LogRenderer.renderRuleLog(containerId);
            return;
        }

        const newLogEntries = state.rule_log.slice(PerformanceMonitor._lastLogCount);
        const newHistoryEntries = (state.turn_history || []).slice(PerformanceMonitor._lastHistoryCount);

        if (newLogEntries.length === 0 && newHistoryEntries.length === 0) return;

        if (newHistoryEntries.length > 0) {
            const turnHistorySection = ruleLogEl.querySelector('.turn-history-section');
            if (turnHistorySection) {
                [...newHistoryEntries].reverse().forEach(event => {
                    const filteredEvent = LogFilter.applyFilters([event])[0];
                    if (filteredEvent) {
                        const entry = LogRenderer.createTurnEventElement(event);
                        turnHistorySection.insertBefore(entry, turnHistorySection.firstChild || null);
                    }
                });
            }
        }

        if (newLogEntries.length > 0) {
            const ruleLogSection = ruleLogEl.querySelector('.rule-log-section');
            if (ruleLogSection) {
                [...newLogEntries].reverse().forEach(entry => {
                    const div = LogRenderer.createStandaloneLogEntry(
                        { entry, body: entry.replace(/^\[Turn \d+\]\s*/, ''), turnPrefix: '' },
                        currentLang,
                        showFriendlyAbilities
                    );
                    ruleLogSection.insertBefore(div, ruleLogSection.firstChild || null);
                });
            }
        }

        PerformanceMonitor._lastLogCount = currentLogCount;
        PerformanceMonitor._lastHistoryCount = currentHistoryCount;

        if (!State.showingFullLog) ruleLogEl.scrollTop = 0;
    }
};
