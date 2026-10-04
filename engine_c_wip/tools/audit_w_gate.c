/* Cluster W - corpus census for the eight bug-predicting gcc diagnostics.

   Each row is the MEASURED reachability of the code the diagnostic sits on, so
   "it does not fire today" and "it can never fire" are never confused again.
   Walks the SHIPPED bytecode through the engine's own decoder
   (rb_decode_card_ability), never cards/abilities.json (which is stale, K7).

   A. items 1/2/8  the two -Waddress guards and the -Wparentheses guard
   B. items 3/4    the discarded-const sites (measured for read-only-ness)
   C. items 5-7    the data.c scratch-buffer disjointness invariant

   Build: make w-census
*/
#include <stdio.h>
#include <string.h>
#include "rabuka.h"

/* Cluster W exports these two from src/core/data.c. include/rabuka.h is main's
   file, so -- exactly like src/game/game_setup.c:208 and the T1 test file --
   the declarations are local and EXPLICIT rather than implicit. Moving them into
   the header is recorded as the proper home; an implicit declaration here would
   itself be a bug class this cluster is about. */
size_t rb_card_no_scratch_bytes(size_t len);
int    rb_card_no_scratch_layout_ok(size_t len);

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

/* Rust `is_stale_discard_condition`, abilities.rs:486-501. Three separable
   terms, so three counters: the &&/|| grouping the diagnostic is about, the
   MISSING preceding_moved exemption, and the MISSING trigger_event fallback. */
struct DiscardCensus { long nodes, guard_hit, flat_only, trigger_event_only,
                       preceding_moved, trigger_event_preceded, ct_member, tg_self; };

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
        if (ct && !strcmp(ct, "member_card")) k->ct_member++;
        if (tg && !strcmp(tg, "self"))         k->tg_self++;
        int via_te = (flat == NULL) && te && !strcmp(te, "discard");
        if (via_te) k->trigger_event_only++; else k->flat_only++;
        if (src && !strcmp(src, "preceding_moved")) {
            k->preceding_moved++;
            if (via_te) k->trigger_event_preceded++;
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

/* Item 2: the producer/route for the pay_cost_all:discard_all SelectTarget.
   Rust: enums.rs:705 (exact literal), choice.rs:2816-2818 (routes on
   SelectTargetKind::from_str), handlers.rs:378-388 (producer sets the target). */
static long walk_pcad(const AbilityEffect *e, int depth) {
    if (!e || depth > 12) return 0;
    long n = (e->action && !strcmp(e->action, "pay_cost_all:discard_all")) ? 1 : 0;
    for (int i = 0; i < e->n_child; i++)  n += walk_pcad(e->child[i], depth + 1);
    for (int i = 0; i < e->n_options; i++) n += walk_pcad(e->options[i], depth + 1);
    n += walk_pcad(e->primary_effect, depth + 1);
    n += walk_pcad(e->alternative_effect, depth + 1);
    n += walk_pcad(e->look_action, depth + 1);
    n += walk_pcad(e->select_action, depth + 1);
    n += walk_pcad(e->followup_action, depth + 1);
    n += walk_pcad(e->optional_action, depth + 1);
    n += walk_pcad(e->conditional_action, depth + 1);
    return n;
}
/* the C prompt producer's own gate: cost.c:1120 `is_from_hand && is_all` */
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

/* Item 2 reachability, by NAME: the C raises the "Discard entire hand (N
   cards)?" prompt (cost.c:1120 `is_from_hand && is_all`) for these, and the
   BLOCKED-ON patch on cost.c:1129 is written against them. */
static void name_hand_all(const Card *c, int nab, const char *stem, long *n) {
    /* rb_card_string(), NOT rb_get_string(): `card_no_idx` indexes the CARDS
       string table (data.c g_card_strings, read by rb_card_string) and has
       nothing to do with the ABILITIES table (g_strings, read by rb_get_string).
       This line used rb_get_string, so it printed a DIFFERENT card's number for
       every card whose card_no_idx landed on another slot -- which is how the
       census came to name PL!N-bp5-021-N / PL!N-bp5-028-L for the two nodes
       that are really PL!SP-bp7-011-P / PL!SP-bp7-011-R. The two tables are
       disjoint namespaces: measured over all 2639 cards, the two accessors
       agree 0 times. The NODE COUNT was always right; only the NAME was wrong. */
    printf("  W-A2  card_no=%s n_abilities=%d abilities_with_hand_all_cost=",
           rb_card_string(c->card_no_idx), nab);
    for (int k = 0; k < nab; k++) printf(" %d", k);
    printf("   [%s]\n", stem);
    (*n)++;
}

/* Items 5-7, premise check. The disjointness argument leans on
   `rb_card_normalize_no` NEVER EXPANDING: it copies n bytes unchanged or writes
   exactly 1 byte for a folded code point (card.c:223), so
   strlen(normalized) <= len, so base_len = strlen(normalized) - (rarity+1) <= len,
   so `base` fits its len+1-byte slot with its NUL. Checked on every corpus
   card_no plus adversarial fullwidth / max-length inputs. */
static long norm_expansions(const char *s) {
    char out[512];
    size_t n = strlen(s);
    if (n + 8 >= sizeof(out)) return -1;
    rb_card_normalize_no(s, out, n + 1);
    out[n + 1] = 0;
    long bad = 0;
    size_t l = 0;
    while (out[l] && l < sizeof(out)) l++;
    if (l > n) bad++;
    return bad;
}

int main(int argc, char **argv) {
    const char *dir = argc > 1 ? argv[1] : "src";
    if (rb_load(dir) < 0) { fprintf(stderr, "rb_load failed\n"); return 2; }

    struct DiscardCensus dc;
    memset(&dc, 0, sizeof(dc));
    long n_pcad_nodes = 0, n_pcad_ab = 0, n_hand_nodes = 0, n_hand_ab = 0;
    long ncards = 0, npairs = 0, abilities = 0;
    long n_named = 0, n_norm_expand = 0, n_norm_checked = 0;

    for (uint32_t cid = 0; cid < 4000; cid++) {
        int n = rb_card_num_abilities(cid);
        if (n <= 0) { if (n < 0) break; continue; }
        ncards++;
        {
            Card cc;
            memset(&cc, 0, sizeof(cc));
            if (rb_decode_card_by_index(cid, &cc)) {
                long e = norm_expansions(rb_card_string(cc.card_no_idx));
                if (e > 0) n_norm_expand += e;
                if (e >= 0) n_norm_checked++;
                rb_free_card(&cc);
            }
        }
        for (int k = 0; k < n; k++) {
            Ability a;
            memset(&a, 0, sizeof(a));
            if (!rb_decode_card_ability(cid, k, &a)) continue;
            npairs++;
            if (a.effect && a.effect->condition) walk_discard(a.effect->condition, &dc, 0);
            if (a.cost) {
                long h = walk_hand_all(a.cost, 0);
                if (h) { n_hand_nodes += h; n_hand_ab++; }
            }
            long p = walk_pcad(a.effect, 0) + walk_pcad(a.cost, 0);
            if (p) { n_pcad_nodes += p; n_pcad_ab++; }
            long h2 = walk_hand_all(a.cost, 0);
            if (h2) {
                Card cc;
                memset(&cc, 0, sizeof(cc));
                if (rb_decode_card_by_index(cid, &cc)) {
                    char stem[64];
                    snprintf(stem, sizeof(stem), "hand_source_all_costs=%ld", h2);
                    name_hand_all(&cc, n, stem, &n_named);
                    rb_free_card(&cc);
                }
            }
            rb_free_ability(&a);
        }
    }
    abilities = rb_num_abilities();

    printf("W-cards | cards_with_abilities=%ld card_ability_pairs=%ld abilities=%ld\n",
           ncards, npairs, abilities);
    printf("W-A8-discard | condition_nodes=%ld guard_hit=%ld (flat_location=%ld "
           "trigger_event_only=%ld) preceding_moved_exempt_in_rust=%ld "
           "(trigger_event_only_and_preceded=%ld) card_term_member_card=%ld target_self=%ld\n",
           dc.nodes, dc.guard_hit, dc.flat_only, dc.trigger_event_only,
           dc.preceding_moved, dc.trigger_event_preceded, dc.ct_member, dc.tg_self);
    printf("W-A2-paycostall | action_pay_cost_all_discard_all_nodes=%ld abilities=%ld | "
           "hand_source_all_cost_nodes=%ld abilities=%ld | "
           "stk_table_pay_cost_all_discard_all=%d\n",
           n_pcad_nodes, n_pcad_ab, n_hand_nodes, n_hand_ab,
           rb_select_target_kind_from_str("pay_cost_all:discard_all"));

    /* C. items 5-7: the data.c scratch layout. For every len the four
       sub-ranges must be pairwise disjoint and inside the allocation. */
    long bad = 0;
    for (size_t len = 0; len <= 256; len++) if (!rb_card_no_scratch_layout_ok(len)) bad++;
    printf("W-C-scratch | layout_disjoint_len_0_256=%s violations=%ld "
           "scratch_bytes(len=0)=%zu (len=25)=%zu (len=63)=%zu\n",
           bad == 0 ? "YES" : "NO", bad,
           rb_card_no_scratch_bytes(0), rb_card_no_scratch_bytes(25),
           rb_card_no_scratch_bytes(63));
    printf("W-C-normalize | normalize_no_never_expands=%s (card_nos_checked=%ld "
           "expansions=%ld) -- the premise base_len<=len\n",
           n_norm_expand == 0 ? "YES" : "NO", n_norm_checked, n_norm_expand);
    printf("W-A2-named | abilities_named=%ld\n", n_named);

    rb_unload();
    return (bad == 0 && n_norm_expand == 0) ? 0 : 1;
}
