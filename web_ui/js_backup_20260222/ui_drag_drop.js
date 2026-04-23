/**
 * UI Drag and Drop Module
 * Handles drag events for playing cards and arranging the board.
 */
import { State } from './state.js';

export const DragDrop = {
    draggedCardIdx: -1,
    draggedSource: null,
    dragClone: null,
    dragIndex: null,
    dragCardId: null,
    dragOriginalElement: null,
    dragStartX: 0,
    dragStartY: 0,
    isDraggingActive: false,
    DRAG_THRESHOLD: 5,

    init: () => {
        // Native DnD listeners (Fallback/Simple)
        document.addEventListener('dragstart', DragDrop.handleNativeDragStart);
        document.addEventListener('dragend', DragDrop.handleNativeDragEnd);
        document.addEventListener('dragover', DragDrop.handleNativeDragOver);
        document.addEventListener('dragleave', DragDrop.handleNativeDragLeave);
        document.addEventListener('drop', DragDrop.handleNativeDrop);
    },

    // --- Custom Drag System (Visual Follower) ---

    customDragStart: (e, source, index, cardId, element) => {
        DragDrop.draggedSource = source;
        DragDrop.dragIndex = index;
        DragDrop.dragCardId = cardId;
        DragDrop.dragOriginalElement = element;
        DragDrop.dragStartX = e.clientX;
        DragDrop.dragStartY = e.clientY;
        DragDrop.isDraggingActive = false;

        document.addEventListener('mousemove', DragDrop.onCustomDragMove);
        document.addEventListener('mouseup', DragDrop.onCustomDragEnd);
    },

    onCustomDragMove: (e) => {
        if (!DragDrop.isDraggingActive) {
            const dist = Math.sqrt(Math.pow(e.clientX - DragDrop.dragStartX, 2) + Math.pow(e.clientY - DragDrop.dragStartY, 2));
            if (dist > DragDrop.DRAG_THRESHOLD) {
                DragDrop.startActualDrag(e);
            }
            return;
        }

        DragDrop.moveDragClone(e);

        if (DragDrop.dragClone) {
            DragDrop.dragClone.style.display = 'none';
            const target = document.elementFromPoint(e.clientX, e.clientY);
            DragDrop.dragClone.style.display = 'block';

            document.querySelectorAll('.drop-hover').forEach(el => el.classList.remove('drop-hover'));
            if (target) {
                const dropTarget = target.closest('.valid-drop-target');
                if (dropTarget) dropTarget.classList.add('drop-hover');
            }
        }
    },

    startActualDrag: (e) => {
        DragDrop.isDraggingActive = true;
        DragDrop.dragClone = DragDrop.createDragClone(DragDrop.dragOriginalElement);
        DragDrop.moveDragClone(e);
        DragDrop.dragOriginalElement.classList.add('dragging-source');

        if (window.Tooltips && window.Tooltips.highlightValidZones) {
            window.Tooltips.highlightValidZones(DragDrop.draggedSource, DragDrop.dragIndex);
        }
    },

    onCustomDragEnd: (e) => {
        document.removeEventListener('mousemove', DragDrop.onCustomDragMove);
        document.removeEventListener('mouseup', DragDrop.onCustomDragEnd);

        if (!DragDrop.isDraggingActive) {
            DragDrop.resetDragState();
            return;
        }

        if (DragDrop.dragClone) {
            DragDrop.dragClone.style.display = 'none';
            const target = document.elementFromPoint(e.clientX, e.clientY);
            DragDrop.dragClone.style.display = 'block';

            if (target && DragDrop.draggedSource !== null && DragDrop.dragIndex !== null) {
                const dropTarget = target.closest('.valid-drop-target');
                if (dropTarget) {
                    const targetId = dropTarget.id;
                    let targetZone = null;
                    let targetIndex = -1;

                    if (targetId.startsWith('my-stage-slot-')) {
                        targetZone = 'stage';
                        targetIndex = parseInt(targetId.replace('my-stage-slot-', ''));
                    } else if (targetId.startsWith('my-live-slot-')) {
                        targetZone = 'live';
                        targetIndex = parseInt(targetId.replace('my-live-slot-', ''));
                    } else if (targetId.startsWith('opp-stage-slot-')) {
                        targetZone = 'opp-stage';
                        targetIndex = parseInt(targetId.replace('opp-stage-slot-', ''));
                    } else if (targetId === 'my-discard') {
                        targetZone = 'discard';
                    } else if (targetId === 'my-hand') {
                        targetZone = 'hand';
                    }

                    if (targetZone) {
                        DragDrop.executeDrop(DragDrop.draggedSource, DragDrop.dragIndex, targetZone, targetIndex);
                    }
                }
            }
        }
        DragDrop.resetDragState();
    },

    createDragClone: (originalElement) => {
        const clone = originalElement.cloneNode(true);
        clone.id = 'drag-clone';
        clone.style.position = 'fixed';
        clone.style.pointerEvents = 'none';
        clone.style.zIndex = '99999';
        clone.style.width = originalElement.offsetWidth + 'px';
        clone.style.height = originalElement.offsetHeight + 'px';
        clone.style.transform = 'scale(1.1) rotate(3deg)';
        clone.style.boxShadow = '0 15px 40px rgba(0,0,0,0.5)';
        clone.style.border = '3px solid var(--accent-gold)';
        clone.style.opacity = '1';
        clone.style.transition = 'none';
        document.body.appendChild(clone);
        return clone;
    },

    moveDragClone: (e) => {
        if (DragDrop.dragClone) {
            DragDrop.dragClone.style.left = (e.clientX - DragDrop.dragClone.offsetWidth / 2) + 'px';
            DragDrop.dragClone.style.top = (e.clientY - DragDrop.dragClone.offsetHeight / 2) + 'px';
        }
    },

    resetDragState: () => {
        if (DragDrop.dragClone) DragDrop.dragClone.remove();
        DragDrop.dragClone = null;
        if (DragDrop.dragOriginalElement) DragDrop.dragOriginalElement.classList.remove('dragging-source');
        DragDrop.dragOriginalElement = null;
        if (window.Tooltips && window.Tooltips.clearHighlights) window.Tooltips.clearHighlights();
        DragDrop.draggedSource = null;
        DragDrop.dragIndex = null;
        DragDrop.dragCardId = null;
        DragDrop.isDraggingActive = false;
    },

    // --- Legacy/Native DnD Handlers ---

    handleNativeDragStart: (e) => {
        if (e.target.classList.contains('card')) {
            const source = e.target.closest('#my-hand') ? 'hand' : e.target.closest('#my-stage') ? 'stage' : null;
            if (!source) return;

            const idParts = e.target.id.split('-');
            const index = parseInt(idParts[idParts.length - 1]);

            e.dataTransfer.setData("source", source);
            e.dataTransfer.setData("index", index);
            e.dataTransfer.effectAllowed = "move";
            e.target.classList.add('dragging');

            if (window.Tooltips && window.Tooltips.highlightValidZones) {
                window.Tooltips.highlightValidZones(source, index);
            }
        }
    },

    handleNativeDragEnd: (e) => {
        e.target.classList.remove('dragging');
        if (window.Tooltips && window.Tooltips.clearHighlights) window.Tooltips.clearHighlights();
    },

    handleNativeDragOver: (e) => {
        e.preventDefault();
        e.dataTransfer.dropEffect = "move";
    },

    handleNativeDragLeave: (e) => {
        // Optional logic
    },

    handleNativeDrop: (e) => {
        const source = e.dataTransfer.getData("source");
        const index = parseInt(e.dataTransfer.getData("index"));
        if (!source || isNaN(index)) return;

        const dropTarget = e.target.closest('.valid-drop-target');
        if (dropTarget) {
            const targetId = dropTarget.id;
            let targetZone = null;
            let targetIndex = -1;

            if (targetId.startsWith('my-stage-slot-')) {
                targetZone = 'stage';
                targetIndex = parseInt(targetId.replace('my-stage-slot-', ''));
            } else if (targetId.startsWith('my-live-slot-')) {
                targetZone = 'live';
                targetIndex = parseInt(targetId.replace('my-live-slot-', ''));
            } else if (targetId === 'my-hand') {
                targetZone = 'hand';
            } else if (targetId === 'my-discard') {
                targetZone = 'discard';
            }

            if (targetZone) {
                DragDrop.executeDrop(source, index, targetZone, targetIndex);
            }
        }
    },

    executeDrop: async (source, index, targetZone, targetIndex) => {
        const state = State.data;
        if (!state || !state.legal_actions) return;

        let action = null;
        if (source === 'hand') {
            if (targetZone === 'stage') {
                action = state.legal_actions.find(a => (a.type === 'PLAY' || a.type === 'FORMATION') && a.hand_idx === index && (a.area_idx === targetIndex || a.slot_idx === targetIndex));
            } else if (targetZone === 'live') {
                action = state.legal_actions.find(a => (a.type === 'PLAY' || a.type === 'LIVE_SET') && a.hand_idx === index);
            } else if (targetZone === 'discard') {
                action = state.legal_actions.find(a => (a.hand_idx === index || a.index === index) && (a.type === 'SELECT_HAND' || (a.name && (a.name.includes('Discard') || a.name.includes('控え室')))));
            }
        } else if (source === 'stage') {
            if (targetZone === 'stage') {
                action = state.legal_actions.find(a => (a.type === 'FORMATION' || a.type === 'MOVE') && (a.source_idx === index || a.prev_idx === index) && (a.area_idx === targetIndex || a.slot_idx === targetIndex));
            }
        }

        if (action && window.doAction) {
            window.doAction(action.id);
        }
    }
};
