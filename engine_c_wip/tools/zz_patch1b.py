import io, sys

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
    print("OK %s (%d replacement(s))" % (path, count))

rw("src/engine.c",
"""     g->baton_touch_arriving_card_id = -1;
     for (int i = 0; i < 64; i++) g->gained_card_ids[i] = -1;""",
"""     g->baton_touch_arriving_card_id = -1;
     /* Rust: `pending_success_replacement_card_id: Option<i16>` is None on a new
      * game. memset leaves 0, and 0 is a LEGAL card index, so the -1 sentinel
      * has to be written explicitly or every fresh game claims card 0. */
     g->queue.pending_success_replacement_card_id = -1;
     for (int i = 0; i < 64; i++) g->gained_card_ids[i] = -1;""")

rw("src/test_game.c",
"""     tg->state.baton_touch_arriving_card_id = -1;
     for (int i = 0; i < 64; i++) tg->state.gained_card_ids[i] = -1;""",
"""     tg->state.baton_touch_arriving_card_id = -1;
     /* Option<i16> = None. memset leaves 0 and 0 is a legal card index. */
     tg->state.queue.pending_success_replacement_card_id = -1;
     for (int i = 0; i < 64; i++) tg->state.gained_card_ids[i] = -1;""")

rw("src/ability/ability_queue.c",
"""    q->selected_heart_color = -1;
}

static void entry_clear_pending_actions""",
"""    q->selected_heart_color = -1;
    /* Option<i16> = None; 0 is a legal card index. */
    q->pending_success_replacement_card_id = -1;
}

static void entry_clear_pending_actions""")

rw("src/ability/ability_queue.c",
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
   State queries""")
