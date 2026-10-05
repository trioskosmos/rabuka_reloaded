import io, hashlib
P = "tests/test_tp_activation_offer.c"
s = io.open(P, encoding="utf-8", newline="").read()

# 1) Park the state in a Main phase owned by player 0. The generator reads
#    state->p[state->active] (src/turn/phase.c:2196), so an unset `active` would
#    scan the wrong player and emit nothing.
old = """    game.state.phase = RB_PHASE_MAIN;
    for (int step = 0; step < 20; step++) {"""
new = """    game.state.phase = RB_PHASE_MAIN;
    game.state.active = 0;
    for (int step = 0; step < 20; step++) {"""
assert old in s, "anchor 1 missing"
s = s.replace(old, new, 1)

# 2) Report the WHOLE generated row set once, so a zero offer count is
#    attributable: 0 rows means the phase precondition failed, rows-but-no-
#    USE_ABILITY means the ability scan ran and declined.
old2 = """    printf("info: total OFFERED activations = %d\\n", activations);"""
new2 = """    {
        RbGeneratedActionList dbg = rb_generate_action_candidates(&game.state);
        int use_ability_rows = 0;
        for (int i = 0; i < dbg.count; i++)
            if (dbg.actions[i].action_type == RB_SETUP_ACT_USE_ABILITY) use_ability_rows++;
        printf("info: generated rows=%d of which USE_ABILITY=%d "
               "(phase=%d active=%d energy_active=%d)\\n",
               dbg.count, use_ability_rows, game.state.phase, game.state.active,
               game.state.p[0].energy_active);
    }
    printf("info: total OFFERED activations = %d\\n", activations);"""
assert old2 in s, "anchor 2 missing"
s = s.replace(old2, new2, 1)

old3 = """    game.state.phase = RB_PHASE_MAIN;

    /* RUST :110-111"""
new3 = """    game.state.phase = RB_PHASE_MAIN;
    game.state.active = 0;

    /* RUST :110-111"""
assert old3 in s, "anchor 3 missing"
s = s.replace(old3, new3, 1)

with io.open(P, "r+", encoding="utf-8", newline="") as f:
    f.seek(0)
    f.write(s)
    f.truncate()
print("sha", hashlib.sha256(s.encode("utf-8")).hexdigest())