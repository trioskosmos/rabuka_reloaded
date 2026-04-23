/**
 * Centralized DOM Element IDs and Constants
 * Single source of truth for all hardcoded DOM IDs across the frontend
 */

export const DOM_IDS = {
  // Modals
  MODAL_ROOM: 'room-modal',
  MODAL_PERSPECTIVE: 'perspective-modal',
  MODAL_LOG_VIEWER: 'log-viewer-modal',
  MODAL_PERFORMANCE: 'performance-modal',
  MODAL_SETUP: 'setup-modal',
  MODAL_DISCARD: 'discard-modal',
  MODAL_PASTE_REPLAY: 'paste-replay-modal',

  // Main Containers
  CONTAINER_ACTIONS: 'actions',
  CONTAINER_MOBILE_ACTION_BAR: 'mobile-action-bar',
  CONTAINER_BOARD_PLAYER: 'board-player',
  CONTAINER_BOARD_OPPONENT: 'board-opponent',
  CONTAINER_GAME_FEED: 'game-feed',
  CONTAINER_RULE_LOG: 'rule-log',
  REPLAY_CONTROLS: 'replay-controls',
  SELECTION_MODAL: 'selection-modal',
  SELECTION_CONTENT: 'selection-content',
  ACTIVE_ABILITIES_PANEL: 'active-abilities-panel',
  MY_DECK_BOTTOM: 'my-deck-bottom',
  OPP_DECK_BOTTOM: 'opp-deck-bottom',

  // Log Viewer Specific
  LOG_VIEWER_CONTENT: 'log-viewer-content',
  LOG_VIEWER_ENTRIES: 'log-viewer-entries',
  FILTER_TRIGGER: 'filter-trigger',
  FILTER_EFFECT: 'filter-effect',

  // Public Rooms
  PUBLIC_ROOMS_LIST: 'public-rooms-list',
  ROOM_CODE_INPUT: 'room-code-input',

  // UI Elements
  SYSTEM_STATUS_BADGE: 'system-status-badge',
  ROOM_CODE_HEADER: 'room-code-header',
  ROOM_DISPLAY: 'room-display',
  HEADER_DEBUG_INFO: 'header-debug-info',
  MOBILE_SIDEBAR_TOGGLE: 'mobile-sidebar-toggle',
  SWITCH_BTN: 'switch-btn',
  BTN_SHOW_PLAYER: 'btn-show-player',
  BTN_SHOW_OPPONENT: 'btn-show-opponent',

  // Debug Overlay Elements
  DEBUG_SYNC: 'debug-sync',
  DEBUG_ROOM: 'debug-room',
  DEBUG_SESSION: 'debug-session',
  DEBUG_VIEW: 'debug-view',
  DEBUG_POLL: 'debug-poll',
  DEBUG_DELAY: 'debug-delay',

  // Layout
  SIDEBAR_LEFT: 'sidebar-left',
  SIDEBAR_RIGHT: 'sidebar-right',
  RESIZER_LEFT: 'resizer-left',
  RESIZER_RIGHT: 'resizer-right',

  // Special
  LOOKED_CARDS_PANEL: 'looked-cards-panel',
  LOOKED_CARDS_CONTENT: 'looked-cards-content',
};

export const DISPLAY_VALUES = {
  NONE: 'none',
  FLEX: 'flex',
  BLOCK: 'block',
  GRID: 'grid',
};

export const COLORS = {
  // Status colors
  ONLINE: '#2ecc71',
  OFFLINE: '#e74c3c',
  UNKNOWN: '#e74c3c',
  WARNING: '#f59e0b',
  
  // Accent colors
  ACCENT_BLUE: '#0096ff',
  ACCENT_RED: '#ff5555',
  ACCENT_GOLD: '#f1c40f',
  ACCENT_PINK: 'var(--accent-pink)',
  
  // Text colors
  TEXT_DARK: '#333',
  TEXT_LIGHT: '#fff',
  TEXT_SEMIDARK: '#666',
  
  // Background colors
  BG_BLUE_LIGHT: 'rgba(0,150,255,0.1)',
  BG_ERROR: 'rgba(255,85,85,0.1)',
};

export const CSS_CLASSES = {
  RESIZING: 'resizing',
  ACTION_PENDING: 'action-pending',
  SIDEBAR_OPEN: 'sidebar-open',
  ACTIVE: 'active',
  HIGHLIGHTED: 'highlighted',
  COL_RESIZE: 'col-resize',
};
