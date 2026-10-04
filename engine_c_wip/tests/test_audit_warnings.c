/* Cluster W - permanent regression guard for the bug-predicting-warning fixes.
 *
 * Run: make t T=audit_warnings
 *
 * Every CHECK here is red against the pre-W tree and green against this one, or
 * it is an invariant that no future edit may break. Nothing here re-implements a
 * decoder or a reader: the census walks the SHIPPED bytecode through the
 * engine's own rb_decode_card_ability, and the layout check calls the function
 * data.c itself uses to place its four buffers.
 *
 * Rows:
 *   A  items 1/2/8  reachability of the three dead/wrong guards, pinned so a
 *                   future regeneration cannot quietly make a fix unreachable
 *   B  items 3/4    the const-correctness of the two aliased locals, checked by
 *                   CALLING the functions rather than by reading the source
 *   C  items 5-7    the data.c scratch-layout invariant, machine-checked
 *   D  the gate's own two structural promises (see the bottom of the file)
 *   E  the pay_cost_all:discard_all prompt is ANSWERED end to end -- the two
 *                   corpus abilities whose prompt neither dispatcher could route
 */
#include <stdio.h>
#include <string.h>
#include "rabuka.h"
#include "test_game.h"

/* Cluster W exports these two from src/core/data.c. include/rabuka.h is main's
   file, so the declarations are local and EXPLICIT -- the same shape
   src/game/game_setup.c:208 uses. An implicit declaration here would itself be
   the bug class this suite guards against. */
size_t rb_card_no_scratch_bytes(size_t len);
int    rb_card_no_scratch_layout_ok(size_t len);

/* W3's subject. src/ability/condition.c:2336 defines it; NO header declares it
   and it has ZERO callers, because checklist D3 ("wire rb_check_heart_type_all
   / rb_check_heart_colors -- needs Rust's negation-skip exceptions at
   condition.rs:610-623") is still open. So W3's const fix is correct but
   measurement-neutral today: the function is compiled and unreachable. Declared
   here so that the moment D3 wires it, this suite is already exercising it
   rather than the first caller discovering a broken predicate. */
int rb_check_heart_type_all(const struct GameState *g, int actor,
                            const Condition *c, int negation);
/* W4's subject. Defined at src/ability/choice.c:5513 with a file-scope forward
   declaration at :407 and two call sites (:1017, :5247); still undeclared in the
   header for the same header-bill reason as the two above. */
void rb_resolver_handle_number_selection(GameState *g, int selected);

static int n_checks = 0, n_failed = 0;

static void ck(int cond, const char *what, const char *detail) {
    n_checks++;
    if (cond) return;
    n_failed++;
    printf("FAIL: audit_warnings: %s: %s\n", what, detail);
}
static void ckeq(long got, long want, const char *what) {
    n_checks++;
    if (got == want) return;
    n_failed++;
    printf("FAIL: audit_warnings: %s: (got %ld expected %ld)\n", what, got, want);
}

/* ── A. the discard-location guard: is_stale_discard_condition ─────────────
   Rust twin: core/game_state/abilities.rs:486-501.
   C site: src/core/game_state_abilities.c queue_zone_abilities.
   The C is missing TWO of Rust's four terms and both are load-bearing, so the
   node counts they select are pinned here. If a regeneration makes either set
   empty, the corresponding fix has become unreachable and this suite must say
   so instead of letting the fix rot into dead code. */
struct DiscardCensus { long nodes, guard_hit, flat_only, trigger_event_only,
                       preceding_moved, trig_and_pred; };

static const char *cond_str(const Condition *c, const char *key) {
    if (!c) return NULL;
    for (uint32_t i = 0; i < c->n_fields; i++)
        if (c->fields[i].key && !strcmp(c->fields[i].key, key)
            && c->fields[i].v.tag == RB_TAG_STR)
            return c->fields[i].v.s;
    return NULL;
}
static const Condition *cond_sub(const Condition *c, const char *key) {
    if (!c) return NULL;
    for (uint32_t i = 0; i < c->n_fields; i++)
        if (c->fields[i].key && !strcmp(c->fields[i].key, key)
            && (c->fields[i].v.tag == RB_TAG_OBJVAR || c->fields[i].v.tag == RB_TAG_OBJECT)
            && c->fields[i].v.cond)
            return c->fields[i].v.cond;
    return NULL;
}
static void walk_discard(const Condition *c, struct DiscardCensus *k, int depth) {
    if (!c || depth > 8) return;
    k->nodes++;
    const char *flat = cond_str(c, "location");
    const char *te   = cond_str(cond_sub(c, "trigger_event"), "location");
    const char *ct   = cond_str(c, "card_type");
    const char *tg   = cond_str(c, "target");
    const char *src  = cond_str(c, "source");
    int card_term = (ct && !strcmp(ct, "member_card")) || (tg && !strcmp(tg, "self"));
    int is_discard = flat ? !strcmp(flat, "discard") : (te && !strcmp(te, "discard"));
    if (is_discard && card_term) {
        k->guard_hit++;
        int via_te = (flat == NULL) && te && !strcmp(te, "discard");
        if (via_te) k->trigger_event_only++; else k->flat_only++;
        if (src && !strcmp(src, "preceding_moved")) {
            k->preceding_moved++;
            if (via_te) k->trig_and_pred++;
        }
    }
    for (uint32_t i = 0; i < c->n_fields; i++) {
        if ((c->fields[i].v.tag == RB_TAG_OBJVAR || c->fields[i].v.tag == RB_TAG_OBJECT)
            && c->fields[i].v.cond) walk_discard(c->fields[i].v.cond, k, depth + 1);
        if (c->fields[i].v.tag == RB_TAG_ARRAY)
            for (uint32_t j = 0; j < c->fields[i].v.arr_n; j++)
                if ((c->fields[i].v.arr[j].tag == RB_TAG_OBJVAR
                     || c->fields[i].v.arr[j].tag == RB_TAG_OBJECT)
                    && c->fields[i].v.arr[j].cond)
                    walk_discard(c->fields[i].v.arr[j].cond, k, depth + 1);
    }
}
static int eff_flag(const AbilityEffect *e, const char *k) {
    for (int i = 0; i < e->n_extra; i++)
        if (e->extra_k[i] && !strcmp(e->extra_k[i], k) && e->extra_v[i]
            && (!strcmp(e->extra_v[i], "true") || !strcmp(e->extra_v[i], "1"))) return 1;
    return 0;
}
static long walk_hand_all(const AbilityEffect *c, int depth) {
    if (!c || depth > 12) return 0;
    long n = (c->source && !strcmp(c->source, "hand") && eff_flag(c, "all")) ? 1 : 0;
    for (int i = 0; i < c->n_child; i++)  n += walk_hand_all(c->child[i], depth + 1);
    for (int i = 0; i < c->n_options; i++) n += walk_hand_all(c->options[i], depth + 1);
    return n;
}

/* the C prompt producer's own gate: cost.c:1120 `is_from_hand && is_all` */
static void row_a(void) {
    struct DiscardCensus k;
    memset(&k, 0, sizeof(k));
    long hand_all_nodes = 0, hand_all_ab = 0, npairs = 0;
    for (uint32_t cid = 0; cid < 4000; cid++) {
        int n = rb_card_num_abilities(cid);
        if (n <= 0) { if (n < 0) break; continue; }
        for (int i = 0; i < n; i++) {
            Ability a;
            memset(&a, 0, sizeof(a));
            if (!rb_decode_card_ability(cid, i, &a)) continue;
            npairs++;
            if (a.effect && a.effect->condition) walk_discard(a.effect->condition, &k, 0);
            long h = walk_hand_all(a.cost, 0);
            if (h) { hand_all_nodes += h; hand_all_ab++; }
            rb_free_ability(&a);
        }
    }
    ckeq((long)rb_num_abilities(), 985, "A0 abilities walked (shipped bytecode)");
    ckeq(npairs, 2116, "A0 card-ability pairs walked");
    /* 37 guard-reachable nodes; 16 of them were decided by the WRONG predicate
       before the fix (27 via flat location only, 10 via trigger_event only, and
       6 exempted by Rust's preceding_moved term -- disjoint from the 10). */
    ckeq(k.guard_hit, 37, "A1 guard-reachable condition nodes (location=discard && member_card|self)");
    ckeq(k.flat_only, 27, "A2 of those reached via the flat `location` key");
    ckeq(k.trigger_event_only, 10, "A3 of those reachable ONLY via trigger_event.location (term b)");
    ckeq(k.preceding_moved, 6, "A4 of those exempted by Rust source==preceding_moved (term a)");
    ckeq(k.trig_and_pred, 0, "A5 terms (a) and (b) select DISJOINT node sets");
    ck(k.guard_hit > 0, "A6 the W8 guard is reachable at all",
       "a guard with 0 reachable nodes is not a fix, it is dead code");
    /* Item 2's producer half. The routing fix cannot fire until cost.c sets
       pending.target; these 2 nodes are why that producer half matters. */
    ckeq(hand_all_nodes, 2, "A7 hand+all cost nodes (the C prompt producer's reach)");
    ckeq(hand_all_ab, 2, "A8 abilities carrying one");
    ck(rb_select_target_kind_from_str("pay_cost_all:discard_all") >= 0,
       "A9 the exact-match table knows Rust's literal",
       "enums.rs:705 spells it pay_cost_all:discard_all with a COLON; the C's old "
       "check used an underscore and so could never match anything");
    ck(rb_select_target_kind_from_str("pay_cost_all_discard") < 0,
       "A10 the old underscore spelling is NOT a recognised target",
       "proves the old guard was unreachable, not merely unidiomatic");
    ck(rb_select_target_kind_from_str("") < 0,
       "A11 an empty pending.target does not route to a handler",
       "W2's replacement is a presence test that can actually be false");
}

/* ── B. the discarded-const sites ───────────────────────────────────────────
   W3  src/ability/condition.c rb_check_heart_type_all  (const RbPlayer *)
   W4  src/ability/choice.c    rb_resolver_handle_number_selection
   Both were "initialisation discards const". Behaviour is unchanged either way
   -- the locals are never written through and never escape -- so the assertion
   is a CALL-SHAPE contract, not a value: the predicate must still answer the
   question Rust's check_heart_type_all answers (card.rs), and the number handler
   must still record the answer and leave no prompt behind. */
static void row_b(void) {
    static TestGame tg;
    test_game_new(&tg);

    Condition c;
    memset(&c, 0, sizeof(c));
    /* location=stage, heart_type=all, negation=0: TRUE iff a stage member
       carries a Heart00 base heart. */
    c.n_fields = 3;
    c.fields[0].key = "location";     c.fields[0].v.tag = RB_TAG_STR; c.fields[0].v.s = "stage";
    c.fields[1].key = "heart_type";   c.fields[1].v.tag = RB_TAG_STR; c.fields[1].v.s = "all";
    c.fields[2].key = "negation";    c.fields[2].v.tag = RB_TAG_TRUE;

    ck(rb_check_heart_type_all(&tg.state, 0, &c, 0) == 0,
       "B1 an empty stage satisfies no all-heart member",
       "an empty stage has no Heart00 base heart, so the predicate must be false");

    /* a non-stage location short-circuits to TRUE (Rust reads Zone::from_str and
       the non-Stage arm answers true) -- and that early return happens BEFORE
       the `const RbPlayer *P` line, so it also proves the const change did not
       move a side effect. */
    Condition c2 = c;
    c2.fields[0].v.s = "hand";
    ck(rb_check_heart_type_all(&tg.state, 0, &c2, 0) == 1,
       "B2 a non-stage location returns true without touching the player",
       "rb_check_heart_type_all's location gate is upstream of the const alias");

    /* negation=1 on an empty stage. Rust's negation arm is
       `target_ids.iter().any(|id| !card_has_all_heart(id))` (predicates.rs, the
       `get_negation()` branch of check_heart_type_all) -- an ANY over the ids,
       NOT a fold of the positive arm. `any` over an EMPTY list is FALSE, so the
       correct answer on an empty stage is 0. An earlier revision of this check
       asserted 1 on the reasoning that "negation of false is true"; that is the
       fold of `!any(...)`, which is a DIFFERENT predicate from Rust's
       `any(!...)`. Corrected here against the reference, not relaxed: the
       assertion still exercises the negation arm and now pins Rust's exact
       `.any(|id| !card_has_all_heart(id))` shape. */
    ckeq(rb_check_heart_type_all(&tg.state, 0, &c, 1), 0,
         "B3 the negated form on an empty stage is false (Rust folds .any(!has_all), "
         "and any over zero ids is false)");

    /* W4: a number answer records the value, clears the prompt and leaves the
       queue idle. Called with no prompt open, which is the shape the const
       change cannot alter but which would catch a mangled local. */
    tg.state.queue.resume_parent = NULL;
    tg.state.queue.resume_child = -1;
    rb_resolver_handle_number_selection(&tg.state, 7);
    ckeq(tg.state.queue.choice_result, 7, "B4 number selection records the answer");
    ck(!rb_has_pending_choice(&tg.state), "B5 number selection leaves no prompt", "");
    ckeq(tg.state.queue.resume_mode, 0, "B6 number selection clears resume_mode");
}

/* ── C. items 5-7, the data.c scratch layout ────────────────────────────────
   The three -Wrestrict diagnostics are a GCC FALSE POSITIVE: it reasons about
   the whole OBJECT `scratch` and cannot be told "same object, disjoint
   sub-ranges". The proof is the range arithmetic, and here it is EVALUATED for
   every len instead of asserted once in a comment. */
static void row_c(void) {
    long bad = 0;
    for (size_t len = 0; len <= 256; len++)
        if (!rb_card_no_scratch_layout_ok(len)) bad++;
    ckeq(bad, 0, "C1 all four scratch sub-ranges are pairwise disjoint and inside "
                 "the allocation for len 0..256");
    ckeq((long)rb_card_no_scratch_bytes(0), 10, "C2 scratch size at len 0");
    /* the corpus's longest card_no is 25 bytes (verified in data.c's own comment
       against cards/cards.json); 6*25+10 = 160. */
    ckeq((long)rb_card_no_scratch_bytes(25), 160, "C3 scratch size at len 25 (longest card_no)");
    ckeq((long)rb_card_no_scratch_bytes(63), 388, "C4 scratch size at len 63");

    /* The premise the argument leans on: rb_card_normalize_no never expands, so
       strlen(normalized) <= len, so base_len <= len, so `base` fits its len+1
       slot with room for its NUL. Checked over every corpus card_no.

       READ THE CARD NUMBER WITH rb_card_string(), NOT rb_get_string().
       `card_no_idx` indexes the CARDS string table (data.c g_card_strings);
       rb_get_string() indexes the ABILITIES table (data.c g_strings). The two
       are disjoint namespaces -- measured over all 2639 cards the two
       accessors agree 0 times -- so reading the card number through the wrong
       one yields another card's number for almost every id. That mistake is
       what produced the old "671 of 2639 resolve, 534 to a lower index, 1968
       unresolved" figures: the 1968 "card numbers" were never card numbers. */
    long expansions = 0, checked = 0;
    for (uint32_t i = 0; i < rb_num_cards(); i++) {
        Card cc;
        if (!rb_decode_card_by_index(i, &cc)) continue;
        const char *no = rb_card_string(cc.card_no_idx);
        if (no && *no) {
            size_t n = strlen(no);
            char out[512];
            if (n + 8 < sizeof(out)) {
                rb_card_normalize_no(no, out, n + 1);
                out[n + 1] = 0;
                size_t l = 0;
                while (out[l] && l < sizeof(out)) l++;
                checked++;
                if (l > n) expansions++;
            }
        }
        rb_free_card(&cc);
    }
    ckeq(expansions, 0, "C5 rb_card_normalize_no never expands (the base_len<=len premise)");
    ck(checked > 1600, "C6 the normalize check actually ran over the corpus",
       "a premise check over 0 card_nos proves nothing");

    /* rb_find_card_by_no over the REAL card numbers. MEASURED: it resolves
       2639 of 2639, every one of them to its OWN index -- 0 misses, 0 to a
       lower index, 0 to a higher one, and 0 duplicate card_no keys in the
       blob. The forgiving tail (base+rarity rebuild, lowest-prefix, substring)
       documented in include/test_game.h is therefore NOT what a corpus card
       number reaches: `card_no_exact` (data.c) hits first, every time. The
       tail still matters for keys the corpus does not contain -- it is what
       makes a mistyped print resolve instead of failing -- so it is not
       removed; it is simply not on this path, and the old claim that it
       dominates the corpus was a measurement artefact.

       Pinned as a contract, not as a numbering accident: "every card number the
       blob holds resolves to itself" is a property a future data.c edit could
       genuinely break (an off-by-one in card_no_at, a folded-vs-raw comparison,
       a scan bounded by the wrong count), and it is the property the rest of
       the test corpus silently relies on. */
    long checked_rt = 0, valid_idx = 0, non_det = 0, resolves = 0, own = 0, wrong = 0;
    for (uint32_t i = 0; i < rb_num_cards(); i++) {
        Card cc;
        if (!rb_decode_card_by_index(i, &cc)) continue;
        const char *no = rb_card_string(cc.card_no_idx);
        if (no && *no) {
            int a1 = rb_find_card_by_no(no);
            int a2 = rb_find_card_by_no(no);
            checked_rt++;
            if (a1 != a2) non_det++;
            if (a1 >= 0) {
                resolves++;
                if ((uint32_t)a1 < rb_num_cards()) valid_idx++;
                if ((uint32_t)a1 == i) own++; else wrong++;
            }
        }
        rb_free_card(&cc);
    }
    ckeq(non_det, 0, "C7 rb_find_card_by_no is deterministic over the whole corpus "
                    "-- a cross-spliced scratch would make it non-deterministic");
    ckeq(valid_idx, resolves, "C8 every non-negative answer is a valid card index");
    ck(resolves > 600, "C9 the resolution set is big enough for C7/C8 to mean anything",
       "a corpus cross-splice that only hit 3 cards would pass a tiny control");
    ckeq(checked_rt, 2639, "C10 every one of the blob's 2639 card numbers was looked up");
    ckeq(resolves, checked_rt, "C11 NO card number in the blob fails to resolve "
                              "(the old '1968 unresolved' were not card numbers)");
    ckeq(own, checked_rt, "C12 every corpus card number resolves to ITS OWN index "
                         "(the old '534 to a lower index' was the wrong string table)");
    ckeq(wrong, 0, "C13 no card number resolves to a different card");
}

/* ── D. the gate's own promises ────────────────────────────────────────────
   These do not test engine behaviour; they test that the gate added in this
   cluster cannot silently become the thing the brief forbade. */
static void row_d(void) {
    /* The gate's flag list must contain the nine bug-predicting classes and
       must NOT contain a blanket -Werror. tools/warnings_audit.sh takes the
       flags from the Makefile; a grep of the Makefile is the contract. */
    FILE *mf = fopen("Makefile", "r");
    ck(mf != NULL, "D1 Makefile is readable from the suite's working directory", "");    if (mf) {
        char line[512];
        int has_flags = 0, has_base = 0, has_test = 0, blanket_werror = 0;
        while (fgets(line, sizeof line, mf)) {
            if (strstr(line, "WARN_AUDIT_FLAGS")) has_flags = 1;
            if (strstr(line, "WARN_AUDIT_BASELINE")) has_base = 1;
            if (strstr(line, "warnings-audit-test")) has_test = 1;
            /* a blanket -Werror in a RULE would block every agent. Comment
               lines are skipped: the Makefile's own prose says "NOT a blanket
               -Werror", and matching that sentence would fail the suite for
               saying the right thing. */
            {
                const char *p = line;
                while (*p == ' ' || *p == '\t') p++;
                if (*p != '#' && strstr(line, "-Werror")
                    && !strstr(line, "Werror=array-bounds")
                    && !strstr(line, "Werror=maybe-uninitialized"))
                    blanket_werror = 1;
            }
        }
        fclose(mf);
        ck(has_flags, "D2 Makefile defines WARN_AUDIT_FLAGS", "");
        ck(has_base, "D3 Makefile defines the WARN_AUDIT_BASELINE ratchet", "");
        ck(has_test, "D4 Makefile defines warnings-audit-test", "");
        ck(!blanket_werror, "D5 no blanket -Werror in the Makefile",
           "a blanket -Werror with 113 warnings blocks every agent in the repo");
    }
    /* D6: both gate scripts must exist and be non-empty. A Makefile target that
       points at a missing script is a gate that silently does nothing. */
    FILE *a = fopen("tools/warnings_audit.sh", "r");
    FILE *b = fopen("tools/warnings_audit_tests.sh", "r");
    ck(a != NULL, "D6 tools/warnings_audit.sh exists", "make warnings-audit would fail");
    ck(b != NULL, "D7 tools/warnings_audit_tests.sh exists",
       "make warnings-audit-test would fail");
    if (a) fclose(a);
    if (b) fclose(b);
}

/* ── E. the pay_cost_all:discard_all prompt is ANSWERED, not merely raised ───
   Rows A7/A8 pin that 2 corpus nodes reach the C's hand-discard prompt
   producer (cost.c `is_from_hand && is_all`). The cards are **PL!SP-bp7-011-P**
   and **PL!SP-bp7-011-R** (blob ids 2336/2337), ability #0 each: a 登場 with
   cost `move_cards source=hand destination=discard all=true optional=true` and
   effect `draw_card`, measured through the engine's own rb_decode_card_ability
   on the SHIPPED bytecode.

   CORRECTION: an earlier revision of the census named these two nodes
   PL!N-bp5-021-N / PL!N-bp5-028-L. That name came from reading `card_no_idx`
   with rb_get_string() -- the ABILITIES string table -- instead of
   rb_card_string(), the CARDS one. Those two prints do carry the ability the
   census was counting, but they are NOT the cards whose cost reaches the C's
   hand-discard producer. The node count (2) was always right; the name was not.

   This row is the end-to-end half. A7-A11 prove the prompt is RAISED and that
   the routing table knows the literal; they cannot prove the prompt is
   ANSWERABLE. That needs both halves of W2 at once:
     producer  cost.c writes pending.target = "pay_cost_all:discard_all"  (W2b)
     consumer  choice.c routes on rb_select_target_kind_from_str(...)     (W2)
   With the consumer alone the prompt is raised and unanswerable; with the
   producer alone nothing consumes the key. The discriminator against the
   pre-fix tree is NOT "no crash" -- the old code also cleared the prompt -- it
   is WHETHER THE HAND WAS ACTUALLY DISCARDED. Old: the answer fell through to
   the "record chosen target then clear" default with an empty target, so the
   hand was untouched. Rust (cost/handlers.rs offer_all_hand_discard ->
   choice.rs handle_pay_cost_all_discard): the whole hand goes to the waitroom. */
/* the two W2 corpus nodes' effect is `draw_card`, so the deck must NOT be
           empty: a C draw from an empty deck recycles the waitroom, which would
           put the very cards the cost just discarded straight back into the hand
           and make the assertion below pass or fail for the wrong reason. */
static int bag_has(const RbBag *b, int id) {
    for (int i = 0; i < b->n; i++) if (b->cards[i] == id) return 1;
    return 0;
}

static void row_e(void) {
    /* PL!SP-bp7-011-P and PL!SP-bp7-011-R are DIFFERENT cards of one card, and
       both use test_id (the shared template): only one instance of each is
       needed and the template is the id whose ability #0 the census measured.
       The two FILLERS must be distinct instances -- the same id twice in one
       hand is not a two-card hand -- so they come from test_new_id, where the
       identity is irrelevant. */
    static const char *cases[2] = { "PL!SP-bp7-011-P", "PL!SP-bp7-011-R" };
    static const char *filler_no = "PL!N-PR-009-PR";
    for (int ci = 0; ci < 2; ci++) {
        static TestGame tg;
        test_card_copy_pool_reset();
        test_game_new(&tg);
        /* The SUBJECT is staged with test_add_to_stage (a raw placement that does
           not touch the hand), so the hand holds only the two fillers. Played
           from hand it would still be in hand when the whole-hand cost resolves
           and the cost would discard the member that is on stage, which Rust
           cannot do. test_id (the shared template) is correct for the subject:
           only ONE instance of each print is needed, and the copy pool's ids do
           not carry the ability links, so the template is the id whose ability
           #0 the census measured. The two FILLERS must be distinct instances --
           the same id twice in one hand is not a two-card hand -- so they come
           from test_new_id, where the identity is irrelevant. */
        int cid  = test_id(&tg, cases[ci]);
        int f1   = test_new_id(&tg, filler_no);
        int f2   = test_new_id(&tg, filler_no);
        int dk   = test_new_id(&tg, filler_no);   /* deck filler, a DIFFERENT id */
        ck(cid >= 0 && f1 >= 0 && f2 >= 0 && dk >= 0, "E0 fixture ids resolved",
           "a -1 here makes every hand assertion vacuous");
        if (cid < 0 || f1 < 0 || f2 < 0 || dk < 0) continue;
        ck(test_id_exact(&tg, cases[ci]) == cid,
           "E1 test_id_exact confirms the card identity, not a forgiving match",
           "rb_find_card_by_no's substring fallback would resolve a typo to another card");
        tg.state.p[0].deck.n = 0; tg.state.p[1].deck.n = 0;
        for (int i = 0; i < 20; i++) test_add_to_deck_pl(&tg, 0, dk);
        for (int i = 0; i < 20; i++) test_add_to_deck_pl(&tg, 1, dk);
        test_add_to_hand(&tg, f1);
        test_add_to_hand(&tg, f2);
        test_add_to_stage(&tg, 1, cid);
        test_fire_debut(&tg, cid);

        const RbChoice *ch = test_has_pending_choice(&tg) ? rb_get_pending_choice(&tg.state) : NULL;
        char what[256];
        snprintf(what, sizeof what,
                 "%s: the optional whole-hand cost raises a SelectTarget prompt "
                 "(got %s)", cases[ci], ch ? "one" : "NONE");
        ck(ch && ch->kind == RB_CHOICE_SELECT_TARGET, "E2", what);
        if (!ch || ch->kind != RB_CHOICE_SELECT_TARGET) continue;
        ck(!strcmp(ch->target, "pay_cost_all:discard_all"),
           "E3 pending.target carries Rust's EXACT literal",
           "the routing key is the target; an empty or differently-spelled one "
           "leaves the prompt unanswerable");
        snprintf(what, sizeof what,
                 "%s: answering PAY discards the whole hand (hand %d -> %d, "
                 "waitroom %d -> %d)", cases[ci],
                 tg.state.p[0].hand.n, -1, tg.state.p[0].discard.n, -1);

        int hand_before = tg.state.p[0].hand.n;
        int disc_before = tg.state.p[0].discard.n;
        rb_resume_with_choice(&tg.state, 1);          /* 1 == PAY in this shape */
        snprintf(what, sizeof what,
                 "%s: answering PAY sends BOTH hand cards to the waitroom "
                 "(hand %d->%d, waitroom %d->%d)", cases[ci], hand_before,
                 tg.state.p[0].hand.n, disc_before, tg.state.p[0].discard.n);
        ck(!rb_has_pending_choice(&tg.state),
           "E4 the prompt is CONSUMED by the answer, not re-raised", what);
        /* Identity, not counts: the cost's effect draws a card, so the hand
           length is not a stable observable. WHICH cards left the hand is. */
        ck(!bag_has(&tg.state.p[0].hand, f1) && !bag_has(&tg.state.p[0].hand, f2),
           "E5 neither hand card is still in hand after PAY", what);
        ck(bag_has(&tg.state.p[0].discard, f1) && bag_has(&tg.state.p[0].discard, f2),
           "E6 both hand cards are in the waitroom (RbPlayer::discard IS Rust's "
           "waitroom, rabuka.h)", what);
        ck(tg.state.p[0].stage[1] == cid,
           "E7 the member that raised the prompt is still on stage",
           "paying the cost must not discard the card that paid it");
    }

    /* The DECLINE arm, same prompt: an optional cost the player refuses must
       leave the hand alone. Rust's skip path never calls the discard. */
    static TestGame tg2;
    test_card_copy_pool_reset();
    test_game_new(&tg2);
    int cid = test_id(&tg2, cases[0]);
    int d1 = test_new_id(&tg2, filler_no);
    int d2 = test_new_id(&tg2, filler_no);
    int d3 = test_new_id(&tg2, filler_no);
    if (cid >= 0 && d1 >= 0 && d2 >= 0 && d3 >= 0) {
        tg2.state.p[0].deck.n = 0; tg2.state.p[1].deck.n = 0;
        for (int i = 0; i < 20; i++) test_add_to_deck_pl(&tg2, 0, d3);
        for (int i = 0; i < 20; i++) test_add_to_deck_pl(&tg2, 1, d3);
        test_add_to_hand(&tg2, d1);
        test_add_to_hand(&tg2, d2);
        test_add_to_stage(&tg2, 1, cid);
        test_fire_debut(&tg2, cid);
        ck(test_has_pending_choice(&tg2), "E8 the same prompt is raised again", "");
        if (test_has_pending_choice(&tg2)) {
            int hand_before = tg2.state.p[0].hand.n;
            int disc_before = tg2.state.p[0].discard.n;
            rb_resume_with_choice(&tg2.state, -1);      /* skip */
            ckeq(tg2.state.p[0].hand.n, hand_before,
                 "E9 declining an OPTIONAL cost discards nothing");
            ckeq(tg2.state.p[0].discard.n, disc_before,
                 "E10 declining an OPTIONAL cost adds nothing to the waitroom");
            ck(!rb_has_pending_choice(&tg2.state),
               "E11 the declined prompt is consumed too", "");
        }
    }
}

int main(void) {
    if (rb_load("src") < 0) { fprintf(stderr, "rb_load failed\n"); return 2; }
    row_a();
    row_b();
    row_c();
    row_d();
    row_e();
    rb_unload();
    printf("audit_warnings: %d checks, %d failed\n", n_checks, n_failed);
    return n_failed ? 1 : 0;
}
