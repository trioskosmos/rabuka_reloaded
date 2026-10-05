import io, sys

# FORWARD re-application of the same six hunks zz_unpatch.py reverts.
HUNKS = [
 ("src/turn/live.c",
"""    g->queue.resume_actor = player_pl;
    g->queue.resume_host = card_id;""",
"""    g->queue.resume_actor = player_pl;
    /* live.rs:1761 `game_state.pending_success_replacement_card_id = Some(card_id)`.
     *
     * The replaced card is carried in the DEDICATED field, exactly as Rust does,
     * and NOT in the shared `resume_host`. `resume_host` is still written on the
     * line below because other resume paths read it legitimately and its meaning
     * is unchanged; what changed is that the success-replacement RESUME no longer
     * DEPENDS on it. It was measured clobbered between this prompt being raised
     * and the answer arriving -- 7 of 13 prompt/result pairs on the corpus carried
     * prompt_original=293 but result_original=494 -- so the replacement was
     * applied to the wrong card. Rust's `Option<i16>` is `-1` here, and 0 is a
     * LEGAL card index, so presence is tested with `>= 0`, never `!= 0`. */
    g->queue.pending_success_replacement_card_id = card_id;
    g->queue.resume_host = card_id;"""),

 ("src/ability/choice.c",
"""        rb_handle_success_replacement_choice(g, pl, host, physical_index, !was_skip);""",
"""        /* turn/actions/mod.rs:894 -- `if let Some(replaced_card_id) =
         * game_state.pending_success_replacement_card_id.take()`. The `.take()`
         * is load-bearing: it LEAVES None behind, so a second read of the same
         * prompt cannot see the card again. Read from the DEDICATED field, not
         * from the shared `resume_host` (whose value is whatever the last resume
         * path left there). The `host` fallback can only be reached if some other
         * producer raised target="success_replacement" without stamping the slot,
         * which is not a shape this tree has; it keeps the pre-fix behaviour for
         * it rather than silently replacing card 0. */
        int replaced_card_id = g->queue.pending_success_replacement_card_id;
        g->queue.pending_success_replacement_card_id = -1;   /* Option::take() */
        if (replaced_card_id < 0) replaced_card_id = host;
        rb_handle_success_replacement_choice(g, pl, replaced_card_id, physical_index, !was_skip);"""),

 ("src/engine.c",
"""     g->baton_touch_arriving_card_id = -1;
     for (int i = 0; i < 64; i++) g->gained_card_ids[i] = -1;""",
"""     g->baton_touch_arriving_card_id = -1;
     /* Rust: `pending_success_replacement_card_id: Option<i16>` is None on a new
      * game. memset leaves 0, and 0 is a LEGAL card index, so the -1 sentinel
      * has to be written explicitly or every fresh game claims card 0. */
     g->queue.pending_success_replacement_card_id = -1;
     for (int i = 0; i < 64; i++) g->gained_card_ids[i] = -1;"""),

 ("src/test_game.c",
"""     tg->state.baton_touch_arriving_card_id = -1;
     for (int i = 0; i < 64; i++) tg->state.gained_card_ids[i] = -1;""",
"""     tg->state.baton_touch_arriving_card_id = -1;
     /* Option<i16> = None. memset leaves 0 and 0 is a legal card index. */
     tg->state.queue.pending_success_replacement_card_id = -1;
     for (int i = 0; i < 64; i++) tg->state.gained_card_ids[i] = -1;"""),

 ("src/ability/ability_queue.c",
"""    q->selected_heart_color = -1;
}

static void entry_clear_pending_actions""",
"""    q->selected_heart_color = -1;
    /* Option<i16> = None; 0 is a legal card index. */
    q->pending_success_replacement_card_id = -1;
}

static void entry_clear_pending_actions"""),

 ("src/ability/ability_queue.c",
"""    q->selected_heart_color = -1;
}

/* ==========================================================================
   State queries""",
"""    q->selected_heart_color = -1;
    /* core/game_state/mod.rs:433-434 -- concede also clears
     * `pending_success_replacement_card_id`, and this memset is that clear. */
    q->pending_success_replacement_card_id = -1;
}

/* ==========================================================================
   State queries"""),
]

ok = True
for path, old, new in HUNKS:
    with io.open(path, 'r', encoding='utf-8', newline='') as f:
        s = f.read()
    n = s.count(old)
    if n != 1:
        print("ANCHOR COUNT %d != 1 in %s -- ABORT" % (n, path)); ok = False; continue
    with io.open(path, 'w', encoding='utf-8', newline='') as f:
        f.write(s.replace(old, new, 1))
    print("REPATCHED %s" % path)
sys.exit(0 if ok else 1)
