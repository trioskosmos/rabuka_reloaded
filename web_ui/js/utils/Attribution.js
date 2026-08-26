import { State } from '../state.js';
import { fixImg } from '../constants.js';
import * as i18n from '../i18n/index.js';

// ════════════════════════════════════════════════════════════════════
// Attribution — shared helper for "where does this bonus come from".
//
// The engine emits GameStateDisplay.effect_attribution:
//   { [targetCardId]: [{ source_card_id, ability_text, kind, amount, color }] }
// covering constant (常時) bonuses, success-zone bonuses, and gained
// abilities, plus GameStateDisplay.ability_applications for recent
// one-shot applications (traced ModifyBlade / ModifyCost / etc.).
//
// Consumers:
//   - CardRenderer.renderCardBonuses → per-badge tooltips
//   - CardDetailModal → "Active Effects" section
//   - HeaderStats → per-source drill-down
// ════════════════════════════════════════════════════════════════════

const KIND_BY_BADGE = {
    'bonus-blade': ['blade'],
    'set-blade': ['blade_set'],
    'bonus-heart': ['heart', 'need_heart'],
    'set-heart': ['heart'],
    'bonus-score': ['score'],
    'set-score': ['score_set'],
    'bonus-cost': ['cost'],
    'set-cost': ['cost_set'],
    'bonus-trigger': ['gained_ability'],
    'bonus-transform': [],
};

function kindLabel(kind) {
    const key = `attr_kind_${kind}`;
    const t = i18n.t(key);
    return (t && t !== key) ? t : kind;
}

export function sourceName(cardId) {
    if (cardId === undefined || cardId === null || cardId < 0) return '?';
    const card = State.resolveCardData(cardId);
    return card?.name || `#${cardId}`;
}

function formatAmount(entry) {
    if (!entry.amount && entry.amount !== 0) return '';
    if (entry.kind === 'gained_ability') return '';
    const sign = entry.amount > 0 ? '+' : '';
    return `${sign}${entry.amount} `;
}

export function getAttribution(cardId) {
    return State.data?.effect_attribution?.[cardId] || [];
}

export function getRecentApplications(cardId) {
    const apps = State.data?.ability_applications || [];
    return apps.filter(a => a.target_card_id === cardId);
}

// Entries matching a badge type ('bonus-blade', 'set-cost', …).
export function entriesForBadge(cardId, badgeType) {
    const kinds = KIND_BY_BADGE[badgeType];
    if (!kinds || !kinds.length) return [];
    return getAttribution(cardId).filter(e => kinds.includes(e.kind));
}

// Human-readable one-liner for an attribution entry.
export function formatEntry(entry) {
    const from = i18n.t('attr_from', { card: sourceName(entry.source_card_id) });
    const text = entry.ability_text ? `「${entry.ability_text}」` : '';
    return `${formatAmount(entry)}${kindLabel(entry.kind)} ${text} ${from}`.replace(/\s+/g, ' ').trim();
}

// Tooltip text for a single bonus badge. Empty string when nothing applies.
export function badgeTooltip(cardId, badgeType) {
    const entries = entriesForBadge(cardId, badgeType);
    if (!entries.length) return '';
    return entries.map(formatEntry).join('\n');
}

// ── DOM builders ────────────────────────────────────────────────────

function entryRow(entry) {
    const row = document.createElement('div');
    row.className = 'attr-entry';

    const imgWrap = document.createElement('span');
    imgWrap.className = 'attr-source-thumb';
    const srcCard = State.resolveCardData(entry.source_card_id);
    if (srcCard?.card_no && typeof srcCard.card_no === 'string' && !srcCard.card_no.startsWith('-')) {
        const mapped = State.cardImageMapping?.[srcCard.card_no];
        const img = document.createElement('img');
        img.src = fixImg(mapped || `img/cards_webp/${srcCard.card_no}.webp`);
        img.loading = 'lazy';
        img.onerror = () => img.remove();
        imgWrap.appendChild(img);
    }
    row.appendChild(imgWrap);

    const body = document.createElement('div');
    body.className = 'attr-entry-body';
    const line1 = document.createElement('div');
    line1.className = 'attr-line1';
    const amount = document.createElement('b');
    amount.textContent = formatAmount(entry) + kindLabel(entry.kind);
    line1.appendChild(amount);
    const from = document.createElement('span');
    from.className = 'attr-from';
    from.textContent = ' ' + i18n.t('attr_from', { card: sourceName(entry.source_card_id) });
    line1.appendChild(from);
    body.appendChild(line1);

    if (entry.ability_text) {
        const line2 = document.createElement('div');
        line2.className = 'attr-line2';
        line2.textContent = entry.ability_text;
        body.appendChild(line2);
    }
    row.appendChild(body);
    return row;
}

// Full "Active Effects" list element for a card (empty div when none).
export function renderActiveEffects(cardId) {
    const wrap = document.createElement('div');
    wrap.className = 'attr-section';

    const title = document.createElement('div');
    title.className = 'attr-title';
    title.textContent = i18n.t('attr_active_effects');
    wrap.appendChild(title);

    const entries = getAttribution(cardId);
    if (!entries.length) {
        const empty = document.createElement('div');
        empty.className = 'attr-empty';
        empty.style.opacity = '0.55';
        empty.style.fontSize = '0.72rem';
        empty.textContent = i18n.t('attr_none');
        wrap.appendChild(empty);
        return wrap;
    }
    entries.forEach(e => wrap.appendChild(entryRow(e)));
    return wrap;
}

// Recent one-shot ability applications on this card (may be empty; the
// engine keeps a bounded trace so this is a "recent activity" view).
export function renderRecentApplications(cardId, limit = 6) {
    const apps = getRecentApplications(cardId).slice(-limit).reverse();
    if (!apps.length) return null;

    const wrap = document.createElement('div');
    wrap.className = 'attr-section attr-recent';
    const title = document.createElement('div');
    title.className = 'attr-title';
    title.textContent = i18n.t('attr_recent_activity');
    wrap.appendChild(title);
    apps.forEach(a => {
        wrap.appendChild(entryRow({
            source_card_id: a.source_card_id,
            ability_text: a.ability_text || a.effect_type,
            kind: a.effect_type,
            amount: a.amount,
            color: null,
        }));
    });
    return wrap;
}
