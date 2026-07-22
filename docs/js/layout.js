import { DOMUtils } from './utils/DOMUtils.js';
import { CSS_CLASSES, DOM_IDS } from './constants_dom.js';
import { State } from './state.js';

function setSidebarButtonState(button, isActive) {
    if (!button) return;
    button.classList.toggle('active', isActive);
}

let _bothFlipped = false;

function setBoardMode(mode) {
    const gb = document.getElementById('game-board');
    const enteringBoth = mode === 'both';
    gb.classList.toggle('both-mode', enteringBoth);
    if (!enteringBoth) {
        gb.classList.remove('both-mode-flipped');
        _bothFlipped = false;
    }

    if (mode === 'both') {
        DOMUtils.setVisible(DOM_IDS.CONTAINER_BOARD_PLAYER, true);
        DOMUtils.setVisible(DOM_IDS.CONTAINER_BOARD_OPPONENT, true);
        DOMUtils.addClass(DOM_IDS.BTN_SHOW_BOTH, CSS_CLASSES.ACTIVE);
        DOMUtils.removeClass(DOM_IDS.BTN_SHOW_PLAYER, CSS_CLASSES.ACTIVE);
        DOMUtils.removeClass(DOM_IDS.BTN_SHOW_OPPONENT, CSS_CLASSES.ACTIVE);
    } else {
        DOMUtils.setVisible(DOM_IDS.CONTAINER_BOARD_PLAYER, mode === 'player');
        DOMUtils.setVisible(DOM_IDS.CONTAINER_BOARD_OPPONENT, mode === 'opponent');
        DOMUtils.removeClass(DOM_IDS.BTN_SHOW_BOTH, CSS_CLASSES.ACTIVE);
        DOMUtils.removeClass(DOM_IDS.BTN_SHOW_PLAYER, CSS_CLASSES.ACTIVE);
        DOMUtils.removeClass(DOM_IDS.BTN_SHOW_OPPONENT, CSS_CLASSES.ACTIVE);
        DOMUtils.addClass(DOM_IDS[`BTN_SHOW_${mode.toUpperCase()}`], CSS_CLASSES.ACTIVE);
    }
}

function updateMobileSidebarToggleState(side, isOpen) {
    const id = side === 'left' ? DOM_IDS.MOBILE_TOGGLE_LOG : DOM_IDS.MOBILE_TOGGLE_ACTIONS;
    const btn = DOMUtils.getElement(id);
    setSidebarButtonState(btn, isOpen);
}

let mobileSidebarOverlayHandler = null;
const OVERLAY_EVENTS = ['mousedown', 'touchstart'];

function isAnySidebarOpen() {
    return document.querySelectorAll('.sidebar.active').length > 0;
}

function isClickInsideSidebar(target) {
    return target && target.closest('.sidebar');
}

function refreshOverlay() {
    const open = isAnySidebarOpen();
    document.body.classList.toggle(CSS_CLASSES.SIDEBAR_OPEN, open);
}

function handleOverlayEvent(e) {
    if (isClickInsideSidebar(e.target)) return;
    closeSidebar();
}

function setupOverlayListener() {
    if (mobileSidebarOverlayHandler) return;
    mobileSidebarOverlayHandler = handleOverlayEvent;
    setTimeout(() => {
        if (mobileSidebarOverlayHandler) {
            OVERLAY_EVENTS.forEach(evt => {
                document.addEventListener(evt, mobileSidebarOverlayHandler, { passive: true });
            });
        }
    }, 10);
}

function teardownOverlayListener() {
    if (mobileSidebarOverlayHandler) {
        OVERLAY_EVENTS.forEach(evt => {
            document.removeEventListener(evt, mobileSidebarOverlayHandler);
        });
        mobileSidebarOverlayHandler = null;
    }
}

document.addEventListener('DOMContentLoaded', () => {
    // Pre-load static card database
    State.loadStaticCardDatabase();

    // Elements
    const leftSidebar = DOMUtils.getElement(DOM_IDS.SIDEBAR_LEFT);
    const rightSidebar = DOMUtils.getElement(DOM_IDS.SIDEBAR_RIGHT);
    const resizerLeft = DOMUtils.getElement(DOM_IDS.RESIZER_LEFT);
    const resizerRight = DOMUtils.getElement(DOM_IDS.RESIZER_RIGHT);

    const STORAGE_KEY_LEFT = 'lovelive_layout_left_width';
    const STORAGE_KEY_RIGHT = 'lovelive_layout_right_width';

    // Min/Max constraints
    const MIN_WIDTH = 150;
    const MAX_WIDTH_PCT = 0.45; // 45% of screen width

    // Restore Preferences
    const savedLeftObj = localStorage.getItem(STORAGE_KEY_LEFT);
    const savedRightObj = localStorage.getItem(STORAGE_KEY_RIGHT);

    if (savedLeftObj && leftSidebar) DOMUtils.setStyle(DOM_IDS.SIDEBAR_LEFT, 'width', savedLeftObj + 'px');
    if (savedRightObj && rightSidebar) DOMUtils.setStyle(DOM_IDS.SIDEBAR_RIGHT, 'width', savedRightObj + 'px');

    // Drag State
    let isResizingLeft = false;
    let isResizingRight = false;

    // --- Left Resizer Logic ---
    if (resizerLeft) {
        resizerLeft.addEventListener('mousedown', (e) => {
            isResizingLeft = true;
            resizerLeft.classList.add('resizing');
            document.body.style.cursor = 'col-resize';
            document.body.style.userSelect = 'none'; // Prevent text selection
        });
    }

    // --- Right Resizer Logic ---
    if (resizerRight) {
        resizerRight.addEventListener('mousedown', (e) => {
            isResizingRight = true;
            resizerRight.classList.add('resizing');
            document.body.style.cursor = 'col-resize';
            document.body.style.userSelect = 'none';
        });
    }

    // --- Global Mouse Move ---
    document.addEventListener('mousemove', (e) => {
        if (!isResizingLeft && !isResizingRight) return;

        const containerWidth = window.innerWidth;

            if (isResizingLeft && leftSidebar) {
            // New Width = Mouse X position
            let newWidth = e.clientX;

            // Constrain
            if (newWidth < MIN_WIDTH) newWidth = MIN_WIDTH;
            if (newWidth > containerWidth * MAX_WIDTH_PCT) newWidth = containerWidth * MAX_WIDTH_PCT;

            leftSidebar.style.width = newWidth + 'px';
        }

            if (isResizingRight && rightSidebar) {
            // New Width = Container Width - Mouse X position
            let newWidth = containerWidth - e.clientX;

            // Constrain
            if (newWidth < MIN_WIDTH) newWidth = MIN_WIDTH;
            if (newWidth > containerWidth * MAX_WIDTH_PCT) newWidth = containerWidth * MAX_WIDTH_PCT;

            rightSidebar.style.width = newWidth + 'px';
        }
    });

    // --- Global Mouse Up ---
    document.addEventListener('mouseup', () => {
        if (isResizingLeft) {
            isResizingLeft = false;
            if (resizerLeft) resizerLeft.classList.remove('resizing');
            if (leftSidebar) localStorage.setItem(STORAGE_KEY_LEFT, parseInt(leftSidebar.style.width) || 0);
        }
        if (isResizingRight) {
            isResizingRight = false;
            if (resizerRight) resizerRight.classList.remove('resizing');
            if (rightSidebar) localStorage.setItem(STORAGE_KEY_RIGHT, parseInt(rightSidebar.style.width) || 0);
        }

        document.body.style.cursor = '';
        document.body.style.userSelect = '';
    });
});

/**
 * Mobile Sidebar Logic
 */
function toggleOneSidebar(side) {
    const sidebar = document.querySelector(`.sidebar-${side}`);
    const isOpen = sidebar && sidebar.classList.contains('active');

    if (sidebar) {
        sidebar.classList.toggle('active');
    }

    updateMobileSidebarToggleState(side === 'left' ? 'left' : 'right', !isOpen);
    refreshOverlay();

    if (!isOpen) {
        setupOverlayListener();
    } else if (!isAnySidebarOpen()) {
        teardownOverlayListener();
    }
}

export function toggleLogSidebar() {
    toggleOneSidebar('left');
}

export function toggleActionsSidebar() {
    toggleOneSidebar('right');
}

/**
 * Explicitly closes all mobile sidebars.
 */
export function closeSidebar() {
    document.querySelectorAll('.sidebar.active').forEach(s => s.classList.remove('active'));
    document.body.classList.remove(CSS_CLASSES.SIDEBAR_OPEN);
    updateMobileSidebarToggleState('left', false);
    updateMobileSidebarToggleState('right', false);
    teardownOverlayListener();
}

/**
 * Tabbed Board Switching — static perspective, no auto-flip
 */
export function switchBoard(side) {
    setBoardMode(side);
}
