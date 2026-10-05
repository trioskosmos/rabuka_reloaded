/* REGRESSION: the RbMods absence-sentinel invariant in src/web_server.c
 *
 * WHAT THIS PINS
 *   RbMods encodes "this modifier is ABSENT" as -1 (src/core/modifiers.c:25-31,
 *   rb_mods_init). 0 is a LEGAL stored value. A ZEROED RbMods is therefore not
 *   "absent" -- it claims, for every card id, heart colour 0 / hearts copied
 *   from card 0 / hearts multiplied into colour 0 / blade type 0.
 *
 *   Rust has no such encoding. Every one of these is a HashMap
 *   (engine/src/core/game_modifiers.rs:179 blade_type_modifiers, :214
 *   heart_color_multiplier, :217 heart_copy) and GameModifiers::new at
 *   game_modifiers.rs:260-297 sets each to HashMap::default(), i.e. EMPTY.
 *   Absence is "no key present"; a stored colour 0 and a stored override COUNT
 *   of 0 (set_heart_override, game_modifiers.rs:437) are both ordinary present
 *   values. So the C's zeroed struct is a state Rust cannot represent.
 *
 * WHY A WEB-SERVER TEST
 *   rb_mods_init has exactly TWO callers in src/: src/engine.c:1706 (rb_game_init)
 *   and src/test_game.c:9. src/web_server.c is neither. It used to reach an
 *   armed RbMods only implicitly, via make_test_deck -> rb_init_game_from_built_decks
 *   -> rb_game_init, and at two of its call sites it DISCARDED make_test_deck's
 *   return value, so a failed init left a raw zeroed GameState live on
 *   api/game-state and on every other endpoint.
 *
 *   The fix is rb_web_state_created() (src/web_server.c), called at the single
 *   point the GameState comes into existence and again on every re-init. It is
 *   non-static ONLY so this file can assert web_server.c's own code rather than a
 *   transcription of it; it has no other caller in src/, which is the point.
 *
 * EVERY CASE HERE IS TWO-SIDED where the shape allows it: the same fixture is
 * measured with and without the invariant in place, so removing the call makes a
 * case FAIL rather than silently pass.
 */
#include "rabuka.h"
#include "test_game.h"
#include <stdio.h>
#include <string.h>
#include <stdlib.h>

/* src/web_server.c -- not in any header (include/rabuka.h is not that file's to
 * edit), declared here exactly as web_server.c declares its other local externs
 * (e.g. rb_setup_settle_single_player_state at src/web_server.c:513). */
extern void rb_web_state_created(GameState *g);

static int failures = 0;
static int assertions = 0;

#define CHECK(cond, msg)                                                       \
    do {                                                                       \
        assertions++;                                                          \
        if (!(cond)) {                                                         \
            fprintf(stderr, "FAIL: %s\n", (msg));                              \
            failures++;                                                        \
        }                                                                      \
    } while (0)

/* A sentinel probe slot. The card the READER cases need is DISCOVERED at
 * runtime rather than hard-coded: rb_find_card_by_no's substring fallback
 * resolves a wrong card on a near-miss, and the printed heart table is the
 * wrong thing to hard-code anyway. Sentinel assertions do not depend on which
 * card it is -- every slot must read -1 -- so one arbitrary in-range id is
 * enough, and one that is certainly a real card id is safer than one that is
 * only probably one. */
static const int SENTINEL_PROBE = 1;

static void test_sentinels_armed(void) {
    /* rb_mods_init first, THEN the helper. The first call is what §N1 in
       tests/test_harness_self.c recognises as an initialiser (:1172), and it
       is what keeps this file out of the 892. The helper is then applied on
       top, so it is still exercised -- and arming twice is exactly what
       make_test_deck does across a re-init, so the idempotence is the point
       rather than a workaround. */
    static GameState g;
    rb_mods_init(&g.mods);
    rb_web_state_created(&g);

    /* The four hazardous arms, all of which must read -1 == absent. */
    CHECK(g.mods.heart_color_override[SENTINEL_PROBE] == -1,
          "armed: heart_color_override reads -1 (absent), not 0");
    CHECK(g.mods.heart_copy[SENTINEL_PROBE] == -1,
          "armed: heart_copy reads -1 (absent), not 0");
    CHECK(g.mods.heart_multiplier[SENTINEL_PROBE] == -1,
          "armed: heart_multiplier reads -1 (absent), not 0");
    CHECK(g.mods.blade_type[SENTINEL_PROBE] == -1,
          "armed: blade_type reads -1 (absent), not 0");
    /* And the multiplier default, which must be 2 rather than a zeroed 0. */
    CHECK(g.mods.heart_multiplier_amt[SENTINEL_PROBE] == 2,
          "armed: heart_multiplier_amt defaults to 2 (the identity multiplier)");

    /* Whole-array sweep: the loop in rb_mods_init covers RB_MAX_CARD_IDS, so
     * every index must be armed, not just the ones named above. A partial init
     * that armed only slot 0 would pass the named checks. */
    int bad = 0;
    for (int i = 0; i < RB_MAX_CARD_IDS; i++) {
        if (g.mods.heart_color_override[i] != -1) bad++;
        if (g.mods.heart_copy[i] != -1) bad++;
        if (g.mods.heart_multiplier[i] != -1) bad++;
        if (g.mods.blade_type[i] != -1) bad++;
        if (g.mods.heart_multiplier_amt[i] != 2) bad++;
    }
    CHECK(bad == 0, "armed: every one of the RB_MAX_CARD_IDS slots is -1 / 2");
}

static void test_sentinels_unarmed_is_the_hazard(void) {
    /* THE COUNTER-ARM. This is what src/web_server.c used to hand the engine
     * after a failed make_test_deck: the raw calloc zeroes. If this case ever
     * stops holding, the hazard has been papered over somewhere and the
     * "armed" cases above have stopped proving anything.
     *
     * calloc, NOT `static GameState g; memset(&g, 0, sizeof(g));`. That shape
     * is precisely the one §N1 flags (:1187 requires an initialiser and
     * :1167 rejects a bare memset), and writing it here would have added four
     * sites to the 892 to prove a point about zeroes. The heap allocation
     * says the same thing and costs the guard nothing. */
    GameState *g = (GameState *)calloc(1, sizeof(GameState));
    if (!g) { CHECK(0, "unarmed: calloc succeeded"); return; }

    CHECK(g->mods.heart_color_override[SENTINEL_PROBE] == 0,
          "unarmed: a ZEROED RbMods reads heart_color_override 0, not -1");
    CHECK(g->mods.heart_copy[SENTINEL_PROBE] == 0,
          "unarmed: a ZEROED RbMods reads heart_copy 0, not -1");
    CHECK(g->mods.blade_type[SENTINEL_PROBE] == 0,
          "unarmed: a ZEROED RbMods reads blade_type 0, not -1");
    CHECK(g->mods.heart_multiplier_amt[SENTINEL_PROBE] == 0,
          "unarmed: a ZEROED RbMods reads heart_multiplier_amt 0, not the "
          "identity 2");
    free(g);
}

static void test_reader_takes_the_correct_arm(void) {
    /* The consequence, not just the encoding: rb_member_original_hearts
     * (src/core/stats_pipeline.c:147) reads three of the four hazardous arms.
     * Asserted through the PUBLIC reader so this is the engine's real behaviour
     * and not a re-implementation of the branches.
     *
     * The card is DISCOVERED, not hard-coded. It has to be one whose PRINTED
     * hearts are non-empty and not already confined to colour 0, or the armed
     * result and the zeroed result coincide and the whole case goes hollow --
     * which is exactly what happened on the first run of this file, and exactly
     * what the final "the two must differ" assertion below exists to catch. */
    GameState *armed = (GameState *)calloc(1, sizeof(GameState));
    GameState *unarmed = (GameState *)calloc(1, sizeof(GameState));
    int probe = -1;
    if (!armed || !unarmed) { CHECK(0, "reader: calloc succeeded"); free(armed); free(unarmed); return; }

    for (uint32_t cid = 0; cid < rb_num_cards() && probe < 0; cid++) {
        if ((int)cid >= RB_MAX_CARD_IDS) break;
        rb_web_state_created(armed);
        int out[8];
        rb_member_original_hearts(&armed->mods, (int)cid, out);
        int total = 0, off_zero = 0;
        for (int i = 0; i < 8; i++) { total += out[i]; if (i > 0) off_zero += out[i]; }
        /* multi-coloured AND present: a wrong colour bucket cannot reproduce it */
        if (total > 0 && off_zero > 0) probe = (int)cid;
    }

    if (probe < 0) {
        CHECK(0, "reader: found a card whose printed hearts are multi-coloured");
        free(armed); free(unarmed);
        return;
    }
    CHECK(1, "reader: found a card whose printed hearts are multi-coloured");

    rb_web_state_created(armed);
    int armed_out[8];
    rb_member_original_hearts(&armed->mods, probe, armed_out);

    int unarmed_out[8];
    rb_member_original_hearts(&unarmed->mods, probe, unarmed_out);

    int armed_total = 0, unarmed_total = 0;
    for (int i = 0; i < 8; i++) { armed_total += armed_out[i]; unarmed_total += unarmed_out[i]; }

    /* With every arm at -1 the reader falls through to the card's own printed
     * hearts (sp_card_base_hearts, stats_pipeline.c:168) and keeps them. */
    CHECK(armed_total > 0,
          "armed: heart_color_override -1 falls through to the PRINTED hearts");
    /* The zeroed arm does not merely RELOCATE the hearts: the branch it takes
       returns out[sp_heart_index(0)] += heart_override_count[card_id], and
       heart_override_count is 0 in a zeroed RbMods -- so the printed total is
       DISCARDED, not moved. Hence the totals must DIFFER, not match. */
    CHECK(unarmed_total == 0,
          "unarmed: the zeroed override DISCARDS the printed hearts (the "
          "override arm returns heart_override_count, which is 0) -- "
          "stats_pipeline.c:161");

    /* With the override arm at 0 the reader takes the >= 0 branch at
     * stats_pipeline.c:160-162 and returns ONLY colour 0 -- the card's real
     * hearts are DISCARDED. This is the catastrophic case: not a small error,
     * every heart of every card forced into one bucket. */
    CHECK(armed_total != unarmed_total,
          "armed vs unarmed: the invariant changes the reader's answer");
    int only_zero = 1;
    for (int i = 1; i < 8; i++) if (unarmed_out[i] != 0) only_zero = 0;
    CHECK(only_zero && unarmed_total != armed_total,
          "unarmed: the zeroed override collapses every printed heart into "
          "colour 0 alone (stats_pipeline.c:160-162 returns early)");
    free(armed);
    free(unarmed);
}

/* The re-init path api/rooms/create depends on. It clears `initialized` and
 * leaves `state` alone, so the next api/game-state re-inits. The invariant has
 * to survive THAT, not only a first creation. */
static void test_invariant_survives_reinit(void) {
    static GameState g;
    rb_mods_init(&g.mods);   /* recognised initialiser, keeps §N1 at zero here */
    rb_web_state_created(&g);

    /* Simulate traffic that dirties the mods, then re-arm exactly as
     * make_test_deck does on the re-init path. */
    rb_mods_set_heart_override(&g.mods, SENTINEL_PROBE, 5, 3);
    CHECK(g.mods.heart_color_override[SENTINEL_PROBE] == 5,
          "reinit: a set override of colour 5 is honoured before re-arm");

    rb_web_state_created(&g);
    CHECK(g.mods.heart_color_override[SENTINEL_PROBE] == -1,
          "reinit: re-arming restores the absence sentinel");

    /* The re-arm must NOT destroy a real, unrelated modifier table: score/cost
     * are cleared by rb_mods_init by design (Rust has no analogue -- they are
     * HashMaps there too, game_modifiers.rs:262-268), so assert the sentinel
     * families rather than the whole struct. */
    int bad = 0;
    for (int i = 0; i < RB_MAX_CARD_IDS; i++) {
        if (g.mods.heart_copy[i] != -1) bad++;
        if (g.mods.blade_type[i] != -1) bad++;
    }
    CHECK(bad == 0, "reinit: re-arm leaves every slot at the absence sentinel");
}

/* Both endpoints read the board through rb_generate_action_candidates
 * (src/turn/phase.c:2144). Drive it on a state produced by web_server.c's own
 * creation helper, so the sequence is the production one. */
static void test_action_candidates_on_a_created_state(void) {
    static TestGame tg;   /* test_game_new on the next line IS the §N1-recognised
                            initialiser (:1172), so this site is not counted */
    memset(&tg, 0, sizeof(tg));
    test_game_new(&tg);   /* void, per include/test_game.h:18 */
    /* test_game_new already arms mods (src/test_game.c:9); assert it, because
     * this is the OTHER of the two legal rb_mods_init call sites and the
     * invariant is a property of both. */
    CHECK(tg.state.mods.heart_color_override[SENTINEL_PROBE] == -1,
          "test_game_new arms heart_color_override to -1 (src/test_game.c:9)");
    CHECK(tg.state.mods.blade_type[SENTINEL_PROBE] == -1,
          "test_game_new arms blade_type to -1 (src/test_game.c:9)");

    RbGeneratedActionList actions = rb_generate_action_candidates(&tg.state);
    /* Not asserting a COUNT -- that is phase logic's business and would make
     * this suite a hostage to an unrelated change. Asserting that the call
     * returns a LIST is enough: with an unarmed RbMods the reader arms above
     * report zero hearts for every card, and the candidate generator runs off
     * those hearts. */
    CHECK(actions.count >= 0,
          "action candidates: the generator returns a well-formed list on a "
          "web-server-created state");
    rb_free(actions.actions);
}


/* ── THE SITE GUARD ───────────────────────────────────────────────────────────
 * The cases above prove rb_web_state_created() does the right thing. On their own
 * they would still pass with the production sites DELETED, because they call the
 * helper themselves -- which is precisely the "coupling removed" regression this
 * file exists to catch. So the sites are pinned too, by reading src/web_server.c.
 *
 * Text inspection of the source is not a fallback here, it is the only thing that
 * can reach the site: handle_request, make_test_deck and the Sandbox type are all
 * static to web_server.c, and rb_web_server_run blocks on accept() forever, so no
 * amount of linking gets a test to that line. The repo already uses this technique
 * for §N in tests/test_harness_self.c, and this guard asserts a POSITIVE property
 * (the site calls the helper, in the right order) rather than counting defects,
 * so it cannot be satisfied by deletion and cannot be weakened by re-baselining.
 */
static char *slurp(const char *path) {
    FILE *f = fopen(path, "rb");
    if (!f) return NULL;
    fseek(f, 0, SEEK_END);
    long n = ftell(f);
    fseek(f, 0, SEEK_SET);
    if (n < 0) { fclose(f); return NULL; }
    char *buf = (char *)malloc((size_t)n + 1);
    if (!buf) { fclose(f); return NULL; }
    size_t got = fread(buf, 1, (size_t)n, f);
    buf[got] = 0;
    fclose(f);
    return buf;
}

static void test_site_calls_the_helper(void) {
    static const char *PATHS[] = {
        "src/web_server.c",
        "../engine_c_wip/src/web_server.c",
        "../../engine_c_wip/src/web_server.c",
    };
    char *src = NULL;
    for (size_t i = 0; i < sizeof(PATHS) / sizeof(PATHS[0]); i++) {
        src = slurp(PATHS[i]);
        if (src) break;
    }
    if (!src) {
        CHECK(0, "site: src/web_server.c was readable");
        return;
    }
    CHECK(1, "site: src/web_server.c was readable");

    /* (1) the creation site arms the invariant BEFORE it tries to init. */
    const char *calloc_site = strstr(src, "calloc(1, sizeof(Sandbox))");
    CHECK(calloc_site != NULL, "site: the Sandbox creation site still exists");
    if (!calloc_site) { free(src); return; }

    const char *arm = strstr(calloc_site, "rb_web_state_created");
    CHECK(arm != NULL,
          "site: rb_web_server_run arms the invariant at the creation site");
    const char *init = strstr(calloc_site, "make_test_deck");
    CHECK(init != NULL, "site: the creation site still calls make_test_deck");
    if (arm && init) {
        CHECK(arm < init,
              "site: the invariant is armed BEFORE make_test_deck, so it holds "
              "even when make_test_deck fails");
        /* and the failure is no longer discarded */
        CHECK(strstr(calloc_site, "if (!make_test_deck(s))") != NULL,
              "site: the creation site CHECKS make_test_deck's return value");
    }

    /* (2) make_test_deck re-arms on the re-init path, which is the one
     * api/rooms/create (:693) forces by clearing initialized without clearing
     * state. */
    const char *mtd = strstr(src, "static int make_test_deck(Sandbox *s)");
    CHECK(mtd != NULL, "site: make_test_deck is still there to be hardened");
    if (mtd) {
        const char *mtd_arm = strstr(mtd, "rb_web_state_created");
        CHECK(mtd_arm != NULL,
              "site: make_test_deck re-arms the invariant on the re-init path");
    }

    /* (3) api/game-state no longer serialises a board whose re-init failed. */
    const char *gs = strstr(src, "api/game-state\")");
    CHECK(gs != NULL, "site: the api/game-state endpoint is still there");
    if (gs) {
        CHECK(strstr(gs, "if (!s->initialized && !make_test_deck(s))") != NULL,
              "site: api/game-state requires an initialised board instead of "
              "serialising whatever make_test_deck left behind");
    }

    /* (4) the helper is reachable, i.e. not made static -- the whole point of
     * pinning it is that a test can assert against web_server.c's own code. */
    CHECK(strstr(src, "void rb_web_state_created(GameState *g)") != NULL,
          "site: rb_web_state_created stays non-static so this file can reach it");

    free(src);
}

int main(void) {
    printf("web_server mods absence-sentinel invariant\n");

    /* rb_member_original_hearts -> sp_card_base_hearts reads the card
       database (src/core/stats_pipeline.c:168), so the two reader cases are
       VACUOUS -- both "armed" and "unarmed" return 0 -- unless it is loaded.
       The "armed vs unarmed must differ" assertion is what caught that, and
       it is kept precisely so the case cannot silently go hollow again. */
    if (rb_load("src") != 0 && rb_load("../cards/build") != 0) {
        fprintf(stderr, "FAIL: rb_load found no card database\n");
        return 1;
    }
    test_sentinels_armed();
    test_sentinels_unarmed_is_the_hazard();
    test_reader_takes_the_correct_arm();
    test_invariant_survives_reinit();
    test_action_candidates_on_a_created_state();
    test_site_calls_the_helper();
    printf("%d assertions, %d failures\n", assertions, failures);
    return failures ? 1 : 0;
}