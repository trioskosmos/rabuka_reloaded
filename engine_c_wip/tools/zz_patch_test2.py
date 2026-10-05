import io, sys

PATH = "tests/test_parity_zones_position.c"

TEST = r'''
/* =======================================================================
 * THE `position|destination` ANSWER IS ROUTED BY THE ENTRY'S RAW ROUTE.
 *
 * Rust: engine/src/ability/choice.rs:2819-2826
 *
 *     Some(ChoiceRoute::Raw(s)) if s.starts_with("position_change") => {
 *         return self.handle_position_change_choice(gs, choice_card_no, selected);
 *     }
 *
 * and handle_position_change_choice splits that route back into the seat and
 * the SOURCE position -- choice.rs:3119-3155, `tgt.strip_prefix
 * ("position_change:")`, `explicit_source_pos = Some(parts[1])` when
 * `stage_position_index(parts[1]).is_some()`.
 *
 * The C passed the LITERAL "position_change:self" instead, which discards both
 * the stamped tail (position.rs:151 / :273 / :427 write
 * `position_change:{target}:{tail}`) and the real target. So
 * `explicit_source_pos` stayed NULL and the answer fell through to the
 * fallback at the bottom of rb_resolver_handle_position_change_choice, which
 * needs the ACTIVATING member -- and on this path `resume_eff` is NULL and
 * `s_activating_card` is already cleared, so nothing moved.
 *
 * Asserted here is what Rust asserts, at the seam Rust reads it: the route
 * string is on the ENTRY, it carries the stamped tail, and answering moves the
 * member that tail names.
 * ======================================================================= */
static void test_position_destination_answer_uses_the_entry_route(void)
{
    /* 「 中央のメンバーを右サイドエリアにポジションチェンジ 」 — the
     * source_position arm, position.rs:427-431. */
    static TestGame tg;
    test_game_new(&tg);
    clear_p1(&tg);
    int src = test_new_id(&tg, "PL!S-bp5-111-R");   /* the member being moved */
    int other = test_new_id(&tg, "PL!S-bp5-111-R");
    int filler = test_new_id(&tg, "PL!-sd1-010-SD");
    if (src < 0 || other < 0 || filler < 0) {
        CHECK(0, "route fixtures resolve");
        return;
    }
    fill_decks(&tg, filler, 20);
    test_give_energy(&tg, 10);
    tg.state.p[0].stage[AREA_LEFT]   = filler;
    tg.state.p[0].stage[AREA_CENTER] = src;
    tg.state.p[0].stage[AREA_RIGHT]  = other;

    test_activate_ability(&tg, src);
    CHECK(rb_has_pending_choice(&tg.state),
          "the source-position arm raises a position|destination prompt");

    /* position.rs:427-431 stamps the route on the ENTRY before the prompt. */
    int cur = tg.state.queue.cur;
    CHECK(cur >= 0 && cur < tg.state.queue.n_entries,
          "there IS a current entry to carry the route (Rust always has one)");
    if (cur < 0 || cur >= tg.state.queue.n_entries) return;
    const RbQueueEntry *en = &tg.state.queue.entries[cur];
    CHECK(en->has_choice_route_raw,
          "the entry carries a Raw choice route (queue.rs:340-342 reads it)");
    if (en->has_choice_route_raw) {
        CHECK(strncmp(en->choice_route_raw, "position_change:", 17) == 0,
              "the route is the position_change form choice.rs:2819 matches on");
        /* The tail is the SOURCE position, and it is what becomes
         * explicit_source_pos at choice.rs:3147-3148. */
        const char *tail = strrchr(en->choice_route_raw, ':');
        CHECK(tail && rb_stage_position_index(tail + 1) >= 0,
              "the route's tail names a real source position, so it is not discarded");
    }

    /* Now answer. Source is center, so the options are [left, right]; pick the
     * right one. If the route was discarded, explicit_source_pos is NULL and the
     * fallback finds no activating card, and NOTHING moves -- which is the
     * defect this test exists to catch. */
    rb_resume_with_choice(&tg.state, 1);
    drain_all(&tg, 6);
    CHECK_EQ(tg.state.p[0].stage[AREA_RIGHT], src,
             "the member the stamped tail named lands on the chosen destination "
             "(choice.rs:3147-3148 feeds explicit_source_pos from the route)");
    CHECK_EQ(tg.state.p[0].stage[AREA_CENTER], other,
             "the displaced member takes the vacated source position");
}

'''

def rw(path, old, new, count=1):
    with io.open(path, 'r', encoding='utf-8', newline='') as f:
        s = f.read()
    n = s.count(old)
    if n != count:
        print("ANCHOR COUNT %d != %d in %s -- ABORT" % (n, count, path))
        sys.exit(1)
    with io.open(path, 'w', encoding='utf-8', newline='') as f:
        f.write(s.replace(old, new, 1))
    print("OK %s" % path)

rw(PATH, "int main(void)\n{", TEST.lstrip('\n') + "int main(void)\n{")

rw(PATH,
"""    test_both_position_change_opponent_pass_reads_opponent_stage();
    rb_unload();""",
"""    test_both_position_change_opponent_pass_reads_opponent_stage();
    test_position_destination_answer_uses_the_entry_route();
    rb_unload();""")
