// Test: simulate web UI normalizeCode + allCardsArr flow
const fs = require('fs');
const path = require('path');

function normalizeCode(code) {
  if (!code) return '';
  return code.replace(/＋/g, '+').replace(/－/g, '-').replace(/ー/g, '-').trim().toUpperCase();
}

const cardsJson = JSON.parse(fs.readFileSync(path.join(__dirname, 'cards', 'cards.json'), 'utf8'));
const allCardsArr = Object.values(cardsJson).filter(c => c && c.card_no);

// Build dictionary like getQRCardDict does (with normalizeCode)
const sorted = allCardsArr.map(c => normalizeCode(c.card_no)).filter(Boolean).sort();
const dict = {};
sorted.forEach((k, i) => { dict[k] = i; });
console.log(`Dictionary: ${sorted.length} cards (normalized)`);

// Check for any full-width chars that would cause mismatch
const fullWidth = allCardsArr.filter(c => c.card_no.includes('＋') || c.card_no.includes('！'));
console.log(`Cards with full-width chars: ${fullWidth.length}`);
if (fullWidth.length > 0) {
  for (const c of fullWidth.slice(0, 5)) {
    console.log(`  raw: ${c.card_no} → normalized: ${normalizeCode(c.card_no)}`);
  }
}

// Parse a deck with full-width chars
const deckText = `3 x PL!-pb1-018-R
2 x PL!N-sd1-008-SD
4 x PL!SP-bp1-005-P
2 x PL!SP-bp4-004-P＋`;

const entries = [];
for (let line of deckText.split('\n')) {
  line = line.trim();
  if (!line) continue;
  const parts = line.split(/\s*x\s*/i);
  if (parts.length >= 2) {
    const a = parts[0].trim(), b = parts[parts.length - 1].trim();
    let cardNo, qty;
    if (/^\d+$/.test(a)) { qty = parseInt(a, 10); cardNo = normalizeCode(b); }
    else if (/^\d+$/.test(b)) { qty = parseInt(b, 10); cardNo = normalizeCode(a); }
    else { cardNo = normalizeCode(line); qty = 1; }
    entries.push([cardNo, qty]);
  }
}

console.log(`\nDeck entries (normalized):`);
entries.forEach(([c, q]) => console.log(`  ${c} x ${q} — dict idx: ${dict[c]}`));

// Encode
const buf = Buffer.alloc(1 + entries.length * 3);
buf[0] = entries.length + 1;
let fail = false;
for (let i = 0; i < entries.length; i++) {
  const idx = dict[entries[i][0]];
  if (idx === undefined) { console.error(`NOT IN DICT: ${entries[i][0]}`); fail = true; continue; }
  buf[1 + i * 3] = ((idx >> 8) & 0xFF) + 1;
  buf[1 + i * 3 + 1] = (idx & 0xFF) + 1;
  buf[1 + i * 3 + 2] = Math.min(entries[i][1], 255) + 1;
}
if (fail) { console.error('ENCODE FAILED'); process.exit(1); }

// Decode
const data = buf.toString('binary');
const count = data.charCodeAt(0) - 1;
const decoded = [];
for (let i = 0; i < count; i++) {
  const base = 1 + i * 3;
  const idx = ((data.charCodeAt(base) - 1) << 8) | (data.charCodeAt(base + 1) - 1);
  const qty = data.charCodeAt(base + 2) - 1;
  decoded.push([sorted[idx], qty || 1]);
}

let ok = true;
for (let i = 0; i < entries.length; i++) {
  if (entries[i][0] !== decoded[i][0] || entries[i][1] !== decoded[i][1]) {
    console.error(`MISMATCH ${i}: ${entries[i]} vs ${decoded[i]}`);
    ok = false;
  }
}
console.log(ok ? `\n✓ PASSED — ${buf.length} bytes binary` : '\n✗ FAILED');
