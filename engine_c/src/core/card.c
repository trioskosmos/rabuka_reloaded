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