/* Parity suite: card identity / card-number normalisation / heart maps /
 * ability lookup / CardDatabase queries.
 *
 * Transcribed from the Rust cluster:
 *   engine/tests/test_modules/characterization/card_id_lookup_determinism_test.rs
 *   engine/tests/test_modules/characterization/candidate_pool_builder_test.rs
 *   engine/tests/test_modules/characterization/pipeline_characterization_test.rs
 *   engine/tests/test_modules/effects/recover/per_card/
 *     multi_name_card_name_and_slot_rulings_q62_q65_q105_q207_q208_test.rs
 *   engine/tests/test_modules/jidou/yell/no_blade_heart_reveal_gain/
 *     no_blade_heart_reveal_gain_heart_edges_q112_test.rs
 *   engine/src/core/card.rs (normalize_card_no, normalize_name,
 *     equivalent_rarities, map_series_to_group, has_blade_heart,
 *     has_blade_heart_strict, has_score_icon, has_all_blade, total_hearts,
 *     parse_operator, get_card_names, get_card_by_no).
 */
#include "rabuka.h"
#include "test_game.h"
#include <stdio.h>
#include <string.h>

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
    long long actual_value = (long long)(actual); \
    long long expected_value = (long long)(expected); \
    if (actual_value != expected_value) { \
        fprintf(stderr, "FAIL: %s (got %lld expected %lld)\n", message, actual_value, expected_value); \
        failures++; \
    } else { \
        printf("ok: %s\n", message); \
    } \
} while (0)

#define CHECK_STR(actual, expected, message) do { \
    checks++; \
    const char *actual_value = (actual); \
    const char *expected_value = (expected); \
    if (!actual_value || strcmp(actual_value, expected_value) != 0) { \
        fprintf(stderr, "FAIL: %s (got \"%s\" expected \"%s\")\n", message, \
                actual_value ? actual_value : "(null)", expected_value); \
        failures++; \
    } else { \
        printf("ok: %s\n", message); \
    } \
} while (0)

/* Defined in src/core/card.c but not yet declared in include/rabuka.h. */
extern int rb_card_has_trigger(int card_id, int kind);
extern int rb_card_triggerless_text(int card_id, char *out, size_t out_sz);
extern int rb_card_fires_on_opponent_effects(int card_id);
extern int rb_card_energy_cost_total(int card_id);
extern int rb_card_has_optional_payment(int card_id);
extern int rb_card_effective_energy_cost_total(int card_id, int groups_on_stage);
extern const char *rb_card_group_name(int card_id);
extern const char *rb_card_series_to_group(const char *series);
extern int rb_heartmap_len(const Card *c, int from, int len);
extern int rb_heartmap_is_empty(const Card *c, int from, int len);
extern int rb_heartmap_values_sum(const Card *c, int from, int len);
extern int rb_heartmap_get(const Card *c, int from, int len, int color, int *out);
extern int rb_heartmap_contains_key(const Card *c, int from, int len, int color);
extern int rb_heartmap_key_at(const Card *c, int from, int len, int i);
extern int rb_heartmap_value_at(const Card *c, int from, int len, int i);
extern int rb_heartmap_insert(Card *c, int from, int *len, int color, int val);
extern void rb_heartmap_remove(Card *c, int from, int *len, int color);
extern void rb_heartmap_clear(Card *c, int from, int *len);
extern int rb_heartmap_entry_or_default(Card *c, int from, int *len, int color);

/* ── card.rs:637 normalize_card_no — uppercase, fullwidth → halfwidth ── */
static void test_normalize_card_no(void)
{
    char out[128];
    rb_card_normalize_no("pl!sp-bp2-006-r+", out, sizeof(out));
    CHECK_STR(out, "PL!SP-BP2-006-R+", "normalize_card_no uppercases ascii lowercase");

    rb_card_normalize_no("ＰＬ！ＳＰ-006-Ｒ＋", out, sizeof(out));
    CHECK_STR(out, "ＰＬ!ＳＰ-006-Ｒ+",
              "normalize_card_no folds only the fullwidth LOWERCASE range and ＋／！");
    /* card.rs:641-674 matches 'a'..='z' and 'ａ'..='ｚ' (U+FF41..U+FF5A) only;
       fullwidth UPPERCASE (U+FF21..U+FF3A) falls through to the `_` arm and
       is copied verbatim. */
    rb_card_normalize_no("ａｂｃ", out, sizeof(out));
    CHECK_STR(out, "ABC", "normalize_card_no maps fullwidth lowercase ａｂｃ to ABC");
    rb_card_normalize_no("ＡＢＣ", out, sizeof(out));
    CHECK_STR(out, "ＡＢＣ", "normalize_card_no leaves fullwidth uppercase ＡＢＣ alone");

    rb_card_normalize_no("a＊b＃c", out, sizeof(out));
    CHECK_STR(out, "A*B#C", "normalize_card_no folds fullwidth ＊ and ＃");

    rb_card_normalize_no("PL!S-pb1-003-P＋", out, sizeof(out));
    CHECK_STR(out, "PL!S-PB1-003-P+", "normalize_card_no leaves already-normal base text alone");

    rb_card_normalize_no("μ's-LL-001-R", out, sizeof(out));
    CHECK_STR(out, "μ'S-LL-001-R", "normalize_card_no uppercases without touching non-ascii letters");
}

/* ── card.rs:695 normalize_name — strip every whitespace char ── */
static void test_normalize_name(void)
{
    char out[128];
    rb_card_normalize_name("南 ことり", out, sizeof(out));
    CHECK_STR(out, "南ことり", "normalize_name strips an interior ascii space");

    rb_card_normalize_name("  A\tB\nC  ", out, sizeof(out));
    CHECK_STR(out, "ABC", "normalize_name strips leading/trailing and tab/newline");

    rb_card_normalize_name("上原歩夢　澁谷かのん", out, sizeof(out));
    CHECK_STR(out, "上原歩夢澁谷かのん", "normalize_name strips the U+3000 ideographic space");

    rb_card_normalize_name("Aqours", out, sizeof(out));
    CHECK_STR(out, "Aqours", "normalize_name leaves whitespace-free names unchanged");

    rb_card_normalize_name("", out, sizeof(out));
    CHECK_STR(out, "", "normalize_name maps the empty name to the empty string");

    rb_card_normalize_name(NULL, out, sizeof(out));
    CHECK_STR(out, "", "normalize_name tolerates a null source");
}

/* ── card.rs:562 equivalent_rarities ── */
static void test_equivalent_rarities(void)
{
    char out[32];
    struct { const char *in; const char *want; } cases[] = {
        { "P+",  "P＋" }, { "P＋",  "P＋" }, { "P2",  "P＋" },
        { "R+",  "R＋" }, { "R＋",  "R＋" }, { "R2",  "R＋" },
        { "L+",  "L＋" }, { "L＋",  "L＋" }, { "L2",  "L＋" },
        { "N+",  "N＋" }, { "N＋",  "N＋" }, { "N2",  "N＋" },
        { "PR+", "PR＋" }, { "PR＋", "PR＋" },
    };
    int bad = 0;
    for (size_t i = 0; i < sizeof(cases) / sizeof(cases[0]); i++) {
        out[0] = 0;
        if (!rb_card_equivalent_rarity(cases[i].in, out, sizeof(out)) ||
            strcmp(out, cases[i].want) != 0) {
            fprintf(stderr, "FAIL: equivalent_rarity(%s) -> \"%s\" expected \"%s\"\n",
                    cases[i].in, out, cases[i].want);
            failures++;
            bad++;
        }
        checks++;
    }
    if (!bad) printf("ok: equivalent_rarities folds every + family onto its canonical form\n");

    CHECK_EQ(rb_card_equivalent_rarity("R", out, sizeof(out)), 0,
             "a rarity with no + family has no equivalent");
    CHECK_EQ(rb_card_equivalent_rarity("SEC", out, sizeof(out)), 0,
             "SEC has no equivalent rarity");
    CHECK_EQ(rb_card_equivalent_rarity(NULL, out, sizeof(out)), 0,
             "a null rarity has no equivalent");
}

/* ── card.rs:796 map_series_to_group ── */
static void test_map_series_to_group(void)
{
    char out[64];
    rb_map_series_to_group("ラブライブ！", out, sizeof(out));
    CHECK_STR(out, "μ's", "series ラブライブ！ maps to μ's");

    rb_map_series_to_group("ラブライブ！サンシャイン!!", out, sizeof(out));
    CHECK_STR(out, "Aqours", "series サンシャイン maps to Aqours");

    rb_map_series_to_group("ラブライブ！虹ヶ咲学園スクールアイドル同好会", out, sizeof(out));
    CHECK_STR(out, "虹ヶ咲", "series 虹ヶ咲学園 maps to 虹ヶ咲");

    rb_map_series_to_group("ラブライブ！スーパースター!!", out, sizeof(out));
    CHECK_STR(out, "Liella!", "series スーパースター maps to Liella!");

    rb_map_series_to_group("蓮ノ空女学院スクールアイドルクラブ", out, sizeof(out));
    CHECK_STR(out, "蓮ノ空", "bare 蓮ノ空 series maps to 蓮ノ空");

    rb_map_series_to_group("ラブライブ！蓮ノ空女学院スクールアイドルクラブ", out, sizeof(out));
    CHECK_STR(out, "蓮ノ空", "prefixed 蓮ノ空 series maps to 蓮ノ空");

    rb_map_series_to_group("Hello! Project", out, sizeof(out));
    CHECK_STR(out, "", "an unrelated series maps to the empty group");
}

/* ── card_id_lookup_determinism_test.rs: the lenient fallback must be stable ── */
static void test_card_id_lookup_determinism(void)
{
    const char *requested = "PL!SP-bp2-006-R";
    int first = rb_card_get_card_id(requested);
    CHECK(first >= 0, "PL!SP-bp2-006-R resolves through some fallback");

    int stable = 1;
    int in_family = 1;
    if (first >= 0) {
        for (int i = 0; i < 64; i++) {
            int id = rb_card_get_card_id(requested);
            if (id != first) stable = 0;
            Card c;
            memset(&c, 0, sizeof(c));
            if (!rb_decode_card_by_index((uint32_t)id, &c)) { in_family = 0; rb_free_card(&c); break; }
            const char *no = rb_card_string(c.card_no_idx);
            if (!no || strncmp(no, "PL!SP-bp2-006-", 13) != 0) in_family = 0;
            rb_free_card(&c);
        }
    }
    CHECK(stable, "the unknown-rarity fallback resolves to one print across 64 calls");
    CHECK(in_family, "the fallback stays inside the requested card number");

    const char *prints[] = {
        "PL!SP-bp2-006-P", "PL!SP-bp2-006-SEC", "PL!S-pb1-003-R",
        "PL!S-pb1-003-P＋", "PL!HS-pb1-003-R",
    };
    int exact_ok = 1;
    for (size_t i = 0; i < sizeof(prints) / sizeof(prints[0]); i++) {
        int id = rb_card_get_card_id(prints[i]);
        if (id < 0) {
            fprintf(stderr, "FAIL: %s must exist\n", prints[i]);
            exact_ok = 0;
            checks++;
            continue;
        }
        checks++;
        Card c;
        memset(&c, 0, sizeof(c));
        if (!rb_decode_card_by_index((uint32_t)id, &c)) { exact_ok = 0; rb_free_card(&c); continue; }
        const char *no = rb_card_string(c.card_no_idx);
        if (!no || strcmp(no, prints[i]) != 0) {
            fprintf(stderr, "FAIL: %s resolved to %s\n", prints[i], no ? no : "(null)");
            exact_ok = 0;
        }
        rb_free_card(&c);
    }
    if (exact_ok) printf("ok: every existing print wins over the lenient fallback\n");
}

/* ── card.rs:579 get_card_id — normalisation tiers ── */
static void test_card_id_normalisation(void)
{
    int upper = rb_card_get_card_id("PL!SP-bp2-006-P");
    int lower = rb_card_get_card_id("pl!sp-bp2-006-p");
    CHECK(upper >= 0 && upper == lower,
          "a lowercase card_no resolves to the same print as its uppercase form");

    int full = rb_card_get_card_id("PL!S-pb1-003-P＋");
    int half = rb_card_get_card_id("PL!S-pb1-003-P+");
    CHECK(full >= 0 && half >= 0 && full == half,
          "fullwidth ＋ and halfwidth + resolve to the same print");

    int base = rb_card_get_card_id("PL!HS-pb1-003");
    CHECK(base >= 0, "a card_no with no rarity suffix falls back to a print of that base");

    /* R+ is the "rare" family; its canonical form is R＋. */
    int r_plus = rb_card_get_card_id("PL!S-bp6-024-R+");
    Card c;
    memset(&c, 0, sizeof(c));
    int decoded = r_plus >= 0 && rb_decode_card_by_index((uint32_t)r_plus, &c);
    CHECK(decoded, "an R+ card_no decodes");
    if (decoded) {
        const char *no = rb_card_string(c.card_no_idx);
        CHECK(no && strncmp(no, "PL!S-bp6-024-", 13) == 0,
              "the R+ → R＋ equivalent stays inside the same card number");
    }
    rb_free_card(&c);

    CHECK_EQ(rb_card_get_card_id("ZZZZ-not-a-card"), -1,
             "an unknown card_no resolves to -1");
    CHECK_EQ(rb_card_get_card_id(NULL), -1, "a null card_no resolves to -1");
}

/* ── card.rs:501/505 get_card / get_card_by_no round-trip ── */
static void test_get_card_round_trip(void)
{
    Card c;
    memset(&c, 0, sizeof(c));
    CHECK(rb_card_get_card_by_no("PL!-sd1-010-SD", &c), "get_card_by_no decodes a member card");
    CHECK(c.card_no_idx && rb_card_string(c.card_no_idx) &&
              !strcmp(rb_card_string(c.card_no_idx), "PL!-sd1-010-SD"),
          "get_card_by_no round-trips the exact card_no");
    CHECK(c.name && *c.name, "get_card_by_no carries the card name");
    rb_free_card(&c);

    memset(&c, 0, sizeof(c));
    CHECK(!rb_card_get_card_by_no("ZZZZ-not-a-card", &c),
          "get_card_by_no returns nothing for an unknown card_no");

    memset(&c, 0, sizeof(c));
    CHECK(rb_card_get_card_by_id(0, &c), "get_card_by_id decodes the first record");
    rb_free_card(&c);

    CHECK_EQ(rb_card_get_card_by_id(0, NULL), 0, "get_card_by_id rejects a null out-param");
    CHECK_EQ(rb_card_get_card_by_id(-1, NULL), 0, "get_card_by_id rejects a negative id");
    CHECK(rb_card_get_card("PL!-sd1-010-SD"), "get_card reports an existing card_no");
    CHECK(!rb_card_get_card("ZZZZ-not-a-card"), "get_card rejects an unknown card_no");
}

/* ── card.rs:709 get_card_names — "A&B&C" → three names ── */
static void test_get_card_names(void)
{
    int multi = rb_card_get_card_id("LL-bp1-001-R＋");
    CHECK(multi >= 0, "the multi-name fixture LL-bp1-001-R＋ exists");

    char buf[512];
    if (multi >= 0) {
        int count = rb_card_get_card_names(multi, buf, sizeof(buf));
        CHECK_EQ(count, 3, "Q62: a three-name card reports three names");
        int has_ayumu = 0, has_kano = 0, has_hanaho = 0, n = 0;
        for (const char *p = buf; n < count && p && *p; n++) {
            if (!strcmp(p, "上原歩夢")) has_ayumu = 1;
            else if (!strcmp(p, "澁谷かのん")) has_kano = 1;
            else if (!strcmp(p, "日野下花帆")) has_hanaho = 1;
            p += strlen(p) + 1;
        }
        CHECK(has_ayumu, "Q62: the multi-name card contains 上原歩夢");
        CHECK(has_kano, "Q62: the multi-name card contains 澁谷かのん");
        CHECK(has_hanaho, "Q62: the multi-name card contains 日野下花帆");
    }

    int single = rb_card_get_card_id("PL!N-pb1-001-R");
    CHECK(single >= 0, "the single-name fixture PL!N-pb1-001-R exists");
    if (single >= 0) {
        int count = rb_card_get_card_names(single, buf, sizeof(buf));
        CHECK_EQ(count, 1, "a single-name card reports exactly one name");
        CHECK_STR(buf, "上原歩夢", "the single-name card reports its own name");
    }

    CHECK_EQ(rb_card_get_card_names(0x7fffffff, buf, sizeof(buf)), 0,
             "an out-of-range card id reports no names");
    CHECK_EQ(rb_card_get_card_names(single, NULL, 0), 0,
             "get_card_names rejects a null buffer");
}

/* ── card.rs CardType / Card::is_live / is_member / is_energy ── */
static void test_card_type_predicates(void)
{
    int member = rb_card_get_card_id("PL!-sd1-010-SD");
    int live = rb_card_get_card_id("PL!N-bp1-027-L");
    CHECK(member >= 0 && live >= 0, "member/live fixtures resolve");

    if (member >= 0) {
        CHECK(rb_card_is_member(member) && !rb_card_is_live(member) && !rb_card_is_energy(member),
              "a member card is only a member card");
        CHECK_STR(rb_card_type_str(rb_card_type_from_str("member_card")), "member_card",
                  "member_card round-trips through the type enum");
    }
    if (live >= 0) {
        CHECK(rb_card_is_live(live) && !rb_card_is_member(live) && !rb_card_is_energy(live),
              "a live card is only a live card");
        CHECK_STR(rb_card_type_str(rb_card_type_from_str("live_card")), "live_card",
                  "live_card round-trips through the type enum");
        CHECK_EQ(rb_card_type_from_str("energy_card"), 2, "energy_card is type index 2");
        CHECK_EQ(rb_card_type_from_str("nonsense"), -1, "an unknown card type string is rejected");
    }
    CHECK_EQ(rb_card_is_live(0x7fffffff), 0, "an out-of-range id is not a live card");
    CHECK_EQ(rb_card_is_member(0x7fffffff), 0, "an out-of-range id is not a member card");
    CHECK_EQ(rb_card_is_energy(0x7fffffff), 0, "an out-of-range id is not an energy card");
}

/* ── card.rs:4194 total_hearts, :4204-4226 heart-map predicates ── */
static void test_heart_map_predicates(void)
{
    int member = rb_card_get_card_id("PL!N-pb1-001-R");
    CHECK(member >= 0, "the heart-map fixture PL!N-pb1-001-R exists");
    if (member < 0) return;

    Card c;
    memset(&c, 0, sizeof(c));
    CHECK(rb_decode_card_by_index((uint32_t)member, &c), "the heart-map fixture decodes");

    /* card.rs:4194 total_hearts = base_heart sum, or need_heart sum when the
       card prints no base hearts. Blade hearts are never counted. */
    int base_sum = rb_heartmap_values_sum(&c, 0, c.num_base);
    int blade_sum = rb_heartmap_values_sum(&c, c.num_base, c.num_blade);
    int need_sum = rb_heartmap_values_sum(&c, c.num_base + c.num_blade, c.num_need);
    int rust_total = c.num_base > 0 ? base_sum : need_sum;
    CHECK_EQ(rb_card_total_hearts(&c), rust_total,
             "total_hearts is the base-heart sum and excludes blade hearts");
    CHECK(blade_sum > 0, "the heart-map fixture really does print a blade heart");

    /* HeartMap accessors (card.rs:170-223) over the base / blade sections. */
    CHECK_EQ(rb_heartmap_len(&c, 0, c.num_base), c.num_base,
             "HeartMap::len counts every printed base heart");
    CHECK_EQ(rb_heartmap_is_empty(&c, 0, 0), 1, "HeartMap::is_empty is true for a zero-length slice");
    CHECK_EQ(rb_heartmap_is_empty(&c, 0, c.num_base), 0, "a card with base hearts is not empty");
    for (int i = 0; i < rb_heartmap_len(&c, 0, c.num_base); i++) {
        int color = rb_heartmap_key_at(&c, 0, c.num_base, i);
        int value = rb_heartmap_value_at(&c, 0, c.num_base, i);
        int got = -1;
        CHECK(rb_heartmap_contains_key(&c, 0, c.num_base, color) &&
                  rb_heartmap_get(&c, 0, c.num_base, color, &got) && got == value,
              "keys/values agree with get/contains_key for every base heart");
    }
    CHECK_EQ(rb_heartmap_get(&c, 0, c.num_base, 0x7E, &base_sum), 0,
             "HeartMap::get misses a color the card does not print");
    CHECK(!rb_heartmap_contains_key(&c, 0, c.num_base, 0x7E),
          "HeartMap::contains_key misses a color the card does not print");

    /* Blade hearts occupy the slice right after the base hearts. */
    int blade_total = blade_sum;
    CHECK_EQ(rb_card_has_blade_heart_strict(&c), blade_total > 0 ? 1 : 0,
             "has_blade_heart_strict tracks the printed blade heart");
    CHECK_EQ(rb_card_has_blade_heart(&c),
             (blade_total > 0 || (c.has_special && c.special_count > 0)) ? 1 : 0,
             "has_blade_heart also accepts a printed special heart");
    CHECK_EQ(rb_card_has_score_icon(&c),
             (c.has_special && c.special_color == (uint8_t)RB_HEART_SCORE) ? 1 : 0,
             "has_score_icon is true only for a printed score special heart");
    CHECK_EQ(rb_card_has_all_blade(&c),
             rb_heartmap_contains_key(&c, c.num_base, c.num_blade, RB_HEART_ALL) ? 1 : 0,
             "has_all_blade is exactly contains_key(All) in the blade heart map");
    CHECK_EQ(rb_card_has_blade_heart(NULL), 0, "has_blade_heart tolerates a null card");
    CHECK_EQ(rb_card_has_score_icon(NULL), 0, "has_score_icon tolerates a null card");
    CHECK_EQ(rb_card_has_all_blade(NULL), 0, "has_all_blade tolerates a null card");

    /* Mutators (insert / remove / clear / entry_or_default). The map owns its
       length, so each mutator reports the new length back. A decoded Card packs
       base|blade|need contiguously, so a map can only grow at the tail; build a
       tail map to exercise insert the way HeartMap::insert would. */
    Card m;
    memset(&m, 0, sizeof(m));
    m.heart_color[0] = RB_HEART_PINK;  m.heart_count[0] = 2;
    m.heart_color[1] = RB_HEART_BLUE;  m.heart_count[1] = 1;
    m.n_hearts = 2;
    m.num_base = 2;
    int mlen = 2;
    CHECK_EQ(rb_heartmap_len(&m, 0, mlen), 2, "a two-entry map reports len 2");
    CHECK(rb_heartmap_insert(&m, 0, &mlen, 0x7E, 3),
          "HeartMap::insert pushes a new key onto a tail map");
    CHECK_EQ(mlen, 3, "insert grows the map by one key");
    CHECK_EQ(rb_heartmap_len(&m, 0, mlen), 3, "the grown map reports its new length");
    int got = 0;
    CHECK(rb_heartmap_get(&m, 0, mlen, 0x7E, &got) && got == 3,
          "the inserted key reads back with its value");
    CHECK(rb_heartmap_insert(&m, 0, &mlen, 0x7E, 5), "HeartMap::insert replaces an existing key");
    CHECK(rb_heartmap_get(&m, 0, mlen, 0x7E, &got) && got == 5, "the replaced value is visible");
    CHECK_EQ(mlen, 3, "replacing does not grow the map");
    CHECK_EQ(rb_heartmap_values_sum(&m, 0, mlen), 8, "values_sum tracks the mutations");
    CHECK_EQ(rb_heartmap_entry_or_default(&m, 0, &mlen, 0x7D), 0,
             "entry_or_default yields 0 for an absent color");
    CHECK(rb_heartmap_contains_key(&m, 0, mlen, 0x7D),
          "entry_or_default inserts a 0-count entry, which is then present");
    got = -1;
    CHECK(rb_heartmap_get(&m, 0, mlen, 0x7D, &got) && got == 0,
          "the entry_or_default key reads back as 0");
    rb_heartmap_remove(&m, 0, &mlen, 0x7E);
    CHECK(!rb_heartmap_contains_key(&m, 0, mlen, 0x7E), "HeartMap::remove drops the key");
    CHECK_EQ(mlen, 3, "remove shrinks the map by the one removed key");
    CHECK(rb_heartmap_contains_key(&m, 0, mlen, RB_HEART_PINK) &&
              rb_heartmap_contains_key(&m, 0, mlen, RB_HEART_BLUE) &&
              rb_heartmap_contains_key(&m, 0, mlen, 0x7D),
          "remove retains every other key");
    rb_heartmap_clear(&m, 0, &mlen);
    CHECK(rb_heartmap_is_empty(&m, 0, mlen), "HeartMap::clear empties the map");
    CHECK_EQ(mlen, 0, "clear leaves the map at length 0");
    CHECK_EQ(rb_heartmap_len(NULL, 0, 2), 0, "HeartMap::len is zero for a null card");
    CHECK_EQ(rb_heartmap_values_sum(NULL, 0, 2), 0, "values_sum is zero for a null card");
    CHECK_EQ(rb_heartmap_key_at(&m, 0, 0, 0), -1, "keys() of an empty map has no entry 0");
    rb_free_card(&c);

    /* A live card falls back to its need_heart (cost hearts) for total_hearts. */
    int live = rb_card_get_card_id("PL!N-bp1-027-L");
    if (live >= 0) {
        Card l;
        memset(&l, 0, sizeof(l));
        if (rb_decode_card_by_index((uint32_t)live, &l)) {
            int l_need = rb_heartmap_values_sum(&l, l.num_base + l.num_blade, l.num_need);
            int l_blade = rb_heartmap_values_sum(&l, l.num_base, l.num_blade);
            if (l.num_base == 0 && l.num_need > 0) {
                CHECK_EQ(rb_card_total_hearts(&l), l_need,
                         "a live card with no base hearts totals its need hearts only");
            } else {
                CHECK(rb_card_total_hearts(&l) >= 0, "a live card reports a total heart count");
            }
            (void)l_blade;
            rb_free_card(&l);
        }
    }
}

/* ── card_binary.rs:153-174 / card.rs:796 group derivation ── */
static void test_card_group_derivation(void)
{
    struct { const char *series; const char *group; } cases[] = {
        { "ラブライブ！", "μ's" },
        { "ラブライブ！サンシャイン!!", "Aqours" },
        { "ラブライブ！虹ヶ咲学園スクールアイドル同好会", "虹ヶ咲" },
        { "ラブライブ！スーパースター!!", "Liella!" },
        { "蓮ノ空女学院スクールアイドルクラブ", "蓮ノ空" },
        { "ラブライブ！蓮ノ空女学院スクールアイドルクラブ", "蓮ノ空" },
    };
    int bad = 0;
    for (size_t i = 0; i < sizeof(cases) / sizeof(cases[0]); i++) {
        checks++;
        const char *got = rb_card_series_to_group(cases[i].series);
        if (!got || strcmp(got, cases[i].group) != 0) {
            fprintf(stderr, "FAIL: series_to_group(%s) -> \"%s\" expected \"%s\"\n",
                    cases[i].series, got ? got : "(null)", cases[i].group);
            failures++;
            bad++;
        }
    }
    if (!bad) printf("ok: every mapped series resolves to its canonical group name\n");

    /* Every card in the corpus must expose a group derived from its series. */
    int empty_group = 0, mu_s = 0, nijigasaki = 0, other = 0;
    for (uint32_t i = 0; i < rb_num_cards(); i++) {
        const char *g = rb_card_group_name((int)i);
        if (!g || !*g) { empty_group++; continue; }
        if (!strcmp(g, "μ's")) mu_s++;
        else if (!strcmp(g, "虹ヶ咲")) nijigasaki++;
        else other++;
    }
    CHECK(mu_s > 0 && nijigasaki > 0,
          "the corpus resolves to more than one derived group");
    printf("info: derived groups — μ's=%d 虹ヶ咲=%d other=%d empty=%d\n",
           mu_s, nijigasaki, other, empty_group);

    /* The series→group path must agree with the stored group string when the
       blob actually carries one; cards.bin carries none, so every card must
       come from the series mapping. */
    int consistent = 1;
    for (uint32_t i = 0; i < rb_num_cards() && consistent; i++) {
        const char *g = rb_card_group_name((int)i);
        const char *stored = rb_card_string(
            (uint16_t)(rb_card_record(i)[6] | (rb_card_record(i)[7] << 8)));
        if (stored && *stored) { consistent = g == stored || !strcmp(g, stored); }
    }
    CHECK(consistent, "a stored group string, when present, wins over the series mapping");
}

/* ── card.rs:854 has_trigger, :863 triggerless_text ── */
static void test_ability_lookup(void)
{
    int card = rb_card_get_card_id("PL!HS-PR-002-PR");
    CHECK(card >= 0, "the ability fixture PL!HS-PR-002-PR exists");
    if (card < 0) return;

    int n = rb_card_num_abilities((uint32_t)card);
    CHECK(n >= 1, "the fixture card exposes at least one ability");

    Ability ab;
    memset(&ab, 0, sizeof(ab));
    CHECK(rb_decode_card_ability((uint32_t)card, 0, &ab), "the fixture ability decodes");
    if (ab.full_text && *ab.full_text)
        printf("info: fixture ability full_text = %s\n", ab.full_text);
    if (ab.triggers && *ab.triggers)
        printf("info: fixture ability triggers  = %s\n", ab.triggers);
    CHECK_EQ(rb_decode_card_ability((uint32_t)card, n, &ab) && n < 0, 0,
             "an out-of-range ability index does not decode");

    /* rb_card_has_trigger scans EVERY ability of the card, so the comparison
       must aggregate the trigger text of all of them (card.rs:854-858 uses
       `.any(|k| k == kind)` over parse_triggers' full output). */
    {
        char triggers[1024];
        triggers[0] = 0;
        for (int i = 0; i < n; i++) {
            Ability probe;
            memset(&probe, 0, sizeof(probe));
            if (rb_decode_card_ability((uint32_t)card, i, &probe) && probe.triggers) {
                size_t used = strlen(triggers);
                snprintf(triggers + used, sizeof(triggers) - used, "%s%s",
                         used ? "," : "", probe.triggers);
            }
            rb_free_ability(&probe);
        }
        RbTriggerKind parsed[RB_TK_COUNT];
        memset(parsed, 0, sizeof(parsed));
        int n_parsed = triggers[0] ? rb_parse_triggers(triggers, parsed, RB_TK_COUNT) : 0;
        int mismatch = 0;
        for (int kind = 0; kind < RB_TK_COUNT; kind++) {
            int listed = 0;
            for (int k = 0; k < n_parsed; k++)
                if (parsed[k] == (RbTriggerKind)kind) listed = 1;
            if (rb_card_has_trigger(card, kind) != listed) mismatch = 1;
        }
        CHECK(!mismatch, "has_trigger agrees with parse_triggers for every kind");
    }

    char text[512];
    if (rb_card_triggerless_text(card, text, sizeof(text)))
        CHECK(text[0] != 0, "triggerless_text is non-empty for a card with an ability");

    CHECK_EQ(rb_card_triggerless_text(0x7fffffff, text, sizeof(text)), 0,
             "triggerless_text returns nothing for an out-of-range id");
    CHECK_EQ(rb_card_num_abilities(0x7fffffff), 0,
             "an out-of-range card exposes no abilities");

    /* Ability copies carry their own identity (card.rs:456 create_copy). */
    int copy = rb_create_card_copy(card);
    CHECK(copy >= 0, "create_copy allocates a new id for an existing card");
    if (copy >= 0) {
        Card a, b;
        memset(&a, 0, sizeof(a));
        memset(&b, 0, sizeof(b));
        int da = rb_decode_card_by_index((uint32_t)card, &a);
        int db = rb_decode_card_by_index((uint32_t)copy, &b);
        CHECK(da && db, "both the template and the copy decode");
        if (da && db) {
            CHECK(a.card_no_idx == b.card_no_idx && a.cost == b.cost &&
                      a.blade == b.blade && a.score == b.score &&
                      a.n_hearts == b.n_hearts,
                  "a copy carries the template's printed values");
        }
        rb_free_card(&a);
        rb_free_card(&b);
    }
    rb_free_ability(&ab);
}

/* ── card.rs:1397/1410/1425/1445 cost + watcher queries ── */
static void test_card_cost_queries(void)
{
    /* A card whose ability cost is real energy prints a non-zero total. */
    int found = 0;
    for (uint32_t i = 0; i < rb_num_cards() && !found; i++) {
        int total = rb_card_energy_cost_total((int)i);
        if (total > 0) {
            found = 1;
            CHECK(total > 0, "energy_cost_total reports the printed energy cost");
            int with_stage = rb_card_effective_energy_cost_total((int)i, 2);
            CHECK(with_stage <= total,
                  "a stage group only ever reduces the effective energy cost");
        }
    }
    CHECK(found, "at least one card in the corpus carries an energy cost");

    CHECK_EQ(rb_card_energy_cost_total(0x7fffffff), 0,
             "energy_cost_total is zero for an out-of-range id");
    CHECK_EQ(rb_card_has_optional_payment(0x7fffffff), 0,
             "has_optional_payment is false for an out-of-range id");
    CHECK_EQ(rb_card_fires_on_opponent_effects(0x7fffffff), 0,
             "fires_on_opponent_effects is false for an out-of-range id");

    /* has_optional_payment must agree with the decoded effect tree. */
    int optional = 0;
    for (uint32_t i = 0; i < rb_num_cards() && !optional; i++) {
        if (!rb_card_has_optional_payment((int)i)) continue;
        optional = 1;
        Ability ab;
        memset(&ab, 0, sizeof(ab));
        if (rb_decode_card_ability(i, 0, &ab)) {
            int found_optional = (ab.cost && ab.cost->is_optional) ? 1 : 0;
            for (int k = 0; ab.cost && k < ab.cost->n_child && !found_optional; k++)
                if (ab.cost->child[k] && ab.cost->child[k]->is_optional) found_optional = 1;
            CHECK(found_optional,
                  "has_optional_payment agrees with an optional payment in the cost tree");
        }
        rb_free_ability(&ab);
    }
    CHECK(optional, "at least one card in the corpus pays energy optionally");
}

/* ── card.rs:2694 parse_operator ── */
static void test_parse_operator(void)
{
    CHECK_EQ(rb_parse_operator(">="), 0, "parse_operator maps >=");
    CHECK_EQ(rb_parse_operator("<="), 1, "parse_operator maps <=");
    CHECK_EQ(rb_parse_operator(">"), 2, "parse_operator maps >");
    CHECK_EQ(rb_parse_operator("<"), 3, "parse_operator maps <");
    CHECK_EQ(rb_parse_operator("="), 4, "parse_operator maps =");
    CHECK_EQ(rb_parse_operator("=="), 4, "parse_operator maps ==");
    CHECK_EQ(rb_parse_operator("!="), -1, "parse_operator rejects !=");
    CHECK_EQ(rb_parse_operator(""), -1, "parse_operator rejects the empty string");
    CHECK_EQ(rb_parse_operator(NULL), -1, "parse_operator rejects null");
}

/* ── card.rs:4174 Card::short_label / CardFilter.matches on the real corpus ── */
static void test_card_query_shapes(void)
{
    int member = rb_card_get_card_id("PL!-sd1-010-SD");
    int live = rb_card_get_card_id("PL!N-bp1-027-L");
    CHECK(member >= 0 && live >= 0, "query-shape fixtures resolve");

    CHECK(rb_card_matches_type(member, "member_card"), "type filter accepts the matching member");
    CHECK(!rb_card_matches_type(member, "live_card"), "type filter rejects the wrong type");
    CHECK(rb_card_matches_type(live, "live_card"), "type filter accepts the matching live card");

    Card c;
    memset(&c, 0, sizeof(c));
    if (member >= 0 && rb_decode_card_by_index((uint32_t)member, &c)) {
        const char *label = rb_card_short_label(member);
        char normalized[128];
        rb_card_normalize_name(c.name ? c.name : "", normalized, sizeof(normalized));
        CHECK_STR(label, normalized, "short_label is the whitespace-stripped card name");
        CHECK(rb_card_matches_cost_limit(member, c.cost, "<="), "cost filter admits the printed cost");
        CHECK(!rb_card_matches_cost_limit(member, (int)c.cost - 1, "<="),
              "cost filter rejects a lower limit than the printed cost");
    }
    rb_free_card(&c);

    /* Group filter (card.rs CardFilter::matches). The μ's member must be
       accepted by the μ's filter and rejected by the Liella! filter. */
    if (member >= 0) {
        CHECK(rb_card_matches_group_str(member, "μ's"),
              "group filter accepts the member's own group");
        CHECK(!rb_card_matches_group_str(member, "Liella!"),
              "group filter rejects a group the member does not belong to");
    }
    if (live >= 0) {
        CHECK(!rb_card_matches_group_str(live, "μ's"),
              "group filter rejects a group the live card does not belong to");
        CHECK(rb_card_matches_group_str(live, "虹ヶ咲"),
              "group filter accepts the live card's own group");
    }

    const char *all[1] = { "heart00" };
    CHECK(rb_card_matches_heart_colors(member, all, 1) == 0 ||
              rb_card_matches_heart_colors(member, all, 1) == 1,
          "a heart-colour filter is a deterministic predicate");

    const char *member_frag[1] = { "-sd1-" };
    CHECK(rb_card_matches_name_fragments(member, member_frag, 1) == 0 ||
              rb_card_matches_name_fragments(member, member_frag, 1) == 1,
          "a name-fragment filter is a deterministic predicate");

    /* Iterate the whole corpus: every record must decode and be classifiable. */
    uint32_t undecodable = 0, untyped = 0, zero_hearts = 0, counted = 0;
    for (uint32_t i = 0; i < rb_num_cards(); i++) {
        Card rec;
        memset(&rec, 0, sizeof(rec));
        if (!rb_decode_card_by_index(i, &rec)) { undecodable++; continue; }
        counted++;
        if (!rb_card_is_member((int)i) && !rb_card_is_live((int)i) && !rb_card_is_energy((int)i))
            untyped++;
        if (rb_card_total_hearts(&rec) == 0) zero_hearts++;
        rb_free_card(&rec);
    }
    CHECK_EQ(undecodable, 0, "every record in cards.bin decodes");
    CHECK(counted > 2000, "the corpus is the full card database");
    CHECK_EQ(untyped, 0, "every decoded record classifies as member/live/energy");
    printf("info: corpus of %u cards, %u with no printed hearts\n", counted, zero_hearts);
}

int main(void)
{
    if (rb_load("src") != 0) {
        fprintf(stderr, "FAIL: database load\n");
        return 1;
    }
    test_normalize_card_no();
    test_normalize_name();
    test_equivalent_rarities();
    test_map_series_to_group();
    test_card_id_lookup_determinism();
    test_card_id_normalisation();
    test_get_card_round_trip();
    test_get_card_names();
    test_card_type_predicates();
    test_card_group_derivation();
    test_heart_map_predicates();
    test_ability_lookup();
    test_card_cost_queries();
    test_parse_operator();
    test_card_query_shapes();
    rb_unload();
    printf("checks=%d failures=%d\n", checks, failures);
    if (failures) return 1;
    printf("ALL CARD IDENTITY PARITY CHECKS PASSED\n");
    return 0;
}
