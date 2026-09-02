with open('engine_c/src/ability/effects/state.c', 'r') as f:
    content = f.read()

# Fix all the card property functions to take const Card* instead of int card_id
replacements = [
    # total_hearts
    ('int rb_card_total_hearts(int card_id) {\n    Card c;\n    if (!rb_decode_card_by_index((uint32_t)card_id, &c)) return 0;\n    int total = 0;\n    for (int h = 0; h < c.n_hearts; h++) total += c.heart_count[h];\n    rb_free_card(&c);\n    return total;\n}',
     'int rb_card_total_hearts(const Card *c) {\n    if (!c) return 0;\n    int total = 0;\n    for (int h = 0; h < c->n_hearts; h++) total += c->heart_count[h];\n    return total;\n}'),
    
    # has_blade_heart
    ('int rb_card_has_blade_heart(int card_id) {\n    Card c;\n    if (!rb_decode_card_by_index((uint32_t)card_id, &c)) return 0;\n    int r = (c.blade > 0) || (c.has_special && c.special_count > 0);\n    rb_free_card(&c);\n    return r;\n}',
     'int rb_card_has_blade_heart(const Card *c) {\n    if (!c) return 0;\n    return (c->blade > 0) || (c->has_special && c->special_count > 0);\n}'),
    
    # has_score_icon
    ('int rb_card_has_score_icon(int card_id) {\n    Card c;\n    if (!rb_decode_card_by_index((uint32_t)card_id, &c)) return 0;\n    int r = (c.has_special && c.special_color == RB_HEART_SCORE);\n    rb_free_card(&c);\n    return r;\n}',
     'int rb_card_has_score_icon(const Card *c) {\n    if (!c) return 0;\n    return (c->has_special && c->special_color == RB_HEART_SCORE);\n}'),
    
    # has_all_blade
    ('int rb_card_has_all_blade(int card_id) {\n    Card c;\n    if (!rb_decode_card_by_index((uint32_t)card_id, &c)) return 0;\n    int r = 0;\n    for (int h = 0; h < c.n_hearts; h++)\n        if (c.heart_color[h] == 7 && c.heart_count[h] > 0) { r = 1; break; }\n    rb_free_card(&c);\n    return r;\n}',
     'int rb_card_has_all_blade(const Card *c) {\n    if (!c) return 0;\n    int r = 0;\n    for (int h = 0; h < c->n_hearts; h++)\n        if (c->heart_color[h] == 7 && c->heart_count[h] > 0) { r = 1; break; }\n    return r;\n}'),
    
    # get_score
    ('int rb_card_get_score(int card_id) {\n    Card c;\n    if (!rb_decode_card_by_index((uint32_t)card_id, &c)) return 0;\n    int s = c.score;\n    rb_free_card(&c);\n    return s;\n}',
     'int rb_card_get_score(const Card *c) {\n    if (!c) return 0;\n    return c->score;\n}'),
    
    # parse_heart_color
    ('int rb_parse_heart_color(const char *s) {\n    if (!s) return 0;\n    if (!strcmp(s, "heart00") || !strcmp(s, "h00") || !strcmp(s, "heart07") || !strcmp(s, "b_heart07")) return 0;\n    if (!strcmp(s, "heart01") || !strcmp(s, "h01")) return 1;\n    if (!strcmp(s, "heart02") || !strcmp(s, "h02")) return 2;\n    if (!strcmp(s, "heart03") || !strcmp(s, "h03")) return 3;\n    if (!strcmp(s, "heart04") || !strcmp(s, "h04")) return 4;\n    if (!strcmp(s, "heart05") || !strcmp(s, "h05")) return 5;\n    if (!strcmp(s, "heart06") || !strcmp(s, "h06")) return 6;\n    if (!strcmp(s, "all") || !strcmp(s, "b_all")) return 7;\n    if (strncmp(s, "b_", 2) == 0) return rb_parse_heart_color(s + 2);\n    return 0;\n}',
     'RbHeartColor rb_parse_heart_color(const char *s) {\n    if (!s) return 0;\n    if (!strcmp(s, "heart00") || !strcmp(s, "h00") || !strcmp(s, "heart07") || !strcmp(s, "b_heart07")) return 0;\n    if (!strcmp(s, "heart01") || !strcmp(s, "h01")) return 1;\n    if (!strcmp(s, "heart02") || !strcmp(s, "h02")) return 2;\n    if (!strcmp(s, "heart03") || !strcmp(s, "h03")) return 3;\n    if (!strcmp(s, "heart04") || !strcmp(s, "h04")) return 4;\n    if (!strcmp(s, "heart05") || !strcmp(s, "h05")) return 5;\n    if (!strcmp(s, "heart06") || !strcmp(s, "h06")) return 6;\n    if (!strcmp(s, "all") || !strcmp(s, "b_all")) return 7;\n    if (strncmp(s, "b_", 2) == 0) return rb_parse_heart_color(s + 2);\n    return 0;\n}'),
]

for old, new in replacements:
    content = content.replace(old, new)

with open('engine_c/src/ability/effects/state.c', 'w') as f:
    f.write(content)
print('Done')