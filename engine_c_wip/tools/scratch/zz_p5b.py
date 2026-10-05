import io, hashlib

PATH = "src/turn/phase.c"

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
    "           Declared here for the same reason and with the same precedent as",
    "           rb_check_effect_position at src/turn/triggers.c:6-8. */",
    "        void rb_rules_clear_turn_state(GameState *g);",
    "        rb_rules_clear_turn_state(g);",
])
assert src.count(OLD) == 1, "anchor count %d" % src.count(OLD)

NEW = eol.join([
    "           Now declared in include/rabuka.h, beside rb_advance_phase. It used",
    "           to be declared HERE, mid-function: legal C, but it left the symbol",
    "           invisible to every other translation unit, and before the body",
    "           existed in rules.c it was spelled only inside COMMENT blocks in this",
    "           file and in live.c -- so checklist item I2 could read `[x]` for a",
    "           symbol with no definition and no caller. The header also records what",
    "           it clears against Rust turn/phases.rs:313-317. */",
    "        rb_rules_clear_turn_state(g);",
])

src = src.replace(OLD, NEW, 1)
write(PATH, src)
print("before=%s after=%s" % (before, h(PATH)))

chk = read(PATH)
assert "void rb_rules_clear_turn_state(GameState *g);" not in chk, "local decl survived"
assert chk.count("rb_rules_clear_turn_state(g);") == 1, "call site count wrong"
print("local declaration removed, call site intact")