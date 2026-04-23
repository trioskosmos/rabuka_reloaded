import { DOMUtils } from './utils/DOMUtils.js';
import { CSS_CLASSES, DOM_IDS } from './constants_dom.js';

function setSidebarButtonState(button, isActive) {
    if (!button) return;
    button.textContent = isActive ? 'X' : '=';
    button.style.background = isActive ? '#444' : 'var(--accent-pink)';
}

function setBoardVisibility(showPlayerBoard) {
    DOMUtils.setVisible(DOM_IDS.CONTAINER_BOARD_PLAYER, showPlayerBoard);
    DOMUtils.setVisible(DOM_IDS.CONTAINER_BOARD_OPPONENT, !showPlayerBoard);

    if (showPlayerBoard) {
        DOMUtils.addClass(DOM_IDS.BTN_SHOW_PLAYER, CSS_CLASSES.ACTIVE);
        DOMUtils.removeClass(DOM_IDS.BTN_SHOW_OPPONENT, CSS_CLASSES.ACTIVE);
        return;
    }

    DOMUtils.removeClass(DOM_IDS.BTN_SHOW_PLAYER, CSS_CLASSES.ACTIVE);
    DOMUtils.addClass(DOM_IDS.BTN_SHOW_OPPONENT, CSS_CLASSES.ACTIVE);
}

function updateMobileSidebarToggleState(isOpen) {
    const btn = DOMUtils.getElement(DOM_IDS.MOBILE_SIDEBAR_TOGGLE);
    setSidebarButtonState(btn, isOpen);
}

let mobileSidebarOverlayHandler = null;

document.addEventListener('DOMContentLoaded', () => {
    // Pre-load static card database
    import('./state.js').then(({ State }) => {
        State.loadStaticCardDatabase();
    });

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
            if (leftSidebar) localStorage.setItem(STORAGE_KEY_LEFT, parseInt(leftSidebar.style.width));
        }
        if (isResizingRight) {
            isResizingRight = false;
            if (resizerRight) resizerRight.classList.remove('resizing');
            if (rightSidebar) localStorage.setItem(STORAGE_KEY_RIGHT, parseInt(rightSidebar.style.width));
        }

        document.body.style.cursor = '';
        document.body.style.userSelect = '';
    });
});

/**
 * Mobile Sidebar Logic
 */
export function toggleSidebar() {
    const sidebars = document.querySelectorAll('.sidebar');
    const btn = DOMUtils.getElement(DOM_IDS.MOBILE_SIDEBAR_TOGGLE);
    if (!sidebars.length || !btn) return;

    const shouldOpen = !Array.from(sidebars).some(s => s.classList.contains('active'));
    sidebars.forEach(s => s.classList.toggle('active', shouldOpen));

    document.body.classList.toggle(CSS_CLASSES.SIDEBAR_OPEN, shouldOpen);
    setSidebarButtonState(btn, shouldOpen);

    // If opening, add one-time listener to close on overlay click
    if (shouldOpen) {
        mobileSidebarOverlayHandler = (e) => {
            // If clicking specifically on the ::after overlay (which is targetable via 'body' since it's a pseudo-element)
            // or if clicking the main-content while sidebar is open
            if (e.target === document.body || e.target.closest('.main-content')) {
                closeSidebar();
                document.removeEventListener('mousedown', mobileSidebarOverlayHandler);
                mobileSidebarOverlayHandler = null;
            }
        };
        // Use a timeout to avoid immediate trigger from the same click
        setTimeout(() => {
            if (mobileSidebarOverlayHandler) {
                document.addEventListener('mousedown', mobileSidebarOverlayHandler);
            }
        }, 10);
    } else if (mobileSidebarOverlayHandler) {
        document.removeEventListener('mousedown', mobileSidebarOverlayHandler);
        mobileSidebarOverlayHandler = null;
    }
}

/**
 * Explicitly closes the mobile sidebars.
 */
export function closeSidebar() {
    const sidebars = document.querySelectorAll('.sidebar.active');
    if (!sidebars.length) {
        if (mobileSidebarOverlayHandler) {
            document.removeEventListener('mousedown', mobileSidebarOverlayHandler);
            mobileSidebarOverlayHandler = null;
        }
        return;
    }

    sidebars.forEach(s => s.classList.remove('active'));
    document.body.classList.remove(CSS_CLASSES.SIDEBAR_OPEN);
    updateMobileSidebarToggleState(false);
    if (mobileSidebarOverlayHandler) {
        document.removeEventListener('mousedown', mobileSidebarOverlayHandler);
        mobileSidebarOverlayHandler = null;
    }
}

/**
 * Tabbed Board Switching
 */
export function switchBoard(side) {
    setBoardVisibility(side === 'player');
}
