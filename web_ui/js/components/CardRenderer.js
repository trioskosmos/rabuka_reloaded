import { State } from '../state.js';
import { fixImg as fixImgPath, isMulliganPhase, isLiveCardSetPhase } from '../constants.js';
import * as i18n from '../i18n/index.js';
import { Tooltips } from '../ui_tooltips.js';
import { DOMUtils } from '../utils/DOMUtils.js';
import { DOM_IDS } from '../constants_dom.js';

function openCardDetailModal(card, zoneHint) {
    const m = window.__modals?.CardDetailModal;
    if (m) m.open(card, zoneHint);
}

function openPlayActionModal(card, actions) {
    const m = window.__modals?.PlayActionModal;
    if (m) m.open(card, actions);
}

function openStageAbilityModal(card, actions) {
    const m = window.__modals?.StageAbilityModal;
    if (m) m.open(card, actions);
}

// Image loading with retry logic and error handling
export const ImageLoader = {
    loadedImages: new Set(),
    failedImages: new Map(), // src -> retry count
    observer: null,

    init() {
        if (typeof IntersectionObserver !== 'undefined' && !this.observer) {
            this.observer = new IntersectionObserver((entries) => {
                entries.forEach(entry => {
                    if (entry.isIntersecting) {
                        const img = entry.target;
                        if (img.dataset.src && !img.complete) {
                            this._doLoad(img, img.dataset.src);
                        }
                    }
                });
            }, { rootMargin: '50px' });
        }
    },

    _doLoad(img, src, isRetry = false) {
        // Clear previous handlers to avoid duplicates
        img.onload = null;
        img.onerror = null;

        const delays = [200, 500, 1000, 3000, 10000];
        const maxRetries = delays.length;

        img.onload = () => {
            this.loadedImages.add(src);
            this.failedImages.delete(src);
            img.style.opacity = '1';
        };

        img.onerror = () => {
            const retries = this.failedImages.get(src) || 0;
            if (retries < maxRetries) {
                this.failedImages.set(src, retries + 1);
                img.style.opacity = '0.5';
                setTimeout(() => {
                    img.dispatchEvent(new CustomEvent('imageRetrying'));
                    this._doLoad(img, src, true);
                }, delays[retries]);
            } else {
                // Try fallback paths (＋→2, ＋→+, etc.) when all retries exhausted
                const fbRaw = img.dataset.fallbackPaths;
                if (fbRaw) {
                    let fallbacks;
                    try { fallbacks = JSON.parse(fbRaw); } catch (_) { fallbacks = null; }
                    if (fallbacks && fallbacks.length > 0) {
                        const next = fallbacks.shift();
                        img.dataset.fallbackPaths = JSON.stringify(fallbacks);
                        this.failedImages.delete(src);
                        console.log('[ImageLoader] Trying fallback path:', next);
                        this._doLoad(img, next, true);
                        return;
                    }
                }
                console.warn('[ImageLoader] Failed to load image after retries + fallbacks:', src);
                img.style.opacity = '0.3';
                img.dispatchEvent(new CustomEvent('imagePermanentFailure'));
            }
        };

        // Always use cache-busting on retries and re-renders
        const loadSrc = (isRetry || this.failedImages.has(src))
            ? src + (src.includes('?') ? '&' : '?') + '_retry=' + Date.now()
            : src;
        img.src = loadSrc;
    },

    /**
     * Retry a previously failed image. Resets retry count so subsequent
     * state updates (pending choices, re-renders) can give it another shot.
     */
    retryImage(img, src) {
        if (!src || this.loadedImages.has(src)) return;
        this.failedImages.set(src, 0);
        img.style.opacity = '0.5';
        img.dispatchEvent(new CustomEvent('imageRetrying'));
        this._doLoad(img, src, true);
    },

    loadImage(img, src) {
        // src may have _fallbacks attached by resolveCardImagePath
        const fallbacks = src?._fallbacks;
        if (fallbacks && fallbacks.length > 0) {
            img.dataset.fallbackPaths = JSON.stringify(fallbacks);
        }
        if (!src) return;
        this.init();

        // If already successfully loaded, just set it
        if (this.loadedImages.has(src)) {
            img.src = src;
            img.style.opacity = '1';
            return;
        }

        // If previously failed, reset retry count so new DOM elements
        // (e.g. from pending choice re-renders) get a fresh attempt
        if (this.failedImages.has(src)) {
            this.retryImage(img, src);
            return;
        }

        // For immediate load, set src directly
        img.dataset.src = src;
        this._doLoad(img, src);

        // Also observe for lazy loading (in case img is off-screen)
        if (this.observer) {
            this.observer.observe(img);
        }
    },
};

// Consistent image path resolution across all card displays
/**
 * Resolve card image path. Returns the primary candidate path as a string.
 * If there are fallback alternatives (e.g. ＋ → 2, ＋ → +), they are stored
 * as a non-enumerable _fallbacks array on the returned string so the image
 * loader can try them when the primary path fails.
 */
export function resolveCardImagePath(cardNo) {
    if (!cardNo) return '';
    if (cardNo === '-1' || cardNo === -1 || cardNo === '-2' || cardNo === -2) return '';

    const candidates = [];

    // Unicode NFKC normalization: full-width ＋ (U+FF0B) → half-width + (U+002B).
    // This ensures consistent handling regardless of input character width,
    // making 2, ＋, and + equivalent throughout the resolution pipeline.
    const nfkc = cardNo.normalize('NFKC');

    // 1. Direct mapping lookup — try normalized first (future-proof),
    //    then original (current mapping uses full-width keys).
    const mapped = State.cardImageMapping?.[nfkc] || State.cardImageMapping?.[cardNo];
    if (mapped) return fixImgPath(mapped);

    // 2. Also try the opposite width variant in mapping
    const altWidth = nfkc.includes('+')
        ? nfkc.replace(/\+/g, '＋')
        : cardNo.includes('＋') ? cardNo.replace(/＋/g, '+') : null;
    if (altWidth && altWidth !== nfkc && altWidth !== cardNo) {
        const altMapped = State.cardImageMapping?.[altWidth];
        if (altMapped) return fixImgPath(altMapped);
    }

    // 3. Try ＋ → 2, + → 2 variant in mapping (webp uses PR2 not PR＋)
    if (nfkc.includes('+')) {
        const with2 = nfkc.replace(/\+/g, '2');
        const mapped2 = State.cardImageMapping?.[with2];
        if (mapped2) return fixImgPath(mapped2);
    }

    // 4. Try compressed name in mapping (PL!HS-PR-017-PR → PL!HS-017-PR)
    const stripped = nfkc.replace(/^(PL![\w]*)-[A-Z]+-(\d*)-/, '$1-$2-');
    if (stripped !== nfkc) {
        const compressedMapped = State.cardImageMapping?.[stripped];
        if (compressedMapped) return fixImgPath(compressedMapped);
    }

    // 5. Compressed ＋ → 2 (with ＋ ↔ + normalization first)
    if (nfkc.includes('+')) {
        const with2 = nfkc.replace(/\+/g, '2');
        const stripped2 = with2.replace(/^(PL![\w]*)-[A-Z]+-(\d*)-/, '$1-$2-');
        if (stripped2 !== with2) {
            const mapped2 = State.cardImageMapping?.[stripped2];
            if (mapped2) return fixImgPath(mapped2);
        }
    }

    // 6. Build candidate file paths. Always try all three filename conventions
    //    regardless of input form: R2 (numeric), R＋ (full-width), R+ (half-width).
    const addWebp = (n) => candidates.push(fixImgPath(`img/cards_webp/${n}.webp`));
    const seen = new Set();
    const add = (n) => { if (!seen.has(n)) { seen.add(n); addWebp(n); } };
    add(nfkc.replace(/\+/g, '2'));   // R2 variant
    add(nfkc.replace(/\+/g, '＋'));  // R＋ variant (full-width)
    add(nfkc);                       // R+ variant (half-width)

    // 7. Rarity fallback: use rare_list from card database to find alternative rarities
    if (State.staticCardDatabase && State.cardImageMapping) {
        const cardEntry = State.staticCardDatabase[cardNo];
        const rareList = cardEntry?.rare_list;
        if (rareList && rareList.length > 1) {
            for (const entry of rareList) {
                const altCardNo = entry.card_no;
                if (altCardNo !== cardNo) {
                    const altMapped = State.cardImageMapping[altCardNo];
                    if (altMapped) candidates.push(fixImgPath(altMapped));
                }
            }
        }
    }

    // 8. Desperate fallback: strip the last rarity segment and try base-key match
    const rarityFallback = resolveCardImagePath._rarityCache || (() => {
        const cache = {};
        if (State.cardImageMapping) {
            for (const key of Object.keys(State.cardImageMapping)) {
                const base = key.replace(/-[^-]+$/, '');
                if (!cache[base]) cache[base] = [];
                cache[base].push(key);
            }
        }
        resolveCardImagePath._rarityCache = cache;
        return cache;
    })();
    const baseKey = cardNo.replace(/-[^-]+$/, '');
    if (baseKey !== cardNo && rarityFallback[baseKey]) {
        for (const alt of rarityFallback[baseKey]) {
            if (alt !== cardNo) {
                const mappedAlt = State.cardImageMapping?.[alt];
                if (mappedAlt) candidates.push(fixImgPath(mappedAlt));
            }
        }
    }

    // Return primary path with fallbacks attached as a non-enumerable property.
    // Must use new String() so Object.defineProperty has an object target.
    const result = new String(candidates[0] || '');
    if (candidates.length > 1) {
        Object.defineProperty(result, '_fallbacks', {
            value: candidates.slice(1),
            enumerable: false,
            configurable: true,
        });
    }
    return result;
}

export const CardRenderer = {
    /**
     * Maps engine card data to UI-specific properties (CSS classes, labels, etc.)
     */
    getCardViewModel: (card, options = {}) => {
        if (!card) return null;

        const state = State.data;
        const { isSelected, isValid, mini, compact, micro, containerId } = options;

        // Resolve card data if it's just a number or missing name
        let resolvedCard = card;
        if (typeof card === 'number') {
            resolvedCard = State.resolveCardData(card);
        } else if (card.card_no) {
            // Card has card_no - always try to enrich from index/static database
            // This ensures we get _img and other static data even if name is present
            const indexed = State.resolveCardData(card.card_no);
            if (indexed) {
                resolvedCard = { ...card, ...indexed };
            }
        }
        // Rust backend format: card_no, name, card_type, orientation
        // Support both hidden field and card_no === -2/-1 (number or string) for hidden cards
        const isHidden = resolvedCard.hidden || resolvedCard.is_hidden || 
                         resolvedCard.card_no === -2 || resolvedCard.card_no === -1 ||
                         resolvedCard.card_no === "-2" || resolvedCard.card_no === "-1";
        // Engine sends card_type as string enum; static database uses `type`.
        // Check case-insensitively and also check card_no prefix as fallback.
        const isLive = CardRenderer.isCardLive(resolvedCard);

        // 1. Determine CSS Classes
        const classNames = ['card'];
        if (micro) classNames.push('card-micro');
        else if (compact) classNames.push('card-compact');
        else if (mini) classNames.push('card-mini');
        if (resolvedCard.is_new) classNames.push('new-card');
        if (isLive) classNames.push('type-live');

        if (isLive) {
            classNames.push('orientation-landscape');
        }

        if (isSelected) {
            classNames.push('sel');
            if (isMulliganPhase(state.phase)) classNames.push('sel-mulligan');
            else if (isLiveCardSetPhase(state.phase)) classNames.push('sel-live');
            else classNames.push('sel-action');
        }
        if (isValid && containerId !== 'my-hand') classNames.push('valid-target');
        if (!isValid && containerId === 'my-hand') classNames.push('inert');

        // Sticky class for view model: if we match current global hover, keep it
        const isCurrentlyHovered = options.actionId !== undefined && options.actionId === State.hoveredActionId;
        if (isCurrentlyHovered) {
            classNames.push('hover-highlight');
        }
        if (isHidden) classNames.push('card-back');

        // 2. Determine Display Name & Image
        let displayName = 'Card';
        let imgPath = '';

        if (!isHidden) {
            displayName = resolvedCard.name || `[${resolvedCard.card_type}]` || 'Card';
            const cardNo = resolvedCard.card_no;
            if (cardNo) {
                imgPath = resolveCardImagePath(cardNo);
            } else {
                imgPath = fixImgPath(resolvedCard.img || resolvedCard.img_path || '');
            }
        } else {
            imgPath = fixImgPath('img/texticon/lltcg-back.png');
        }

        return {
            classes: classNames.join(' '),
            displayName,
            imgPath,
            cost: 0, // Rust backend doesn't provide cost in card display
            isHidden,
            isValid,
            actionId: options.actionId
        };
    },

    isCardLive: (cardOrData) => {
        if (!cardOrData) return false;
        const rawType = (cardOrData.card_type || cardOrData.type || '').toLowerCase();
        return rawType === 'live' || rawType === 'ライブ' ||
            (typeof cardOrData.card_no === 'string' && cardOrData.card_no.startsWith('live'));
    },

    /**
     * Creates a single card DOM element from a ViewModel
     */
    createCardDOM: (viewModel, cardData, onClick = null) => {
        const div = document.createElement('div');
        div.className = viewModel.classes;

        if (viewModel.actionId !== undefined || cardData.card_no !== undefined) {
            Tooltips.attachCardData(div, cardData, viewModel.actionId);
        }

        if (viewModel.imgPath) {
            const img = document.createElement('img');
            img.draggable = false;
            ImageLoader.loadImage(img, viewModel.imgPath);
            div.appendChild(img);
        }

        if (onClick) {
            div.style.cursor = 'pointer';
            div.onclick = (e) => {
                e.stopPropagation();
                onClick(viewModel.actionId);
            };

            if (viewModel.isValid) {
                div.setAttribute('data-action-id', viewModel.actionId);
                div.onmouseenter = () => {
                    if (window.highlightActionBtn) window.highlightActionBtn(viewModel.actionId, true);
                };
                div.onmouseleave = () => {
                    if (window.highlightActionBtn) window.highlightActionBtn(viewModel.actionId, false);
                };
            }
        }

        return div;
    },

    /**
     * Updates an existing card DOM element with new ViewModel
     */
    updateCardDOM: (el, viewModel, cardData, onClick = null) => {
        DOMUtils.patchClasses(el, viewModel.classes);
        
        // Stickiness: Only apply if we have a match, but DON'T aggressively remove if actionId is briefly missing
        // or if it was already hovered (let CSS :hover handle local mouse, and highlightActionBtn handle remote)
        const isMatch = viewModel.actionId !== undefined && viewModel.actionId === State.hoveredActionId;
        if (isMatch) {
            el.classList.add('hover-highlight');
        } else if (viewModel.actionId !== undefined && State.hoveredActionId !== null) {
            // We are hovering a different action, so remove this one
            el.classList.remove('hover-highlight');
        }
        // Note: we don't remove if actionId is undefined to prevent flickering during transient states

        if (viewModel.actionId !== undefined || cardData.card_no !== undefined) {
            Tooltips.attachCardData(el, cardData, viewModel.actionId);
        }

        const imgPath = viewModel.imgPath;
        const existingImg = el.querySelector('img');
        
        if (existingImg) {
            if (imgPath) {
                if (existingImg.dataset.originalPath !== imgPath) {
                    ImageLoader.loadImage(existingImg, imgPath);
                    existingImg.dataset.originalPath = imgPath;
                } else if (!ImageLoader.loadedImages.has(imgPath)) {
                    ImageLoader.retryImage(existingImg, imgPath);
                }
                existingImg.style.display = '';
            } else {
                existingImg.remove();
            }
        } else if (imgPath) {
            const img = document.createElement('img');
            img.draggable = false;
            img.dataset.originalPath = imgPath;
            ImageLoader.loadImage(img, imgPath);
            el.prepend(img);
        }

        el.style.cursor = onClick ? 'pointer' : '';
        el.onclick = onClick ? (e) => {
            e.stopPropagation();
            onClick();
        } : null;

        if (onClick && viewModel.isValid) {
            el.setAttribute('data-action-id', viewModel.actionId);
            el.onmouseenter = () => {
                if (window.highlightActionBtn) window.highlightActionBtn(viewModel.actionId, true);
            };
            el.onmouseleave = () => {
                if (window.highlightActionBtn) window.highlightActionBtn(viewModel.actionId, false);
            };
        } else {
            el.removeAttribute('data-action-id');
            el.onmouseenter = null;
            el.onmouseleave = null;
        }

        return el;
    },

    renderCards: (containerId, cards, clickable = false, mini = false, selectedIndices = [], validActionMap = {}, hasGlobalSelection = false, filter = null, micro = false) => {
        const el = DOMUtils.getElement(containerId);
        if (!el) return;
        if (!cards) {
            DOMUtils.clear(containerId);
            return;
        }

        const cardCount = cards.length;

        if (filter) {
            DOMUtils.clear(containerId);
        }

        // Phase 1: Build a key→element map from the live DOM for identity tracking
        const keyToEl = new Map();
        for (let i = 0; i < el.children.length; i++) {
            const child = el.children[i];
            const k = child.dataset.cardKey;
            if (k) keyToEl.set(k, child);
        }

        // Track which keys we still see so we can remove stale nodes
        const seenKeys = new Set();
        let insertBefore = null;

        // Phase 2: Update or create for each card position
        for (let idx = 0; idx < cardCount; idx++) {
            const card = cards[idx];
            if (filter && !filter(card, idx)) continue;

            // Compute a stable card identity key
            let cardKey;
            if (card === null) {
                cardKey = `null_${idx}`;
            } else if (card.id !== undefined && card.id >= 0) {
                cardKey = `${card.card_no}_${card.id}`;
            } else if (card.card_no && card.card_no !== '-1' && card.card_no !== -1) {
                cardKey = card.card_no;
            } else {
                cardKey = `anon_${idx}`;
            }
            seenKeys.add(cardKey);

            const isSelected = selectedIndices.includes(idx);
            const action = validActionMap[idx];
            const isValid = action !== undefined;

            // Check if we already have a DOM element for this card (by key)
            let existingChild = keyToEl.get(cardKey);
            if (existingChild && el.children[idx] !== existingChild) {
                // Card exists but at wrong position — move it
                const ref = el.children[idx] || null;
                el.insertBefore(existingChild, ref);
            }

            // Get or use the element currently at this position
            const childAtPos = el.children[idx] || null;

            if (card === null) {
                if (childAtPos && childAtPos.classList.contains('placeholder')) {
                    childAtPos.style.visibility = 'hidden';
                } else {
                    const placeholder = document.createElement('div');
                    placeholder.className = 'card placeholder' + (mini ? ' card-mini' : '');
                    placeholder.style.visibility = 'hidden';
                    if (childAtPos) el.replaceChild(placeholder, childAtPos);
                    else el.appendChild(placeholder);
                }
                continue;
            }

            const onClick = clickable && (State.uiMode === 'view' || isValid || !hasGlobalSelection) ? (act) => {
                const _uiMode = State.uiMode;

                if (_uiMode === 'view') {
                    openCardDetailModal(card, containerId);
                    return;
                }

                if (!isValid) return;

                if (_uiMode === 'play' && containerId === 'my-hand') {
                    const legalActions = State.data?.legal_actions || [];
                    const playActions = legalActions.filter(a =>
                        a.action_type === 'play_member_to_stage' &&
                        (a.parameters?.card_index === idx || a.parameters?.card_indices?.includes(idx))
                    );
                    if (playActions.length > 0) {
                        if (window._playSel && window._playSel.cardIdx === idx) {
                            window._playSel = null;
                        } else {
                            window._playSel = { cardIdx: idx, card, actions: playActions };
                        }
                        if (window.render) window.render();
                        return;
                    }
                }

                if (_uiMode === 'play' && (containerId === 'my-stage' || containerId === 'my-live')) {
                    const legalActions = State.data?.legal_actions || [];
                    const abilityActions = legalActions.filter(a => {
                        if (a.action_type !== 'use_ability') return false;
                        const params = a.parameters || {};
                        if (params.stage_area && containerId === 'my-stage') return true;
                        if (params.card_indices && containerId === 'my-live') return true;
                        if (params.card_id !== undefined && params.card_id === card.id) return true;
                        return false;
                    });
                    if (abilityActions.length > 0) {
                        openStageAbilityModal(card, abilityActions);
                        return;
                    }
                }

                if (action.action_type === 'select_mulligan') {
                    const handIdx = action.parameters?.card_index ?? action.parameters?.card_indices?.[0];
                    if (handIdx !== undefined) {
                        const isNowSelected = !State.localMulliganSelection.has(handIdx);
                        if (isNowSelected) {
                            State.localMulliganSelection.add(handIdx);
                        } else {
                            State.localMulliganSelection.delete(handIdx);
                        }
                        // Toggle this card without full re-render (preserves images)
                        const cardEl = document.getElementById(`${containerId}-card-${idx}`);
                        if (cardEl) {
                            cardEl.classList.toggle('sel', isNowSelected);
                            cardEl.classList.toggle('sel-mulligan', isNowSelected);
                        }
                        // Sync action button
                        const actionBtn = State.mulliganButtons.get(handIdx);
                        if (actionBtn) {
                            actionBtn.classList.toggle('sel', isNowSelected);
                            actionBtn.classList.toggle('sel-mulligan', isNowSelected);
                            const thumb = actionBtn.querySelector('.action-card-thumb');
                            if (thumb) {
                                thumb.classList.toggle('sel', isNowSelected);
                                thumb.classList.toggle('sel-mulligan', isNowSelected);
                            }
                        }
                    }
                    return;
                }
                if (action.action_type === 'select_live_card') {
                    const handIdx = action.parameters?.card_index ?? action.parameters?.card_indices?.[0];
                    if (handIdx !== undefined) {
                        if (State.localLiveCardSelection.has(handIdx)) {
                            State.localLiveCardSelection.delete(handIdx);
                        } else {
                            const pp = State.perspectivePlayer;
                            const p = pp === 0 ? State.data?.player1 : State.data?.player2;
                            const alreadyPlaced = p?.live_zone?.cards?.length || 0;
                            if (State.localLiveCardSelection.size + alreadyPlaced >= 3) return;
                            State.localLiveCardSelection.add(handIdx);
                        }
                    }
                    window.render?.();
                    return;
                }
                if (window.selectedAction?.index === action.index) {
                    window.selectedAction = null;
                    document.querySelectorAll('.card.selected').forEach(c => c.classList.remove('selected'));
                    if (window.highlightActionBtn) window.highlightActionBtn(null, false);
                    return;
                }
                document.querySelectorAll('.card.selected').forEach(c => c.classList.remove('selected'));
                window.selectedAction = action;
                document.querySelector(`[data-action-id="${action.index}"]`)?.classList.add('selected');
                if (window.highlightActionBtn) window.highlightActionBtn(action.index, true);
            } : null;

            const viewModel = CardRenderer.getCardViewModel(card, {
                isSelected, isValid, mini, micro, containerId, actionId: action?.index
            });

            if (existingChild && !existingChild.classList.contains('placeholder')) {
                // Card was found by key — update in place
                CardRenderer.updateCardDOM(existingChild, viewModel, card, onClick);
                existingChild.id = `${containerId}-card-${idx}`;
                existingChild.dataset.cardKey = cardKey;
                CardRenderer.renderCardBonuses(existingChild, card, true);
            } else {
                // New card — create DOM with enter animation
                const cardEl = CardRenderer.createCardDOM(viewModel, card, onClick);
                cardEl.id = `${containerId}-card-${idx}`;
                cardEl.dataset.cardKey = cardKey;
                cardEl.classList.add('card-enter');
                CardRenderer.renderCardBonuses(cardEl, card, true);
                if (childAtPos) el.replaceChild(cardEl, childAtPos);
                else el.appendChild(cardEl);
                // Kick off enter animation on next frame
                requestAnimationFrame(() => requestAnimationFrame(() => {
                    cardEl.classList.remove('card-enter');
                }));
            }
            // Play selection highlight — only on my-hand
            if (containerId === 'my-hand') {
                const cardEl2 = existingChild || el.querySelector(`[id="${containerId}-card-${idx}"]`);
                if (cardEl2) {
                    const isSel = window._playSel && window._playSel.cardIdx === idx;
                    cardEl2.classList.toggle('selected-for-play', isSel);
                }
            }
        }

        // Phase 3: Remove stale DOM elements (cards no longer in the list)
        for (let i = el.children.length - 1; i >= cardCount; i--) {
            const exiting = el.children[i];
            if (exiting && exiting.dataset.cardKey && !seenKeys.has(exiting.dataset.cardKey)) {
                exiting.classList.add('card-exit');
                setTimeout(() => { if (exiting.parentNode) exiting.remove(); }, 150);
            } else {
                el.removeChild(el.lastChild);
            }
        }
    },

    renderStage: (containerId, stage, underCards = [[], [], []], clickable, validActionMap = {}, hasGlobalSelection = false) => {
        const el = DOMUtils.getElement(containerId);
        if (!el) return;

        const existingAreas = Array.from(el.children);
        
        for (let i = 0; i < 3; i++) {
            const slot = stage[i];
            const under = underCards[i] || [];
            const action = validActionMap[i];
            const isValid = action !== undefined;
            const existingArea = existingAreas[i];

            let area, slotDiv;
            if (existingArea) {
                area = existingArea;
                slotDiv = area.querySelector('.member-slot');
            } else {
                area = document.createElement('div');
                area.className = 'member-area board-slot-container';
                slotDiv = document.createElement('div');
                const isTapped = slot && slot.orientation === 'Wait';
                slotDiv.className = `member-slot${slot && slot.card_no ? ' filled' : ''}${isTapped ? ' tapped' : ''}`;
                area.appendChild(slotDiv);
                el.appendChild(area);
            }

            // Render under-cards (energy or member cards stacked beneath)
            let underContainer = area.querySelector('.under-cards');
            if (under.length > 0) {
                if (!underContainer) {
                    underContainer = document.createElement('div');
                    underContainer.className = 'under-cards';
                    area.appendChild(underContainer);
                }
                // Sync under-card count
                while (underContainer.children.length < under.length) {
                    const uc = document.createElement('div');
                    const cardType = (under[underContainer.children.length].card_type || '').toLowerCase();
                    uc.className = 'under-card' + (cardType === 'energy' ? ' energy-type' : cardType === 'member' ? ' member-type' : '');
                    underContainer.appendChild(uc);
                }
                while (underContainer.children.length > under.length) {
                    underContainer.removeChild(underContainer.lastChild);
                }
                under.forEach((card, uIdx) => {
                    const ucEl = underContainer.children[uIdx];
                    const imgPath = resolveCardImagePath(card.card_no);
                    let img = ucEl.querySelector('img');
                    if (!img) {
                        img = document.createElement('img');
                        img.draggable = false;
                        ucEl.appendChild(img);
                    }
                    ImageLoader.loadImage(img, imgPath);
                    Tooltips.attachCardData(ucEl, card);
                });
                underContainer.style.display = '';
            } else {
                if (underContainer) {
                    underContainer.style.display = 'none';
                }
            }

            // Rust backend format: slot is { card_no, name, card_type, orientation }
            const isTapped = slot && slot.orientation === 'Wait';
            const filledClass = (slot && slot.card_no ? ' filled' : '');
            const tappedClass = isTapped ? ' tapped' : '';
            const validClass = isValid ? ' valid-target' : '';
            const hoverClass = (isValid && action?.index === State.hoveredActionId) ? ' hover-highlight' : '';

            const newClassName = `member-slot${filledClass}${tappedClass}${validClass}${hoverClass}`;
            if (slotDiv.className !== newClassName) slotDiv.className = newClassName;
            slotDiv.id = `${containerId}-slot-${i}`;

            if (slot && slot.card_no) {
                const fixedPath = resolveCardImagePath(slot.card_no);
                const existingImg = slotDiv.querySelector('img');
                if (existingImg) {
                    if (existingImg.src !== fixedPath) {
                        ImageLoader.loadImage(existingImg, fixedPath);
                    }
                } else {
                    const img = document.createElement('img');
                    img.draggable = false;
                    ImageLoader.loadImage(img, fixedPath);
                    slotDiv.innerHTML = '';
                    slotDiv.appendChild(img);
                }

                Tooltips.attachCardData(area, slot, isValid ? action.index : undefined);
                Tooltips.attachCardData(slotDiv, slot, isValid ? action.index : undefined);
                if (isValid) {
                    slotDiv.setAttribute('data-action-id', action.index);
                } else {
                    slotDiv.removeAttribute('data-action-id');
                }
                CardRenderer.renderCardBonuses(slotDiv, slot);
            } else {
                slotDiv.innerHTML = '';
                // Clear under-cards display when no member in slot
                if (underContainer) {
                    underContainer.style.display = 'none';
                }
                slotDiv.removeAttribute('data-action-id');
            }

            // Play selection: show cost on my-stage only
            const _areaNames = ['left', 'center', 'right'];
            let playTargetCost = null;
            let _playSelMatch = null;
            if (containerId === 'my-stage' && State.uiMode === 'play' && window._playSel) {
                _playSelMatch = window._playSel.actions.find(a => {
                    const p = a.parameters || {};
                    return p.stage_area === _areaNames[i] || p.available_areas?.some(av => av.area === _areaNames[i] && av.available);
                });
                if (_playSelMatch) {
                    const params = _playSelMatch.parameters || {};
                    const areaInfo = params.available_areas?.find(a => a.area === _areaNames[i]);
                    playTargetCost = areaInfo ? areaInfo.cost : (params.base_cost ?? 0);
                }
            }
            let costLabel = slotDiv.querySelector('.play-cost-label');
            if (playTargetCost !== null) {
                if (!costLabel) {
                    costLabel = document.createElement('div');
                    costLabel.className = 'play-cost-label';
                    slotDiv.appendChild(costLabel);
                }
                costLabel.textContent = playTargetCost;
                slotDiv.classList.add('play-target');
            } else {
                if (costLabel) costLabel.remove();
                slotDiv.classList.remove('play-target');
            }

            // Attach play-target click handler when _playSel matches this area
            if (_playSelMatch) {
                const playTargetClick = (e) => {
                    e.stopPropagation();
                    const zoneArea = _areaNames[i];
                    if (window.doAction) {
                        const actionCopy = JSON.parse(JSON.stringify(_playSelMatch));
                        actionCopy.parameters = actionCopy.parameters || {};
                        actionCopy.parameters.stage_area = zoneArea;
                        window.doAction(actionCopy);
                        window._playSel = null;
                        if (window.render) window.render();
                    }
                };
                area.onclick = playTargetClick;
                slotDiv.onclick = playTargetClick;
                area.style.cursor = 'pointer';
                slotDiv.style.cursor = 'pointer';
            } else if (clickable && (isValid || !hasGlobalSelection)) {
                const clickHandler = () => {
                    const _uiMode = State.uiMode;
                    const slotCard = slot || null;

                    if (_uiMode === 'view' && slotCard && slotCard.card_no) {
                        const m = window.__modals?.CardDetailModal;
                        if (m) m.open(slotCard, containerId);
                        return;
                    }

                    if (_uiMode === 'play' && slotCard && slotCard.card_no) {
                        const legalActions = State.data?.legal_actions || [];
                        const abilityActions = legalActions.filter(a => {
                            if (a.action_type !== 'use_ability') return false;
                            const params = a.parameters || {};
                            if (params.stage_area !== undefined) {
                                const areaNames = ['left', 'center', 'right'];
                                return params.stage_area === areaNames[i] || params.stage_area === areaNames[i] + '_side';
                            }
                            return params.card_id === slotCard.id || params.card_no === slotCard.card_no;
                        });
                        if (abilityActions.length > 0) {
                            const m = window.__modals?.StageAbilityModal;
                            if (m) { m.open(slotCard, abilityActions); return; }
                        }
                        const m = window.__modals?.CardDetailModal;
                        if (m) { m.open(slotCard, containerId); return; }
                    }

                    if (isValid) {
                        if (window.selectedAction && window.selectedAction.card_index !== undefined) {
                            if (action.card_index === window.selectedAction.card_index && window.doAction) {
                                window.doAction(action);
                                window.selectedAction = null;
                                document.querySelectorAll('.card.selected').forEach(c => c.classList.remove('selected'));
                            }
                        } else if (window.doAction) {
                            window.doAction(action);
                        }
                    } else if (_uiMode === 'view' && slotCard && slotCard.card_no) {
                        // Already handled above
                    } else if (window.onStageSlotClick) {
                        window.onStageSlotClick(i);
                    }
                };
                area.onclick = clickHandler;
                slotDiv.onclick = clickHandler;
                area.style.cursor = 'pointer';

                if (isValid && action?.index != null) {
                    const idx = action.index;
                    area.onmouseenter = () => {
                        if (window.highlightActionBtn) window.highlightActionBtn(idx, true);
                    };
                    area.onmouseleave = () => {
                        if (window.highlightActionBtn) window.highlightActionBtn(idx, false);
                    };
                } else {
                    area.onmouseenter = null;
                    area.onmouseleave = null;
                }
            } else {
                area.onclick = null;
                slotDiv.onclick = null;
                area.style.cursor = '';
                area.onmouseenter = null;
                area.onmouseleave = null;
            }
        }
    },

    renderLiveZone: (containerId, liveCards, visible, validActionMap = {}, hasGlobalSelection = false) => {
        const state = State.data;
        const el = DOMUtils.getElement(containerId);
        if (!el) return;

        // Hide opponent's live zone cards during set phase (not revealed until performance)
        const phase = state?.phase;
        const isSetPhase = phase === 'LiveCardSetFirstAttacker' || phase === 'LiveCardSetSecondAttacker';
        const isOpponentZone = containerId === 'opp-live';
        const shouldHideCards = isSetPhase && isOpponentZone;

        const existingSlots = Array.from(el.children);

        for (let i = 0; i < 3; i++) {
            let card = liveCards[i];
            const hideForOpponent = shouldHideCards && card;
            const isMemberInLiveZone = card && card.card_no && !CardRenderer.isCardLive(card);
            if (hideForOpponent || isMemberInLiveZone) {
                card = { card_no: -1, hidden: true, card_type: 'live' };
            }
            const action = validActionMap[i];
            const isValid = action != null;
            const validClass = isValid ? ' valid-target' : '';
            const existingSlot = existingSlots[i];

            let slot;
            if (existingSlot) {
                slot = existingSlot;
            } else {
                slot = document.createElement('div');
                el.appendChild(slot);
            }

            const viewModel = CardRenderer.getCardViewModel(card, {
                isValid,
                containerId,
                actionId: action?.index
            });
            const isCardHidden = viewModel?.isHidden;
            
            let newClassName = viewModel ? viewModel.classes : (`card empty orientation-landscape${validClass}`);
            if (isValid && action?.index === State.hoveredActionId) {
                newClassName += ' hover-highlight';
            }
            if (slot.className !== newClassName) slot.className = newClassName;
            slot.id = `${containerId}-slot-${i}`;

            if (card && card.card_no) {
                const fixedPath = viewModel?.imgPath || resolveCardImagePath(card.card_no);
                const existingImg = slot.querySelector('img');
                const existingInner = slot.querySelector('.live-card-inner');

                if (existingInner && existingImg) {
                    if (existingImg.src !== fixedPath) {
                        ImageLoader.loadImage(existingImg, fixedPath);
                    }
                } else {
                    const img = document.createElement('img');
                    img.draggable = false;
                    ImageLoader.loadImage(img, fixedPath);

                    const inner = document.createElement('div');
                    inner.className = 'live-card-inner';
                    inner.appendChild(img);

                    slot.innerHTML = '';
                    slot.appendChild(inner);
                }
                
                Tooltips.attachCardData(slot, card, isValid && action?.index != null ? action.index : undefined);
                if (!isValid) slot.removeAttribute('data-action-id');

                if (!isCardHidden) {
                    CardRenderer.renderCardBonuses(slot, card, true);
                }

                if (isValid || State.uiMode === 'view') {
                    slot.style.cursor = 'pointer';
                    slot.onclick = () => {
                        const _uiMode = State.uiMode;
                        if (_uiMode === 'view' && card && card.card_no && card.card_no > 0) {
                            const m = window.__modals?.CardDetailModal;
                            if (m) m.open(card, containerId);
                            return;
                        }
                        if (_uiMode === 'play') {
                            const legalActions = State.data?.legal_actions || [];
                            const abilityActions = legalActions.filter(a => {
                                if (a.action_type !== 'use_ability') return false;
                                const params = a.parameters || {};
                                return params.card_indices?.includes(i) || params.card_id === card?.id;
                            });
                            if (abilityActions.length > 0 && card && card.card_no) {
                                const m = window.__modals?.StageAbilityModal;
                                if (m) { m.open(card, abilityActions); return; }
                            }
                        }
                        if (window.doAction && isValid) window.doAction(action);
                    };
                    
                    const idx = action?.index;
                    slot.onmouseenter = () => {
                        if (window.highlightActionBtn && idx != null) window.highlightActionBtn(idx, true);
                    };
                    slot.onmouseleave = () => {
                        if (window.highlightActionBtn && idx != null) window.highlightActionBtn(idx, false);
                    };
                } else {
                    slot.onclick = null;
                    slot.style.cursor = '';
                    slot.onmouseenter = null;
                    slot.onmouseleave = null;
                }
            } else {
                slot.innerHTML = '';
                slot.onclick = null;
                slot.style.cursor = '';
            }
        }
    },

    renderDiscardPile: (containerId, discard, playerIdx, validActionMap = {}, hasGlobalSelection = false, showModalCallback = null) => {
        const el = DOMUtils.getElement(containerId);
        if (!el) return;

        const action = validActionMap && validActionMap['all'];
        const isValid = action !== undefined;
        const hoverClass = (isValid && action?.index === State.hoveredActionId) ? ' hover-highlight' : '';
        el.className = 'discard-pile-visual ' + (isValid ? 'valid-target' : '') + hoverClass;

        DOMUtils.clear(containerId);

        if (!discard || discard.length === 0) {
            el.classList.add('empty');
            DOMUtils.setHTML(containerId, `<span style="opacity:0.3; font-size:0.8rem;">${i18n.t('discard_pile')}</span>`);
        } else {
            const showCount = Math.min(3, discard.length);
            for (let i = 0; i < showCount; i++) {
                const card = discard[discard.length - 1 - i];
                const vm = CardRenderer.getCardViewModel(card, { mini: true });
                const div = CardRenderer.createCardDOM(vm, card);
                div.style.transform = `translate(${i * 2}px, ${i * 2}px)`;
                div.style.zIndex = 10 - i;
                el.appendChild(div);
            }
        }

        if (isValid || (!hasGlobalSelection && discard && discard.length > 0)) {
            el.style.cursor = 'pointer';
            el.onclick = (e) => {
                e.stopPropagation();
                if (isValid && window.doAction) {
                    window.doAction(action);
                } else if (!isValid && showModalCallback) {
                    showModalCallback(playerIdx);
                }
            };
            if (isValid) {
                el.onmouseenter = () => {
                    if (window.highlightActionBtn) window.highlightActionBtn(action.index, true);
                };
                el.onmouseleave = () => {
                    if (window.highlightActionBtn) window.highlightActionBtn(action.index, false);
                };
            }
        } else {
            el.onclick = null;
        }
    },

    renderLookedCards: (validActionMap = {}, overrideCards = null, overrideTitle = null) => {
        const state = State.data;
        const panel = DOMUtils.getElement(DOM_IDS.LOOKED_CARDS_PANEL);
        const content = DOMUtils.getElement(DOM_IDS.LOOKED_CARDS_CONTENT);
        if (!panel || !content) return;

        // Include pending choice selection cards here (modal is hidden, so show them in sidebar panel)
        const pendingSelectionCards = state.pending_choice?.selection_cards || [];
        let cards = overrideCards || (pendingSelectionCards.length > 0 ? pendingSelectionCards : (state.looked_cards?.cards || state.looked_cards || []));

        // Show ALL looked-at/revealed cards. Non-selectable cards are rendered
        // grayed out below (via isClickable=false), not hidden.
        // The legal_actions determine which cards are valid picks;
        // cards without a matching action are shown as non-interactive.

        if (cards.length === 0) {
            DOMUtils.setVisible(DOM_IDS.LOOKED_CARDS_PANEL, false);
            return;
        }
        DOMUtils.setVisible(DOM_IDS.LOOKED_CARDS_PANEL, true, 'flex');

        let headerHtml = "";
        if (overrideTitle) {
            headerHtml = `<div class="looked-cards-header">${overrideTitle}</div>`;
        } else if (state.pending_choice && (state.pending_choice.title || state.pending_choice.text)) {
            const title = state.pending_choice.title || state.pending_choice.text;
            headerHtml = `<div class="looked-cards-header">${Tooltips.enrichAbilityText(title)}</div>`;
        }

        if (state.pending_choice && state.pending_choice.choose_count > 1) {
            const total = state.pending_choice.choose_count;
            const v_rem = state.pending_choice.v_remaining;
            const remaining = (v_rem === -1) ? total : (v_rem + 1);
            const label = remaining > 1 ? i18n.t('pick_more', { count: remaining }) : i18n.t('pick_last');
            headerHtml += `<div class="looked-cards-subtitle">${label}</div>`;
        }

        DOMUtils.clear(DOM_IDS.LOOKED_CARDS_CONTENT);
        if (headerHtml) {
            const headerDiv = document.createElement('div');
            headerDiv.className = 'looked-cards-meta';
            headerDiv.innerHTML = headerHtml;
            content.appendChild(headerDiv);
        }

        cards.forEach((c, idx) => {
            if (c === null) {
                const placeholder = document.createElement('div');
                placeholder.className = 'looked-card-item placeholder';
                placeholder.style.visibility = 'hidden';
                content.appendChild(placeholder);
                return;
            }

            // Match by card_id against legal_actions, not by array index
            let action = validActionMap[idx];
            if (!action && state.legal_actions) {
                const cardId = c.id !== undefined ? c.id : c.card_id;
                action = state.legal_actions.find(a => {
                    const params = a.parameters || {};
                    return params.card_id === cardId || params.card_id === c.card_id;
                });
            }
            const isClickable = (action !== undefined && action !== null);

            const viewModel = CardRenderer.getCardViewModel(c, {
                mini: true,
                isValid: isClickable,
                actionId: action?.index,
                containerId: DOM_IDS.LOOKED_CARDS_CONTENT
            });

            const onClick = isClickable ? () => {
                if (window.doAction) window.doAction(action);
            } : null;

            const cardEl = CardRenderer.createCardDOM(viewModel, c, onClick);
            
            // Explicitly set class and ID for the item
            cardEl.classList.add('looked-card-item');
            cardEl.id = `looked-card-${idx}`;
            CardRenderer.renderCardBonuses(cardEl, c, true);
            
            content.appendChild(cardEl);
        });
    },

    // ════════════════════════════════════════════════════════════════
    // renderCardBonuses — renders texticon status badges on a card
    //
    // This shows runtime modifiers as small texticon badges overlaying
    // the card image.  The badges come from 3 sources in CardDisplay:
    //
    //   1. bonus_* (additive, with +/-):
    //      - ModifyBlade, GainResource(blade)          → +N icon_blade.png
    //      - ModifyHeart, GainResource(heart)           → +N heart_X.png
    //      - ModifyScore, GainAbility(score)             → +N icon_score.png
    //      - ModifyCost                                  → +N icon_energy.png
    //
    //   2. set_* (absolute/override, no +/-):
    //      - SetBladeCount, SetBladeType                 → N icon_blade.png
    //      - SetHeartType                                 → N heart_X.png
    //      - SetScore                                     → N icon_score.png
    //      - SetCost, SetCostToUse                        → N icon_energy.png
    //
    //   3. bonus_triggers (no number, just icon):
    //      - GainAbility(constant)                       → jyouji.png
    //      - GainAbility(live_success)                   → live_success.png
    //      - GainAbility(live_start)                     → live_start.png
    //      - GainAbility(debut)                          → toujyou.png
    //
    // One-shot effects (Draw, Discard, Move, etc.) produce NO badges.
    // Energy-under-member is visible as stacked cards (not a badge).
    // Restrictions are tracked in prohibition_effects (no badge).
    // ════════════════════════════════════════════════════════════════
    renderCardBonuses: (slotEl, card, overlay = false) => {
        if (!card) return;
        let existing = slotEl.querySelector('.card-bonuses');
        if (existing) existing.remove();

        const bonuses = [];

        // Additive bonuses — shown with +/- prefix (e.g. "+2 Blade")
        if (card.bonus_blade && card.bonus_blade !== 0) {
            bonuses.push({ type: 'bonus-blade', value: card.bonus_blade, icon: 'icon_blade.png', isAdditive: true });
        }

        const heartIcons = ['heart_00.png','heart_01.png','heart_02.png','heart_03.png','heart_04.png','heart_05.png','heart_06.png','icon_all.png'];
        if (card.bonus_hearts && Array.isArray(card.bonus_hearts)) {
            card.bonus_hearts.forEach((val, idx) => {
                if (val && val !== 0 && idx < heartIcons.length) {
                    bonuses.push({ type: 'bonus-heart', value: val, icon: heartIcons[idx], isAdditive: true });
                }
            });
        }

        if (card.bonus_score && card.bonus_score !== 0) {
            bonuses.push({ type: 'bonus-score', value: card.bonus_score, icon: 'icon_score.png', isAdditive: true });
        }

        if (card.bonus_cost && card.bonus_cost !== 0) {
            bonuses.push({ type: 'bonus-cost', value: card.bonus_cost, icon: 'icon_energy.png', isAdditive: true });
        }

        // Set/override values from Set* actions — shown without +/- (e.g. "5 Blade" not "+5")
        // SetBladeCount → set_blade shows blade icon with plain number
        if (card.set_blade && card.set_blade !== 0) {
            bonuses.push({ type: 'set-blade', value: card.set_blade, icon: 'icon_blade.png', isAdditive: false });
        }
        // SetHeartType → set_hearts shows heart icons with plain number
        if (card.set_hearts && Array.isArray(card.set_hearts)) {
            card.set_hearts.forEach((val, idx) => {
                if (val && val !== 0 && idx < heartIcons.length) {
                    bonuses.push({ type: 'set-heart', value: val, icon: heartIcons[idx], isAdditive: false });
                }
            });
        }
        // SetScore → set_score shows score icon with plain number
        if (card.set_score && card.set_score !== 0) {
            bonuses.push({ type: 'set-score', value: card.set_score, icon: 'icon_score.png', isAdditive: false });
        }
        // SetCost → set_cost shows energy icon with plain number
        if (card.set_cost && card.set_cost !== 0) {
            bonuses.push({ type: 'set-cost', value: card.set_cost, icon: 'icon_energy.png', isAdditive: false });
        }

        // Trigger texticon badges from gained abilities (e.g. jyouji.png, live_success.png)
        // gain_ability populates bonus_triggers with the trigger type of the gained ability.
        if (card.bonus_triggers && Array.isArray(card.bonus_triggers)) {
            const triggerIconMap = {
                'jyouji': 'jyouji.png',
                'live_success': 'live_success.png',
                'live_start': 'live_start.png',
                'toujyou': 'toujyou.png',
                'kidou': 'kidou.png',
                'jidou': 'jidou.png',
            };
            card.bonus_triggers.forEach(trigger => {
                const icon = triggerIconMap[trigger];
                if (icon) {
                    bonuses.push({ type: 'bonus-trigger', value: null, icon: icon, isAdditive: false });
                }
            });
        }

        if (card.heart_transform) {
            bonuses.push({ type: 'bonus-transform', value: null, icon: card.heart_transform.replace('heart', 'heart_') + '.png', isAdditive: false });
        }

        if (bonuses.length === 0) return;

        const container = document.createElement('div');
        container.className = 'card-bonuses' + (overlay ? ' overlay' : '');

        bonuses.forEach(b => {
            const badge = document.createElement('div');
            badge.className = `bonus-badge ${b.type}`;
            if (b.value !== null) {
                const valSpan = document.createElement('span');
                valSpan.className = 'bonus-value';
                // Additive = +/- prefix, Set = plain number
                valSpan.textContent = b.isAdditive && b.value > 0 ? `+${b.value}` : `${b.value}`;
                badge.appendChild(valSpan);
            }
            if (b.icon) {
                const img = document.createElement('img');
                img.src = fixImgPath('img/texticon/' + b.icon);
                img.alt = '';
                badge.appendChild(img);
            }
            if (b.type === 'bonus-transform') {
                badge.title = `Heart transform → ${card.heart_transform}`;
            } else if (b.value !== null) {
                badge.title = `${b.type.replace('bonus-', '').replace('set-', '')} ${b.isAdditive && b.value > 0 ? '+' : ''}${b.value}`;
            }
            container.appendChild(badge);
        });

        slotEl.appendChild(container);
    }
};
