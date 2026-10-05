import io, sys, hashlib

PATH = "src/turn/triggers.c"

def read(p):
    with io.open(p, "r", encoding="utf-8", newline="") as f:
        return f.read()

def write(p, s):
    with io.open(p, "w", encoding="utf-8", newline="") as f:
        f.write(s)

def h(p):
    return hashlib.sha256(open(p, "rb").read()).hexdigest()[:12]

src = read(PATH)
before = h(PATH)
eol = "\r\n" if "\r\n" in src else "\n"

OLD = eol.join([
    '/* Map a trigger string to its texticon filename for card badge display.',
    '   Mirrors engine/src/triggers.rs::trigger_to_texticon. Returns a static',
    '   string literal; do not free. */',
    'const char *rb_trigger_to_texticon(const char *trigger) {',
    '    if (!trigger) return "jyouji";',
    '    if (!strcmp(trigger, RB_TSTR_CONSTANT))      return "jyouji";',
    '    if (!strcmp(trigger, RB_TSTR_LIVE_SUCCESS))   return "live_success";',
    '    if (!strcmp(trigger, RB_TSTR_LIVE_START))     return "live_start";',
    '    if (!strcmp(trigger, RB_TSTR_DEBUT) || !strcmp(trigger, RB_TSTR_DEBUT_EN))',
    '        return "toujyou";',
    '    if (!strcmp(trigger, RB_TSTR_ACTIVATION))     return "kidou";',
    '    if (!strcmp(trigger, RB_TSTR_AUTO))           return "jidou";',
    '    return "jyouji";',
    '}',
    '',
])
assert src.count(OLD) == 1, "block not found uniquely: %d" % src.count(OLD)

NEW = eol.join([
    '/* DELETED 2026-10-05 (zero-callers census, checked RUST FIRST as required).',
    '   rb_trigger_to_texticon had ZERO callers in the C tree, and checking Rust',
    '   first is what settles it: its ONLY caller engine-wide is',
    '   game/display.rs:1569 and :1599, inside the `bonus_triggers` map that the',
    '   card-badge frontend renders. engine/src/triggers.rs:137-138 says so in its',
    '   own doc comment -- "Used by gain_ability to show the gained ability\'s',
    '   trigger type as a texticon on the card" -- i.e. it is a DISPLAY-LAYER',
    '   helper, not a gameplay one.',
    '',
    '   Zero C callers therefore does not mean "premature": the C has no display',
    '   layer at all, so the function had nothing to serve. Deleting the port is',
    '   correct and the tombstone is here so a future reader does not re-port it',
    '   from triggers.rs and then have to discover again that nothing can call it.',
    '',
    '   The six RB_TSTR_* constants it compared are NOT orphaned: rb_canonical_trigger',
    '   above and kind_of_token below both use the same set, and they are live.',
    '   Re-porting, when a display layer exists, is a one-function job against',
    '   triggers.rs:139-149.',
    '   HISTORY (deleted body, for the record): mapped CONSTANT->jyouji,',
    '   LIVE_SUCCESS->live_success, LIVE_START->live_start, DEBUT|DEBUT_EN->toujyou,',
    '   ACTIVATION->kidou, AUTO->jidou, with "jyouji" as the NULL and unknown',
    '   fallback -- identical to triggers.rs:140-148 including the fallback arm. */',
    '',
])

src = src.replace(OLD, NEW, 1)
write(PATH, src)
print("before=%s after=%s eol=%r" % (before, h(PATH), eol))

chk = read(PATH)
assert "const char *rb_trigger_to_texticon" not in chk, "DEFINITION STILL PRESENT"
assert "rb_trigger_to_texticon(const char *trigger) {" not in chk, "DEFINITION STILL PRESENT"
print("definition removed; only tombstone mention remains")