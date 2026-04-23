export const Phase = {
    ROCK_PAPER_SCISSORS: "RockPaperScissors",
    CHOOSE_FIRST_ATTACKER: "ChooseFirstAttacker",
    MULLIGAN: "Mulligan",
    ACTIVE: "Active",
    ENERGY: "Energy",
    DRAW: "Draw",
    MAIN: "Main",
    LIVE_CARD_SET: "LiveCardSet",
    FIRST_ATTACKER_PERFORMANCE: "FirstAttackerPerformance",
    SECOND_ATTACKER_PERFORMANCE: "SecondAttackerPerformance",
    LIVE_VICTORY_DETERMINATION: "LiveVictoryDetermination",
};

export const isStaticHost = window.location.hostname.includes('github.io') ||
    (window.location.protocol === 'file:') ||
    (window.location.hostname === '' && !window.location.port);

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

    if (isGithub && url.toLowerCase().endsWith('.png') && !url.toLowerCase().includes('.webp')) {
        url = url.replace(/\.png$/i, '.webp');
    }

    const base = getAppBaseUrl();
    if (base !== '/' && !url.startsWith('http')) {
        url = base + url;
    }

    return url;
};
