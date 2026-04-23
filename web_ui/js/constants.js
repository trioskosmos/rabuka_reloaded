/**
 * Game Constants and Core Utilities
 */

// Phase enum from game_state.py / logic.rs (Rust)
export const Phase = {
    SETUP: -4, "-4": "SETUP",
    RPS: -3, "-3": "RPS",
    TURN_CHOICE: -2, "-2": "TURN_CHOICE",
    MULLIGAN_P1: -1, "-1": "MULLIGAN_P1",
    MULLIGAN_P2: 0, "0": "MULLIGAN_P2",
    ACTIVE: 1, "1": "ACTIVE",
    ENERGY: 2, "2": "ENERGY",
    DRAW: 3, "3": "DRAW",
    MAIN: 4, "4": "MAIN",
    LIVE_SET: 5, "5": "LIVE_SET",
    PERFORMANCE_P1: 6, "6": "PERFORMANCE_P1",
    PERFORMANCE_P2: 7, "7": "PERFORMANCE_P2",
    LIVE_RESULT: 8, "8": "LIVE_RESULT",
    TERMINAL: 9, "9": "TERMINAL",
    RESPONSE: 10, "10": "RESPONSE",
};

// Refined detection: If we are on localhost/127.0.0.1 or have a port, we are NOT a static host
export const isStaticHost = window.location.hostname.includes('github.io') ||
    (window.location.protocol === 'file:') ||
    (window.location.hostname === '' && !window.location.port);

// Helper to get the base URL (handles GitHub Pages subdirectories)
export const getAppBaseUrl = () => {
    const loc = window.location;
    if (loc.hostname.includes('github.io')) {
        const parts = loc.pathname.split('/');
        if (parts.length > 2 && parts[1]) {
            return `/${parts[1]}/`;
        }
    }
    return '/';
};

export const fixImg = (path) => {
    if (!path) return 'img/texticon/icon_energy.png';
    let url = path;

    if (url.startsWith('/')) url = url.substring(1);

    if (!url.startsWith('img/') && !url.startsWith('http')) {
        url = 'img/' + url;
    }

    const isGithub = window.location.hostname.includes('github') || window.location.hostname.includes('rabukasim');


    if (isGithub && url.toLowerCase().endsWith('.png')) {
        url = url.replace(/\.png$/i, '.webp');
    }

    const base = getAppBaseUrl();
    if (base !== '/' && !url.startsWith('http')) {
        url = base + url;
    }

    return url;
};
