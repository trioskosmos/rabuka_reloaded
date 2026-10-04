#include "rabuka.h"
#include <string.h>
#include <stdio.h>
#include <stdlib.h>

/* Ported faithfully from engine/src/ability/effects/score.rs.
   Every helper below carries the Rust line it mirrors.
   execute_modify_score covers:
     - operation split add / remove / set (score.rs:301-306, :102-107)
     - target resolution self / opponent / live_total (score.rs:52-54, :186)
     - card_type "member_card" -> stage-only candidate pool, otherwise
       live ++ success ++ stage (score.rs:254-284)
     - target_filter = full CardFilter minus negation and card_property
       (score.rs:195-211)
     - per_unit multiplication value * (matching / per_unit_count), capped by
       repeat_limit (score.rs:213-252)
     - the min:0 / score_floor floor (score.rs:57-59, :131-147, :327-336)
   execute_modify_required_hearts* / _yell_count / _limit / _success follow the
   remaining score.rs functions. */

static const char *sc_extra(const AbilityEffect *e, const char *k) {
    for (int i = 0; i < e->n_extra; i++)
        if (e->extra_k[i] && !strcmp(e->extra_k[i], k)) return e->extra_v[i];
    return NULL;
}
static int sc_extra_int(const AbilityEffect *e, const char *k) {
    const char *v = sc_extra(e, k); return v ? atoi(v) : 0;
}
static int sc_is_true(const char *v) {
    return v && (!strcmp(v, "true") || !strcmp(v, "1"));
}

/* The host card currently resolving: score.rs reads gs.activating_card for the
   per-unit count pool and self.activating_card_id for the self_target
   recipient test. Both are the same card in the C queue model. */
static int sc_host(const GameState *g) {
    return g->queue.resume_host >= 0 ? g->queue.resume_host : g->activating_card;
}

/* Does decoded card `c` print a heart of colour `col` (0..7) in its BASE or
   NEED heart map? Mirrors util.rs::card_matches_heart_colors (util.rs:1089-1109)
   and the util.rs:2616-2622 half of the `heart_colors` per-unit count. Both
   Rust spellings read `card.base_heart` and `card.need_heart` and NEVER the
   blade map, so a colour a member carries only as a blade heart must not
   satisfy either. The C Card lays the three heart maps out as
   [0, num_base) = base_heart, [num_base, num_base+num_blade) = blade_heart,
   [num_base+num_blade, n_hearts) = need_heart, so the blade span is skipped.
   util.rs:1097 `get_card(id).is_none_or(..)` means an UNDECODABLE card
   satisfies the colour test; that is what the `return 1` below reproduces. */
static int card_has_heart_color(int cid, int col) {
    Card c; if (!rb_decode_card_by_index((uint32_t)cid, &c)) return 1;
    int blade_end = c.num_base + c.num_blade;
    int r = 0;
    for (int i = 0; i < c.n_hearts; i++) {
        if ((int)c.heart_color[i] != col || c.heart_count[i] <= 0) continue;
        if (i < c.num_base || i >= blade_end) { r = 1; break; }
    }
    rb_free_card(&c);
    return r;
}

/* Comma / space / ideographic-comma separated list of colour names -> indices. */
static int sc_parse_colors(const char *s, int *out, int max) {
    if (!s || !*s) return 0;
    char buf[256]; strncpy(buf, s, sizeof(buf) - 1); buf[sizeof(buf) - 1] = 0;
    char *tok = strtok(buf, ",、 ");
    int n = 0;
    while (tok && n < max) {
        out[n++] = rb_heart_index(rb_parse_heart_color(tok));
        tok = strtok(NULL, ",、 ");
    }
    return n;
}

/* ── group_names: ONE entry, not the whole CSV ─────────────────────────────
 *
 * `AbilityEffect::group_name()` (core/card.rs:2462) is `group_names.first()`.
 * Rust models `group_names` as a Vec<String>; the C bytecode decoder flattens
 * it into ONE comma-joined extra, so a whole-CSV compare tests the literal
 * token "lilywhite,μ's" and no card is *called* that. Measured over the decoded
 * database: 802 effects carry `group_names` and 0 carry a `group` key, so the
 * old `group`-first fallback was dead weight and `group_names` is the only
 * wire key.
 *
 * util.c carries `rb_group_name_first` / `rb_effect_group_name` /
 * `rb_card_matches_effect_group` for exactly this, but they are `static` and
 * are NOT declared in include/rabuka.h, so this file keeps a local twin rather
 * than editing a file this change does not own. */
static const char *sc_group_name_first(const char *group_names, char *buf, size_t bufsz) {
    if (!group_names || !*group_names) return NULL;
    const char *comma = strchr(group_names, ',');
    size_t len = comma ? (size_t)(comma - group_names) : strlen(group_names);
    if (!buf || bufsz == 0) return group_names;
    if (len >= bufsz) len = bufsz - 1;
    memcpy(buf, group_names, len);
    buf[len] = '\0';
    return buf;
}

/* OR semantics over the list, i.e. util.rs::card_matches_heart_colors. */
static int heart_color_matches(int cid, const char *hc) {
    if (!hc || !*hc) return 1;
    int colors[8];
    int n = sc_parse_colors(hc, colors, 8);
    for (int i = 0; i < n; i++)
        if (card_has_heart_color(cid, colors[i])) return 1;
    return 0;
}

/* score.rs:195-202 / :212-219 / :344-351 — util::CardFilter::from_effect(effect)
   plus the explicit `filter.group` override and `filter.exclude_self =
   exclude_self_id`. `from_effect` is rb_effect_filter_subset (core/card.c), so
   that call now also carries need_heart_total / need_heart_operator /
   need_heart_color (util.rs:1839-1841) — score.rs:337-343 asks for the FULL
   filter precisely because 絶対的LOVER needs them — and the card_type fallback
   chain (util.rs:1809 -> card.c:824). The two overrides below are the only parts
   score.rs applies on top of from_effect. */
static void sc_build_filter(const AbilityEffect *e, RbCardFilter *f, int exclude_self_id) {
    rb_effect_filter_subset(e, f);
    /* score.rs:180 `let group_filter = effect.group_name().map(...)` then
       score.rs:216-217 / :348-349 `filter.group = Some(g)` — ONE name, the
       first. `group` is not a key on any decoded effect, so that fallback was
       always dead. */
    char gbuf[128];
    const char *g = sc_group_name_first(sc_extra(e, "group_names"), gbuf, sizeof gbuf);
    if (g && *g) {
        strncpy(f->group, g, sizeof f->group - 1);
        f->group[sizeof f->group - 1] = 0;
        f->has_group = 1;
    }
    /* score.rs:46-50 + :202 — exclude_self is *overwritten* with the activating
       card id, or cleared when the effect does not carry exclude_self.
       score.rs:219 / :351 sets the field and NOTHING ELSE, so the filter is not
       rebuilt afterwards: CardFilter::has_filter (util.rs:1425-1446) has no
       exclude_self term to begin with, and an otherwise-empty filter matches
       every card, so the local recompute this used to end with was a no-op. */
    f->has_exclude_self = 0;
    f->exclude_self_id = -1;
    if (exclude_self_id >= 0) {
        f->exclude_self_id = exclude_self_id;
        f->has_exclude_self = 1;
    }
}

/* util.rs:686 card_series_matches_group — the canonical group→series mapping. */
static int sc_series_matches_group(const char *series, const char *group) {
    if (!series || !group) return 0;
    if (!strcmp(group, "μ's")) {
        /* each series line is judged on its own so multi-series joint cards do
           not match through a bare ラブライブ！ line */
        const char *p = series;
        while (*p) {
            const char *nl = strchr(p, '\n');
            size_t len = nl ? (size_t)(nl - p) : strlen(p);
            char line[512];
            if (len < sizeof line) {
                memcpy(line, p, len); line[len] = 0;
                if (strstr(line, "ラブライブ！") &&
                    !strstr(line, "サンシャイン") && !strstr(line, "虹ヶ咲") &&
                    !strstr(line, "スーパースター") && !strstr(line, "蓮ノ空"))
                    return 1;
            }
            if (!nl) break;
            p = nl + 1;
        }
        return 0;
    }
    if (!strcmp(group, "Aqours"))  return strstr(series, "サンシャイン") != NULL;
    if (!strcmp(group, "虹ヶ咲"))  return strstr(series, "虹ヶ咲") != NULL;
    if (!strcmp(group, "Liella!")) return strstr(series, "スーパースター") != NULL;
    if (!strcmp(group, "蓮ノ空"))  return strstr(series, "蓮ノ空") != NULL;
    return 0;
}

/* util.rs norm_group_name — ！(U+FF01) → !, µ(U+00B5) → μ(U+03BC).
   Returns 1 when the string needed rewriting. */
static int sc_norm_group(const char *s, char *out, size_t out_sz) {
    int changed = 0;
    size_t w = 0;
    for (const unsigned char *p = (const unsigned char *)s; *p; ) {
        if (p[0] == 0xEF && p[1] == 0xBC && p[2] == 0x81 && w + 1 < out_sz) {
            out[w++] = '!'; p += 3; changed = 1; continue;
        }
        if (p[0] == 0xC2 && p[1] == 0xB5 && w + 2 < out_sz) {
            out[w++] = (char)0xCE; out[w++] = (char)0xBC; p += 2; changed = 1; continue;
        }
        if (w + 1 < out_sz) out[w++] = (char)*p;
        p++;
    }
    out[w] = 0;
    return changed;
}

/* util.rs card_matches_group_str, ported exactly. util.c's
   rb_card_matches_group_str feeds the raw (frequently empty) card group string
   into strstr(), so an empty `g` makes every card match every group; the group
   filter is therefore inert engine-wide. score.c evaluates the group predicate
   itself so its own filters behave like Rust's; the central fix belongs in
   util.c (reported separately). */
static int sc_card_matches_group(int cid, const char *group_name) {
    if (!group_name) return 1;
    char gn[256];
    sc_norm_group(group_name, gn, sizeof gn);

    Card c;
    if (!rb_decode_card_by_index((uint32_t)cid, &c)) return 0;
    const char *unit = c.unit_idx ? rb_card_string(c.unit_idx) : NULL;
    const char *grp  = rb_card_string(c.group_idx);
    const char *series = c.series_idx ? rb_card_string(c.series_idx) : NULL;
    if (!unit) unit = "";

    int match = 0;
    if (!strcmp(unit, gn)) match = 1;
    if (!match && (strstr(unit, "！") || strstr(unit, "µ"))) {
        char un[256];
        sc_norm_group(unit, un, sizeof un);
        if (!strcmp(un, gn)) match = 1;
    }
    if (!match && grp && !strcmp(grp, group_name)) match = 1;

    char names[512];
    names[0] = 0;
    if (!match && c.name) {
        snprintf(names, sizeof names, "%s", c.name);
        if (strstr(names, gn)) match = 1;
    }
    if (!match && (strstr(names, "！") || strstr(names, "µ"))) {
        char nn[512];
        sc_norm_group(names, nn, sizeof nn);
        if (strstr(nn, gn)) match = 1;
    }
    if (!match) {
        char extra[512];
        extra[0] = 0;
        if (rb_card_get_card_names(cid, extra, sizeof extra) && extra[0]) {
            if (strstr(extra, gn)) match = 1;
            else if (strstr(extra, "！") || strstr(extra, "µ")) {
                char en[512];
                sc_norm_group(extra, en, sizeof en);
                if (strstr(en, gn)) match = 1;
            }
        }
    }
    if (!match) match = sc_series_matches_group(series, gn);

    /* Constant set_card_identity abilities add group memberships in all zones. */
    if (!match) {
        int n = rb_card_num_abilities((uint32_t)cid);
        for (int i = 0; i < n && !match; i++) {
            Ability ab;
            memset(&ab, 0, sizeof ab);
            if (!rb_decode_card_ability((uint32_t)cid, i, &ab)) continue;
            AbilityEffect *eff = ab.effect;
            if (eff && eff->action && !strcmp(eff->action, "set_card_identity")) {
                const char *ids = sc_extra(eff, "identities");
                if (!ids) ids = sc_extra(eff, "identity");
                if (ids) {
                    char buf[512];
                    snprintf(buf, sizeof buf, "%s", ids);
                    char *tok = strtok(buf, ",、 []\"'");
                    while (tok) {
                        if (!strcmp(tok, gn)) { match = 1; break; }
                        if ((strstr(tok, "！") || strstr(tok, "µ"))) {
                            char tn[256];
                            sc_norm_group(tok, tn, sizeof tn);
                            if (!strcmp(tn, gn)) { match = 1; break; }
                        }
                        tok = strtok(NULL, ",、 []\"'");
                    }
                }
            }
            rb_free_ability(&ab);
        }
    }
    rb_free_card(&c);
    return match;
}

/* single-card CardFilter::matches (util.rs CardFilter::matches, skip_empty) */
static int sc_filter_matches(const RbCardFilter *f, int cid) {
    int one = cid;
    int out[1];
    return rb_matching_ids(f, &one, 1, out, 1) == 1;
}

static int sc_matches(const RbCardFilter *f, int cid) {
    if (cid < 0) return 0;
    int ok = sc_filter_matches(f, cid);
    /* util.c's group predicate accepts every card; override it with the
     score.c port of util.rs::card_matches_group_str. */
    if (ok && f->has_group && f->group[0] && !sc_card_matches_group(cid, f->group))
        ok = 0;
    return ok;
}

/* util.rs::zone_cards for the per-unit zones score.rs can name. Returns -1 for
   an unknown zone so the caller can reproduce util.rs:2363 `_ => return 1`. */
static int sc_zone_cards(const RbPlayer *P, const char *zone, int *out, int max) {
    if (!zone) return -1;
    int n = 0;
    if (!strcmp(zone, "stage") || !strcmp(zone, "member") ||
        !strcmp(zone, "人") || !strcmp(zone, "members")) {
        for (int i = 0; i < RB_STAGE_SIZE && n < max; i++)
            if (P->stage[i] != RB_EMPTY_SLOT) out[n++] = P->stage[i];
        return n;
    }
    if (!strcmp(zone, "hand") || !strcmp(zone, "card")) {
        for (int i = 0; i < P->hand.n && n < max; i++) out[n++] = P->hand.cards[i];
        return n;
    }
    if (!strcmp(zone, "under_member") || !strcmp(zone, "下")) {
        for (int s = 0; s < RB_STAGE_SIZE; s++)
            for (int i = 0; i < P->under_cards[s].n && n < max; i++)
                out[n++] = P->under_cards[s].cards[i];
        return n;
    }
    if (!strcmp(zone, "discard") || !strcmp(zone, "waitroom")) {
        for (int i = 0; i < P->discard.n && n < max; i++) out[n++] = P->discard.cards[i];
        return n;
    }
    if (!strcmp(zone, "live_card_zone") || !strcmp(zone, "live") ||
        !strcmp(zone, "live_zone")) {
        for (int i = 0; i < P->live.n && n < max; i++) out[n++] = P->live.cards[i];
        return n;
    }
    if (!strcmp(zone, "success_live_zone") || !strcmp(zone, "success_live_card_zone") ||
        !strcmp(zone, "success")) {
        for (int i = 0; i < P->success.n && n < max; i++) out[n++] = P->success.cards[i];
        return n;
    }
    return -1;
}

/* util.rs:2398 apply_distinct_filter — duplicate names collapse to one unit. */
static int sc_apply_distinct(int *ids, int n) {
    char seen[RB_MAX_ZONE][96];
    int nseen = 0, kept = 0;
    for (int i = 0; i < n; i++) {
        Card c;
        char key[96];
        if (rb_decode_card_by_index((uint32_t)ids[i], &c)) {
            rb_card_normalize_name(c.name ? c.name : "", key, sizeof key);
            rb_free_card(&c);
        } else {
            key[0] = 0;
        }
        int dup = 0;
        for (int s = 0; s < nseen; s++)
            if (!strcmp(seen[s], key)) { dup = 1; break; }
        if (dup) continue;
        if (nseen < RB_MAX_ZONE) {
            snprintf(seen[nseen], sizeof seen[0], "%s", key);
            nseen++;
        }
        ids[kept++] = ids[i];
    }
    return kept;
}

/* util.rs:2305-2429 resolve_per_unit_count, restricted to the feature set a
   modify_score effect can carry (group / card_type / need_heart_total /
   exclude_self / distinct / heart_colors / state). */
static int score_per_unit_count(const GameState *gs, int pl, const RbCardFilter *filter,
                                const char *zone, const char *heart_colors,
                                const char *state_filter) {
    const RbPlayer *P = &gs->p[pl];

    /* util.rs:2320-2344 — per_unit_type "heart_colors" counts the DISTINCT
       heart colours carried by stage members that pass the filter. */
    if (zone && !strcmp(zone, "heart_colors")) {
        int colors[8];
        int ncol = sc_parse_colors(heart_colors, colors, 8);
        if (ncol == 0) return 0;
        int stage[RB_STAGE_SIZE]; int ns = 0;
        for (int i = 0; i < RB_STAGE_SIZE; i++)
            if (P->stage[i] != RB_EMPTY_SLOT) stage[ns++] = P->stage[i];
        int found[8]; memset(found, 0, sizeof found);
        for (int i = 0; i < ns; i++) {
            if (!sc_matches(filter, stage[i])) continue;
            for (int c = 0; c < ncol; c++)
                if (card_has_heart_color(stage[i], colors[c])) found[colors[c]] = 1;
        }
        int r = 0;
        for (int c = 0; c < 8; c++) r += found[c];
        return r;
    }

    /* score.rs:225-227 — the つ counter is the energy paid by the current cost
       step (mods.last_cost_energy_count). That field has no home in RbMods yet
       and rabuka.h:1279 forbids adding it unconfirmed, so 0 is the value, the
       same one util.c's RB_LAST_COST_ENERGY accessor yields for the identical
       clause in rb_resolve_per_unit_count (util.c:2529). Reporting 1 here
       instead would multiply the effect by one unit that was never paid. */
    if (zone && !strcmp(zone, "つ")) return 0;

    /* util.rs:2350-2357 — "枚" is under_member only when the filter already
       pins card_type to member_card, otherwise it counts hand cards. */
    const char *eff_zone = zone;
    if (zone && !strcmp(zone, "枚"))
        eff_zone = (filter->card_type[0] && !strcmp(filter->card_type, "member_card"))
                 ? "under_member" : "hand";

    int ids[RB_MAX_ZONE];
    int n = sc_zone_cards(P, eff_zone, ids, RB_MAX_ZONE);
    if (n < 0) return 1;                       /* util.rs:2363 _ => return 1 */
    if (n == 0) return 0;

    /* util.rs:2404-2413 — the state filter only applies to stage cards. */
    if (state_filter && eff_zone && !strcmp(eff_zone, "stage")) {
        int kept = 0;
        for (int i = 0; i < n; i++) {
            const char *ori = rb_mods_get_orientation((RbMods *)&gs->mods, ids[i]);
            if (!rb_orientation_matches_state(ori, state_filter)) continue;
            ids[kept++] = ids[i];
        }
        n = kept;
    }

    int keep[RB_MAX_ZONE];
    if (!heart_colors || !*heart_colors) {
        /* util.rs:2416 count_matching_distinct(cards, db, filter, is_stage) */
        int nk = 0;
        for (int i = 0; i < n; i++)
            if (sc_matches(filter, ids[i]))
                keep[nk++] = ids[i];
        n = nk;
    } else {
        /* util.rs:2418-2427 */
        int nk = 0;
        for (int i = 0; i < n; i++) {
            if (!sc_matches(filter, ids[i])) continue;
            if (!heart_color_matches(ids[i], heart_colors)) continue;
            keep[nk++] = ids[i];
        }
        n = nk;
    }
    if (filter->distinct) n = sc_apply_distinct(keep, n);
    return n;
}

/* score.rs:219-223 — per_unit_type "heart_colors" wins; otherwise `location`
   is the zone override and per_unit_type is only a fallback. */
static const char *sc_per_unit_zone(const AbilityEffect *e) {
    const char *pt = sc_extra(e, "per_unit_type");
    if (pt && !strcmp(pt, "heart_colors")) return pt;
    const char *loc = sc_extra(e, "location");
    if (loc && *loc) return loc;
    return pt;
}

/* score.rs:206-208 (live_total) and :495-496 (per card):
       let has_floor = effect.score_floor_any().is_some()
           || effect_constraint.as_deref() == Some("min:0");
   Both spellings come off the decoded filter, so the Japanese 「…合計スコアは０
   未満にはならない」 phrasing is NOT a third source: abilities.json already
   carries effect_constraint:"min:0" + score_floor:0 on every effect whose text
   prints it (PL!N-bp5-011-*). The old strstr() here therefore added a floor to
   negative per-card modifiers Rust never floors. */
static int sc_has_floor(const AbilityEffect *e) {
    if (sc_extra(e, "score_floor")) return 1;
    const char *ec = sc_extra(e, "effect_constraint");
    if (ec && !strcmp(ec, "min:0")) return 1;
    return 0;
}

/* score.rs:330-359 */
static int sc_apply_to_card(GameState *gs, const AbilityEffect *e, int cid, int delta,
                           int has_floor, int is_set) {
    if (has_floor) {
        int current_mod = rb_mods_get_score(&gs->mods, cid);
        if (current_mod + delta < 0) return 0;   /* score.rs:331-336 */
    }
    if (is_set) rb_mods_set_score(&gs->mods, cid, delta);
    else        rb_mods_add_score(&gs->mods, cid, delta);
    int source = sc_host(gs);
    if (source >= 0) {
        rb_mods_trace_push(&gs->mods, source, e->text,
                           is_set ? RB_EFFECT_SCORE_SET : RB_EFFECT_SCORE_BONUS,
                           cid, -1, delta);
    }
    return 1;
}

/* LiveSuccess / constant score modifiers, i.e. the sum of every live card's
   printed score plus the accumulated bonus. score.rs:120-128. */
static int sc_live_base_total(const GameState *gs, int pl) {
    const RbPlayer *P = &gs->p[pl];
    int total = 0;
    for (int i = 0; i < P->live.n; i++) {
        Card c;
        if (rb_decode_card_by_index((uint32_t)P->live.cards[i], &c)) {
            total += (int)c.score;
            rb_free_card(&c);
        }
    }
    for (int i = 0; i < P->success.n; i++) {
        Card c;
        if (rb_decode_card_by_index((uint32_t)P->success.cards[i], &c)) {
            total += (int)c.score;
            rb_free_card(&c);
        }
    }
    return total;
}

int rb_execute_modify_score(GameState *gs, int actor, AbilityEffect *e) {
    if (!gs || !e) return -1;

    const char *op = sc_extra(e, "operation"); if (!op) op = "add";   /* score.rs:19-23 */
    int value = sc_extra(e, "value") ? sc_extra_int(e, "value")
                                     : (e->count >= 0 ? e->count : 0); /* score.rs:24 */
    const char *target = e->target ? e->target : "self";             /* target_name(), card.rs:2336 */
    const char *heart_colors = sc_extra(e, "heart_colors");
    const char *state_filter = sc_extra(e, "state");
    /* score.rs:181-182 `effect.per_unit_any()` / `effect.per_unit_count_any()`
       read the FILTER struct, and the C decoder carries them as the first-class
       AbilityEffect fields -- NOT as n_extra entries (a decoded modify_score
       carries `per_unit` in extras but only `per_unit_count` in the field).
       Reading per_unit_count out of n_extra therefore always yielded 0 and the
       `max(1)` clamp turned EVERY 「N枚につき」 into 「1枚につき」, so PL!SP-pb2-008-R
       (2 cards per +1) paid +1 per card and the max_repeats 2 cap hid it. */
    int per_unit = e->per_unit ? 1 : sc_is_true(sc_extra(e, "per_unit"));
    int per_unit_count_val = e->per_unit_count > 0 ? e->per_unit_count
                                                    : sc_extra_int(e, "per_unit_count");
    if (per_unit_count_val < 1) per_unit_count_val = 1;              /* score.rs:92/243 max(1) */
    int self_target = (!strcmp(e->self_target_field, "true") ||
                       sc_is_true(sc_extra(e, "self_target")));      /* score.rs:36 */
    int exclude_self_id = sc_is_true(sc_extra(e, "exclude_self"))
                        ? gs->activating_card : -1;                   /* score.rs:46-50 */
    int cap = e->repeat_limit;                                      /* score.rs:93/244 */

    const char *master = (actor == 0) ? "p1" : "p2";
    int is_live_total = target && !strcmp(target, "live_total");     /* score.rs:52 */
    const char *resolved_target = is_live_total ? "self" : target;   /* score.rs:54 */
    /* score.rs:186 resolve_target_player_mut — exactly one player; "both" and
       unknown targets fall back to player1 (abilities.rs:2527-2545). */
    int owner = rb_target_player_index(resolved_target, master);
    if (owner < 0) owner = 0;

    RbCardFilter sf;
    sc_build_filter(e, &sf, exclude_self_id);

    /* ── live_total (score.rs:60-184) ─────────────────────────────────── */
    if (is_live_total) {
        int effective_value = value;
        if (per_unit) {
            const char *zone = sc_per_unit_zone(e);
            int matching_count = score_per_unit_count(gs, owner, &sf, zone,
                                                      heart_colors, state_filter);
            int units = matching_count / per_unit_count_val;
            if (cap > 0 && units > cap) units = cap;
            effective_value = value * units;
        }
        int delta = !strcmp(op, "add")    ?  effective_value
                  : !strcmp(op, "remove") ? -effective_value
                  : !strcmp(op, "set")    ?  effective_value
                  : 0;                                              /* score.rs:102-107 */
        int owner_p1 = (owner == 0);
        int current_bonus = owner_p1 ? (int)gs->mods.p1_constant_total_score_bonus
                                     : (int)gs->mods.p2_constant_total_score_bonus;
        int base_total = sc_live_base_total(gs, owner);
        int has_floor = sc_has_floor(e);
        int projected_total = base_total + current_bonus + delta;
        int clamped_delta = delta;
        if (has_floor && projected_total < 0) {
            int max_negative = -(base_total + current_bonus);
            clamped_delta = max_negative > delta ? max_negative : delta;
        }
        if (!(clamped_delta == 0 && has_floor && delta < 0)) {       /* score.rs:138-147 */
            if (owner_p1) gs->mods.p1_constant_total_score_bonus =
                              (int16_t)(current_bonus + clamped_delta);
            else          gs->mods.p2_constant_total_score_bonus =
                              (int16_t)(current_bonus + clamped_delta);
        }
        int source = sc_host(gs);
        if (source >= 0 && clamped_delta != 0) {
            rb_mods_trace_push(&gs->mods, source, e->text,
                               !strcmp(op, "set") ? RB_EFFECT_SCORE_SET
                                                 : RB_EFFECT_SCORE_BONUS,
                               source, -1, clamped_delta);
        }
        return 0;
    }

    /* ── per-card modifiers (score.rs:185-395) ─────────────────────────── */
    /* score.rs:209-211 — negation and card_property describe the per-unit
       COUNT predicate, never which cards receive the modifier. */
    RbCardFilter tsf = sf;
    tsf.negation = 0;

    int final_value = value;
    if (per_unit) {
        const char *zone = sc_per_unit_zone(e);
        int matching_count = score_per_unit_count(gs, owner, &sf, zone,
                                                  heart_colors, state_filter);
        int units = matching_count / per_unit_count_val;
        if (cap > 0 && units > cap) units = cap;
        final_value = value * units;                                /* score.rs:249 */
    }

    /* score.rs:254-284 — candidate pool. */
    int cand[RB_MAX_ZONE];
    int ncand = 0;
    const RbPlayer *P = &gs->p[owner];
    if (tsf.card_type[0] && !strcmp(tsf.card_type, "member_card")) {
        ncand = sc_zone_cards(P, "stage", cand, RB_MAX_ZONE);
        {
            int w = 0;
            for (int i = 0; i < ncand; i++)
                if (sc_matches(&tsf, cand[i]))
                    cand[w++] = cand[i];
            ncand = w;
        }
    } else {
        for (int i = 0; i < P->live.n && ncand < RB_MAX_ZONE; i++)
            cand[ncand++] = P->live.cards[i];
        for (int i = 0; i < P->success.n && ncand < RB_MAX_ZONE; i++)
            cand[ncand++] = P->success.cards[i];
        for (int i = 0; i < RB_STAGE_SIZE && ncand < RB_MAX_ZONE; i++)
            if (P->stage[i] != RB_EMPTY_SLOT) cand[ncand++] = P->stage[i];
        /* score.rs:275-281 — a self-targeting live-card ability owns its own
           recipient wherever the card currently lives (e.g. still in hand). */
        if (self_target) {
            int act = sc_host(gs);
            if (act >= 0) {
                int seen = 0;
                for (int i = 0; i < ncand; i++) if (cand[i] == act) { seen = 1; break; }
                if (!seen && ncand < RB_MAX_ZONE) cand[ncand++] = act;
            }
        }
        /* score.rs:431-449 -- a card id names ONE physical card, but the chain
           above can see the same id twice: a live being performed is both the
           live card and, once it has succeeded, an entry in the success live
           card zone. Without this order-preserving dedup the pool holds that
           card twice and a self-targeted effect applies once per occurrence --
           「このカードのスコアを＋１する」 (PL!-pb2-041-L) scored +2 from a single
           activation. `seen.insert` keeps the FIRST hit, as Rust's retain does. */
        {
            int w = 0;
            for (int i = 0; i < ncand; i++) {
                int dup = 0;
                for (int k = 0; k < w; k++) if (cand[k] == cand[i]) { dup = 1; break; }
                if (dup) continue;
                cand[w++] = cand[i];
            }
            ncand = w;
        }
    }

    /* score.rs:285-309 — recipients. */
    int recv[RB_MAX_ZONE];
    int nr = 0;
    int act = sc_host(gs);
    for (int i = 0; i < ncand; i++) {
        int cid = cand[i];
        if (!sc_matches(&tsf, cid)) continue;
        if (self_target && (act < 0 || cid != act)) continue;
        recv[nr++] = cid;
    }

    int delta = !strcmp(op, "add")    ?  final_value
              : !strcmp(op, "remove") ? -final_value
              : !strcmp(op, "set")    ?  final_value
              : 0;                                                  /* score.rs:301-306 */
    int has_floor = sc_has_floor(e);
    int is_set = !strcmp(op, "set");
    for (int i = 0; i < nr; i++)
        sc_apply_to_card(gs, e, recv[i], delta, has_floor, is_set);
    return 0;
}

/* ── Shared helpers for the remaining score.rs ports ── */

/* True when every PRINTED BASE heart colour on the card is in `exc`.
   score.rs:119-132 `PerUnitFilter::heart_set_all_excluded` reads
   `card.base_heart.as_ref()` and tests `base.hearts.keys()`, so a card with no
   base_heart (a live card) is NEVER all-excluded, and a member's blade hearts
   and need_heart are outside the test. The C Card lays its three heart maps out
   as [0, num_base) = base_heart, then blade_heart, then need_heart, so only the
   first num_base entries participate. */
static int sc_card_all_hearts_excluded(const Card *c, const int *exc, int nexc) {
    if (c->num_base <= 0) return 0;
    for (int i = 0; i < c->num_base && i < c->n_hearts; i++) {
        int col = (int)c->heart_color[i];
        int found = 0;
        for (int k = 0; k < nexc; k++) if (exc[k] == col) { found = 1; break; }
        if (!found) return 0;
    }
    return 1;
}

/* Printed BASE heart icons of one colour (score.rs:93-96
   `base.hearts.get(&hc)`), which is what a 「そのメンバーが持つheartNN」 per-unit
   count reads -- not the blade- or need-heart merges the C record also holds. */
static int sc_card_base_heart_count(int cid, int col) {
    Card c; if (!rb_decode_card_by_index((uint32_t)cid, &c)) return 0;
    int r = 0;
    for (int i = 0; i < c.num_base && i < c.n_hearts; i++)
        if ((int)c.heart_color[i] == col) r += c.heart_count[i];
    rb_free_card(&c);
    return r;
}

static int sc_card_moved_this_turn(const GameState *g, int cid) {
    return (cid >= 0 && cid < RB_MAX_CARD_IDS) ? (g->moved_this_turn[cid] != 0) : 0;
}
static int sc_card_appeared_this_turn(const GameState *g, int cid) {
    for (int i = 0; i < g->n_cards_appeared_this_turn; i++)
        if (g->cards_appeared_this_turn[i] == cid) return 1;
    return 0;
}

/* ── PerUnitFilter (score.rs:56-133) ─────────────────────────────────────
   「〜1人につき」 unit criteria, gathered ONCE and re-tested at each card. The
   criteria are cumulative and each is optional; `seen_names` carries the
   already-claimed names across calls for the distinct-names rule. */
typedef struct {
    const GameState *gs;
    int exclude_self;                  /* the activating card never counts   */
    int distinct_names;                /* duplicate-named members count once */
    int act;                           /* gs.activating_card, or -1          */
    const char *group;
    const char *timing_condition;
    const int  *exc; int n_exc;
    char seen[64][192];
    int n_seen;
} ScPerUnitFilter;

/* score.rs:100-115 `claim_distinct_name` -- key is the joined card_names list
   when the card has any, else the normalised printed name. */
static int sc_claim_distinct_name(ScPerUnitFilter *f, int cid) {
    Card c;
    if (!rb_decode_card_by_index((uint32_t)cid, &c)) return 0;   /* :105-107 */
    char key[192];
    key[0] = 0;
    char names[512];
    int n_names = rb_card_get_card_names(cid, names, sizeof names);
    if (n_names > 0 && names[0]) {
        size_t w = 0;
        const char *p = names;
        for (int i = 0; i < n_names && p < names + sizeof names; i++) {
            if (i && w + 1 < sizeof key) key[w++] = '|';   /* names.join("|") */
            size_t l = strlen(p);
            if (w + l >= sizeof key) l = sizeof key - w - 1;
            memcpy(key + w, p, l); w += l; key[w] = 0;
            p += l + 1;
        }
    } else {
        rb_card_normalize_name(c.name ? c.name : "", key, sizeof key);
    }
    rb_free_card(&c);
    for (int i = 0; i < f->n_seen; i++)
        if (!strcmp(f->seen[i], key)) return 0;
    if (f->n_seen < 64) {
        snprintf(f->seen[f->n_seen], sizeof f->seen[0], "%s", key);
        f->n_seen++;
    }
    return 1;
}

/* score.rs:71-97 `PerUnitFilter::accepts`, in Rust's order. */
static int sc_puf_accepts(ScPerUnitFilter *f, int cid) {
    if (f->exclude_self && f->act >= 0 && cid == f->act) return 0;
    if (f->group && !rb_card_matches_group_str(cid, f->group)) return 0;
    if (f->distinct_names && !sc_claim_distinct_name(f, cid)) return 0;
    if (f->timing_condition && !strcmp(f->timing_condition, "appeared_or_moved_this_turn")
        && !sc_card_moved_this_turn(f->gs, cid)
        && !sc_card_appeared_this_turn(f->gs, cid)) return 0;
    if (f->n_exc == 0) return 1;
    Card c;
    if (!rb_decode_card_by_index((uint32_t)cid, &c)) return 1;
    int all_excluded = sc_card_all_hearts_excluded(&c, f->exc, f->n_exc);
    rb_free_card(&c);
    return !all_excluded;
}

/* Faithful port of execute_modify_required_hearts (required_hearts.rs:28-223).
   Handles per_unit (both the per_unit_heart_colors total-icon mode and the
   default per-card location count, both gated by the full PerUnitFilter), the
   success-zone live+success card pool, group/original-score filters, and
   per-colour set/add of need_heart modifiers. */
int rb_execute_modify_required_hearts(GameState *gs, int actor, AbilityEffect *e) {
    if (!gs || !e) return -1;
    const char *operation = sc_extra(e, "operation"); if (!operation) operation = "decrease";
    /* card.rs:2457 `value_or_count` is `value_any().or(count)` -- `value` WINS
       over `count`. The C read `count` first, so an effect carrying both (the
       parser emits `count` alongside `value` on several modify_required_hearts
       sub-actions) applied the wrong number. */
    const char *vtext = sc_extra(e, "value");
    int value = vtext ? atoi(vtext) : (e->count >= 0 ? e->count : 0);
    const char *target = e->target ? e->target : "self";
    int per_unit = e->per_unit ? 1 : sc_is_true(sc_extra(e, "per_unit"));
    int per_unit_count = e->per_unit_count > 0 ? e->per_unit_count
                                                : sc_extra_int(e, "per_unit_count");
    if (per_unit_count <= 0) per_unit_count = 1;
    /* required_hearts.rs:40 `let group_name = effect.group_name();` consumed at
       :159-161 `card_matches_group_str(db, card_id, Some(gn))` — ONE name, the
       first. `group` is not a key on any decoded effect, so reading it first
       was a dead key that also shadowed the real one. */
    char grp_buf[128];
    const char *group = sc_group_name_first(sc_extra(e, "group_names"),
                                            grp_buf, sizeof grp_buf);
    const char *location = sc_extra(e, "location");
    const char *timing_condition = sc_extra(e, "timing_condition");
    int original_value = sc_is_true(sc_extra(e, "original_value"));
    int original_count = sc_extra_int(e, "original_count");
    /* OriginalScoreFilter.count / .operator are Option<u8> / Option<&str>: the
       threshold is only applied when the text stated BOTH. */
    int has_original_count = (sc_extra(e, "original_count") != NULL);
    const char *original_operator = sc_extra(e, "original_operator");
    int exclude_self = sc_is_true(sc_extra(e, "exclude_self"));
    int self_target = (!strcmp(e->self_target_field, "true") || sc_is_true(sc_extra(e, "self_target"))) ? 1 : 0;
    int exc_colors[8]; int n_exc = sc_parse_colors(sc_extra(e, "exclude_heart_colors"), exc_colors, 8);
    int max_flag = sc_is_true(sc_extra(e, "max"));
    int repeat_limit = e->repeat_limit;
    int pu_colors[8]; int n_pu = sc_parse_colors(sc_extra(e, "per_unit_heart_colors"), pu_colors, 8);
    int is_distinct = (e->distinct_flag != 0);
    int act = sc_host(gs);

    int pl = rb_target_player_index(target, actor == 0 ? "p1" : "p2");
    if (pl < 0) pl = actor;
    const RbPlayer *P = &gs->p[pl];

    /* ── per_unit: recompute value from matching units ── */
    /* required_hearts.rs:67-73 -- the unit criteria are resolved ONCE and both
       per-unit branches gate on the SAME set; only what each unit is worth
       differs. The previous C applied exclude_self + group to the heart-icon
       branch and nothing at all to the card-count branch beyond the group. */
    ScPerUnitFilter puf;
    memset(&puf, 0, sizeof puf);
    puf.gs = gs;
    puf.exclude_self = exclude_self;
    puf.distinct_names = is_distinct;
    puf.act = act;
    puf.group = group;
    puf.timing_condition = timing_condition;
    puf.exc = exc_colors;
    puf.n_exc = n_exc;

    if (per_unit) {
        int per_unit_base = max_flag ? 1 : value;
        int units;
        if (n_pu > 0) {
            /* required_hearts.rs:76-101 -- the PRINTED BASE heart icon total of
               the named colours across stage members the filter accepts. */
            int total = 0;
            for (int i = 0; i < RB_STAGE_SIZE; i++) {
                int cid = P->stage[i];
                if (cid == RB_EMPTY_SLOT) continue;
                if (!sc_puf_accepts(&puf, cid)) continue;
                for (int j = 0; j < n_pu; j++) total += sc_card_base_heart_count(cid, pu_colors[j]);
            }
            units = total / (per_unit_count > 0 ? per_unit_count : 1);
            if (repeat_limit > 0 && units > repeat_limit) units = repeat_limit;
        } else {
            /* required_hearts.rs:103-113 + score.rs:36-50 `per_unit_source_cards`
               -- success zone / live zone, and the STAGE for every other spelling
               (including no location at all). */
            int zone_cards[RB_MAX_ZONE]; int nz = 0;
            if (location && (!strcmp(location, "success_live_zone") || !strcmp(location, "success_live_card_zone"))) {
                for (int i = 0; i < P->success.n; i++) zone_cards[nz++] = P->success.cards[i];
            } else if (location && (!strcmp(location, "live_card_zone") || !strcmp(location, "live_zone"))) {
                for (int i = 0; i < P->live.n; i++) zone_cards[nz++] = P->live.cards[i];
            } else {
                for (int i = 0; i < RB_STAGE_SIZE; i++)
                    if (P->stage[i] != RB_EMPTY_SLOT) zone_cards[nz++] = P->stage[i];
            }
            int count = 0;
            for (int i = 0; i < nz; i++)
                if (sc_puf_accepts(&puf, zone_cards[i])) count++;
            units = count / (per_unit_count > 0 ? per_unit_count : 1);
            if (repeat_limit > 0 && units > repeat_limit) units = repeat_limit;
        }
        value = per_unit_base * units;
    }

    /* ── Build candidate card ids (live, or live+success when self-target
       activating card sits in success_live_card_zone). ── */
    int in_success = 0;
    if (act >= 0) for (int i = 0; i < P->success.n; i++) if (P->success.cards[i] == act) { in_success = 1; break; }
    int card_ids[RB_MAX_ZONE]; int nids = 0;
    if (self_target && in_success) {
        for (int i = 0; i < P->live.n && nids < RB_MAX_ZONE; i++) card_ids[nids++] = P->live.cards[i];
        for (int i = 0; i < P->success.n && nids < RB_MAX_ZONE; i++) card_ids[nids++] = P->success.cards[i];
    } else {
        for (int i = 0; i < P->live.n && nids < RB_MAX_ZONE; i++) card_ids[nids++] = P->live.cards[i];
    }

    /* Filter by self_target / group / original score. */
    int recv[RB_MAX_ZONE]; int nr = 0;
    for (int i = 0; i < nids && nr < RB_MAX_ZONE; i++) {
        int cid = card_ids[i];
        /* required_hearts.rs:152-157 -- `if let Some(act) = gs.activating_card`
           means the filter is SKIPPED entirely when there is no activating card,
           so a self-targeted effect with no host still reaches the pool. The C
           rejected every card in that state. */
        if (self_target && act >= 0 && cid != act) continue;
        if (group && !rb_card_matches_group_str(cid, group)) continue;
        /* score.rs:145-159 `OriginalScoreFilter::accepts` -- the threshold is
           applied only when the text stated a score, a count AND an operator;
           an incomplete qualifier ACCEPTS rather than filtering the card out,
           and an operator the engine does not know is a parser gap, also an
           accept. The C rejected on a missing operator and on an undecodable
           card, which is the opposite of Rust on both. */
        if (original_value && original_operator && has_original_count) {
            Card c;
            if (rb_decode_card_by_index((uint32_t)cid, &c)) {
                int score = (int)c.score;
                rb_free_card(&c);
                if (!rb_compare_counts(original_operator, score, original_count)) continue;
            }
        }
        recv[nr++] = cid;
    }

    /* Resolve colours (default heart00) and apply per-colour modifiers. */
    int colors[8]; int ncol = sc_parse_colors(sc_extra(e, "heart_colors"), colors, 8);
    if (ncol == 0) { colors[0] = 0; ncol = 1; }
    int per_color_value = value;
    for (int ci = 0; ci < ncol; ci++) {
        int color = colors[ci];
        for (int i = 0; i < nr; i++) {
            int cid = recv[i];
            int delta;
            if (!strcmp(operation, "decrease")) delta = -value;
            else if (!strcmp(operation, "increase")) delta = value;
            else if (!strcmp(operation, "set")) delta = per_color_value;
            else continue;
            if (!strcmp(operation, "set"))
                rb_mods_set_need_heart(&gs->mods, cid, color, (int16_t)per_color_value);
            else
                rb_mods_add_need_heart(&gs->mods, cid, color, delta);
        }
    }
    return 0;
}

/* Faithful port of execute_modify_required_hearts_standard (score.rs:686-734). */
void rb_execute_modify_required_hearts_standard(GameState *gs, int actor,
        const char *operation, int value, const char **heart_colors, int n_colors,
        const char *target, const char *effect_text) {
    if (!gs) return;
    int colors[8]; int ncol = 0;
    if (!heart_colors || n_colors <= 0) { colors[0] = 0; ncol = 1; }
    else { for (int i = 0; i < n_colors && ncol < 8; i++) colors[ncol++] = rb_heart_index(rb_parse_heart_color(heart_colors[i])); }

    int pl = rb_target_player_index(target, actor == 0 ? "p1" : "p2");
    if (pl < 0) pl = actor;
    const RbPlayer *P = &gs->p[pl];

    for (int ci = 0; ci < ncol; ci++) {
        int color = colors[ci];
        for (int i = 0; i < P->live.n; i++) {
            int cid = P->live.cards[i];
            int modifier = !strcmp(operation, "increase") ? value
                         : !strcmp(operation, "decrease") ? -value : 0;
            rb_mods_add_need_heart(&gs->mods, cid, color, (int16_t)modifier);
        }
    }
    (void)effect_text;
}

/* Faithful port of execute_modify_yell_count (score.rs:736-759). */
int rb_execute_modify_yell_count(GameState *gs, int actor, AbilityEffect *e) {
    if (!gs || !e) return -1;
    const char *operation = sc_extra(e, "operation"); if (!operation) operation = "subtract";
    int count = e->count >= 0 ? e->count : sc_extra_int(e, "count");
    int slot = (actor == 1) ? 2 : 1;
    if (!strcmp(operation, "add")) rb_add_yell_count_modifier(gs, (uint8_t)slot, (int32_t)count);
    else if (!strcmp(operation, "subtract")) rb_add_yell_count_modifier(gs, (uint8_t)slot, -(int32_t)count);
    return 0;
}

/* Faithful port of execute_modify_limit (score.rs:761-790). */
int rb_execute_modify_limit(GameState *gs, int actor, AbilityEffect *e) {
    if (!gs || !e) return -1;
    const char *operation = sc_extra(e, "operation"); if (!operation) operation = "decrease";
    int count = e->count >= 0 ? e->count : sc_extra_int(e, "count");
    if (gs->n_prohibition < 64) {
        /* restriction.rs:253-263 -- only the two known operations register an
           entry. The C pushed an EMPTY prohibition string for anything else,
           so an unknown op left a blank entry in the prohibition list. */
        char *b = gs->prohibition[gs->n_prohibition];
        if (!strcmp(operation, "decrease")) {
            snprintf(b, 48, "limit_decrease:%d", count);
            gs->n_prohibition++;
        } else if (!strcmp(operation, "increase")) {
            snprintf(b, 48, "limit_increase:%d", count);
            gs->n_prohibition++;
        }
    }
    (void)actor;
    return 0;
}

/* Faithful port of execute_modify_required_hearts_success (score.rs:792-853). */
int rb_execute_modify_required_hearts_success(GameState *gs, int actor, AbilityEffect *e) {
    if (!gs || !e) return -1;
    const char *operation = sc_extra(e, "operation"); if (!operation) operation = "increase";
    int value = e->count >= 0 ? e->count : sc_extra_int(e, "value");
    const char *target = e->target ? e->target : "self";
    const char *card_type = sc_extra(e, "card_type");
    const char *heart_colors = sc_extra(e, "heart_colors");

    int pl = rb_target_player_index(target, actor == 0 ? "p1" : "p2");
    if (pl < 0) pl = actor;
    RbPlayer *P = &gs->p[pl];

    int card_ids[RB_MAX_ZONE]; int nids = 0;
    if (card_type && !strcmp(card_type, "live_card")) {
        for (int i = 0; i < P->success.n && nids < RB_MAX_ZONE; i++) card_ids[nids++] = P->success.cards[i];
    }

    int delta = !strcmp(operation, "increase") ? value
              : !strcmp(operation, "decrease") ? -value : 0;
    if (delta == 0) return 0;

    int colors[8]; int ncol = sc_parse_colors(heart_colors, colors, 8);
    if (ncol == 0) { for (int i = 0; i < 7; i++) colors[i] = i; ncol = 7; }

    for (int i = 0; i < nids; i++) {
        int cid = card_ids[i];
        for (int ci = 0; ci < ncol; ci++) {
            rb_mods_add_need_heart(&gs->mods, cid, colors[ci], (int16_t)delta);
        }
    }
    return 0;
}
