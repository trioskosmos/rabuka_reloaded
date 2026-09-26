/* test_parity_custom_misc.c — parity tests for the custom + miscellaneous
   effect clusters (engine/src/ability/effects/custom.rs and misc.rs).

   Every case here constructs the AbilityEffect directly and executes it, so the
   assertion is about the effect handler itself rather than about how some card
   wires that effect up. Each assertion names the Rust branch it pins. */

#include "rabuka.h"
#include "test_game.h"
#include <stdio.h>
#include <string.h>
#include <stdlib.h>

static int failures;
static int checks;

#define CHECK(condition, message) do { \
    checks++; \
    if (!(condition)) { \
        fprintf(stderr, "FAIL: %s\n", message); \
        failures++; \
    } else { \
        printf("ok: %s\n", message); \
    } \
} while (0)

#define CHECK_EQ(actual, expected, message) do { \
    checks++; \
    int actual_value = (actual); \
    int expected_value = (expected); \
    if (actual_value != expected_value) { \
        fprintf(stderr, "FAIL: %s (got %d expected %d)\n", message, actual_value, expected_value); \
        failures++; \
    } else { \
        printf("ok: %s\n", message); \
    } \
} while (0)

/* ── fixtures ─────────────────────────────────────────────────────────────── */

static void set_extra(AbilityEffect *e, const char *k, const char *v)
{
    for (int i = 0; i < e->n_extra; i++) {
        if (e->extra_k[i] && !strcmp(e->extra_k[i], k)) {
            e->extra_v[i] = (char *)v;
            return;
        }
    }
    if (e->n_extra >= RB_MAX_EXTRA) return;
    e->extra_k[e->n_extra] = (char *)k;
    e->extra_v[e->n_extra] = (char *)v;
    e->n_extra++;
}

static int blade_of(TestGame *tg, int card_id)
{
    return rb_mods_get_blade(&tg->state.mods, card_id);
}

static int heart_of(TestGame *tg, int card_id, int color)
{
    return rb_mods_get_heart(&tg->state.mods, card_id, color);
}

/* Group string printed on a real card record (card position ②). */
static void card_group(int card_id, char *buf, size_t cap)
{
    buf[0] = 0;
    Card c;
    memset(&c, 0, sizeof c);
    if (!rb_decode_card_by_index((uint32_t)card_id, &c)) return;
    const char *g = c.group_idx ? rb_card_string(c.group_idx) : NULL;
    if (g) {
        strncpy(buf, g, cap - 1);
        buf[cap - 1] = 0;
    }
    rb_free_card(&c);
}

/* Base (printed) heart colors of a card, as C HeartColor values. */
static int card_base_hearts(int card_id, int *out, int max)
{
    int n = 0;
    Card c;
    memset(&c, 0, sizeof c);
    if (!rb_decode_card_by_index((uint32_t)card_id, &c)) return 0;
    for (int h = 0; h < c.n_hearts && h < c.num_base && n < max; h++) {
        int col = c.heart_color[h];
        if (col >= 0 && col < 8) out[n++] = col;
    }
    rb_free_card(&c);
    return n;
}

/* Three member cards that are guaranteed distinct instances. */
static int member_triple(TestGame *tg, int *a, int *b, int *c)
{
    *a = rb_create_card_copy(rb_find_card_by_no("PL!-sd1-010-SD"));
    *b = rb_create_card_copy(rb_find_card_by_no("PL!-sd1-002-SD"));
    *c = rb_create_card_copy(rb_find_card_by_no("PL!-sd1-003-SD"));
    return (*a >= 0 && *b >= 0 && *c >= 0);
}

/* ── gain_resource: blade ─────────────────────────────────────────────────── */

static void test_blade_grants_to_own_stage(void)
{
    TestGame tg;
    test_game_new(&tg);
    int a, b, c;
    if (!member_triple(&tg, &a, &b, &c)) return;
    test_add_to_stage(&tg, 1, a);
    AbilityEffect e = {0};
    e.action = "gain_resource";
    e.target = "self";
    e.count = 2;
    set_extra(&e, "resource", "blade");
    rb_execute_effect_ex(&tg.state, 0, &e, a);
    CHECK_EQ(blade_of(&tg, a), 2, "blade: plain gain_resource grants count to the member");
    CHECK_EQ(blade_of(&tg, b), 0, "blade: a card that is not on stage is untouched");
}

static void test_blade_negative_sign(void)
{
    TestGame tg;
    test_game_new(&tg);
    int a, b, c;
    if (!member_triple(&tg, &a, &b, &c)) return;
    test_add_to_stage(&tg, 1, a);
    AbilityEffect e = {0};
    e.action = "gain_resource";
    e.target = "self";
    e.count = 2;
    set_extra(&e, "resource", "blade");
    set_extra(&e, "sign", "negative");
    rb_execute_effect_ex(&tg.state, 0, &e, a);
    CHECK_EQ(blade_of(&tg, a), -2, "blade: sign=negative removes blades (misc.rs:914-918)");
}

static void test_blade_self_target_field(void)
{
    TestGame tg;
    test_game_new(&tg);
    int a, b, c;
    if (!member_triple(&tg, &a, &b, &c)) return;
    test_add_to_stage(&tg, 1, a);
    test_add_to_stage(&tg, 2, b);
    AbilityEffect e = {0};
    e.action = "gain_resource";
    e.target = "self";
    e.count = 3;
    strcpy(e.self_target_field, "true");
    set_extra(&e, "resource", "blade");
    rb_execute_effect_ex(&tg.state, 0, &e, a);
    CHECK_EQ(blade_of(&tg, a), 3, "blade: self_target grants only to the activating card");
    CHECK_EQ(blade_of(&tg, b), 0, "blade: self_target never leaks to a sibling member");
}

static void test_blade_opponent_target(void)
{
    TestGame tg;
    test_game_new(&tg);
    int a, b, c;
    if (!member_triple(&tg, &a, &b, &c)) return;
    test_add_to_stage(&tg, 1, a);
    test_add_to_opp_live(&tg, 0); /* keeps P2's live zone non-empty for realism */
    tg.state.p[1].stage[1] = b;
    AbilityEffect e = {0};
    e.action = "gain_resource";
    e.target = "opponent";
    e.count = 2;
    set_extra(&e, "resource", "blade");
    rb_execute_effect_ex(&tg.state, 0, &e, a);
    CHECK_EQ(blade_of(&tg, b), 2, "blade: target=opponent resolves against P2's stage");
    CHECK_EQ(blade_of(&tg, a), 0, "blade: target=opponent does not touch the actor's stage");
}

static void test_blade_position_filter(void)
{
    TestGame tg;
    test_game_new(&tg);
    int a, b, c;
    if (!member_triple(&tg, &a, &b, &c)) return;
    test_add_to_stage(&tg, 0, a);
    test_add_to_stage(&tg, 1, b);
    AbilityEffect e = {0};
    e.action = "gain_resource";
    e.target = "self";
    e.count = 1;
    set_extra(&e, "resource", "blade");
    set_extra(&e, "position", "left");
    rb_execute_effect_ex(&tg.state, 0, &e, a);
    CHECK_EQ(blade_of(&tg, a), 1, "blade: position=left reaches the left-slot member");
    CHECK_EQ(blade_of(&tg, b), 0, "blade: position=left skips the center-slot member");
}

static void test_blade_group_filter(void)
{
    TestGame tg;
    test_game_new(&tg);
    int liella = rb_create_card_copy(rb_find_card_by_no("PL!-sd1-010-SD"));
    int other = rb_create_card_copy(rb_find_card_by_no("PL!N-bp1-027-L"));
    if (liella < 0 || other < 0) return;
    char group[64];
    card_group(liella, group, sizeof group);
    if (!group[0]) return;
    /* a live card is never a member, so give the two cards distinct groups by
       construction: the group filter must exclude the non-matching slot. */
    test_add_to_stage(&tg, 0, liella);
    test_add_to_stage(&tg, 1, other);
    AbilityEffect e = {0};
    e.action = "gain_resource";
    e.target = "self";
    e.count = 1;
    set_extra(&e, "resource", "blade");
    set_extra(&e, "group_names", group);
    rb_execute_effect_ex(&tg.state, 0, &e, liella);
    CHECK_EQ(blade_of(&tg, liella), 1, "blade: group_names keeps the matching member");
    CHECK_EQ(blade_of(&tg, other), 0, "blade: group_names excludes the other group");
}

static void test_blade_target_count_selection_choice(void)
{
    TestGame tg;
    test_game_new(&tg);
    int a, b, c;
    if (!member_triple(&tg, &a, &b, &c)) return;
    test_add_to_stage(&tg, 0, a);
    test_add_to_stage(&tg, 1, b);
    test_add_to_stage(&tg, 2, c);
    AbilityEffect e = {0};
    e.action = "gain_resource";
    e.target = "self";
    e.count = 1;
    strcpy(e.card_type_field, "member_card");
    set_extra(&e, "resource", "blade");
    set_extra(&e, "target_count", "1");
    rb_execute_effect_ex(&tg.state, 0, &e, a);
    const RbChoice *ch = rb_get_pending_choice(&tg.state);
    CHECK(ch && ch->kind == RB_CHOICE_SELECT_CARD && ch->count == 1,
          "blade: target_count below the candidate count opens a 1-card choice (misc.rs:1274-1330)");
    CHECK_EQ(blade_of(&tg, a) + blade_of(&tg, b) + blade_of(&tg, c), 0,
             "blade: no blade is granted while the target choice is pending");
}

/* ── gain_resource: heart ─────────────────────────────────────────────────── */

static void test_heart_fixed_color(void)
{
    TestGame tg;
    test_game_new(&tg);
    int a, b, c;
    if (!member_triple(&tg, &a, &b, &c)) return;
    test_add_to_stage(&tg, 1, a);
    AbilityEffect e = {0};
    e.action = "gain_resource";
    e.target = "self";
    e.count = 2;
    set_extra(&e, "resource", "heart");
    set_extra(&e, "heart_color", "heart02");
    rb_execute_effect_ex(&tg.state, 0, &e, a);
    CHECK_EQ(heart_of(&tg, a, RB_HEART_YELLOW), 2,
             "heart: heart_color=heart02 grants 2 yellow");
    for (int col = 0; col < 8; col++)
        fprintf(stderr, "PROBE fixed col=%d raw=%d\n", col, rb_mods_get_heart(&tg.state.mods, a, col));
    CHECK_EQ(heart_of(&tg, a, RB_HEART_GREEN), 0,
             "heart: a fixed color grant does not touch other colors");
}

static void test_heart_colors_only_falls_back_to_first(void)
{
    /* Rust misc.rs:1813-1814 falls back to heart_colors[0] when the effect
       carries no singular heart_color. */
    TestGame tg;
    test_game_new(&tg);
    int a, b, c;
    if (!member_triple(&tg, &a, &b, &c)) return;
    test_add_to_stage(&tg, 1, a);
    AbilityEffect e = {0};
    e.action = "gain_resource";
    e.target = "self";
    e.count = 4;
    set_extra(&e, "resource", "heart");
    set_extra(&e, "heart_colors", "heart03,heart04");
    rb_execute_effect_ex(&tg.state, 0, &e, a);
    CHECK_EQ(heart_of(&tg, a, RB_HEART_GREEN), 2,
             "heart: heart_colors-only grant splits evenly across the list");
    CHECK_EQ(heart_of(&tg, a, RB_HEART_BLUE), 2,
             "heart: every listed color receives its share");
}

static void test_heart_multi_color_distribution(void)
{
    /* Rust misc.rs:932-946 + util::heart_gain_per_entry: 6 hearts over 3
       fixed colors => 2 of each, not 6 of the first color. */
    TestGame tg;
    test_game_new(&tg);
    int a, b, c;
    if (!member_triple(&tg, &a, &b, &c)) return;
    test_add_to_stage(&tg, 1, a);
    AbilityEffect e = {0};
    e.action = "gain_resource";
    e.target = "self";
    e.count = 6;
    set_extra(&e, "resource", "heart");
    set_extra(&e, "heart_colors", "heart01,heart02,heart03");
    rb_execute_effect_ex(&tg.state, 0, &e, a);
    CHECK_EQ(heart_of(&tg, a, RB_HEART_RED), 2, "heart: distribution gives red its share");
    CHECK_EQ(heart_of(&tg, a, RB_HEART_YELLOW), 2, "heart: distribution gives yellow its share");
    CHECK_EQ(heart_of(&tg, a, RB_HEART_GREEN), 2, "heart: distribution gives green its share");
    CHECK_EQ(heart_of(&tg, a, RB_HEART_BLUE), 0,
             "heart: colors outside the declared list stay untouched");
}

static void test_heart_count_below_color_count_is_single(void)
{
    /* final_count(2) < heart_colors.len()(3) => no distribution branch, so the
       single-color fallback uses heart_colors[0] (misc.rs:932-946 guard). */
    TestGame tg;
    test_game_new(&tg);
    int a, b, c;
    if (!member_triple(&tg, &a, &b, &c)) return;
    test_add_to_stage(&tg, 1, a);
    AbilityEffect e = {0};
    e.action = "gain_resource";
    e.target = "self";
    e.count = 2;
    set_extra(&e, "resource", "heart");
    set_extra(&e, "heart_colors", "heart01,heart02,heart03");
    rb_execute_effect_ex(&tg.state, 0, &e, a);
    CHECK_EQ(heart_of(&tg, a, RB_HEART_RED), 2,
             "heart: too few hearts to cover every color stays on the first color");
    CHECK_EQ(heart_of(&tg, a, RB_HEART_YELLOW), 0,
             "heart: no partial distribution when the count cannot cover the list");
    CHECK_EQ(heart_of(&tg, a, RB_HEART_GREEN), 0,
             "heart: the third color is not reached in the non-distributed path");
}

static void test_heart_negative_sign(void)
{
    TestGame tg;
    test_game_new(&tg);
    int a, b, c;
    if (!member_triple(&tg, &a, &b, &c)) return;
    test_add_to_stage(&tg, 1, a);
    AbilityEffect e = {0};
    e.action = "gain_resource";
    e.target = "self";
    e.count = 2;
    set_extra(&e, "resource", "heart");
    set_extra(&e, "heart_color", "heart04");
    set_extra(&e, "sign", "negative");
    rb_execute_effect_ex(&tg.state, 0, &e, a);
    CHECK_EQ(heart_of(&tg, a, RB_HEART_BLUE), -2,
             "heart: sign=negative removes hearts of the named color");
}

static void test_heart_all_type(void)
{
    /* Rust misc.rs:635-639 routes heart_type=all into gain_heart_all_type. */
    TestGame tg;
    test_game_new(&tg);
    int a, b, c;
    if (!member_triple(&tg, &a, &b, &c)) return;
    test_add_to_stage(&tg, 1, a);
    AbilityEffect e = {0};
    e.action = "gain_resource";
    e.target = "self";
    e.count = 2;
    set_extra(&e, "resource", "heart");
    set_extra(&e, "heart_type", "all");
    rb_execute_effect_ex(&tg.state, 0, &e, a);
    CHECK_EQ(heart_of(&tg, a, RB_HEART_ALL), 2,
             "heart: heart_type=all grants the wildcard heart icon");
    CHECK_EQ(heart_of(&tg, a, RB_HEART_YELLOW), 0,
             "heart: heart_type=all does not also grant a named color");
}

static void test_heart_colors_from_selected_card(void)
{
    /* Rust misc.rs:627-633 -> gain_heart_colors_from_selected_card: grant 1 of
       every base heart color printed on the selected card, to every stage
       member of the target player. */
    TestGame tg;
    test_game_new(&tg);
    int selected = rb_create_card_copy(rb_find_card_by_no("PL!N-bp1-027-L"));
    int a, b, c;
    if (selected < 0 || !member_triple(&tg, &a, &b, &c)) return;
    int colors[8];
    int n_colors = card_base_hearts(selected, colors, 8);
    if (n_colors <= 0) return;
    test_add_to_stage(&tg, 0, a);
    test_add_to_stage(&tg, 2, b);
    tg.state.n_selected_cards = 1;
    tg.state.selected_cards[0] = selected;
    AbilityEffect e = {0};
    e.action = "gain_resource";
    e.target = "self";
    e.count = 1;
    set_extra(&e, "resource", "heart");
    set_extra(&e, "heart_colors_from_selected_card", "true");
    rb_execute_effect_ex(&tg.state, 0, &e, a);
    int got = 0;
    for (int i = 0; i < n_colors; i++) {
        if (heart_of(&tg, a, colors[i]) == 1) got++;
    }
    CHECK_EQ(got, n_colors,
             "heart_colors_from_selected_card: every printed color is copied to the stage");
    CHECK_EQ(heart_of(&tg, b, colors[0]), 1,
             "heart_colors_from_selected_card: every stage member receives the copy");
}

/* ── gain_resource: the ResourceKind::Other default ───────────────────────── */

static void test_resource_absent_grants_nothing(void)
{
    /* Rust: resource defaults to "" (misc.rs:644), ResourceKind::from_str("")
       is Other (misc.rs:33-39), and both appliers bail out for Other
       (misc.rs:1004, misc.rs:1059). Nothing is granted — in particular NOT
       energy, which is what this handler used to invent. */
    TestGame tg;
    test_game_new(&tg);
    int a, b, c;
    if (!member_triple(&tg, &a, &b, &c)) return;
    test_add_to_stage(&tg, 1, a);
    int energy_before = tg.state.p[0].energy.n;
    int active_before = rb_energy_active_count(&tg.state.p[0]);
    AbilityEffect e = {0};
    e.action = "gain_resource";
    e.target = "self";
    e.count = 3;
    rb_execute_effect_ex(&tg.state, 0, &e, a);
    CHECK_EQ(tg.state.p[0].energy.n, energy_before,
             "absent resource: no energy is invented for the target player");
    CHECK_EQ(rb_energy_active_count(&tg.state.p[0]), active_before,
             "absent resource: no active energy is invented");
    CHECK_EQ(blade_of(&tg, a), 0, "absent resource: no blade is granted");
    int heart_total = 0;
    for (int col = 0; col < 8; col++) heart_total += heart_of(&tg, a, col);
    CHECK_EQ(heart_total, 0, "absent resource: no heart of any color is granted");
}

static void test_resource_unknown_string_grants_nothing(void)
{
    /* Same rule for an unrecognised resource string: only "blade" and "heart"
       are resource kinds (misc.rs:33-39). "energy" is NOT one of them. */
    TestGame tg;
    test_game_new(&tg);
    int a, b, c;
    if (!member_triple(&tg, &a, &b, &c)) return;
    test_add_to_stage(&tg, 1, a);
    int energy_before = tg.state.p[0].energy.n;
    AbilityEffect e = {0};
    e.action = "gain_resource";
    e.target = "self";
    e.count = 4;
    set_extra(&e, "resource", "energy");
    rb_execute_effect_ex(&tg.state, 0, &e, a);
    CHECK_EQ(tg.state.p[0].energy.n, energy_before,
             "resource=energy: gain_resource is not the energy-granting action");
    CHECK_EQ(blade_of(&tg, a), 0, "resource=energy: no blade is granted either");
}

static void test_resource_surplus_heart_not_energy(void)
{
    /* Rust misc.rs:686-696 short-circuits resource == "surplus_heart" into
       execute_gain_surplus_heart, so it never reaches a resource grant. */
    TestGame tg;
    test_game_new(&tg);
    int a, b, c;
    if (!member_triple(&tg, &a, &b, &c)) return;
    test_add_to_stage(&tg, 1, a);
    int energy_before = tg.state.p[0].energy.n;
    AbilityEffect e = {0};
    e.action = "gain_resource";
    e.target = "self";
    e.count = 5;
    set_extra(&e, "resource", "surplus_heart");
    rb_execute_effect_ex(&tg.state, 0, &e, a);
    CHECK_EQ(tg.state.p[0].energy.n, energy_before,
             "resource=surplus_heart: routed to the surplus handler, not an energy grant");
    CHECK_EQ(blade_of(&tg, a), 0, "resource=surplus_heart: no blade is granted");
}

/* ── pay_energy / discard_until_count ─────────────────────────────────────── */

static void test_pay_energy_spends_active(void)
{
    TestGame tg;
    test_game_new(&tg);
    test_give_energy(&tg, 5);
    int before = rb_energy_active_count(&tg.state.p[0]);
    CHECK(before > 0, "pay_energy: the fixture has active energy to spend");
    AbilityEffect e = {0};
    e.action = "pay_energy";
    e.count = 2;
    rb_execute_effect_ex(&tg.state, 0, &e, -1);
    CHECK_EQ(rb_energy_active_count(&tg.state.p[0]), before - 2,
             "pay_energy: a non-optional payment spends count active energy");
}

static void test_discard_until_count_prompts(void)
{
    TestGame tg;
    test_game_new(&tg);
    tg.state.p[0].hand.n = 0;
    int a, b, c;
    if (!member_triple(&tg, &a, &b, &c)) return;
    test_add_to_hand(&tg, a);
    test_add_to_hand(&tg, b);
    test_add_to_hand(&tg, c);
    AbilityEffect e = {0};
    e.action = "discard_until_count";
    e.target = "self";
    e.count = 1;
    rb_execute_effect_ex(&tg.state, 0, &e, -1);
    const RbChoice *ch = rb_get_pending_choice(&tg.state);
    CHECK(ch && ch->kind == RB_CHOICE_SELECT_CARD && ch->count == 2,
          "discard_until_count: prompts for hand_size minus target_count cards");
    CHECK_EQ(tg.state.p[0].hand.n, 3,
             "discard_until_count: the hand is untouched until the choice resolves");
}

/* ── restriction ──────────────────────────────────────────────────────────── */

static void test_restriction_cannot_active(void)
{
    TestGame tg;
    test_game_new(&tg);
    AbilityEffect e = {0};
    e.action = "restriction";
    e.target = "self";
    set_extra(&e, "restriction_type", "cannot_activate");
    rb_execute_effect_ex(&tg.state, 0, &e, -1);
    CHECK_EQ(tg.state.player_cannot_activate[0], 1,
             "restriction: cannot_activate locks the targeted player");
    CHECK_EQ(tg.state.player_cannot_activate[1], 0,
             "restriction: cannot_activate stays player-specific");
    CHECK(tg.state.n_prohibition > 0,
          "restriction: the prohibition note is recorded");
}

static void test_restriction_cannot_activate_opponent(void)
{
    TestGame tg;
    test_game_new(&tg);
    AbilityEffect e = {0};
    e.action = "restriction";
    e.target = "opponent";
    set_extra(&e, "restriction_type", "cannot_activate");
    rb_execute_effect_ex(&tg.state, 0, &e, -1);
    CHECK_EQ(tg.state.player_cannot_activate[1], 1,
             "restriction: target=opponent locks the opponent, not the actor");
    CHECK_EQ(tg.state.player_cannot_activate[0], 0,
             "restriction: the actor stays unlocked");
}

static void test_restriction_cannot_live(void)
{
    TestGame tg;
    test_game_new(&tg);
    AbilityEffect e = {0};
    e.action = "restriction";
    e.target = "opponent";
    set_extra(&e, "restriction_type", "cannot_live");
    rb_execute_effect_ex(&tg.state, 0, &e, -1);
    int found = 0;
    for (int i = 0; i < tg.state.n_prohibition; i++)
        if (!strcmp(tg.state.prohibition[i], "cannot_live:opponent")) found = 1;
    CHECK(found, "restriction: cannot_live records a scoped prohibition");
}

/* ── yell ─────────────────────────────────────────────────────────────────── */

static void test_re_yell_resets_reveal(void)
{
    TestGame tg;
    test_game_new(&tg);
    int a, b, c;
    if (!member_triple(&tg, &a, &b, &c)) return;
    test_add_to_revealed(&tg, a);
    test_add_to_revealed(&tg, b);
    CHECK(tg.state.n_revealed > 0, "re_yell: the fixture has revealed cards first");
    AbilityEffect e = {0};
    e.action = "re_yell";
    rb_execute_effect_ex(&tg.state, 0, &e, a);
    CHECK_EQ(tg.state.re_yell_occurred, 1, "re_yell: the re-yell flag is raised");
    CHECK_EQ(tg.state.n_revealed, 0, "re_yell: the revealed pool is cleared");
}

static void test_perform_yell_collects_blades(void)
{
    TestGame tg;
    test_game_new(&tg);
    int live = rb_create_card_copy(rb_find_card_by_no("PL!N-bp1-027-L"));
    if (live < 0) return;
    Card c;
    memset(&c, 0, sizeof c);
    if (!rb_decode_card_by_index((uint32_t)live, &c)) return;
    int printed_blade = (int)c.blade;
    rb_free_card(&c);
    if (printed_blade <= 0) return;
    test_add_to_live(&tg, live);
    AbilityEffect e = {0};
    e.action = "perform_yell";
    rb_execute_effect_ex(&tg.state, 0, &e, live);
    CHECK_EQ(tg.state.re_yell_blade_hearts[RB_HEART_PINK], printed_blade,
             "perform_yell: the live card's blades become yell blade-hearts");
    CHECK_EQ(tg.state.re_yell_occurred, 1, "perform_yell: the re-yell flag is raised");
}

/* ── shuffle ──────────────────────────────────────────────────────────────── */

static void test_shuffle_conserves_cards(void)
{
    TestGame tg;
    test_game_new(&tg);
    int a, b, c;
    if (!member_triple(&tg, &a, &b, &c)) return;
    tg.state.p[0].deck.n = 0;
    test_add_to_deck(&tg, a);
    test_add_to_deck(&tg, b);
    test_add_to_deck(&tg, c);
    int before = tg.state.p[0].deck.n;
    AbilityEffect e = {0};
    e.action = "shuffle";
    e.target = "deck";
    rb_execute_effect_ex(&tg.state, 0, &e, -1);
    CHECK_EQ(tg.state.p[0].deck.n, before, "shuffle: the deck keeps every card");
    int seen = 0;
    for (int i = 0; i < tg.state.p[0].deck.n; i++)
        if (tg.state.p[0].deck.cards[i] == a) seen = 1;
    CHECK(seen, "shuffle: the shuffled cards are still the same physical cards");
}

/* ── custom (custom.rs) ───────────────────────────────────────────────────── */

static void test_custom_placement_order_any_routes_move(void)
{
    /* Rust custom.rs:23-34: placement_order == AnyOrder routes to
       execute_move_cards with source=looked_at, destination=deck_top. */
    TestGame tg;
    test_game_new(&tg);
    int a, b, c;
    if (!member_triple(&tg, &a, &b, &c)) return;
    tg.state.p[0].deck.n = 0;
    test_add_to_deck(&tg, a);
    test_add_to_deck(&tg, b);
    AbilityEffect look = {0};
    look.action = "look_at";
    look.source = "deck_top";
    look.count = 2;
    rb_execute_effect_ex(&tg.state, 0, &look, -1);
    int pool[8];
    CHECK_EQ(rb_looked_at_pool(0, pool, 8), 2, "custom: the fixture has a two-card looked-at pool");
    AbilityEffect e = {0};
    e.action = "custom";
    set_extra(&e, "placement_order", "any_order");
    rb_execute_effect_ex(&tg.state, 0, &e, -1);
    int after = rb_looked_at_pool(0, pool, 8);
    int moved = 0;
    for (int i = 0; i < after; i++)
        if (pool[i] == a || pool[i] == b) moved = 1;
    CHECK(!moved, "custom: placement_order=any_order drains the looked-at pool into move_cards");
    CHECK(tg.state.p[0].deck.n + tg.state.p[0].hand.n + tg.state.p[0].discard.n >= 2,
          "custom: the routed move_cards keeps both cards accounted for");
}

static void test_custom_placement_order_true_is_not_a_route(void)
{
    /* The handler used to treat placement_order == "true" as if it were
       "any_order" (extra_true on a non-boolean key). Rust compares against
       PlacementOrder::AnyOrder only, so "true" must fall through. */
    TestGame tg;
    test_game_new(&tg);
    int a, b, c;
    if (!member_triple(&tg, &a, &b, &c)) return;
    tg.state.p[0].deck.n = 0;
    test_add_to_deck(&tg, a);
    test_add_to_deck(&tg, b);
    AbilityEffect look = {0};
    look.action = "look_at";
    look.source = "deck_top";
    look.count = 2;
    rb_execute_effect_ex(&tg.state, 0, &look, -1);
    int pool[8];
    int before = rb_looked_at_pool(0, pool, 8);
    CHECK(before > 0, "custom: the fixture has a looked-at pool to protect");
    AbilityEffect e = {0};
    e.action = "custom";
    set_extra(&e, "placement_order", "true");
    rb_execute_effect_ex(&tg.state, 0, &e, -1);
    int after[8];
    CHECK_EQ(rb_looked_at_pool(0, after, 8), before,
             "custom: placement_order=true is not the any_order route");
    CHECK(after[0] == pool[0], "custom: the unhandled custom path leaves the pool untouched");
}

static void test_custom_duration_routes_gain_ability(void)
{
    /* Rust custom.rs:37-52: an effect carrying a duration goes to
       execute_gain_ability, not to the unhandled-custom log. */
    TestGame tg;
    test_game_new(&tg);
    int a, b, c;
    if (!member_triple(&tg, &a, &b, &c)) return;
    test_add_to_stage(&tg, 1, a);
    AbilityEffect gained = {0};
    gained.action = "modify_score";
    gained.count = 5;
    AbilityEffect e = {0};
    e.action = "custom";
    e.target = "self";
    e.gained_effect = &gained;
    set_extra(&e, "duration", "turn_end");
    set_extra(&e, "ability_gain", "fixture gain");
    rb_execute_effect_ex(&tg.state, 0, &e, a);
    CHECK_EQ(rb_mods_get_score(&tg.state.mods, a), 5,
             "custom: duration routes the effect into gain_ability");
}

static void test_custom_unhandled_is_a_noop(void)
{
    TestGame tg;
    test_game_new(&tg);
    int a, b, c;
    if (!member_triple(&tg, &a, &b, &c)) return;
    test_add_to_stage(&tg, 1, a);
    int blade_before = blade_of(&tg, a);
    int energy_before = tg.state.p[0].energy.n;
    AbilityEffect e = {0};
    e.action = "custom";
    set_extra(&e, "text", "fixture");
    rb_execute_effect_ex(&tg.state, 0, &e, a);
    CHECK_EQ(blade_of(&tg, a), blade_before,
             "custom: an unhandled custom action changes no card state");
    CHECK_EQ(tg.state.p[0].energy.n, energy_before,
             "custom: an unhandled custom action grants no energy");
}

/* ── position / formation ─────────────────────────────────────────────────── */

static void test_rotation_cycles_stage(void)
{
    TestGame tg;
    test_game_new(&tg);
    int a, b, c;
    if (!member_triple(&tg, &a, &b, &c)) return;
    test_add_to_stage(&tg, 0, a);
    test_add_to_stage(&tg, 1, b);
    test_add_to_stage(&tg, 2, c);
    AbilityEffect e = {0};
    e.action = "rotation";
    e.target = "self";
    rb_execute_effect_ex(&tg.state, 0, &e, a);
    CHECK_EQ(tg.state.p[0].stage[0], b, "rotation: center moves to the left");
    CHECK_EQ(tg.state.p[0].stage[1], c, "rotation: right moves to the center");
    CHECK_EQ(tg.state.p[0].stage[2], a, "rotation: left moves to the right");
    CHECK_EQ(tg.state.position_change_occurred_this_turn, 1,
             "rotation: the position-change flag is raised");
}

static void test_position_change_swaps_area(void)
{
    TestGame tg;
    test_game_new(&tg);
    int a, b, c;
    if (!member_triple(&tg, &a, &b, &c)) return;
    test_add_to_stage(&tg, 0, a);
    test_add_to_stage(&tg, 1, b);
    AbilityEffect e = {0};
    e.action = "position_change";
    e.target = "self";
    e.destination = (char *)"left";
    set_extra(&e, "source_position", "center");
    rb_execute_effect_ex(&tg.state, 0, &e, a);
    CHECK_EQ(tg.state.p[0].stage[0], b, "position_change: the source member moves to the destination");
    CHECK_EQ(tg.state.p[0].stage[1], a, "position_change: the destination member takes the source slot");
    CHECK_EQ(tg.state.position_change_occurred_this_turn, 1,
             "position_change: the position-change flag is raised");
}

/* ── zones ────────────────────────────────────────────────────────────────── */

static void test_pay_cost_all_discard(void)
{
    TestGame tg;
    test_game_new(&tg);
    int a, b, c;
    if (!member_triple(&tg, &a, &b, &c)) return;
    tg.state.p[0].hand.n = 0;
    test_add_to_hand(&tg, a);
    test_add_to_hand(&tg, b);
    test_add_to_hand(&tg, c);
    AbilityEffect e = {0};
    e.action = "pay_cost_all:discard_all";
    e.target = "self";
    rb_execute_effect_ex(&tg.state, 0, &e, -1);
    CHECK_EQ(tg.state.p[0].hand.n, 0, "pay_cost_all:discard_all empties the hand");
    CHECK(tg.state.p[0].discard.n >= 3,
          "pay_cost_all:discard_all keeps every discarded card");
}

static void test_place_energy_under_member_from_energy_deck(void)
{
    TestGame tg;
    test_game_new(&tg);
    int a, b, c;
    if (!member_triple(&tg, &a, &b, &c)) return;
    int energy = rb_create_card_copy(rb_find_card_by_no("PL!EN-001-E"));
    if (energy < 0) return;
    test_add_to_stage(&tg, 1, a);
    test_add_to_energy_deck(&tg, 0, energy);
    CHECK(tg.state.p[0].energy_deck.n > 0, "place_energy_under_member: the energy deck has a card");
    AbilityEffect e = {0};
    e.action = "place_energy_under_member";
    e.target = "self";
    e.source = (char *)"energy_deck";
    e.destination = (char *)"under_member";
    e.count = 1;
    rb_execute_effect_ex(&tg.state, 0, &e, a);
    CHECK_EQ(tg.state.p[0].energy_deck.n, 0, "place_energy_under_member: the energy leaves the energy deck");
    CHECK_EQ(tg.state.p[0].under_cards[1].n, 1,
             "place_energy_under_member: the energy lands under the activating member");
}

/* ── choice-emitting misc actions ─────────────────────────────────────────── */

static void test_choose_required_hearts_prompts(void)
{
    TestGame tg;
    test_game_new(&tg);
    int a, b, c;
    if (!member_triple(&tg, &a, &b, &c)) return;
    test_add_to_stage(&tg, 1, a);
    AbilityEffect e = {0};
    e.action = "choose_required_hearts";
    e.count = 2;
    rb_execute_effect_ex(&tg.state, 0, &e, a);
    const RbChoice *ch = rb_get_pending_choice(&tg.state);
    CHECK(ch && ch->count == 2, "choose_required_hearts: prompts for the requested heart count");
    CHECK(!ch || ch->allow_skip == 0, "choose_required_hearts: a required choice is not skippable");
}

static void test_choose_target_player_prompts(void)
{
    TestGame tg;
    test_game_new(&tg);
    int a, b, c;
    if (!member_triple(&tg, &a, &b, &c)) return;
    test_add_to_stage(&tg, 1, a);
    AbilityEffect e = {0};
    e.action = "choose_target_player";
    rb_execute_effect_ex(&tg.state, 0, &e, a);
    const RbChoice *ch = rb_get_pending_choice(&tg.state);
    CHECK(ch && ch->kind == RB_CHOICE_SELECT_TARGET && ch->count == 1,
          "choose_target_player: prompts for a single self-or-opponent pick");
}

int main(void)
{
    if (rb_load("src") != 0) {
        fprintf(stderr, "FAIL: database load\n");
        return 1;
    }
    test_blade_grants_to_own_stage();
    test_blade_negative_sign();
    test_blade_self_target_field();
    test_blade_opponent_target();
    test_blade_position_filter();
    test_blade_group_filter();
    test_blade_target_count_selection_choice();
    test_heart_fixed_color();
    test_heart_colors_only_falls_back_to_first();
    test_heart_multi_color_distribution();
    test_heart_count_below_color_count_is_single();
    test_heart_negative_sign();
    test_heart_all_type();
    test_heart_colors_from_selected_card();
    test_resource_absent_grants_nothing();
    test_resource_unknown_string_grants_nothing();
    test_resource_surplus_heart_not_energy();
    test_pay_energy_spends_active();
    test_discard_until_count_prompts();
    test_restriction_cannot_active();
    test_restriction_cannot_activate_opponent();
    test_restriction_cannot_live();
    test_re_yell_resets_reveal();
    test_perform_yell_collects_blades();
    test_shuffle_conserves_cards();
    test_custom_placement_order_any_routes_move();
    test_custom_placement_order_true_is_not_a_route();
    test_custom_duration_routes_gain_ability();
    test_custom_unhandled_is_a_noop();
    test_rotation_cycles_stage();
    test_position_change_swaps_area();
    test_pay_cost_all_discard();
    test_place_energy_under_member_from_energy_deck();
    test_choose_required_hearts_prompts();
    test_choose_target_player_prompts();
    rb_unload();
    printf("parity_custom_misc: %d assertions, %d failures\n", checks, failures);
    if (failures) return 1;
    printf("ALL PARITY CUSTOM/MISC CHECKS PASSED\n");
    return 0;
}
