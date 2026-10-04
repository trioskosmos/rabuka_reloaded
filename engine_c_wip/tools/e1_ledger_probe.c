/* Runtime proof that E1's three success_zone_* ledgers are now WRITTEN and READ.
 * Before the port they were 0 writers / 0 readers (CENSUS_fields.md rows 344-346).
 * This places the reachable corpus cards in the zones Rust's E1 scans and prints
 * the ledgers after rb_recalc_constants.
 *
 * Fixtures, by measured reachability (tools/census_e1_exact.c):
 *   cid 291  PL!-bp6-022-L  常時 modify_required_hearts        -> success zone
 *   a 常時 gain_resource blade / heart host                     -> success zone
 *   a 常時 modify_score live_total host                         -> success zone
 * Only the numbers matter here: a non-zero ledger after a recalc is the claim. */
#include <stdio.h>
#include <string.h>
#include "rabuka.h"
#include "test_game.h"

static TestGame tg;

/* First constant (常時) gain_resource / modify_score host found in the corpus,
 * located by predicate rather than hard-coded, so a corpus regeneration cannot
 * quietly turn this probe vacuous. */
static int find_const_host(const char *want_action, int want_live_total)
{
    int fallback = -1;
    for (int pass = 0; pass < 2 && fallback < 0; pass++) {
        /* pass 0 demands a condition-FREE effect, so the probe exercises the
         * apply arm rather than the condition gate; pass 1 accepts any. */
        for (int cid = 0; cid < rb_num_cards(); cid++) {
            int n = rb_card_num_abilities((uint32_t)cid);
            for (int ai = 0; ai < n; ai++) {
                Ability ab; memset(&ab, 0, sizeof ab);
                if (!rb_decode_card_ability((uint32_t)cid, ai, &ab)) continue;
                int ok = ab.triggers && strstr(ab.triggers, "\xe5\xb8\xb8\xe6\x99\x82")
                         && ab.effect && ab.effect->action;
                if (ok && !strcmp(ab.effect->action, want_action) &&
                    (!want_live_total || (ab.effect->target &&
                                          !strcmp(ab.effect->target, "live_total")))) {
                    int no_cond = !ab.effect->has_condition;
                    rb_free_ability(&ab);
                    if (pass == 0 && !no_cond) continue;
                    return cid;
                }
                rb_free_ability(&ab);
            }
        }
        fallback = 0;
    }
    return -1;
}

static long ledger_nonzero(void)
{
    long n = 0;
    for (int cid = 0; cid < RB_MAX_CARD_IDS; cid++) {
        if (tg.state.mods.success_zone_blade_bonuses[cid]) n++;
        if (tg.state.mods.success_zone_score_bonuses[cid]) n++;
        for (int c = 0; c < 8; c++)
            if (tg.state.mods.success_zone_heart_bonuses[cid][c]) n++;
    }
    return n;
}

int main(int argc, char **argv)
{
    if (rb_load(argc > 1 ? argv[1] : "src") != 0) { fprintf(stderr, "load failed\n"); return 2; }

    int nrh = find_const_host("modify_required_hearts", 0);
    int ngr = find_const_host("gain_resource", 0);
    int nms = find_const_host("modify_score", 1);
    printf("HOSTS need_heart=%d gain_resource=%d modify_score_live_total=%d\n", nrh, ngr, nms);

    /* E1's GainResource candidate set is the target player's STAGE
     * (modifiers.rs:2374-2407), so a stage member is required before any blade
     * or heart ledger can move. Without one, 0 candidates is faithful, not a
     * dead ledger. */
    int member = -1;
    for (int cid = 0; cid < rb_num_cards() && member < 0; cid++)
        if (rb_card_is_member(cid)) member = cid;
    printf("STAGE_MEMBER %d\n", member);

    /* Dump what E2 will actually compute for the chosen blade/heart host, so a
     * zero ledger is attributable rather than mysterious. */
    if (ngr >= 0) {
        int n = rb_card_num_abilities((uint32_t)ngr);
        for (int ai = 0; ai < n; ai++) {
            Ability ab; memset(&ab, 0, sizeof ab);
            if (!rb_decode_card_ability((uint32_t)ngr, ai, &ab)) continue;
            if (ab.effect && ab.effect->action && !strcmp(ab.effect->action, "gain_resource")) {
                const char *res = NULL, *icon = NULL;
                for (int k = 0; k < ab.effect->n_extra; k++) {
                    if (ab.effect->extra_k[k] && !strcmp(ab.effect->extra_k[k], "resource")) res = ab.effect->extra_v[k];
                    if (ab.effect->extra_k[k] && !strcmp(ab.effect->extra_k[k], "resource_icon_count")) icon = ab.effect->extra_v[k];
                }
                printf("HOST_DETAIL cid=%d resource=%s count=%d resource_icon_count=%s "
                       "n_child=%d has_condition=%d\n",
                       ngr, res ? res : "(none)", ab.effect->count, icon ? icon : "(none)",
                       ab.effect->n_child, ab.effect->has_condition);
                break;
            }
            rb_free_ability(&ab);
        }
    }

    test_game_new(&tg);
    printf("BEFORE nonzero_ledger_entries=%ld\n", ledger_nonzero());

    /* Rust E1 scans live + success for BOTH seats (modifiers.rs:2232-2237). */
    if (member >= 0) test_add_to_stage(&tg, 1, member);   /* centre */
    if (nrh >= 0) test_add_to_success(&tg, nrh);
    if (ngr >= 0) test_add_to_success(&tg, ngr);
    if (nms >= 0) test_add_to_success(&tg, nms);
    rb_recalc_constants(&tg.state);

    printf("ZONES stage=[%d,%d,%d] success.n=%d success=[",
           tg.state.p[0].stage[0], tg.state.p[0].stage[1], tg.state.p[0].stage[2],
           tg.state.p[0].success.n);
    for (int i = 0; i < tg.state.p[0].success.n; i++)
        printf("%s%d", i ? "," : "", tg.state.p[0].success.cards[i]);
    printf("]\n");

    /* Brute force: try EVERY constant (常時) gain_resource host on its own and
     * report which ones move a ledger, so a zero is attributable to that host's
     * own filters rather than to the port. */
    int tried = 0, moved = 0;
    for (int cid = 0; cid < rb_num_cards(); cid++) {
        int n = rb_card_num_abilities((uint32_t)cid);
        int is_host = 0; const char *pos = NULL; const char *grp = NULL;
        const char *res = NULL; int per_unit = 0;
        for (int ai = 0; ai < n; ai++) {
            Ability ab; memset(&ab, 0, sizeof ab);
            if (!rb_decode_card_ability((uint32_t)cid, ai, &ab)) continue;
            if (ab.triggers && strstr(ab.triggers, "\xe5\xb8\xb8\xe6\x99\x82") &&
                ab.effect && ab.effect->action && !strcmp(ab.effect->action, "gain_resource")) {
                is_host = 1;
                for (int k = 0; k < ab.effect->n_extra; k++) {
                    const char *k2 = ab.effect->extra_k[k]; const char *v = ab.effect->extra_v[k];
                    if (!k2) continue;
                    if (!strcmp(k2, "position")) pos = v;
                    else if (!strcmp(k2, "group_names")) grp = v;
                    else if (!strcmp(k2, "resource")) res = v;
                    else if (!strcmp(k2, "per_unit")) per_unit = 1;
                }
            }
            rb_free_ability(&ab);
        }
        if (!is_host) continue;
        tried++;
        tg.state.p[0].success.n = 0;
        test_add_to_success(&tg, cid);
        rb_recalc_constants(&tg.state);
        if (ledger_nonzero() > 0) {
            moved++;
            if (moved <= 6)
                printf("  MOVES_LEDGER cid=%d resource=%s position=%s groups=%s per_unit=%d\n",
                       cid, res ? res : "-", pos ? pos : "-", grp ? grp : "-", per_unit);
        }
    }
    printf("GAIN_RESOURCE_HOSTS tried=%d moved_ledger=%d\n", tried, moved);

    printf("AFTER  nonzero_ledger_entries=%ld\n", ledger_nonzero());
    printf("P1_LIVE_TOTAL_BONUS %d\n", (int)tg.state.mods.p1_constant_total_score_bonus);
    for (int cid = 0; cid < RB_MAX_CARD_IDS; cid++) {
        if (tg.state.mods.success_zone_blade_bonuses[cid])
            printf("  blade_bonus[%d]=%d\n", cid, tg.state.mods.success_zone_blade_bonuses[cid]);
        if (tg.state.mods.success_zone_score_bonuses[cid])
            printf("  score_bonus[%d]=%d\n", cid, tg.state.mods.success_zone_score_bonuses[cid]);
        for (int c = 0; c < 8; c++)
            if (tg.state.mods.success_zone_heart_bonuses[cid][c])
                printf("  heart_bonus[%d][%d]=%d\n", cid, c, tg.state.mods.success_zone_heart_bonuses[cid][c]);
    }
    printf("NEED_HEART_BEFORE_AFTER note: E1 must NOT clear need_heart "
           "(modifiers.rs:2185); that is E3's job (modifiers.rs:2148)\n");
    rb_unload();
    return 0;
}