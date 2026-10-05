import io, sys

PATH = "tests/test_parity_zone_source.c"

TEST = r'''
/* =======================================================================
 * F. THE REPLACED CARD RIDES IN THE DEDICATED SLOT, NOT IN `resume_host`.
 *
 * Rust carries it in its own field and CONSUMES it on read:
 *
 *     engine/src/turn/live.rs:1761
 *         game_state.pending_success_replacement_card_id = Some(card_id);
 *     engine/src/turn/actions/mod.rs:894
 *         if let Some(replaced_card_id) =
 *             game_state.pending_success_replacement_card_id.take()
 *
 * The C overloaded the SHARED `resume_host` slot (live.c), which is clobbered
 * between the prompt being raised and the answer arriving. MEASURED on the
 * corpus: 7 of 13 prompt/result pairs carried prompt_original=293 but
 * result_original=494 -- the replacement was applied to the wrong card, which
 * is "CROSSROADS is in the waitroom (replaced)" in this suite.
 *
 * The second half of the Rust shape is the `.take()`: it leaves None behind,
 * so a SECOND read of the same prompt cannot see the card again. Both halves
 * are asserted, and the presence test is `>= 0` because 0 is a legal card id.
 * ======================================================================= */
static void live_phase_replacement_carries_the_card_in_the_dedicated_slot(void)
{
    static TestGame tg;
    test_game_new(&tg);
    clear_p1(&tg);
    int crossroads = test_id(&tg, CROSSROADS);
    int muse = test_id(&tg, BOKURA);
    pin_id(crossroads, CROSSROADS);
    pin_id(muse, BOKURA);
    if (crossroads < 0 || muse < 0) return;

    /* F0: the sentinel is None on a fresh state, NOT 0 -- 0 is a legal card
     * index, so a memset-zeroed slot would read as "card 0 is pending". */
    CHECK_EQ(tg.state.queue.pending_success_replacement_card_id, -1,
             "F: a fresh game has NO pending replacement (None, not card 0)");

    test_add_to_live(&tg, crossroads);
    test_add_to_discard(&tg, muse);
    tg.state.p1_live_won = 1;

    rb_move_live_to_success_and_handle_wins(&tg.state);
    CHECK(pending(&tg), "F: the replacement prompt is open");
    /* live.rs:1761 -- the replaced card is stamped into the DEDICATED field
     * the moment the prompt is raised. */
    CHECK_EQ(tg.state.queue.pending_success_replacement_card_id, crossroads,
             "F: live.rs:1761 stamps the replaced card into pending_success_replacement_card_id");

    /* THE CLOBBER, reproduced deterministically. `resume_host` is shared with
     * every other resume path, and anything that runs between the prompt and
     * the answer overwrites it. Stamp a different card into it -- exactly the
     * shape the corpus measured (prompt_original=293, result_original=494) --
     * and the answer must still replace CROSSROADS. */
    tg.state.queue.resume_host = muse;

    pick(&tg, 0);
    CHECK(test_zone_has_id(&tg, 0, "discard", crossroads),
          "F: CROSSROADS is the card replaced -- a clobbered resume_host must not change that "
          "(turn/actions/mod.rs:894 reads pending_success_replacement_card_id, not a shared slot)");
    CHECK(test_zone_has_id(&tg, 0, "success", muse),
          "F: the chosen waitroom live reaches the success zone");
    /* mod.rs:894 `.take()` leaves None behind. */
    CHECK_EQ(tg.state.queue.pending_success_replacement_card_id, -1,
             "F: mod.rs:894 .take() consumes the slot -- a second read sees None");
}

'''

def rw(path, old, new, count=1):
    with io.open(path, 'r', encoding='utf-8', newline='') as f:
        s = f.read()
    n = s.count(old)
    if n != count:
        print("ANCHOR COUNT %d != %d in %s -- ABORT" % (n, count, path))
        sys.exit(1)
    s = s.replace(old, new, count)
    with io.open(path, 'w', encoding='utf-8', newline='') as f:
        f.write(s)
    print("OK %s" % path)

rw(PATH,
"""static void debut_reveal_success_zone_replacement_preserves_target_identity_dreamin(void)""",
TEST.lstrip('\n') + """static void debut_reveal_success_zone_replacement_preserves_target_identity_dreamin(void)""")

rw(PATH,
"""    run("E_debut_dreamin",       debut_reveal_success_zone_replacement_preserves_target_identity_dreamin);""",
"""    run("F_dedicated_slot",      live_phase_replacement_carries_the_card_in_the_dedicated_slot);
    run("E_debut_dreamin",       debut_reveal_success_zone_replacement_preserves_target_identity_dreamin);""")
