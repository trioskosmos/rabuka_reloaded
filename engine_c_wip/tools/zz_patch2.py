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
    print("OK %s" % path)

# ── choice.c : read the route misc.c stamped instead of hardcoding it ───────
rw("src/ability/choice.c",
"""    if (!strcmp(tgt, "position|destination") || !strcmp(tgt, "position_destination")) {
        /* choice.rs:2710-2717 \u2014 a `position|destination` answer whose entry
           carries `ChoiceRoute::Raw("position_change\u2026")` is routed to
           handle_position_change_choice BEFORE the SelectTargetKind match, and
           handle_position_change_choice is what calls
           execute_position_change_with_destination (choice.rs:3208). The C
           stores no choice_card_no, but misc.c `h_execute_position_change` is
           the ONLY producer of this target string and it is the port of
           position.rs:91 `execute_position_change`, which always stamps
           `Raw("position_change\u2026")` on the entry before raising the prompt
           (position.rs:143/197/266/367/419/480/546). So the route is
           unconditional here, and "position_change:self" is the spelling
           position.rs:480 writes for the destination-less step. */
        rb_resolver_handle_position_change_choice(self, g, "position_change:self", sel);
        return;
    }""",
"""    if (!strcmp(tgt, "position|destination") || !strcmp(tgt, "position_destination")) {
        /* choice.rs:2819-2826 \u2014 `Some(ChoiceRoute::Raw(s)) if
           s.starts_with("position_change") => return self
           .handle_position_change_choice(gs, choice_card_no, selected)`. The
           route is the ENTRY's `choice_card_no`, and it is the ONLY thing that
           tells handle_position_change_choice WHICH seat and WHICH source
           position the answer applies to (choice.rs:3119-3155 splits it back
           into target / explicit_source_pos).

           This arm used to pass the LITERAL "position_change:self", which
           discards both halves: the tail misc.c already stamped on the entry
           (`position_change:{target}:{tail}`, position.rs:151/273/427) and the
           real target. `explicit_source_pos` therefore stayed NULL and the
           answer fell to the fallback at the bottom of
           rb_resolver_handle_position_change_choice, which needs the ACTIVATING
           member \u2014 but `resume_eff` is NULL on this path and `s_activating_card`
           has already been cleared, so NOTHING moved and the swap was silently
           dropped.

           The entry's route string is read from the pair the header reserves for
           it (`has_choice_route_raw` / `choice_route_raw`, ability_queue.c:466).
           The fallback keeps the old literal for a prompt raised with no current
           entry, which is the only way that stamp can be absent. */
        char route_buf[80];
        const char *route = "position_change:self";
        int cur = g->queue.cur;
        if (cur >= 0 && cur < g->queue.n_entries) {
            const RbQueueEntry *en = &g->queue.entries[cur];
            if (en->has_choice_route_raw && en->choice_route_raw[0]) {
                snprintf(route_buf, sizeof(route_buf), "%s", en->choice_route_raw);
                route = route_buf;
            }
        }
        rb_resolver_handle_position_change_choice(self, g, route, sel);
        return;
    }""")

# ── misc.c : there must BE an entry for the route to land on ───────────────
# ── misc.c : both_master is DEFINED at :2212, below this call site at :1549 ─
rw("src/ability/effects/misc.c",
"""static int position_destination_prompt(GameState *g, int actor, const AbilityEffect *e,
                                       const char *route_target, const char *route_tail)""",
"""/* Defined at :2212, below this call site; forward-declared so the entry-minting
 * below can name the ability MASTER the way misc.c:2334 does. */
static int both_master(const GameState *g, int actor);

static int position_destination_prompt(GameState *g, int actor, const AbilityEffect *e,
                                       const char *route_target, const char *route_tail)""")

rw("src/ability/effects/misc.c",
"""    char raw[64];
    snprintf(raw, sizeof(raw), "position_change:%s:%s",
             (route_target && *route_target) ? route_target : "self",
             route_tail ? route_tail : "");
    rb_queue_set_choice_route_raw(g, raw);""",
"""    char raw[64];
    snprintf(raw, sizeof(raw), "position_change:%s:%s",
             (route_target && *route_target) ? route_target : "self",
             route_tail ? route_tail : "");
    /* THE MISSING ENTRY, and it is the same close misc.c already does at :2348.
     * `rb_queue_set_choice_route_raw` is a NO-OP when there is no current entry
     * (ability_queue.c:468-469), and Rust's `current_entry_mut()` at
     * position.rs:151 / :273 / :427 is never None because Rust reaches these
     * lines from inside execute_effect, which always runs attached to an entry
     * (queue.rs:311-409 mints a dummy for the Idle state). The C reaches the
     * same line with n_entries == 0 -- the resolver's own RESUME_CAPTURE line
     * reports `cur=0 n=0` on this path -- so the route was silently dropped,
     * the answer arrived with no `position_change` prefix at all, and
     * `explicit_source_pos` stayed NULL.
     *
     * Minting is SAFE here and only here: rb_queue_ensure_current_entry only
     * mints when `cur` is out of range, which is precisely the case where there
     * is nothing behind `cur` to orphan. live.c:1598-1603 records the measured
     * cost of minting UNCONDITIONALLY (10 lines in parity_sequential). */
    {
        int master = both_master(g, actor);
        rb_queue_ensure_current_entry(g, master == 0 ? "p1" : "p2");
    }
    rb_queue_set_choice_route_raw(g, raw);""")
