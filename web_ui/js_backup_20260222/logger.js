/**
 * Logging System
 * Handles application logs and the game feed UI.
 */

export const logs = [];
export const feedItems = [];

export function log(msg, type = 'normal') {
    const timestamp = new Date().toLocaleTimeString();
    logs.unshift({ timestamp, msg, type });
    if (logs.length > 100) logs.pop();

    // Also add to game feed if it's an interesting human action
    if (type === 'action' || type === 'score') {
        addToFeed(msg, type);
    }
}

function addToFeed(msg, type) {
    const icons = {
        'action': '🎫',
        'score': '✨',
        'effect': '🪄',
        'turn': '📅'
    };
    const icon = icons[type] || '📝';

    feedItems.unshift({ msg, icon, timestamp: Date.now() });
    if (feedItems.length > 20) feedItems.shift();
    renderFeed();
}

function renderFeed() {
    const feedEl = document.getElementById('game-feed');
    if (!feedEl) return;

    feedEl.innerHTML = feedItems.map(item => `
        <div class="feed-item">
            <span class="feed-icon">${item.icon}</span>
            <span class="feed-text">${item.msg}</span>
        </div>
    `).join('');
    // feedEl.scrollTop = 0; // Usually better to scroll to top for new items unless reversed
}
