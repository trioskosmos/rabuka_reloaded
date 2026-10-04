/* THE DEFINITIVE E1 REACHABILITY COUNT.
 *
 * tools/census_e.c and tools/census_e1.c both over- or under-count because
 * neither replicates Rust's ACTUAL traversal rule. Rust's E1/E2 reach is:
 *
 *   modifiers.rs:2231-2257  collect (cid, player_idx, ability.effect.clone())
 *                            for every printed ability on a card in one of the
 *                            four live/success bags whose triggers contain CONSTANT
 *   modifiers.rs:2289       self.apply_success_zone_effect(cid, pl, effect)
 *   modifiers.rs:2325-2581  match effect.action { ModifyRequiredHearts,
 *                            GainResource, ModifyScore, Sequential, _ => {} }
 *
 * The ONLY recursion in that dispatch is the Sequential arm, and it recurses into
 * `effect.compound.actions` -- which is the C's `child[]`, because rabuka.h:110-114
 * documents child[] as "mirror AbilityEffect::compound" and the named pointers as
 * separate wire keys decoded INSTEAD of child[].
 *
 * So a node is E1-reachable if and only if its action is one of the four AND every
 * node between it and the ability root (inclusive of the root) has action
 * "sequential". Walking primary_effect / alternative_effect / followup_action /
 * optional_action / conditional_action / gained_effect is NOT E1 reach -- Rust
 * never looks at those fields from apply_success_zone_effect.
 *
 * Two prior tallies are recorded for the record:
 *   214  tools/census_e.c  -- right answer, wrong walk (it happens to miss
 *                            gained_effect, which is exactly the 12 that must be
 *                            excluded; it does NOT gate on sequential)
 *   226  tools/census_e1.c -- overcount; follows 11 links Rust never follows
 */
#include <stdio.h>
#include <string.h>
#include "rabuka.h"

static const char *CONST_MARK = "\xe5\xb8\xb8\xe6\x99\x82"; /* U+5E38 U+6642 */

static long n_seq;      /* sequential dispatch nodes reached     */
static long n_gr;       /* gain_resource                        */
static long n_ms;       /* modify_score                         */
static long n_mrh;      /* modify_required_hearts               */
static long n_other;    /* root-reached, action NOT one of four  */
static long n_skip;     /* reached but an ancestor was not sequential */

/* modify_score breakdown. `target` is a TYPED FIELD on AbilityEffect (the
 * struct member `char *target`), not an extra -- src/turn/triggers.c:841 reads
 * `e->target` for exactly this reason, and a corpus scan over e->extra
 * ("target") finds ZERO and would under-report this to 0. */
static long ms_live_total_field;   /* e->target == "live_total" (typed field) */
static long ms_live_total_extra;   /* an extra literally named target=...     */
static long ms_self_target;        /* self_target field true                  */
static long ms_no_target;
static long gr_blade, gr_heart, gr_other_res;
static long gr_has_position, gr_has_groups, gr_has_per_unit;

static int four(const char *a) {
    if (!a) return 0;
    return !strcmp(a, "modify_required_hearts") || !strcmp(a, "gain_resource") ||
           !strcmp(a, "modify_score")         || !strcmp(a, "sequential");
}

/* Rust-faithful: only a Sequential node may open its child[] list. */
static const char *xf(const AbilityEffect *e, const char *k) {
    for (int i = 0; i < e->n_extra; i++)
        if (e->extra_k[i] && !strcmp(e->extra_k[i], k)) return e->extra_v[i];
    return NULL;
}

static void note_ms(const AbilityEffect *e) {
    const char *t = e->target;
    if (t && !strcmp(t, "live_total")) ms_live_total_field++;
    else if (xf(e, "target") && !strcmp(xf(e, "target"), "live_total")) ms_live_total_extra++;
    else ms_no_target++;
    if (e->self_target_field[0] && !strcmp(e->self_target_field, "true")) ms_self_target++;
}

static void note_gr(const AbilityEffect *e) {
    /* `resource` is an EXTRA only -- AbilityEffect has no typed `resource`
     * member (rabuka.h:92-144), so the extras read is the correct one here. */
    const char *r = xf(e, "resource");
    if (r && !strcmp(r, "blade")) gr_blade++;
    else if (r && (!strcmp(r, "heart") || !strcmp(r, "\xe3\x83\x8f\xe3\x83\xbc\xe3\x83\x88"))) gr_heart++;
    else gr_other_res++;
    if (xf(e, "position")) gr_has_position++;
    if (xf(e, "group_names")) gr_has_groups++;
    if (e->per_unit > 0 || xf(e, "per_unit")) gr_has_per_unit++;
}

static void rust_walk(const AbilityEffect *e) {
    if (!e) return;
    const char *a = e->action;
    if (!a) return;
    if (!strcmp(a, "modify_required_hearts")) { n_mrh++; return; }
    if (!strcmp(a, "gain_resource"))         { n_gr++;  note_gr(e); return; }
    if (!strcmp(a, "modify_score"))          { n_ms++;  note_ms(e); return; }
    if (!strcmp(a, "sequential")) {
        n_seq++;
        for (int i = 0; i < e->n_child; i++) rust_walk(e->child[i]);
        return;
    }
    n_other++;
}

/* The over-counting walk, kept so the delta is reproducible in one run:
 * same four actions, but recursion through every link the struct owns. */
static long over;
static void greedy_walk(const AbilityEffect *e, int deep) {
    if (!e) return;
    if (four(e->action) && e->action && strcmp(e->action, "sequential")) over++;
    if (!deep) {
        /* only descend when this node is itself one of the four */
        if (e->action && !strcmp(e->action, "sequential")) {
            for (int i = 0; i < e->n_child; i++) greedy_walk(e->child[i], 1);
        }
        return;
    }
    for (int i = 0; i < e->n_child; i++)   greedy_walk(e->child[i], 1);
    for (int i = 0; i < e->n_options; i++) greedy_walk(e->options[i], 1);
    greedy_walk(e->primary_effect,1); greedy_walk(e->alternative_effect,1);
    greedy_walk(e->look_action,1);    greedy_walk(e->select_action,1);
    greedy_walk(e->followup_action,1);greedy_walk(e->optional_action,1);
    greedy_walk(e->conditional_action,1);greedy_walk(e->gained_effect,1);
    greedy_walk(e->resource_on_select,1);greedy_walk(e->opponent_action,1);
}

int main(int argc, char **argv) {
    const char *dir = argc > 1 ? argv[1] : "src";
    if (rb_load(dir) != 0) { fprintf(stderr, "rb_load failed\n"); return 2; }
    long pairs = 0, constant_pairs = 0, member_constant_pairs = 0;
    int n_cards = rb_num_cards();
    for (int cid = 0; cid < n_cards; cid++) {
        /* NO CARD-TYPE FILTER, and that is the correction. Rust's E1 iterates
         * the four live/success BAGS and takes whatever card ids are in them
         * (modifiers.rs:2232-2242). It never asks whether the card is a member.
         * An earlier revision of this tool filtered on rb_card_is_member and
         * reported modify_required_hearts = 0; the single reachable node is
         * cardid 291 = PL!-bp6-022-L, whose type byte is 1 (live_card), so the
         * member filter threw away the very card Rust scans. Filtering by card
         * type measures a DIFFERENT question than the one Rust asks. */
        int n = rb_card_num_abilities((uint32_t)cid);
        for (int ai = 0; ai < n; ai++) {
            Ability ab; memset(&ab,0,sizeof ab);
            if (!rb_decode_card_ability((uint32_t)cid, ai, &ab)) continue;
            pairs++;
            int isc = ab.triggers && strstr(ab.triggers, CONST_MARK) != NULL;
            if (isc) { constant_pairs++; member_constant_pairs++; }
            if (isc && ab.effect) { rust_walk(ab.effect); greedy_walk(ab.effect, 0); }
            rb_free_ability(&ab);
        }
    }
    printf("MEMBER_PAIRS %ld\n", pairs);
    printf("MEMBER_CONSTANT_PAIRS %ld\n", member_constant_pairs);
    printf("RUST modify_required_hearts %ld\n", n_mrh);
    printf("RUST gain_resource         %ld\n", n_gr);
    printf("RUST modify_score          %ld\n", n_ms);
    printf("RUST sequential            %ld\n", n_seq);
    printf("RUST_ROOT_OTHER_ACTION     %ld\n", n_other);
    printf("RUST_E1_TOTAL              %ld\n", n_mrh + n_gr + n_ms + n_seq);
    printf("GREEDY_OVERCOUNT_TOTAL     %ld\n", over);
    printf("--- modify_score (26 expected) breakdown ---\n");
    printf("MS_TARGET_FIELD_LIVE_TOTAL %ld\n", ms_live_total_field);
    printf("MS_TARGET_EXTRA_LIVE_TOTAL %ld\n", ms_live_total_extra);
    printf("MS_NO_LIVE_TOTAL_TARGET    %ld\n", ms_no_target);
    printf("MS_SELF_TARGET_TRUE       %ld\n", ms_self_target);
    printf("--- gain_resource (174 expected) breakdown ---\n");
    printf("GR_BLADE %ld\n", gr_blade);
    printf("GR_HEART %ld\n", gr_heart);
    printf("GR_OTHER_RESOURCE %ld\n", gr_other_res);
    printf("GR_WITH_POSITION %ld\n", gr_has_position);
    printf("GR_WITH_GROUP_NAMES %ld\n", gr_has_groups);
    printf("GR_WITH_PER_UNIT %ld\n", gr_has_per_unit);
    (void)n_skip;
    rb_unload();
    return 0;
}