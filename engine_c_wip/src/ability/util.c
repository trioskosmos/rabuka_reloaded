/* util.c — pure card/zone/comparison helpers.
   Mirror engine/src/ability/util.rs (compare_counts, card_matches_type,
   card_matches_group_str, card_at_position, pos_to_area, orientation_matches_state,
   zone_cards, CardFilter, constant_per_unit_units, compute_play_cost helpers,
   matching_ids, per_unit resolution, distinct filtering, selection primitives).
   These are shared by the condition evaluators and the effect executors; keeping
   them in one place mirrors the Rust single-source layout.

   rb_card_matches_group_str is a faithful port of util.rs:679-725: exact unit
   match (via the canonical rb_card_unit_name accessor), the spacing-insensitive
   and '！'(U+FF01)/'µ'(U+00B5) normalized fallbacks, the comparison against the
   card's DERIVED group (rb_card_group_name — the stored group string is "" for
   every record in cards.bin, so it must never be read directly),
   per-name containment over the '&'/'＆'-separated name list, series
   membership, then the constant set_card_identity memberships.

   WHY UNIT IS THE LOAD-BEARING TERM: cards.json has no `group` key on any of
   its 2,639 cards, but it does carry `unit` on 2,198 of them, and every name in
   abilities.json's group_names / exclude_group_names / counted_by lists is a
   UNIT name (CatChu!, KALEIDOSCORE, 5yncri5e!, Printemps, …) or one of the
   five series-derived names (μ's, Aqours, 虹ヶ咲, Liella!, 蓮ノ空). Both are
   therefore live: term 1 reads unit, term 2 reads the series-derived group. */

#include "rabuka.h"
#include <string.h>
#include <stdlib.h>
#include <stdio.h>
#include <ctype.h>

/* Heart-all wildcard key — mirrors util.rs HEART_ALL_KEY ("heart00"). */
#define RB_HEART_ALL_KEY "heart00"

/* ── Pending header fields ─────────────────────────────────────────────
   game_modifiers.rs:183  last_cost_energy_count: u8   still has no home in
   include/rabuka.h, so that path stays behind its switch:

      game_modifiers.rs:183  last_cost_energy_count: u8   in RbMods

   `card_property` used to be on this list too, guarded by
   RB_CARD_FILTER_CARD_PROPERTY with RF_CARD_PROPERTY falling back to "". That
   guard is GONE. include/rabuka.h:2438 now declares `char card_property[64]` in
   RbCardFilter unconditionally, so the macro could only ever expand one way:
   with the field present, `rf->card_property` is simply the field. A switch that
   has no off position is not a switch, and keeping one would mean the next
   reader cannot tell whether the matcher is live. card.c's producer lost the
   same guard for the same reason; the C port is now unconditional, which is what
   util.rs:1654-1673 and util.rs:1882 have always been. */
#ifdef RB_MODS_LAST_COST_ENERGY_COUNT
#define RB_LAST_COST_ENERGY(m) ((m)->last_cost_energy_count)
#else
#define RB_LAST_COST_ENERGY(m) (0)
#endif

/* Card::has_draw_icon (card.rs:4358), defined in src/core/card.c. Its three
   siblings — has_blade_heart / has_score_icon / has_all_blade — are declared in
   include/rabuka.h:1981-1984; this one is not, because the header is owned by
   another writer. Declared here so the fourth arm of rb_check_card_property can
   be reached; the header still wants the prototype. */
int rb_card_has_draw_icon(const Card *c);

/* ── Internal filter struct (mirrors CardFilter) ─────────────────────── */

/* See RbCardFilter in rabuka.h for the public version of this struct. */
/* The local copy below is used inside this TU; the public one in the header
   is used by external callers. They must stay in sync. */
typedef struct {
    char  card_type[32];
    char  group[64];
    int   has_group;
    int   cost_limit;
    char  cost_op[8];
    int   has_cost_limit;
    char  characters[256];
    int   has_characters;
    char  exclude_characters[256];
    int   has_exclude_characters;
    char  heart_colors[8][24];
    int   n_heart_colors;
    int   require_all_heart_colors;
    int   heart_color_count;
    int   need_heart_total;
    char  need_heart_operator[8];
    char  need_heart_color[24];
    int   has_need_heart_total;
    char  name_fragments[8][64];
    int   n_name_fragments;
    int   original_blade_limit;
    char  original_blade_op[8];
    int   has_original_blade;
    char  ability_filter[32];
    char  ability_filter_triggers[8][32];
    int   n_ability_filter_triggers;
    /* util.rs:1102 card_property — 64 to match rabuka.h's public member. */
    char  card_property[64];
    int   has_card_property;
    /* util.rs:1110 or_ability_filters: Vec<AbilityFilterBranch>, one JSON branch
       object per slot (see card.c's card_filter_or_branches). */
    char  or_ability_filters[4][192];
    int   n_or_ability_filters;
    int   negation;            /* Rust util.rs:1105 negation — negates card_property */
    int   exclude_self_id;
    int   has_exclude_self;
    int   cost_total;
    char  cost_total_op[8];
    int   has_cost_total;
    int   cost_values[16];
    int   n_cost_values;
    int   distinct;
    int   blade_limit;
    char  blade_op[8];
    int   has_blade_limit;
    int   has_filter;
    /* ── The six carriers include/rabuka.h:2450-2462 added and Rust's
     *    CardFilter reads (checklist H1). They used to reach RbCardFilter and
     *    STOP there: nothing in this TU copied them in and nothing tested them,
     *    so `local_filter_matches` and Rust's `CardFilter::matches` disagreed on
     *    every effect that set one. Rust twins, per field:
     *      original_heart_total (+ operator) util.rs:1356-1357, matched :1790-1798
     *      exclude_group_names                 util.rs:1531-1537 (in check_group)
     *      cost_limit_min / cost_limit_max     util.rs:1744-1752
     *      groups[]                            util.rs:1518-1529 (in check_group)
     *      exclude_cards[]                     util.rs:1489-1496 (check_exclude_cards)
     *      exclude_names[]                     util.rs:1498-1513 (check_exclude_names)
     *    `has_original_heart_total` is derived from the operator string, not from
     *    a header flag: Rust makes the operator unconditional whenever the total
     *    is present (util.rs:1907 `.or(Some("<="))`), so a producer can never
     *    supply one without the other and the string is a complete presence flag.
     *    exclude_cards / exclude_names are `None` in from_effect (util.rs:1908-1909)
     *    — they are set programmatically — so they carry has-flags of their own. */
    int   original_heart_total;
    char  original_heart_operator[8];
    int   has_original_heart_total;
    char  exclude_group_names[64];
    char  groups[8][64];
    int   n_groups;
    int   cost_limit_min;
    int   has_cost_limit_min;
    int   cost_limit_max;
    int   has_cost_limit_max;
    int   exclude_cards[RB_MAX_ZONE];
    int   n_exclude_cards;
    char  exclude_names[8][64];
    int   n_exclude_names;
} LocalCardFilter;

/* ── UTF-8 / normalization helpers ───────────────────────────────────── */

/* Normalize a UTF-8 copy: '！'(EF BC 81)→'!', 'µ'(C2 B5)→'μ'(CE BC).
   Caller must rb_free the result. Returns NULL on alloc failure. */
static char *norm_str(const char *s) {
    if (!s) return NULL;
    size_t n = strlen(s);
    char *out = rb_malloc(n + 1);
    if (!out) return NULL;
    size_t j = 0;
    for (size_t i = 0; i < n; ) {
        if ((unsigned char)s[i] == 0xEF && (unsigned char)s[i+1] == 0xBC && (unsigned char)s[i+2] == 0x81) {
            out[j++] = '!'; i += 3; continue; /* fullwidth ! */
        }
        if ((unsigned char)s[i] == 0xC2 && (unsigned char)s[i+1] == 0xB5) {
            out[j++] = (char)0xCE; out[j++] = (char)0xBC; i += 2; continue; /* µ → μ */
        }
        out[j++] = s[i++];
    }
    out[j] = '\0';
    return out;
}

/* ── TOMBSTONE (2026-10-04). `rb_norm_group_name` REMOVED.
 *
 * A census classified this DEAD-WRONG: "Rust normalises FULLWIDTH '!' (U+FF01)
 * and micro (U+00B5). A C norm_group_name that the group gate needs;
 * unreferenced, so the group gate cannot use it." The second sentence is FALSE
 * and the whole DEAD-WRONG verdict falls with it.
 *
 * This function was a one-line alias: `return norm_str(s);`. `norm_str`
 * (util.c:150) is LIVE with six call sites -- util.c:425, :494, :514, :541,
 * :3132, :3137 -- and util.c:494 is inside `rb_card_matches_group_str`
 * (util.c:491), which IS the group gate. So the U+FF01 / U+00B5 normalisation
 * the gate needs is implemented AND reached, today, on the live path.
 *
 * What was actually dead is the NAME, not the capability. Deleted so the tree
 * stops carrying a second spelling of norm_str that no one calls.
 *
 * Kept deliberately: `norm_str` itself, which does the byte work --
 *   EF BC 81 -> '!'        (U+FF01 FULLWIDTH EXCLAMATION MARK)
 *   C2 B5    -> CE BC      (U+00B5 MICRO SIGN -> U+03BC GREEK SMALL MU)
 * Both are spelled as explicit byte values, never as literal characters, so a
 * legacy-codepage rewrite of this file cannot silently turn the filter into a
 * no-op.
 *
 * Deadness proved by gcc -Wunused-function: "'rb_norm_group_name' defined but
 * not used". */

/* Strip ASCII whitespace, mirroring CardDatabase::normalize_name. */
static void rb_norm_ws(const char *s, char *out, size_t outsz) {
    size_t j = 0;
    if (!s) { if (outsz) out[0] = '\0'; return; }
    for (size_t i = 0; s[i]; i++) {
        char c = s[i];
        if (c == ' ' || c == '\t' || c == '\n' || c == '\r') continue;
        if (j + 1 < outsz) out[j++] = c;
    }
    out[j] = '\0';
}

/* case-insensitive equality (portable, no strcasecmp dependency). */
static int str_ieq(const char *a, const char *b) {
    if (!a || !b) return 0;
    while (*a && *b) {
        char ca = *a, cb = *b;
        if (ca >= 'A' && ca <= 'Z') ca += 32;
        if (cb >= 'A' && cb <= 'Z') cb += 32;
        if (ca != cb) return 0;
        a++; b++;
    }
    return *a == *b;
}

/* Rust `char::is_whitespace` is the Unicode White_Space property, NOT
   "ASCII whitespace". util.rs:696-706 filters on it for BOTH sides, so the
   previous byte-wise `if (c == ' ' || '\t' || '\n' || '\r')` strip silently
   diverged on U+3000 IDEOGRAPHIC SPACE, U+00A0 NBSP, U+2000-200A, U+2028/29,
   U+202F, U+205F and U+0085 — a query like 「lily　white」 (full-width space)
   matched in Rust and NOT here. Same set as core/card.c's card_whitespace(). */
static int cp_is_rust_whitespace(uint32_t cp) {
    return (cp >= 0x09 && cp <= 0x0D) || cp == 0x20 || cp == 0x85 || cp == 0xA0 ||
           cp == 0x1680 || (cp >= 0x2000 && cp <= 0x200A) ||
           cp == 0x2028 || cp == 0x2029 || cp == 0x202F || cp == 0x205F ||
           cp == 0x3000;
}

/* Decode one char and advance *p, mirroring Rust's str::chars(): a malformed
   lead byte yields the byte's own value and advances exactly one byte, so the
   comparison can never read past the terminator. */
static uint32_t utf8_next(const char **p) {
    const unsigned char *s = (const unsigned char *)*p;
    uint32_t cp = s[0];
    size_t n;
    if (cp < 0x80)            { *p += 1; return cp; }
    else if (cp < 0xC0)       { *p += 1; return 0xFFFD; }
    else if (cp < 0xE0)       { n = 2; cp &= 0x1F; }
    else if (cp < 0xF0)       { n = 3; cp &= 0x0F; }
    else if (cp < 0xF8)       { n = 4; cp &= 0x07; }
    else                      { *p += 1; return 0xFFFD; }
    for (size_t i = 1; i < n; i++) {
        if ((s[i] & 0xC0) != 0x80) { *p += 1; return 0xFFFD; }
        cp = (cp << 6) | (s[i] & 0x3F);
    }
    *p += n;
    return cp;
}

/* Mirror util.rs::eq_ignoring_whitespace (util.rs:696-706).
   The corpus spells one unit group two ways — `lily white` on 25 cards and
   `lilywhite` on 106 — and cards.json `unit` keeps both, so a raw `==` made a
   `『lily white』` filter skip every unspaced card and a `『lilywhite』` filter
   skip every spaced one. Rust added this term explicitly for that reason; the C
   had no equivalent, so both spellings were half-dead here. Allocation free;
   the ！/µ variants are handled separately by norm_str. */
static int str_eq_ignoring_ws(const char *a, const char *b) {
    if (!a || !b) return 0;
    const char *pa = a, *pb = b;
    for (;;) {
        uint32_t ca = 0, cb = 0;
        int ea = 1, eb = 1;
        while (*pa) { ca = utf8_next(&pa); if (!cp_is_rust_whitespace(ca)) { ea = 0; break; } }
        while (*pb) { cb = utf8_next(&pb); if (!cp_is_rust_whitespace(cb)) { eb = 0; break; } }
        if (ea || eb) return ea && eb;
        if (ca != cb) return 0;
    }
}

/* ── Effect-field reader ─────────────────────────────────────────────── */

/* Mirror the Rust typed getters (operation_any, location_any, heart_type_any,
   original_count_any, original_operator_any, …) mapped to extra_k/extra_v. */
static const char *eff_extra(const AbilityEffect *e, const char *k) {
    if (!e) return NULL;
    for (int i = 0; i < e->n_extra; i++)
        if (e->extra_k[i] && !strcmp(e->extra_k[i], k)) return e->extra_v[i];
    return NULL;
}

/* ── Comparison / threshold ──────────────────────────────────────────── */

/* Mirror util::compare_counts. operator NULL defaults to ">=". */
int rb_compare_counts(const char *operator, int actual, int expected) {
    const char *op = operator ? operator : ">=";
    if (!strcmp(op, ">=")) return actual >= expected;
    if (!strcmp(op, ">"))  return actual >  expected;
    if (!strcmp(op, "<=")) return actual <= expected;
    if (!strcmp(op, "<"))  return actual <  expected;
    if (!strcmp(op, "==") || !strcmp(op, "="))  return actual == expected;
    if (!strcmp(op, "!=")) return actual != expected;
    return 1;
}

/* Mirror util.rs::cost_threshold_met — original_count/operator gate on the
   card's cost (used by cost-reduction / play-cost auras). */
int rb_cost_threshold_met(const Card *card, const AbilityEffect *e) {
    const char *th = eff_extra(e, "original_count");
    if (!th || !*th) return 1;
    int threshold = atoi(th);
    int cost = card ? card->cost : 0;
    const char *op = eff_extra(e, "original_operator");
    int met = 1;
    if (op) {
        if      (!strcmp(op, ">=")) met = cost >= threshold;
        else if (!strcmp(op, "<=")) met = cost <= threshold;
        else if (!strcmp(op, ">"))  met = cost >  threshold;
        else if (!strcmp(op, "<"))  met = cost <  threshold;
        else if (!strcmp(op, "==")) met = cost == threshold;
        else if (!strcmp(op, "!=")) met = cost != threshold;
        else met = 1;
    } else {
        met = (cost == threshold);
    }
    return met;
}

/* ── Card property predicates ────────────────────────────────────────── */

/* Mirror card.rs::has_blade_heart / has_score_icon / has_all_blade. */
int rb_card_has_blade_heart(const Card *c) {
    if (!c) return 0;
    if (c->num_blade > 0) return 1;
    if (c->has_special && c->special_count > 0) return 1;
    return 0;
}
int rb_card_has_blade_heart_strict(const Card *c) {
    return c && c->num_blade > 0;
}
int rb_card_has_score_icon(const Card *c) {
    return c && c->has_special && c->special_color == (uint8_t)RB_HEART_SCORE;
}
int rb_card_has_all_blade(const Card *c) {
    if (!c) return 0;
    int base = c->num_base;
    int end = base + c->num_blade;
    if (end > c->n_hearts) end = c->n_hearts;
    for (int h = base; h < end; h++)
        if (c->heart_color[h] == (uint8_t)RB_HEART_ALL) return 1;
    return 0;
}

/* ── Card-type predicate ──────────────────────────────────────────────── */

/* Rust `str::trim` over `char::is_whitespace` — util.rs:595 does
   `card_type_filter.map(str::trim)` BEFORE the match, so the EMPTY arm below is
   reached by a whitespace-only filter and not only by a genuinely empty one.
   Same predicate as cp_is_rust_whitespace (:226), so U+3000 / U+00A0 /
   U+2000-200A / U+2028/29 / U+202F / U+205F / U+0085 trim here exactly as
   there. Returns a NUL-terminated copy; caller frees. */
static char *rs_str_trim_dup(const char *s) {
    if (!s) return NULL;
    const char *p = s;
    while (*p) {
        const char *save = p;
        if (cp_is_rust_whitespace(utf8_next(&p))) { s = save; continue; }
        break;
    }
    const char *end = s + strlen(s);
    while (end > s) {
        /* Step back over the last UTF-8 sequence and test its code point. */
        const char *seq = end - 1;
        while (seq > s && ((unsigned char)*seq & 0xC0) == 0x80) seq--;
        const char *probe = seq;
        uint32_t cp = utf8_next(&probe);
        if (probe != end || !cp_is_rust_whitespace(cp)) break;
        end = seq;
    }
    size_t n = (size_t)(end - s);
    char *out = (char *)malloc(n + 1);
    if (!out) return NULL;
    memcpy(out, s, n);
    out[n] = '\0';
    return out;
}

/* Mirror util.rs::card_matches_type_for (util.rs:585-621). card_id is a
 * card_no index.
 *
 * Three defects this replaces, all measured — the arms are transcribed from the
 * Rust, not paraphrased:
 *
 *  1. THE `member` ALIAS WAS MISSING. util.rs:598 spells the member arm
 *     `member_card` OR `member`; the C had only `member_card`, so `member` fell
 *     through. Rust's own comment (util.rs:589-594) records the observed
 *     spellings — "member", "Member" and "" — and util.rs:613-616 records the
 *     consequence: PL!N-bp7-028-L 「自分のステージにいるすべてのグループのメンバーは
 *     heart01を得る」 arrived as `card_type: "member"` and applied to anything on
 *     the stage.
 *  2. THE COMPARISON WAS CASE-SENSITIVE. Every arm is `eq_ignore_ascii_case`;
 *     the C used `strcmp`, so "Member"/"MEMBER_CARD"/"Live_Card" were all
 *     fall-throughs.
 *  3. IT FAILED OPEN. util.rs:607-619 is `Some(other) => false` with the comment
 *     "Fail CLOSED, and say so" — "I do not know what this means" is not "it
 *     matches". The C's fall-through was `return 1`, i.e. a filter that should
 *     REJECT a card ACCEPTED it: a wrong-value bug, not a missing feature.
 *     This one is VERIFIED-NOOP: 0 on today's corpus — `card_type` filters there
 *     are exactly `member_card`, `live_card` and `energy_card` — and it is still
 *     wrong the moment new data ships, which is why it is fixed here rather than
 *     closed as unreachable.
 *
 * `str_ieq` (:208) is the file's existing ASCII-case-insensitive equality, i.e.
 * Rust's `eq_ignore_ascii_case` without the locale dependence of strcasecmp.
 * Asserted by tests/test_parity_resolver.c::card_matches_type_covers_every_arm
 * and ::known_divergences_in_the_cards_matcher. */
int rb_card_matches_type(int card_id, const char *filter) {
    if (!filter) return 1;                      /* Rust: None => true */
    char *owned = rs_str_trim_dup(filter);
    const char *f = owned ? owned : filter;
    int is_live   = rb_card_is_live(card_id);
    int is_energy = rb_card_is_energy(card_id);
    int is_member = !is_live && !is_energy;
    int r;
    if (str_ieq(f, "live_card"))                     r = is_live;
    else if (str_ieq(f, "member_card") ||
             str_ieq(f, "member"))                   r = is_member;
    else if (str_ieq(f, "energy_card"))               r = is_energy;
    else if (str_ieq(f, "card") || !*f)              r = 1;
    else                                             r = 0;  /* fail CLOSED */
    free(owned);
    return r;
}

/* ── Orientation predicate ────────────────────────────────────────────── */

/* Mirror util::orientation_matches_state. */
int rb_orientation_matches_state(const char *orientation, const char *state) {
    if (!orientation) return state && !strcmp(state, "active");
    return !strcmp(orientation, state);
}

/* ── Group matching ───────────────────────────────────────────────────── */

/* Mirror util.rs::card_series_matches_group. */
static int rb_card_series_matches_group(const char *series, const char *group) {
    if (!series || !group) return 0;
    if (!strcmp(group, "μ's")) {
        const char *p = series;
        while (*p) {
            const char *nl = strchr(p, '\n');
            size_t len = nl ? (size_t)(nl - p) : strlen(p);
            char line[1024];
            if (len >= sizeof(line)) len = sizeof(line) - 1;
            memcpy(line, p, len);
            line[len] = '\0';
            if (strstr(line, "ラブライブ！") &&
                !strstr(line, "サンシャイン") &&
                !strstr(line, "虹ヶ咲") &&
                !strstr(line, "スーパースター") &&
                !strstr(line, "蓮ノ空"))
                return 1;
            if (!nl) break;
            p = nl + 1;
        }
        return 0;
    }
    if (!strcmp(group, "Aqours"))   return strstr(series, "サンシャイン") != NULL;
    if (!strcmp(group, "虹ヶ咲"))  return strstr(series, "虹ヶ咲") != NULL;
    if (!strcmp(group, "Liella!")) return strstr(series, "スーパースター") != NULL;
    if (!strcmp(group, "蓮ノ空"))  return strstr(series, "蓮ノ空") != NULL;
    return 0;
}

/* ── TOMBSTONE (2026-10-04). `rb_debug_group_match` REMOVED.
 *
 * A census classified this DEAD-WRONG: "the C body is empty; Rust
 * util.rs:652 debug_group_match performs unit/group/name/series checks under
 * RABUKA_DEBUG_GROUP=1 + trace level; the C comment calls it a 'Mirror' and
 * then says 'no-op in the C port'; the word Mirror is false." That much is
 * right, but the prescribed reading -- "implement the checker" -- is not, and
 * implementing it would have been wrong three ways:
 *
 *  (1) RUST HAS TWO OF THESE, AND ONE IS ALSO EMPTY. util.rs:642-649 is
 *      `#[cfg(feature = "no_std")] fn debug_group_match(...) {}` -- an empty
 *      body, with the same signature. Only util.rs:651-651's
 *      `#[cfg(not(feature = "no_std"))]` twin has the real body. So "the Rust
 *      twin has a real checker" is true of exactly one of the two definitions
 *      that share the name.
 *  (2) The real Rust body is PURELY DIAGNOSTIC. It computes four booleans,
 *      formats them into a debug string, and RETURNS NOTHING. It does not
 *      mutate state and does not influence `result` -- `result` is an input it
 *      only prints. There is no parity-visible behaviour for a C port to have
 *      or lack, so it cannot be a DEAD-WRONG engine function at all.
 *  (3) It is gated on `log::log_enabled!(Level::Trace)` AND an environment
 *      variable (`RABUKA_DEBUG_GROUP=1`). This C tree has neither a trace log
 *      level nor an env-var reader in that path, so a port would be code that
 *      can never fire in any build that exists here.
 *
 * The real defect was the false word "Mirror" in the comment: it read as
 * "implemented, deliberately" when it was "not implemented, and Rust's real
 * version needs facilities this port does not have". Deleting the empty body
 * removes the lie; the reasoning above is the replacement record.
 *
 * Deadness proved by gcc -Wunused-function: "'rb_debug_group_match' defined but
 * not used". */

/* Mirror the constant set_card_identity term of util.rs:628-635: a card whose
   ROOT ability effect has action "set_card_identity" carries every entry of its
   `identities` list as an extra group membership. The C decoder flattens a
   string array into a CSV extra, so the list is split back out here; each entry
   is compared exactly, with the same ！/µ normalized fallback the unit / name
   terms use. Runtime identities recorded by rb_set_card_identity (a C-only
   registry) are OR-ed in by the caller. */
static int card_identity_ability_matches(int card_id, const char *gn) {
    int n_abilities = rb_card_num_abilities((uint32_t)card_id);
    for (int i = 0; i < n_abilities; i++) {
        Ability ab;
        if (!rb_decode_card_ability((uint32_t)card_id, i, &ab)) continue;
        int match = 0;
        const AbilityEffect *eff = ab.effect;
        if (eff && eff->action && !strcmp(eff->action, "set_card_identity")) {
            const char *ids = NULL;
            for (int k = 0; k < eff->n_extra; k++)
                if (eff->extra_k[k] && !strcmp(eff->extra_k[k], "identities") && eff->extra_v[k]) {
                    ids = eff->extra_v[k];
                    break;
                }
            for (const char *cur = ids; cur && *cur && !match; ) {
                const char *comma = strchr(cur, ',');
                size_t len = comma ? (size_t)(comma - cur) : strlen(cur);
                char tok[64];
                if (len >= sizeof tok) len = sizeof tok - 1;
                memcpy(tok, cur, len);
                tok[len] = '\0';
                /* The CSV writer quotes an entry only when it contains a
                   separator, so strip one layer of quotes. */
                size_t tl = strlen(tok);
                if (tl >= 2 && tok[0] == '"' && tok[tl - 1] == '"') {
                    memmove(tok, tok + 1, tl - 2);
                    tok[tl - 2] = '\0';
                }
                if (tok[0]) {
                    if (!strcmp(tok, gn)) {
                        match = 1;
                    } else if (strstr(tok, "！") || strstr(tok, "µ")) {
                        char *tn = norm_str(tok);
                        if (tn) { if (!strcmp(tn, gn)) match = 1; rb_free(tn); }
                    }
                }
                if (!comma) break;
                cur = comma + 1;
            }
        }
        rb_free_ability(&ab);
        if (match) return 1;
    }
    return 0;
}

/* Mirror util.rs::card_matches_group_str — group/unit/name/series + set_card_identity.

   Faithful to util.rs:679-725. The previous body tested raw bidirectional
   substrings (strstr(group_name, g) with an empty card group is always
   non-NULL), so EVERY card matched EVERY group filter and every 「『X』の…」
   ability in the engine silently did nothing.

   TERM ORDER (each term is Rust's, in Rust's order):
     1. unit        — util.rs:691-696. Exact against the NORMALISED query, then
                       whitespace-insensitive, then the ！/µ-normalised form.
                       This is the term the 「『CatChu!』」 style filters live on:
                       cards.json carries `unit` on 2,198 cards and it is the
                       only card field the data actually populates (the `group`
                       key is absent from all 2,639).
     2. group       — util.rs:698-699. rb_card_group_name() is the DERIVED
                       (series→group) accessor from core/card.c, because the
                       stored `group` index resolves to "" for every record.
     3. names       — util.rs:700-701, containment over the '&'/U+FF06 split.
     4. series      — util.rs:705.
     5. identities  — util.rs:710-717, the constant set_card_identity list.

   NORMALISATION: norm_str maps U+FF01 '！' → ASCII '!' and U+00B5 'µ' →
   U+03BC 'μ', and is applied to the QUERY unconditionally. The card side is
   only re-normalised when it actually contains one of those two characters,
   which is what Rust does (util.rs:696) and is what makes the corpus's split
   spellings work: `unit` holds BOTH 'みらくらぱーく！' (51 cards, U+FF01) and
   'みらくらぱーく!' (45 cards, ASCII), while group_names holds only the U+FF01
   form — either one matches either filter. U+FF0B '＋' is deliberately NOT
   folded: Rust's norm_group_name folds only U+FF01 and U+00B5, and no unit or
   group_names string in cards.json / abilities.json contains it, so folding it
   would only make the C diverge from the reference.

   EXHAUSTIVE DIFF (2026-10-02). Every term above was re-derived from Rust and
   compared against this function over the WHOLE corpus: all 2,639 cards x 45
   group strings (the 20 distinct `group_names`/`exclude_group_names` values
   abilities.json can deliver, their whole-CSV joins, all 5 KNOWN_GROUPS, every
   distinct `unit` value in cards.json, and whitespace/width variants) =
   118,755 (card, group) pairs. Result: OR-level verdict, the per-term vector,
   and the '&'/U+FF06 split part count agree with Rust on EVERY pair except the
   131 pairs whose query contains a non-ASCII space, which str_eq_ignoring_ws
   used to miss (now fixed — see its comment). In particular the report that
   this matcher is "too permissive for 虹ヶ咲" is NOT reproducible: for the
   虹ヶ咲 query there is not one disagreement, in either direction, over all
   2,639 cards. The two extra C-only terms are both faithful: the CSV-split
   constant `identities` list reproduces Rust's real `ids` array (the corpus
   carries exactly one set_card_identity ability, on PL!HS-bp2-020-L /
   PL!HS-bp5-018-L / PL!HS-sd1-020-SD), and rb_card_matches_identity_str reads
   only the C-only runtime registry, which no group filter exercises at load.
   `rb_card_series_to_group` and the literals here carry the corpus's own
   codepoints — 虹ヶ咲 is U+8679 U+30F6 U+54B2 (KATAKANA small ke) and 蓮ノ空
   is U+84EE U+30CE U+7A7A; the hiragana U+30F1 / U+83B2 spellings match
   NOTHING and silently disable both terms. */
int rb_card_matches_group_str(int card_id, const char *group_name) {
    if (!group_name) return 1;

    char *gn = norm_str(group_name);
    if (!gn) return 0;

    Card c;
    if (!rb_decode_card_by_index((uint32_t)card_id, &c)) { rb_free(gn); return 0; }

    /* Canonical unit accessor — never Card.unit_idx + rb_card_string, which
       cannot tell "no unit" from "index 0xFFFF". */
    const char *u = rb_card_unit_name(card_id);
    const char *s = rb_card_string(c.series_idx);

    /* util.rs:691-696 — unit == gn || eq_ignoring_whitespace(unit, gn)
       || (unit has ！/µ && norm(unit) == gn). */
    int match = 0;
    if (u && *u) {
        if (!strcmp(u, gn)) {
            match = 1;
        } else if (str_eq_ignoring_ws(u, gn)) {
            match = 1;
        } else if (strstr(u, "！") || strstr(u, "µ")) {
            char *unorm = norm_str(u);
            if (unorm) { match = !strcmp(unorm, gn); rb_free(unorm); }
        }
    }

    /* util.rs:698-699 — c.group == g, then the same whitespace-insensitive
       form. The group must come from rb_card_group_name(): the stored group
       index resolves to "" for every record in cards.bin, so reading it
       directly makes this term dead (and made the old body match everything).
       Compared against the RAW query, as Rust does. */
    if (!match) {
        const char *g = rb_card_group_name(card_id);
        if (g && *g && (!strcmp(g, group_name) || str_eq_ignoring_ws(g, group_name)))
            match = 1;
    }

    /* util.rs:700-701 — any of the card's names contains gn, with the same
       ！/µ normalized fallback. Names are '&'/U+FF06 separated. */
    if (!match) {
        char names[1024];
        names[0] = '\0';
        int n_names = rb_card_get_card_names(card_id, names, sizeof names);
        const char *p = names;
        for (int i = 0; i < n_names && p && *p && !match; i++) {
            if (strstr(p, gn)) {
                match = 1;
            } else if (strstr(p, "！") || strstr(p, "µ")) {
                char *nn = norm_str(p);
                if (nn) { if (strstr(nn, gn)) match = 1; rb_free(nn); }
            }
            p += strlen(p) + 1;
        }
    }

    /* util.rs:705 — multi-series joint cards match via any constituent line. */
    if (!match && s && rb_card_series_matches_group(s, gn)) match = 1;

    /* util.rs:710-717 — constant set_card_identity ("treated as") memberships:
       the ability-declared list first, then the runtime registry the C port
       records when the ability has actually fired. */
    if (!match) match = card_identity_ability_matches(card_id, gn);
    if (!match) match = rb_card_matches_identity_str(card_id, group_name);

    rb_free_card(&c);
    rb_free(gn);
    return match;
}

/* Mirror util.rs::card_matches_any_group — pass if groups is empty or card
   matches ANY entry. */
int rb_card_matches_any_group(int card_id, const char **groups, int n) {
    if (!groups || n <= 0) return 1;
    for (int i = 0; i < n; i++)
        if (groups[i] && rb_card_matches_group_str(card_id, groups[i])) return 1;
    return 0;
}

/* ── group_names: ONE entry, never the whole list ───────────────────────── */

/* Rust models `group_names` as a `Vec<String>` (core/card.rs:2007
   `group_names_any`) but reads exactly ONE entry everywhere the engine filters
   cards by group:
     core/card.rs:2397   `AbilityEffect::filter_subset` -> `group: self.group_name()`
     core/card.rs:2462   `group_name()` = `group_names.first()`
     core/card.rs:3860   `Condition::filter_subset`   -> `group_names.first()`
     util.rs:197         `play_cost_reduction_matches`-> `gn.first()`
     util.rs:237         `per_unit_cost_reduction`    -> `effect.group_name()`
   The C bytecode decoder flattens the array into ONE comma-joined extra, so
   every one of those sites handed `rb_card_matches_group_str` a string like
   "lilywhite,μ's" / "CatChu!,KALEIDOSCORE,5yncri5e!" / "DiverDiva,虹ヶ咲" and it
   compared that WHOLE string as ONE exact token against the card's unit /
   derived group / name / series. No card is *called* any of those values, so the
   filter admitted nothing.
   MEASURED over all 2,639 cards with the C database: group "lilywhite,μ's"
   (PL!-pb1-007-P＋ / -R, both a modify_cost and a sequential) matched 0 cards;
   the corpus carries 3 such shapes — ("Aqours","SaintSnow") x10,
   ("CatChu!","KALEIDOSCORE","5yncri5e!") x3, ("lilywhite","μ's") x2,
   ("SunnyPassion","Liella!") x2 — so 24 decoded effects were affected.

   This is deliberately NOT rb_card_matches_any_group: `any_group` is a
   DIFFERENT Rust function (util.rs:799) and is only correct where the Rust
   source spells it out (choice.c / move.c already port that one). Every site
   routed through here corresponds to a `.first()`.

   NOT static: cost.c / score.c / misc.c / ability_effects.c each carry a local
   twin of this trio because the helpers were file-private and undeclared.
   Declare all three in include/rabuka.h (signatures below) and delete the
   twins; a local twin is a second place for `.first()` to go wrong.

   NOTE resolver.c:31 forward-declares
       const char *rb_effect_group_name(const AbilityEffect *e);
   which is NOT this signature (it drops the `buf`/`bufsz` scratch pair). It is
   currently never called there, so it links but is a wrong-type declaration
   waiting to be used; it must be deleted, not satisfied. */
const char *rb_group_name_first(const char *group_names, char *buf, size_t bufsz) {
    if (!group_names || !*group_names) return NULL;
    const char *comma = strchr(group_names, ',');
    size_t len = comma ? (size_t)(comma - group_names) : strlen(group_names);
    if (!buf || bufsz == 0) return group_names;
    if (len >= bufsz) len = bufsz - 1;
    memcpy(buf, group_names, len);
    buf[len] = '\0';
    return buf;
}

/* Read the effect's group filter the way Rust does: `group_names` is the wire
   key (there is NO `group` extra anywhere in the card database — measured: 802
   `group_names`, 12 `exclude_group_names`, 17 `group_reference`, 3
   `per_group`, 3 `per_group_count`, and not one `group`), and only its first
   entry is the filter. Kept as a named helper so the two facts cannot drift. */
const char *rb_effect_group_name(const AbilityEffect *e, char *buf, size_t bufsz) {
    if (!e) return NULL;
    const char *names = eff_extra(e, "group_names");
    if (!names || !*names) names = eff_extra(e, "group");
    return rb_group_name_first(names, buf, bufsz);
}

/* Pass if the effect names no group, else match `group_names.first()`.
   `buf` is caller-owned scratch so the stage / hand / discard scan loops stay
   allocation free. */
int rb_card_matches_effect_group(const AbilityEffect *e, int card_id,
                                 char *buf, size_t bufsz) {
    const char *g = rb_effect_group_name(e, buf, bufsz);
    if (!g || !*g) return 1;
    return rb_card_matches_group_str(card_id, g);
}


/* ── Card predicate helpers ───────────────────────────────────────────── */

/* Mirror util.rs::card_matches_characters (util.rs:789-801) — the query name is
   whitespace-normalized and must be CONTAINED IN one of the card's '&'-separated
   names. The C port used to strstr the card's raw unsplit name, which also
   matched a query spanning the separator (「A&B」 matched a joint card 「A&B&C」). */
int rb_card_matches_characters(int card_id, const char **names, int n) {
    if (n <= 0) return 1;
    char card_names[1024];
    card_names[0] = '\0';
    int n_card_names = rb_card_get_card_names(card_id, card_names, sizeof card_names);
    int r = 0;
    const char *p = card_names;
    for (int i = 0; i < n_card_names && p && *p && !r; i++, p += strlen(p) + 1) {
        for (int k = 0; k < n; k++) {
            if (!names[k]) continue;
            char norm_name[256];
            rb_card_normalize_name(names[k], norm_name, sizeof norm_name);
            if (strstr(p, norm_name)) { r = 1; break; }
        }
    }
    return r;
}

/* Mirror util.rs::card_matches_cost_limit_op_for (util.rs:1095-1116) — the
 * compared quantity is `cost.or(score)` and the operator table is Rust's single
 * canonical one, `compare_with_operator` (util.rs:2301-2311):
 *
 *      ">="            => actual >= expected
 *      ">"             => actual >  expected
 *      "<="            => actual <= expected
 *      "<"             => actual <  expected
 *      "=" | "==" | "exact" => actual == expected
 *      "!="            => actual != expected
 *      _               => None
 *
 * with two caller-side fallbacks in the same order the Rust chains them
 * (util.rs:1107-1113): `None` plus `comparison == Some("min")` reads as `>=`
 * ("a card-text-only spelling of >= that the canonical table does not carry"),
 * and a still-`None` operator keeps the `<=` reading — "a cost cap defaults to
 * a ceiling".
 *
 * THE MISSING ARM, and it is the REACHABLE one. The table above carries `"=="`
 * and the C's did not: it had `exact` and `=` only. `cost_limit_operator ==
 * "=="` appears on exactly ONE condition node in the whole corpus —
 * PL!S-PR-045-PR (card id 1591), 「コスト7のメンバーからバトンタッチして登場した場合」,
 * with `cost_limit: 7, cost_limit_operator: "=="`. Rust asks for a
 * cost-EXACTLY-7 member and the C asked for a cost-AT-MOST-7 one, so a cheaper
 * member's baton touch satisfied the debut condition in C and not in Rust. The
 * other three spellings carried by the 40 corpus nodes that have a cost_limit
 * key (`>=`, `<=`, `=`) were already in the C.
 *
 * `"!="` is landed in the same edit because it is the same table: the Rust list
 * above has six arms and the C had four, and leaving `!=` out would keep the
 * table short by one arm for the next caller. No corpus node carries it.
 *
 * KNOWN DIVERGENCE, NOT FIXED HERE — the QUANTITY, not the operator. Rust
 * branches on the Option (`c.cost.or(c.score)`, so a card with neither is
 * `None` and the whole predicate is FALSE); the C branches on card TYPE and
 * reads a zeroed cost, so a card with neither passes any non-negative limit.
 * That is a separate change with its own reachability question and it is
 * asserted red on purpose by
 * tests/test_parity_resolver.c::known_divergences_in_the_cards_matcher row (c).
 */
int rb_card_matches_cost_limit(int card_id, int cost_limit, const char *comparison) {
    Card c;
    if (!rb_decode_card_by_index((uint32_t)card_id, &c)) return 0;
    int value = rb_card_is_live(card_id) ? (int)c.score : (int)c.cost;
    int r;
    if (!comparison || !*comparison)            r = value <= cost_limit;
    else if (!strcmp(comparison, ">="))         r = value >= cost_limit;
    else if (!strcmp(comparison, ">"))          r = value >  cost_limit;
    else if (!strcmp(comparison, "<="))         r = value <= cost_limit;
    else if (!strcmp(comparison, "<"))          r = value <  cost_limit;
    else if (!strcmp(comparison, "exact") ||
             !strcmp(comparison, "=")     ||
             !strcmp(comparison, "=="))        r = value == cost_limit;
    else if (!strcmp(comparison, "!="))         r = value != cost_limit;
    else if (!strcmp(comparison, "min"))        r = value >= cost_limit;
    else                                        r = value <= cost_limit;
    rb_free_card(&c);
    return r;
}
int rb_card_matches_cost_limit_op(int card_id, int cost_limit, const char *comparison) {
    return rb_card_matches_cost_limit(card_id, cost_limit, comparison);
}

static int card_has_heart_in_range(const Card *c, int hc, int start, int end) {
    for (int i = start; i < end && i < c->n_hearts; i++)
        if ((int)c->heart_color[i] == hc) return 1;
    return 0;
}

/* Sum of the printed heart COUNTS of colour `hc` over [start, end) — mirrors
   HeartMap::get(&hc).unwrap_or(&0). Pass hc < 0 to sum every colour
   (HeartMap::values_sum). */
static int heart_count_in_range(const Card *c, int hc, int start, int end) {
    int total = 0;
    for (int i = start; i < end && i < c->n_hearts; i++)
        if (hc < 0 || (int)c->heart_color[i] == hc) total += c->heart_count[i];
    return total;
}

/* Mirror Card::total_hearts (card.rs:4453-4461): base_heart.values_sum() when the
   card prints base hearts (a member), else need_heart.values_sum() over the need
   slice (a live card's cost hearts). Blade hearts are never counted. */
static int card_total_hearts(const Card *c) {
    if (c->num_base > 0) return heart_count_in_range(c, -1, 0, c->num_base);
    return heart_count_in_range(c, -1, c->num_base + c->num_blade,
                                c->num_base + c->num_blade + c->num_need);
}

/* Mirror util.rs::heart_count_for_threshold (util.rs:1111-1120): whichever of the
   printed base heart and the need heart holds more hearts of `color` counts. */
static int heart_count_for_threshold(const Card *c, int hc) {
    int base = heart_count_in_range(c, hc, 0, c->num_base);
    int need = heart_count_in_range(c, hc, c->num_base + c->num_blade,
                                    c->num_base + c->num_blade + c->num_need);
    return base > need ? base : need;
}

/* Mirror util.rs::card_matches_heart_colors — OR logic over the listed colors. */
int rb_card_matches_heart_colors(int card_id, const char **heart_colors, int n) {
    if (n <= 0) return 1;
    Card c;
    /* util.rs:845 — a card the database does not know PASSES (is_none_or). */
    if (!rb_decode_card_by_index((uint32_t)card_id, &c)) return 1;
    int r = 0;
    for (int k = 0; k < n; k++) {
        int hc = (int)rb_parse_heart_color(heart_colors[k]);
        int found = (c.num_base > 0)
            ? card_has_heart_in_range(&c, hc, 0, c.num_base)
            : card_has_heart_in_range(&c, hc, c.num_base + c.num_blade,
                                      c.num_base + c.num_blade + c.num_need);
        if (found) { r = 1; break; }
    }
    rb_free_card(&c);
    return r;
}

/* Mirror util.rs::card_matches_all_heart_colors — AND logic over the listed colors. */
int rb_card_matches_all_heart_colors(int card_id, const char **heart_colors, int n) {
    if (n <= 0) return 1;
    Card c;
    /* util.rs:869 — same is_none_or pass for an unknown card. */
    if (!rb_decode_card_by_index((uint32_t)card_id, &c)) return 1;
    int r = 1;
    for (int k = 0; k < n; k++) {
        int hc = (int)rb_parse_heart_color(heart_colors[k]);
        int found = (c.num_base > 0)
            ? card_has_heart_in_range(&c, hc, 0, c.num_base)
            : card_has_heart_in_range(&c, hc, c.num_base + c.num_blade,
                                      c.num_base + c.num_blade + c.num_need);
        if (!found) { r = 0; break; }
    }
    rb_free_card(&c);
    return r;
}

/* Mirror util.rs::card_matches_name_fragments — name must contain every fragment. */
int rb_card_matches_name_fragments(int card_id, const char **fragments, int n) {
    if (n <= 0) return 1;
    Card c;
    if (!rb_decode_card_by_index((uint32_t)card_id, &c)) return 0;
    int r = 1;
    if (c.name) {
        char norm_card[512];
        rb_card_normalize_name(c.name, norm_card, sizeof norm_card);
        for (int k = 0; k < n; k++) {
            if (!fragments[k]) continue;
            char norm_frag[256];
            rb_card_normalize_name(fragments[k], norm_frag, sizeof norm_frag);
            if (!strstr(norm_card, norm_frag)) { r = 0; break; }
        }
    } else {
        r = 0;
    }
    rb_free_card(&c);
    return r;
}

/* Mirror util.rs::card_matches_name_constraint — exact normalized match. */
int rb_card_matches_name_constraint(int card_id, const char *name_constraint) {
    if (!name_constraint) return 1;
    Card c;
    if (!rb_decode_card_by_index((uint32_t)card_id, &c)) return 0;
    char cons[256];
    rb_norm_ws(name_constraint, cons, sizeof cons);
    int r = 0;
    if (c.name) {
        char nm[256];
        rb_norm_ws(c.name, nm, sizeof nm);
        const char *p = nm;
        while (*p) {
            const char *sep = NULL;
            for (const char *q = p; *q; q++) {
                if (*q == '&') { sep = q; break; }
                if ((unsigned char)q[0] == 0xEF && (unsigned char)q[1] == 0xBC &&
                    (unsigned char)q[2] == 0x86) { sep = q; break; }
            }
            size_t len = sep ? (size_t)(sep - p) : strlen(p);
            char tok[256];
            if (len >= sizeof tok) len = sizeof tok - 1;
            memcpy(tok, p, len); tok[len] = '\0';
            if (!strcmp(tok, cons)) { r = 1; break; }
            p = sep ? sep + (sep[0] == '&' ? 1 : 3) : p + strlen(p);
        }
    }
    rb_free_card(&c);
    return r;
}

/* ── HeartColor / duration / target helpers ───────────────────────────── */

/* Mirror util.rs::heart_gain_per_entry. */
int rb_heart_gain_per_entry(int total, int n_colors) {
    int len = n_colors > 0 ? n_colors : 1;
    return total / len;
}

/* Mirror util.rs::is_all_heart_type. */
int rb_is_all_heart_type(const AbilityEffect *e) {
    const char *ht = eff_extra(e, "heart_type");
    return ht && (!strcmp(ht, "all") || !strcmp(ht, RB_HEART_ALL_KEY));
}

/* Mirror util.rs::constant_per_unit_zone. */
const char *rb_constant_per_unit_zone(const AbilityEffect *e) {
    const char *loc = eff_extra(e, "location");
    if (loc) return loc;
    const char *per = eff_extra(e, "per_unit_type");
    if (per) return per;
    return "hand";
}

/* Mirror util.rs::target_player_index — returns -1 when unresolvable. */
int rb_target_player_index(const char *target, const char *master) {
    if (!target) return -1;
    int master_p2 = master && (!strcmp(master, "player2") || !strcmp(master, "p2"));
    if (!strcmp(target, "self"))     return master_p2 ? 1 : 0;
    if (!strcmp(target, "opponent")) return master_p2 ? 0 : 1;
    if (str_ieq(target, "player1") || !strcmp(target, "p1")) return 0;
    if (str_ieq(target, "player2") || !strcmp(target, "p2")) return 1;
    return -1;
}

/* Mirror util.rs::target_player_label. */
const char *rb_target_player_label(const char *target, const char *master) {
    if (target) {
        int master_p2 = master && (!strcmp(master, "player2") || !strcmp(master, "p2"));
        if (!strcmp(target, "self"))     return master_p2 ? "P2" : "P1";
        if (!strcmp(target, "opponent")) return master_p2 ? "P1" : "P2";
    }
    return "P1";
}

/* Mirror util.rs::parse_duration. */
int rb_parse_duration(const char *s) {
    if (!s) return RB_TEMP_LIVE_END;
    if (!strcmp(s, "permanent") || !strcmp(s, "as_long_as")) return RB_TEMP_PERM;
    if (!strcmp(s, "this_turn") || !strcmp(s, "turn_end") ||
        !strcmp(s, "until_end_of_turn") || !strcmp(s, "first_turn")) return RB_TEMP_TURN_END;
    return RB_TEMP_LIVE_END;
}

/* ── Position / stage helpers ─────────────────────────────────────────── */

/* Mirror util.rs::activation_position_index. */
int rb_activation_position_index(const char *p) {
    if (!p) return -1;
    while (*p == ' ' || *p == '\t') p++;
    if (!strcmp(p, "left") || !strcmp(p, "left_side")) return 0;
    if (!strcmp(p, "center")) return 1;
    if (!strcmp(p, "right") || !strcmp(p, "right_side")) return 2;
    return -1;
}

/* Mirror util.rs::stage_position_index. */
int rb_stage_position_index(const char *pos) {
    if (!pos) return -1;
    if (!strcmp(pos, "center") || !strcmp(pos, "センターエリア")) return 1;
    if (!strcmp(pos, "left_side") || !strcmp(pos, "左サイドエリア") || !strcmp(pos, "left")) return 0;
    if (!strcmp(pos, "right_side") || !strcmp(pos, "右サイドエリア") || !strcmp(pos, "right")) return 2;
    return -1;
}

/* Mirror util.rs::pos_to_area — delegates to stage index, RightSide fallback. */
int rb_pos_to_area(const char *pos) {
    if (!pos) return 1;
    if (!strcmp(pos, "left_side"))  return 0;
    if (!strcmp(pos, "center"))     return 1;
    if (!strcmp(pos, "right_side")) return 2;
    return 1;
}

/* Mirror util.rs::card_at_position. */
int rb_card_at_position(const struct GameState *g, int pl, const char *pos) {
    int idx = -1;
    if (!pos) return -1;
    if (!strcmp(pos, "left_side")) idx = 0;
    else if (!strcmp(pos, "center")) idx = 1;
    else if (!strcmp(pos, "right_side")) idx = 2;
    else return -1;
    if (idx < 0 || idx >= RB_STAGE_SIZE) return -1;
    int cid = g->p[pl].stage[idx];
    return cid == RB_EMPTY_SLOT ? -1 : cid;
}

/* Per-card identity override slots (mirrors Rust set_card_identity). */
#define RB_MAX_IDENT 8
static char *g_ident[RB_MAX_CARD_IDS];
static int  g_ident_n[RB_MAX_CARD_IDS];

/* Forward declarations for mutually-recursive helpers. */
/* NAMED zone_cards_bag, NOT zone_bag, ON PURPOSE.
 *
 * src/test_game.c:723 defines a second static `zone_bag` with a DIFFERENT
 * zone->bag table (it maps energy_deck -> &P->energy_deck and has no deck_top /
 * deck_bottom). This one is the port of Rust `zone_cards` (util.rs:2211-2234),
 * whose table deliberately OMITS energy_deck, so energy_deck resolves to NULL
 * here. See the full note at the definition. The two are NOT duplicates and
 * must never be merged -- see that note for why each answer is correct for its
 * own question -- but while they shared the name `zone_bag`, a grep found two
 * tables and could not tell which applied, which is exactly how a divergence
 * stays silent. The rename is what removes the hazard; the table is unchanged. */
static RbBag *zone_cards_bag(RbPlayer *P, const char *zone);
/* External linkage on purpose: the definition at ~line 1801 is non-static and
   move.c calls this across translation units. Declaring it `static` here fixed
   the linkage to internal, so the whole tree failed to link with "undefined
   reference" from move.c. Do not re-add `static`. */
int rb_get_selection_indices_filter(const int *cards, int n, const RbCardFilter *filter,
                                    int self_target_only, int activating_card,
                                    int *out_idx, int max);
static int rb_resolve_selection_filter(const int *cards, int n, const RbCardFilter *filter,
                                        int count, int is_all, int self_target_only,
                                        int activating_card);

/* ── Zone card access ──────────────────────────────────────────────────── */

/* Mirror util.rs::zone_cards — fill out_ids (capacity max) with card_no indices
   for the named zone of player pl; returns the count written. */
int rb_zone_cards(const struct GameState *g, int pl, const char *zone, int *out_ids, int max) {
    const RbPlayer *P = &g->p[pl];
    int n = 0;
    #define PUSH(cid) do { if (n < max) out_ids[n++] = (cid); } while(0)
    if (!zone) return 0;
    if (!strcmp(zone, "stage")) {
        for (int i = 0; i < RB_STAGE_SIZE; i++) if (P->stage[i] != RB_EMPTY_SLOT) PUSH(P->stage[i]);
    } else if (!strcmp(zone, "hand")) {
        for (int i = 0; i < P->hand.n; i++) PUSH(P->hand.cards[i]);
    } else if (!strcmp(zone, "deck") || !strcmp(zone, "deck_top") || !strcmp(zone, "deck_bottom")) {
        for (int i = 0; i < P->deck.n; i++) PUSH(P->deck.cards[i]);
    } else if (!strcmp(zone, "discard") || !strcmp(zone, "waitroom")) {
        for (int i = 0; i < P->discard.n; i++) PUSH(P->discard.cards[i]);
    } else if (!strcmp(zone, "energy") || !strcmp(zone, "energy_zone")) {
        for (int i = 0; i < P->energy.n; i++) PUSH(P->energy.cards[i]);
    } else if (!strcmp(zone, "live") || !strcmp(zone, "live_card_zone")) {
        for (int i = 0; i < P->live.n; i++) PUSH(P->live.cards[i]);
    } else if (!strcmp(zone, "success") || !strcmp(zone, "success_live_zone") ||
               !strcmp(zone, "success_live_card_zone")) {
        for (int i = 0; i < P->success.n; i++) PUSH(P->success.cards[i]);
    }
    #undef PUSH
    return n;
}

/* Mirror util.rs::zone_card_ids. */
int rb_zone_card_ids(const GameState *g, int pl, const char *zone, int *out_ids, int max) {
    return rb_zone_cards(g, pl, zone, out_ids, max);
}

/* Mirror util.rs::get_zone_card_count. */
int rb_get_zone_card_count(const GameState *g, int pl, const char *zone) {
    if (!zone) return 0;
    const RbPlayer *P = &g->p[pl];
    if (!strcmp(zone, "stage")) {
        int n = 0;
        for (int i = 0; i < RB_STAGE_SIZE; i++) if (P->stage[i] != RB_EMPTY_SLOT) n++;
        return n;
    }
    if (!strcmp(zone, "under_member") || !strcmp(zone, "under")) {
        int n = 0;
        for (int s = 0; s < RB_STAGE_SIZE; s++) n += P->under_cards[s].n;
        return n;
    }
    return rb_count_in_zone(g, pl, zone);
}

/* Mirror util.rs::count_in_zone (with optional card_type/group filter). */
int rb_count_in_zone(const GameState *g, int pl, const char *zone) {
    RbPlayer *P = (RbPlayer *)&g->p[pl];
    if (!strcmp(zone, "stage")) {
        int n = 0;
        for (int i = 0; i < RB_STAGE_SIZE; i++) if (P->stage[i] != RB_EMPTY_SLOT) n++;
        return n;
    }
    if (!strcmp(zone, "under_member") || !strcmp(zone, "under")) {
        int n = 0;
        for (int s = 0; s < RB_STAGE_SIZE; s++) n += P->under_cards[s].n;
        return n;
    }
    RbBag *b = zone_cards_bag(P, zone);
    return b ? b->n : 0;
}
int rb_count_in_zone_filtered(const GameState *g, int pl, const char *zone,
                              const char *card_type, const char *group) {
    if (!zone) return 0;
    const RbPlayer *P = &g->p[pl];
    /* util.rs count_in_zone takes a CardFilter, whose `group` is
       `group_names.first()` (card.rs:2397) — never the whole CSV list. */
    char gbuf[128];
    const char *g1 = rb_group_name_first(group, gbuf, sizeof gbuf);
    if (!strcmp(zone, "stage")) {
        int n = 0;
        for (int i = 0; i < RB_STAGE_SIZE; i++) {
            int cid = P->stage[i];
            if (cid == RB_EMPTY_SLOT) continue;
            if (card_type && !rb_card_matches_type(cid, card_type)) continue;
            if (g1 && *g1 && !rb_card_matches_group_str(cid, g1)) continue;
            n++;
        }
        return n;
    }
    if (!strcmp(zone, "under_member") || !strcmp(zone, "under")) {
        int n = 0;
        for (int s = 0; s < RB_STAGE_SIZE; s++) {
            const RbBag *u = &P->under_cards[s];
            for (int i = 0; i < u->n; i++) {
                int cid = u->cards[i];
                if (card_type && !rb_card_matches_type(cid, card_type)) continue;
                if (g1 && *g1 && !rb_card_matches_group_str(cid, g1)) continue;
                n++;
            }
        }
        return n;
    }
    int ids[RB_MAX_ZONE];
    int n = rb_zone_cards(g, pl, zone, ids, RB_MAX_ZONE);
    int count = 0;
    for (int i = 0; i < n; i++) {
        if (card_type && !rb_card_matches_type(ids[i], card_type)) continue;
        if (g1 && *g1 && !rb_card_matches_group_str(ids[i], g1)) continue;
        count++;
    }
    return count;
}

/* ── Zone move / place helpers ─────────────────────────────────────────── */

/* Resolve a zone STRING to the bag that physically holds it.
 *
 * Mirror engine/src/ability/util.rs:2066-2088 `zone_cards`, which maps
 * `Deck | DeckTop | DeckBottom` onto the SAME `main_deck` slice and falls into
 * the `_ => &[]` arm for `Zone::EnergyDeck` -- so energy_deck counts as EMPTY
 * there and must keep resolving to NULL below (rb_count_in_zone shares this
 * helper). The deck_top/deck_bottom arms were simply absent, which made
 * rb_place_card_in_zone() report "not placed" for them AFTER the caller had
 * already lifted the card out of its source: the card was DELETED from the
 * game. Proof: test_parity_bp7 kanon_selects_each_group_to_deck_bottom_and_draws
 * measured CENSUS 44->41, three cards gone while the deck length looked merely
 * unchanged. A zone count cannot tell "never moved" from "destroyed"; only a
 * whole-board census can.
 *
 * placement (util.rs:2330-2348) is the only operation Rust performs on
 * energy_deck, and it is handled by its own arm in rb_place_card_in_zone.
 *
 * ── ADJUDICATED AGAINST src/test_game.c's `zone_bag` (the "divergent alias
 *    tables" finding). DO NOT MERGE THE TWO.
 *
 * A global census flagged two static functions named `zone_bag` with different
 * zone->bag tables and asked for one to be unified or renamed. Checked against
 * Rust rather than against token similarity, and they are NOT duplicates --
 * they answer two different questions, each correctly:
 *
 *   THIS function ports Rust `zone_cards` (util.rs:2211-2234), whose match arms
 *   are Hand / Deck+DeckTop+DeckBottom / Discard+Waitroom / EnergyZone+Energy /
 *   LiveCardZone / SuccessLiveZone. `Zone::EnergyDeck` is NOT among them, so it
 *   reaches the `_ => &[]` arm and yields an EMPTY slice. Every consumer here
 *   (rb_count_in_zone, rb_remove_card_from_zone, rb_place_card_in_zone) is
 *   therefore right to report energy_deck as empty, and this table is correct
 *   AS IT STANDS. Changing it to include energy_deck would make
 *   rb_count_in_zone disagree with Rust `count_in_zone` (util.rs:2264-2281),
 *   which is built on `zone_cards`.
 *
 *   test_game.c:723 `zone_bag` is NOT a port of `zone_cards`. It is a test
 *   INSPECTION helper behind test_zone_len / test_zone_ids /
 *   test_zone_count_of_id / test_zone_has_id, whose job is "what is physically
 *   in this collection" -- and `energy_deck` IS a real collection there
 *   (Rust tests read `player.energy_deck.cards` directly).
 *
 * Unifying them would have deleted a correct function, which is the same trap
 * the census itself recorded for `card_total_hearts`. So the TABLE IS UNCHANGED
 * and only the NAME moved: `zone_bag` -> `zone_cards_bag`, naming the Rust item
 * this one ports. That is enough -- the hazard was never the two tables, it was
 * that one grep returned two tables and no way to tell which applied.
 *
 * HAZARD REPORTED, NOT FIXED (test_game.c is not this file's): a C test that
 * asserts PARITY against a Rust `zone_cards(...)` call using "energy_deck",
 * "deck_top" or "deck_bottom" gets a different answer from
 * test_zone_len/test_zone_ids than the Rust reference would give. That is a
 * measurement hazard in the test harness, not an engine bug. */
static RbBag *zone_cards_bag(RbPlayer *P, const char *zone) {
    if (!strcmp(zone, "hand")) return &P->hand;
    if (!strcmp(zone, "deck") || !strcmp(zone, "main_deck") ||
        !strcmp(zone, "deck_top") || !strcmp(zone, "deck_bottom")) return &P->deck;
    if (!strcmp(zone, "waitroom") || !strcmp(zone, "discard")) return &P->discard;
    if (!strcmp(zone, "energy") || !strcmp(zone, "energy_zone")) return &P->energy;
    if (!strcmp(zone, "live") || !strcmp(zone, "live_card_zone")) return &P->live;
    if (!strcmp(zone, "success") || !strcmp(zone, "success_live_zone") ||
        !strcmp(zone, "success_live_card_zone")) return &P->success;
    return NULL;
}

int rb_remove_card_from_zone(GameState *g, int pl, int card_id, const char *zone) {
    RbPlayer *P = &g->p[pl];
    if (!strcmp(zone, "stage")) {
        for (int i = 0; i < RB_STAGE_SIZE; i++)
            if (P->stage[i] == card_id) { P->stage[i] = RB_EMPTY_SLOT; P->stage_wait[i] = 0; return 1; }
        return 0;
    }
    if (!strcmp(zone, "under_member") || !strcmp(zone, "under")) {
        for (int s = 0; s < RB_STAGE_SIZE; s++) {
            RbBag *u = &P->under_cards[s];
            for (int i = 0; i < u->n; i++)
                if (u->cards[i] == card_id) {
                    for (int k = i; k < u->n - 1; k++) u->cards[k] = u->cards[k + 1];
                    u->n--; return 1;
                }
        }
        return 0;
    }
    /* Rust's remove_card_from_zone (util.rs:2181-2241) matches Zone::Deck and
     * nothing else among the deck spellings: `deck_top`, `deck_bottom` and
     * `energy_deck` fall to the `_ => {}` arm and remove NOTHING. zone_cards_bag()
     * above deliberately maps deck_top/deck_bottom onto main_deck, because that
     * is what zone_cards does (util.rs:2073) and place_card_in_zone needs it;
     * this guard is what keeps the two Rust functions' differing scopes from
     * being merged into one. Without it a src of "deck_bottom" would start
     * removing cards the Rust engine leaves in the deck. */
    if (!strcmp(zone, "deck_top") || !strcmp(zone, "deck_bottom") ||
        !strcmp(zone, "energy_deck"))
        return 0;
    RbBag *b = zone_cards_bag(P, zone);
    if (!b) return 0;
    for (int i = 0; i < b->n; i++)
        if (b->cards[i] == card_id) {
            for (int k = i; k < b->n - 1; k++) b->cards[k] = b->cards[k + 1];
            b->n--; return 1;
        }
    return 0;
}

/* util.rs:2453-2463 `stage_first_empty`. The probe order is centre, then left,
 * then right -- NOT ascending index. rb_stage_first_empty (zones.c:193) scans
 * 0,1,2 and so disagrees with Rust whenever both slot 0 and slot 1 are empty.
 * The SameArea arm below has to use THIS one to stay faithful; the stage arm
 * above it is left on rb_stage_first_empty so the earlier full-stage fix keeps
 * the behaviour it was measured against. Divergence reported, not silently
 * reconciled: zones.c is not this file. */
static int rb_util_stage_first_empty_rust(const int stage[RB_STAGE_SIZE]) {
    if (stage[1] == RB_EMPTY_SLOT) return 1;
    if (stage[0] == RB_EMPTY_SLOT) return 0;
    if (stage[2] == RB_EMPTY_SLOT) return 2;
    return -1;
}

/* Park a card whose intended bag is at RB_MAX_ZONE. Rust has no ceiling at
 * all -- every destination bag is a SmallVec whose add_card cannot fail
 * (util.rs:2347, :2351, :2381, :2389, :2393) -- so overflow is a C-port
 * artefact, and the only question is where the card goes instead of nowhere.
 * The WAITROOM is the established overflow sink (util.rs:2363-2367 parks a
 * full-stage card there); the HAND is the catch-all's sink (util.rs:2446), and
 * it is also the one bag whose limit (RB_MAX_HAND) is not the one just hit.
 * Returns 1 either way: a full bag is never a "not placed" verdict, because
 * every caller has already lifted the card out of its source zone. */
static int rb_park_overflow(RbPlayer *P, int card_id) {
    if (P->discard.n < RB_MAX_ZONE) {
        P->discard.cards[P->discard.n++] = card_id;
        return 1;
    }
    if (P->hand.n < RB_MAX_HAND) {
        P->hand.cards[P->hand.n++] = card_id;
        return 1;
    }
    return 1;
}

int rb_place_card_in_zone(GameState *g, int pl, int card_id, const char *zone,
                          int vacated_area) {
    RbPlayer *P = &g->p[pl];
    if (!strcmp(zone, "stage") || !strcmp(zone, "empty_area")) {
        int area = vacated_area >= 0 ? vacated_area : rb_stage_first_empty(P->stage);
        if (area < 0 || area >= RB_STAGE_SIZE) {
            /* util.rs:2354-2368 — a FULL stage is NOT a failure. Rust's
             * `Some(Zone::Stage) | Some(Zone::EmptyArea)` arm parks the card in
             * the WAITROOM ("Stage full — return card to discard instead of
             * hand") and still returns `true`.
             *
             * Returning 0 here is the second card-destroying defect of the day
             * (the first was zone_cards_bag() below missing deck_top/deck_bottom):
             * EVERY caller of this function has already lifted the card out of
             * its source zone — rb_move_card at util.c:976 removes first, and
             * the three choice.c call sites (choice.c:962 send_to_dst, :1732
             * hand-cost, :4276 energy-zone) all call rb_waitroom_remove_card
             * first — so a "not placed" verdict leaves the card in no zone at
             * all. Measured on the nico family: board census 12 -> 11 and
             * 14 -> 13 with an `empty_area` destination onto a full stage.
             * Only a whole-board census distinguishes "moved" from "destroyed";
             * a zone count reads the same either way. */
            if (P->discard.n < RB_MAX_ZONE) P->discard.cards[P->discard.n++] = card_id;
            return 1;
        }
        P->stage[area] = card_id; P->stage_wait[area] = 0;
        return 1;
    }
    /* util.rs:2330-2336 `Zone::Deck | Zone::DeckTop` -- INSERT at
     * vacated_stage_area (defaulting to 0, i.e. the top of the bag, which is
     * index 0 because the C deck bag is top-first), clamped to the deck
     * length. util.rs:2337-2340 `Zone::DeckBottom` -- push, i.e. the end of
     * the bag. util.rs:2345-2348 `Zone::EnergyDeck` -- push onto the energy
     * deck bag.
     *
     * These three were the card-destroying gap: zone_cards_bag() could not name
     * deck_bottom, so every caller that had ALREADY removed the card from its
     * source zone got a 0 back and the card ended up in no zone at all. The
     * CHOICE-driven path is the one that measured it (CENSUS 44->41 in
     * test_parity_bp7); move.c's plain path already appended for deck_bottom. */
    if (!strcmp(zone, "deck") || !strcmp(zone, "deck_top")) {
        int idx = vacated_area >= 0 ? vacated_area : 0;
        if (idx > P->deck.n) idx = P->deck.n;
        if (P->deck.n < RB_MAX_ZONE) {
            for (int k = P->deck.n; k > idx; k--) P->deck.cards[k] = P->deck.cards[k - 1];
            P->deck.cards[idx] = card_id;
            P->deck.n++;
        } else {
            return rb_park_overflow(P, card_id);
        }
        return 1;
    }
    if (!strcmp(zone, "deck_bottom")) {
        if (P->deck.n < RB_MAX_ZONE) P->deck.cards[P->deck.n++] = card_id;
        else return rb_park_overflow(P, card_id);
        return 1;
    }
    if (!strcmp(zone, "energy_deck")) {
        if (P->energy_deck.n < RB_MAX_ZONE)
            P->energy_deck.cards[P->energy_deck.n++] = card_id;
        else return rb_park_overflow(P, card_id);
        return 1;
    }
    /* util.rs:2396-2422 `Some(Zone::SameArea)`. A DEDICATED arm, not a zone_bag
     * lookup: the destination names an AREA RELATIONSHIP, not a bag, so it can
     * resolve to stage, to hand, or -- via first_empty -- to any of the three
     * slots. Corpus: effect.destination=same_area on 9 abilities, e.g.
     * PL!S-bp3-006-P. Before this arm the generic tail returned 0 AFTER the
     * caller had already lifted the card out of its source, so all 9 deleted.
     *
     * Rust's order is vacated_area if that slot is empty, else first empty,
     * else the HAND. It never returns false, so the hand is the floor. */
    if (!strcmp(zone, "same_area")) {
        if (vacated_area >= 0 && vacated_area < RB_STAGE_SIZE &&
            P->stage[vacated_area] == RB_EMPTY_SLOT) {
            P->stage[vacated_area] = card_id;
            P->stage_wait[vacated_area] = 0;
            rb_zone_track_deployment(g, pl, card_id);
            return 1;
        }
        int ep = rb_util_stage_first_empty_rust(P->stage);
        if (ep >= 0) {
            P->stage[ep] = card_id;
            P->stage_wait[ep] = 0;
            rb_zone_track_deployment(g, pl, card_id);
        } else {
            rb_hand_add(P, card_id);
        }
        return 1;
    }
    /* util.rs:2423-2441 `Some(Zone::UnderMember)`, Rule 4.5.5. "under" is the
     * same variant (enums.rs:108 aliases it). Corpus: destination=under_member
     * on 21 effect + 5 cost records. move.c:1333 and choice.c:966 intercept the
     * stage-choice and choice routes for this name, but the plain move routes
     * (move.c:1542/1575/1767/1915, move_missing.c:250, choice.c:1738/4584) all
     * arrive here, so the arm has to exist.
     *
     * Rust's fallback prefers an OCCUPIED slot, center then left then right, and
     * sends the card to the WAITROOM when the stage is entirely empty --
     * it returns true in every one of those cases. */
    if (!strcmp(zone, "under_member") || !strcmp(zone, "under")) {
        /* util.rs:2426-2437: `vacated_stage_area` is Option<usize>, so Rust
         * only consults it when it is Some. vacated_area < 0 is the C's None. */
        int target_idx = -1;
        if (vacated_area >= 0) {
            /* util.rs:2438 pos_to_area(target_idx) -> MemberArea::from_index,
             * whose `_` arm is RightSide (zones.rs:102-109, util.rs:2468-2470).
             * An out-of-range index therefore lands on slot 2, NOT nowhere. */
            target_idx = (vacated_area < RB_STAGE_SIZE) ? vacated_area
                                                       : RB_STAGE_SIZE - 1;
        } else if (P->stage[1] != RB_EMPTY_SLOT) {
            target_idx = 1;
        } else if (P->stage[0] != RB_EMPTY_SLOT) {
            target_idx = 0;
        } else if (P->stage[2] != RB_EMPTY_SLOT) {
            target_idx = 2;
        }
        if (target_idx < 0) {
            /* util.rs:2435-2436: the stage is entirely empty, so Rust parks
             * the card in the WAITROOM and returns true. */
            if (P->discard.n < RB_MAX_ZONE) {
                P->discard.cards[P->discard.n++] = card_id;
                return 1;
            }
            return rb_park_overflow(P, card_id);
        }
        rb_stage_place_under_card(P, target_idx, card_id);
        return 1;
    }
    RbBag *b = zone_cards_bag(P, zone);
    if (!b) {
        /* util.rs:2442-2449 `_ =>` arm. An unrecognised destination is NOT a
         * failure: Rust parks the card in the HAND, or in the WAITROOM for the
         * empty string, and returns `true`.
         *
         * Returning 0 here is a DELETION, and this is the narrowest place that
         * fact is visible. Every caller has already lifted the card out of its
         * source -- rb_move_card below removes first, and the choice.c call
         * sites (send_to_dst, hand cost, energy zone) all drain the source
         * bag before calling -- so a "not placed" verdict leaves the card in
         * NO zone at all. Measured on this file's own probe: whole-board
         * census 1 -> 0 for each of same_area, deck_top_or_bottom, front,
         * under_member and "". A per-zone count cannot tell "moved nowhere"
         * from "destroyed"; only the census can.
         *
         * Corpus destinations that reach this arm, because Zone::from_str
         * (enums.rs:89-122) maps them to a real variant that
         * place_card_in_zone has no dedicated arm for: deck_top_or_bottom x3
         * (e.g. PL!S-bp6-002-P) and front x1 (PL!HS-pb1-014-P+). This is
         * FAITHFUL, not a gap: Rust's own match ends in the same catch-all
         * (engine/src/ability/util.rs:2548-2555) and sends BOTH to
         * `player.hand.add_card(card_id)` — `_ =>` fires because neither
         * DeckTopOrBottom nor Front appears in the arms above it. So the C's
         * hand destination is what Rust does, and porting a dedicated
         * deck_top/front arm here would DIVERGE.
         *
         * An earlier version of this comment also listed `""` as a corpus
         * destination reaching this arm. That was wrong: `destination: ""`
         * measures 0 unique abilities and 0 cards, so no corpus node carries
         * it. The `zone && !zone[0]` branch below is still correct and still
         * needed — it is Rust's `if destination.is_empty()` at util.rs:2549,
         * reachable only from a non-corpus caller — but it is not, and never
         * was, a corpus destination. Rust has no failure arm anywhere in
         * place_card_in_zone, so no destination string can delete a card. */
        if (zone && !zone[0]) {
            if (P->discard.n < RB_MAX_ZONE)
                P->discard.cards[P->discard.n++] = card_id;
            else return rb_park_overflow(P, card_id);
        } else {
            rb_hand_add(P, card_id);
        }
        return 1;
    }
    if (b->n < RB_MAX_ZONE) { b->cards[b->n++] = card_id; return 1; }
    /* A FULL bag is not a "not placed" verdict either. Every Rust zone here is
     * a SmallVec whose add_card cannot fail (util.rs:2347, :2351, :2381,
     * :2389, :2393), so the only difference from Rust is the fixed ceiling.
     * Returning 0 made a full bag a deletion -- probe: census 513 -> 512 with
     * the waitroom at RB_MAX_ZONE. Park it in the WAITROOM, which is what the
     * Stage arm above already does for a full stage (util.rs:2363-2367). */
    return rb_park_overflow(P, card_id);
}

int rb_move_card(GameState *g, int pl, int card_id, const char *src,
                 const char *dst, int vacated_area) {
    if (rb_remove_card_from_zone(g, pl, card_id, src)) {
        /* Rust push_movement_event (modifiers.rs:1833-1907) sets
         * recently_moved_from_zone = Some(source_zone) on EVERY recorded
         * movement (:1875-1877), before it looks at the destination. That
         * field is what populates TriggerEvent.moved_from_zone
         * (abilities.rs:448, :2269), which is what a move-phase (idoutsuuji)
         * condition gated on the SOURCE zone matches against
         * (actions/mod.rs:1509).
         *
         * The C recorded the destination only -- or nothing at all -- so every
         * source-gated move condition was unreachable: the field declared at
         * rabuka.h:1555 had NO WRITER anywhere in src/ (grep for
         * recently_moved_from_zone over engine_c_wip/src returns only the
         * declaration). Recording the SOURCE is what the field is named for and
         * what Rust stores; the destination was never the value. */
        if (src && *src)
            snprintf(g->recently_moved_from_zone,
                     sizeof g->recently_moved_from_zone, "%s", src);
        return rb_place_card_in_zone(g, pl, card_id, dst, vacated_area);
    }
    return 0;
}

int rb_move_cards(GameState *g, int pl, const int *card_ids, int n,
                  const char *src, const char *dst, int vacated_area) {
    int c = 0;
    for (int i = 0; i < n; i++)
        if (rb_move_card(g, pl, card_ids[i], src, dst, vacated_area)) c++;
    return c;
}

int rb_resolve_indices_to_ids(const GameState *g, int pl, const char *zone,
                               const int *indices, int n_idx, int *out) {
    int ids[RB_MAX_ZONE];
    int n = rb_zone_cards(g, pl, zone, ids, RB_MAX_ZONE);
    int m = 0;
    for (int i = 0; i < n_idx; i++) {
        int idx = indices[i];
        if (idx >= 0 && idx < n) out[m++] = ids[idx];
    }
    return m;
}

/* ── Card-filter matching (local helpers) ──────────────────────────────── */

/* util.rs:1595-1602 any_trigger — a wanted label matches when the RESOLVED
   trigger text STARTS WITH it. 「常時」 is a prefix of 「常時:...」, so
   starts_with accepts the suffixed form; what it does NOT accept is the reverse
   (a bare 「常時」 request matching inside a longer token) or any mid-token hit.
   The C port used strstr (CONTAINS), which accepted both. */
static int local_any_trigger_prefix(int card_id, const char (*wanted)[32], int n_wanted)
{
    int n = rb_card_num_abilities((uint32_t)card_id);
    for (int i = 0; i < n; i++) {
        Ability ab;
        if (!rb_decode_card_ability((uint32_t)card_id, i, &ab)) continue;
        int hit = 0;
        if (ab.triggers) {
            for (int j = 0; j < n_wanted && !hit; j++) {
                size_t len = strlen(wanted[j]);
                if (len && !strncmp(ab.triggers, wanted[j], len)) hit = 1;
            }
        }
        rb_free_ability(&ab);
        if (hit) return 1;
    }
    return 0;
}

/* util.rs:1584-1617 card_passes_ability_filter.
   `require_ability` selects Unconstrained::RequireAbility — the shape the
   or_ability_filters branches use — where a filter name that names no trigger
   list degrades to "has an ability at all", because in a disjunction a
   vacuously-true branch would make the whole OR meaningless. The standalone
   `ability_filter` uses Unconstrained::Pass, where the same case constrains
   nothing. The trigger list is `Option<Vec<String>>` in Rust; an absent or EMPTY
   list is the `_` arm, so a filter that needs a trigger list and got none
   rejects nothing (Pass) or requires only an ability (RequireAbility). */
static int local_card_passes_ability_filter(int card_id, const char *ability_filter,
                                            const char (*triggers)[32], int n_triggers,
                                            int require_ability)
{
    int has_ability = rb_card_num_abilities((uint32_t)card_id) > 0;
    int unconstrained = require_ability ? has_ability : 1;
    const char *af = ability_filter ? ability_filter : "";
    if (!strcmp(af, "no_ability")) return !has_ability;
    if (!strcmp(af, "has_ability")) return has_ability;
    if (!strcmp(af, "no_ability_type"))
        return n_triggers > 0 ? !local_any_trigger_prefix(card_id, triggers, n_triggers)
                              : unconstrained;
    if (!strcmp(af, "has_ability_type"))
        return n_triggers > 0 ? local_any_trigger_prefix(card_id, triggers, n_triggers)
                              : unconstrained;
    return 1;   /* an unrecognised name constrains nothing, in either form */
}

/* One parsed or_ability_filters branch — card.rs:942-945 AbilityFilterBranch. */
typedef struct {
    char ability_filter[32];
    char triggers[4][32];
    int  n_triggers;
} LocalOrBranch;

/* Read `"key":"value"` out of a branch object. Returns 0 when absent. */
static int local_or_branch_field(const char *json, const char *key, char *dst, size_t cap)
{
    dst[0] = 0;
    char pattern[64];
    snprintf(pattern, sizeof pattern, "\"%s\"", key);
    const char *p = strstr(json, pattern);
    if (!p) return 0;
    p += strlen(pattern);
    while (*p && *p != ':') {
        if (*p == ',') return 0;
        p++;
    }
    if (*p != ':') return 0;
    p++;
    while (*p == ' ' || *p == '"') p++;
    size_t n = 0;
    while (*p && *p != '"' && n + 1 < cap) dst[n++] = *p++;
    dst[n] = 0;
    return n > 0;
}

/* Parse one `{"$variant":N,"$fields":{...}}` branch object into its two
   decision fields. A branch that names no ability_filter is NOT an error and is
   not skipped: Rust's `ability_filter.unwrap_or("")` sends it to the `_` arm,
   which constrains nothing, so such a branch satisfies the OR. */
static int local_or_branch_parse(const char *json, LocalOrBranch *out)
{
    memset(out, 0, sizeof *out);
    if (!json || !*json) return 0;
    local_or_branch_field(json, "ability_filter", out->ability_filter,
                          sizeof out->ability_filter);
    const char *ta = strstr(json, "\"ability_filter_triggers\"");
    if (!ta) return 1;
    const char *lb = strchr(ta, '[');
    if (!lb) return 1;
    const char *rb = strchr(lb, ']');
    if (!rb) return 1;
    const char *p = lb + 1;
    while (p < rb && out->n_triggers < 4) {
        while (p < rb && (*p == ',' || *p == ' ' || *p == '"')) p++;
        const char *start = p;
        while (p < rb && *p != ',' && *p != ' ' && *p != '"') p++;
        size_t len = (size_t)(p - start);
        if (len && len < sizeof(out->triggers[0])) {
            memcpy(out->triggers[out->n_triggers], start, len);
            out->triggers[out->n_triggers][len] = 0;
            out->n_triggers++;
        }
    }
    return 1;
}

/* ── Whole-group-list membership (checklist H1) ────────────────────────────
 *
 * Rust's `CardFilter::check_group` (util.rs:1515-1539) tests THREE things, and
 * only the first of them existed in C:
 *
 *     if let Some(g) = self.group {
 *         if !card_matches_group_str(db, id, Some(g)) {
 *             if let Some(gs) = self.groups {
 *                 if !gs.iter().any(|gn| card_matches_group_str(db, id, Some(gn))) { return false }
 *             } else { return false }
 *         }
 *     } else if let Some(gs) = self.groups {
 *         if !gs.iter().any(|gn| card_matches_group_str(db, id, Some(gn))) { return false }
 *     }
 *     for g in self.exclude_group_names { if card_matches_group_str(..) { return false } }
 *
 * `group` is `group_names.first()` and `groups` is the WHOLE list
 * (util.rs:1852-1856), so the list always contains the first entry — a filter
 * naming two groups accepts a card in EITHER, which the old C arm could not do:
 * it read one name and rejected. Both forms go through the already-verified
 * rb_card_matches_group_str (118,755 pairs, zero disagreements); nothing here
 * re-implements it. */

/* One CSV token -> does `card_id` carry that group? 128 bytes covers every
 * group name in the database (20 distinct values, longest well under 64). */
static int local_group_token_matches(int card_id, const char *start, size_t len) {
    char tok[128];
    if (len == 0 || len >= sizeof tok) len = len ? sizeof tok - 1 : 0;
    if (len == 0) return 0;
    memcpy(tok, start, len);
    tok[len] = '\0';
    return rb_card_matches_group_str(card_id, tok);
}

/* util.rs:1519 / :1527 / :1532 — `.any()` over a comma-joined list. */
static int local_group_csv_any(int card_id, const char *csv) {
    if (!csv || !*csv) return 0;
    const char *p = csv;
    for (;;) {
        const char *comma = strchr(p, ',');
        size_t len = comma ? (size_t)(comma - p) : strlen(p);
        if (local_group_token_matches(card_id, p, len)) return 1;
        if (!comma) return 0;
        p = comma + 1;
    }
}

/* Same `.any()`, over the array form RbCardFilter.groups / exclude_names use. */
static int local_group_array_any(int card_id, const char (*names)[64], int n) {
    for (int i = 0; i < n && i < 8; i++)
        if (names[i][0] && rb_card_matches_group_str(card_id, names[i])) return 1;
    return 0;
}

/* Check if card_id matches the LocalCardFilter fields. */
static int local_filter_matches(const LocalCardFilter *f, int card_id) {
    Card filter_probe;
    /* util.rs:1711-1713 — check_exclude_self then check_exclude_cards, both
       before any field probe. `exclude_cards` is an exact-id list. */
    if (f->has_exclude_self && card_id == f->exclude_self_id) return 0;
    if (f->n_exclude_cards > 0) {
        int excluded = 0;
        for (int i = 0; i < f->n_exclude_cards && !excluded; i++)
            if (f->exclude_cards[i] == card_id) excluded = 1;
        if (excluded) return 0;
    }
    /* util.rs:1717 -> :1498-1513 check_exclude_names — equality on the card
       name with `char::is_whitespace` stripped from BOTH sides, which is exactly
       str_eq_ignoring_ws (util.rs:696-706). A card the database cannot decode is
       skipped, so an exclude_names entry never rejects an unknown id. */
    if (f->n_exclude_names > 0) {
        Card c;
        int rejected = 0;
        if (rb_decode_card_by_index((uint32_t)card_id, &c)) {
            for (int i = 0; i < f->n_exclude_names && !rejected; i++)
                if (c.name && str_eq_ignoring_ws(c.name, f->exclude_names[i])) rejected = 1;
            rb_free_card(&c);
        }
        if (rejected) return 0;
    }
    if (f->card_type[0] && !rb_card_matches_type(card_id, f->card_type)) return 0;
    /* CardFilter.group is populated from `group_names`, i.e. it may still hold
       the WHOLE comma-joined list (move.c:1617 and card.c:783 hand the raw CSV
       straight to RbCardFilter.group). Rust's CardFilter.group is
       `group_names.first()`, so split before comparing. */
    if (f->has_group || f->n_groups > 0) {
        int group_ok = 1;
        if (f->has_group) {
            char gbuf[128];
            const char *g = rb_group_name_first(f->group, gbuf, sizeof gbuf);
            if (g && *g && !rb_card_matches_group_str(card_id, g)) group_ok = 0;
        }
        /* A `group` that matched short-circuits the list, and an ABSENT list is
           a rejection — which is why this must not run when group_ok is still
           1. util.rs:1517-1525. */
        if (!group_ok && !local_group_array_any(card_id, f->groups, f->n_groups))
            return 0;
    }
    /* util.rs:1531-1537 — the excluded list is checked unconditionally, group
       match or not, and rejects on the FIRST hit. */
    if (local_group_csv_any(card_id, f->exclude_group_names)) return 0;
    if (f->has_cost_limit && !rb_card_matches_cost_limit(card_id, f->cost_limit, f->cost_op[0] ? f->cost_op : NULL))
        return 0;
    if (f->n_cost_values > 0) {
        Card c;
        if (!rb_decode_card_by_index((uint32_t)card_id, &c)) return 0;
        int cost = c.cost;
        rb_free_card(&c);
        int found = 0;
        for (int i = 0; i < f->n_cost_values; i++) if (f->cost_values[i] == cost) found = 1;
        if (!found) return 0;
    }
    /* util.rs:1744-1752 — `cost_limit_min` is the card-text spelling of ">=" and
       `cost_limit_max` of "<="; Rust passes those operators LITERALLY into the
       same card_matches_cost_limit_op_for the `cost_limit` term uses
       (util.rs:1733-1737), so the twin to reuse is rb_card_matches_cost_limit,
       not a second comparison written here. */
    if (f->has_cost_limit_min && !rb_card_matches_cost_limit(card_id, f->cost_limit_min, ">="))
        return 0;
    if (f->has_cost_limit_max && !rb_card_matches_cost_limit(card_id, f->cost_limit_max, "<="))
        return 0;
    if (f->has_cost_total) {
        Card c;
        if (!rb_decode_card_by_index((uint32_t)card_id, &c)) return 0;
        int total = c.cost;
        rb_free_card(&c);
        if (!rb_compare_counts(f->cost_total_op[0] ? f->cost_total_op : NULL, total, f->cost_total)) return 0;
    }
    if (f->has_blade_limit) {
        Card c;
        if (!rb_decode_card_by_index((uint32_t)card_id, &c)) return 0;
        int blade = c.blade;
        rb_free_card(&c);
        if (!rb_compare_counts(f->blade_op[0] ? f->blade_op : NULL, blade, f->blade_limit)) return 0;
    }
    if (f->has_characters) {
        /* Rust (util.rs:1474-1476): a single `characters` list, matched with
           card_matches_characters (name contains the normalized character).
           RbCardFilter::characters is one name string, so wrap it as a
           one-element list. The previous code cast the char[256] buffer to a
           `const char **` and dereferenced it as a pointer. */
        const char *names[1];
        names[0] = f->characters;
        if (!rb_card_matches_characters(card_id, names, 1)) return 0;
    }
    if (f->has_exclude_characters) {
        /* Passes only if it does NOT match the excluded characters */
        Card c;
        if (rb_decode_card_by_index((uint32_t)card_id, &c) && c.name) {
            char nc[512], ec[256];
            rb_card_normalize_name(c.name, nc, sizeof nc);
            rb_card_normalize_name(f->exclude_characters, ec, sizeof ec);
            if (strstr(nc, ec)) { rb_free_card(&c); return 0; }
            rb_free_card(&c);
        }
    }
    if (f->n_heart_colors > 0) {
        const char *colors[8];
        for (int i = 0; i < f->n_heart_colors; i++) colors[i] = f->heart_colors[i];
        int ok = f->require_all_heart_colors
            ? rb_card_matches_all_heart_colors(card_id, colors, f->n_heart_colors)
            : rb_card_matches_heart_colors(card_id, colors, f->n_heart_colors);
        if (!ok) return 0;
    }
    if (f->heart_color_count > 0) {
        /* util.rs:1257-1271 — heart_count_for_threshold(color) >= min_count for
           EVERY listed colour when require_all_heart_colors, else for ANY one.
           The C port used a 0/1 presence flag (so a threshold of 2 could never
           pass) and ignored require_all. */
        Card c;
        if (rb_decode_card_by_index((uint32_t)card_id, &c)) {
            int passes = f->require_all_heart_colors ? 1 : 0;
            for (int hc_idx = 0; hc_idx < f->n_heart_colors; hc_idx++) {
                int hc = (int)rb_parse_heart_color(f->heart_colors[hc_idx]);
                int meets = heart_count_for_threshold(&c, hc) >= f->heart_color_count;
                if (f->require_all_heart_colors) {
                    if (!meets) { passes = 0; break; }
                } else if (meets) {
                    passes = 1;
                    break;
                }
            }
            rb_free_card(&c);
            if (!passes) return 0;
        }
    }
    if (f->has_need_heart_total) {
        /* util.rs:1282-1308 — with need_heart_color, the COUNT of that colour
           in need_heart; otherwise Card::total_hearts() (printed base hearts for
           a member, need_heart for a live card). The C port counted heart
           ENTRIES for the total (c.num_base / c.num_need) and a 0/1 presence
           flag for the per-colour form. */
        Card c;
        int total = 0;
        if (rb_decode_card_by_index((uint32_t)card_id, &c)) {
            if (f->need_heart_color[0]) {
                int hc = (int)rb_parse_heart_color(f->need_heart_color);
                total = heart_count_in_range(&c, hc,
                    c.num_base + c.num_blade, c.num_base + c.num_blade + c.num_need);
            } else {
                total = card_total_hearts(&c);
            }
            rb_free_card(&c);
        }
        const char *op = f->need_heart_operator[0] ? f->need_heart_operator : ">=";
        if (!rb_compare_counts(op, total, f->need_heart_total)) return 0;
    }
    if (f->n_name_fragments > 0) {
        const char *frags[8];
        for (int i = 0; i < f->n_name_fragments; i++) frags[i] = f->name_fragments[i];
        if (!rb_card_matches_name_fragments(card_id, frags, f->n_name_fragments)) return 0;
    }
    if (f->has_original_blade) {
        Card c;
        if (!rb_decode_card_by_index((uint32_t)card_id, &c)) return 0;
        int blade = c.blade;
        rb_free_card(&c);
        if (!rb_compare_counts(f->original_blade_op[0] ? f->original_blade_op : NULL,
                               blade, f->original_blade_limit)) return 0;
    }
    /* util.rs:1788-1798 — 「元々持つハートの数がN以下/つ以上」. The total is the
       PRINTED base-heart sum and nothing else:
           db.get_card(id).and_then(|c| c.base_heart.as_ref())
              .map(|bh| bh.hearts.values().fold(0u8, |a, n| a.saturating_add(*n)))
              .unwrap_or(0)
       so a live card or energy card (no base_heart) totals 0 and therefore
       satisfies 「1個以下」. card_total_hearts() is the WRONG total here — it
       falls back to need_heart on a live card (util.rs:1592-1597 uses it for
       need_heart_total) — so the base slice is summed directly.
       DEFAULT operator is "<=", NOT compare_counts' ">=": util.rs:1907 spells
       `.or(Some("<="))`. Every reachable operator is in rb_compare_counts' table
       — from_effect feeds Operator::as_str (>=, <=, >, <, =) and the wire alias
       "=="; measured over the corpus all 5 `heart_total_operator` values are
       "<=", so the missing "exact" arm of compare_with_operator is unreachable. */
    if (f->has_original_heart_total) {
        Card c;
        int total = 0;
        if (rb_decode_card_by_index((uint32_t)card_id, &c)) {
            total = heart_count_in_range(&c, -1, 0, c.num_base);
            rb_free_card(&c);
        }
        if (!rb_compare_counts(f->original_heart_operator[0] ? f->original_heart_operator : "<=",
                               total, f->original_heart_total)) return 0;
    }
    /* util.rs:1779-1784 — check_ability_filter (util.rs:1619-1652) then
       check_card_property (util.rs:1654-1673), in that order. The C port used to
       open ONE `Card c` around the ability test and read `has_ability` off
       `c.ability`, but rb_decode_card_by_index always sets `ability = NULL`
       (card.c:127 — abilities are decoded lazily, one at a time, through
       rb_decode_card_ability), so has_ability was hard-wired to 0: `has_ability`
       rejected every card, `no_ability` accepted every card, and BOTH
       no_ability_type and has_ability_type silently degenerated (the former into
       "always pass", the latter into "always reject", because their trigger scan
       was gated on the same dead flag). Rust reads `!card.abilities.is_empty()`
       (util.rs:1590), which is rb_card_num_abilities — the same source
       effects/look.c's private copy used.

       Rust also gates the whole term on `db.get_card(id)` being Some
       (util.rs:1621), so an undecodable card is skipped, exactly as before. */
    if (rb_decode_card_by_index((uint32_t)card_id, &filter_probe)) {
        int passes = 1;
        if (f->ability_filter[0] &&
            !local_card_passes_ability_filter(card_id, f->ability_filter,
                                              (const char (*)[32])f->ability_filter_triggers,
                                              f->n_ability_filter_triggers, 0))
            passes = 0;
        /* util.rs:1634-1650 — when OR branches are present they ARE the whole
           ability filter: the card passes if ANY branch matches, each branch
           judged with Unconstrained::RequireAbility. */
        if (passes && f->n_or_ability_filters > 0) {
            passes = 0;
            for (int i = 0; i < f->n_or_ability_filters && !passes; i++) {
                LocalOrBranch br;
                if (!local_or_branch_parse(f->or_ability_filters[i], &br)) continue;
                passes = local_card_passes_ability_filter(card_id, br.ability_filter,
                                                          (const char (*)[32])br.triggers,
                                                          br.n_triggers, 1);
            }
        }
        rb_free_card(&filter_probe);
        if (!passes) return 0;
    }
    /* util.rs:1654-1673 (CardFilter::check_card_property, checked last, after the
       ability filter). `negation` inverts the property test. An unknown property
       string is false, so with negation it passes — a card the database cannot
       decode also counts as "no property", matching Rust's
       `get_card(id).is_some_and(..)` on a missing card. */
    if (f->has_card_property && f->card_property[0]) {
        Card c;
        const Card *cp = rb_decode_card_by_index((uint32_t)card_id, &c) ? &c : NULL;
        int passes = rb_check_card_property(f->card_property, f->negation, cp);
        if (cp) rb_free_card(&c);
        if (!passes) return 0;
    }
    return 1;
}

/* ── CardFilter::matches (ported) ─────────────────────────────────────── */

/* Mirror util.rs::CardFilter::has_filter. util.rs:1443-1444 counts
   or_ability_filters and card_property; the latter was already here, the former
   was missing because no producer existed. */
static int local_has_filter(const LocalCardFilter *f) {
    return f->card_type[0] || f->has_group || f->has_cost_limit || f->has_characters ||
           f->has_exclude_characters || f->n_heart_colors > 0 || f->has_need_heart_total ||
           f->n_name_fragments > 0 || f->has_original_blade || f->ability_filter[0] ||
           f->n_or_ability_filters > 0 ||
           f->has_card_property || f->has_exclude_self || f->has_cost_total ||
           f->n_cost_values > 0 || f->has_blade_limit;
}

/* Mirror util.rs::CardFilter::matches — check all present filter fields. */
static int card_filter_matches(const LocalCardFilter *f, int card_id, int skip_empty) {
    if (skip_empty && card_id < 0) return 0;
    return local_filter_matches(f, card_id);
}

/* Public: matches() with skip_empty. */
static int card_filter_matches_skip(const LocalCardFilter *f, int card_id, int skip_empty) {
    return card_filter_matches(f, card_id, skip_empty);
}

/* Public: matches_card (skip_empty=0). */
static int card_filter_matches_card(const LocalCardFilter *f, int card_id) {
    return card_filter_matches(f, card_id, 0);
}

/* ── matching_ids / matching_indices (ported) ──────────────────────────── */

/* Copy the public RbCardFilter into the local mirror. Returns 0 when the
   filter has nothing set, in which case every card matches (Rust: an
   all-None CardFilter rejects nothing). */
static int local_filter_from_public(const RbCardFilter *rf, LocalCardFilter *out) {
    LocalCardFilter f;
    memset(&f, 0, sizeof f);
    if (!rf) return 0;
    if (rf->card_type[0])    strncpy(f.card_type, rf->card_type, sizeof f.card_type - 1);
    if (rf->group[0])        { strncpy(f.group, rf->group, sizeof f.group - 1); f.has_group = 1; }
    if (rf->has_cost_limit)  { f.cost_limit = rf->cost_limit;
                                if (rf->cost_op[0]) strncpy(f.cost_op, rf->cost_op, sizeof f.cost_op - 1);
                                f.has_cost_limit = 1; }
    if (rf->characters[0])   { strncpy(f.characters, rf->characters, sizeof f.characters - 1); f.has_characters = 1; }
    if (rf->exclude_characters[0]) { strncpy(f.exclude_characters, rf->exclude_characters, sizeof f.exclude_characters - 1); f.has_exclude_characters = 1; }
    for (int i = 0; i < rf->n_heart_colors && i < 8; i++)
        strncpy(f.heart_colors[i], rf->heart_colors[i], sizeof f.heart_colors[i] - 1);
    f.n_heart_colors = rf->n_heart_colors;
    f.require_all_heart_colors = rf->require_all_heart_colors;
    f.heart_color_count = rf->heart_color_count;
    if (rf->has_need_heart_total) {
        f.need_heart_total = rf->need_heart_total;
        if (rf->need_heart_operator[0]) strncpy(f.need_heart_operator, rf->need_heart_operator, sizeof f.need_heart_operator - 1);
        if (rf->need_heart_color[0]) strncpy(f.need_heart_color, rf->need_heart_color, sizeof f.need_heart_color - 1);
        f.has_need_heart_total = 1;
    }
    for (int i = 0; i < rf->n_name_fragments && i < 8; i++)
        strncpy(f.name_fragments[i], rf->name_fragments[i], sizeof f.name_fragments[i] - 1);
    f.n_name_fragments = rf->n_name_fragments;
    if (rf->has_original_blade) {
        f.original_blade_limit = rf->original_blade_limit;
        if (rf->original_blade_op[0]) strncpy(f.original_blade_op, rf->original_blade_op, sizeof f.original_blade_op - 1);
        f.has_original_blade = 1;
    }
    if (rf->ability_filter[0]) {
        strncpy(f.ability_filter, rf->ability_filter, sizeof f.ability_filter - 1);
        for (int i = 0; i < rf->n_ability_filter_triggers && i < 8; i++)
            strncpy(f.ability_filter_triggers[i], rf->ability_filter_triggers[i],
                    sizeof f.ability_filter_triggers[i] - 1);
        f.n_ability_filter_triggers = rf->n_ability_filter_triggers;
    }
    f.negation = rf->negation;
    /* util.rs:1882 card_property: effect.card_property_any() — the member is
       unconditional in rabuka.h now, so this is a plain copy (see the header
       comment at the top of this file for why RF_CARD_PROPERTY is gone). */
    {
        const char *cp = rf->card_property;
        if (cp && *cp) { strncpy(f.card_property, cp, sizeof f.card_property - 1); f.has_card_property = 1; }
    }
    /* util.rs:1881 or_ability_filters: effect.or_ability_filters_any() — copied
       verbatim per branch; local_or_branch_parse reads the keys back out at
       match time so the wire shape stays the single source of truth. */
    for (int i = 0; i < rf->n_or_ability_filters
                    && i < (int)(sizeof(f.or_ability_filters) / sizeof(f.or_ability_filters[0])); i++)
        strncpy(f.or_ability_filters[i], rf->or_ability_filters[i],
                sizeof f.or_ability_filters[i] - 1);
    f.n_or_ability_filters = rf->n_or_ability_filters;
    if (f.n_or_ability_filters >
        (int)(sizeof(f.or_ability_filters) / sizeof(f.or_ability_filters[0])))
        f.n_or_ability_filters = (int)(sizeof(f.or_ability_filters) / sizeof(f.or_ability_filters[0]));
    if (rf->has_exclude_self) { f.exclude_self_id = rf->exclude_self_id; f.has_exclude_self = 1; }
    if (rf->has_cost_total) { f.cost_total = rf->cost_total;
                               if (rf->cost_total_op[0]) strncpy(f.cost_total_op, rf->cost_total_op, sizeof f.cost_total_op - 1);
                               f.has_cost_total = 1; }
    for (int i = 0; i < rf->n_cost_values && i < 16; i++) f.cost_values[i] = rf->cost_values[i];
    f.n_cost_values = rf->n_cost_values;
    f.has_blade_limit = rf->has_blade_limit;
    f.blade_limit = rf->blade_limit;
    if (rf->blade_op[0]) strncpy(f.blade_op, rf->blade_op, sizeof f.blade_op - 1);
    f.distinct = rf->distinct;
    /* ── The six H1 carriers. They are copied here for the first time: until
     *    now RbCardFilter carried them and local_filter_matches never looked at
     *    any of them, so a producer that populated one changed nothing. */
    /* util.rs:1356-1357 / :1906-1907 — total from the `heart_total_limit` wire
       key, operator from `heart_total_operator` with Rust's "<=" default
       applied by the matcher. Presence is read off the operator string because
       Rust guarantees the operator is Some whenever the total is (`.or(...)`). */
    if (rf->original_heart_operator[0]) {
        f.original_heart_total = rf->original_heart_total;
        strncpy(f.original_heart_operator, rf->original_heart_operator,
                sizeof f.original_heart_operator - 1);
        f.has_original_heart_total = 1;
    }
    if (rf->exclude_group_names[0])
        strncpy(f.exclude_group_names, rf->exclude_group_names,
                sizeof f.exclude_group_names - 1);
    for (int i = 0; i < rf->n_groups && i < 8; i++)
        strncpy(f.groups[i], rf->groups[i], sizeof f.groups[i] - 1);
    f.n_groups = rf->n_groups > 8 ? 8 : rf->n_groups;
    f.cost_limit_min = rf->cost_limit_min;
    f.has_cost_limit_min = rf->has_cost_limit_min;
    f.cost_limit_max = rf->cost_limit_max;
    f.has_cost_limit_max = rf->has_cost_limit_max;
    /* util.rs:1489-1496 — exclude_cards is `Option<&[i16]>`; Rust rejects on
       exact id equality, so the whole RB_MAX_ZONE window is carried. */
    for (int i = 0; i < rf->n_exclude_cards && i < RB_MAX_ZONE; i++)
        f.exclude_cards[i] = rf->exclude_cards[i];
    f.n_exclude_cards = rf->n_exclude_cards > RB_MAX_ZONE ? RB_MAX_ZONE : rf->n_exclude_cards;
    /* util.rs:1498-1513 — exclude_names compares whitespace-stripped names for
       EQUALITY, which is str_eq_ignoring_ws in the matcher. */
    for (int i = 0; i < rf->n_exclude_names && i < 8; i++)
        strncpy(f.exclude_names[i], rf->exclude_names[i], sizeof f.exclude_names[i] - 1);
    f.n_exclude_names = rf->n_exclude_names > 8 ? 8 : rf->n_exclude_names;
    *out = f;
    return 1;
}

/* Mirror util.rs::matching_indices — return indices into cards where the
   filter matches. Rust enumerates `cards` and filters in place, so duplicate
   ids at different positions each produce their own index; the C port used to
   round-trip through matching_ids and map values back, which collapsed
   [A, B, A] into [0, 0]. */
int rb_matching_indices_filter(const RbCardFilter *rf, const int *cards, int n, int *out_idx, int max) {
    if (!cards || !out_idx) return 0;
    LocalCardFilter f;
    if (!local_filter_from_public(rf, &f)) {
        int r = 0;
        for (int i = 0; i < n && r < max; i++) out_idx[r++] = i;
        return r;
    }
    int r = 0;
    for (int i = 0; i < n && r < max; i++)
        if (card_filter_matches(&f, cards[i], 1)) out_idx[r++] = i;
    return r;
}

/* Mirror util.rs::matching_ids — return card IDs matching the filter.
 *
 * THE `has_filter` SHORT-CIRCUIT THAT WAS HERE IS GONE. It read
 * `if (!rf || !rf->has_filter || !local_filter_from_public(rf, &f))`, which
 * passes EVERY card through whenever `has_filter` is false. Rust's
 * matching_ids (util.rs:2065-2076) has no such arm: it is
 * `cards.iter().filter(|&&id| filter.matches(db, id, skip_empty)).copied()`
 * and nothing else, so an absent term is decided by `matches()` — which
 * rejects — never by a precomputed flag.
 *
 * The two disagree, because CardFilter::has_filter (util.rs:1461-1482) lists 21
 * fields and NOT `exclude_cards` (:1494-1496) or `exclude_names` (:1498-1513),
 * both of which `matches()` reads. `has_filter` is Rust's "could this reject a
 * card" flag for a CONSTRUCTED filter, and those two are filled programmatically
 * AFTER construction (util.rs:2089); it is not a precondition on matching.
 * rb_matching_indices_filter (:2125) never had the term and was already
 * Rust-equivalent, so the two C entry points for one Rust predicate disagreed.
 *
 * MEASURED, 2 639-card corpus, before the fix:
 *   empty filter            C 2639  Rust 2639  (agree — an all-None filter
 *                                               rejects nothing, which is also
 *                                               what the surviving pass-through
 *                                               below yields)
 *   exclude_cards = [0]     C 2639  Rust 2638  (DISAGREE by one card: C accepted
 *                                               the card Rust rejects)
 * That is one wrong card in a candidate pool, from a filter that looks empty.
 *
 * UNREACHABLE TODAY, measured: walking every effect of all 985 abilities through
 * rb_effect_filter_subset produces ZERO filters with has_filter false while
 * carrying exclude_cards / exclude_names / exclude_self, so no suite count moves
 * and none of the numbers in this batch's report are affected by this edit. It
 * is fixed because it is a wrong-value bug on a reachable public entry point
 * (rb_count_matching_filter, rb_build_candidate_pool and
 * rb_matching_ids_filtered all reach it), not because it is firing today.
 *
 * The surviving `!local_filter_from_public` arm is the real "nothing to match
 * with" case: that function returns 0 only for a NULL filter pointer. An
 * all-zero RbCardFilter still goes through local_filter_matches, which is
 * correct — it is exactly Rust's all-None CardFilter. */
int rb_matching_ids(const RbCardFilter *rf, const int *cards, int n, int *out, int max) {
    if (!cards || !out) return 0;
    LocalCardFilter f;
    if (!local_filter_from_public(rf, &f)) {
        int m = 0;
        for (int i = 0; i < n && m < max; i++) out[m++] = cards[i];
        return m;
    }
    int m = 0;
    for (int i = 0; i < n && m < max; i++) {
        if (card_filter_matches(&f, cards[i], 1)) out[m++] = cards[i];
    }
    return m;
}

/* Mirror util.rs::count_matching — count cards matching the filter. */
int rb_count_matching_filter(const RbCardFilter *f, const int *cards, int n) {
    int ids[RB_MAX_ZONE];
    int m = rb_matching_ids(f, cards, n, ids, RB_MAX_ZONE);
    return m;
}

/* Legacy matching_indices (card_type + group only, backward compatible). */
int rb_matching_indices(const int *cards, int n, const char *card_type,
                        const char *group, int *out_idx, int max) {
    RbCardFilter f;
    memset(&f, 0, sizeof f);
    if (card_type) strncpy(f.card_type, card_type, sizeof f.card_type - 1);
    if (group)     strncpy(f.group, group, sizeof f.group - 1);
    f.has_filter = (card_type && card_type[0]) || (group && group[0]);
    return rb_matching_indices_filter(&f, cards, n, out_idx, max);
}

/* Legacy count_matching (card_type + group only). */
int rb_count_matching(const int *cards, int n, const char *card_type, const char *group) {
    RbCardFilter f;
    memset(&f, 0, sizeof f);
    if (card_type) strncpy(f.card_type, card_type, sizeof f.card_type - 1);
    if (group)     strncpy(f.group, group, sizeof f.group - 1);
    f.has_filter = (card_type && card_type[0]) || (group && group[0]);
    return rb_count_matching_filter(&f, cards, n);
}

/* ── Distinct filtering ────────────────────────────────────────────────── */

/* Mirror util.rs::distinct_should_dedupe. */
int rb_distinct_should_dedupe(RbDistinctType d) {
    return d == RB_DISTINCT_CARDNAME || d == RB_DISTINCT_TRUE || d == RB_DISTINCT_DISTINCT;
}

/* Mirror util.rs::dedupe_by_normalized_name. */
int rb_dedupe_by_normalized_name(const int *items, int n, int *out, int max) {
    if (!items || !out) return 0;
    char seen[RB_MAX_ZONE][256];
    int  nseen = 0;
    int  m = 0;
    for (int i = 0; i < n; i++) {
        int cid = items[i];
        Card c;
        int found = 0;
        if (rb_decode_card_by_index((uint32_t)cid, &c)) {
            if (c.name) {
                char nm[256];
                rb_norm_ws(c.name, nm, sizeof nm);
                int dup = 0;
                for (int s = 0; s < nseen; s++)
                    if (!strcmp(seen[s], nm)) { dup = 1; break; }
                if (!dup) {
                    if (nseen < RB_MAX_ZONE) {
                        strncpy(seen[nseen], nm, 255); seen[nseen][255] = 0; nseen++;
                    }
                    found = 1;
                }
            } else {
                found = 1;
            }
            rb_free_card(&c);
        } else {
            found = 1;
        }
        if (found && m < max) out[m++] = items[i];
    }
    return m;
}

/* Mirror util.rs::apply_distinct_filter. */
int rb_apply_distinct_filter(const int *cards, int n, RbDistinctType d,
                             int *out, int max) {
    if (!rb_distinct_should_dedupe(d)) {
        int m = 0;
        for (int i = 0; i < n && m < max; i++) out[m++] = cards[i];
        return m;
    }
    return rb_dedupe_by_normalized_name(cards, n, out, max);
}

/* Mirror util.rs::count_distinct_member_name_units — Q278/Q279. */
int rb_count_distinct_member_name_units(const int *cards, int n) {
    char seen[RB_MAX_ZONE][256];
    int  nseen = 0;
    int  joints[RB_MAX_ZONE];
    int  njoints = 0;
    for (int i = 0; i < n && i < RB_MAX_ZONE; i++) {
        Card c;
        if (!rb_decode_card_by_index((uint32_t)cards[i], &c)) continue;
        if (c.name) {
            char nm[256];
            rb_norm_ws(c.name, nm, sizeof nm);
            if (strchr(nm, '&')) {
                if (njoints < RB_MAX_ZONE) joints[njoints++] = cards[i];
            } else {
                int dup = 0;
                for (int s = 0; s < nseen; s++)
                    if (!strcmp(seen[s], nm)) { dup = 1; break; }
                if (!dup && nseen < RB_MAX_ZONE) {
                    strncpy(seen[nseen], nm, 255); seen[nseen][255] = 0; nseen++;
                }
            }
        }
        rb_free_card(&c);
    }
    int count = nseen;
    for (int j = 0; j < njoints; j++) {
        Card c;
        if (!rb_decode_card_by_index((uint32_t)joints[j], &c)) continue;
        if (c.name) {
            char nm[256];
            rb_norm_ws(c.name, nm, sizeof nm);
            int has_new = 0;
            const char *p = nm;
            while (*p) {
                const char *sep = strchr(p, '&');
                size_t len = sep ? (size_t)(sep - p) : strlen(p);
                char tok[256];
                if (len >= sizeof tok) len = sizeof tok - 1;
                memcpy(tok, p, len); tok[len] = '\0';
                int in_set = 0;
                for (int s = 0; s < nseen; s++)
                    if (!strcmp(seen[s], tok)) { in_set = 1; break; }
                if (!in_set) {
                    if (nseen < RB_MAX_ZONE) {
                        strncpy(seen[nseen], tok, 255); seen[nseen][255] = 0; nseen++;
                    }
                    has_new = 1;
                }
                p = sep ? sep + 1 : p + strlen(p);
            }
            if (has_new) count++;
        }
        rb_free_card(&c);
    }
    return count;
}

/* Mirror util.rs::filter_distinct — return indices into cards, deduplicated by
   the normalized name of the card each index points at. Rust filters the INDEX
   list and then dedupes it, so two positions holding equal-name cards collapse
   to the first position; the C port used to round-trip through ids, which
   returned the first index of every distinct id. */
int rb_filter_distinct(const int *cards, int n, const char *card_type,
                       const char *group, RbDistinctType distinct,
                       int *out_idx, int max) {
    RbCardFilter f;
    memset(&f, 0, sizeof f);
    if (card_type) strncpy(f.card_type, card_type, sizeof f.card_type - 1);
    if (group)     strncpy(f.group, group, sizeof f.group - 1);
    f.has_filter = (card_type && card_type[0]) || (group && group[0]);
    int idxs[RB_MAX_ZONE];
    int mn = rb_matching_indices_filter(&f, cards, n, idxs, RB_MAX_ZONE);
    if (!rb_distinct_should_dedupe(distinct)) {
        int m = 0;
        for (int i = 0; i < mn && m < max; i++) out_idx[m++] = idxs[i];
        return m;
    }
    /* dedupe_by_normalized_name over the index list (util.rs:1871-1886). */
    char seen[RB_MAX_ZONE][256];
    int  nseen = 0;
    int  m = 0;
    for (int i = 0; i < mn && m < max; i++) {
        int cid = cards[idxs[i]];
        Card c;
        int keep;
        if (rb_decode_card_by_index((uint32_t)cid, &c)) {
            if (c.name) {
                char nm[256];
                rb_norm_ws(c.name, nm, sizeof nm);
                int dup = 0;
                for (int s = 0; s < nseen; s++)
                    if (!strcmp(seen[s], nm)) { dup = 1; break; }
                if (!dup && nseen < RB_MAX_ZONE) {
                    strncpy(seen[nseen], nm, 255); seen[nseen][255] = 0; nseen++;
                }
                keep = !dup;
            } else {
                keep = 1;
            }
            rb_free_card(&c);
        } else {
            keep = 1; /* card missing from the database is kept (util.rs:1880-1882) */
        }
        if (keep) out_idx[m++] = idxs[i];
    }
    return m;
}

/* Mirror util.rs::count_matching_distinct. The CardName variant is joint-aware
   (Q278/Q279): a multi-name card contributes one unit only when it introduces a
   name not already present as a single-name card. */
int rb_count_matching_distinct(const int *cards, int n, const char *card_type,
                               const char *group, RbDistinctType distinct) {
    RbCardFilter f;
    memset(&f, 0, sizeof f);
    if (card_type) strncpy(f.card_type, card_type, sizeof f.card_type - 1);
    if (group)     strncpy(f.group, group, sizeof f.group - 1);
    f.has_filter = (card_type && card_type[0]) || (group && group[0]);
    if (!rb_distinct_should_dedupe(distinct))
        return rb_count_matching_filter(&f, cards, n);
    int matching[RB_MAX_ZONE];
    int mn = rb_matching_indices_filter(&f, cards, n, matching, RB_MAX_ZONE);
    int ids[RB_MAX_ZONE];
    for (int i = 0; i < mn; i++) ids[i] = cards[matching[i]];
    if (distinct == RB_DISTINCT_CARDNAME)
        return rb_count_distinct_member_name_units(ids, mn);
    int deduped[RB_MAX_ZONE];
    return rb_apply_distinct_filter(ids, mn, distinct, deduped, RB_MAX_ZONE);
}

/* ── Candidate pool / selection helpers ────────────────────────────────── */

/* Mirror util.rs::build_candidate_pool — filter the given card list through
   the effect's CardFilter. */
int rb_build_candidate_pool(const GameState *g, int pl, const RbCardFilter *f,
                            int *out, int max) {
    if (!g || !out || max <= 0) return 0;
    int ids[RB_MAX_ZONE];
    int n = rb_zone_cards(g, pl, "hand", ids, RB_MAX_ZONE);
    return rb_matching_ids(f, ids, n, out, max);
}

/* Mirror util.rs::matching_ids_filtered. Rust order (util.rs:1766-1803):
   1. push exclude_ids into the filter (rejects by exact card id),
   2. matching_ids,
   3. when `distinct` is set, dedupe by name and THEN additionally drop
      results whose name matches an excluded card's name,
   4. truncate to target_count.
   The C port previously applied the name exclusion before the dedupe and never
   rejected by id. */
int rb_matching_ids_filtered(const RbCardFilter *f, const int *cards, int n,
                             RbDistinctType distinct, const int *exclude_ids,
                             int n_exclude, int target_count, int *out, int max) {
    if (!cards || !out) return 0;
    int m = rb_matching_ids(f, cards, n, out, max);
    /* 1. exclude_cards: exact-id rejection (util.rs:1167-1174). */
    if (n_exclude > 0) {
        int w = 0;
        for (int i = 0; i < m; i++) {
            int dropped = 0;
            for (int e = 0; e < n_exclude; e++)
                if (out[i] == exclude_ids[e]) { dropped = 1; break; }
            if (!dropped) out[w++] = out[i];
        }
        m = w;
    }
    if (rb_distinct_should_dedupe(distinct)) {
        /* 2. dedupe by name. */
        int deduped[RB_MAX_ZONE];
        int dn = rb_apply_distinct_filter(out, m, distinct, deduped, RB_MAX_ZONE);
        if (dn > max) dn = max;
        memcpy(out, deduped, sizeof(int) * (size_t)dn);
        m = dn;
        /* 3. additionally exclude by excluded card NAME. */
        if (n_exclude > 0) {
            char excl_names[RB_MAX_ZONE][256];
            int n_excl_names = 0;
            for (int e = 0; e < n_exclude && n_excl_names < RB_MAX_ZONE; e++) {
                Card c;
                if (rb_decode_card_by_index((uint32_t)exclude_ids[e], &c)) {
                    if (c.name) {
                        rb_norm_ws(c.name, excl_names[n_excl_names], 256);
                        n_excl_names++;
                    }
                    rb_free_card(&c);
                }
            }
            if (n_excl_names > 0) {
                int w = 0;
                for (int i = 0; i < m; i++) {
                    Card c;
                    int drop = 0;
                    if (rb_decode_card_by_index((uint32_t)out[i], &c)) {
                        if (c.name) {
                            char nm[256];
                            rb_norm_ws(c.name, nm, sizeof nm);
                            for (int s = 0; s < n_excl_names; s++)
                                if (!strcmp(nm, excl_names[s])) { drop = 1; break; }
                        }
                        rb_free_card(&c);
                    }
                    if (!drop) out[w++] = out[i];
                }
                m = w;
            }
        }
    }
    if (target_count > 0 && m > target_count) m = target_count;
    return m;
}

/* ── Selection primitives ──────────────────────────────────────────────── */

/* Mirror util.rs::classify_selection — returns 0=Skip, 1=Exact, 2=Prompt. */
int rb_classify_selection(const int *idxs, int n, int count, int is_all) {
    if (is_all) return 1;
    if (n < count) return 0;
    if (n > count) return 2;
    return 1;
}

/* Mirror util.rs::get_selection_indices — backward-compatible card_type+group version. */
int rb_get_selection_indices(const int *cards, int n, const char *card_type,
                             const char *group, int self_target_only,
                             int activating_card, int *out_idx, int max) {
    RbCardFilter f;
    memset(&f, 0, sizeof f);
    if (card_type) strncpy(f.card_type, card_type, sizeof f.card_type - 1);
    if (group)     strncpy(f.group, group, sizeof f.group - 1);
    f.has_filter = (card_type && card_type[0]) || (group && group[0]);
    return rb_get_selection_indices_filter(cards, n, &f, self_target_only,
                                            activating_card, out_idx, max);
}

/* New: RbCardFilter-based get_selection_indices (full filter support). */
int rb_get_selection_indices_filter(const int *cards, int n, const RbCardFilter *filter,
                                     int self_target_only, int activating_card,
                                     int *out_idx, int max) {
    if (!cards || !out_idx) return 0;
    int r = rb_matching_indices_filter(filter, cards, n, out_idx, max);
    if (self_target_only && activating_card >= 0) {
        int w = 0;
        for (int i = 0; i < r; i++)
            if (cards[out_idx[i]] == activating_card)
                out_idx[w++] = out_idx[i];
        r = w;
    }
    return r;
}

/* Mirror util.rs::resolve_selection — backward-compatible card_type+group version. */
int rb_resolve_selection(const int *cards, int n, const char *card_type,
                         const char *group, int count, int is_all,
                         int self_target_only, int activating_card) {
    RbCardFilter f;
    memset(&f, 0, sizeof f);
    if (card_type) strncpy(f.card_type, card_type, sizeof f.card_type - 1);
    if (group)     strncpy(f.group, group, sizeof f.group - 1);
    f.has_filter = (card_type && card_type[0]) || (group && group[0]);
    return rb_resolve_selection_filter(cards, n, &f, count, is_all,
                                        self_target_only, activating_card);
}

/* New: RbCardFilter-based resolve_selection (full filter support). */
int rb_resolve_selection_filter(const int *cards, int n, const RbCardFilter *filter,
                                 int count, int is_all, int self_target_only,
                                 int activating_card) {
    int idxs[RB_MAX_ZONE];
    int mn = rb_get_selection_indices_filter(cards, n, filter, self_target_only,
                                               activating_card, idxs, RB_MAX_ZONE);
    return rb_classify_selection(idxs, mn, count, is_all);
}

/* Mirror util.rs::zone_remove_at_indices — remove from zone by indices (descending). */
int rb_zone_remove_at_indices(GameState *g, int pl, const char *zone,
                              const int *indices, int n_indices) {
    if (!g || !zone || !indices || n_indices <= 0) return 0;
    int sorted[RB_MAX_ZONE];
    int sn = 0;
    for (int i = 0; i < n_indices && sn < RB_MAX_ZONE; i++) sorted[sn++] = indices[i];
    for (int i = 0; i < sn - 1; i++)
        for (int j = i + 1; j < sn; j++)
            if (sorted[j] > sorted[i]) { int t = sorted[i]; sorted[i] = sorted[j]; sorted[j] = t; }
    int removed = 0;
    RbPlayer *P = &g->p[pl];
    for (int i = 0; i < sn; i++) {
        int idx = sorted[i];
        if (!strcmp(zone, "hand")) {
            if (idx >= 0 && idx < P->hand.n) {
                for (int k = idx; k < P->hand.n - 1; k++) P->hand.cards[k] = P->hand.cards[k + 1];
                P->hand.n--; removed++;
            }
        } else if (!strcmp(zone, "discard") || !strcmp(zone, "waitroom")) {
            if (idx >= 0 && idx < P->discard.n) {
                for (int k = idx; k < P->discard.n - 1; k++) P->discard.cards[k] = P->discard.cards[k + 1];
                P->discard.n--; removed++;
            }
        } else if (!strcmp(zone, "energy")) {
            if (idx >= 0 && idx < P->energy.n) {
                for (int k = idx; k < P->energy.n - 1; k++) P->energy.cards[k] = P->energy.cards[k + 1];
                P->energy.n--; removed++;
            }
        } else if (!strcmp(zone, "live") || !strcmp(zone, "live_card_zone")) {
            if (idx >= 0 && idx < P->live.n) {
                for (int k = idx; k < P->live.n - 1; k++) P->live.cards[k] = P->live.cards[k + 1];
                P->live.n--; removed++;
            }
        } else if (!strcmp(zone, "success") || !strcmp(zone, "success_live_zone") ||
                   !strcmp(zone, "success_live_card_zone")) {
            if (idx >= 0 && idx < P->success.n) {
                for (int k = idx; k < P->success.n - 1; k++) P->success.cards[k] = P->success.cards[k + 1];
                P->success.n--; removed++;
            }
        }
    }
    return removed;
}

/* ── Cost-reduction helpers ────────────────────────────────────────────── */

/* Mirror util.rs::find_modify_cost — search the effect tree for ModifyCost. */
const AbilityEffect *rb_find_modify_cost(const AbilityEffect *effect,
                                          const char *op, const char *loc) {
    if (!effect) return NULL;
    if (!strcmp(effect->action ? effect->action : "", "modify_cost")) {
        if (op) {
            const char *eff_op = eff_extra(effect, "operation");
            if (!eff_op || strcmp(eff_op, op)) return NULL;
        }
        if (loc) {
            const char *eff_loc = eff_extra(effect, "location");
            if (!eff_loc || strcmp(eff_loc, loc)) return NULL;
        }
        return effect;
    }
    for (int i = 0; i < effect->n_child; i++) {
        const AbilityEffect *found = rb_find_modify_cost(effect->child[i], op, loc);
        if (found) return found;
    }
    if (effect->primary_effect) {
        const AbilityEffect *found = rb_find_modify_cost(effect->primary_effect, op, loc);
        if (found) return found;
    }
    if (effect->alternative_effect) {
        const AbilityEffect *found = rb_find_modify_cost(effect->alternative_effect, op, loc);
        if (found) return found;
    }
    if (effect->look_action) {
        const AbilityEffect *found = rb_find_modify_cost(effect->look_action, op, loc);
        if (found) return found;
    }
    if (effect->select_action) {
        const AbilityEffect *found = rb_find_modify_cost(effect->select_action, op, loc);
        if (found) return found;
    }
    if (effect->followup_action) {
        const AbilityEffect *found = rb_find_modify_cost(effect->followup_action, op, loc);
        if (found) return found;
    }
    return NULL;
}

/* Mirror util.rs::play_cost_reduction_matches.
   util.rs:193-200 gates on `group_names.first()`, and the wire key is
   `group_names` — this read `group`, which does not exist on any effect in the
   card database, so the whole group gate was dead (every card passed it). */
static int rb_play_cost_reduction_matches(const AbilityEffect *effect, int card_id,
                                          const Card *card) {
    char gbuf[128];
    if (!rb_card_matches_effect_group(effect, card_id, gbuf, sizeof gbuf)) return 0;
    const char *cost_limit = eff_extra(effect, "cost_limit");
    if (cost_limit) {
        int limit = atoi(cost_limit);
        if (card->cost != limit) return 0;
    }
    if (!rb_cost_threshold_met(card, effect)) return 0;
    const char *ct = eff_extra(effect, "card_type");
    if (ct && strcmp(ct, "member_card")) return 0;
    const char *af = eff_extra(effect, "ability_filter");
    if (af && !strcmp(af, "no_ability")) {
        if (card->ability) return 0;
    }
    return 1;
}

/* Count stage cards matching a group name. util.rs:242 filters the stage by
   `card_matches_group_str(card_db, id, group_name)` with
   `group_name() == group_names.first()`. */
static int rb_stage_count_group(const RbPlayer *P, const char *group_names) {
    char gbuf[128];
    const char *g = rb_group_name_first(group_names, gbuf, sizeof gbuf);
    if (!g || !*g) return 0;
    int count = 0;
    for (int i = 0; i < RB_STAGE_SIZE; i++) {
        int cid = P->stage[i];
        if (cid == RB_EMPTY_SLOT) continue;
        if (rb_card_matches_group_str(cid, g)) count++;
    }
    return count;
}

/* Mirror util.rs::per_unit_cost_reduction. */
static int rb_per_unit_cost_reduction(const AbilityEffect *effect,
                                       const RbPlayer *P, int hand_count) {
    const char *pul  = eff_extra(effect, "per_unit_location");
    const char *loc  = eff_extra(effect, "location");
    const char *count_zone = pul ? pul : loc;
    if (!count_zone) count_zone = "hand";
    int raw_count;
    /* util.rs:236 `count_zone == "stage" && effect.group_names_any().is_some()`
       — a PRESENCE test on the LIST, then the stage is filtered by
       `group_name()` = `group_names.first()`. This read the extra key `group`,
       which no effect in the card database carries, so the branch was dead and
       every stage+group per-unit cost reduction counted the whole stage. */
    const char *group_names = eff_extra(effect, "group_names");
    if (!group_names || !*group_names) group_names = eff_extra(effect, "group");
    if (!strcmp(count_zone, "stage") && group_names && *group_names) {
        raw_count = rb_stage_count_group(P, group_names);
    } else {
        raw_count = hand_count;
    }
    /* util.rs:248 `effect.per_unit_count_any().unwrap_or(1).max(1)`. The VM
       (vm.c:1040) fills ONLY the first-class field and never the extra, so
       reading the extra made every 「N枚につき」 a 「1枚につき」. Field first,
       matching `filter_u8_getter!` (card.rs:1800-1805), with the extra as the
       fallback for an effect synthesised outside the decoder. */
    int per_unit_count = effect->per_unit_count > 0 ? effect->per_unit_count : 1;
    const char *puc = eff_extra(effect, "per_unit_count");
    if (effect->per_unit_count <= 0 && puc && *puc) per_unit_count = atoi(puc);
    if (per_unit_count < 1) per_unit_count = 1;
    int exclude_self = 0;
    const char *es = eff_extra(effect, "exclude_self");
    if (es && !strcmp(es, "true")) exclude_self = 1;
    int effective = exclude_self ? (raw_count > 0 ? raw_count - 1 : 0) : raw_count;
    const char *val = eff_extra(effect, "value");
    int value = val ? atoi(val) : 1;
    return (effective / per_unit_count) * value;
}

/* Mirror util.rs::scan_abilities_for_cost_reduction. */
static int rb_scan_abilities_for_cost_reduction(uint32_t card_idx,
                                                 int target_id, const Card *target_card,
                                                 const GameState *g, int pl,
                                                 int hand_count, int hand_condition_guard) {
    int n_abilities = rb_card_num_abilities(card_idx);
    for (int i = 0; i < n_abilities; i++) {
        Ability ab;
        if (!rb_decode_card_ability(card_idx, i, &ab)) continue;
        int r = -1;
        if (ab.effect) {
            const AbilityEffect *mc = rb_find_modify_cost(ab.effect, "subtract", "hand");
            if (mc && rb_play_cost_reduction_matches(mc, target_id, target_card)) {
                if (hand_condition_guard) {
                    const AbilityEffect *cond_eff = mc->condition
                        ? (const AbilityEffect *)mc->condition : NULL;
                    /* Skip if the effect's condition requires location == hand
                       (the aura card is on stage, not in hand). */
                    (void)cond_eff;
                }
                if (mc->per_unit)
                    r = rb_per_unit_cost_reduction(mc, &g->p[pl], hand_count);
                else {
                    const char *val = eff_extra(mc, "value");
                    r = val ? atoi(val) : 1;
                }
            }
        }
        rb_free_ability(&ab);
        if (r >= 0) return r;
    }
    return 0;
}

/* Mirror util.rs::calculate_play_cost_reduction. */
int rb_calculate_play_cost_reduction(const GameState *g, int pl, int hand_count,
                                      int card_id) {
    Card card;
    if (!rb_decode_card_by_index((uint32_t)card_id, &card)) return 0;
    int reduction = 0;
    /* 1. Self-reduction from own abilities */
    {
        int n_abilities = rb_card_num_abilities((uint32_t)card_id);
        for (int i = 0; i < n_abilities; i++) {
            Ability ab;
            if (!rb_decode_card_ability((uint32_t)card_id, i, &ab)) continue;
            if (ab.effect) {
                const AbilityEffect *mc = rb_find_modify_cost(ab.effect, "subtract", "hand");
                if (mc && rb_play_cost_reduction_matches(mc, card_id, &card)) {
                    if (mc->per_unit)
                        reduction = rb_per_unit_cost_reduction(mc, &g->p[pl], hand_count);
                    else {
                        const char *val = eff_extra(mc, "value");
                        reduction = val ? atoi(val) : 1;
                    }
                }
            }
            rb_free_ability(&ab);
        }
    }
    /* 2. Stage card auras */
    const RbPlayer *P = &g->p[pl];
    for (int s = 0; s < RB_STAGE_SIZE; s++) {
        int stage_id = P->stage[s];
        if (stage_id == RB_EMPTY_SLOT) continue;
        int r = rb_scan_abilities_for_cost_reduction((uint32_t)stage_id,
                                                      card_id, &card, g, pl,
                                                      hand_count, 1);
        if (r > 0) reduction += r;
    }
    /* 3. Success live card auras */
    if (reduction == 0) {
        for (int i = 0; i < P->success.n; i++) {
            int live_id = P->success.cards[i];
            int r = rb_scan_abilities_for_cost_reduction((uint32_t)live_id,
                                                           card_id, &card, g, pl,
                                                           hand_count, 0);
            if (r > 0) { reduction = r; break; }
        }
    }
    rb_free_card(&card);
    return reduction;
}

/* ── Per-unit zone table, card tally and score sum ────────────────────────
 *
 * ONE implementation, three callers. `rb_resolve_per_unit_count` is the public
 * (card_type, group) entry point; `rb_constant_per_unit_units` below holds a
 * whole RbCardFilter built by rb_effect_filter_subset. Rust has the same shape:
 * `resolve_per_unit_count` / `resolve_per_unit_score_sum` take `&CardFilter`,
 * and hearts.rs:60-88 is just one caller of them. The two C callers used to
 * carry two private zone tables, which is how the two drifted apart.
 */

/* Mirror util.rs::per_unit_zone (util.rs:2660-2677): which zone a per-unit
   clause counts over. NULL means the clause names no zone this engine can
   count; the tally caller then reads the count as 1 (util.rs:2633-2636) and the
   score-sum caller as 0 (util.rs:2694-2696), exactly as Rust's two callers do.
   `枚` is the one clause that is not a fixed zone: it is the HAND unless the
   filter restricts the clause to member cards, in which case it is the
   under-member zone (util.rs:2665-2671). */
static const char *per_unit_zone_name(const char *per_unit_type, const RbCardFilter *f) {
    if (!per_unit_type) return NULL;
    if (!strcmp(per_unit_type, "stage") || !strcmp(per_unit_type, "member") ||
        !strcmp(per_unit_type, "人") || !strcmp(per_unit_type, "members"))
        return "stage";
    if (!strcmp(per_unit_type, "hand") || !strcmp(per_unit_type, "card")) return "hand";
    if (!strcmp(per_unit_type, "under_member")) return "under_member";
    if (!strcmp(per_unit_type, "枚"))
        return (f && f->card_type[0] && !strcmp(f->card_type, "member_card"))
                   ? "under_member" : "hand";
    if (!strcmp(per_unit_type, "discard")) return "discard";
    if (!strcmp(per_unit_type, "live_card_zone")) return "live";
    if (!strcmp(per_unit_type, "success_live_zone") ||
        !strcmp(per_unit_type, "success_live_card_zone"))
        return "success";
    return NULL;
}

/* The cards a per-unit clause ranges over, in Rust's order and Rust's scoping.
   util.rs:2734-2748 `under_member_cards`: 「このメンバーの下に置かれているカード」
   counts ONE member's under-cards — the host's own slot when the host is on
   stage, the whole field only when the host is unknown. Walking every bag
   multiplied the gain by every member on stage: test_parity_hearts_constants
   reported 2 for "1 other A-RISE member grants exactly 1 heart05", 3 for "two
   others -> +2", and 2 for "energy under a DIFFERENT area plus a member under
   center -> still 0". Same shape and same fix as dynamic_count.c's per-unit arm.
   util.rs:2643-2653: the wait/active state filter is STAGE-ONLY. */
static int per_unit_zone_cards(const GameState *g, int pl, const char *zone,
                               const char *state_filter, int host_card_id,
                               int *ids, int max) {
    const RbPlayer *P = &g->p[pl];
    if (!strcmp(zone, "under_member")) {
        int area = -1;
        if (host_card_id >= 0)
            for (int s = 0; s < RB_STAGE_SIZE; s++)
                if (P->stage[s] == host_card_id) { area = s; break; }
        int lo = (area >= 0) ? area : 0;
        int hi = (area >= 0) ? area + 1 : RB_STAGE_SIZE;
        int n = 0;
        for (int s = lo; s < hi; s++)
            for (int i = 0; i < P->under_cards[s].n && n < max; i++)
                ids[n++] = P->under_cards[s].cards[i];
        return n;
    }
    if (!strcmp(zone, "stage")) {
        int n = 0;
        for (int i = 0; i < RB_STAGE_SIZE && n < max; i++) {
            int cid = P->stage[i];
            if (cid == RB_EMPTY_SLOT) continue;
            if (state_filter) {
                const char *ori = rb_mods_get_orientation((RbMods *)&g->mods, cid);
                if (!rb_orientation_matches_state(ori, state_filter)) continue;
            }
            ids[n++] = cid;
        }
        return n;
    }
    return rb_zone_cards(g, pl, zone, ids, max);
}

/* Mirror util.rs::resolve_per_unit_count (util.rs:2592-2656) with
   util.rs::count_filtered (util.rs:2753-2773) for a caller that holds a whole
   CardFilter. `per_unit_count` deliberately does NOT divide here: Rust returns
   the card TALLY and each CALLER applies the divisor (hearts.rs:94 for the
   constant units, util.rs:248 for the per-unit cost reduction, score.rs:182 for
   the score modifier), so a divisor added at this level would double-divide
   every 「N枚につき」. */
static int per_unit_count_filtered(const GameState *g, int pl, const char *per_unit_type,
                                   const RbCardFilter *f, const char *state_filter,
                                   int host_card_id) {
    if (!per_unit_type) return 1;
    /* util.rs:2607-2631 — `per_unit_type == "heart_colors"` counts the DISTINCT
       heart colours the matching STAGE cards carry, reading each card's
       base_heart / need_heart and never its blade map. BOTH Rust call sites hand
       this function an EMPTY colour list (hearts.rs:83 `&[]`,
       effects/resource.rs:521 `&[]`), because the colours describe the GAIN, not
       the clause; an empty list inserts nothing, so `colors_found` is always
       empty and the arm can only ever report 0. Same computation, loop elided —
       which is why this is 0 and not the 1 an unknown zone would read as. */
    if (!strcmp(per_unit_type, "heart_colors")) return 0;
    const char *zone = per_unit_zone_name(per_unit_type, f);
    if (!zone) return 1;
    int ids[RB_MAX_ZONE];
    int n = per_unit_zone_cards(g, pl, zone, state_filter, host_card_id, ids, RB_MAX_ZONE);
    /* util.rs:2771 `apply_distinct_filter` — a SET-level dedupe by normalized
       name (util.rs:2078-2092 count_matching_distinct), which the per-card
       predicate cannot express: 「名前の異なるメンバーカード1枚につき」. */
    if (rb_distinct_should_dedupe((RbDistinctType)f->distinct)) {
        int kept[RB_MAX_ZONE];
        int m = rb_matching_ids(f, ids, n, kept, RB_MAX_ZONE);
        return rb_dedupe_by_normalized_name(kept, m, ids, RB_MAX_ZONE);
    }
    return rb_count_matching_filter(f, ids, n);
}

/* Mirror util.rs::resolve_per_unit_score_sum (util.rs:2685-2726) — the SUM of
   the PRINTED scores of the cards the clause matches, for
   「…のスコアの合計Nにつき」. A different quantity from the card tally: four
   score-1 lives are 4 cards but only 4 points, and 「合計5につき」 must not fire on
   them, so counting cards there granted a blade 「合計5につき」 that the card text
   never promised. Same zone resolution, same filter and the same host scoping as
   the tally above, so the two always agree on WHICH cards are in scope
   (hearts.rs:64-65 says so in as many words). Neither a distinct dedupe nor a
   heart-colour gate applies here: util.rs:2713-2722 filters with the bare
   `filter.matches` and sums `Option<u8>` scores. */
static int per_unit_score_sum(const GameState *g, int pl, const char *per_unit_type,
                              const RbCardFilter *f, const char *state_filter,
                              int host_card_id) {
    const char *zone = per_unit_zone_name(per_unit_type, f);
    if (!zone) return 0;
    int ids[RB_MAX_ZONE];
    int n = per_unit_zone_cards(g, pl, zone, state_filter, host_card_id, ids, RB_MAX_ZONE);
    int kept[RB_MAX_ZONE];
    int m = rb_matching_ids(f, ids, n, kept, RB_MAX_ZONE);
    long total = 0;
    for (int i = 0; i < m; i++) {
        Card c;
        /* util.rs:2718-2722 `filter_map(get_card(cid).and_then(|c| c.score))` — a
           card the database cannot decode, or one that prints no score, adds
           nothing at all. */
        if (!rb_decode_card_by_index((uint32_t)kept[i], &c)) continue;
        total += c.score;
        rb_free_card(&c);
    }
    /* util.rs:2723-2725 — a score total across a live's worth of cards cannot
       overflow a u8, but clamp rather than wrap: a wrapped small number would
       silently grant fewer blades. */
    if (total > 255) total = 255;
    return (int)total;
}

/* Mirror util.rs::constant_per_unit_units (hearts.rs:52-114) — the units part of
   a constant per_unit gain (before base). `zone` is `location.or(per_unit_type)
   .unwrap_or(hand)`; the count itself is the shared tally / score sum above. */
int rb_constant_per_unit_units(const AbilityEffect *effect, const GameState *g, int pl,
                               int host_card_id) {
    /* hearts.rs:59 — `constant_per_unit_zone`, i.e. `location.or(per_unit_type)
       .unwrap_or(hand)`. The hand-rolled zone dispatch this replaced keyed off
       that string directly, and matched no arm for 「枚」 (the spelling a clause
       that names no `location` arrives as), so every such 「N枚につき」 fell through
       to an unknown-zone count and reported 0 units. `per_unit_zone` is Rust's
       own spelling table for that string. */
    const char *zone = rb_constant_per_unit_zone(effect);

    /* hearts.rs:60-61 — `effect.filter_subset()` then `filter.exclude_self =
       Some(host_card_id)`. The hand-rolled build this replaces read exactly two
       effect fields and stopped, so the count silently ignored heart_colors,
       require_all_heart_colors, heart_color_count, need_heart_total,
       need_heart_color, card_property, distinct, cost_limit, cost_values,
       cost_total, characters, exclude_characters, name_fragments,
       original_blade_limit, blade_limit, ability_filter, ability_filter_triggers
       and or_ability_filters — all of which CardFilter::matches (util.rs:1676-1790)
       consults, and several of which real per_unit effects carry. Measured over
       cards/abilities.json `unique_abilities`: 17 of the 53 per_unit effects
       carry one of distinct / heart_colors / card_property, e.g.
       PL!N-bp7-003 ab#1 `distinct: card_name`, PL!HS-bp2-020-L ab#1
       `distinct: card_name`, PL!-pb2-004 ab#0 and ab#1
       `card_property: has_score_icon`, PL!SP-pb2-008 ab#0
       `card_property: has_blade_heart`, PL!S-sd1-001-SD ab#0
       `heart_colors: [heart02]` on a bare 「1枚につき」. The shared builder also
       reads `card_type_field` before the extra and takes `.first()` of
       group_names (util.rs:1821-1824), so those two facts stop being
       hand-maintained here. */
    RbCardFilter f;
    rb_effect_filter_subset(effect, &f);
    f.exclude_self_id = host_card_id;
    f.has_exclude_self = (host_card_id >= 0);
    /* `has_filter` was computed before exclude_self was overwritten, and it is
       the flag rb_matching_ids short-circuits on. Rust applies check_exclude_self
       (util.rs:1680) whatever else the filter holds, so an effect that names no
       other qualifier must still not count its own host. */
    if (f.has_exclude_self) f.has_filter = 1;

    /* hearts.rs:66-88 — 「…のスコアの合計Nにつき」 counts POINTS; everything else
       counts CARDS. The C had no score arm at all, so the one corpus effect that
       needs it (PL!-pb2-030-N ab#0, `per_unit_aggregate: "score"`,
       `per_unit_count: 5`, `per_unit_type: "card"`, `location:
       "success_live_zone"`) divided a card tally by 5. */
    const char *aggregate = eff_extra(effect, "per_unit_aggregate");
    const char *state = eff_extra(effect, "state");
    int per_count = (aggregate && !strcmp(aggregate, "score"))
                        ? per_unit_score_sum(g, pl, zone, &f, state, host_card_id)
                        : per_unit_count_filtered(g, pl, zone, &f, state, host_card_id);

    /* hearts.rs:94 `per_count / effect.per_unit_count_any().unwrap_or(1).max(1)`
       — the FIELD is the only place the decoder puts it (vm.c:1040), so the
       extra-only read below always yielded 0 and then 1, turning every
       「N枚につき」 into 「1枚につき」. 24 corpus effects carry =1, 4 carry =2,
       1 carries =4. */
    int per_unit_count = effect->per_unit_count > 0 ? effect->per_unit_count : 1;
    const char *puc = eff_extra(effect, "per_unit_count");
    if (effect->per_unit_count <= 0 && puc && *puc) per_unit_count = atoi(puc);
    if (per_unit_count < 1) per_unit_count = 1;
    int units = per_count / per_unit_count;
    /* hearts.rs:96-100 -- `if effect.max.unwrap_or(false) { if let Some(cap) =
       effect.count_any() { units = units.min(cap) } }`. `count_any()`
       (core/card.rs:2411-2418) is `self.count.or_else(kind.filter().count)`, so
       the FIRST source Rust consults is the typed `count` field -- exactly
       `AbilityEffect::count`, which vm.c:741-744 fills from the `count` key when
       the tag is I64.
     *
     * This read `eff_extra(effect, "count")`, and that key is NEVER POPULATED:
     * vm.c's `count` branch assigns the struct field and `continue`s, so no
     * extras row is ever created for it. Measured over all 985 abilities: of the
     * 25 effects whose `max` extra is "true", 25 carry a non-negative
     * `AbilityEffect::count` and 0 carry a `count` extra. The cap was therefore
     * dead on every one of them, and the C returned an UNCAPPED unit count
     * wherever Rust returns `units.min(cap)`.
     *
     * The `max` GATE is unchanged and stays: `max` is a genuine extra (populated
     * on 25 effects) because vm.c:839-863 keeps the optional / conditional /
     * conditional_negation / is_further / max family as stringified extras as
     * well as struct flags. Only the cap's VALUE SOURCE moves, from a key that
     * cannot exist to the field Rust reads first.
     *
     * Rust's SECOND source, `kind.filter().count`, stays unreachable: the C does
     * not decode the nested filter object's fields into extras at all, which is
     * a vm.c gap and not this one. See the extras-key census note in
     * src/core/data.c for the full measured list of keys that reach the wire and
     * are dropped at decode. */
    const char *max_str = eff_extra(effect, "max");
    if (max_str && !strcmp(max_str, "true") && effect->count >= 0) {
        int cap = effect->count;
        if (units > cap) units = cap;
    }
    return units;
}

/* ── Per-unit calculation ──────────────────────────────────────────────── */

/* Mirror util.rs::calculate_per_unit_multiplier. */
int rb_calculate_per_unit_multiplier(const GameState *g, int pl, const char *per_unit_type,
                                      const char *state_filter) {
    if (!per_unit_type) return 1;
    const RbPlayer *P = &g->p[pl];
    if (!strcmp(per_unit_type, "member") || !strcmp(per_unit_type, "人") ||
        !strcmp(per_unit_type, "members")) {
        int count = 0;
        for (int i = 0; i < RB_STAGE_SIZE; i++) {
            int cid = P->stage[i];
            if (cid == RB_EMPTY_SLOT) continue;
            if (state_filter) {
                const char *ori = rb_mods_get_orientation((RbMods *)&g->mods, cid);
                if (!rb_orientation_matches_state(ori, state_filter)) continue;
            }
            count++;
        }
        return count;
    }
    if (!strcmp(per_unit_type, "hand") || !strcmp(per_unit_type, "card") ||
        !strcmp(per_unit_type, "枚")) {
        return P->hand.n;
    }
    if (!strcmp(per_unit_type, "energy")) return P->energy.n;
    if (!strcmp(per_unit_type, "live_card_zone")) return P->live.n;
    if (!strcmp(per_unit_type, "discard")) return P->discard.n;
    if (!strcmp(per_unit_type, "under_member") || !strcmp(per_unit_type, "下")) {
        int n = 0;
        for (int s = 0; s < RB_STAGE_SIZE; s++) n += P->under_cards[s].n;
        return n;
    }
    return 1;
}

/* Mirror util.rs::resolve_per_unit_count.
   Zone selection (util.rs:2660-2677, `per_unit_zone`): note that the bare 「枚」
   unit resolves to the HAND unless the filter is restricted to member cards, in
   which case it resolves to the under-member zone. The C port sent 「枚」 straight
   to under_member, which inverted the count for the common 「1枚につき」 gains.
   Both now live in `per_unit_zone_name` / `per_unit_zone_cards` above, shared
   with rb_constant_per_unit_units, because Rust has one `per_unit_zone` and one
   `resolve_per_unit_count` behind both entry points.

   AUDITED AGAINST RUST, NOT CHANGED: `per_unit_count` does NOT belong here.
   util.rs:2592-2656 `resolve_per_unit_count` returns the CARD TALLY
   (`count_filtered` -> `count_matching_distinct`); the `per_unit_count` divisor
   is applied by each CALLER — hearts.rs:94 for the constant per-unit units,
   util.rs:248 for the per-unit cost reduction, score.rs:182 for the score
   modifier. Dividing here as well would double-divide every 「N枚につき」.
   Likewise the zone loops below count CARDS, so no divisor may be added.

   The `group` argument arrives from the callers' `group_names` extra and may
   still be the whole CSV list; util.rs's `count_filtered` filters through a
   CardFilter whose group is `group_names.first()` (card.rs:2397), which is what
   the shared matcher's `rb_group_name_first` split does.

   This entry point's contract is the (card_type, group) PAIR only — Rust's
   callers that reach for a filter hold a real CardFilter and go through
   rb_constant_per_unit_units instead. So the filter built here carries no
   distinct / heart_colors / card_property / exclude_self, and the behaviour of
   this function is unchanged by the refactor. */
int rb_resolve_per_unit_count(const GameState *g, int pl, const char *per_unit_type,
                              const char *card_type, const char *group,
                              const char *state_filter, int host_card_id) {
    if (!per_unit_type) return 1;
    /* score.rs:232-234 / misc.rs:832-834 — 「1つにつき」 on a cost that pays ENERGY
       counts the energy paid by the most recent cost step, i.e.
       GameModifiers::last_cost_energy_count (game_modifiers.rs:183), not a zone.
       Until that field reaches RbMods the accessor macro yields the pre-field
       behaviour (0). Rust's callers test `per_unit_type == Some("つ")` BEFORE
       calling resolve_per_unit_count (effects/resource.rs:512-525), so this arm
       is that test, and it has to stay outside the shared tally — where the unit
       has no zone and would read as 1. */
    if (!strcmp(per_unit_type, "つ")) return RB_LAST_COST_ENERGY((RbMods *)&g->mods);
    /* The alias spellings this entry point has always accepted, folded onto the
       canonical names `per_unit_zone_name` knows. They are C-side conveniences:
       util.rs's own callers name the canonical enum (`Zone::Waitroom`,
       `Zone::LiveCardZone`) and never these strings, and no per_unit effect in
       cards/abilities.json uses one. Kept so the two existing callers
       (effects/state.c:1308, effects/rules.c:1627), which hand this function a
       raw `per_unit_type` extra, keep the contract they were written against. */
    if (!strcmp(per_unit_type, "waitroom"))      per_unit_type = "discard";
    else if (!strcmp(per_unit_type, "under"))    per_unit_type = "under_member";
    else if (!strcmp(per_unit_type, "live"))      per_unit_type = "live_card_zone";
    RbCardFilter f;
    memset(&f, 0, sizeof f);
    if (card_type) strncpy(f.card_type, card_type, sizeof f.card_type - 1);
    if (group)     strncpy(f.group, group, sizeof f.group - 1);
    f.has_filter = (card_type && card_type[0]) || (group && group[0]);
    return per_unit_count_filtered(g, pl, per_unit_type, &f, state_filter, host_card_id);
}

/* Mirror util.rs::resolve_discard_per_unit_count. */
int rb_resolve_discard_per_unit_count(const GameState *g, int last_discard_count,
                                      const char *card_type, const char *group) {
    if (g->n_recently_moved > 0) {
        return rb_count_matching(g->recently_moved, g->n_recently_moved, card_type, group);
    }
    return last_discard_count;
}

/* ── Distinct name search (ported) ─────────────────────────────────────── */

/* Mirror util.rs::max_distinct_names — simplified greedy in C. */
int rb_max_distinct_names(const int *cards, int n) {
    if (n <= 0) return 0;
    char seen[RB_MAX_ZONE][256];
    int  nseen = 0;
    for (int i = 0; i < n && i < RB_MAX_ZONE; i++) {
        Card c;
        if (!rb_decode_card_by_index((uint32_t)cards[i], &c)) continue;
        if (c.name) {
            char nm[256];
            rb_norm_ws(c.name, nm, sizeof nm);
            int dup = 0;
            for (int s = 0; s < nseen; s++)
                if (!strcmp(seen[s], nm)) { dup = 1; break; }
            if (!dup) {
                if (nseen < RB_MAX_ZONE) {
                    strncpy(seen[nseen], nm, 255); seen[nseen][255] = 0; nseen++;
                }
            }
        }
        rb_free_card(&c);
    }
    return nseen;
}

/* ── TOMBSTONE (2026-10-04). `rb_max_distinct_names_greedy` REMOVED.
 *
 * A census classified this DEAD-WRONG: "Rust returns
 * DistinctNamesResult{distinct, collision}; the C returns only the count and
 * silently DROPS had_collision. The C comment says 'delegates to greedy',
 * which is true and is exactly the problem - the delegate has two outputs."
 * The mechanical observation is correct and is why this function is gone. But
 * the interesting part is what it points at, and it is NOT here.
 *
 * This function delegated to `rb_max_distinct_names` (util.c:2917). That is
 * DEAD-ADJACENT but NOT dead and NOT production: it has no caller anywhere under
 * src/ -- its only callers are tests/test_game.c:1276, reached through a
 * hand-written `extern` at test_game.c:1273. So the dropped `had_collision` was
 * unobservable in production even in principle.
 *
 * THE REAL GAP, reported rather than fixed here: Rust READS that flag on a live
 * path, and the C tree has no port of it at all. engine/src/core/card.rs:680-687:
 *
 *     let best = util::max_distinct_names(&name_sets);
 *     if let Some(n) = distinct_only { best.distinct >= n }
 *     else                          { !best.collision }
 *
 * The `else` arm -- "does a collision-free assignment exist" -- has NO C
 * counterpart, because `rb_max_distinct_names` returns a bare int and its only
 * callers are tests. This is a missing port on a live Rust path, not a latent
 * bug in a dead wrapper, and it is worth more attention than the wrapper was.
 * It is NOT fixed here: giving `rb_max_distinct_names` a second output means
 * changing a function tests already hand-declare, and the caller that needs the
 * `else` arm does not exist yet. Wiring it belongs with whoever ports the
 * card.rs distinct-name filter.
 *
 * Deadness proved by gcc -Wunused-function: "'rb_max_distinct_names_greedy'
 * defined but not used". */

/* Mirror util.rs::prune_dominated (util.rs:1003-1018) — sort, dedup, then for
   each candidate skip it when some already kept mask is a superset (m ⊆ k),
   otherwise drop every kept mask that m subsumes and push m.
   Divergence from Rust, structural: the Rust twin operates on u128 and the C
   header declares uint64_t, so the C form can only reason about the low 64 bits.
   `kept` is heap-sized from the deduped count so a long input cannot overflow a
   fixed array; the input array itself is used for the sort, so it is the
   caller's buffer and is not clamped to RB_MAX_ZONE. */
static int cmp_u64(const void *a, const void *b) {
    uint64_t va = *(const uint64_t *)a, vb = *(const uint64_t *)b;
    return (va > vb) - (va < vb);
}
void rb_prune_dominated(uint64_t *masks, int *n) {
    if (!masks || !n || *n <= 1) return;
    int total = *n;
    qsort(masks, (size_t)total, sizeof(uint64_t), cmp_u64);
    int m = 1;
    for (int i = 1; i < total; i++)
        if (masks[i] != masks[m - 1]) masks[m++] = masks[i];
    *n = m;
    uint64_t *kept = (uint64_t *)rb_malloc(sizeof(uint64_t) * (size_t)m);
    if (!kept) return;
    int nk = 0;
    for (int i = 0; i < *n; i++) {
        uint64_t mi = masks[i];
        int dominated = 0;
        for (int j = 0; j < nk; j++)
            if ((kept[j] & mi) == mi) { dominated = 1; break; }
        if (dominated) continue;
        int w = 0;
        for (int j = 0; j < nk; j++)
            if ((mi & kept[j]) != kept[j]) kept[w++] = kept[j];
        nk = w;
        kept[nk++] = mi;
    }
    memcpy(masks, kept, sizeof(uint64_t) * (size_t)nk);
    rb_free(kept);
    *n = nk;
}

/* ── Temporary effects ─────────────────────────────────────────────────── */

/* Check whether a given effect_type has a revert handler. */
static int rb_is_revertable_effect_type(const char *effect_type) {
    if (!effect_type) return 0;
    if (!strcmp(effect_type, "blade_bonus")) return 1;
    if (!strcmp(effect_type, "heart_bonus")) return 1;
    if (!strcmp(effect_type, "score_bonus")) return 1;
    if (!strcmp(effect_type, "score_set")) return 1;
    if (!strcmp(effect_type, "cost_bonus")) return 1;
    if (!strcmp(effect_type, "cost_set")) return 1;
    if (!strcmp(effect_type, "heart_override")) return 1;
    if (!strcmp(effect_type, "modify_cost")) return 1;
    if (!strcmp(effect_type, "set_heart_type")) return 1;
    if (!strncmp(effect_type, "gain_blade", 10)) return 1;
    if (!strncmp(effect_type, "gain_heart", 10)) return 1;
    if (!strncmp(effect_type, "gain_ability:", 12)) return 1;
    if (!strncmp(effect_type, "set_blade_type:", 15)) return 1;
    if (!strncmp(effect_type, "modify_score_", 13)) return 1;
    return 0;
}

/* Mirror util::push_temporary_effect. */
void rb_util_push_temporary_effect(
    GameState *g,
    const char *effect_type,
    const char *duration,
    const char *target_player_id,
    const char *description)
{
    if (!g || !effect_type) return;
    if (!duration || !strcmp(duration, "permanent")) return;
    if (!rb_is_revertable_effect_type(effect_type)) {
        fprintf(stderr, "[warn] temporary effect type '%s' has no expiry revert handler\n", effect_type);
    }
    if (g->n_temp_effects >= RB_MAX_TEMP_EFFECTS) {
        fprintf(stderr, "[warn] temporary effects buffer full, dropping '%s'\n", effect_type);
        return;
    }
    RbTempEffect *te = &g->temp_effects[g->n_temp_effects++];
    te->card_id = -1;
    te->dur = rb_parse_duration(duration);
    te->blade = 0;
    te->score = 0;
    te->cost = 0;
    te->gained_card_id = -1;
    te->gained_index = -1;
    te->revert_kind = RB_REVERT_NONE;
    te->previous_value = 0;
    te->previous_value2 = 0;
    te->revert_text[0] = '\0';
    for (int c = 0; c < 8; c++) { te->heart[c] = 0; te->need_heart[c] = 0; }
    (void)target_player_id;
    (void)description;
}

/* Low-level: push a pre-parsed temporary effect. */
int rb_push_temporary_effect(GameState *g, int card_id, int dur, int blade,
                              int score, int cost, const int *heart,
                              const int *need_heart) {
    if (!g || g->n_temp_effects >= RB_MAX_TEMP_EFFECTS) return 0;
    RbTempEffect *e = &g->temp_effects[g->n_temp_effects];
    e->card_id = card_id;
    e->dur = dur;
    e->blade = blade;
    e->score = score;
    e->cost = cost;
    e->gained_card_id = -1;
    e->gained_index = -1;
    e->revert_kind = RB_REVERT_NONE;
    e->previous_value = 0;
    e->previous_value2 = 0;
    e->revert_text[0] = '\0';
    for (int i = 0; i < 8; i++) {
        e->heart[i] = heart ? heart[i] : 0;
        e->need_heart[i] = need_heart ? need_heart[i] : 0;
    }
    g->n_temp_effects++;
    return 1;
}

/* ── Card property / blade filter helpers ──────────────────────────────── */

/* Mirror util.rs::check_card_property (util.rs:1656-1662) — the four arms are
   exactly Card's icon family. `has_draw_icon` (card.rs:4358) was MISSING here,
   so every draw-icon card fell through to `has = 0` and was rejected; 82 of the
   2,639 decoded cards carry a Draw icon, so this is not a rare branch. effects/
   look.c's private card_property test called this same helper and therefore
   inherited the same hole. condition.c:2633 / :3802 have their own copies and
   still lack the arm — reported, not touched (not this file). */
int rb_check_card_property(const char *prop, int negation, const Card *c) {
    int has = 0;
    if (!prop) has = 1;
    else if (!strcmp(prop, "has_blade_heart")) has = rb_card_has_blade_heart(c);
    else if (!strcmp(prop, "has_score_icon"))  has = rb_card_has_score_icon(c);
    else if (!strcmp(prop, "has_all_blade"))   has = rb_card_has_all_blade(c);
    else if (!strcmp(prop, "has_draw_icon"))   has = rb_card_has_draw_icon(c);
    return negation ? !has : has;
}

/* Mirror util.rs::filter_current_blade — post-filter by CURRENT blade total. */
int rb_filter_current_blade(const int *cands, int n, const GameState *g,
                            int blade_limit, const char *op, int *out, int max) {
    if (blade_limit < 0) {
        int m = 0;
        for (int i = 0; i < n && m < max; i++) out[m++] = cands[i];
        return m;
    }
    const char *o = op ? op : ">=";
    int m = 0;
    for (int i = 0; i < n; i++) {
        int cid = cands[i];
        int base = 0;
        Card c;
        if (rb_decode_card_by_index((uint32_t)cid, &c)) { base = c.blade; rb_free_card(&c); }
        int set       = rb_mods_get_blade_set((RbMods *)&g->mods, cid);
        int effective = set != 0 ? set : base;
        int additive  = rb_mods_get_blade((RbMods *)&g->mods, cid) - set;
        int total     = rb_saturate_u8(effective + additive);
        if (rb_compare_counts(o, total, blade_limit) && m < max) out[m++] = cid;
    }
    return m;
}

/* ── Identity / group helpers ──────────────────────────────────────────── */

void rb_set_card_identity(int cid, const char *name) {
    if (cid < 0 || cid >= RB_MAX_CARD_IDS || !name || !*name) return;
    /* rb_malloc, not libc malloc: this table lives for the whole process and is
       never freed, so under -DRB_NO_MALLOC it belongs in the arena like every
       other engine allocation. (It was the only raw malloc left in this file;
       norm_str and rb_prune_dominated already went through rb_malloc/rb_free.) */
    if (!g_ident[cid]) g_ident[cid] = (char *)rb_malloc((size_t)RB_MAX_IDENT * 28);
    char *base = g_ident[cid];
    if (!base) return;
    for (int i = 0; i < g_ident_n[cid]; i++)
        if (!strcmp(base + (size_t)i * 28, name)) return;
    if (g_ident_n[cid] < RB_MAX_IDENT) {
        strncpy(base + (size_t)g_ident_n[cid] * 28, name, 27);
        base[(size_t)g_ident_n[cid] * 28 + 27] = 0;
        g_ident_n[cid]++;
    }
}

int rb_card_matches_identity_str(int card_id, const char *group_name) {
    if (card_id < 0 || card_id >= RB_MAX_CARD_IDS || !group_name) return 0;
    char *gn = norm_str(group_name);
    int match = 0;
    char *base = g_ident[card_id];
    for (int i = 0; i < g_ident_n[card_id]; i++) {
        char *slot = base ? base + (size_t)i * 28 : NULL;
        char *io = slot ? norm_str(slot) : NULL;
        if ((io && (strstr(io, gn) || strstr(gn, io))) ||
            (slot && (strstr(slot, group_name) || strstr(group_name, slot))))
            match = 1;
        if (io) rb_free(io);
        if (match) break;
    }
    if (gn) rb_free(gn);
    return match;
}

/* ── Filter construction helpers ───────────────────────────────────────── */

/* Mirror util.rs::filter_from_parts. */
int rb_filter_from_parts(const char *card_type, const char *group, int cost_limit,
                          const char *cost_op, RbCardFilter *out) {
    if (!out) return 0;
    memset(out, 0, sizeof(RbCardFilter));
    if (card_type) strncpy(out->card_type, card_type, sizeof out->card_type - 1);
    if (group) strncpy(out->group, group, sizeof out->group - 1);
    out->cost_limit = cost_limit;
    if (cost_op) strncpy(out->cost_op, cost_op, sizeof out->cost_op - 1);
    out->has_filter = (card_type && card_type[0]) || (group && group[0]) || cost_limit >= 0;
    return 1;
}

/* Mirror util.rs::filter_from_parts_full. */
int rb_filter_from_parts_full(const char *card_type, const char *group, int cost_limit,
                               const char *cost_op, int cost_total, const char *cost_total_op,
                               RbCardFilter *out) {
    if (!out) return 0;
    memset(out, 0, sizeof(RbCardFilter));
    if (card_type) strncpy(out->card_type, card_type, sizeof out->card_type - 1);
    if (group) strncpy(out->group, group, sizeof out->group - 1);
    out->cost_limit = cost_limit;
    if (cost_op) strncpy(out->cost_op, cost_op, sizeof out->cost_op - 1);
    out->cost_total = cost_total;
    if (cost_total_op) strncpy(out->cost_total_op, cost_total_op, sizeof out->cost_total_op - 1);
    out->has_filter = (card_type && card_type[0]) || (group && group[0]) || cost_limit >= 0;
    return 1;
}

/* ── Cannot-baton-touch helper ─────────────────────────────────────────── */

int rb_has_cannot_baton_touch_protection_util(int incoming, int existing) {
    return rb_has_cannot_baton_touch_protection(incoming, existing);
}

/* ── exclude-self helper ───────────────────────────────────────────────── */

int rb_exclude_self(const int *ids, int n, int self_id) {
    if (!ids) return 0;
    int kept = 0;
    for (int i = 0; i < n; i++) if (ids[i] != self_id) kept++;
    return kept;
}

/* ── HeartColor helpers ────────────────────────────────────────────────── */

RbHeartColor rb_parse_heart_color(const char *s) {
    if (!s) return RB_HEART_PINK;
    if (!strcmp(s, "heart00") || !strcmp(s, "h00")) return RB_HEART_PINK;
    if (!strcmp(s, "heart01") || !strcmp(s, "h01")) return RB_HEART_RED;
    if (!strcmp(s, "heart02") || !strcmp(s, "h02")) return RB_HEART_YELLOW;
    if (!strcmp(s, "heart03") || !strcmp(s, "h03")) return RB_HEART_GREEN;
    if (!strcmp(s, "heart04") || !strcmp(s, "h04")) return RB_HEART_BLUE;
    if (!strcmp(s, "heart05") || !strcmp(s, "h05")) return RB_HEART_PURPLE;
    if (!strcmp(s, "heart06") || !strcmp(s, "h06")) return RB_HEART_ORANGE;
    if (!strcmp(s, "b_all")) return RB_HEART_ALL;
    if (!strcmp(s, "draw"))  return RB_HEART_DRAW;
    if (!strcmp(s, "score")) return RB_HEART_SCORE;
    if (!strcmp(s, "all"))   return RB_HEART_ALL;
    if (!strcmp(s, "heart07") || !strcmp(s, "b_heart07")) return RB_HEART_PINK;
    if (s[0] == 'b' && s[1] == '_') return rb_parse_heart_color(s + 2);
    return RB_HEART_PINK;
}
