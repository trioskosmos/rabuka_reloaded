/**
 * UI Tooltips & Highlighting Module
 * Handles contextual tooltips, ability text enrichment, and visual highlights for actions.
 */
import { State } from './state.js';
import { Rendering } from './ui_rendering.js';
import { ICON_DATA_URIs } from './assets_registry.js';

let tooltipTimeout = null;
let tooltipHideTimeout = null;
let currentTooltipTarget = null;
let currentTooltipSidebarTarget = null;

export const Tooltips = {
    /**
     * Replaces bracketed icons [Auto] and {{icon|alt}} with actual <img> tags.
     */
    enrichAbilityText: (text) => {
        if (!text) return "";

        // Keep a list of masked segments to restore later
        const placeholders = [];

        // 1. First, mask all existing HTML tags to prevent text-based highlighting from matching inside them
        text = text.replace(/<[^>]+>/g, (match) => {
            const id = `__TAG_${placeholders.length}__`;
            placeholders.push(match);
            return id;
        });

        // 2. Hide curly-brace tags {{...}} to prevent highlighting matching into alt text
        text = text.replace(/{{(.*?)\|(.*?)}}/g, (match) => {
            const id = `__PH_${placeholders.length}__`;
            placeholders.push(match);
            return id;
        });

        // 3. Apply text-based highlighting only on the unmasked text

        // Colors (SIC Attributes)
        text = text.replace(/(ピンク|レッド|赤|イエロー|黄|グリーン|緑|ブルー|青|パープル|紫|オール|Pink|Red|Yellow|Green|Blue|Purple|All)/g, match => {
            const m = match.toLowerCase();
            let cls = 'hl-keyword';
            if (m.includes('pink') || m.includes('red') || m === 'ピンク' || m === 'レッド' || m === '赤') cls = 'hl-heart';
            if (m.includes('yellow') || m.includes('green') || m === 'イエロー' || m === '黄' || m === 'グリーン' || m === '緑') cls = 'hl-energy';
            if (m.includes('blue') || m.includes('purple') || m === 'ブルー' || m === '青' || m === 'パープル' || m === '紫') cls = 'hl-blade';
            return `<span class="${cls}">${match}</span>`;
        });

        // Zones (Discard, Stage, Live Zone)
        const zoneList = ['控え室', 'メンバー置場', 'ライブ置場', 'エナジー置場', '待機室', '手札', 'デッキ', '山札', 'Discard', 'Stage', 'Live Zone', 'Hand', 'Deck', 'Performance'];
        const zoneRegex = new RegExp(`(${zoneList.join('|')})`, 'g');
        text = text.replace(zoneRegex, '<span class="hl-zone">$1</span>');

        // Mechanics (Hearts, Blades, Energy, Yell)
        const mechRegex = /(ハート|ピース|エナジー|ボルテージ|勧誘|Hearts|Blades|Energy|Voltage|Scout|Yell|RUSH|LIVE|LIVE!|Appeal|HEART|DREAM)/g;
        text = text.replace(mechRegex, '<span class="hl-keyword">$1</span>');

        // Success/Failure
        text = text.replace(/(SUCCESS|成功)/g, '<span class="hl-success">$1</span>');
        text = text.replace(/(FAILURE|失敗)/g, '<span class="hl-failure">$1</span>');

        // Card Numbers (e.g., PL!HS-PR-010-PR)
        text = text.replace(/(PL![\w-]+)/g, '<span class="hl-card">$1</span>');

        // 4. Restore ALL placeholders in reverse order to handle nesting if any occurred (though unlikely here)
        for (let i = placeholders.length - 1; i >= 0; i--) {
            text = text.replace(`__TAG_${i}__`, placeholders[i]);
            text = text.replace(`__PH_${i}__`, placeholders[i]);
        }

        // 5. Now insert icons and other tags
        const iconMap = {
            '登場時': 'texticon/toujyou.png',
            '自動': 'texticon/jidou.png',
            '永続': 'texticon/jyouji.png',
            '起動': 'texticon/kidou.png',
            'ターン1': 'texticon/turn1.png',
            'ライブスタート': 'texticon/live_start.png',
            'LIVE START': 'texticon/live_start.png',
            'ピンク': 'color_pink.png', 'Pink': 'color_pink.png',
            'レッド': 'color_red.png', '赤': 'color_red.png', 'Red': 'color_red.png',
            'イエロー': 'color_yellow.png', '黄': 'color_yellow.png', 'Yellow': 'color_yellow.png',
            'グリーン': 'color_green.png', '緑': 'color_green.png', 'Green': 'color_green.png',
            'ブルー': 'color_blue.png', '青': 'color_blue.png', 'Blue': 'color_blue.png',
            'パープル': 'color_purple.png', '紫': 'color_purple.png', 'Purple': 'color_purple.png',
            'オール': 'color_all.png', 'All': 'color_all.png',
            'ライブ開始時': 'icon_live_start.png', 'LIVE START': 'icon_live_start.png',
            '成功時': 'icon_live_success.png',
            'ライブ成功時': 'icon_live_success.png', 'LIVE SUCCESS': 'icon_live_success.png',
            'エネ': 'icon_energy.png', 'Energy': 'icon_energy.png', 'エネルギー': 'icon_energy.png',
            'ハート': 'heart_01.png', 'Hearts': 'heart_01.png', 'ピース': 'heart_01.png',
            'ピンクハート': 'heart_00.png', 'Pink Hearts': 'heart_00.png',
            'レッドハート': 'heart_01.png', 'Red Hearts': 'heart_01.png',
            'イエローハート': 'heart_02.png', 'Yellow Hearts': 'heart_02.png',
            'グリーンハート': 'heart_03.png', 'Green Hearts': 'heart_03.png',
            'ブルーハート': 'heart_04.png', 'Blue Hearts': 'heart_04.png',
            'パープルハート': 'heart_05.png', 'Purple Hearts': 'heart_05.png',
            '全色ハート': 'heart_06.png', 'All Color Hearts': 'heart_06.png',
            'ブレード': 'icon_blade.png', 'Blade': 'icon_blade.png', 'Blades': 'icon_blade.png',
            '開始時': 'icon_live_start.png',
        };

        for (const [key, path] of Object.entries(iconMap)) {
            const regex = new RegExp(`【${key}】`, 'g');
            let style = 'height: 1.1em; vertical-align: middle;';
            if (path.includes('live_start') || path.includes('live_success')) {
                style += ' min-width: 3.5em;';
            } else if (path.includes('all')) {
                style += ' min-width: 2em;';
            }

            let src = path.includes('/') ? `img/${path}` : `img/texticon/${path}`;
            if (typeof ICON_DATA_URIs !== 'undefined' && ICON_DATA_URIs[key]) {
                src = ICON_DATA_URIs[key];
            }
            text = text.replace(regex, `<img src="${src}" alt="${key}" style="${style}">`);
        }

        // --- Complex Replacement for {{icon|alt}} syntax ---
        text = text.replace(/{{(.*?)\|(.*?)}}/g, (match, img, alt) => {
            let src = "img/" + img;
            if (img.endsWith('.png') && !img.includes('/')) {
                src = "img/texticon/" + img;
            }
            let style = "height: 1.1em; vertical-align: middle;";
            if (img.includes('live_start') || img.includes('live_success')) {
                style += ' min-width: 3.5em;';
            }
            const iconKey = img.replace('.png', '');
            if (typeof ICON_DATA_URIs !== 'undefined' && ICON_DATA_URIs[iconKey]) {
                src = ICON_DATA_URIs[iconKey];
            }
            // Use quotes for event handlers, and ensure structural correctness
            return `<span class="icon-wrapper"><img src="${src}" alt="${alt}" style="${style}" onerror="this.style.display='none'; this.nextElementSibling.style.display='inline';"><span style="display:none;">${alt}</span></span>`;
        });
        text = text.replace(/\\n/g, '<br>');

        return text;
    },

    /**
     * Determines which ability text to show based on settings and language.
     */
    /**
     * Translates and enriches ability text for display.
     */
    getEffectiveAbilityText: (card) => {
        const raw = Tooltips.getEffectiveRawText(card);
        return Tooltips.enrichAbilityText(raw);
    },

    /**
     * Translates ability text based on settings and language, without HTML enrichment.
     */
    getEffectiveRawText: (card) => {
        if (!card) return "";
        const rawText = card.text || "";
        const originalText = card.original_text;

        let effectiveText = "";
        if (State.currentLang === 'en') {
            const originalTextEn = card.original_text_en || '';
            if (State.showFriendlyAbilities) {
                effectiveText = window.translateAbility ? window.translateAbility(rawText, 'en') : rawText;
            } else {
                if (originalTextEn) {
                    effectiveText = originalTextEn;
                } else {
                    effectiveText = window.translateAbility ? window.translateAbility(rawText, 'en') : rawText;
                }
            }
        } else if (State.showFriendlyAbilities) {
            effectiveText = window.translateAbility ? window.translateAbility(rawText, 'jp') : rawText;
        } else {
            if (originalText) {
                effectiveText = originalText;
            } else {
                effectiveText = window.translateAbility ? window.translateAbility(rawText, 'jp') : rawText;
            }
        }
        return effectiveText;
    },

    /**
     * Helper to find a card object in the current game state by its unique ID.
     */
    findCardById: (cardId) => {
        const state = State.data;
        if (!state || !state.players) return null;
        for (const p of state.players) {
            if (p.hand) {
                const c = p.hand.find(x => x.id === cardId);
                if (c) return c;
            }
            if (p.stage) {
                const c = p.stage.find(x => x && x.id === cardId);
                if (c) return c;
            }
            if (p.live_zone) {
                const c = p.live_zone.find(x => x && x.id === cardId);
                if (c) return c;
            }
            if (p.energy) {
                const eEntry = p.energy.find(x => x && x.card && x.card.id === cardId);
                if (eEntry) return eEntry.card;
            }
            if (p.discard) {
                const c = p.discard.find(x => x && (typeof x === 'object' ? x.id === cardId : x === cardId));
                if (c) return typeof c === 'object' ? c : { id: c, name: "Card" };
            }
            if (p.waiting_room) {
                const c = p.waiting_room.find(x => x && x.id === cardId);
                if (c) return c;
            }
        }
        return null;
    },

    showTooltip: (target, e, forceTarget = null, useSidebar = false, explicitText = null) => {
        const effectiveTarget = forceTarget || target;
        const descPanel = document.getElementById('card-desc-panel');
        const descContent = document.getElementById('card-desc-content');

        // 1. HIGHLIGHT MANAGEMENT
        if (currentTooltipTarget && currentTooltipTarget !== effectiveTarget) {
            currentTooltipTarget.classList.remove('highlight-hover');
        }
        currentTooltipTarget = effectiveTarget;
        effectiveTarget.classList.add('highlight-hover');

        // 2. DATA RESOLUTION
        const state = State.data;
        const perspectivePlayer = State.perspectivePlayer;
        let cardObj = null;

        // Resolve via Action OR Card ID
        const actionId = effectiveTarget.dataset.actionId;
        const cardId = effectiveTarget.dataset.cardId;

        if (actionId !== undefined && state && state.legal_actions) {
            const actionObj = state.legal_actions.find(a => a.id === parseInt(actionId));
            if (actionObj) {
                Tooltips.highlightAction(actionObj);
                // Priority 1: source_card_id
                if (actionObj.source_card_id !== undefined && actionObj.source_card_id !== -1) {
                    cardObj = Tooltips.findCardById(actionObj.source_card_id);
                }
                // Priority 2: Index-based lookup for your own cards
                const p = state.players[perspectivePlayer];
                if (!cardObj && p) {
                    if (actionObj.hand_idx !== undefined && p.hand) cardObj = p.hand[actionObj.hand_idx];
                    else if (actionObj.area_idx !== undefined && p.stage) cardObj = p.stage[actionObj.area_idx];
                }
            }
        }

        if (!cardObj && cardId !== undefined) {
            cardObj = Tooltips.findCardById(parseInt(cardId));
        }

        // 3. UI UPDATE (SIDEBAR ONLY)
        if (descContent && descPanel) {
            const descTitle = document.getElementById('card-desc-title');
            let finalText = explicitText;
            let titleText = (State.currentLang === 'jp' ? 'カード能力' : 'Card Ability');

            // If we have a resolved card, prioritize its full ability
            if (cardObj && !cardObj.hidden) {
                finalText = Tooltips.getEffectiveRawText(cardObj);
                titleText = cardObj.name || titleText;
            }
            // Fallback to action info if it's an action button
            else if (actionId !== undefined && state && state.legal_actions) {
                const actionObj = state.legal_actions.find(a => a.id === parseInt(actionId));
                if (actionObj) {
                    titleText = actionObj.name || titleText;
                    if (!finalText) finalText = actionObj.raw_text || actionObj.text;
                }
            }

            // Fallback to data-text if no card was found (e.g. choice buttons)
            if (!finalText) {
                finalText = effectiveTarget.dataset.text;
            }

            if (finalText) {
                if (descTitle) descTitle.innerHTML = titleText;
                descContent.innerHTML = Tooltips.enrichAbilityText(finalText);
                descContent.dataset.rawText = finalText;
                descPanel.style.display = 'flex';

                // Success: we clear any "stale" hiding logic
                if (tooltipHideTimeout) {
                    clearTimeout(tooltipHideTimeout);
                    tooltipHideTimeout = null;
                }
            } else {
                // USER REQUEST: The panel should persist.
                // We DON'T hide the panel just because this specific element has no text,
                // unless it's a deliberate mouseout of the board entirely (handled in hideTooltip).
            }
        }

        // 4. FLOATING TOOLTIP REMOVAL (Cleanup)
        const floatingTooltip = document.getElementById('floating-tooltip');
        if (floatingTooltip) {
            floatingTooltip.style.display = 'none';
            floatingTooltip.style.opacity = '0';
        }
    },

    hideTooltip: (immediate = false) => {
        clearTimeout(tooltipTimeout);
        tooltipTimeout = null;

        const hideAction = () => {
            tooltipHideTimeout = null;

            // Remove highlight from previous target
            if (currentTooltipTarget) {
                currentTooltipTarget.classList.remove('highlight-hover');
            }
            currentTooltipTarget = null;

            // CRITICAL: We NO LONGER hide the card-desc-panel on mouseout.
            // It persists until a new card is hovered or the page is refreshed.
            // This fixes the "only sometimes comes up" issue by preventing it from disappearing
            // when the mouse moves between cards or into "dead space" between elements.
            Tooltips.clearHighlights();
        };

        if (immediate) {
            clearTimeout(tooltipHideTimeout);
            hideAction();
            return;
        }

        if (tooltipHideTimeout) return;
        tooltipHideTimeout = setTimeout(hideAction, 100);
    },

    addHighlight: (idOrEl, className) => {
        const el = typeof idOrEl === 'string' ? document.getElementById(idOrEl) : idOrEl;
        if (el) {
            el.classList.add(className);
            if (el.closest && el.closest('.card-area.hand')) {
                el.scrollIntoView({ behavior: 'smooth', block: 'nearest', inline: 'center' });
            }
        }
    },

    clearHighlights: () => {
        document.querySelectorAll('.highlight-source, .highlight-target, .highlight-target-opp, .valid-drop-target, .drop-hover, .highlight-hover').forEach(el => {
            el.classList.remove('highlight-source', 'highlight-target', 'highlight-target-opp', 'valid-drop-target', 'drop-hover', 'highlight-hover');
        });
    },

    highlightAction: (a) => {
        const state = State.data;
        if (!state) return;
        Tooltips.clearHighlights();

        const perspectivePlayer = State.perspectivePlayer;
        const actingPlayer = state.current_player ?? state.active_player;
        const selfPrefix = (actingPlayer === perspectivePlayer ? 'my' : 'opp');
        const oppPrefix = (actingPlayer === perspectivePlayer ? 'opp' : 'my');

        // Helper to get prefix for a target player
        const getPlayerPrefix = (targetId) => {
            if (targetId === undefined) return selfPrefix;
            return (targetId === perspectivePlayer ? 'my' : 'opp');
        };

        const m = a.metadata || {};
        const targetPlayer = m.target_player;
        const targetPrefix = getPlayerPrefix(targetPlayer);

        let specificHighlighted = false;

        if (a.type === 'PLAY') {
            if (a.hand_idx !== undefined) {
                Tooltips.addHighlight(`${selfPrefix}-hand-card-${a.hand_idx}`, 'highlight-source');
                specificHighlighted = true;
            }
            if (a.area_idx !== undefined) {
                Tooltips.addHighlight(`${selfPrefix}-stage-slot-${a.area_idx}`, 'highlight-target');
                specificHighlighted = true;
            }
        } else if (a.type === 'LIVE_SET') {
            if (a.hand_idx !== undefined) {
                Tooltips.addHighlight(`${selfPrefix}-hand-card-${a.hand_idx}`, 'highlight-source');
                specificHighlighted = true;
            }
            Tooltips.addHighlight(`${selfPrefix}-live`, 'highlight-target');
            specificHighlighted = true;
        } else if (a.type === 'ABILITY' || (a.metadata && a.metadata.category === 'ABILITY')) {
            if (a.location === 'discard' || (a.metadata && a.metadata.location === 'discard')) {
                Tooltips.addHighlight(`${selfPrefix}-discard`, 'highlight-source');
                specificHighlighted = true;
            } else if (a.area_idx !== undefined) {
                Tooltips.addHighlight(`${selfPrefix}-stage-slot-${a.area_idx}`, 'highlight-source');
                specificHighlighted = true;
            } else if (a.slot_idx !== undefined) {
                Tooltips.addHighlight(`${targetPrefix}-stage-slot-${a.slot_idx}`, 'highlight-source');
                specificHighlighted = true;
            } else if (a.source_card_id !== undefined && a.source_card_id !== -1) {
                // Find and highlight by ID if indices aren't available
                Tooltips.highlightCardById(a.source_card_id);
                specificHighlighted = true;
            }
        } else if (a.type === 'MULLIGAN') {
            if (a.hand_idx !== undefined) {
                Tooltips.addHighlight(`${selfPrefix}-hand-card-${a.hand_idx}`, 'highlight-target');
                specificHighlighted = true;
            }
        } else if (a.type === 'SELECT_HAND') {
            const hIdx = a.hand_idx ?? m.hand_idx;
            if (hIdx !== undefined) {
                const id = `${targetPrefix}-hand-card-${hIdx}`;
                Tooltips.addHighlight(id, 'highlight-source');
                specificHighlighted = true;
                const card = document.getElementById(id);
                if (card && card.dataset.text) {
                    Tooltips.showTooltip(card, { clientX: card.getBoundingClientRect().right + 10, clientY: card.getBoundingClientRect().top });
                }
            }
        } else if (a.type === 'SELECT_STAGE' || a.type === 'SELECT_MEMBER') {
            const idx = a.slot_idx ?? a.area_idx ?? m.slot_idx;
            if (idx !== undefined) {
                const id = `${targetPrefix}-stage-slot-${idx}`;
                Tooltips.addHighlight(id, 'highlight-target');
                specificHighlighted = true;
                const slot = document.getElementById(id);
                if (slot && slot.dataset.text) {
                    Tooltips.showTooltip(slot, { clientX: slot.getBoundingClientRect().right + 10, clientY: slot.getBoundingClientRect().top });
                }
            }
        } else if (a.type === 'SELECT' || a.type === 'TARGET_OPPONENT' || m.category === 'CHOICE' || m.category === 'SELECT') {
            if (a.type === 'TARGET_OPPONENT' || (state.pending_choice && state.pending_choice.type === 'TARGET_OPPONENT_MEMBER') || m.opcode === 32) { // O_TAP_OPPONENT = 32
                const idx = a.index ?? a.slot_idx ?? m.slot_idx;
                if (idx !== undefined) {
                    const prefix = (a.type === 'TARGET_OPPONENT' || m.opcode === 32) ? oppPrefix : targetPrefix;
                    const id = `${prefix}-stage-slot-${idx}`;
                    Tooltips.addHighlight(id, 'highlight-target');
                    specificHighlighted = true;
                }
            } else if (m.hand_idx !== undefined) {
                const id = `${targetPrefix}-hand-card-${m.hand_idx}`;
                Tooltips.addHighlight(id, 'highlight-target');
                specificHighlighted = true;
            } else if (m.slot_idx !== undefined) {
                const id = `${targetPrefix}-stage-slot-${m.slot_idx}`;
                Tooltips.addHighlight(id, 'highlight-target');
                specificHighlighted = true;
            } else if (m.energy_idx !== undefined) {
                const id = `${targetPrefix}-energy-slot-${m.energy_idx}`;
                Tooltips.addHighlight(id, 'highlight-target');
                specificHighlighted = true;
            } else {
                Tooltips.addHighlight(`select-list-item-${a.index ?? a.id}`, 'highlight-target');
                specificHighlighted = true;
            }
        } else if (a.type === 'SELECT_DISCARD' || m.category === 'DISCARD') {
            Tooltips.addHighlight(`${targetPrefix}-discard`, 'highlight-target');
            specificHighlighted = true;
        } else if (a.type === 'COLOR' || (a.id >= 580 && a.id <= 586)) {
            // Visual aids for color select
            specificHighlighted = true;
        } else if (a.type === 'SELECT_LIVE' || a.type === 'LIVE_PERFORM' || (a.id >= 900 && a.id <= 902) || m.category === 'LIVE') {
            const idx = (a.area_idx !== undefined) ? a.area_idx : (a.slot_idx !== undefined ? a.slot_idx : (a.id >= 900 && a.id <= 902 ? a.id - 900 : (a.id >= 600 && a.id <= 602 ? a.id - 600 : -1)));
            if (idx !== -1) {
                const id = `${targetPrefix}-live-slot-${idx}`;
                Tooltips.addHighlight(id, 'highlight-target');
                specificHighlighted = true;
            }
        } else if (a.type === 'FORMATION' || a.type === 'MOVE') {
            if (a.src_idx !== undefined || a.source_idx !== undefined || a.prev_idx !== undefined) {
                const idx = a.src_idx !== undefined ? a.src_idx : (a.source_idx !== undefined ? a.source_idx : a.prev_idx);
                Tooltips.addHighlight(`${targetPrefix}-stage-slot-${idx}`, 'highlight-source');
            }
            if (a.dst_idx !== undefined || a.area_idx !== undefined || a.slot_idx !== undefined) {
                const idx = a.dst_idx !== undefined ? a.dst_idx : (a.area_idx !== undefined ? a.area_idx : a.slot_idx);
                Tooltips.addHighlight(`${targetPrefix}-stage-slot-${idx}`, 'highlight-target');
            }
            specificHighlighted = true;
        }

        // --- CATCH-ALL FOR ALL ACTION IDs ---
        // If not specific highlighted, but we have common indices, try to highlight them as target
        if (!specificHighlighted) {
            // Check for explicit ID ranges if metadata is missing
            const aid = a.id;
            if (aid >= 600 && aid <= 602) { // Stage/Live Slot selection
                const prefix = (state.phase === 'LiveResult' ? selfPrefix : oppPrefix); // Typically targets opponent in Response
                const targetId = state.phase === 'LiveResult' ? `${prefix}-live-slot-${aid - 600}` : `${prefix}-stage-slot-${aid - 600}`;
                Tooltips.addHighlight(targetId, 'highlight-target');
            } else if (aid >= 300 && aid <= 399) { // Mulligan (only applies in Mulligan phase or if type is explicitly MULLIGAN)
                if (state.phase && state.phase.includes('Mulligan')) {
                    Tooltips.addHighlight(`${selfPrefix}-hand-card-${aid - 300}`, 'highlight-target');
                }
            } else if (aid >= 400 && aid <= 499) { // Live Set
                Tooltips.addHighlight(`${selfPrefix}-hand-card-${aid - 400}`, 'highlight-source');
                Tooltips.addHighlight(`${selfPrefix}-live`, 'highlight-target');
            } else if (aid >= 500 && aid <= 509) { // SELECT_MODE
                // Intentionally do nothing. Mode choices are abstract and don't target hand cards.
            } else if (aid >= 8000 && aid <= 8999) { // LOOK_AND_CHOOSE or generic choice
                // For LOOK_AND_CHOOSE, the UI element often has data-action-id matching this,
                // but the card itself might be in the looked-cards panel.
                // It is abstract in terms of board state highlighting.
            } else {
                const slotIdx = a.slot_idx !== undefined ? a.slot_idx : (a.index !== undefined ? a.index : a.choice_idx);
                if (slotIdx !== undefined && slotIdx !== -1) {
                    if (a.type === 'SELECT_LIVE' || (a.id >= 900 && a.id <= 902)) {
                        Tooltips.addHighlight(`${selfPrefix}-live-slot-${slotIdx}`, 'highlight-target');
                    } else {
                        Tooltips.addHighlight(`${selfPrefix}-stage-slot-${slotIdx}`, 'highlight-target');
                    }
                }
                if (a.hand_idx !== undefined && a.hand_idx !== -1) {
                    Tooltips.addHighlight(`${selfPrefix}-hand-card-${a.hand_idx}`, 'highlight-target');
                }
                if (a.area_idx !== undefined && a.area_idx !== -1) {
                    const id = a.type === 'LIVE_SET' ? `${selfPrefix}-live-slot-${a.area_idx}` : `${selfPrefix}-stage-slot-${a.area_idx}`;
                    Tooltips.addHighlight(id, 'highlight-target');
                }
            }
        }

        // Always check for a source card ID even if specific highlighted
        let srcCardId = a.source_card_id;
        // If action has no source but we're in a pending choice, the choice might have the source
        if ((srcCardId === undefined || srcCardId === -1) && state.pending_choice) {
            srcCardId = state.pending_choice.source_card_id || (state.pending_choice.params ? state.pending_choice.params.source_card_id : -1);
        }

        if (srcCardId !== undefined && srcCardId !== -1) {
            Tooltips.highlightCardById(srcCardId, 'highlight-source');
        }
    },

    highlightPendingSource: () => {
        const state = State.data;
        if (!state || !state.pending_choice) return;
        const choice = state.pending_choice;
        const srcId = choice.source_card_id || (choice.params ? choice.params.source_card_id : -1);

        if (srcId === undefined || srcId === -1) return;

        let found = false;
        const perspectivePlayer = State.perspectivePlayer;
        const selfPrefix = (state.active_player === perspectivePlayer ? 'my' : 'opp');

        const area = choice.area !== undefined ? choice.area : (choice.params ? choice.params.area : undefined);
        if (area !== undefined) {
            Tooltips.addHighlight(`${selfPrefix}-stage-slot-${area}`, 'highlight-source');
            found = true;
        }

        const handIdx = choice.hand_idx !== undefined ? choice.hand_idx : (choice.params ? choice.params.hand_idx : undefined);
        if (handIdx !== undefined) {
            Tooltips.addHighlight(`${selfPrefix}-hand-card-${handIdx}`, 'highlight-source');
            found = true;
        }

        if (!found) {
            Tooltips.highlightCardById(srcId);
        }
    },

    highlightCardById: (srcId, className = 'highlight-source') => {
        const state = State.data;
        if (!state) return;

        const perspectivePlayer = State.perspectivePlayer;
        const playersMap = [
            { id: perspectivePlayer, prefix: 'my' },
            { id: 1 - perspectivePlayer, prefix: 'opp' }
        ];

        playersMap.forEach(pMap => {
            const p = state.players[pMap.id];
            if (!p) return;

            if (p.stage) {
                p.stage.forEach((card, idx) => {
                    const cid = card ? card.id : -1;
                    if (cid === srcId) Tooltips.addHighlight(`${pMap.prefix}-stage-slot-${idx}`, 'highlight-source');
                });
            }
            if (p.hand) {
                p.hand.forEach((card, idx) => {
                    const cid = card ? card.id : -1;
                    if (cid === srcId) Tooltips.addHighlight(`${pMap.prefix}-hand-card-${idx}`, 'highlight-source');
                });
            }
            if (p.live_zone) {
                p.live_zone.forEach((cardObj, idx) => {
                    const cid = cardObj ? cardObj.id : -1;
                    if (cid === srcId) Tooltips.addHighlight(`${pMap.prefix}-live-slot-${idx}`, 'highlight-source');
                });
            }
            if (p.discard && p.discard.some(c => (typeof c === 'object' ? c.id === srcId : c === srcId))) {
                Tooltips.addHighlight(`${pMap.prefix}-discard`, 'highlight-source');
            }
            if (p.energy) {
                p.energy.forEach((e, idx) => {
                    const cid = (e && e.card) ? e.card.id : -1;
                    if (cid === srcId) Tooltips.addHighlight(`${pMap.prefix}-energy-slot-${idx}`, className);
                });
            }
        });
    },

    highlightValidZones: (source, index) => {
        const state = State.data;
        if (!state || !state.legal_actions) return;

        const validTargets = new Set();
        const handIdx = index;

        state.legal_actions.forEach(a => {
            if (source === 'hand') {
                if (a.hand_idx === handIdx) {
                    if (a.type === 'PLAY' || a.type === 'FORMATION') {
                        if (a.area_idx !== undefined) validTargets.add(`my-stage-slot-${a.area_idx}`);
                        if (a.slot_idx !== undefined) validTargets.add(`my-stage-slot-${a.slot_idx}`);
                    }
                    if (a.type === 'LIVE_SET') {
                        if (a.slot_idx !== undefined) {
                            validTargets.add(`my-live-slot-${a.slot_idx}`);
                        } else {
                            for (let i = 0; i < 3; i++) validTargets.add(`my-live-slot-${i}`);
                        }
                    }
                }
                if ((a.hand_idx === handIdx || a.index === handIdx) &&
                    (a.type === 'SELECT_HAND' || (a.name && a.name.includes('Discard')))) {
                    validTargets.add('my-discard');
                }
                if (a.hand_idx === handIdx && a.id >= 600 && a.id <= 602) {
                    validTargets.add(`opp-stage-slot-${a.id - 600}`);
                }
            } else if (source === 'stage') {
                const sourceSlot = index;
                if (a.id >= 600 && a.id <= 602) {
                    validTargets.add(`opp-stage-slot-${a.id - 600}`);
                }
                if ((a.type === 'FORMATION' || a.type === 'MOVE') &&
                    (a.source_idx === sourceSlot || a.prev_idx === sourceSlot)) {
                    if (a.area_idx !== undefined) validTargets.add(`my-stage-slot-${a.area_idx}`);
                    if (a.slot_idx !== undefined) validTargets.add(`my-stage-slot-${a.slot_idx}`);
                }
            } else if (source === 'discard') {
                if (a.discard_idx === index || a.index === index) {
                    if (a.type === 'SELECT_DISCARD' || a.type === 'SELECT_CARD') {
                        validTargets.add('my-hand');
                    }
                    if (a.type === 'PLAY_MEMBER_FROM_DISCARD') {
                        if (a.area_idx !== undefined) validTargets.add(`my-stage-slot-${a.area_idx}`);
                    }
                }
            } else if (source === 'deck') {
                const Phase = { DRAW: 'DRAW' }; // Fallback
                if (a.id === 2 || (a.id === 0 && state.phase === Phase.DRAW)) {
                    validTargets.add('my-hand');
                }
            }
        });

        validTargets.forEach(id => {
            const el = document.getElementById(id);
            if (el) {
                el.classList.add('valid-drop-target');
                if (id.includes('slot-')) {
                    const container = el.closest('.board-slot-container');
                    if (container) container.classList.add('valid-drop-target');
                }
            }
        });
    },

    highlightStageCard: (areaIdx) => {
        Tooltips.clearHighlights();
        Tooltips.addHighlight(`my-stage-slot-${areaIdx}`, 'highlight-source');
    },

    getEffectiveActionText: (action) => {
        if (!action) return "";
        const rawText = action.raw_text || action.text || "";
        let effectiveText = rawText;

        const currentLang = State.currentLang;
        const showFriendlyAbilities = State.showFriendlyAbilities;

        // If we want friendly/English, use the translator on the raw pseudocode
        if ((currentLang === 'en' || showFriendlyAbilities) && window.translateAbility) {
            effectiveText = window.translateAbility(rawText, currentLang);
        }
        // Otherwise in Japanese mode without friendly abilities, try to get original Japanese text
        else if (currentLang === 'jp') {
            const srcCard = State.resolveCardData(action.source_card_id);
            if (srcCard && srcCard.original_text) {
                effectiveText = srcCard.original_text;
            } else {
                // Fallback: Force translation if no original text found (to avoid raw variable names)
                if (window.translateAbility) {
                    effectiveText = window.translateAbility(rawText, 'jp');
                }
            }
        }

        // Apply icon enrichment
        let text = Tooltips.enrichAbilityText(effectiveText);

        // Final cleanup: strip technical prefixes if they persist in the summary
        text = text.replace(/TRIGGER:\s*/g, '');
        text = text.replace(/\[TRIGGER\]\s*/g, '');

        return text;
    },

    getActionTags: (action, vertical = false) => {
        if (!action || !action.triggers) return "";
        const tags = [];
        if (action.triggers.includes(1)) tags.push(`<img src="img/texticon/toujyou.png" alt="[登場時]" style="height:14px; vertical-align:middle;">`);
        if (action.triggers.includes(2)) tags.push(`<img src="img/texticon/live_kaishi.png" alt="[開始時]" style="height:14px; vertical-align:middle;">`);
        if (action.triggers.includes(7)) tags.push(`<img src="img/texticon/kidou.png" alt="[起動]" style="height:14px; vertical-align:middle;">`);
        if (action.triggers.includes(6)) tags.push(`<img src="img/texticon/jyouji.png" alt="[常時]" style="height:14px; vertical-align:middle;">`);

        if (tags.length === 0) return "";
        if (vertical) {
            return `<div style="display:flex; flex-direction:column; align-items:center; gap:1px; margin-left:4px;">${tags.join('')}</div>`;
        }
        return `<div style="display:flex; gap:3px; margin-top:2px;">${tags.join('')}</div>`;
    }
};

// Global Event Listeners for Tooltips
if (typeof document !== 'undefined') {
    document.body.addEventListener('mouseover', (e) => {
        const selector = '.card, .member-slot, .energy-pip, .modifier-line, .action-btn, .action-group, .btn, .modal-content, .active-ability-tag, .perf-guide-entry';
        const target = e.target.closest(selector);

        console.log("Tooltip mouseover event:", e.target, "Target matched:", target);

        if (target) {
            if (tooltipHideTimeout) {
                clearTimeout(tooltipHideTimeout);
                tooltipHideTimeout = null;
            }
            Tooltips.showTooltip(target, e, null, false);
        } else {
            Tooltips.hideTooltip();
        }
    });

    document.body.addEventListener('mouseout', (e) => {
        const selector = '.card, .member-slot, .energy-pip, .modifier-line, .action-btn, .action-group, .btn, .modal-content, .active-ability-tag, .perf-guide-entry';
        const target = e.target.closest(selector);
        if (target) {
            const nextTarget = e.relatedTarget ? e.relatedTarget.closest(selector) : null;
            if (!nextTarget) {
                Tooltips.hideTooltip();
            }
        }
    });

    window.addEventListener('scroll', () => Tooltips.hideTooltip(true), { passive: true });
    window.addEventListener('click', () => Tooltips.hideTooltip(true));
}
