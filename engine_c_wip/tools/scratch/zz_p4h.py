import io, hashlib

PATH = "include/rabuka.h"

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
    "const char *rb_trigger_zone_label(int is_stage);",
    "const char *rb_trigger_to_texticon(const char *trigger);",
    "",
])
assert src.count(OLD) == 1, "anchor count %d" % src.count(OLD)

NEW = eol.join([
    "const char *rb_trigger_zone_label(int is_stage);",
    "/* rb_trigger_to_texticon WAS HERE and was DELETED 2026-10-05, together with",
    " * its definition in src/turn/triggers.c. Its only caller engine-wide is",
    " * Rust game/display.rs:1569 and :1599 (the `bonus_triggers` badge map), and",
    " * the C has no display layer, so it had zero reachable callers here. The",
    " * tombstone and the full deleted body are at src/turn/triggers.c, which is",
    " * where the Rust citation (triggers.rs:139-149) is recorded. */",
    "",
])

src = src.replace(OLD, NEW, 1)
write(PATH, src)
print("before=%s after=%s" % (before, h(PATH)))

chk = read(PATH)
assert "const char *rb_trigger_to_texticon(const char *trigger);" not in chk
print("prototype removed, tombstone in place")