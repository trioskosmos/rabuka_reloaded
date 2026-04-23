import { State } from '../state.js';
import * as i18n from '../i18n/index.js';
import { Tooltips } from '../ui_tooltips.js';
import { DOMUtils } from '../utils/DOMUtils.js';
import { DOM_IDS } from '../constants_dom.js';
import { ModalManager } from '../utils/ModalManager.js';

export const LogViewerModal = {
    init: () => {
        // Initialize modal if needed
        const modal = DOMUtils.getElement(DOM_IDS.MODAL_LOG_VIEWER);
        if (modal) {
            ModalManager.setupBackdropClose(DOM_IDS.MODAL_LOG_VIEWER, LogViewerModal.close);
        }
        // Load metadata enums for enhanced annotations
        LogViewerModal._metadata = null;
        LogViewerModal._revMaps = null;
        LogViewerModal.loadMetadata();
    },

    loadMetadata: async () => {
        try {
            const res = await fetch('/data/metadata.json');
            if (!res.ok) return;
            const meta = await res.json();
            LogViewerModal._metadata = meta;
            // Build reverse maps: category -> { value: key }
            const rev = {};
            Object.keys(meta).forEach(cat => {
                const obj = meta[cat];
                if (obj && typeof obj === 'object') {
                    rev[cat] = {};
                    Object.keys(obj).forEach(k => {
                        const v = obj[k];
                        // store stringified value for easy lookup
                        rev[cat][String(v)] = k;
                    });
                }
            });
            LogViewerModal._revMaps = rev;
        } catch (e) {
            // ignore silently
            LogViewerModal._metadata = null;
            LogViewerModal._revMaps = null;
        }
    },

    open: (focusedGroupId = null) => {
        const modalShown = ModalManager.show(DOM_IDS.MODAL_LOG_VIEWER);
        if (!modalShown) return;

        const contentContainer = DOMUtils.getElement(DOM_IDS.LOG_VIEWER_CONTENT);
        if (!contentContainer) return;

        LogViewerModal.renderLogs(contentContainer, focusedGroupId);

        // Focus on specific group if provided
        if (focusedGroupId) {
            setTimeout(() => {
                const groupEl = document.querySelector(`[data-group-id="${focusedGroupId}"]`);
                if (groupEl) {
                    groupEl.scrollIntoView({ behavior: 'smooth', block: 'start' });
                    groupEl.classList.add('highlighted');
                    setTimeout(() => groupEl.classList.remove('highlighted'), 2000);
                }
            }, 100);
        }
    },

    close: () => {
        ModalManager.hide(DOM_IDS.MODAL_LOG_VIEWER);
    },

    renderLogs: (container, focusedGroupId) => {
        const state = State.data;
        if (!state || !state.rule_log) {
            container.innerHTML = `<div class="log-viewer-empty">${i18n.t('no_logs_available') || 'No logs available'}</div>`;
            return;
        }

        const currentLang = State.currentLang;
        const showFriendlyAbilities = State.showFriendlyAbilities;
        const legacyLog = state.rule_log || [];
        const structured = state.turn_history || [];

        DOMUtils.clear(container.id || DOM_IDS.LOG_VIEWER_CONTENT);
        const fragment = document.createDocumentFragment();

        // Add search and filter bar
        const filterBar = LogViewerModal.createFilterBar();
        fragment.appendChild(filterBar);

        // Group logs
        let groupedLogs = [];

        if (structured && structured.length > 0) {
            // Prefer structured turn_history when available (no execution grouping yet)
            structured.forEach(ev => {
                const turnNumber = ev.turn || 0;
                const text = ev.description || '';
                groupedLogs.push({ entry: text, body: text, turnNumber, isStandalone: true, ev });
            });
        } else {
            let currentGroup = null;
            legacyLog.forEach((entry) => {
                const idMatch = entry.match(/\[Turn \d+\] \[ID: (\d+)\] (.*)/);
                const executionId = idMatch ? idMatch[1] : null;
                const body = idMatch ? idMatch[2] : entry.replace(/^\[Turn \d+\]\s*/, '');
                const turnMatch = entry.match(/^\[Turn (\d+)\]/);
                const turnNumber = turnMatch ? parseInt(turnMatch[1]) : 0;

                if (executionId) {
                    if (!currentGroup || currentGroup.id !== executionId) {
                        currentGroup = {
                            id: executionId,
                            entries: [],
                            turnNumber,
                            fullEntry: entry
                        };
                        groupedLogs.push(currentGroup);
                    }
                    currentGroup.entries.push(body);
                } else {
                    currentGroup = null;
                    groupedLogs.push({
                        entry,
                        body,
                        turnNumber,
                        isStandalone: true
                    });
                }
            });
        }

        // Render groups with enhanced formatting
        const logContent = document.createElement('div');
        logContent.className = 'log-viewer-entries';

        groupedLogs.forEach((group, idx) => {
            if (group.entries) {
                const blockEl = LogViewerModal.createExpandedLogBlock(
                    group,
                    currentLang,
                    showFriendlyAbilities,
                    focusedGroupId
                );
                logContent.appendChild(blockEl);
            } else {
                const entryEl = LogViewerModal.createStandaloneEntry(
                    group,
                    currentLang,
                    showFriendlyAbilities
                );
                logContent.appendChild(entryEl);
            }
        });

        fragment.appendChild(logContent);
        container.appendChild(fragment);
    },

    createFilterBar: () => {
        const bar = document.createElement('div');
        bar.className = 'log-viewer-filter-bar';

        bar.innerHTML = `
            <input type="text"
                   class="form-input log-viewer-search"
                   placeholder="${i18n.t('search') || 'Search logs...' }"
                   onkeyup="LogViewerModal.filterLogs(this.value)">
            <div class="log-viewer-filter-toggles">
                <label class="filter-checkbox-label">
                    <input type="checkbox" id="${DOM_IDS.FILTER_TRIGGER}" checked onchange="LogViewerModal.filterLogs()">
                    <span>${i18n.t('triggers')}</span>
                </label>
                <label class="filter-checkbox-label">
                    <input type="checkbox" id="${DOM_IDS.FILTER_EFFECT}" checked onchange="LogViewerModal.filterLogs()">
                    <span>${i18n.t('effects')}</span>
                </label>
            </div>
        `;
        return bar;
    },

    filterLogs: (searchText = '') => {
        const entries = document.querySelectorAll('.log-viewer-entries > div');
        const filterTrigger = DOMUtils.getElement(DOM_IDS.FILTER_TRIGGER)?.checked ?? true;
        const filterEffect = DOMUtils.getElement(DOM_IDS.FILTER_EFFECT)?.checked ?? true;
        const searchLower = searchText.toLowerCase();

        entries.forEach(entry => {
            const isTrigger = entry.classList.contains('log-entry-trigger');
            const isEffect = entry.classList.contains('log-entry-effect');
            const text = entry.textContent.toLowerCase();

            let show = true;

            // Filter by type
            if (isTrigger && !filterTrigger) show = false;
            if (isEffect && !filterEffect) show = false;

            // Filter by search text
            if (searchText && !text.includes(searchLower)) show = false;

            entry.style.display = show ? 'block' : 'none';
        });
    },

    createExpandedLogBlock: (group, currentLang, showFriendlyAbilities, focusedGroupId) => {
        const isExpanded = focusedGroupId && parseInt(focusedGroupId) === parseInt(group.id);

        const block = document.createElement('div');
        block.className = `log-viewer-block log-entry-trigger ${isExpanded ? 'expanded' : ''}`;
        block.setAttribute('data-group-id', group.id);

        let headerEntry = group.entries[0];
        let detailEntries = group.entries.slice(1);

        // Parse and enhance header
        const enhancedHeader = LogViewerModal.enhanceAbilityDescription(
            headerEntry,
            currentLang,
            showFriendlyAbilities,
            group.fullEntry
        );

        const headerDiv = document.createElement('div');
        headerDiv.className = 'log-viewer-header';
        headerDiv.innerHTML = `
            <div class="log-viewer-turn-badge">T${group.turnNumber}</div>
            <div class="log-viewer-header-content">
                <div class="log-viewer-condition">${enhancedHeader.condition}</div>
                <div class="log-viewer-result">${enhancedHeader.result}</div>
            </div>
            <div class="log-viewer-expand-icon">▸</div>
        `;

        block.appendChild(headerDiv);

        if (detailEntries.length > 0) {
            const detailsDiv = document.createElement('div');
            detailsDiv.className = `log-viewer-details ${isExpanded ? '' : 'collapsed'}`;

            detailEntries.forEach((detail, idx) => {
                const enhancedDetail = LogViewerModal.enhanceAbilityDescription(
                    detail,
                    currentLang,
                    showFriendlyAbilities
                );

                const detailEl = document.createElement('div');
                detailEl.className = 'log-viewer-detail-item';
                detailEl.innerHTML = `
                    <div class="log-viewer-detail-index">${idx + 1}</div>
                    <div class="log-viewer-detail-content">
                        <div class="log-viewer-detail-condition">${enhancedDetail.condition}</div>
                        <div class="log-viewer-detail-result">${enhancedDetail.result}</div>
                    </div>
                `;
                detailsDiv.appendChild(detailEl);
            });

            block.appendChild(detailsDiv);

            headerDiv.onclick = () => {
                const isCollapsed = detailsDiv.classList.toggle('collapsed');
                block.classList.toggle('expanded', !isCollapsed);
            };
            headerDiv.style.cursor = 'pointer';
        }

        return block;
    },

    createStandaloneEntry: (group, currentLang, showFriendlyAbilities) => {
        const div = document.createElement('div');
        div.className = 'log-viewer-standalone log-entry-effect';

        const enhanced = LogViewerModal.enhanceAbilityDescription(
            group.body,
            currentLang,
            showFriendlyAbilities
        );

        div.innerHTML = `
            <div class="log-viewer-turn-badge">T${group.turnNumber}</div>
            <div class="log-viewer-content">
                ${enhanced.condition ? `<div class="log-viewer-condition">${enhanced.condition}</div>` : ''}
                ${enhanced.result ? `<div class="log-viewer-result">${enhanced.result}</div>` : ''}
            </div>
        `;

        return div;
    },

    enhanceAbilityDescription: (text, currentLang, showFriendlyAbilities, fullEntryRaw = null) => {
        // Parse and provide human-readable condition + result format

        // Look for ability trigger patterns
        const triggerMatch = text.match(/\[Trigger:(\d+)\](.*?): (.*)/i) ||
                            text.match(/\[Rule (.*?)\](.*?): (.*)/i) ||
                            text.match(/\[(Activated|Triggered|Turn Start|Turn End)\](.*?): (.*)/i);

        if (triggerMatch) {
            const triggerInfo = triggerMatch[1];
            const cardName = triggerMatch[2]?.trim() || '';
            const pseudocode = triggerMatch[3]?.trim() || '';

            let condition = '';
            let result = '';

            // Prefer extracting the exact ability text from card data when possible
            let extracted = null;
            if (cardName && LogViewerModal._metadata) {
                const srcCard = State.resolveCardDataByName(cardName);
                if (srcCard) {
                    const triggerLabel = triggerInfo ? String(triggerInfo) : null;
                    try {
                        extracted = Tooltips.extractRelevantAbility(srcCard, triggerLabel);
                    } catch (e) {
                        extracted = null;
                    }
                }
            }

            if (extracted) {
                // We have the exact ability block from the card; translate if requested
                if ((currentLang === 'en' || showFriendlyAbilities) && window.translateAbility) {
                    const t = window.translateAbility("EFFECT: " + extracted, currentLang);
                    result = t.replace(/^.*?: /, '').replace(/^→ /, '');
                } else {
                    result = extracted;
                }
            } else {
                // Translate pseudocode to readable format
                if (showFriendlyAbilities && window.translateAbility) {
                    const fullText = window.translateAbility("EFFECT: " + pseudocode, currentLang);
                    result = fullText.replace(/^.*?: /, '').replace(/^→ /, '');
                } else {
                    result = LogViewerModal.convertPseudocodeToReadable(pseudocode);
                }
            }

            // Build condition string
            if (!isNaN(triggerInfo)) {
                const trigName = LogViewerModal._revMaps && LogViewerModal._revMaps['triggers'] ? (LogViewerModal._revMaps['triggers'][String(triggerInfo)] || null) : null;
                if (trigName) {
                    condition = `<strong>${trigName}</strong> (${triggerInfo}): ${cardName || 'Card'}`;
                } else {
                    condition = `<strong>Trigger ${triggerInfo}:</strong> ${cardName || 'Card'}`;
                }
            } else if (triggerInfo.includes('Rule')) {
                condition = `<strong>Rule ${triggerInfo}:</strong> ${cardName || ''}`;
            } else {
                condition = `<strong>${triggerInfo}:</strong> ${cardName || ''}`;
            }

            return { condition, result };
        }

        // Parse action patterns
        if (text.includes('PLAYS') || text.includes('plays')) {
            const cardMatch = text.match(/plays (.*?)(?:\s|$|,)/i);
            const card = cardMatch ? cardMatch[1] : 'a card';
            return {
                condition: '<strong>Action:</strong> Card played',
                result: `Player plays ${card}`
            };
        }

        if (text.includes('Mulligan')) {
            const cardMatch = text.match(/Mulligan: (.*)/i);
            const card = cardMatch ? cardMatch[1] : 'card';
            return {
                condition: '<strong>Mulligan</strong>',
                result: `Selected ${card}`
            };
        }

        if (text.includes('EFFECT:') || text.includes('Effect:')) {
            const effectMatch = text.match(/Effect: (.*)/i);
            const effect = effectMatch ? effectMatch[1] : text;
            return {
                condition: '<strong>Effect:</strong>',
                result: LogViewerModal.convertPseudocodeToReadable(effect)
            };
        }

        // Fallback
        return {
            condition: '',
            result: text
        };
    },

    convertPseudocodeToReadable: (pseudocode) => {
        // Convert known pseudocode patterns to readable English
        let readable = pseudocode;

        const conversions = [
            { pattern: /draw\s*\((\d+)\)/gi, replacement: 'Draw $1 card(s)' },
            { pattern: /search\s*\((\d+)\)/gi, replacement: 'Search for $1 card(s)' },
            { pattern: /discard\s*\((\d+)\)/gi, replacement: 'Discard $1 card(s)' },
            { pattern: /reduce_cost\s*\((\d+)\)/gi, replacement: 'Reduce cost by $1' },
            { pattern: /buff_blade\s*\((\d+),\s*(\d+)\)/gi, replacement: 'Boost slot $1 appeal by $2' },
            { pattern: /buff_heart\s*\((\d+),\s*(\d+),\s*(\d+)\)/gi, replacement: 'Add $3 hearts to slot $1 ($2)' },
            { pattern: /prevent_baton/gi, replacement: 'Cannot use baton touch' },
            { pattern: /prevent_activate/gi, replacement: 'Cannot activate members' },
            { pattern: /tap_/gi, replacement: 'Tap ' },
            { pattern: /untap_/gi, replacement: 'Untap ' },
            { pattern: /select_member/gi, replacement: 'Select a member' },
            { pattern: /move_card/gi, replacement: 'Move card' },
            { pattern: /shuffle_deck/gi, replacement: 'Shuffle deck' },
            { pattern: /(?:if|condition)_/gi, replacement: 'If ' },
            { pattern: /(?:then|effect)_/gi, replacement: 'Then ' },
            { pattern: /MET/gi, replacement: '✓ Met' },
            { pattern: /NOT_MET/gi, replacement: '✗ Not met' },
            { pattern: /FAIL/gi, replacement: '✗ Failed' },
            { pattern: /SUCCESS/gi, replacement: '✓ Success' }
        ];

        conversions.forEach(({ pattern, replacement }) => {
            readable = readable.replace(pattern, replacement);
        });

        // Add spaces after commas and semicolons if missing
        readable = readable.replace(/,([^ ])/g, ', $1');
        readable = readable.replace(/;([^ ])/g, '; $1');

        // Replace opcode mentions like "OP 123" with enum name when available
        if (LogViewerModal._revMaps && LogViewerModal._revMaps['opcodes']) {
            const opmap = LogViewerModal._revMaps['opcodes'];
            readable = readable.replace(/\bOP\s*[:=]?\s*(\d+)\b/gi, (m, num) => {
                const name = opmap[String(num)];
                return name ? `${name}(${num})` : m;
            });
            // also catch standalone opcode tokens like "OPCODE 123"
            readable = readable.replace(/\bOPCODE\s*[:=]?\s*(\d+)\b/gi, (m, num) => {
                const name = opmap[String(num)];
                return name ? `${name}(${num})` : m;
            });
        }

        // Annotate enums if metadata is available
        return LogViewerModal.annotateEnums(readable);
    }
};

// Export for global use
window.LogViewerModal = LogViewerModal;

// --- Annotation helpers (added after main export to keep functions accessible) ---
LogViewerModal.annotateEnums = (text) => {
    if (!LogViewerModal._revMaps) return text;
    let out = text;

    // 1) Annotate numeric enum values like "(213)" or standalone numbers
    out = out.replace(/\b(\d{2,4})\b/g, (m, num) => {
        // Try to find this number in any rev map
        for (const cat of Object.keys(LogViewerModal._revMaps)) {
            const name = LogViewerModal._revMaps[cat][String(num)];
            if (name) return `${name} (${cat}:${num})`;
        }
        return m;
    });

    // 2) Annotate uppercase tokens that match enum keys (e.g., TURN_1, DRAW, REDUCE_COST)
    out = out.replace(/\b([A-Z0-9_]{2,60})\b/g, (m) => {
        // skip short common words
        if (m.length <= 1) return m;
        for (const cat of Object.keys(LogViewerModal._metadata || {})) {
            const obj = LogViewerModal._metadata[cat];
            if (obj && Object.prototype.hasOwnProperty.call(obj, m)) {
                return `${m} (${cat})`;
            }
        }
        return m;
    });

    return out;
};
