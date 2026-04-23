document.addEventListener('DOMContentLoaded', () => {
    // Elements
    const leftSidebar = document.getElementById('sidebar-left');
    const rightSidebar = document.getElementById('sidebar-right');
    const resizerLeft = document.getElementById('resizer-left');
    const resizerRight = document.getElementById('resizer-right');

    const STORAGE_KEY_LEFT = 'lovelive_layout_left_width';
    const STORAGE_KEY_RIGHT = 'lovelive_layout_right_width';

    // Min/Max constraints
    const MIN_WIDTH = 150;
    const MAX_WIDTH_PCT = 0.45; // 45% of screen width

    // Restore Preferences
    const savedLeftObj = localStorage.getItem(STORAGE_KEY_LEFT);
    const savedRightObj = localStorage.getItem(STORAGE_KEY_RIGHT);

    if (savedLeftObj) leftSidebar.style.width = savedLeftObj + 'px';
    if (savedRightObj) rightSidebar.style.width = savedRightObj + 'px';

    // Drag State
    let isResizingLeft = false;
    let isResizingRight = false;

    // --- Left Resizer Logic ---
    resizerLeft.addEventListener('mousedown', (e) => {
        isResizingLeft = true;
        resizerLeft.classList.add('resizing');
        document.body.style.cursor = 'col-resize';
        document.body.style.userSelect = 'none'; // Prevent text selection
    });

    // --- Right Resizer Logic ---
    resizerRight.addEventListener('mousedown', (e) => {
        isResizingRight = true;
        resizerRight.classList.add('resizing');
        document.body.style.cursor = 'col-resize';
        document.body.style.userSelect = 'none';
    });

    // --- Global Mouse Move ---
    document.addEventListener('mousemove', (e) => {
        if (!isResizingLeft && !isResizingRight) return;

        const containerWidth = window.innerWidth;

        if (isResizingLeft) {
            // New Width = Mouse X position
            let newWidth = e.clientX;

            // Constrain
            if (newWidth < MIN_WIDTH) newWidth = MIN_WIDTH;
            if (newWidth > containerWidth * MAX_WIDTH_PCT) newWidth = containerWidth * MAX_WIDTH_PCT;

            leftSidebar.style.width = newWidth + 'px';
        }

        if (isResizingRight) {
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
            resizerLeft.classList.remove('resizing');
            localStorage.setItem(STORAGE_KEY_LEFT, parseInt(leftSidebar.style.width));
        }
        if (isResizingRight) {
            isResizingRight = false;
            resizerRight.classList.remove('resizing');
            localStorage.setItem(STORAGE_KEY_RIGHT, parseInt(rightSidebar.style.width));
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
    const btn = document.getElementById('mobile-sidebar-toggle');
    if (!sidebars.length || !btn) return;

    let isActive = false;
    sidebars.forEach(s => {
        s.classList.toggle('active');
        if (s.classList.contains('active')) isActive = true;
    });

    document.body.classList.toggle('sidebar-open');

    if (isActive) {
        btn.textContent = '✕';
        btn.style.background = '#444';
    } else {
        btn.textContent = '☰';
        btn.style.background = 'var(--accent-pink)';
    }
}

/**
 * Tabbed Board Switching
 */
export function switchBoard(side) {
    const playerBoard = document.getElementById('board-player');
    const opponentBoard = document.getElementById('board-opponent');
    const playerBtn = document.getElementById('btn-show-player');
    const opponentBtn = document.getElementById('btn-show-opponent');

    if (!playerBoard || !opponentBoard || !playerBtn || !opponentBtn) return;

    if (side === 'player') {
        playerBoard.style.display = 'block';
        opponentBoard.style.display = 'none';
        playerBtn.classList.add('active');
        opponentBtn.classList.remove('active');
    } else {
        playerBoard.style.display = 'none';
        opponentBoard.style.display = 'block';
        playerBtn.classList.remove('active');
        opponentBtn.classList.add('active');
    }
}
