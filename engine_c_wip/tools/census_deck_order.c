/* census_deck_order.c — deck-order reachability census.
 *
 * Walks the shipped bytecode with the ENGINE'S OWN decoder (rb_decode_ability
 * over all 985 unique abilities, then rb_card_get_ability_idx per card-ability
 * pair so every hit can be named) and applies the `maybe_prompt_deck_order`
 * predicate from engine/src/ability/effects/move/mod.rs, cited by CONTENT
 * because that file is being edited:
 *
 *   is_deck_dest       = Zone::from_str(destination) in
 *                        {Deck, DeckTop, DeckBottom}
 *   is_eligible_source = Zone::from_str(source) in
 *                        {Discard, SelectedCards, Hand}
 *                        || (source == LookedAt && effect.all_any())
 *   raise iff is_eligible_source && is_deck_dest
 *              && placement_order == Some(PlacementOrder::AnyOrder)
 *              && taken.len() > 1
 *
 * `taken.len()` is dynamic — it is whatever rb_move_resolve_cards_from_source
 * produced — so the probe reports the STATIC half and prints the declared
 * `count` so a reader can tell which of the hits can actually raise the prompt.
 * A negative `count` means 「すべて」 in this codebase's convention.
 *
 * It also reports the SEPARATE producer: `look_and_select` nodes whose
 * select_action carries `placement_order: any_order`, i.e. the shape Rust's
 * second call site (the resume-side Finalize arm) handles. Those need
 * src/ability/effects/look.c's rb_look_resume_indices and are NOT covered by
 * the move_cards producer in move_missing.c.
 *
 * Build (no data regeneration, no .o left behind):
 *   gcc -std=c11 -O2 -Iinclude -Isrc -Isrc/core/generated \
 *       -o rb_census_deck_order tools/census_deck_order.c <OBJ_LIB from the
 *       Makefile> && ./rb_census_deck_order src
 */
#include <stdio.h>
#include <string.h>
#include <stdlib.h>
#include "rabuka.h"

static const char *xget(const AbilityEffect *e, const char *k)
{
    if (!e) return NULL;
    for (int i = 0; i < e->n_extra; i++)
        if (e->extra_k[i] && !strcmp(e->extra_k[i], k)) return e->extra_v[i];
    return NULL;
}
static int xbool(const AbilityEffect *e, const char *k)
{
    const char *v = xget(e, k);
    return v && (!strcmp(v, "true") || !strcmp(v, "1"));
}

static int is_deck_dest(const char *d)
{
    return d && (!strcmp(d, "deck") || !strcmp(d, "deck_top") || !strcmp(d, "deck_bottom"));
}
static int is_eligible_source(const char *s, const AbilityEffect *e)
{
    if (!s) return 0;
    if (!strcmp(s, "discard") || !strcmp(s, "selected_cards") || !strcmp(s, "hand")) return 1;
    if (!strcmp(s, "looked_at") && xbool(e, "all")) return 1;
    return 0;
}

static unsigned g_ab;
static char cur_no[64];
static char cur_trig[200];
static int  cur_ab;

static void walk(const AbilityEffect *e, int depth)
{
    if (!e || depth > 14) return;
    if (e->action && !strcmp(e->action, "move_cards")) {
        const char *po = xget(e, "placement_order");
        int deck = is_deck_dest(e->destination);
        int elig = is_eligible_source(e->source, e);
        if (po && !strcmp(po, "any_order")) {
            printf("PROBE-MOVE ab=%u card=%s deck_dest=%d eligible_source=%d src=%-14s dst=%-12s count=%d all=%d trig=%.60s\n",
                   g_ab, cur_no, deck, elig,
                   e->source ? e->source : "(null)", e->destination ? e->destination : "(null)",
                   e->count, xbool(e, "all"), cur_trig);
        }
    }
    if (e->action && !strcmp(e->action, "look_and_select")) {
        const AbilityEffect *sel = e->select_action;
        const char *po = sel ? xget(sel, "placement_order") : NULL;
        if (po && !strcmp(po, "any_order"))
            printf("PROBE-LAS  ab=%u card=%s sel_dst=%-12s any_number=%d count=%d trig=%.60s\n",
                   g_ab, cur_no,
                   (sel && sel->destination) ? sel->destination : "(null)",
                   sel ? xbool(sel, "any_number") : 0,
                   sel ? sel->count : 0, cur_trig);
    }
    if (e->action && !strcmp(e->action, "custom")) {
        const char *po = xget(e, "placement_order");
        if (po && !strcmp(po, "any_order"))
            printf("PROBE-CUST ab=%u card=%s src=%-14s dst=%-12s count=%d trig=%.60s\n",
                   g_ab, cur_no,
                   e->source ? e->source : "(null)",
                   e->destination ? e->destination : "(null)", e->count, cur_trig);
    }
    for (int i = 0; i < e->n_child; i++)   walk(e->child[i], depth + 1);
    for (int i = 0; i < e->n_options; i++) walk(e->options[i], depth + 1);
    walk(e->primary_effect, depth + 1);
    walk(e->alternative_effect, depth + 1);
    walk(e->look_action, depth + 1);
    walk(e->select_action, depth + 1);
    walk(e->followup_action, depth + 1);
    walk(e->optional_action, depth + 1);
    walk(e->conditional_action, depth + 1);
    walk(e->gained_effect, depth + 1);
    walk(e->resource_on_select, depth + 1);
    walk(e->opponent_action, depth + 1);
}

int main(int argc, char **argv)
{
    const char *dir = argc > 1 ? argv[1] : "src";
    if (rb_load(dir) != 0) { fprintf(stderr, "rb_load failed\n"); return 2; }
    long n_ab = 0;
    for (unsigned i = 0; i < rb_num_abilities(); i++) {
        Ability ab; memset(&ab, 0, sizeof ab);
        if (!rb_decode_ability(i, &ab)) continue;
        n_ab++;
        g_ab = i;
        cur_ab = 0;
        snprintf(cur_trig, sizeof cur_trig, "%s", ab.triggers ? ab.triggers : "");
        snprintf(cur_no, sizeof cur_no, "-");
        walk(ab.effect, 0);
        rb_free_ability(&ab);
    }
    /* Pass 2: same walk, but per CARD, so every hit carries the card number and
     * the GLOBAL ability index it belongs to (rb_card_get_ability_idx). */
    for (int cid = 0; cid < rb_num_cards(); cid++) {
        int n = rb_card_num_abilities((uint32_t)cid);
        for (int ai = 0; ai < n; ai++) {
            uint32_t gi = 0;
            if (rb_card_get_ability_idx((uint32_t)cid, ai, &gi)) g_ab = gi;
            Ability ab; memset(&ab, 0, sizeof ab);
            if (!rb_decode_card_ability((uint32_t)cid, ai, &ab)) continue;
            snprintf(cur_trig, sizeof cur_trig, "%s", ab.triggers ? ab.triggers : "");
            { Card c;
              if (rb_decode_card_by_index((uint32_t)cid, &c)) {
                  const char *no = rb_card_string(c.card_no_idx);
                  snprintf(cur_no, sizeof cur_no, "%s", no ? no : "-");
                  rb_free_card(&c);
              } else snprintf(cur_no, sizeof cur_no, "-");
            }
            walk(ab.effect, 0);
            rb_free_ability(&ab);
        }
    }
    printf("ABILITIES %ld\n", n_ab);
    rb_unload();
    return 0;
}