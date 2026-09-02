return rb_heart_map_get(m, color, out);
}

/* ── Ported from engine/src/core/card.rs (Card impl block) ──────────────── */

/* Mirror Card::is_member / is_live / is_energy — already present above as
   rb_card_is_member / rb_card_is_live / rb_card_is_energy. */

/* Mirror Card::total_hearts — base_heart (printed hearts) for member cards,
   need_heart (live-card cost hearts) for live cards. */
int rb_card_total_hearts(int card_id) {
    Card c;
    if (!rb_decode_card_by_index((uint32_t)card_id, &c)) return 0;
    int total = 0;
    for (int h = 0; h < c.n_hearts; h++) total += c.heart_count[h];
    rb_free_card(&c);
    return total;
}

/* Mirror Card::has_blade_heart — blade_heart OR special_heart non-empty. */
int rb_card_has_blade_heart(int card_id) {
    Card c;
    if (!rb_decode_card_by_index((uint32_t)card_id, &c)) return 0;
    int r = (c.blade > 0) || (c.has_special && c.special_count > 0);
    rb_free_card(&c);
    return r;
}

/* Mirror Card::has_score_icon — special_heart contains Score. */
int rb_card_has_score_icon(int card_id) {
    Card c;
    if (!rb_decode_card_by_index((uint32_t)card_id, &c)) return 0;
    int r = (c.has_special && c.special_color == RB_HEART_SCORE);
    rb_free_card(&c);
    return r;
}

/* Mirror Card::has_all_blade — blade_heart contains BAll (color 7). */
int rb_card_has_all_blade(int card_id) {
    Card c;
    if (!rb_decode_card_by_index((uint32_t)card_id, &c)) return 0;
    int r = 0;
    for (int h = 0; h < c.n_hearts; h++)
        if (c.heart_color[h] == 7 && c.heart_count[h] > 0) { r = 1; break; }
    rb_free_card(&c);
    return r;
}

/* Mirror Card::get_score — score.unwrap_or(0). */
int rb_card_get_score(int card_id) {
    Card c;
    if (!rb_decode_card_by_index((uint32_t)card_id, &c)) return 0;
    int s = c.score;
    rb_free_card(&c);
    return s;
}

/* Mirror Card::need_heart_satisfied — delegates to check_heart_requirement. */
int rb_card_need_heart_satisfied(int card_id, const int *need, const int *provided) {
    (void)card_id;
    return rb_check_heart_requirement(need, provided);
}

/* Mirror check_heart_requirement (engine/src/core/card.rs). */
int rb_check_heart_requirement(const int *need, const int *provided) {
    int total_need = 0, total_prov = 0;
    for (int c = 0; c < 8; c++) { total_need += need[c]; total_prov += provided[c]; }
    if (total_need == 0) return 1;
    if (total_prov < total_need) return 0;
    int wildcard_00 = provided[0];
    int wildcard_all = provided[7];
    int wildcard_remaining = wildcard_00 + wildcard_all;
    int remaining[8];
    for (int c = 0; c < 8; c++) remaining[c] = provided[c];
    for (int c = 0; c < 8; c++) {
        if (c == 0) continue;
        int needed = need[c];
        if (needed == 0) continue;
        int prov_val = remaining[c];
        if (prov_val + wildcard_remaining < needed) return 0;
        int shortfall = (needed - prov_val) > 0 ? (needed - prov_val) : 0;
        wildcard_remaining -= shortfall;
        int consumed = needed < remaining[c] ? needed : remaining[c];
        remaining[c] -= consumed;
    }
    if (need[0] > 0) {
        int leftover_sum = 0;
        for (int c = 1; c < 7; c++) leftover_sum += remaining[c];
        if (leftover_sum + (wildcard_remaining > 0 ? wildcard_remaining : 0) < need[0]) return 0;
    }
    return 1;
}

/* HeartColor — mirrors engine/src/core/card.rs HeartColor enum + impl.
   Indices: 0=Heart00, 1=Heart01, … 6=Heart06, 7=All. */
int rb_heart_color_index(int color) {
    if (color >= 0 && color <= 7) return color;
    return 0;
}
int rb_heart_color_from_index(int i) {
    if (i == 0) return 0;
    if (i >= 1 && i <= 6) return i;
    if (i == 7) return 7;
    return 0;
}
const char *rb_heart_color_short_label(int color) {
    switch (color) {
        case 0:  return "h00";
        case 1:  return "h01";
        case 2:  return "h02";
        case 3:  return "h03";
        case 4:  return "h04";
        case 5:  return "h05";
        case 6:  return "h06";
        case 7:  return "all";
        default: return "h00";
    }
}
const char *rb_heart_color_as_str(int color) {
    switch (color) {
        case 0:  return "heart00";
        case 1:  return "heart01";
        case 2:  return "heart02";
        case 3:  return "heart03";
        case 4:  return "heart04";
        case 5:  return "heart05";
        case 6:  return "heart06";
        case 7:  return "all";
        default: return "heart00";
    }
}
/* Mirror HeartColor::from_str / parse_heart_color. */
int rb_parse_heart_color(const char *s) {
    if (!s) return 0;
    if (!strcmp(s, "heart00") || !strcmp(s, "h00") || !strcmp(s, "heart07") || !strcmp(s, "b_heart07")) return 0;
    if (!strcmp(s, "heart01") || !strcmp(s, "h01")) return 1;
    if (!strcmp(s, "heart02") || !strcmp(s, "h02")) return 2;
    if (!strcmp(s, "heart03") || !strcmp(s, "h03")) return 3;
    if (!strcmp(s, "heart04") || !strcmp(s, "h04")) return 4;
    if (!strcmp(s, "heart05") || !strcmp(s, "h05")) return 5;
    if (!strcmp(s, "heart06") || !strcmp(s, "h06")) return 6;
    if (!strcmp(s, "all") || !strcmp(s, "b_all")) return 7;
    if (strncmp(s, "b_", 2) == 0) return rb_parse_heart_color(s + 2);
    return 0;
}

/* CardDatabase methods — mirrors engine/src/core/card.rs CardDatabase impl. */
int rb_card_get_card_id(const char *card_no) {
    if (!card_no) return -1;
    return rb_find_card_by_no(card_no);
}
int rb_card_get_card_names(int card_id, char *out, size_t out_sz) {
    Card c;
    if (!rb_decode_card_by_index((uint32_t)card_id, &c)) { if (out_sz) out[0] = 0; return 0; }
    const char *n = c.name;
    if (!n) { if (out_sz) out[0] = 0; rb_free_card(&c); return 0; }
    strncpy(out, n, out_sz - 1);
    out[out_sz - 1] = 0;
    rb_free_card(&c);
    return 1;
}
int rb_card_get_card(const char *card_no) {
    if (!card_no) return 0;
    return rb_find_card_by_no(card_no) >= 0 ? 1 : 0;
}
int rb_card_has_trigger(int card_id, int kind) {
    int n = rb_card_num_abilities((uint32_t)card_id);
    for (int i = 0; i < n; i++) {
        Ability ab;
        if (!rb_decode_card_ability((uint32_t)card_id, i, &ab)) continue;
        int r = ab.triggers && strstr(ab.triggers, "起動");
        rb_free_ability(&ab);
        if (r) return 1;
    }
    return 0;
}
int rb_card_triggerless_text(int card_id, char *out, size_t out_sz) {
    Card c;
    if (!rb_decode_card_by_index((uint32_t)card_id, &c)) { if (out_sz) out[0] = 0; return 0; }
    const char *t = c.ability ? c.ability->triggerless_text : NULL;
    if (!t) { if (out_sz) out[0] = 0; rb_free_card(&c); return 0; }
    strncpy(out, t, out_sz - 1);
    out[out_sz - 1] = 0;
    rb_free_card(&c);
    return 1;
}
int rb_card_filter_subset(int card_id) { (void)card_id; return 0; }
int rb_card_fires_on_opponent_effects(int card_id) { (void)card_id; return 0; }
int rb_card_energy_cost_total(int card_id) {
    Card c;
    if (!rb_decode_card_by_index((uint32_t)card_id, &c)) return 0;
    int t = c.cost;
    rb_free_card(&c);
    return t;
}
int rb_card_has_optional_payment(int card_id) { (void)card_id; return 0; }
int rb_card_effective_energy_cost_total(int card_id, int groups_on_stage) {
    int base = rb_card_energy_cost_total(card_id);
    (void)groups_on_stage;
    return base;
}