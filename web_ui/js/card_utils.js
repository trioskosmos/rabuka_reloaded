import { State } from './state.js';

export function normalizeCode(code) {
  if (!code) return '';
  return code.replace(/＋/g, '+').replace(/－/g, '-').replace(/ー/g, '-').trim().toUpperCase();
}

export function extractCardId(title) {
  const parts = title.split(/\s*:\s*/);
  return normalizeCode(parts[0]);
}

// ---- Card lookup helpers ----
let _ciIndex = null;
function _ensureIndex() {
  const db = State.staticCardDatabase;
  if (!db || _ciIndex) return;
  _ciIndex = {};
  for (const key of Object.keys(db)) {
    _ciIndex[key.replace(/＋/g, '+').toUpperCase()] = key;
  }
}

/** Resolve a card record by card number, tolerating ＋/+/case differences. */
export function lookupCard(no) {
  const db = State.staticCardDatabase;
  if (!db || !no) return null;
  let card = db[no];
  if (card) return card;
  _ensureIndex();
  const nk = no.replace(/＋/g, '+').toUpperCase();
  const actualKey = _ciIndex?.[nk];
  return actualKey ? db[actualKey] : null;
}

// ---- Point system (現在ポイントが設定されているカード) ----
const _POINT_MAP = {
    'LL-bp2-001-R+': 5, 'LL-bp2-001-R＋': 5,
    'PL!N-bp1-003-R+': 4, 'PL!N-bp1-003-P': 4, 'PL!N-bp1-003-P＋': 4, 'PL!N-bp1-003-SEC': 4,
    'PL!N-bp1-012-R+': 3, 'PL!N-bp1-012-P': 3, 'PL!N-bp1-012-P＋': 3, 'PL!N-bp1-012-SEC': 3,
    'PL!N-bp1-002-R+': 2, 'PL!N-bp1-002-P': 2, 'PL!N-bp1-002-P＋': 2, 'PL!N-bp1-002-SEC': 2,
    'PL!N-sd1-008-SD': 2, 'PL!N-sd1-008-RM': 2, 'PL!HS-bp2-014-N': 2,
    'PL!N-pb1-011-R': 2, 'PL!N-pb1-011-P＋': 2,
    'PL!SP-bp1-005-R': 1, 'PL!SP-bp1-005-P': 1, 'PL!N-bp1-029-L': 1,
    'PL!SP-sd1-019-SD': 1, 'PL!SP-sd1-019-RM': 1, 'PL!SP-sd1-019-SD2': 1, 'PL!SP-sd1-019-P': 1,
    'PL!SP-sd1-020-SD': 1, 'PL!SP-sd1-020-RM': 1, 'PL!SP-sd1-020-SD2': 1, 'PL!SP-sd1-020-P': 1,
    'PL!SP-pb1-014-N': 1, 'PL!N-bp3-030-L': 1, 'PL!N-bp4-030-L': 1,
};
let _ptCI = null;
export function cardPoints(no) {
    let p = _POINT_MAP[no];
    if (p !== undefined) return p;
    if (!_ptCI) {
        _ptCI = {};
        for (const [k, v] of Object.entries(_POINT_MAP)) {
            _ptCI[k.replace(/＋/g, '+').toUpperCase()] = v;
        }
    }
    const nk = no.replace(/＋/g, '+').toUpperCase();
    return _ptCI[nk] ?? 0;
}

/**
 * Analyze a deck given as an array of card numbers (expanded, one per copy).
 * Returns { members, lives, energy, points }. Point cards that aren't recognized
 * count toward `unknown` rather than `points`.
 */
export function analyzeDeckList(cardNos) {
    let members = 0, lives = 0, energy = 0, points = 0, unknown = 0;
    for (const no of cardNos) {
        if (!no || typeof no !== 'string' || !no.includes('-')) continue;
        const type = lookupCard(no)?.type || '';
        if (type === 'メンバー') members++;
        else if (type === 'ライブ') lives++;
        else if (type === 'エネルギー') energy++;
        const pt = cardPoints(no);
        if (pt > 0) points += pt;
        else unknown++;
    }
    return { members, lives, energy, points, unknown };
}

/** Short label for a deck composition, e.g. "M:48 L:12 P:5". */
export function deckCompositionLabel(analysis) {
    const parts = [`M:${analysis.members}`, `L:${analysis.lives}`];
    if (analysis.points > 0) parts.push(`P:${analysis.points}`);
    return parts.join(' ');
}
