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

ANCHOR = "void rb_rules_clear_turn_state(GameState *g);"
assert src.count(ANCHOR) == 1, "anchor count %d" % src.count(ANCHOR)

ADD = eol.join([
    ANCHOR,
    "",
    " * The two readers this clear exists for. They are declared HERE for the",
    " * first time: both were defined in rules.c (rb_rules_cannot_activate_player",
    " * at :2451, rb_rules_cannot_live_player at :2441) and called from",
    " * src/turn/live.c and src/turn/triggers.c, which each had to spell the",
    " * prototype out locally -- so a caller in any other file, including a",
    " * test, would get an implicit int() declaration and, at",
    " * -Wimplicit-function-declaration, no argument checking at all. Same",
    " * defect class as the rb_gsc_* prototypes above.",
    " *",
    " * Both consult a TURN STAMP and return 0 when the stamp is stale, which is",
    " * what makes rb_rules_clear_turn_state's omission of cannot_live_players",
    " * correct: the rollover produces the same observable result as Rust's",
    " * explicit `cannot_live_players.clear()`.",
    " */",
    "int  rb_rules_cannot_activate_player(GameState *g, int pl);",
    "int  rb_rules_cannot_live_player(GameState *g, int pl);",
])
assert src.count(ANCHOR + eol) == 1

# append after the single occurrence of the declaration line
src = src.replace(ANCHOR, ADD, 1)
write(PATH, src)
print("before=%s after=%s" % (before, h(PATH)))

chk = read(PATH)
for s in ("int  rb_rules_cannot_activate_player(GameState *g, int pl);",
          "int  rb_rules_cannot_live_player(GameState *g, int pl);"):
    assert chk.count(s) == 1, "MISSING: " + s
print("both reader prototypes present exactly once")