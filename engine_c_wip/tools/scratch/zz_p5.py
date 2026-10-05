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
    "void rb_check_expired_effects(GameState *g, int which);",
    "void rb_advance_phase(GameState *g);",
    "",
])
assert src.count(OLD) == 1, "anchor count %d" % src.count(OLD)

NEW = eol.join([
    "void rb_check_expired_effects(GameState *g, int which);",
    "void rb_advance_phase(GameState *g);",
    "",
    "/* rb_rules_clear_turn_state is DEFINED in src/ability/effects/rules.c:2500",
    " * and CALLED from src/turn/phase.c:753 in the turn rollover, right beside the",
    " * GameState-side folds of the same three Rust per-turn sets.",
    " *",
    " * The prototype lived mid-function inside phase.c and, before the body",
    " * existed, inside a COMMENT block in both phase.c and live.c -- so the",
    " * symbol once had two declarations, no definition, and no caller, while the",
    " * checklist still read `[x]`. The definition now exists, so this is the",
    " * single declaration and phase.c's local one is redundant.",
    " *",
    " * What it clears, against Rust turn/phases.rs:313-317:",
    " *   :314 turn_limited_abilities_used.clear()",
    " *        no clear needed -- the C use-limit table is keyed by turn",
    " *        (ability_queue.c:1258/1267/1282 test q->use_turn != cur_turn),",
    " *        which is the same guarantee the explicit clear buys Rust.",
    " *   :315 cannot_activate_members.clear()  -> cannot_activate_members[2] = 0",
    " *   :316 cannot_live_players.clear()",
    " *        NOT cleared here, by design: the flag carries a turn stamp",
    " *        (rules.c:2432-2433 write cannot_live_turn) and its only reader",
    " *        (rb_rules_cannot_live_player, rules.c:2445) returns 0 when the",
    " *        stamp is stale. A stamp is what BOTH Rust clear sites produce.",
    " *   :317 wait_immune_members.clear()      -> n_wait_immune = 0",
    " * The GameState-side members of the same sets (player_cannot_activate[2],",
    " * n_cannot_active_cards, n_prohibition_effects) are cleared by phase.c",
    " * itself, on either side of the call. */",
    "void rb_rules_clear_turn_state(GameState *g);",
    "",
])

src = src.replace(OLD, NEW, 1)
write(PATH, src)
print("before=%s after=%s" % (before, h(PATH)))

chk = read(PATH)
assert chk.count("void rb_rules_clear_turn_state(GameState *g);") == 1
print("prototype present exactly once")