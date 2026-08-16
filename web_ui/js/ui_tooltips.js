/**
 * UI Tooltips & Highlighting facade
 * Delegates to TextEnricher and Highlighter for backward compatibility.
 */
import { State } from './state.js';
import { TextEnricher } from './utils/TextEnricher.js';
import { Highlighter } from './components/Highlighter.js';
import { fixImg } from './constants.js';
import { resolveCardImagePath } from './components/CardRenderer.js';

let tooltipTimeout = null;
let tooltipHideTimeout = null;
let currentTooltipTarget = null;

// Lazily-cached DOM nodes — re-queried if cache is stale.
let _panel, _content, _title, _image;
function dom() {
    if (!_panel || !_panel.isConnected) {
        _panel = document.getElementById('card-desc-panel');
        _content = document.getElementById('card-desc-content');
        _title = document.getElementById('card-desc-title');
        _image = document.getElementById('card-desc-image');
    }
    return { panel: _panel, content: _content, title: _title, image: _image };
}

export const Tooltips = {
    findCardById: (cardId) => State.resolveCardData(cardId),

    attachCardData: (el, cardOrId, actionId = undefined) => {
        if (!el || !cardOrId) return;

        let card = typeof cardOrId === 'object' ? cardOrId : Tooltips.findCardById(cardOrId);
        if (!card && typeof cardOrId === 'string' && cardOrId.includes('ID:')) {
            const idMatch = cardOrId.match(/ID: (\d+)/);
            if (idMatch) card = Tooltips.findCardById(parseInt(idMatch[1]));
        }
        // Stage slot objects have card_no but not id/card_id — resolve from database
        if (!card && cardOrId.card_no) {
            card = State.resolveCardData(cardOrId.card_no);
        }

        if (!card) {
            if (actionId !== undefined && actionId !== 0) el.setAttribute('data-action-id', actionId);
            return;
        }

        if (card.card_no) {
            const resolved = State.resolveCardData(card.card_no);
            if (resolved && resolved.card_no === card.card_no) card = resolved;
        }

        const cid = card.id !== undefined ? card.id : card.card_id;
        if (cid !== undefined && cid !== -1) el.setAttribute('data-card-id', cid);
        if (card.name) el.setAttribute('data-card-name', card.name);
        if (card.card_no) el.setAttribute('data-card-no', card.card_no);

        let rawText = (State.cardSet === 'vanilla') ? "" : TextEnricher.getEffectiveRawText(card);

        // If we have an action context with a specific ability index, filter
        // data-text to show only the relevant ability block (not all abilities).
        if (rawText && actionId !== undefined && State.data?.legal_actions) {
            const actionObj = State.data.legal_actions.find(a => a.index === actionId);
            if (actionObj) {
                const abilityIdx = actionObj.parameters?.ability_index ?? actionObj.params?.ability_index;
                if (abilityIdx !== undefined) {
                    const block = TextEnricher.extractRelevantAbility(card, null, abilityIdx);
                    if (block) rawText = block;
                }
            }
        }

        if (rawText) el.setAttribute('data-text', rawText);

        if (actionId !== undefined && actionId !== 0) el.setAttribute('data-action-id', actionId);
    },

    showTooltip: (target, e, forceTarget = null, useSidebar = false, explicitText = null) => {
        const effectiveTarget = forceTarget || target;
        const dataSource = effectiveTarget.closest('[data-card-id],[data-action-id],[data-text]') || effectiveTarget;

        if (currentTooltipTarget && currentTooltipTarget !== dataSource) {
            currentTooltipTarget.classList.remove('highlight-hover');
        }
        currentTooltipTarget = dataSource;
        dataSource.classList.add('highlight-hover');

        const state = State.data;
        const perspectivePlayer = State.perspectivePlayer;
        const actionId = dataSource.dataset.actionId;
        const cardId = dataSource.dataset.cardId;
        const cardNo = dataSource.dataset.cardNo;
        const cardName = dataSource.dataset.cardName;

        // The hovered element is the authoritative source for which card it
        // represents. Resolve it first so we never show a different card just
        // because an action happens to reference some other card.
        let cardObj = cardId !== undefined ? Tooltips.findCardById(parseInt(cardId)) : null;
        if (!cardObj && cardNo) cardObj = State.resolveCardData(cardNo);
        if (!cardObj && cardName) cardObj = State.resolveCardDataByName(cardName);

        // Action is used for text enrichment only; it must never override the
        // hovered card. It is consulted only as a last resort to identify a
        // card when the element itself carries none.
        let actionObj = null;
        if (actionId !== undefined && state && state.legal_actions) {
            // Rust backend: use action.index instead of action.id
            actionObj = state.legal_actions.find(a => a.index === parseInt(actionId));
            if (!cardObj && actionObj) {
                // highlight targets removed — data-action-id was shared across area buttons causing wrong highlights
                // Rust backend format: player1, player2
                const p = perspectivePlayer === 0 ? state.player1 : state.player2;
                if (p) {
                    // Support both parameters and params field names
                    const params = actionObj.parameters || actionObj.params || {};
                    const handCards = p.hand.cards;
                    const liveCards = p.live_zone.cards;
                    const energyCards = p.energy.cards;
                    
                    if (params.card_index !== undefined && handCards.length > 0) cardObj = handCards[params.card_index];
                    else if (params.stage_area && p.stage) {
                        // Rust engine MemberArea serializes as lowercase without underscores: "left", "center", "right"
                        // Support both formats for compatibility
                        const areaMap = { 
                            'left': p.stage.left_side, 
                            'left_side': p.stage.left_side, 
                            'center': p.stage.center, 
                            'right': p.stage.right_side, 
                            'right_side': p.stage.right_side 
                        };
                        cardObj = areaMap[params.stage_area.toLowerCase()];
                    }
                    else if (params.card_indices !== undefined && liveCards.length > 0) cardObj = liveCards[params.card_indices[0]];
                    else if (params.card_index !== undefined && energyCards.length > 0) cardObj = energyCards[params.card_index];
                }
                if (!cardObj && actionObj.source_card_id !== undefined && actionObj.source_card_id !== -1) {
                    cardObj = Tooltips.findCardById(actionObj.source_card_id);
                }
            }
        }

        if (cardObj && cardObj.id !== undefined && cardObj.id >= 0) {
            const master = Tooltips.findCardById(cardObj.id);
            if (master) cardObj = master;
        }

        const dText = dataSource.dataset.text;
        // Always get card text - hidden flag should only hide instance-specific data,
        // not the card's ability which is public knowledge from master data
        // VANILLA MODE: Suppress ability text in vanilla/abilityless mode
        const cardText = (State.cardSet === 'vanilla') ? "" : (cardObj ? (TextEnricher.getEffectiveRawText(cardObj) || "") : "");
        let finalAbilityText = cardText;

        let actionLabel = dText || "";

        // Action enrichment: If we have an action object, try to get even better text
        let actionRichText = "";
        if (actionObj) {
            actionRichText = TextEnricher.getEffectiveActionText(actionObj);
            const rawActionRichText = actionRichText.replace(/<[^>]+>/g, "").trim();

            if (rawActionRichText && !TextEnricher.isGenericInstruction(rawActionRichText)) {
                // Only let action text replace card text if the action's source card
                // matches the hovered card (e.g. use_ability on the same card).
                // Otherwise the action text may come from a different card
                // (e.g. play_member_to_stage hovering a stage slot shows the hand card's ability).
                const actionSrcId = actionObj.source_card_id ?? actionObj._source_card_id;
                const sameCard = cardObj && cardObj.id !== undefined && actionSrcId === cardObj.id;
                if (sameCard || !cardObj || !cardText) {
                    finalAbilityText = actionRichText;
                } else {
                    actionLabel = actionRichText;
                }
            } else if (rawActionRichText && !finalAbilityText.includes(rawActionRichText)) {
                // If it's a generic mechanical instruction (like "Play X to Slot 0"),
                // and it's not already in the text, we might want to keep it as a label.
                actionLabel = actionRichText;
            }
        }

        let combinedText = finalAbilityText;

        if (!combinedText) {
            if (!cardObj) {
                combinedText = dText || "";
            } else if (cardText) {
                combinedText = cardText;
            }
        }

        // Only hide the panel if we have absolutely nothing to show (no card and no text)
        if (!combinedText && !cardObj) {
            dom().panel.classList.remove('active');
            return;
        }

        // Product/series prefix from card_no (e.g. "PL!-pb1" from "PL!-pb1-014-R")
        let productLabel = "";
        let fullCardNo = "";
        if (cardObj && cardObj.card_no) {
            const parts = cardObj.card_no.match(/^(PL!-[A-Za-z0-9]+)/);
            if (parts) productLabel = parts[1];
            fullCardNo = cardObj.card_no;
        }
        const cardIdLabel = (cardObj && cardObj.id !== undefined && cardObj.id >= 0) ? ` <span style="opacity:0.6; font-size:0.8em;">(ID: ${cardObj.id})</span>` : "";
        const cardNoLabel = fullCardNo ? ` <span style="opacity:0.5; font-size:0.75em; font-family:monospace;">${fullCardNo}</span>` : "";
        
        let titleText = (dataSource.dataset.cardName || "Card Detail");
        let metadataHtml = "";

        if (cardObj) {
            const translated = window.translateCard ? window.translateCard(cardObj) : { name: cardObj.name, groups: cardObj.groups, units: cardObj.units };
            titleText = productLabel ? `${productLabel} — ${translated.name}` : translated.name;
            titleText += cardNoLabel + cardIdLabel;
        }

        // Show raw ability JSON when Shift is held
        let rawJsonHtml = "";
        if (cardObj && e?.shiftKey) {
            const abilityText = TextEnricher.getEffectiveRawText(cardObj);
            if (abilityText) {
                const escaped = abilityText.replace(/&/g,'&amp;').replace(/</g,'&lt;').replace(/>/g,'&gt;');
                rawJsonHtml = `<pre style="margin-top:8px; padding:6px; background:#1a1a2e; border:1px solid #444; border-radius:4px; font-size:0.7rem; line-height:1.3; max-height:200px; overflow:auto; white-space:pre-wrap; word-break:break-all;">${escaped}</pre>`;
            }
        }

        if (dom().title) {
            dom().title.innerHTML = titleText;
            dom().title.style.display = titleText ? 'block' : 'none';
        }

        const enrichedText = combinedText ? TextEnricher.enrichAbilityText(combinedText) : "";
        dom().content.innerHTML = metadataHtml + enrichedText + rawJsonHtml;
        dom().content.dataset.rawText = enrichedText;

        const imgContainer = dom().image;
        if (imgContainer) {
            if (cardObj && cardObj.card_no) {
                const imgPath = resolveCardImagePath(cardObj.card_no);
                if (imgPath) {
                    imgContainer.innerHTML = '';
                    const img = document.createElement('img');
                    img.src = fixImg(imgPath);
                    img.alt = cardObj.name || '';
                    img.style.display = 'block';
                    img.style.maxWidth = '100%';
                    img.style.width = '100%';
                    img.style.height = 'auto';
                    imgContainer.appendChild(img);
                } else {
                    imgContainer.innerHTML = '';
                }
            } else {
                imgContainer.innerHTML = '';
            }
        }

        dom().panel.classList.add('active');

        if (tooltipHideTimeout) {
            clearTimeout(tooltipHideTimeout);
            tooltipHideTimeout = null;
        }

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
            if (currentTooltipTarget) {
                currentTooltipTarget.classList.remove('highlight-hover');
            }
            currentTooltipTarget = null;
            Highlighter.clearHighlights();
        };

        if (immediate) {
            clearTimeout(tooltipHideTimeout);
            hideAction();
            return;
        }

        if (tooltipHideTimeout) return;
        tooltipHideTimeout = setTimeout(hideAction, 100);
    },

    // Proxies for backwards compatibility
    enrichAbilityText: (text) => TextEnricher.enrichAbilityText(text),
    splitAbilities: (text) => TextEnricher.splitAbilities(text),
    extractRelevantAbility: (card, triggerLabel, abilityIndex) => TextEnricher.extractRelevantAbility(card, triggerLabel, abilityIndex),

    getEffectiveAbilityText: (card) => TextEnricher.getEffectiveAbilityText(card),
    getEffectiveRawText: (card) => TextEnricher.getEffectiveRawText(card),
    isGenericInstruction: (text) => TextEnricher.isGenericInstruction(text),
    isRichAbility: (text) => TextEnricher.isRichAbility(text),
    getEffectiveActionText: (action) => TextEnricher.getEffectiveActionText(action),
    getActionTags: (action, vertical) => TextEnricher.getActionTags(action, vertical),

    addHighlight: (id, cls) => Highlighter.addHighlight(id, cls),
    clearHighlights: () => Highlighter.clearHighlights(),
    highlightAction: (a) => Highlighter.highlightAction(a),
    highlightPendingSource: () => Highlighter.highlightPendingSource(),
    highlightCardById: (src, cls, first) => Highlighter.highlightCardById(src, cls, first),
    highlightValidZones: (src, idx) => Highlighter.highlightValidZones(src, idx),
    highlightStageCard: (idx) => Highlighter.highlightStageCard(idx),
    highlightTargetsForAction: (act) => Highlighter.highlightTargetsForAction(act)
};

// Global Event Listeners for Tooltips
if (typeof document !== 'undefined') {
    document.body.addEventListener('mouseover', (e) => {
        const selector = '.card, .member-slot, .member-area, .board-slot-container, .under-card, .energy-pip, .modifier-line, .action-btn, .action-group, .btn, .active-ability-tag, .perf-guide-entry, .perf-yell-card, .log-entry, .turn-event-item, .active-effect, .turn-event-hover-container, .active-effect-hover-container, .choice-item';
        const target = e.target.closest(selector);

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
        const selector = '.card, .member-slot, .member-area, .board-slot-container, .under-card, .energy-pip, .modifier-line, .action-btn, .action-group, .btn, .active-ability-tag, .perf-guide-entry, .perf-yell-card, .log-entry, .turn-event-item, .active-effect, .choice-item';
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
