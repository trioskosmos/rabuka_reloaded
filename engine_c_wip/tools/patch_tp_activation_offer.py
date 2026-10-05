import io, sys, hashlib

P = "tests/test_tp_activation_offer.c"
src = io.open(P, encoding="utf-8", newline="").read()
before = hashlib.sha256(src.encode("utf-8")).hexdigest()

OLD_START = "int activations = 0;"
OLD_END = 'printf("info: total accepted activations = %d\\n", activations);\n}'

i = src.index(OLD_START)
j = src.index(OLD_END) + len(OLD_END)
old = src[i:j]

NEW = '''int activations = 0;
    int all_charged_two = 1;
    int charged_samples = 0;
    /* Main is where the generator emits anything at all
     * (src/turn/phase.c:2156-2160: Rust matches on current_phase and returns
     * Vec::new() for Active/Energy/Draw/Performance phases). */
    game.state.phase = RB_PHASE_MAIN;
    for (int step = 0; step < 20; step++) {
        int en_before = game.state.p[0].energy_active;
        int stage_before[3];
        for (int i = 0; i < 3; i++) stage_before[i] = game.state.p[0].stage[i];

        /* THE RUST LOOP CONDITION.  Rust :44-48 reads the generated menu and
         * breaks when the card is no longer offered; `activations` counts the
         * steps where it WAS offered.  This replaces the retired press proxy. */
        int offers = count_ability_offers(&game.state, sayaka);
        if (offers == 0) {
            printf("step %d: no longer offered (en=%d)\\n", step, en_before);
            break;
        }
        activations++;
        int res = rb_execute_main_phase_action(&game.state, RB_ACT_USE_ABILITY,
                                              sayaka, -1, -1, -1);
        int en_after = game.state.p[0].energy_active;
        int charged = en_before - en_after;
        if (charged_samples == 0 || res > 0) {
            charged_samples++;
            if (charged != 2) all_charged_two = 0;
        }
        printf("step %d: offers=%d ok=%d en %d->%d (charged %d) stage %d,%d,%d -> %d,%d,%d\\n",
               step, offers, res, en_before, en_after, charged,
               stage_before[0], stage_before[1], stage_before[2],
               game.state.p[0].stage[0], game.state.p[0].stage[1],
               game.state.p[0].stage[2]);
        print_stage(&game);

        /* resolve any pending choices with the first option */
        drain_first_option(&game);
        /* Rust: game_setup::settle_single_player_state -- no C equivalent entry
           point; draining the ability queue + recalculating constants is the
           closest real engine settle.  LEFT AS A DOCUMENTED LIMITATION: unlike
           the offer count above, this entry point really is absent. */
        rb_drain_ability_queue(&game.state);
        test_recalc(&game);
        /* The offer list is read at the top of the next step; re-entering Main
         * keeps the generator's phase precondition satisfied across the
         * settle, which does not itself advance the phase. */
        game.state.phase = RB_PHASE_MAIN;
    }

    CHECK(activations > 0,
          "sayaka's {{E}}{{E}} activation is OFFERED at least once with 9 energy "
          "(RUST: count_sayaka_offers > 0, "
          "activation_offer_bounded_and_terminates_test.rs:44-49)");
    if (charged_samples > 0) {
        CHECK(all_charged_two,
              "every accepted activation charged exactly the {{E}}{{E}} = 2 cost");
    } else {
        printf("note: no accepted activation, so the 2-energy charge was never exercised\\n");
    }
    /* RUST'S ACTUAL ASSERTION, verbatim:
     * activation_offer_bounded_and_terminates_test.rs:75-79,
     * `assert!(activations <= 5, ...)`.  `activations` counts OFFERS here, as it
     * does there; the retired press proxy counted something else and overshot. */
    CHECK(activations <= 5,
          "RUST (:75-79): activations <= 5 -- the activation ran %d times, bounded "
          "by affordable energy (9/2 = 4)", activations);
    printf("info: total OFFERED activations = %d\\n", activations);
}'''

out = src[:i] + NEW + src[j:]

OLD2 = '''/// The `offers == 0` half of the Rust assertion has no C equivalent (no
/// generate_possible_actions). The directly checkable half is the affordability
/// pre-check: pressing an unpayable {{E}}{{E}} activation must be refused
/// cleanly \u2014 no partial resolution, no energy spent, nothing left pending.'''
NEW2 = '''/// Rust, activation_offer_bounded_and_terminates_test.rs:94-120, has THREE
/// assertions and the C previously had only the last two:
///   :111  `assert_eq!(offers, 0, "cost 2 with 1 active energy must not be offered")`
///   :115  the press is rejected with an error mentioning cost/energy
///   :120  `assert!(!game.has_pending_choice())`
/// The missing first one was written off with "no C equivalent (no
/// generate_possible_actions)".  `rb_generate_action_candidates` EXISTS
/// (include/rabuka.h:2061) and `count_ability_offers` above reads it, so the
/// `offers == 0` half is asserted here from the real generator, exactly as
/// Rust does.  The press half is kept unchanged: it is Rust's :115 and it is
/// still the direct check of the affordability pre-check.'''
assert OLD2 in out, "second anchor not found"
out = out.replace(OLD2, NEW2, 1)

OLD3 = '''    int en_before = game.state.p[0].energy_active;
    int res = test_activate_ability(&game, sayaka);
    int en_after = game.state.p[0].energy_active;

    CHECK_EQ(res, 0,
             "unpayable {{E}}{{E}} activation is refused by the affordability pre-check");'''
NEW3 = '''    /* Main is the generator's only productive phase (src/turn/phase.c:2156-2160). */
    game.state.phase = RB_PHASE_MAIN;

    /* RUST :110-111 -- generation WITHHOLDS the unpayable activation.  This is
     * the half the old revision dropped as "no C equivalent"; it is read from
     * the real generator now. */
    int offers = count_ability_offers(&game.state, sayaka);
    CHECK_EQ(offers, 0,
             "RUST (:111): cost 2 with 1 active energy must NOT be offered -- "
             "game_setup::generate_possible_actions withholds it; C: "
             "rb_generate_action_candidates");

    int en_before = game.state.p[0].energy_active;
    int res = test_activate_ability(&game, sayaka);
    int en_after = game.state.p[0].energy_active;

    CHECK_EQ(res, 0,
             "RUST (:113-119): a direct press of an unpayable {{E}}{{E}} "
             "activation is rejected by the affordability pre-check");'''
assert OLD3 in out, "third anchor not found"
out = out.replace(OLD3, NEW3, 1)

# WRITE VIA r+ + truncate, NOT 'w'.
# This tree lives on OneDrive (Attributes 0x80020). Opening with mode 'w' issues
# O_TRUNC at open time and the cloud provider refuses that with EACCES, even
# though r+ opens cleanly and IsReadOnly is False. Truncating after the open
# works, so the file is never actually destroyed by a failed open.
with io.open(P, "r+", encoding="utf-8", newline="") as fh:
    fh.seek(0)
    fh.write(out)
    fh.truncate()
after = hashlib.sha256(out.encode("utf-8")).hexdigest()
print("before", before)
print("after ", after)
print("bytes", len(out.encode("utf-8")))