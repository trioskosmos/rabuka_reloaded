import io, sys, hashlib

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

CRLF = "\r\n" if "\r\n" in src else "\n"
anchor = CRLF.join(["int  rb_gsc_current_ability_source_card_id(const GameState *g);", ""])
assert src.count(anchor) == 1, "anchor not unique: %d" % src.count(anchor)

add = anchor + CRLF.join([
    "int  rb_gsc_mulligan_owner_index(const GameState *g);",
    "int  rb_gsc_recently_moved_cards(const GameState *g, int *out, int max);",
    "void rb_gsc_set_recently_moved_cards(GameState *g, const int *cards, int n);",
    "",
])
print("line ending: %r" % CRLF)

src = src.replace(anchor, add, 1)
write(PATH, src)
print("before=%s after=%s" % (before, h(PATH)))

# verify the edit landed
chk = read(PATH)
for s in ("rb_gsc_mulligan_owner_index(const GameState *g);",
          "rb_gsc_recently_moved_cards(const GameState *g, int *out, int max);",
          "rb_gsc_set_recently_moved_cards(GameState *g, const int *cards, int n);"):
    assert chk.count(s) == 1, "DID NOT LAND: " + s
print("all 3 prototypes present exactly once")