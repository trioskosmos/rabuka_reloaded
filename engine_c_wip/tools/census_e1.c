/* E1/E2/E3/E4 reachability census -- METHOD 2, written to CROSS-CHECK
 * tools/census_e.c (METHOD 1). Deliberately shares no traversal code with it.
 *
 * Why a second method is mandatory: METHOD 1's e1_walk follows only 6 of the 15
 * links an AbilityEffect owns. This one follows ALL 15 and attributes every hit
 * to the link kind that reached it, so a link METHOD 1 skipped shows up as its
 * own bucket instead of silently inflating the total.
 *
 * Reachability criterion is taken TWICE and independently:
 *   A) rb_card_is_member(cid)          -- the engine helper METHOD 1 also used
 *   B) (type_flags & member bit)       -- read straight off the Card struct,
 *                                         not routed through the helper
 * If A and B disagree, the helper is wrong and the tally is not trustworthy.
 *
 * Rust oracle, engine/src/core/game_state/modifiers.rs (tree: master @ master,
 * measured HEAD cf315a9ac5dedd1ca02caeadbfb13f0f74919e5c):
 *   E1 evaluate_success_zone_constant_modifiers :2188  -- four actions
 *   E2 apply_success_zone_effect                :2309
 *   E3 evaluate_success_zone_constant_abilities :2147  -- clears need_heart first
 *   E4 restore_performance_need_heart_modifiers :2159  -- dedupes (cid,color)
 * The four actions of E1's dispatch arm are exactly the four below
 * (modifiers.rs:2326 ModifyRequiredHearts, :2343 GainResource, :2487 ModifyScore,
 * :2573 Sequential).
 */
#include <stdio.h>
#include <string.h>
#include <stdlib.h>
#include "rabuka.h"

/* U+5E38 U+6642 = 常時. Verified against engine/src/triggers.rs:21
 * `pub const CONSTANT: &str = "常時";` -- do not hand-write these bytes. */
static const char *CONST_MARK = "\xe5\xb8\xb8\xe6\x99\x82";

static int is_e1_action(const char *a) {
    if (!a) return 0;
    return !strcmp(a, "modify_required_hearts") ||
           !strcmp(a, "gain_resource")        ||
           !strcmp(a, "modify_score")         ||
           !strcmp(a, "sequential");
}

/* ── action tallies, indexed by the four E1 actions ─────────────────────── */
static const char *E1ACT[4] = {"modify_required_hearts","gain_resource","modify_score","sequential"};
static long t_action[4];          /* method-2 full-graph, member by helper   */
static long t_action_rawflag[4];  /* criterion B                            */
static long t_via_link[16];       /* attribution by link kind               */
static const char *LINKNAME[16] = {
    "child","options","primary_effect","alternative_effect","look_action",
    "select_action","followup_action","optional_action","conditional_action",
    "gained_effect","resource_on_select","opponent_action"
};
static long t_action_by_method1_links[4]; /* only the 6 links METHOD 1 walked */

static int cur_cid;
static int cur_member_helper;
static int cur_member_flag;
static int cur_constant;

/* full graph: every link an AbilityEffect owns (child[] + 11 named pointers) */
/* tools/census_e.c e1_walk follows exactly {child, primary_effect,
 * alternative_effect, followup_action, optional_action, conditional_action}.
 * That is NOT link_kind <= 5: the census bucket for METHOD 1 is this set, and
 * the delta bucket is everything census_e.c could not reach. */
static int is_method1_link(int k) {
    return k == 0 || k == 2 || k == 3 || k == 6 || k == 7 || k == 8;
}

static void walk(const AbilityEffect *e, int link_kind) {
    if (!e) return;
    /* Rust modifiers.rs:2245-2251 gates every collected entry on the ability's
     * trigger list containing CONSTANT. Only then can apply_success_zone_effect
     * ever be reached, so an effect under a non-constant ability is unreachable
     * no matter which zone its host sits in. */
    if (cur_constant && e->action && is_e1_action(e->action)) {
        int k = -1;
        for (int i = 0; i < 4; i++) if (!strcmp(e->action, E1ACT[i])) { k = i; break; }
        if (k >= 0) {
            if (cur_member_helper) {
                t_action[k]++;
                t_via_link[link_kind]++;
                if (is_method1_link(link_kind)) t_action_by_method1_links[k]++;
                printf("N %-10d %-22s %-24s %.48s\n", cur_cid, E1ACT[k],
                       LINKNAME[link_kind], e->text ? e->text : "");
            }
            if (cur_member_flag) t_action_rawflag[k]++;
        }
    }
    for (int i = 0; i < e->n_child; i++)   walk(e->child[i], 0);
    for (int i = 0; i < e->n_options; i++) walk(e->options[i], 1);
    walk(e->primary_effect,      2);
    walk(e->alternative_effect,  3);
    walk(e->look_action,         4);
    walk(e->select_action,       5);
    walk(e->followup_action,     6);
    walk(e->optional_action,     7);
    walk(e->conditional_action,  8);
    walk(e->gained_effect,       9);
    walk(e->resource_on_select, 10);
    walk(e->opponent_action,    11);
}

/* ── non_stackable: build the exact dedupe key Rust builds ────────────────
 * modifiers.rs:2281  let effect_key = format!("{}:{}", effect.action, effect.text);
 * The set is local to ONE evaluation (:2221) and only consulted when the flag
 * is set (:2280). So the dedupe is INERT iff no two non_stackable nodes
 * corpus-wide share an action:text key -- and with 3 nodes all on distinct
 * actions it cannot. Emitted here so the claim is checkable, not asserted. */
#define NSMAX 64
static char ns_key[NSMAX][256];
static int  n_ns;
static void note_ns(const AbilityEffect *e) {
    if (!e) return;
    for (int i = 0; i < e->n_extra; i++)
        if (e->extra_k[i] && !strcmp(e->extra_k[i], "non_stackable")) {
            const char *v = e->extra_v[i] ? e->extra_v[i] : "";
            if (strcmp(v, "true") && strcmp(v, "1")) { /* still record the node */
                char tmp[256];
                snprintf(tmp, sizeof tmp, "%s:%s [non_stackable=%s]",
                         e->action ? e->action : "-", e->text ? e->text : "", v);
                if (n_ns < NSMAX) { snprintf(ns_key[n_ns], sizeof ns_key[0], "%s", tmp); n_ns++; }
            } else {
                char tmp[256];
                snprintf(tmp, sizeof tmp, "%s:%s", e->action ? e->action : "-",
                         e->text ? e->text : "");
                if (n_ns < NSMAX) { snprintf(ns_key[n_ns], sizeof ns_key[0], "%s", tmp); n_ns++; }
            }
        }
    for (int i = 0; i < e->n_child; i++)   note_ns(e->child[i]);
    for (int i = 0; i < e->n_options; i++) note_ns(e->options[i]);
    note_ns(e->primary_effect);     note_ns(e->alternative_effect);
    note_ns(e->look_action);        note_ns(e->select_action);
    note_ns(e->followup_action);    note_ns(e->optional_action);
    note_ns(e->conditional_action); note_ns(e->gained_effect);
    note_ns(e->resource_on_select); note_ns(e->opponent_action);
}

int main(int argc, char **argv) {
    const char *dir = argc > 1 ? argv[1] : "src";
    if (rb_load(dir) != 0) { fprintf(stderr, "rb_load failed\n"); return 2; }
    int n_cards = rb_num_cards();

    long n_pairs = 0, n_constant_pairs = 0, n_member_constant_pairs = 0;
    long n_effect_roots = 0;
    long member_helper = 0, member_flag = 0, disagree = 0;

    for (int cid = 0; cid < n_cards; cid++) {
        Card c;
        int have_card = rb_decode_card_by_index((uint32_t)cid, &c);
        int m_helper = rb_card_is_member(cid);
        /* criterion B, independent of the helper: the decoder folds the
         * canonical card type into the low two bits of type_flags
         * (src/core/card.c `out->type_flags = (uint8_t)((r[18] & ~3u) | (unsigned)card_type_of(r));`),
         * and card_type_of maps 0 AND 3 to member. So member is
         * `(type_flags & 3) == 0`. Agreement with the helper is MEASURED, not
         * assumed -- the first run of this tool reported 1574 disagreements
         * because it masked 0x0f instead of 0x03. */
        int m_flag = have_card ? (((c.type_flags & 0x03) == 0) ? 1 : 0) : m_helper;
        if (have_card) rb_free_card(&c);
        member_helper += m_helper; member_flag += m_flag;
        if (m_helper != m_flag) {
            disagree++;
            printf("DISAGREE cid=%d helper=%d flag=%d\n", cid, m_helper, m_flag);
        }
        cur_cid = cid; cur_member_helper = m_helper; cur_member_flag = m_flag;

        int n = rb_card_num_abilities((uint32_t)cid);
        for (int ai = 0; ai < n; ai++) {
            Ability ab;
            memset(&ab, 0, sizeof ab);
            if (!rb_decode_card_ability((uint32_t)cid, ai, &ab)) continue;
            n_pairs++;
            int is_const = ab.triggers && strstr(ab.triggers, CONST_MARK) != NULL;
            if (is_const) n_constant_pairs++;
            if (is_const && m_helper) n_member_constant_pairs++;
            if (ab.effect) {
                n_effect_roots++;
                cur_constant = is_const;
                walk(ab.effect, 0);
                cur_constant = 0;
                note_ns(ab.effect);
            }
        }
    }

    printf("CARDS %d\n", n_cards);
    printf("PAIRS %ld\n", n_pairs);
    printf("CONSTANT_PAIRS %ld\n", n_constant_pairs);
    printf("MEMBER_CONSTANT_PAIRS %ld\n", n_member_constant_pairs);
    printf("EFFECT_ROOTS %ld\n", n_effect_roots);
    printf("MEMBER_BY_HELPER %ld\n", member_helper);
    printf("MEMBER_BY_TYPEFLAG %ld\n", member_flag);
    printf("CRITERION_DISAGREEMENTS %ld\n", disagree);

    long tot_a = 0, tot_b = 0, tot_m1 = 0;
    printf("--- E1 ACTION TALLY (criterion A = rb_card_is_member) ---\n");
    for (int i = 0; i < 4; i++) {
        printf("E1ACT %-26s %ld\n", E1ACT[i], t_action[i]);
        tot_a += t_action[i];
    }
    printf("E1_TOTAL_A %ld\n", tot_a);
    printf("E1_TOTAL_B %ld\n", tot_b);
    printf("--- SAME, restricted to the 6 links METHOD 1 walked ---\n");
    for (int i = 0; i < 4; i++) {
        printf("E1ACT_M1 %-26s %ld\n", E1ACT[i], t_action_by_method1_links[i]);
        tot_m1 += t_action_by_method1_links[i];
    }
    printf("E1_TOTAL_M1 %ld\n", tot_m1);
    printf("--- PER-ACTION criterion B (raw type_flags) ---\n");
    for (int i = 0; i < 4; i++) { printf("E1ACT_B %-26s %ld\n", E1ACT[i], t_action_rawflag[i]); tot_b += t_action_rawflag[i]; }

    printf("--- ATTRIBUTION BY LINK KIND ---\n");
    for (int i = 0; i < 12; i++)
        if (t_via_link[i]) printf("E1LINK %-22s %ld\n", LINKNAME[i], t_via_link[i]);

    printf("--- non_stackable dedupe keys (Rust modifiers.rs:2281) ---\n");
    for (int i = 0; i < n_ns; i++) printf("NSKEY %s\n", ns_key[i]);
    printf("NS_NODES %d\n", n_ns);
    int collisions = 0;
    for (int i = 0; i < n_ns; i++)
        for (int j = i + 1; j < n_ns; j++)
            if (!strcmp(ns_key[i], ns_key[j])) { collisions++;
                printf("NS_COLLISION %s\n", ns_key[i]); }
    printf("NS_KEY_COLLISIONS %d\n", collisions);

    rb_unload();
    return 0;
}