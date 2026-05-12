import { State } from '../state.js';
import { Phase, fixImg as fixImgPath, isMulliganPhase } from '../constants.js';
import * as i18n from '../i18n/index.js';
import { Tooltips } from '../ui_tooltips.js';
import { DOMUtils } from '../utils/DOMUtils.js';
import { DOM_IDS } from '../constants_dom.js';

// Image loading with retry logic and error handling
export const ImageLoader = {
    loadedImages: new Set(),
    failedImages: new Map(), // src -> retry count
    maxRetries: 2,
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

    _doLoad(img, src) {
        // Clear previous handlers to avoid duplicates
        img.onload = null;
        img.onerror = null;

        img.onload = () => {
            this.loadedImages.add(src);
            this.failedImages.delete(src);
            img.style.opacity = '1';
        };

        img.onerror = () => {
            const retries = this.failedImages.get(src) || 0;
            if (retries < this.maxRetries) {
                this.failedImages.set(src, retries + 1);
                // Retry with a small delay to avoid flooding
                setTimeout(() => this._doLoad(img, src), 100 * (retries + 1));
            } else {
                console.warn('[ImageLoader] Failed to load image after retries:', src);
                img.style.opacity = '0.3';
            }
        };

        // Force reload by adding cache-busting on retry
        const loadSrc = (this.failedImages.get(src) || 0) > 0
            ? src + (src.includes('?') ? '&' : '?') + '_retry=' + Date.now()
            : src;
        img.src = loadSrc;
    },

    loadImage(img, src) {
        if (!src) return;
        this.init();

        // If already successfully loaded, just set it
        if (this.loadedImages.has(src)) {
            img.src = src;
            img.style.opacity = '1';
            return;
        }

        // For immediate load, set src directly
        img.dataset.src = src;
        this._doLoad(img, src);

        // Also observe for lazy loading (in case img is off-screen)
        if (this.observer) {
            this.observer.observe(img);
        }
    }
};

// Consistent image path resolution across all card displays
export function resolveCardImagePath(cardNo) {
    if (!cardNo) return '';
    
    // 1. Direct mapping lookup
    const mapped = State.cardImageMapping?.[cardNo];
    if (mapped) return fixImgPath(mapped);
    
    // 2. Try ＋/+, → 2 variant in mapping (webp uses PR2 not PR＋)
    if (cardNo.includes('＋') || cardNo.endsWith('+')) {
        const with2 = cardNo.replace(/＋/g, '2').replace(/\+$/, '2');
        const mapped2 = State.cardImageMapping?.[with2];
        if (mapped2) return fixImgPath(mapped2);
    }
    
    // 3. Try compressed name in mapping (PL!HS-PR-017-PR → PL!HS-017-PR)
    const stripped = cardNo.replace(/^(PL![\w]*)-[A-Z]+-(\d*)-/, '$1-$2-');
    if (stripped !== cardNo) {
        const compressedMapped = State.cardImageMapping?.[stripped];
        if (compressedMapped) return fixImgPath(compressedMapped);
    }
    
    // 4. Compressed ＋/+, → 2
    if (cardNo.includes('＋')) {
        const normalized = cardNo.replace(/＋/g, '2');
        const stripped2 = normalized.replace(/^(PL![\w]*)-[A-Z]+-(\d*)-/, '$1-$2-');
        if (stripped2 !== normalized) {
            const mapped2 = State.cardImageMapping?.[stripped2];
            if (mapped2) return fixImgPath(mapped2);
        }
    }
    
    // 5. Fallback: try direct webp path (also try ＋→2 variant)
    if (cardNo.includes('＋')) {
        const with2 = cardNo.replace(/＋/g, '2');
        return fixImgPath(`img/cards_webp/${with2}.webp`);
    }
    return fixImgPath(`img/cards_webp/${cardNo}.webp`);
}

export const CardRenderer = {
    /**
     * Maps engine card data to UI-specific properties (CSS classes, labels, etc.)
     */
    getCardViewModel: (card, options = {}) => {
        if (!card) return null;

        const state = State.data;
        const { isSelected, isValid, mini, containerId } = options;

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
        // Support both hidden field and card_no === -2/-1 for hidden cards
        const isHidden = resolvedCard.hidden || resolvedCard.is_hidden || resolvedCard.card_no === -2 || resolvedCard.card_no === -1;
        // Engine sends card_type as string enum
        const isLive = resolvedCard.card_type === 'Live' || resolvedCard.card_type === 'ライブ';

        // 1. Determine CSS Classes
        const classNames = ['card'];
        if (isHidden) classNames.push('hidden');
        if (mini) classNames.push('card-mini');
        if (resolvedCard.is_new) classNames.push('new-card');
        if (isLive) classNames.push('type-live');

        // Orientation Logic (Consolidated Matrix)
        const targetLandscape = isLive || (containerId && (
            containerId.includes('live') ||
            containerId.includes('success') ||
            containerId.includes('selection')
        ));
        const nativeLandscape = isLive;

        if (targetLandscape) {
            classNames.push('orientation-landscape');
        }

        // Image rotation is needed if native orientation doesn't match target orientation
        if (targetLandscape !== nativeLandscape) {
            classNames.push('rotate-img-90');
        }

        if (isSelected) {
            const isMulligan = isMulliganPhase(state.phase);
            classNames.push(isMulligan ? 'mulligan-selected' : 'selected');
        }
        if (isValid && containerId !== 'my-hand') classNames.push('valid-target');

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

    /**
     * Creates a single card DOM element from a ViewModel
     */
    createCardDOM: (viewModel, cardData, onClick = null) => {
        const div = document.createElement('div');
        div.className = viewModel.classes;

        if (viewModel.actionId !== undefined || cardData.card_no !== undefined) {
            Tooltips.attachCardData(div, cardData, viewModel.actionId);
        }

        if (!viewModel.isHidden) {
            if (viewModel.imgPath) {
                const img = document.createElement('img');
                img.draggable = false;
                ImageLoader.loadImage(img, viewModel.imgPath);
                div.appendChild(img);
            }
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

        if (viewModel.isHidden) {
            el.innerHTML = '';
            el.classList.add('card-back');
        } else {
            const imgPath = viewModel.imgPath;
            const existingImg = el.querySelector('img');
            
            if (existingImg) {
                if (imgPath && existingImg.dataset.originalPath !== imgPath) {
                    ImageLoader.loadImage(existingImg, imgPath);
                    existingImg.dataset.originalPath = imgPath;
                    existingImg.style.display = '';
                } else if (!imgPath) {
                    existingImg.remove();
                }
            } else if (imgPath) {
                const img = document.createElement('img');
                img.draggable = false;
                img.dataset.originalPath = imgPath;
                ImageLoader.loadImage(img, imgPath);
                el.prepend(img);
            }


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

    renderCards: (containerId, cards, clickable = false, mini = false, selectedIndices = [], validActionMap = {}, hasGlobalSelection = false, filter = null) => {
        const el = DOMUtils.getElement(containerId);
        if (!el) return;
        if (!cards) {
            DOMUtils.clear(containerId);
            return;
        }

        const existingChildren = Array.from(el.children);
        const cardCount = cards.length;

        if (filter) {
            DOMUtils.clear(containerId);
        } else {
            // Synchronize children count
            while (el.children.length > cardCount) {
                el.removeChild(el.lastChild);
            }
        }

        cards.forEach((card, idx) => {
            if (filter && !filter(card, idx)) return;

            const isSelected = selectedIndices.includes(idx);
            const action = validActionMap[idx];
            const isValid = action !== undefined;
            const existingChild = filter ? null : existingChildren[idx];

            if (card === null) {
                if (existingChild && existingChild.classList.contains('placeholder')) {
                    existingChild.style.visibility = 'hidden';
                } else {
                    const placeholder = document.createElement('div');
                    placeholder.className = 'card placeholder' + (mini ? ' card-mini' : '');
                    placeholder.style.visibility = 'hidden';
                    if (existingChild) el.replaceChild(placeholder, existingChild);
                    else el.appendChild(placeholder);
                }
                return;
            }

            const viewModel = CardRenderer.getCardViewModel(card, {
                isSelected,
                isValid,
                mini,
                containerId,
                actionId: action?.index
            });

            const onClick = clickable && (isValid || !hasGlobalSelection) ? (act) => {
                if (isValid && window.doAction) {
                    window.doAction(action);
                } else if (window.playCard) {
                    window.playCard(idx);
                }
            } : null;

            if (existingChild && !existingChild.classList.contains('placeholder')) {
                CardRenderer.updateCardDOM(existingChild, viewModel, card, onClick);
                existingChild.id = `${containerId}-card-${idx}`;
            } else {
                const cardEl = CardRenderer.createCardDOM(viewModel, card, onClick);
                cardEl.id = `${containerId}-card-${idx}`;
                if (existingChild) el.replaceChild(cardEl, existingChild);
                else el.appendChild(cardEl);
            }
        });
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
                const existingUnder = Array.from(underContainer.children);
                while (underContainer.children.length < under.length) {
                    const uc = document.createElement('div');
                    uc.className = 'under-card';
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

                Tooltips.attachCardData(area, slot, isValid ? action : undefined);
                Tooltips.attachCardData(slotDiv, slot, isValid ? action : undefined);
                if (isValid) {
                    area.setAttribute('data-action-id', action.index);
                    slotDiv.setAttribute('data-action-id', action.index);
                } else {
                    area.removeAttribute('data-action-id');
                    slotDiv.removeAttribute('data-action-id');
                }
            } else {
                slotDiv.innerHTML = '';
                // Clear under-cards display when no member in slot
                if (underContainer) {
                    underContainer.style.display = 'none';
                }
                area.removeAttribute('data-action-id');
                slotDiv.removeAttribute('data-action-id');
            }

            if (clickable && (isValid || !hasGlobalSelection)) {
                const clickHandler = () => {
                    if (isValid && window.doAction) {
                        window.doAction(action);
                    } else if (window.onStageSlotClick) {
                        window.onStageSlotClick(i);
                    }
                };
                area.onclick = clickHandler;
                slotDiv.onclick = clickHandler;
                area.style.cursor = 'pointer';

                if (isValid) {
                    area.onmouseenter = () => {
                        if (window.highlightActionBtn) window.highlightActionBtn(action.index, true);
                    };
                    area.onmouseleave = () => {
                        if (window.highlightActionBtn) window.highlightActionBtn(action.index, false);
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

        const existingSlots = Array.from(el.children);

        for (let i = 0; i < 3; i++) {
            const card = liveCards[i];
            const action = validActionMap[i];
            const isValid = action !== undefined;
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
                
                const rawText = Tooltips.getEffectiveRawText(card);
                if (rawText) DOMUtils.patchAttributes(slot, { 'data-text': rawText });
                DOMUtils.patchAttributes(slot, { 'data-card-id': card.card_no });
                if (isValid) slot.setAttribute('data-action-id', action.index);
                else slot.removeAttribute('data-action-id');

                if (isValid) {
                    slot.style.cursor = 'pointer';
                    slot.onclick = () => { if (window.doAction) window.doAction(action); };
                    
                    slot.onmouseenter = () => {
                        if (window.highlightActionBtn) window.highlightActionBtn(action.index, true);
                    };
                    slot.onmouseleave = () => {
                        if (window.highlightActionBtn) window.highlightActionBtn(action.index, false);
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
                const div = document.createElement('div');
                div.className = 'card card-mini';
                const img = document.createElement('img');
                img.draggable = false;
                ImageLoader.loadImage(img, resolveCardImagePath(card.card_no));
                div.appendChild(img);
                div.style.transform = `translate(${i * 2}px, ${i * 2}px)`;
                div.style.zIndex = 10 - i;

                if (card.card_no !== undefined) {
                    div.setAttribute('data-card-id', card.card_no);
                    const rawText = Tooltips.getEffectiveRawText(card);
                    if (rawText) div.setAttribute('data-text', rawText);
                }
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

        const pendingSelectionCards = state.pending_choice?.selection_cards || [];
        const cards = overrideCards || (pendingSelectionCards.length > 0 ? pendingSelectionCards : (state.looked_cards || []));
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
            headerHtml = `<div class="looked-cards-header">${title}</div>`;
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

            const action = validActionMap[idx];
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
            
            content.appendChild(cardEl);
        });
    }
};
