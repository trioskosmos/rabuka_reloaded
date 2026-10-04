#include "rabuka.h"
#include <string.h>
#include <stdlib.h>
#include <stdio.h>

/* zones.rs::check_effect_position — declared in src/core/zones.c but not yet
   surfaced in rabuka.h; declared locally so this translation unit links. */
int rb_check_effect_position(const char *effect_pos, int card_position);

/* src/core/modifiers.c:715 rb_recalculate_constant_cost_modifiers — the
   faithful port of Rust's
   recalculate_constant_cost_modifiers_with_ids (modifiers.rs:1439-1654). Also
   defined but not yet surfaced in rabuka.h, so it is declared here for the same
   reason and with the same precedent as rb_check_effect_position above. It is
   NOT static in modifiers.c, so no change there is required to call it. */
void rb_recalculate_constant_cost_modifiers(GameState *g);

/* E1, ported in src/core/modifiers.c (Rust
   evaluate_success_zone_constant_modifiers, modifiers.rs:2188-2305). Also not
   yet surfaced in rabuka.h, so it is declared here for the same reason and with
   the same precedent as rb_recalculate_constant_cost_modifiers above. It is NOT
   static in modifiers.c, so no change there is required to call it. */
void rb_evaluate_success_zone_constant_modifiers(GameState *g);

/* E3 + E4, ported in src/core/modifiers.c (Rust
 *   evaluate_success_zone_constant_abilities  modifiers.rs:2147-2150
 *   restore_performance_need_heart_modifiers   modifiers.rs:2159-2182).
 * Like E1 above, neither is surfaced in rabuka.h, so they are declared locally
 * for the same reason and with the same precedent. Neither is static in
 * modifiers.c, so no change there is required to call them. */
void rb_evaluate_success_zone_constant_abilities(GameState *g);
void rb_restore_performance_need_heart_modifiers(GameState *g);

/* src/ability/effects/rules.c:2291 rb_rules_cannot_live_player — the reader for
   `RbRulesState::cannot_live_players[2]`, written by the `cannot_live` arm of
   rb_rules_execute_restriction (rules.c:1922-1925) and folded to Rust's per-turn
   set by the turn stamp in the same file. Defined but not yet surfaced in
   rabuka.h, so it is declared here for the same reason and with the same
   precedent as rb_check_effect_position and
   rb_recalculate_constant_cost_modifiers above. It is NOT static in rules.c, so
   no change there is required to call it. */
int rb_rules_cannot_live_player(GameState *g, int pl);

/* Defined below, next to the faithful live-success port; every scan that
   reproduces a Rust `check_effect_position` gate needs it. */
static const char *effect_activation_position_any(const AbilityEffect *e);

/* Mirror engine/src/core/card.rs:3753
       pub fn get_baton_touch_trigger(&self) -> Option<bool> {
           self.common().and_then(|c| c.baton_touch_trigger)
       }
   read off the TOP-LEVEL condition node of an effect, NON-recursively --
   Rust asks the condition node for its own `common()` field and never walks
   into children, so a flag that lives only on a child of a compound does NOT
   gate at this scan (it is still evaluated normally by the condition gate).
   Tag handling is identical to condition.c's get_bool: RB_TAG_TRUE / FALSE,
   plus the string spellings the wire also uses. */
static int effect_condition_baton_touch_trigger(const AbilityEffect *e) {
    if (!e || !e->condition) return 0;
    const Condition *c = e->condition;
    for (uint32_t i = 0; i < c->n_fields; i++) {
        const CondField *f = &c->fields[i];
        if (!f->key || strcmp(f->key, "baton_touch_trigger")) continue;
        if (f->v.tag == RB_TAG_TRUE)  return 1;
        if (f->v.tag == RB_TAG_FALSE) return 0;
        if (f->v.tag == RB_TAG_STR && f->v.s) return !strcmp(f->v.s, "true");
        return 0;
    }
    return 0;
}

/* Portable trigger scan — mirrors engine/src/triggers.rs:canonical_trigger
   + engine/src/turn/triggers.rs . Wire trigger strings are Japanese:
   "登場" (Debut), "ライブ開始時", "ライブ成功時", "常時", "起動", "自動" */

int rb_trigger_is(const char *triggers, const char *needle) {
    if (!triggers || !needle) return 0;
    return strstr(triggers, needle) != NULL;
}

/* Rust turn/triggers.rs drives every trigger through
 * `GameState::trigger_auto_ability(ability_id, trigger, player_id, ...)`, which
 * builds the AbilityQueueEntry with `player_id: player_id.to_string()`. The C
 * drain path is owner-scoped: `rb_process_player_abilities`
 * (src/core/game_state_abilities.c) collects the available entries whose
 * `player_id` equals "p1"/"p2" and, when nothing matches, clears the queue
 * outright. `rb_queue_push_with_trigger` leaves `player_id` empty, so an entry
 * pushed that way is invisible to the owner-scoped drain and is dropped without
 * ever resolving. Enqueue through `rb_queue_enqueue` (which stamps the owner)
 * and set the trigger token separately, exactly as
 * `rb_build_ability_queue_entry` does for the 自動 scan.

 * `moved` / `n_moved` is the ENQUEUE-TIME movement batch, and it is NOT the
 * same value at every call site. Rust decides per scan:
 *
 *   trigger_debut_abilities        trigger_scan.rs:148 takes
 *                                  `let moved_snapshot = game_state
 *                                   .recently_moved_cards.clone()` BEFORE the
 *                                  loop and passes `moved_snapshot.clone()` to
 *                                  EVERY enqueued 登場 ability (:156, :166,
 *                                  :180, :190, :205, :215, :221). So every 登場
 *                                  entry carries the batch that was live when
 *                                  the member appeared.
 *   trigger_live_start_abilities   trigger_scan.rs:345-346 passes `None`.
 *   trigger_live_success_abilities trigger_scan.rs:525-526 passes `None`.
 *   自動 / TAS scan                abilities.rs:1437 passes `moved`; the C's
 *                                  builder at game_state_abilities.c:1694-1700
 *                                  already reproduces that one.
 *
 * The snapshot is load-bearing, not decoration:
 * `rb_entry_trigger_moved_cards` (game_state_abilities.c:1614) is the FIRST
 * source `moved_card_pool` / `those_cards` conditions consult (condition.c
 * :1194-1200, :3203-3210) and it PREFERS the snapshot over the live global,
 * while `rb_drain_ability_queue` zeroes `g->n_recently_moved` at
 * src/ability/ability_queue.c:1420 before the cost runs. So by the time a 登場
 * effect asks "those cards", the live global is already empty and the entry
 * snapshot is the only surviving evidence that this card's move caused the
 * trigger. Passing NULL from the 登場 scan (as this helper used to for all three
 * scans) left 3 登場 abilities with `source: "those_cards"` and up to 22 with
 * `type: "movement_condition"` resolving against an EMPTY set where Rust sees
 * the batch — a real divergence on real cards, not a test artefact. */
static int rb_push_owned_entry(GameState *g, int pl, int card_id, int ability_idx,
                               const char *trigger,
                               const int *moved, int n_moved)
{
    if (!g || pl < 0 || pl > 1) return 0;
    int idx = rb_queue_enqueue(g, card_id, ability_idx, ability_idx,
                               (pl == 0) ? "p1" : "p2", NULL);
    if (idx < 0) return 0;
    rb_queue_set_entry_trigger(g, idx, trigger, moved, n_moved);
    return 1;
}

/* NOTHING IS RECORDED HERE, and that is now faithful.
   An enqueue records nothing in Rust. All three enqueue paths are pure
   builders — `enqueue_printed_ability` (abilities.rs:1282-1326),
   `enqueue_gained_ability` (:1331-1376) and
   `trigger_auto_ability_by_index_refs` (:1403-1449) — and each ends in a bare
   `self.ability_queue.enqueue(entry)`. `use_limit_recorded` is constructed
   false in all four constructors (queue.rs:391 / :490 / :684, abilities.rs:270)
   and is written in exactly ONE place, `record_ability_use`
   (abilities.rs:216-231), which is reached from eleven call sites, every one
   of them resolution-time: the resolver's Early record (resolver.rs:1180 /
   :1230) and Final record (:1390), the gates (gates.rs:160 / :308 / :318 /
   :331), the choice resume paths (choice.rs:3479 / :3663) and
   turn/actions/mod.rs:1529.

   The `rb_record_use` + `rb_mark_enqueued_entry_use_limit_recorded` pair that
   used to follow each push below was an interim shape that spent the
   turn-limited use at ENQUEUE. It is gone, and the two resolution-time records
   in src/ability/ability_queue.c (the Early record at :1423-1432 and the Final
   record at :1575-1584, both through `drain_record_ability_use` ->
   `rb_record_ability_use`) plus the owner-scoped drain's own records in
   pca_resolve_entry (game_state_abilities.c:2789-2796 and :2817-2820) now own
   it, exactly as they do in Rust.

   It had to be all of it at once. With the flag pre-set but the record still
   made here, the resolution-time gate at game_state_abilities.c:2710
   (`if (ab->use_limit > 0 && !entry->use_limit_recorded)`) became permanently
   unreachable for these entries, so a 「{{ターン1回}}」 ability enqueued by a
   scan was admitted whether or not its use was spent, and — because the
   drain's records all early-return on a set flag — the use was never spent at
   all. Removing the flag WITHOUT removing this record is the half that
   regresses; both halves are gone together.

/* Per-scan (card_id, ability_idx) dedupe set — the C stand-in for Rust's
   `HashSet<(i16, usize)>`, which is scoped to ONE seat's ONE scan in every
   place the Rust uses it (turn/trigger_scan.rs:188 for live_start,
   turn/triggers.rs for the live-success seen set). A fixed array plus a linear
   probe is the port's idiom for it; the cap only bounds pathological inputs. */
#define RB_TRIGGER_SEEN_MAX 64
typedef struct { int card_id; int ability_idx; } RbTriggerSeen;

/* Returns 1 when the pair was NOT already present (and records it), 0 when it
   was a duplicate. Mirrors `seen.insert(pair)` — 0 means "already there". */
static int rb_trigger_seen_add(RbTriggerSeen *seen, int *n, int card_id, int ability_idx) {
    if (!seen || !n) return 1;
    for (int i = 0; i < *n; i++)
        if (seen[i].card_id == card_id && seen[i].ability_idx == ability_idx) return 0;
    if (*n >= RB_TRIGGER_SEEN_MAX) return 0;
    seen[*n].card_id = card_id;
    seen[*n].ability_idx = ability_idx;
    (*n)++;
    return 1;
}


/* Mirror triggers.rs:TriggerKind::from_token (triggers.rs:54-75): trim the
    token, match against the wire constants; unknown → RB_TK_COUNT (no kind).
    Whitespace handling mirrors Rust str::trim on both ends. */
RbTriggerKind rb_trigger_from_token(const char *s) {
    if (!s) return RB_TK_COUNT;
    while (*s == ' ' || *s == '\t' || *s == '\r' || *s == '\n') s++;
    size_t len = strlen(s);
    while (len > 0 && (s[len-1]==' '||s[len-1]=='\t'||s[len-1]=='\r'||s[len-1]=='\n')) len--;
    char tok[64];
    if (len >= sizeof(tok)) return RB_TK_COUNT;
    memcpy(tok, s, len); tok[len] = 0;
    if (!strcmp(tok, RB_TSTR_ACTIVATION))   return RB_TK_ACTIVATION;
    if (!strcmp(tok, RB_TSTR_AUTO))         return RB_TK_AUTO;
    if (!strcmp(tok, RB_TSTR_CONSTANT))     return RB_TK_CONSTANT;
    if (!strcmp(tok, RB_TSTR_DEBUT))        return RB_TK_DEBUT;
    if (!strcmp(tok, RB_TSTR_DEBUT_EN))     return RB_TK_DEBUT;
    if (!strcmp(tok, RB_TSTR_LIVE_START))   return RB_TK_LIVE_START;
    if (!strcmp(tok, RB_TSTR_LIVE_SUCCESS)) return RB_TK_LIVE_SUCCESS;
    if (!strcmp(tok, RB_TSTR_LIVE_SUCCESS_EN)) return RB_TK_LIVE_SUCCESS;
    if (!strcmp(tok, RB_TSTR_MAIN))         return RB_TK_MAIN;
    if (!strcmp(tok, RB_TSTR_BATON_TOUCH))  return RB_TK_BATON_TOUCH;
    return RB_TK_COUNT;
}

/* Queue all stage members' debut abilities for the player who just played.
     Scan ALL abilities for that card (cards can have debut+constant). Mirrors
     Rust Card.abilities:Vec<AbilityRef> via CARD_ABILITY_PAIRS.

   engine/src/turn/trigger_scan.rs::trigger_debut_abilities walks the three
   stage areas and, because the caller names the exact instance that debuted
   (`card_id_opt = Some(..)`, :54-57), only the area actually holding
   `card_id` can match. `area` below is that index; the two gates Rust applies
   per ability are reproduced here:

     * :90-98  `zones::check_effect_position(effect.activation_position_any(), area)`
     * :99-109 `!baton_touch_used && effect.condition.baton_touch_trigger`

   Rust receives `baton_touch_used` as an argument (phases.rs:1076 passes a
   literal `true` on the baton-touch arrival, phases.rs:1317 the play handler's
   flag). The C has no such argument and the signature is fixed by ~30 test
   call sites plus three engine call sites, so the flag is read from the state
   field the C's own baton-touch paths set: engine.c:1092 (single baton) and
   engine.c:955 (double baton) write `g->baton_touch_used[pl] = 1`, and
   engine.c:1566 clears it, so it is exactly "this play action was a baton
   touch".

   THERE IS NO GAINED ARM HERE, and that is now faithful.
   `trigger_debut_abilities` (trigger_scan.rs:23-139) walks
   `card.abilities.iter().enumerate()` and NOTHING ELSE -- no
   `gained_card_abilities` loop, so `trigger_auto_ability_by_index_refs` is
   only ever reached with a printed `ability_index`. The C used to call
   `queue_gained_trigger(g, pl, card_id, "登場")` here, firing a runtime-GAINED
   登場 ability that Rust never fires. That is the same divergence already
   removed from the ライブ開始時 scan below (see the note there); this is its
   登場 twin, and it was still live.

   MEASURED REACHABILITY: 0 of 985. `cards/abilities.json` has 24
   `gain_ability` / `gain_ability_from_source` effect nodes, and the distinct
   values of their `ability_gain_trigger` are 常時 (20 nodes) and ライブ成功時
   (2 nodes) -- never 登場. Four nodes mention 登場 inside their CONDITION
   text (an `appearance_condition` "『蓮ノ空』のメンバーが登場しており"), which
   is a condition, not the granted ability's trigger. So the removal is
   measurement-neutral by construction, exactly as the ライブ開始時 removal was,
   and it stops the C from diverging the day the parser does emit a gained
   登場 ability. */
int rb_trigger_debut(GameState *g, int pl, int card_id) {
    int area = -1;
    if (g && pl >= 0 && pl < 2)
        for (int s = 0; s < RB_STAGE_SIZE; s++)
            if (g->p[pl].stage[s] == card_id) { area = s; break; }
    int baton_touch_used = (g && pl >= 0 && pl < 2) ? g->baton_touch_used[pl] : 0;

    /* trigger_scan.rs:148 `let moved_snapshot = game_state.recently_moved_cards
       .clone()` -- taken ONCE, before the loop, and cloned into every entry. */
    const int *moved = g ? g->recently_moved : NULL;
    int n_moved = g ? g->n_recently_moved : 0;

    int n = rb_card_num_abilities((uint32_t)card_id);
    int queued = 0;
    for(int i=0;i<n;i++){
        Ability ab; if(!rb_decode_card_ability((uint32_t)card_id,i,&ab)) continue;
        if(rb_ability_has_trigger(&ab, RB_TK_DEBUT) &&
           !rb_ability_is_invalidated(g, card_id, "登場")){
             /* Both gates are Rust `continue`s on the SAME ability, applied
                AFTER the trigger match and BEFORE the use-limit test, so an
                ability they reject is never recorded as used either. They are
                guarded on `area >= 0` because Rust guards them with
                `position.is_some_and(..)` -- and because a caller may name a
                card that is not (yet) a stage member, where the C cannot know
                an area and must not invent one. */
             if (area >= 0 &&
                 !rb_check_effect_position(effect_activation_position_any(ab.effect), area)) {
                 rb_free_ability(&ab);
                 continue;
             }
             if (!baton_touch_used && effect_condition_baton_touch_trigger(ab.effect)) {
                 rb_free_ability(&ab);
                 continue;
             }
             if (!rb_use_limit_reached(&g->queue, card_id, i, ab.use_limit < 0 ? 99 : ab.use_limit, g->turn)) {
                 /* Owner-stamped: `pl` is the player who just played `card_id`
                    (engine.c:944 / ability.c:424 / move.c:1230 all pass the actor
                    as `pl`), which is the seat Rust's
                    trigger_auto_ability(..., player_id, ...) stamps. Pushing
                    un-stamped here made the entry invisible to the owner-scoped
                    drain. */
                 if (rb_push_owned_entry(g, pl, card_id, i, "登場", moved, n_moved))
                     queued++;
             }
        }
        rb_free_ability(&ab);
    }
    return queued;
}

int rb_trigger_live_start(GameState *g, int pl) {
    if (!g || pl < 0 || pl > 1) return 0;
    /* ── Q68: ライブ開始時 IS UNUSABLE FOR A 「ライブできない」 PLAYER ─────────
       THIS GATE IS A DELIBERATE DIVERGENCE FROM RUST, and the card text is what
       both sides are measured against.

       cards/cards.json, FAQ Q68 (2025.02.07), on PL!HS-bp2-014-N /
       PL!SP-bp1-001-P / PL!SP-bp1-001-R:
         「『ライブできない』状態のプレイヤーは、ライブカードセットフェイズで
           ライブカード置き場に手札のカードを裏向きで置くことはできますが、
           パフォーマンスフェイズで表向きにしたカードの中にライブカードが
           あったとしても、そのライブカードを含めて控え室に置きます。その結果、
           ライブカード置き場にライブカードが置かれていないため、ライブは行われません。
           （ライブ開始時の能力は使えず、エールも行いません）」
       Three requirements, and this gate is (b) of them: the abilities are
       UNUSABLE. Rust does not honour it -- engine/src/turn/trigger_scan.rs:186-212
       `trigger_live_start_abilities` gates only on is_trigger_suppressed (:195)
       and, for the second attacker, on an empty live-card zone (:207-212), and
       never consults `cannot_live_players` anywhere in the file. Rust reads that
       set for the first time at phases.rs:351, i.e. AFTER this scan (phases.rs:
       245-246) and inside the yell, where live.rs:2053-2055 discards the live
       cards. So Rust fires a cannot_live player's ライブ開始時 and then throws
       their live cards away. No compiled Rust test adjudicates it:
       engine/tests/.../pvp_room_test.rs is the only oracle for the sibling
       field and does not compile (build.rs:90 EXCLUDED_MODULES).

       The C does not inherit that. The writer (rules.c:1922-1925) and the
       per-turn fold (rules.c rb_rules_cannot_live_player) are already in place,
       so this is the one line that makes the set load-bearing.

       Placement: before the suppression test, because both are side-effect-free
       early returns and Rust's order between them is unobservable. It is NOT
       placed after the empty-live-zone guard, which is Rust's own last gate. */
    if (rb_rules_cannot_live_player(g, pl)) return 0;
    if (rb_is_trigger_suppressed(g, pl, "live_start")) return 0;
    /* Rust trigger_scan.rs:202-212 --
         let player = ...;
         if game_state.current_phase == Phase::FirstAttackerPerformance
             && !player.is_first_attacker
             && player.live_card_zone.cards.is_empty()
         { return; }
       Q72: the live-card set phase can be completed with no member on stage, so
       the SECOND attacker whose live window held no card performs with an empty
       live-card zone -- and that performer's ライブ開始時 abilities are NOT
       scanned. The C had no equivalent guard, so a performer with an empty live
       zone still fired every ライブ開始時 on their board.

       `RB_PHASE_PERFORMANCE` is Rust's Phase::FirstAttackerPerformance
       (test_game.h:169 names the mapping). `g->first_attacker` is the seat
       index of the first attacker (web_server.c:106 reads it for the same
       purpose), so `pl != g->first_attacker` is `!player.is_first_attacker`.
       Note the call site src/turn/phase.c:174-175 still holds
       RB_PHASE_LIVE_SET_SECOND at this point (phase.c:186 sets PERFORMANCE
       afterwards), so from the engine path this guard cannot fire; it is
       observable on the direct calls the suites make, which is where Rust's
       phase value is modelled. */
    if (g->phase == RB_PHASE_PERFORMANCE && pl != g->first_attacker &&
        g->p[pl].live.n == 0) return 0;
    int queued=0;
/* engine/src/turn/trigger_scan.rs:216-218 --
         // Track (card_id, ability_idx) to prevent the same card's ability from
         // firing twice when the card is both a stage member AND a live card.
         let mut seen: HashSet<(i16, usize)> = HashSet::default();
       Declared INSIDE this per-seat function and consulted at :282-286, so it
       is seat-scoped: it collapses the live_cards.chain(stage_cards) double
       visit of one card instance and nothing else. The C scan had no set at
       all, so a card sitting in BOTH the live-card zone and stage[0] queued
       its ライブ開始時 twice -- a printed +1 became 2, a +2 became 4.
       `seen.insert` happens only when the trigger actually matches, so the set
       records FIRED pairs, not visited cards, exactly as in Rust. */
    RbTriggerSeen seen[RB_TRIGGER_SEEN_MAX];
    int n_seen = 0;
    /* Live cards in the live-card zone — mirrors Rust's scan of
       player.live_card_zone.cards before the stage scan. Many LiveStart
       autos (e.g. sd1-022) live on the live card, not on a stage member. */
    for (int i = 0; i < g->p[pl].live.n; i++) {
        int cid = g->p[pl].live.cards[i];
        if (cid == RB_EMPTY_SLOT) continue;
        int n = rb_card_num_abilities((uint32_t)cid);
        for (int ai = 0; ai < n; ai++) {
            Ability ab; if(!rb_decode_card_ability((uint32_t)cid, ai, &ab)) continue;
            /* Rust trigger_scan.rs:259-286. The `position` gate
                 if position.is_some_and(|area| !check_effect_position(..)) { continue; }
               is NOT reached for a live-card-zone card (position == None), and the
               trigger test is `ability.has_trigger(TriggerKind::LiveStart)` --
               i.e. the PER-TOKEN classifier (triggers.rs:62-74), not a raw
               substring `contains`. rb_ability_has_trigger (card.rs:834) is that
               classifier, so the English spellings Rust accepts ("Debut" /
               "live_success") are accepted here too instead of only the Japanese
               needle. */
            if (rb_ability_has_trigger(&ab, RB_TK_LIVE_START) &&
                !rb_ability_is_invalidated(g, cid, "ライブ開始時")) {
                if (rb_trigger_seen_add(seen, &n_seen, cid, ai) &&
                    !rb_use_limit_reached(&g->queue, cid, ai, ab.use_limit<0?99:ab.use_limit, g->turn)) {
                     /* MEASURE, THEN PRINT. The push is above the fprintf, not
                        below it. It used to be below: the line printed a literal
                        `queued=1` while the real counter `queued` (declared at
                        the top of this function) was only incremented on success.
                        rb_push_owned_entry returns 0 on a bad seat and on
                        rb_queue_enqueue < 0, so every failed push was REPORTED
                        as a successful enqueue. That is the masked-failure class:
                        a diagnostic that asserts the ability fired when it did
                        not -- the same shape as the `ability_id[512]` bug the
                        J12 rows were opened for, and it was never caught because
                        the number it printed could not disagree with itself.

                        `int pushed` IS `queued`: 1 exactly when the entry reached
                        the queue, 0 otherwise, so the printed field and the
                        accumulated count can no longer drift apart. */
                     /* Owner-stamped: this loop scans g->p[pl].live, so the card
                        belongs to `pl`. Rust's live-start scan binds one
                        player_id_clone for the whole scan
                        (turn/triggers.rs:180, :309-317) and passes it to every
                        trigger_auto_ability, for BOTH the live-card zone and the
                        stage members. Records nothing (see rb_push_owned_entry).
                        The trigger name is \xNN-escaped per the repo rule for
                        fullwidth literals; the bytes are identical to the raw
                        UTF-8 ライブ開始時 they replace. */
                     int pushed = rb_push_owned_entry(g, pl, cid, ai,
                                                      "\xE3\x83\xA9\xE3\x82\xA4\xE3\x83\x96\xE9\x96\x8B\xE5\xA7\x8B\xE6\x99\x82",
                                                      NULL, 0);
                     if (pushed) queued++;
                    fprintf(stderr, "[LIVE_START_SCAN] pl=%d trigger=%s zone=%s label=%s cid=%d ab=%d queued=%d\n",
                            pl, rb_canonical_trigger(ab.triggers), rb_trigger_zone_id(0),
                            rb_trigger_zone_label(0), cid, ai, pushed);
                } else {
                    fprintf(stderr, "[LIVE_START_SCAN] pl=%d trigger=%s zone=%s label=%s cid=%d ab=%d queued=0 use_limit\n",
                            pl, rb_canonical_trigger(ab.triggers), rb_trigger_zone_id(0),
                            rb_trigger_zone_label(0), cid, ai);
                }
            }
            rb_free_ability(&ab);
        }
        /* No gained arm for the live-card zone either -- see the note after the stage
   loop below. */
    }
    /* Stage members — mirrors Rust's scan of player.stage. */
    for(int s=0;s<RB_STAGE_SIZE;s++){
        int cid=g->p[pl].stage[s];
        if(cid==RB_EMPTY_SLOT) continue;
        int n = rb_card_num_abilities((uint32_t)cid);
        for(int i=0;i<n;i++){
            Ability ab; if(!rb_decode_card_ability((uint32_t)cid,i,&ab)) continue;
            /* Rust trigger_scan.rs:259-286 again. Here `position` IS `Some(area)`,
               so the activation-position gate is LIVE -- and it runs BEFORE the
               trigger match and before `seen.insert(..)`, exactly as in Rust, so
               an ability rejected by the gate never claims a `seen` slot and is
               never recorded as used. `area` is the stage slot index, which is
               the MemberArea discriminant Rust derives here
               (trigger_scan.rs:233-237 `MemberArea::from_index(index)`).

               This gate was MISSING in the C: only the GAINED arm of this same
               scan (queue_gained_trigger_seen, triggers.c:124) consulted
               rb_check_effect_position, so a PRINTED ライブ開始時 ability stamped
               `activation_position` fired from every stage area instead of only
               the printed one -- the exact asymmetry that let a
               left/right-only ability go live from the centre. */
            int pos_ok = rb_check_effect_position(effect_activation_position_any(ab.effect), s);
            if (pos_ok && rb_ability_has_trigger(&ab, RB_TK_LIVE_START) &&
               !rb_ability_is_invalidated(g, cid, "ライブ開始時")){
                if(rb_trigger_seen_add(seen, &n_seen, cid, i) &&
                   !rb_use_limit_reached(&g->queue, cid, i, ab.use_limit<0?99:ab.use_limit,g->turn)){
                    /* MEASURE, THEN PRINT -- second site, same defect as the
                       live-card loop above. The push is above the fprintf so
                       `queued=%d` reports the RETURN VALUE of the push instead
                       of the literal 1; a push refused for a bad seat or a
                       negative rb_queue_enqueue now prints queued=0 and leaves
                       the accumulated `queued` untouched, which is what
                       rb_push_owned_entry actually did. */
                     /* Owner-stamped: same `pl` as the live-card loop above —
                        g->p[pl].stage, and Rust passes one player_id_clone for the
                        whole chained live_cards + stage_cards scan. Records
                        nothing (see rb_push_owned_entry). Trigger name
                        \xNN-escaped per the repo rule for fullwidth literals;
                        same bytes as the raw UTF-8 it replaces. */
                     int pushed = rb_push_owned_entry(g, pl, cid, i,
                                                      "\xE3\x83\xA9\xE3\x82\xA4\xE3\x83\x96\xE9\x96\x8B\xE5\xA7\x8B\xE6\x99\x82",
                                                      NULL, 0);
                     if (pushed) queued++;
                    fprintf(stderr, "[LIVE_START_SCAN] pl=%d trigger=%s zone=%s label=%s cid=%d ab=%d queued=%d\n",
                            pl, rb_canonical_trigger(ab.triggers), rb_trigger_zone_id(1),
                            rb_trigger_zone_label(1), cid, i, pushed);
                } else {
                    fprintf(stderr, "[LIVE_START_SCAN] pl=%d trigger=%s zone=%s label=%s cid=%d ab=%d queued=0 use_limit\n",
                            pl, rb_canonical_trigger(ab.triggers), rb_trigger_zone_id(1),
                            rb_trigger_zone_label(1), cid, i);
                }
            }
            rb_free_ability(&ab);
        }
    }
/* NO gained-ability arm here, and that is now faithful.
   Rust trigger_scan.rs:259-320 -- trigger_live_start_abilities -- walks
   `card.abilities` and nothing else. Unlike the ライブ成功時 scan
   (trigger_scan.rs:462-514) and the 起動 candidate list
   (game_setup.rs:2100-2118) it never consults `gained_card_abilities`, so a
   runtime-gained ライブ開始時 ability is never fired by this scan in Rust. The
   C ran `queue_gained_trigger_seen(.., "ライブ開始時", ..)` over both the live
   card and the three stage slots here, i.e. it fired what Rust does not. Removed:
   with no corpus card reaching this path in 19 suites the change is measurement
   NEUTRAL, and it removes the divergence rather than documenting it. */
    /* Process the queued abilities so their effects resolve (e.g., position change) */
    if (queued > 0) {
        rb_drain_ability_queue(g);
    }
    return queued;
}

/* Gate: only fire LiveSuccess when the live actually satisfied its heart
   requirements this turn (mirrors engine/src/core/game_state/abilities.rs:
   should_trigger_live_success — the live must have passed the heart check).
   In the C pipeline `passed` is recorded into g->live_success[pl] before the
   trigger block runs, so gating on it is equivalent to Rust's need_heart scan. */
int rb_should_trigger_live_success(const GameState *g, int pl) {
    if (!g) return 0;
    if (pl < 0 || pl > 1) return 0;
    if (g->p[pl].live.n == 0) return 0;
    return g->live_success[pl] ? 1 : 0;
}

/* Collected (card_id, ability_idx) pairs awaiting enqueue. Rust builds the
   `Vec<(String, String, i16)>` abilities_to_trigger first and only then loops
   it calling trigger_auto_ability (trigger_scan.rs:518-528); the two phases are
   kept separate here for the same reason, so the enqueue order is the COLLECT
   order. */
#define RBLSS_PENDING_MAX 128
typedef struct { int card_id; int ability_idx; } RbLssPending;

/* Mirror rb_trigger_live_start for the ライブ成功時 (LiveSuccess) trigger, but
   with the scan order and the dedupe set Rust uses for THIS trigger, which
   differ from live-start in both directions:

     engine/src/turn/trigger_scan.rs:386-395
         let stage_cards = [LeftSide, Center, RightSide].into_iter().enumerate()
                              .map(|(index, area)| (stage[index], Some(area)));
         let live_cards = live_card_zone.cards.iter().map(|&id| (id, None));
         for (card_id, position) in stage_cards.chain(live_cards) {

   i.e. STAGE FIRST, then the live-card zone -- the exact opposite of the
   live-start scan at :232-238 (`live_cards.chain(stage_cards)`). The two scans
   living in the same C file used to be mirror images of each other, which is
   how the inversion survived: only one of the two can match Rust.

     engine/src/turn/trigger_scan.rs:369  let mut seen: HashSet<(i16, usize)> = ...
     engine/src/turn/trigger_scan.rs:424-428
         if !ability.has_trigger(LiveSuccess) || !seen.insert((card_id, ability_index))
         { continue; }

   The set records FIRED pairs, not visited cards, and is shared with the gained
   arm (:479-482, ability_idx = GAINED_ABILITY_INDEX_BASE + gained_index). A card
   sitting in BOTH the stage and the live-card zone therefore fires exactly once
   -- this is the `+1 -> 4` / `+2 -> 8` double-apply that has been reported
   against the C for two days.

   The two Rust `continue` gates that precede the insert are reproduced too:
   the card-level invalidation test (:396-401, staged members only, because it
   is short-circuited by `position.is_some() &&`) and the per-ability
   activation-position test (:413-423, `position.is_some_and(..)`).

   Gated by rb_should_trigger_live_success (:372-378) and by the trigger
   suppression check, so it only fires on a successful live. */
int rb_trigger_live_success(GameState *g, int pl) {
    if (rb_ability_debug_enabled())
        fprintf(stderr, "[LIVE_SUCCESS_TRIGGER_ENTER] pl=%d state=%d pending=%d cur=%d n=%d\n",
                pl, g->queue.state, g->queue.has_pending, g->queue.cur, g->queue.n_entries);
    if (!g || pl < 0 || pl > 1) return 0;

    /* E3 + E4 -- engine/src/turn/trigger_scan.rs:357-365, the FIRST two
     * statements of Rust's trigger_live_success_abilities:
     *
     *     pub fn trigger_live_success_abilities(game_state: &mut GameState, player_id: &str) {
     *         // Evaluate constant modify_required_hearts abilities on cards in the
     *         // success_live_card_zone before checking live success conditions.
     *         game_state.evaluate_success_zone_constant_abilities();
     *
     *         // Restore performance-time need_heart_modifiers that were cleared above.
     *         // This preserves modifications from live_start triggers and other non-constant
     *         // sources, ensuring should_trigger_live_success uses the correct requirements.
     *         game_state.restore_performance_need_heart_modifiers();
     *
     *         ...
     *         if !game_state.should_trigger_live_success(..) { return; }
     *
     * ORDER IS LOAD-BEARING AND PUTS THEM BEFORE THE GATE: Rust evaluates and
     * restores first and only then tests should_trigger_live_success, which in Rust
     * re-reads the need_heart table (:372-378). So the pair must sit between the
     * NULL/bounds guard above and the rb_should_trigger_live_success call below --
     * moving them after the gate would skip the re-evaluation the gate is supposed
     * to be reading.
     *
     * E3 is the ONLY withdrawal of a plain `modify_required_hearts` need_heart
     * grant anywhere in the tree. Rust has no ledger for that path -- the
     * constant_need_heart prologue in rb_recalc_constants is
     * `modify_required_hearts_global`'s ledger (Rust constant_global_need_heart,
     * game_modifiers.rs:194), a different action -- so E1's need_heart arm
     * (sz_apply_effect -> rb_execute_modify_required_hearts) writes
     * mods.need_heart and nothing ever takes it back. Rust's
     * `need_heart_modifiers.clear()` at modifiers.rs:2148 is the withdraw, and it
     * runs only from these two live-flow sites.
     *
     * NOT wired into rb_recalc_constants, deliberately: Rust's
     * recalculate_constants calls E1 (modifiers.rs:1398), never E3, so a clear
     * there would be a rule Rust does not have. */
    rb_evaluate_success_zone_constant_abilities(g);
    rb_restore_performance_need_heart_modifiers(g);

    if (!rb_should_trigger_live_success(g, pl)) return 0;
    if (rb_is_trigger_suppressed(g, pl, "live_success")) return 0;

    const RbPlayer *P = &g->p[pl];
    RbTriggerSeen seen[RB_TRIGGER_SEEN_MAX];
    int n_seen = 0;
    RbLssPending pending[RBLSS_PENDING_MAX];
    int n_pending = 0;

    /* zone 0 == stage (position = Some(area)), zone 1 == live-card zone
       (position = None). */
    for (int zone = 0; zone < 2; zone++) {
        int count = (zone == 0) ? RB_STAGE_SIZE : P->live.n;
        for (int i = 0; i < count; i++) {
            int cid = (zone == 0) ? P->stage[i] : P->live.cards[i];
            int area = (zone == 0) ? i : -1;
            if (cid == RB_EMPTY_SLOT) continue;
            if (zone == 0 && rb_ability_is_invalidated(g, cid, RB_TSTR_LIVE_SUCCESS))
                continue;

            int nab = rb_card_num_abilities((uint32_t)cid);
            for (int a = 0; a < nab; a++) {
                Ability ab;
                if (!rb_decode_card_ability((uint32_t)cid, a, &ab)) continue;
                if (area >= 0 &&
                    !rb_check_effect_position(effect_activation_position_any(ab.effect), area)) {
                    rb_free_ability(&ab);
                    continue;
                }
                if (rb_ability_has_trigger(&ab, RB_TK_LIVE_SUCCESS) &&
                    rb_trigger_seen_add(seen, &n_seen, cid, a) &&
                    n_pending < RBLSS_PENDING_MAX) {
                    pending[n_pending].card_id = cid;
                    pending[n_pending].ability_idx = a;
                    n_pending++;
                }
                rb_free_ability(&ab);
            }

            /* gained abilities on the same card (Rust: the gained_card_abilities
               loop nested inside the card loop, :462-514). */
            int ng = rb_card_num_gained_abilities(g, cid);
            for (int gi = 0; gi < ng; gi++) {
                const Ability *gab = rb_card_gained_ability(g, cid, gi);
                if (!gab) continue;
                if (area >= 0 &&
                    !rb_check_effect_position(effect_activation_position_any(gab->effect), area))
                    continue;
                int ability_idx = RB_GAINED_ABILITY_INDEX_BASE + gi;
                if (rb_ability_has_trigger(gab, RB_TK_LIVE_SUCCESS) &&
                    rb_trigger_seen_add(seen, &n_seen, cid, ability_idx) &&
                    n_pending < RBLSS_PENDING_MAX) {
                    pending[n_pending].card_id = cid;
                    pending[n_pending].ability_idx = ability_idx;
                    n_pending++;
                }
            }
        }
    }

    /* trigger_auto_ability(..., AbilityTrigger::LiveSuccess, pl, ...) */
    int queued = 0;
    for (int i = 0; i < n_pending; i++) {
        int cid = pending[i].card_id;
        int idx = pending[i].ability_idx;
        if (idx >= RB_GAINED_ABILITY_INDEX_BASE) {
            /* The gained arm (trigger_scan.rs:465-514) has no use-limit gate at
               all — the printed arm's `ability_has_remaining_uses` lives in the
               TAS scan (abilities.rs:971), not here — and, like every Rust
               enqueue, records nothing. */
if (rb_push_owned_entry(g, pl, cid, idx, RB_TSTR_LIVE_SUCCESS, NULL, 0)) queued++;
            continue;
        }
        Ability ab;
        if (!rb_decode_card_ability((uint32_t)cid, idx, &ab)) continue;
        int limit = ab.use_limit < 0 ? 99 : ab.use_limit;
        /* `occurrence` used to be the index of this copy among the live-zone /
           stage duplicates of the same card id; it existed only to compensate
           for the double visit that `seen` now removes, so it is gone. */
        int key = (cid << 16) | (idx & 0xFFFF);
        int reached = rb_use_limit_reached(&g->queue, cid, idx, limit, g->turn);
        fprintf(stderr, "[LIVE_SUCCESS_TRIGGER] pl=%d owner=%d cid=%d ab=%d occurrence=%d use_limit=%d effect=%s options=%d\n",
                pl, rb_owner_of_card(g, cid), cid, idx, 0, ab.use_limit,
                ab.effect && ab.effect->action ? ab.effect->action : "none",
                ab.effect ? ab.effect->n_options : -1);
        fprintf(stderr, "[LIVE_SUCCESS_QUEUE] cid=%d ab=%d occurrence=%d limit=%d reached=%d completed=%d\n",
                cid, idx, 0, limit, reached, key == g->just_completed_ability_key);
        rb_free_ability(&ab);
        if (key == g->just_completed_ability_key || reached) continue;
        if (rb_push_owned_entry(g, pl, cid, idx, RB_TSTR_LIVE_SUCCESS, NULL, 0)) queued++;
    }
    return queued;
}

/* Mirror live.rs::drain_pending_live_success_choices. Re-entrantly drain the
   ability queue until it empties or a pending choice surfaces (the host
   resolver resumes and re-drains). Returns 1 if a pending choice was left
   unresolved (caller should yield to the host), 0 otherwise. */
int rb_drain_live_success_choices(GameState *g) {
    if (!g) return 0;
    int guard = 0;
    while (g->queue.n_entries > 0 && guard++ < 64) {
        rb_drain_ability_queue(g);
        if (rb_has_pending_choice(g)) return 1;
    }
    return 0;
}

/* Helper: apply a single effect as a constant modifier (no condition gate here
    beyond the caller's check). Mirrors the blade/heart/score/need_heart branches
    of engine/src/core/game_state/modifiers.rs:recalculate_constants . */
/* Mirror a stage position across the center line: left<->right, center stays. */
static const char *mirror_position(const char *pos) {
    if (!pos) return NULL;
    if (!strcmp(pos,"left_side"))  return "right_side";
    if (!strcmp(pos,"right_side")) return "left_side";
    return pos; /* center / other */
}
/* Return the card id occupying `pos` on player `pl`, or RB_EMPTY_SLOT. */
static int card_at_position(const GameState *g, int pl, const char *pos) {
    int idx = -1;
    if (!pos) return RB_EMPTY_SLOT;
    if (!strcmp(pos,"center")) idx = 1;
    else if (!strcmp(pos,"left_side")) idx = 0;
    else if (!strcmp(pos,"right_side")) idx = 2;
    else return RB_EMPTY_SLOT;
    return g->p[pl].stage[idx];
}

static int effect_is_live_end(AbilityEffect *e) {
    if (!e) return 0;
    for (int i=0;i<e->n_extra;i++) {
        if (e->extra_k[i] && !strcmp(e->extra_k[i],"duration") && e->extra_v[i]) {
            const char *v = e->extra_v[i];
            if (strstr(v,"live") || strstr(v,"ライブ")) return 1;
        }
    }
    return 0;
}

/* acc != NULL means "record deltas into this temporary effect" (for Duration::LiveEnd
   debut effects). The same code path applies the modifier to the live modifier table. */
static int effect_position_matches(const GameState *g, int pl, int card_id,
                                   const AbilityEffect *e) {
    if (!e) return 1;
    const char *required = NULL;
    for (int i = 0; i < e->n_extra; i++) {
        if (e->extra_k[i] && !strcmp(e->extra_k[i], "activation_position")) {
            required = e->extra_v[i];
            break;
        }
    }
    if (!required || !*required) return 1;
    const char *actual = NULL;
    for (int i = 0; i < RB_STAGE_SIZE; i++) {
        if (g->p[pl].stage[i] != card_id) continue;
        if (i == 0) actual = "left_side";
        else if (i == 1) actual = "center";
        else if (i == 2) actual = "right_side";
        break;
    }
    if (!actual) return 0;
    return strstr(required, actual) != NULL;
}

static int card_on_stage(const GameState *g, int pl, int card_id) {
    for (int i = 0; i < RB_STAGE_SIZE; i++)
        if (g->p[pl].stage[i] == card_id) return 1;
    return 0;
}

/* Last occurrence wins, matching eff_extra (ability/util.c). */
static const char *cextra(const AbilityEffect *e, const char *key) {
    if (!e || !key) return NULL;
    for (int i = 0; i < e->n_extra; i++)
        if (e->extra_k[i] && !strcmp(e->extra_k[i], key)) return e->extra_v[i];
    return NULL;
}

/* First comma-separated entry of an ARRAY extra, as a fresh string.
   engine/src/core/card.rs:2462 AbilityEffect::group_name() is exactly
   `group_names_any().and_then(|gn| gn.first() ..)`, i.e. the FIRST entry of the
   plural list, never the whole comma list. The C decoder stores an ARRAY extra
   as one comma-joined string (vm.c:1116-1122) and only mirrors a ONE-element
   array onto a singular key for `heart_colors` (vm.c:1120), so every other
   plural field must be split by hand. Caller frees. */
static char *cextra_first_entry(const AbilityEffect *e, const char *key) {
    const char *v = cextra(e, key);
    if (!v || !*v) return NULL;
    const char *comma = strchr(v, ',');
    size_t len = comma ? (size_t)(comma - v) : strlen(v);
    while (len > 0 && (v[len - 1] == ' ' || v[len - 1] == '\t')) len--;
    if (len == 0) return NULL;
    char *out = (char *)rb_malloc(len + 1);
    if (!out) return NULL;
    memcpy(out, v, len);
    out[len] = 0;
    return out;
}

/* Split the comma list an ARRAY extra decodes into, through the canonical
   colour parser, and return the number of entries written.
   engine/src/core/game_state/modifiers.rs:1006-1013 keys the constant-heart map
   by `hc.clone()` -- the RAW token of each list entry -- and divides the total
   by `hc_list.len()`; the C map is indexed by the parsed colour, and duplicates
   are KEPT because the divisor counts printed icons, not distinct colours
   (冬毬 PL!SP-bp5-011-R: "heart02,heart02,heart02" is THREE entries, one colour).
   Out-of-range parses (draw / score) are dropped: RbMods has 8 buckets and
   Rust's string key would never be read back by any colour query. */
static int const_heart_color_list(const char *list, int *out, int max) {
    int n = 0;
    if (!list || !*list) return 0;
    const char *p = list;
    while (*p && n < max) {
        const char *comma = strchr(p, ',');
        size_t len = comma ? (size_t)(comma - p) : strlen(p);
        while (len > 0 && (*p == ' ' || *p == '\t')) { p++; len--; }
        while (len > 0 && (p[len - 1] == ' ' || p[len - 1] == '\t')) len--;
        if (len > 0 && len < 32) {
            char tok[32];
            memcpy(tok, p, len);
            tok[len] = 0;
            int col = (int)rb_parse_heart_color(tok);
            if (col >= 0 && col < 8) out[n++] = col;
        }
        if (!comma) break;
        p = comma + 1;
    }
    return n;
}

/* Mirror AbilityEffect::per_unit_any (engine/src/ability/types.rs): the
   decoded `per_unit` field, or a `per_unit: "true"` extra. */
static int ceff_per_unit(const AbilityEffect *e) {
    if (!e) return 0;
    if (e->per_unit > 0) return 1;
    {
        const char *v = cextra(e, "per_unit");
        return v && !strcmp(v, "true");
    }
}

/* Per-unit UNIT COUNT for a constant gain, mirroring
   engine/src/core/game_state/modifiers.rs:736-765 (blade) and :886-913 (heart):
     let player = match effect.target_any() { Some("opponent") => other side, _ => host's own side };
     let units  = constant_per_unit_units(effect, player, ..., card_id);
   `pl` is the seat the host is on (rb_recalc_constants iterates both sides), so
   "the host's own side" is `pl` and `target: "opponent"` flips it. */
static int constant_gain_per_unit_units(const GameState *g, int pl, int host_cid,
                                        const AbilityEffect *e) {
    int units_pl = pl;
    const char *tgt = cextra(e, "target");
    if (tgt && !strcmp(tgt, "opponent")) units_pl = 1 - pl;
    return rb_constant_per_unit_units(e, g, units_pl, host_cid);
}

static void apply_constant_effect(GameState *g, int pl, int host_cid,
                                  AbilityEffect *e, RbTempEffect *acc,
                                  int16_t *total_p1, int16_t *total_p2) {
    if (!e || !e->action) return;
    /* acc != NULL means "record the deltas into this temporary effect" (the
       Duration::LiveEnd branch of modifiers.rs:recalculate_constants). The
       recorded duration comes from the effect's own `duration` field so
       rb_check_expired_effects reverts it at the right phase. */
    if (acc && acc->dur == 0)
        acc->dur = effect_is_live_end(e) ? RB_TEMP_LIVE_END : RB_TEMP_TURN_END;
    /* Position-targeted modifiers (ruby front / love_wing_bell center): the
       effect grants its resource to the member at a given stage position rather
       than the host. Mirrors engine/src/core/game_state/modifiers.rs constant
       application where the effect's `position`/`activation_position` select the
       recipient. */
    int tgt_cid = host_cid;
    int tgt_pl  = pl;
    const char *pos = NULL, *act_pos = NULL;
    for(int i=0;i<e->n_extra;i++){
        if(e->extra_k[i] && !strcmp(e->extra_k[i],"position")) pos=e->extra_v[i];
        if(e->extra_k[i] && !strcmp(e->extra_k[i],"activation_position")) act_pos=e->extra_v[i];
    }
    if (pos) {
        if (act_pos) {            /* front mechanic: affects OPPONENT at mirrored pos */
            tgt_pl = 1 - pl;
            pos = mirror_position(act_pos);
        } else {                  /* same-player position (e.g. own center) */
            tgt_pl = pl;
        }
        int c = card_at_position(g, tgt_pl, pos);
        if (c != RB_EMPTY_SLOT) tgt_cid = c;
    }
    /* NO `modify_cost` arm here, and that is now faithful.
       engine/src/core/game_state/modifiers.rs's constant dispatch
       (ActionType::GainResource :792, ModifyScore :1028, GainAbility :1118,
       GainAbilityFromSource :1202, ModifyRequiredHeartsGlobal :1209,
       Sequential :1250) has NO ModifyCost case, and commit_constant_results
       (:1375) is called with NO cost argument at all. Cost modifiers are
       owned end-to-end by the ONE dedicated pass Rust runs at modifiers.rs:1391-
       1392, `recalculate_constant_cost_modifiers_with_ids(&entries, &hand_ids)`,
       which is this file's rb_recalculate_constant_cost_modifiers — see the
       call at the tail of rb_recalc_constants below.

       The arm that used to live here was strictly weaker than that twin:
         * it read a NON-RUST `sign == "negative"` extra instead of the
           `operation` field, so every printed 「コストはN減る」 (operation
           "subtract") was applied as an ADD of N — e.g. the 17 常時
           modify_cost effects in the corpus are 12 "subtract", 4 "add", 1
           "set";
         * it had no set-to-absolute path at all;
         * it had no per_unit / per_unit_location / per_unit_count /
           exclude_self branch (modifiers.rs:1515-1582);
         * it carried no `effect.condition` gate of its own beyond the caller's
           (the twin evaluates it per ability, modifiers.rs:1503-1506);
         * it had no LL-bp7-001 exclusion (modifiers.rs:1478-1485), so
           「手札3枚捨てて10」 (operation "set", value 10, location hand,
           characters x3, optional) installed a permanent absolute cost of 10;
         * and it keyed the bonus on `tgt_cid`, which defaults to the HOST and
           is rewritten by the position block above, whereas the twin always
           keys it on the card the 常時 is printed ON (`expected.entry(cid)`,
           modifiers.rs:1594) — which is what makes the hand-card discounts work
           at all, since rb_recalc_constants' own scan walks stage + success +
           live and NEVER a hand card.
       Reimplementing any of that here would be a fifth copy of the twin; the
       twin is called once instead. */
    if (!strcmp(e->action,"modify_score")) {
        int cnt=e->count>=0?e->count:1;
        for(int i=0;i<e->n_extra;i++) if(e->extra_k[i] && !strcmp(e->extra_k[i],"value")){
            int v=atoi(e->extra_v[i]); if(v) cnt=v;
        }
        /* target="live_total" (parser-emitted for 「ライブの合計スコアを＋１する」)
           modifies the player's live TOTAL — route into the per-player
           accumulator (mirrors modifiers.rs:687-694). Keying under card_id
           could never match a live card and silently no-op'd. */
        const char *ms_target = e->target ? e->target : NULL;
        if (ms_target && !strcmp(ms_target, "live_total")) {
            /* card_on_stage() is NOT a semantic gate here -- it is Rust's
               `belongs_to_p1` ROUTING test, verbatim:

                   if effect.target_any() == Some("live_total") {
                       let belongs_to_p1 =
                           self.player1.stage.stage.contains(&card_id);
                       if belongs_to_p1 { p1_constant_score_bonus += sv; }
                       else            { p2_constant_score_bonus += sv; }
                   } else { *exp_score.entry(card_id).or_insert(0) += sv; }
                                     -- modifiers.rs:1048-1058

               Since rb_recalc_constants' scan is now STAGE-ONLY (see the zone
               collection above), every card reaching this line is on `pl`'s
               stage by construction and the test is always true. It was NOT
               always true before that narrowing: the scan also walked
               g->p[pl].success and g->p[pl].live, so 「ライブの合計スコアを+1する」
               was rejected outright for the 27 constant nodes that carry it
               (measured MS_TARGET_FIELD_LIVE_TOTAL 27; `target` is a TYPED
               field, not an extra -- tools/census_e1_exact.c). Those are now
               served by rb_evaluate_success_zone_constant_modifiers, whose
               live_total arm (modifiers.rs:2510-2525) keys off the zone index
               and has no membership test, which is exactly Rust's E1.

               DO NOT delete the guard thinking it is a bug: with the scan
               narrowed it is vacuous, and removing it would make a card that
               somehow reached this path credit `pl` unconditionally, diverging
               from the belongs_to_p1 routing above. */
            if (card_on_stage(g, pl, host_cid)) {
                if (pl == 0 && total_p1) *total_p1 = (int16_t)(*total_p1 + cnt);
                if (pl == 1 && total_p2) *total_p2 = (int16_t)(*total_p2 + cnt);
            }
        } else {
            rb_mods_add_score(&g->mods, tgt_cid, cnt);
            if(!acc) g->mods.constant_score[tgt_cid]+=cnt;
            if(acc) acc->score+=cnt;
        }
    } else if (!strcmp(e->action,"gain_blade") || !strcmp(e->action,"add_blade") ||
               !strcmp(e->action,"gain_blade_heart") || !strcmp(e->action,"set_blade_count") ||
               !strcmp(e->action,"modify_blade")) {
        int cnt=e->count>=0?e->count:1;
        for(int i=0;i<e->n_extra;i++) if(e->extra_k[i] && !strcmp(e->extra_k[i],"sign") && e->extra_v[i] && !strcmp(e->extra_v[i],"negative")) cnt = -cnt;
        rb_mods_add_blade(&g->mods, tgt_cid, cnt);
        if(!acc) g->mods.constant_blade[tgt_cid]+=cnt;
        if(acc) acc->blade+=cnt;
    } else if (!strcmp(e->action,"gain_resource")) {
        const char *res=NULL;
        for(int i=0;i<e->n_extra;i++) if(e->extra_k[i] && !strcmp(e->extra_k[i],"resource")) res=e->extra_v[i];
        /* "失う" (lose) is sign:negative -- modifiers.rs:785-797 applies the sign
           to the RESOLVED amount, after the dynamic_count / per_unit / plain
           choice, so keep it out of the base count. */
        int sign_neg = 0;
        for(int i=0;i<e->n_extra;i++) if(e->extra_k[i] && !strcmp(e->extra_k[i],"sign") && e->extra_v[i] && !strcmp(e->extra_v[i],"negative")) sign_neg = 1;
        int per_unit = ceff_per_unit(e);
        if (res && (!strcmp(res,"blade")||!strcmp(res,"ブレード"))) {
            /* modifiers.rs:728-782: dynamic_count, else per_unit, else the
               printed icon count.  per_unit is `units * base` with base = 1
               under `max`, else resource_icon_count (defaulting to count). */
            int cnt = e->count>=0?e->count:1;
            {
                const char *ric = cextra(e, "resource_icon_count");
                if (ric) cnt = atoi(ric);
            }
            if (per_unit) {
                const char *mx = cextra(e, "max");
                int is_max = mx && !strcmp(mx, "true");
                int base = is_max ? 1 : cnt;
                if (base < 0) base = 0;
                cnt = constant_gain_per_unit_units(g, pl, host_cid, e) * base;
            }
            if (sign_neg) cnt = -cnt;
            if (pos && !strcmp(pos,"front")) {
                int source_idx = 0;
                for (int q = 0; q < RB_STAGE_SIZE; q++)
                    if (g->p[pl].stage[q] == host_cid) { source_idx = q; break; }
                int dest_idx = RB_STAGE_SIZE - 1 - source_idx;
                tgt_pl = 1 - pl;
                tgt_cid = g->p[tgt_pl].stage[dest_idx];
                const char *cost_op = NULL;
                int cost_limit = -1;
                for (int i = 0; i < e->n_extra; i++) {
                    if (e->extra_k[i] && !strcmp(e->extra_k[i],"cost_limit_operator")) cost_op = e->extra_v[i];
                    if (e->extra_k[i] && !strcmp(e->extra_k[i],"cost_limit")) cost_limit = atoi(e->extra_v[i]);
                }
                if (tgt_cid == RB_EMPTY_SLOT) return;
                Card target_card;
                if (!rb_decode_card_by_index((uint32_t)tgt_cid, &target_card)) return;
                int target_cost = target_card.cost;
                int allowed = cost_limit < 0 || rb_compare_counts(cost_op, target_cost, cost_limit);
                rb_free_card(&target_card);
                if (!allowed) return;
            }
            rb_mods_add_blade(&g->mods, tgt_cid, cnt);
            if(!acc) g->mods.constant_blade[tgt_cid]+=cnt;
            if(acc) acc->blade+=cnt;
        } else if (res && (!strcmp(res,"heart")||!strcmp(res,"ハート"))) {
            /* modifiers.rs:875-916: per_unit yields the RAW unit tally (no
               printed-count multiplier -- unlike the blade arm), the plain
               branch is `count`. */
            int cnt = per_unit ? constant_gain_per_unit_units(g, pl, host_cid, e)
                               : (e->count>=0?e->count:1);
            if (sign_neg) cnt = -cnt;
            /* Colour buckets -- engine/src/core/game_state/modifiers.rs:991-1023:

                 if is_all_heart_type(effect) {
                     exp_heart[card_id][HEART_ALL_KEY] += n;          // "heart00"
                 } else {
                     let hc_list = effect.heart_colors_any();
                     let per_entry = n / hc_list.len().max(1).max(1);
                     for hc in hc_list { exp_heart[card_id][hc] += per_entry; }
                 }

               The list is the PLURAL `heart_colors`. This read the SINGULAR
               `heart_color` extra, which the decoder only mirrors for a
               ONE-element array (vm.c:1120), so every multi-icon grant left
               `hc` NULL and `col` stayed 0 -- Heart00, the wildcard bucket
               that rb_mods_get_heart adds to EVERY colour query
               (modifiers.c:64-72). 冬毬 PL!SP-bp5-011-R carries three 常時,
               each `heart_colors` = "heart02,heart02,heart02" with count 3, so
               hc_list.len() == 3 and per_entry == 1: the three increments land
               in the SAME heart02 bucket for a total of 3, exactly as printed,
               instead of 3 in Heart00 that answered 3 for all seven colours.

               Rust's empty-list else branch writes NOTHING (the loop body never
               runs), which is why the wildcard fallback below is confined to
               the `heart_type: "all"` arm: of the 34 constant heart gains in
               the corpus, the only one without `heart_colors` is exactly that
               one. */
            if (rb_is_all_heart_type(e)) {
                rb_mods_add_heart(&g->mods, tgt_cid, RB_HEART_PINK, cnt);
                if(!acc) g->mods.constant_heart[tgt_cid][RB_HEART_PINK]+=cnt;
                if(acc) acc->heart[RB_HEART_PINK]+=cnt;
            } else {
                const char *hcl = rb_effect_heart_colors_any(e);
                int cols[16];
                int ncols = const_heart_color_list(hcl, cols, 16);
                int per_entry = (ncols > 0) ? (cnt / ncols) : 0;
                for (int ci = 0; ci < ncols; ci++) {
                    int col = cols[ci];
                    rb_mods_add_heart(&g->mods, tgt_cid, col, per_entry);
                    if(!acc) g->mods.constant_heart[tgt_cid][col]+=(int16_t)per_entry;
                    if(acc) acc->heart[col]+=(int16_t)per_entry;
                }
            }
        }
    } else if (!strcmp(e->action,"modify_required_hearts") ||
               !strcmp(e->action,"modify_required_hearts_global")) {
        /* Two Rust twins, and BOTH of them reduce the requirement of a SET of
           live cards -- never of the 常時's own host:

             * modify_required_hearts_global -- the CONSTANT dispatch arm at
               engine/src/core/game_state/modifiers.rs:1209-1249. Every card in
               the RESOLVED TARGET PLAYER's live_card_zone, amount
               `value_or_count(1)`, signed by `operation`, one entry per
               `heart_colors_any()` token (HEART_ALL_KEY when the list is
               empty). No group / score filter -- the global arm has none.

             * modify_required_hearts -- required_hearts.rs:28-223
               (execute_modify_required_hearts), reached from the constant pass
               through apply_success_zone_effect (modifiers.rs:2303-2319).
               Amount `value_or_count(0)`, the same plural colour list, and a
               candidate set filtered by self_target, group_name and the
               original-score threshold.

           The arm this replaces read the SINGULAR `heart_color`, took `count`
           (which on PL!-bp6-022-L is the ORIGINAL-SCORE THRESHOLD 5, not the
           amount), ignored `operation` entirely, and defaulted `tgt_cid` to
           the HOST. So 「…自分の元々のスコアが５以上の『μ's』のライブカードの
           必要ハートをheart0を2減らす」 put +5 on the activator instead of -2 on
           every μ's live card in the acting player's live-card zone whose
           ORIGINAL score is >= 5, and Wien's global 常時 (PL!SP-pb2-010-P, +1 to
           every OPPONENT live card) put its +1 on itself. */
        int is_global = !strcmp(e->action, "modify_required_hearts_global");
        const char *etgt = cextra(e, "target");
        if (!etgt) etgt = e->target;
        int t_pl = pl;
        if (etgt && (!strcmp(etgt, "opponent") || !strcmp(etgt, "相手"))) t_pl = 1 - pl;

        /* required_hearts.rs:34-35 / modifiers.rs:1212-1213 --
             operation   = operation_any().unwrap_or("decrease" | "increase")
             value       = value_any().or(count).unwrap_or(0 | 1)   */
        const char *op = cextra(e, "operation");
        if (!op) op = is_global ? "increase" : "decrease";
        int val;
        {
            const char *vx = cextra(e, "value");
            if (vx) val = atoi(vx);
            else if (e->count >= 0) val = e->count;
            else val = is_global ? 1 : 0;
        }

        /* required_hearts.rs:116-140 -- the candidate set. With self_target
           and the activating card already in the SUCCESS zone the scan covers
           live_card_zone chained with success_live_card_zone; otherwise the
           live-card zone alone. */
        const RbPlayer *TP = &g->p[t_pl];
        int cand[RB_MAX_ZONE * 2];
        int ncand = 0;
        if (!is_global) {
            const char *stv = cextra(e, "self_target");
            int self_target = stv && !strcmp(stv, "true");
            int activating_in_success = 0;
            if (self_target && g->activating_card >= 0) {
                for (int i = 0; i < TP->success.n && !activating_in_success; i++)
                    if (TP->success.cards[i] == g->activating_card) activating_in_success = 1;
            }
            for (int i = 0; i < TP->live.n && ncand < RB_MAX_ZONE * 2; i++)
                if (TP->live.cards[i] != RB_EMPTY_SLOT) cand[ncand++] = TP->live.cards[i];
            if (activating_in_success)
                for (int i = 0; i < TP->success.n && ncand < RB_MAX_ZONE * 2; i++)
                    if (TP->success.cards[i] != RB_EMPTY_SLOT) cand[ncand++] = TP->success.cards[i];
        } else {
            for (int i = 0; i < TP->live.n && ncand < RB_MAX_ZONE * 2; i++)
                if (TP->live.cards[i] != RB_EMPTY_SLOT) cand[ncand++] = TP->live.cards[i];
        }

        /* required_hearts.rs:147-167 -- the per-card filters.
             self_target   -> only the activating card
             group_name    -> the FIRST entry of `group_names`
             original_value -> the ORIGINAL score must clear
                              `original_count` `original_operator`
           score.rs OriginalScoreFilter::accepts treats a card with NO printed
           score icon, a missing threshold, or an unknown operator as ACCEPTED
           ("a parser gap, not a threshold that failed"), so `score == 0` --
           which is what card.c:105 decodes a scoreless card to -- must pass. */
        char *grp = NULL;
        const char *ov = cextra(e, "original_value");
        int orig_enabled = ov && !strcmp(ov, "true");
        const char *oc = cextra(e, "original_count");
        int orig_count = oc ? atoi(oc) : 0;
        const char *oo = cextra(e, "original_operator");
        if (!is_global) {
            grp = cextra_first_entry(e, "group_names");
            const char *stv = cextra(e, "self_target");
            int self_target = stv && !strcmp(stv, "true");
            int keep[RB_MAX_ZONE * 2];
            int nkeep = 0;
            for (int i = 0; i < ncand; i++) {
                int cid = cand[i];
                if (cid < 0) continue;
                if (self_target && g->activating_card >= 0 && cid != g->activating_card) continue;
                if (grp && !rb_card_matches_group_str(cid, grp)) continue;
                if (orig_enabled && oc && oo) {
                    Card oc_card;
                    if (rb_decode_card_by_index((uint32_t)cid, &oc_card)) {
                        int score = (int)oc_card.score;
                        rb_free_card(&oc_card);
                        /* No score icon == Rust's None == accepted. */
                        if (score > 0 && !rb_compare_counts(oo, score, orig_count)) continue;
                    }
                }
                keep[nkeep++] = cid;
            }
            ncand = nkeep;
            for (int i = 0; i < ncand; i++) cand[i] = keep[i];
        }

        /* required_hearts.rs:182-186 / modifiers.rs:1223-1228 -- the plural
           colour list, defaulting to the Heart00 key when it is absent. */
        const char *hcl = rb_effect_heart_colors_any(e);
        int cols[16];
        int ncols = const_heart_color_list(hcl && *hcl ? hcl : "heart00", cols, 16);
        if (grp) { rb_free(grp); grp = NULL; }

        for (int ci = 0; ci < ncand; ci++) {
            int cid = cand[ci];
            if (cid < 0) continue;
            for (int k = 0; k < ncols; k++) {
                int col = cols[k];
                int delta;
                if (!strcmp(op, "decrease"))       delta = -val;
                else if (!strcmp(op, "increase")) delta = val;
                else if (!strcmp(op, "set"))       delta = val;
                else continue;   /* unknown op: Rust returns Err, applying nothing */
                if (!strcmp(op, "set")) {
                    /* required_hearts.rs:203-207. NOTE: `set` is an absolute
                       write and rb_recalc_constants only reverses the ADDITIVE
                       constant_need_heart ledger, so a constant `set` is not
                       withdrawn on the next recalc. No card in the corpus uses
                       operation:"set" on a 常時, so nothing reaches it today;
                       withdrawing it properly needs a `set` ledger on RbMods
                       (rabuka.h), which is not this file's to add. */
                    rb_mods_set_need_heart(&g->mods, cid, col, delta);
                    if (acc) acc->need_heart[col] = (int16_t)delta;
                    continue;
                }
                rb_mods_add_need_heart(&g->mods, cid, col, delta);
                if(!acc) g->mods.constant_need_heart[cid][col]+=(int16_t)delta;
                if(acc) acc->need_heart[col]+=(int16_t)delta;
            }
        }
    } else if (!strcmp(e->action,"set_cost") || !strcmp(e->action,"set_cost_to_use")) {
        int cnt=e->count>=0?e->count:1;
        for(int i=0;i<e->n_extra;i++) if(e->extra_k[i] && !strcmp(e->extra_k[i],"value")) cnt=atoi(e->extra_v[i]);
        rb_mods_add_cost(&g->mods, tgt_cid, cnt);
        if(!acc) g->mods.constant_cost[tgt_cid]+=cnt;
        if(acc) acc->cost+=cnt;
    } else if (!strcmp(e->action,"set_score")) {
        int cnt=e->count>=0?e->count:1;
        for(int i=0;i<e->n_extra;i++) if(e->extra_k[i] && !strcmp(e->extra_k[i],"value")) cnt=atoi(e->extra_v[i]);
        rb_mods_add_score(&g->mods, tgt_cid, cnt);
        if(!acc) g->mods.constant_score[tgt_cid]+=cnt;
        if(acc) acc->score+=cnt;
    } else if (!strcmp(e->action,"set_blade_count")) {
        int cnt=e->count>=0?e->count:1;
        for(int i=0;i<e->n_extra;i++) if(e->extra_k[i] && !strcmp(e->extra_k[i],"value")) cnt=atoi(e->extra_v[i]);
        rb_mods_add_blade(&g->mods, tgt_cid, cnt);
        if(!acc) g->mods.constant_blade[tgt_cid]+=cnt;
        if(acc) acc->blade+=cnt;
    } else if (!strcmp(e->action,"set_blade_type")) {
        /* Constant blade-color recolor — mirrors engine.c:set_blade_type field write.
            Re-applied idempotently each recalc (no additive tracking needed). */
        const char *bc=NULL;
        for(int i=0;i<e->n_extra;i++) if(e->extra_k[i] && (!strcmp(e->extra_k[i],"blade_color")||!strcmp(e->extra_k[i],"blade_type"))) bc=e->extra_v[i];
        int col=-1;
        if(bc){
            if(!strcmp(bc,"pink")||!strcmp(bc,"heart00")) col=0;
            else if(!strcmp(bc,"red")) col=1;
            else if(!strcmp(bc,"yellow")) col=2;
            else if(!strcmp(bc,"green")) col=3;
            else if(!strcmp(bc,"blue")) col=4;
            else if(!strcmp(bc,"purple")) col=5;
            else if(!strcmp(bc,"orange")) col=6;
            else if(!strcmp(bc,"all")) col=7;
        }
        g->mods.blade_type[tgt_cid]=(int8_t)col;
    } else if (!strcmp(e->action,"set_heart_type")) {
        /* Constant heart recolor — mirrors engine.c:set_heart_type field write. */
        const char *ref=NULL;
        for(int i=0;i<e->n_extra;i++) if(e->extra_k[i] && !strcmp(e->extra_k[i],"ref_value")) ref=e->extra_v[i];
        if(ref && !strcmp(ref,"placed_under")){
            for(int s=0;s<RB_STAGE_SIZE;s++){ if(g->p[tgt_pl].stage[s]==tgt_cid && g->p[tgt_pl].under_cards[s].n>0){ g->mods.heart_copy[tgt_cid]=g->p[tgt_pl].under_cards[s].cards[0]; break; } }
        } else {
            int hcol=7;
            for(int i=0;i<e->n_extra;i++) if(e->extra_k[i] && !strcmp(e->extra_k[i],"heart_color")){
                const char *hc=e->extra_v[i];
                if(!hc) continue;
                else if(!strcmp(hc,"pink")||!strcmp(hc,"heart00")) hcol=0;
                else if(!strcmp(hc,"red")) hcol=1;
                else if(!strcmp(hc,"yellow")) hcol=2;
                else if(!strcmp(hc,"green")) hcol=3;
                else if(!strcmp(hc,"blue")) hcol=4;
                else if(!strcmp(hc,"purple")) hcol=5;
                else if(!strcmp(hc,"orange")) hcol=6;
                else if(!strcmp(hc,"all")) hcol=7;
            }
            g->mods.heart_multiplier[tgt_cid]=(int8_t)hcol;
        }
    } else if (!strcmp(e->action,"gain_ability")) {
        const char *ag=NULL;
        for(int i=0;i<e->n_extra;i++) if(e->extra_k[i] && !strcmp(e->extra_k[i],"ability_gain")) ag=e->extra_v[i];
        AbilityEffect *inner = e->gained_effect;
        int inner_total = inner && inner->action && !strcmp(inner->action, "modify_score") &&
                          inner->target && !strcmp(inner->target, "live_total");
        if (inner && inner->action && !strcmp(inner->action, "conditional_alternative")) {
            if (g->n_delayed_gained_effects < RB_MAX_DELAYED_GAINED) {
                int di = g->n_delayed_gained_effects++;
                g->delayed_gained_effects[di].card_id = host_cid;
                g->delayed_gained_effects[di].effect = rb_effect_deep_clone(inner);
            }
        } else if (inner_total && card_on_stage(g, pl, host_cid)) {
            int cnt = inner->count >= 0 ? inner->count : 0;
            for (int i = 0; i < inner->n_extra; i++) {
                if (inner->extra_k[i] && !strcmp(inner->extra_k[i], "value")) cnt = atoi(inner->extra_v[i]);
            }
            if (pl == 0 && total_p1) *total_p1 = (int16_t)(*total_p1 + cnt);
            if (pl == 1 && total_p2) *total_p2 = (int16_t)(*total_p2 + cnt);
        } else if (ag && strstr(ag, "ハート")) {
            rb_mods_add_heart(&g->mods, tgt_cid, 7, 1);
            if(!acc) g->mods.constant_heart[tgt_cid][7]+=1;
            if(acc) acc->heart[7]+=1;
        }
    } else if (!strcmp(e->action,"gain_ability_from_source")) {
        /* engine/src/core/game_state/modifiers.rs:1148-1154 (ActionType::
           GainAbilityFromSource inside the constant dispatch). The 常時 scan
           never reached this arm: `rb_gain_ability_from_source` is fully
           implemented and is called from the ability path (src/engine.c:621),
           but the constant path simply had no case for the action, so a card
           whose PRINTED 常時 was 「このメンバーの下にあるカードから能力を得る」
           re-derived nothing on every recalculation and its
           `gained_card_abilities` slot stayed empty forever.

           s_gain_ability_from_source (src/ability/effects/ability.c:465) reads
           the target from `g->activating_card` and returns immediately when it
           is < 0, so the call only works because rb_recalc_constants publishes
           activating_card around the apply (see below) -- the same published
           field the Rust reads at modifiers.rs:662 / ability_ops.rs:572.

           rb_gain_ability_from_source is used rather than
           rb_translated_execute_gain_ability_from_source (executor.c:429)
           because the translated one calls rb_recalc_constants itself
           (ability_effects.c:423) -- calling it from inside the scan would
           re-enter the scan, re-clear the slot and re-copy forever. Rust has
           the same shape: the single recalculation pass owns the whole
           gained-set rebuild (modifiers.rs:1268-1308) and
           execute_gain_ability_from_source itself never recalculates. */
        rb_gain_ability_from_source(g, pl, e, host_cid);
    }
    else if (!strcmp(e->action,"draw") || !strcmp(e->action,"draw_card") ||
               !strcmp(e->action,"draw_until_count")) {
        rb_effect_draw_card(g, pl, e, host_cid);
    } else if (!strcmp(e->action,"discard_card") || !strcmp(e->action,"discard_until_count")) {
        rb_effect_move_cards(g, pl, e);
    } else if (!strcmp(e->action,"move_cards") || !strcmp(e->action,"return_to_hand") ||
               !strcmp(e->action,"bounce") || !strcmp(e->action,"back_to_hand") ||
               !strcmp(e->action,"deck_bottom") || !strcmp(e->action,"put_on_bottom")) {
        rb_effect_move_cards(g, pl, e);
    } else if (!strcmp(e->action,"look_at") || !strcmp(e->action,"reveal") ||
               !strcmp(e->action,"reveal_per_group")) {
        rb_effect_look_at(g, pl, e);
    } else if (!strcmp(e->action,"reveal_until_live_card")) {
        rb_effect_reveal_until_live_card(g, pl, e);
    } else if (!strcmp(e->action,"reveal_until_chosen_card")) {
        rb_effect_reveal_until_chosen_card(g, pl, e);
    } else if (!strcmp(e->action,"reveal_until_target")) {
        rb_effect_reveal_until_target(g, pl, e);
    } else if (!strcmp(e->action,"select_cards") || !strcmp(e->action,"select") ||
               !strcmp(e->action,"select_number") || !strcmp(e->action,"look_and_select")) {
        rb_effect_select_cards(g, pl, e);
    } else if (!strcmp(e->action,"change_state")) {
         rb_effect_change_state(g, pl, e, -1);
    } else if (!strcmp(e->action,"position_change")) {
        rb_effect_position_change(g, pl, e, host_cid);
    } else if (!strcmp(e->action,"rotation")) {
        rb_effect_rotation(g, pl, e);
    } else if (!strcmp(e->action,"restriction")) {
        int resolved=0; rb_execute_misc_effect(g, pl, &g->p[pl], e, &resolved);
    } else if (!strcmp(e->action,"energy_placement")) {
        rb_effect_energy_placement(g, pl, e);
    } else if (!strcmp(e->action,"energy_state_change")) {
        rb_effect_energy_state_change(g, pl, e);
    }
    /* Sequential children: walk them (q127_wien leaves_stage_modifier_removed etc.).
       Each sub-effect is gated by its OWN condition, exactly as
       engine/src/core/game_state/modifiers.rs:1176-1185 does:
           ActionType::Sequential => for sub in actions {
               let sub_cond = sub.condition.as_ref().is_none_or(|c| ctx.evaluate_condition(c));
               if !sub_cond { continue; }
               ...
           }
       The PARENT's gate is the caller's (rb_recalc_constants); a compound whose
       parent condition fails is not walked at all. */
    for(int i=0;i<e->n_child;i++) {
        AbilityEffect *ch = e->child[i];
        if (ch && (ch->has_condition || ch->condition) &&
            !rb_eval_condition_for_host(g, pl, host_cid, ch->condition)) continue;
        apply_constant_effect(g, pl, host_cid, ch, acc, total_p1, total_p2);
    }
}

/* Fire a card's 登場 (Debut) abilities by queuing them and draining the queue,
   exactly mirroring Rust's trigger_debut → process_pending_auto_abilities path.
   This executes the ability's full effect tree (move_cards / gain_resource /
   modify_cost / etc.) instead of only the constant modifiers. */
void rb_fire_debut(GameState *g, int pl, int card_id) {
    rb_trigger_debut(g, pl, card_id);
    if (g->queue.n_entries > 0)
        rb_drain_ability_queue(g);
}

/* Revert temporary effects whose Duration matches `which`:
     0 = all, 1 = live_end/during_live (called at live-phase end),
     2 = until_end_of_turn/first_turn (called at turn rollover).
   Reverted entries are compacted out of the array so other durations survive. */
static void remove_prohibition_text(GameState *g, const char *text) {
    if (!g || !text || !*text) return;
    int w = 0;
    for (int i = 0; i < g->n_prohibition; i++)
        if (strcmp(g->prohibition[i], text) != 0) {
            memcpy(g->prohibition[w], g->prohibition[i], sizeof(g->prohibition[w]));
            w++;
        }
    g->n_prohibition = w;
    w = 0;
    for (int i = 0; i < g->n_prohibition_effects; i++)
        if (strcmp(g->prohibition_effects[i], text) != 0) {
            memcpy(g->prohibition_effects[w], g->prohibition_effects[i], sizeof(g->prohibition_effects[w]));
            w++;
        }
    g->n_prohibition_effects = w;
}

void rb_check_expired_effects(GameState *g, int which) {
    int w = g->n_temp_effects;
    int j = 0;
    int gained_changed = 0;
    for(int i=0;i<w;i++){
        RbTempEffect *te=&g->temp_effects[i];
        int expire = (which==0) || (which==1 && te->dur==RB_TEMP_LIVE_END) ||
                     (which==2 && te->dur==RB_TEMP_TURN_END);
        if(expire){
            if (te->gained_index >= 0) {
                int removed_index = te->gained_index;
                if (rb_remove_gained_ability(g, te->gained_card_id, removed_index)) {
                    gained_changed = 1;
                    for (int k = 0; k < g->n_temp_effects; k++) {
                        RbTempEffect *other = &g->temp_effects[k];
                        if (other->gained_card_id == te->gained_card_id &&
                            other->gained_index > removed_index)
                            other->gained_index--;
                    }
                }
            }
            if (te->revert_kind == RB_REVERT_BLADE_SET && te->card_id >= 0)
                g->mods.blade[te->card_id].set = (int16_t)te->previous_value;
            else if (te->revert_kind == RB_REVERT_BLADE_TYPE && te->card_id >= 0)
                g->mods.blade_type[te->card_id] = (int8_t)te->previous_value;
            else if (te->revert_kind == RB_REVERT_HEART_MULT && te->card_id >= 0) {
                g->mods.heart_multiplier[te->card_id] = (int8_t)te->previous_value;
                g->mods.heart_multiplier_amt[te->card_id] = (int8_t)te->previous_value2;
            } else if (te->revert_kind == RB_REVERT_HEART_COPY && te->card_id >= 0)
                g->mods.heart_copy[te->card_id] = (int16_t)te->previous_value;
            else if (te->revert_kind == RB_REVERT_COST_SET && te->card_id >= 0)
                g->mods.cost[te->card_id].set = (int16_t)te->previous_value;
            else if (te->revert_kind == RB_REVERT_TEXT)
                remove_prohibition_text(g, te->revert_text);
            rb_mods_add_blade(&g->mods, te->card_id, -te->blade);
            rb_mods_add_score(&g->mods, te->card_id, -te->score);
            rb_mods_add_cost(&g->mods, te->card_id, -te->cost);
            for(int c=0;c<8;c++){
                rb_mods_add_heart(&g->mods, te->card_id, c, -te->heart[c]);
                rb_mods_add_need_heart(&g->mods, te->card_id, c, -te->need_heart[c]);
            }
        } else {
            g->temp_effects[j++] = g->temp_effects[i];
        }
    }
    g->n_temp_effects = j;
    int invalidation_write = 0;
    for (int i = 0; i < g->n_ability_invalidations; i++) {
        RbAbilityInvalidation *entry = &g->ability_invalidations[i];
        int live_end = !strcmp(entry->duration, "live_end") ||
                       !strcmp(entry->duration, "during_live");
        int turn_end = !strcmp(entry->duration, "until_end_of_turn") ||
                       !strcmp(entry->duration, "first_turn") ||
                       !strcmp(entry->duration, "turn_end");
        int expire = which == 0 || (which == 1 && live_end) ||
                     (which == 2 && turn_end);
        if (!expire) g->ability_invalidations[invalidation_write++] = *entry;
    }
    g->n_ability_invalidations = invalidation_write;
    if (gained_changed) rb_recalc_constants(g);
}

void rb_recalc_constants(GameState *g) {
    if (!g) return;
    for (int i = 0; i < g->n_delayed_gained_effects; i++) {
        rb_effect_free(g->delayed_gained_effects[i].effect);
        g->delayed_gained_effects[i].effect = NULL;
    }
    g->n_delayed_gained_effects = 0;
    int16_t total_p1 = 0;
    int16_t total_p2 = 0;
    for (int i = 0; i < RB_MAX_CARD_IDS; i++) {
        if (g->mods.constant_blade[i]) { rb_mods_add_blade(&g->mods, i, -g->mods.constant_blade[i]); g->mods.constant_blade[i]=0; }
        if (g->mods.constant_score[i]) { rb_mods_add_score(&g->mods, i, -g->mods.constant_score[i]); g->mods.constant_score[i]=0; }
        if (g->mods.constant_cost[i])  { rb_mods_add_cost(&g->mods, i, -g->mods.constant_cost[i]);  g->mods.constant_cost[i]=0; }
        for (int c = 0; c < 8; c++) {
            if (g->mods.constant_heart[i][c])      { rb_mods_add_heart(&g->mods, i, c, -g->mods.constant_heart[i][c]);      g->mods.constant_heart[i][c]=0; }
            if (g->mods.constant_need_heart[i][c]) { rb_mods_add_need_heart(&g->mods, i, c, -g->mods.constant_need_heart[i][c]); g->mods.constant_need_heart[i][c]=0; }
        }
    }

    for (int pl = 0; pl < 2; pl++) {
        /* STAGE ONLY -- Rust's main constant loop draws its entries from
           collect_reusable_constant_stage_effect_ids (modifiers.rs:453-455),
           which chains player1.stage.stage + player2.stage.stage and nothing
           else. The success and live-card bags are a SEPARATE scan, Rust's
           evaluate_success_zone_constant_modifiers (modifiers.rs:2232-2237),
           now ported as rb_evaluate_success_zone_constant_modifiers in
           src/core/modifiers.c and called at the tail of this function.

           This loop used to walk all three zone kinds and push them through
           apply_constant_effect, which conflated the two Rust functions. That
           was not merely a duplicate: it applied the WIDER action set of
           apply_constant_effect (gain_ability and friends, plus the
           activation_position and ability-invalidation gates) to cards Rust
           routes only through E1's FOUR arms (modifiers.rs:2326/:2343/:2487/
           :2573, with `_ => {}` for the rest), and it banked the result in the
           constant_* ledgers instead of the success_zone_* ones, so nothing
           could withdraw it correctly. Both loops scanning the same cards
           would also double every grant. 220 constant effect nodes are
           affected, measured per-action in tools/census_e1_exact.c -- 1
           modify_required_hearts, 176 gain_resource, 29 modify_score,
           14 sequential. */
        int zone_cids[RB_STAGE_SIZE];
        int is_stage[RB_STAGE_SIZE];
        int zn = 0;
        for (int s = 0; s < RB_STAGE_SIZE; s++) {
            if (g->p[pl].stage[s] == RB_EMPTY_SLOT) continue;
            zone_cids[zn] = g->p[pl].stage[s];
            is_stage[zn++] = 1;
        }

        for (int z = 0; z < zn; z++) {
            int cid = zone_cids[z];
            /* engine/src/core/game_state/modifiers.rs:659-662 --
                 // Set activating_card so condition evaluators (e.g. exclude_self
                 // in location_condition) know which card is "self" for this entry.
                 let prev_activating = self.activating_card;
                 self.activating_card = Some(card_id);
               and restored at modifiers.rs:1265-1266. The C scan published
               NOTHING here, so every effect that resolves its host from
               `g->activating_card` instead of from the scan's own `cid` --
               s_gain_ability_from_source most of all -- saw a stale or
               unset value and bailed on its first line. Publishing it around
               the whole per-card apply (and restoring on the way out, because
               rb_recalc_constants re-enters) is what makes the scan-driven
               grant agree with the directly-driven one. */
            int prev_activating = g->activating_card;
            g->activating_card = cid;
            int n = rb_card_num_abilities((uint32_t)cid);
            for (int ai = 0; ai < n; ai++) {
                Ability ab;
                if (!rb_decode_card_ability((uint32_t)cid, ai, &ab)) continue;
                if (rb_ability_has_trigger(&ab, RB_TK_CONSTANT) && ab.effect &&
                    !rb_ability_is_invalidated(g, cid, "常時")) {
                    AbilityEffect *e = ab.effect;
                    int cond_ok = effect_position_matches(g, pl, cid, e) &&
                                  (!e->has_condition || !e->condition ||
                                   rb_eval_condition_for_host(g, pl, cid, e->condition));
                    if (rb_ability_debug_enabled()) {
                        fprintf(stderr, "[recalc] cid=%d pl=%d cond_ok=%d act=%s\n",
                                cid, pl, cond_ok, e->action ? e->action : "-");
                    }
                    /* Faithful port of modifiers.rs:681-689 --
                         if pos_ok {
                             let cond_met = effect.condition.as_ref()
                                 .is_none_or(|c| ctx.evaluate_condition(c));
                             if cond_met { ...apply... }
                         }
                       There is NO else arm: a 常時 whose position gate or
                       condition gate fails applies NOTHING, its `sequential`
                       children included.  The previous else here walked the
                       children whose own condition was absent, which turned
                       every condition FAILURE into a GRANT (鐘 嵐珠
                       PL!N-bp1-012-R＋ granted +2 blade and +2 ALL hearts on
                       0/2 live cards and on 3 live cards with no 『虹ヶ咲』). */
                    if (cond_ok) {
                        apply_constant_effect(g, pl, cid, e, NULL, &total_p1, &total_p2);
                    }
                }
                rb_free_ability(&ab);
            }

            if (is_stage[z]) {
                int ng = rb_card_num_gained_abilities(g, cid);
                for (int gi = 0; gi < ng; gi++) {
                    const Ability *gab = rb_card_gained_ability(g, cid, gi);
                    if (!gab || !gab->effect || !rb_ability_has_trigger(gab, RB_TK_CONSTANT) ||
                        rb_ability_is_invalidated(g, cid, "常時")) continue;
                    int cond_ok = effect_position_matches(g, pl, cid, gab->effect) &&
                                  (!gab->effect->has_condition || !gab->effect->condition ||
                                   rb_eval_condition_for_host(g, pl, cid, gab->effect->condition));
                    /* Same gate as the printed-ability branch above
                       (modifiers.rs:681-689): failure applies NOTHING. */
                    if (cond_ok) {
                        apply_constant_effect(g, pl, cid, gab->effect, NULL, &total_p1, &total_p2);
                    }
                }
            }
            /* modifiers.rs:1265-1266 -- restore the caller's activating_card. */
            g->activating_card = prev_activating;
        }
    }
    /* Under-card constant blade abilities (engine/src/core/game_state/
       modifiers.rs:101-168 grant_under_card_constant_blades, called at
       modifiers.rs:1319 -- after the stage/live main loop and BEFORE
       commit_constant_results, because it accumulates into the same
       exp_blade map).

       Rust does NOT put under-cards into the main entries list: a card that is
       not on stage has no stage position, so the main loop's position/owner
       logic does not apply to it. It runs one narrow pass instead, and the
       C equivalent is exactly that -- the under-card's 常時 blade grant is
       routed to the HOST member it is stacked under, never to the under-card
       itself (which is not on the board).

       The three Rust filters, all of which are `continue`s:
         * action must be GainResource with resource == "blade"
         * the ability must carry a condition, and its location must be
           "under_member" -- "Only under-member-scoped grants route to the
           host; a generic constant gain on an under-card has no such
           meaning" (modifiers.rs:137-140)
         * the HOST must match one of the condition's group_names
       and the amount is resource_icon_count, else count, else 1. */
    for (int pl = 0; pl < 2; pl++) {
        const RbPlayer *P = &g->p[pl];
        for (int area = 0; area < RB_STAGE_SIZE; area++) {
            int host = P->stage[area];
            if (host == RB_EMPTY_SLOT) continue;
            const RbBag *under = &P->under_cards[area];
            for (int u = 0; u < under->n; u++) {
                int under_cid = under->cards[u];
                if (under_cid < 0) continue;
                int nab = rb_card_num_abilities((uint32_t)under_cid);
                for (int a = 0; a < nab; a++) {
                    Ability ab;
                    if (!rb_decode_card_ability((uint32_t)under_cid, a, &ab)) continue;
                    const AbilityEffect *e = ab.effect;
                    if (e && e->action && rb_ability_has_trigger(&ab, RB_TK_CONSTANT) &&
                        !rb_ability_is_invalidated(g, under_cid, "常時") &&
                        !strcmp(e->action, "gain_resource")) {
                        const char *res = cextra(e, "resource");
                        if (res && (!strcmp(res, "blade") || !strcmp(res, "ブレード")) &&
                            e->condition) {
                            const char *loc = rb_cond_get_str(e->condition, "location");
                            if (loc && !strcmp(loc, "under_member")) {
                                const char *groups[8];
                                int n_groups = rb_condition_tree_group_names(e->condition, groups, 8);
                                if (n_groups > 0) {
                                    int host_ok = 0;
                                    for (int gi = 0; gi < n_groups && !host_ok; gi++)
                                        if (rb_card_matches_group_str(host, groups[gi])) host_ok = 1;
                                    if (host_ok) {
                                        int cnt = e->count;
                                        const char *ric = cextra(e, "resource_icon_count");
                                        if (ric) cnt = atoi(ric);
                                        else if (cnt < 0) cnt = 1;
                                        rb_mods_add_blade(&g->mods, host, (int16_t)cnt);
                                        g->mods.constant_blade[host] += (int16_t)cnt;
                                    }
                                }
                            }
                        }
                    }
                    rb_free_ability(&ab);
                }
            }
        }
    }

    /* engine/src/core/game_state/modifiers.rs:1388-1392 -- the LAST statement
       of recalculate_constants' cost half, after commit_constant_results (:1375)
       and before evaluate_success_zone_constant_modifiers (:1398) /
       refresh_yell_sources (:1400):

           // Also recalculate cost modifiers from hand cards (hand-based cost
           // reductions)
           let hand_ids = self.collect_constant_hand_effect_ids();
           self.recalculate_constant_cost_modifiers_with_ids(&entries, &hand_ids);

       `entries` is the stage constant-id list this scan already walked
       (modifiers.rs:1392), so the pass covers exactly stage + hand. That is
       strictly WIDER than the loop above, which walks stage + success + live
       and never a hand card -- and the widening is the point: 12 of the 17
       printed 常時 modify_cost effects are 「手札にあるこのメンバーカードの
       コストは…減る」, and none of them was reachable before.

       It is ONE call, not one per printed effect, because the twin is a
       diff-and-commit over the whole `constant_cost` ledger
       (modifiers.rs:1633-1652): calling it from inside the per-card scan would
       interleave with the `constant_cost[i] = 0` reset above and with the
       set_cost arm's own ledger write, and — because the twin only runs when a
       STAGE/SUCCESS/LIVE card happens to carry a 常時 modify_cost — a bonus
       whose host had left the board would never be withdrawn.

       The twin is declared at the top of this file; it is defined
       non-static in src/core/modifiers.c:715 and was previously ZERO-CALLER. */
    rb_recalculate_constant_cost_modifiers(g);

    g->mods.p1_constant_total_score_bonus = total_p1;
    g->mods.p2_constant_total_score_bonus = total_p2;

    /* engine/src/core/game_state/modifiers.rs:1396-1400 -- the second of the
       two constant scans, and the LAST statement before refresh_yell_sources:

           // Evaluate constant abilities from success live card zone (e.g. Love wing bell)
           tdbg!("RC:14 SUCCESS_ZONE");
           self.evaluate_success_zone_constant_modifiers();
           tdbg!("RC:14b SUCCESS_ZONE_DONE");
           self.refresh_yell_sources();

       It has to sit HERE, after the p1/p2 assignment above, because E1's
       live_total arm (modifiers.rs:2510-2525) ADDS to those two fields rather
       than assigning them; Rust gets that ordering from commit_constant_results
       (:1375) running before :1398. Calling it earlier would have the total
       overwritten a few lines down.

       This is the call that makes success_zone_blade_bonuses /
       success_zone_heart_bonuses / success_zone_score_bonuses live; before it
       they were 0 writers / 0 readers and a previous live's success-zone bonus
       could not be withdrawn. */
    rb_evaluate_success_zone_constant_modifiers(g);

    rb_refresh_yell_sources(g);
}

static int effect_suppresses_trigger(const AbilityEffect *effect, const char *trigger_name) {
    if (!effect || !effect->action) return 0;
    if (strcmp(effect->action, "suppress_ability_trigger") == 0) {
        for (int k = 0; k < effect->n_extra; k++) {
            if (effect->extra_k[k] && strcmp(effect->extra_k[k], "suppressed_trigger") == 0 &&
                effect->extra_v[k] && strcmp(effect->extra_v[k], trigger_name) == 0) return 1;
        }
    }
    for (int k = 0; k < effect->n_child; k++)
        if (effect_suppresses_trigger(effect->child[k], trigger_name)) return 1;
    return 0;
}

int rb_is_trigger_suppressed(GameState *g, int pl, const char *trigger_name) {
    if (!g || pl < 0 || pl > 1 || !trigger_name) return 0;
    char entry[48];
    snprintf(entry, sizeof(entry), "suppress:%d:%s", pl, trigger_name);
    for (int i = 0; i < g->n_prohibition; i++)
        if (strcmp(g->prohibition[i], entry) == 0) return 1;
    const RbPlayer *P = &g->p[pl];
    for (int zone = 0; zone < 2; zone++) {
        int count = zone == 0 ? RB_STAGE_SIZE : P->live.n;
        for (int i = 0; i < count; i++) {
            int cid = zone == 0 ? P->stage[i] : P->live.cards[i];
            if (cid < 0) continue;
            int nab = rb_card_num_abilities((uint32_t)cid);
            for (int a = 0; a < nab; a++) {
                Ability ab;
                if (!rb_decode_card_ability((uint32_t)cid, a, &ab)) continue;
                int suppressed = effect_suppresses_trigger(ab.effect, trigger_name);
                rb_free_ability(&ab);
                if (suppressed) return 1;
            }
        }
    }
    return 0;
}

/* activation_position of an effect (mirrors AbilityEffect::activation_position_any,
   ability/types.rs — the wire field the parser stamps). NULL when absent. */
static const char *effect_activation_position_any(const AbilityEffect *e) {
    if (!e) return NULL;
    for (int i = 0; i < e->n_extra; i++)
        if (e->extra_k[i] && !strcmp(e->extra_k[i], "activation_position"))
            return e->extra_v[i] ? e->extra_v[i] : "";
    return NULL;
}

/* ══════════════════════════════════════════════════════════════════════
 * rb_trigger_live_success_faithful — DELETED.
 *
 * It carried the correct Rust scan order and the `seen` dedupe set for the
 * ライブ成功時 trigger and had ZERO callers anywhere in src/** or tests/**: the
 * whole tree called rb_trigger_live_success (src/turn/live.c:544 and :1485),
 * which scanned live_cards then stage_cards and had no set at all. Two correct
 * implementations of one trigger, one of them dead, is how the inversion
 * survived review.
 *
 * Its body was merged into rb_trigger_live_success (see the comment there,
 * which cites the Rust lines branch by branch), so the alias that was left
 * behind only delegated -- it was a second entry point onto one function, still
 * uncalled. It is now gone: there is exactly ONE implementation of the
 * ライブ成功時 scan, the live one, and no dead duplicate to drift away from it.
 * include/rabuka.h never declared it (only a comment at :2811 named it), so
 * nothing else had to change.
 * ══════════════════════════════════════════════════════════════════════ */
/* Mirror turn/triggers.rs::trigger_auto_abilities_for_player — delegate to
   GameState's method, which handles the scan + enqueue. */
int rb_trigger_auto_abilities_for_player(GameState *g, int pl) {
    if (!g) return 0;
    return rb_trigger_auto_abilities_for_player_with_event(g, pl, NULL, 0, 0, 0);
}

/* Map a raw trigger text to its canonical English key.
   Mirrors engine/src/triggers.rs::canonical_trigger. Returns a static
   string literal; do not free. */
const char *rb_canonical_trigger(const char *raw) {
    if (!raw) return "unknown";
    if (strstr(raw, RB_TSTR_DEBUT) || strstr(raw, RB_TSTR_DEBUT_EN))
        return "debut";
    if (strstr(raw, RB_TSTR_LIVE_START))
        return "live_start";
    if (strstr(raw, RB_TSTR_LIVE_SUCCESS) || strstr(raw, RB_TSTR_LIVE_SUCCESS_EN))
        return "live_success";
    if (strstr(raw, RB_TSTR_ACTIVATION))
        return "activation";
    if (strstr(raw, RB_TSTR_CONSTANT))
        return "constant";
    if (strstr(raw, RB_TSTR_AUTO))
        return "auto";
    return "unknown";
}

const char *rb_trigger_zone_id(int is_stage) {
    return is_stage ? "stage" : "live_card_zone";
}

const char *rb_trigger_zone_label(int is_stage) {
    return is_stage ? "ステージ" : "ライブ置場";
}

/* Map a trigger string to its texticon filename for card badge display.
   Mirrors engine/src/triggers.rs::trigger_to_texticon. Returns a static
   string literal; do not free. */
const char *rb_trigger_to_texticon(const char *trigger) {
    if (!trigger) return "jyouji";
    if (!strcmp(trigger, RB_TSTR_CONSTANT))      return "jyouji";
    if (!strcmp(trigger, RB_TSTR_LIVE_SUCCESS))   return "live_success";
    if (!strcmp(trigger, RB_TSTR_LIVE_START))     return "live_start";
    if (!strcmp(trigger, RB_TSTR_DEBUT) || !strcmp(trigger, RB_TSTR_DEBUT_EN))
        return "toujyou";
    if (!strcmp(trigger, RB_TSTR_ACTIVATION))     return "kidou";
    if (!strcmp(trigger, RB_TSTR_AUTO))           return "jidou";
    return "jyouji";
}

/* Parse a triggers field ("起動" or "ライブ開始時, 登場") into kinds.
   Mirrors engine/src/triggers.rs::parse_triggers + TriggerKind::from_token.
   Comma-separated, whitespace-trimmed; unknown tokens ignored. Returns count. */
static int kind_of_token(const char *t, RbTriggerKind *out) {
    if (!strcmp(t, RB_TSTR_ACTIVATION)) *out = RB_TK_ACTIVATION;
    else if (!strcmp(t, RB_TSTR_AUTO)) *out = RB_TK_AUTO;
    else if (!strcmp(t, RB_TSTR_CONSTANT)) *out = RB_TK_CONSTANT;
    else if (!strcmp(t, RB_TSTR_DEBUT) || !strcmp(t, RB_TSTR_DEBUT_EN)) *out = RB_TK_DEBUT;
    else if (!strcmp(t, RB_TSTR_LIVE_START)) *out = RB_TK_LIVE_START;
    else if (!strcmp(t, RB_TSTR_LIVE_SUCCESS) || !strcmp(t, RB_TSTR_LIVE_SUCCESS_EN)) *out = RB_TK_LIVE_SUCCESS;
    else if (!strcmp(t, RB_TSTR_MAIN)) *out = RB_TK_MAIN;
    else if (!strcmp(t, RB_TSTR_BATON_TOUCH)) *out = RB_TK_BATON_TOUCH;
    else return 0;
    return 1;
}
int rb_parse_triggers(const char *triggers, RbTriggerKind *out, int max) {
    if (!triggers || !out || max <= 0) return 0;
    int n = 0;
    const char *p = triggers;
    while (*p && n < max) {
        while (*p == ' ' || *p == '\t') p++;
        if (!*p) break;
        const char *comma = strchr(p, ',');
        size_t len = comma ? (size_t)(comma - p) : strlen(p);
        while (len && (p[len-1] == ' ' || p[len-1] == '\t')) len--;
        char tok[64];
        if (len >= sizeof tok) len = sizeof tok - 1;
        memcpy(tok, p, len); tok[len] = 0;
        RbTriggerKind k;
        if (kind_of_token(tok, &k)) out[n++] = k;
        if (!comma) break;
        p = comma + 1;
    }
    return n;
}
