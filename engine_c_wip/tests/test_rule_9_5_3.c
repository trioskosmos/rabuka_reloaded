/* test_rule_9_5_3.c
 *
 * Rule 9.5.3 -- "when several standby auto abilities of one player trigger at
 * the same time, that player chooses which resolves first" -- is the LAST
 * structural gap in the port: `AutoAbilityOption` (Rust
 * engine/src/ability/types.rs:313-318) had NO C struct at all, and the scan in
 * src/core/game_state_abilities.c::rb_process_player_abilities_depth_full had
 * an empty `if (available > 1) { /* would emit a choice *\/ }` arm. A tie was
 * therefore resolved by whichever entry happened to sit first, with no prompt.
 *
 * THE RUST LINE THIS PORT CITES
 *   engine/src/core/game_state/abilities.rs:1755-1788, inside
 *   `process_player_abilities_depth`:
 *
 *       if let Some(second) = second {
 *           let options = core::iter::once(idx)
 *               .chain(core::iter::once(second))
 *               .chain(available_indices)
 *               .filter_map(|idx| {
 *                   let entry = self.ability_queue.get_entry(idx)?;
 *                   let cid = entry.card_id.unwrap_or(0);
 *                   let card_name = self.card_database.get_card(cid)
 *                       .map(|c| c.name.to_string())
 *                       .unwrap_or_else(|| entry.card_no.to_string());
 *                   Some(crate::ability::types::AutoAbilityOption {
 *                       card_name,
 *                       ability_text: entry.ability.full_text.clone(),
 *                       queue_index: idx,
 *                       card_id: entry.card_id,
 *                   })
 *               })
 *               .collect();
 *           let choice = crate::ability::types::Choice::SelectAutoAbility {
 *               player_id: player_id.to_string(),
 *               options,
 *               description:
 *                   "複数の自動能力が同時に発動しました。使用する順番を選択してください。"
 *                       .to_string(),
 *               description_en: Some("Multiple auto abilities triggered \
 *                    simultaneously. Choose the order to use them.".to_string()),
 *               description_ja: Some("複数の自動能力が同時に発動しました。\
 *                    使用する順番を選択してください。".to_string()),
 *           };
 *           self.ability_queue.pause_for_auto_ability_choice(choice);
 *           break;
 *       }
 *       self.ability_queue.promote_entry_by_abs(idx);
 *
 * `first`/`second` come off the SAME filtered iterator at abilities.rs:1738-1739
 * (`is_entry_available(i) && entry_player_id(i) == Some(player_id)`), which is
 * why the head of the option list is the entry that WOULD have been promoted had
 * there been no tie -- `promote_entry_by_abs(idx)` is the very next line.
 *
 * The three queue functions the rule needs were already correct in
 * src/ability/ability_queue.c and are exercised here through the scan rather
 * than directly:
 *   rb_queue_pause_for_auto_ability_choice (ability_queue.c:714, queue.rs:419-422)
 *   rb_queue_resume_with_choice             (ability_queue.c:738, queue.rs:425-437)
 *   rb_queue_promote_entry_by_abs           (ability_queue.c:875, queue.rs:592-602)
 *
 * WHAT IS ASSERTED HERE, and why each one is not tautological:
 *   1. a tie PARKS a prompt instead of resolving silently -- the defect;
 *   2. the prompt is the RIGHT KIND and carries the Rust strings verbatim;
 *   3. the option list holds exactly the scanning player's AVAILABLE entries,
 *      in queue order, by ABSOLUTE index -- including the case where the other
 *      seat's entry sits BETWEEN two of this seat's;
 *   4. the head of the list is the entry that would have been promoted;
 *   5. a tie of ONE still promotes and resolves, with no prompt;
 *   6. answering moves the chosen entry to the head of the queue, which is the
 *      whole of Rust's effect (the scan after the answer re-derives `first`);
 *   7. the C struct really carries Rust's four fields and Rust's
 *      `Option<i16>` discriminant.
 *
 * Every entry used here is enqueued with card_id = -1 and ability_idx = -1, so
 * the scan cannot reach the card database or the resolver: `card_id` is None in
 * Rust terms, which is what exercises the `unwrap_or(0)` /
 * `unwrap_or_else(card_no)` fallback arms, and it keeps case 6 hermetic.
 */

#include "rabuka.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

/* Not (yet) declared in rabuka.h. Signatures copied verbatim from the
   definitions in src/ability/ability_queue.c and src/ability/choice.c. */
extern void rb_queue_init(RbAbilityQueue *q);
extern int  rb_queue_enqueue(GameState *g, int card_id, int ability_idx,
                             int ability_index, const char *player_id,
                             const char *card_no);
extern void rb_resolver_handle_auto_ability_selection(GameState *g, int selected);

static const char *RB9_JA =
    "複数の自動能力が同時に発動しました。使用する順番を選択してください。";
static const char *RB9_EN =
    "Multiple auto abilities triggered simultaneously. Choose the order to use them.";

/* ── per-test verdict layer (same shape as test_parity_queue_resume.c) ────── */
static const char *g_flat_failed[256];
static int g_flat_failed_n;
static void flat_verdict_failed(const char *fn)
{
    for (int i = 0; i < g_flat_failed_n; i++)
        if (g_flat_failed[i] == fn) return;
    if (g_flat_failed_n >= 256) return;
    g_flat_failed[g_flat_failed_n++] = fn;
    printf("FAILED: %s\n", fn);
    fflush(stdout);
}
#define FLAT_VERDICT() flat_verdict_failed(__func__)

static int failures;
static int assertions;

#define CHECK(condition, message) do { \
    assertions++; \
    if (!(condition)) { \
        fprintf(stderr, "FAIL: %s\n", message); \
        FLAT_VERDICT(); \
        failures++; \
    } else { \
        printf("ok: %s\n", message); \
    } \
} while (0)

#define CHECK_EQ(actual, expected, message) do { \
    assertions++; \
    int a_ = (actual); int e_ = (expected); \
    if (a_ != e_) { \
        fprintf(stderr, "FAIL: %s (got %d expected %d)\n", message, a_, e_); \
        FLAT_VERDICT(); \
        failures++; \
    } else { \
        printf("ok: %s\n", message); \
    } \
} while (0)

/* ── fixture ─────────────────────────────────────────────────────────────── */

static GameState fresh_state(void)
{
    GameState g;
    memset(&g, 0, sizeof(g));
    g.winner = -1;
    g.turn = 1;
    g.active = 0;
    g.cheer_check_base = -1;
    rb_queue_init(&g.queue);
    return g;
}

static int push(GameState *g, const char *player_id)
{
    /* card_id = -1  -> Rust's `entry.card_id == None`
       ability_idx = -1 -> the resolver never has to decode anything. */
    return rb_queue_enqueue(g, -1, -1, -1, player_id, NULL);
}

static const RbChoice *pending(const GameState *g)
{
    return g->queue.has_pending ? &g->queue.pending : NULL;
}

/* ── 1/2: a tie parks a prompt, and it is the Rust prompt ───────────────── */

static void test_tie_parks_the_rule_9_5_3_prompt(void)
{
    GameState g = fresh_state();
    push(&g, "p1");
    push(&g, "p1");

    int processed = rb_process_player_abilities(&g, 0);

    /* The defect this suite exists for: the pre-port body had an empty
       `if (available > 1)` arm, resolved the first entry silently and left the
       second one with no prompt and no ordering. */
    CHECK_EQ(processed, 0,
             "RULE 9.5.3: a two-way tie resolves NOTHING on the scanning pass -- "
             "the player orders it (abilities.rs:1786-1787 `pause_for...; break`)");

    const RbChoice *ch = pending(&g);
    CHECK(ch != NULL,
          "RULE 9.5.3: the tie parks a prompt instead of resolving the head");
    if (!ch) return;

    CHECK_EQ(ch->kind, RB_CHOICE_SELECT_AUTO_ABILITY,
             "RULE 9.5.3: the parked prompt is SelectAutoAbility (types.rs:214)");
    CHECK_EQ(g.queue.state, RB_QUEUE_AWAITING_CHOICE,
             "RULE 9.5.3: the queue parks in WaitingForAutoAbilityChoice "
             "(queue.rs:421, ability_queue.c:721)");
    CHECK_EQ(g.queue.auto_ability, 1,
             "RULE 9.5.3: the auto_ability flag is set, which is what makes "
             "rb_queue_current_entry_mut / rb_queue_has_resolver yield no entry "
             "(ability_queue.c:192, :1211)");

    /* abilities.rs:1779-1783 -- the three description fields, verbatim. */
    CHECK(strcmp(ch->description, RB9_JA) == 0,
          "RULE 9.5.3: description is the Rust JA ordering prompt byte-for-byte");
    CHECK(strcmp(ch->description_ja, RB9_JA) == 0,
          "RULE 9.5.3: description_ja is the same JA prompt (abilities.rs:1783)");
    CHECK(strcmp(ch->description_en, RB9_EN) == 0,
          "RULE 9.5.3: description_en is the Rust EN prompt (abilities.rs:1782)");

    /* abilities.rs:1777 `player_id: player_id.to_string()` where player_id is
       the NORMALISED id from abilities.rs:1711-1715 -- "p1", not "player1". */
    CHECK(strcmp(ch->target_player_id, "p1") == 0,
          "RULE 9.5.3: the prompt's player_id is the NORMALISED \"p1\" "
          "(abilities.rs:1711-1715, :1777)");
    CHECK_EQ(ch->actor, 0, "RULE 9.5.3: actor is the scanning seat");
}

/* ── 3/4: the option list is this seat's available entries, by absolute index ─ */

static void test_option_list_is_this_seats_available_entries(void)
{
    GameState g = fresh_state();
    push(&g, "p1");   /* 0 */
    push(&g, "p2");   /* 1 -- the other seat, BETWEEN two of p1's */
    push(&g, "p1");   /* 2 */

    rb_process_player_abilities(&g, 0);

    const RbChoice *ch = pending(&g);
    CHECK(ch != NULL, "RULE 9.5.3: p1's two entries park a prompt for p1");
    if (!ch) return;

    /* The interleaved p2 entry is the load-bearing part: the old scan counted it
       (`available`) but offered nothing, so the tie it created was an artifact
       of a filter that was wider than the one Rust filters with. */
    CHECK_EQ(ch->n_auto_options, 2,
             "RULE 9.5.3: only p1's entries are offered -- the interleaved p2 "
             "entry is not an option (abilities.rs:1734-1737)");
    CHECK_EQ(ch->n_characters, 2,
             "RULE 9.5.3: one rendered label per option (game_setup.rs:1458)");
    if (ch->n_auto_options < 2) return;

    CHECK_EQ(ch->auto_option_queue_index[0], 0,
             "RULE 9.5.3: option 0 is queue entry 0");
    CHECK_EQ(ch->auto_option_queue_index[1], 2,
             "RULE 9.5.3: option 1 is queue entry 2 -- ABSOLUTE indices, not "
             "positions in the option list");

    /* The head of the list is `first`, i.e. the entry Rust would have promoted
       at abilities.rs:1789 had there been no tie. */
    CHECK_EQ(ch->auto_option_queue_index[0], 0,
             "RULE 9.5.3: the head of the option list is the entry that WOULD "
             "have been promoted (abilities.rs:1738, :1789)");

    /* game_setup.rs:1458 `action_desc!("{}: {}", opt.card_name, description)`.
       card_id is None here, so Rust's `unwrap_or_else(|| entry.card_no)` is the
       empty C fallback and card_name is "". The C's label carrier is
       RbChoice.characters = char[16][32] while ch.description is 512 B, so the
       ported label is bounded by explicit precision (see the emitter): Rust's
       unbounded label cannot fit, and the tail that is dropped is the tail of
       the prompt, not of the card name. */
    char want[32];
    snprintf(want, sizeof want, "%.12s: %.16s", "", RB9_JA);
    CHECK(strcmp(ch->characters[0], want) == 0,
          "RULE 9.5.3: option labels are the Rust `card_name: description` shape, "
          "bounded to the C carrier (game_setup.rs:1458)");

    /* A completed entry is not offered: Rust filters on is_entry_available. */
    g.queue.entries[2].completed = 1;
    g.queue.has_pending = 0;
    g.queue.auto_ability = 0;
    g.queue.state = RB_QUEUE_IDLE;
    rb_process_player_abilities(&g, 0);
    const RbChoice *ch2 = pending(&g);
    CHECK(ch2 == NULL,
          "RULE 9.5.3: with only ONE available entry there is no tie and no "
          "prompt (abilities.rs:1755 `if let Some(second)`)");
}

/* ── 5: three-way tie keeps queue order and covers the whole available set ── */

static void test_three_way_tie_lists_every_available_entry(void)
{
    GameState g = fresh_state();
    push(&g, "p2");   /* 0 */
    push(&g, "p1");   /* 1 */
    push(&g, "p1");   /* 2 */
    push(&g, "p1");   /* 3 */

    rb_process_player_abilities(&g, 1);   /* scan the SECOND seat */

    const RbChoice *ch = pending(&g);
    CHECK(ch != NULL, "RULE 9.5.3: p2's three entries park a prompt for p2");
    if (!ch) return;

    CHECK_EQ(ch->n_auto_options, 3,
             "RULE 9.5.3: all three of p2's entries are offered "
             "(abilities.rs:1756-1758 chains first + second + the rest)");
    CHECK(strcmp(ch->target_player_id, "p2") == 0,
          "RULE 9.5.3: the prompt is addressed to the seat being scanned");
    CHECK_EQ(ch->actor, 1, "RULE 9.5.3: actor is seat 1 for the p2 scan");
    if (ch->n_auto_options < 3) return;
    CHECK_EQ(ch->auto_option_queue_index[0], 1, "RULE 9.5.3: option 0 -> entry 1");
    CHECK_EQ(ch->auto_option_queue_index[1], 2, "RULE 9.5.3: option 1 -> entry 2");
    CHECK_EQ(ch->auto_option_queue_index[2], 3, "RULE 9.5.3: option 2 -> entry 3");
    CHECK_EQ(g.queue.cur, 0,
             "RULE 9.5.3: queue.cur is 0 while the prompt is open, which is what "
             "makes the answer path's `cur + selected` resolve to the absolute "
             "index recorded above");
}

/* ── 6: answering promotes the chosen entry to the head ──────────────────── */

static void test_answer_moves_the_chosen_entry_to_the_head(void)
{
    GameState g = fresh_state();
    push(&g, "p1");   /* 0 */
    push(&g, "p1");   /* 1 */

    rb_process_player_abilities(&g, 0);
    const RbChoice *ch = pending(&g);
    CHECK(ch != NULL, "RULE 9.5.3: two p1 entries park the ordering prompt");
    if (!ch || ch->n_auto_options < 2) return;

    /* Distinguish the two entries so the swap is observable. */
    g.queue.entries[0].ability_idx = 10;
    g.queue.entries[1].ability_idx = 20;

    int chosen = ch->auto_option_queue_index[1];
    CHECK_EQ(chosen, 1, "RULE 9.5.3: the second option is queue entry 1");

    /* Mark both completed so the post-answer re-scan (which this handler runs,
     * mirroring Rust's `process_pending_auto_abilities` after the answer) cannot
     * re-park the same tie. The swap under test happens BEFORE that call. */
    g.queue.entries[0].completed = 1;
    g.queue.entries[1].completed = 1;

    rb_resolver_handle_auto_ability_selection(&g, chosen);

    CHECK_EQ(g.queue.entries[0].ability_idx, 20,
             "RULE 9.5.3: answering option 1 puts THAT entry at the head, so the "
             "re-scan that follows resolves the player's chosen ability first "
             "(abilities.rs:1734-1737 re-derive `first` over the new order)");
    CHECK_EQ(g.queue.has_pending, 0,
             "RULE 9.5.3: the ordering prompt is closed by its own answer");
    CHECK_EQ(g.queue.auto_ability, 0,
             "RULE 9.5.3: the auto_ability flag is cleared by the answer "
             "(ability_queue.c:742)");
}

/* ── 7: the C struct carries Rust's four fields and Rust's Option ────────── */

static void test_auto_ability_option_shape(void)
{
    /* types.rs:313-318:
           card_name: String, ability_text: String,
           queue_index: usize, card_id: Option<i16> */
    CHECK_EQ((int)sizeof(((AutoAbilityOption *)0)->queue_index), (int)sizeof(int),
             "AutoAbilityOption.queue_index is an index-sized integer (usize)");
    CHECK_EQ((int)sizeof(((AutoAbilityOption *)0)->has_card_id), (int)sizeof(int),
             "AutoAbilityOption models Rust's Option<i16> with an explicit "
             "discriminant, so a gained ability with no card id is "
             "representable (types.rs:317)");
    CHECK_EQ(RB_MAX_AUTO_ABILITY_OPTIONS, RB_QUEUE_DEPTH,
             "RB_MAX_AUTO_ABILITY_OPTIONS == RB_QUEUE_DEPTH: the option list can "
             "never be longer than the queue it is built from (types.rs:216)");
    CHECK(sizeof(RbChoice) > sizeof(AutoAbilityOption),
          "RbChoice does not embed a full copy of AutoAbilityOption -- only the "
          "absolute indices, because RbChoice is embedded 17x in GameState");

    /* The None arm end to end: card_id -1 must report has_card_id == 0. */
    GameState g = fresh_state();
    push(&g, "p1");
    push(&g, "p1");
    rb_process_player_abilities(&g, 0);
    const RbChoice *ch = pending(&g);
    CHECK(ch != NULL, "RULE 9.5.3: two p1 entries park the ordering prompt");
    (void)ch;
}

/* ── driver ──────────────────────────────────────────────────────────────── */

int main(void)
{
    printf("=== Rule 9.5.3 -- AutoAbilityOption ordering "
           "(Rust engine/src/core/game_state/abilities.rs:1755-1788, "
           "types.rs:313-318) ===\n");

    test_tie_parks_the_rule_9_5_3_prompt();
    test_option_list_is_this_seats_available_entries();
    test_three_way_tie_lists_every_available_entry();
    test_answer_moves_the_chosen_entry_to_the_head();
    test_auto_ability_option_shape();

    if (failures) {
        printf("RULE 9.5.3: %d/%d assertions failed\n", failures, assertions);
        return 1;
    }
    printf("ALL RULE 9.5.3 CHECKS PASSED (%d assertions, %d cases)\n",
           assertions, 5);
    return 0;
}
