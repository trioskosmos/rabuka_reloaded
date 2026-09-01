with open('src/ability/condition.c', 'r') as f:
    lines = f.readlines()

start_idx = None
end_idx = None
for i, line in enumerate(lines):
    if 'static int eval_group(const struct GameState *g, int actor, int host_cid, const Condition *c) {' in line:
        start_idx = i
    if start_idx is not None and 'static int eval_group_aggregate(const GameState *g, int actor, const Condition *c) {' in line:
        end_idx = i
        break

if start_idx is None or end_idx is None:
    print(f"start={start_idx}, end={end_idx}")
    exit(1)

new_func = '''static int eval_group(const struct GameState *g, int actor, int host_cid, const Condition *c) {
    (void)host_cid;
    const char *loc = get_str(c, "location");
    if (!loc) loc = "stage";
    int pl = target_player_idx(actor, c);
    int all_members_val = 0;
    int all_members = get_i(c, "all_members", &all_members_val) ? all_members_val : 0;
    if (all_members) {
        const CondValue *gv = find_val(c, "group_names");
        if (!gv || gv->tag != RB_TAG_ARRAY || gv->arr_n == 0) return 0;
        int ids[RB_MAX_ZONE]; int n = zone_ids(g, pl, loc, ids, RB_MAX_ZONE);
        if (n == 0) return 0;
        for (int i = 0; i < n; i++) {
            int matched = 0;
            for (uint32_t gi = 0; gi < gv->arr_n; gi++) {
                const char *t = (gv->arr[gi].tag == RB_TAG_STR) ? gv->arr[gi].s : NULL;
                if (t && rb_card_matches_group_str(ids[i], t)) { matched = 1; break; }
            }
            if (!matched) return 0;
        }
        return 1;
    }
    { int agg = eval_group_aggregate(g, actor, c); if (agg >= 0) return agg; }
    const CondValue *hc = find_val(c, "heart_colors");
    if (hc && hc->tag == RB_TAG_ARRAY && hc->arr_n > 0) {
        int ids[RB_MAX_ZONE]; int n = zone_ids(g, pl, loc, ids, RB_MAX_ZONE);
        int required = 0;
        for (uint32_t k = 0; k < hc->arr_n; k++) {
            int col = RB_HEART_PINK;
            if (hc->arr[k].tag == RB_TAG_I64) col = (int)hc->arr[k].i;
            else if (hc->arr[k].tag == RB_TAG_STR && hc->arr[k].s) col = atoi(hc->arr[k].s);
            int found = 0;
            for (int i = 0; i < n && !found; i++) {
                if (ids[i] < 0) continue;
                Card cc;
                if (rb_decode_card_by_index((uint32_t)ids[i], &cc)) {
                    for (int h = 0; h < cc.n_hearts; h++) {
                        if (cc.heart_color[h] == (uint8_t)col) { found = 1; break; }
                    }
                    rb_free_card(&cc);
                }
            }
            if (found) required++;
        }
        if (required < (int)hc->arr_n) return 0;
        return 1;
    }
    { int multi = eval_group_multi(g, actor, c); if (multi >= 0) return multi; }
    int temporal = 0; get_bool(c, "temporal", &temporal);
    int self_target = 0; get_bool(c, "self_target", &self_target);
    const CondValue *gn = find_val(c, "group_names");
    if (temporal && self_target && gn && gn->tag == RB_TAG_ARRAY && gn->arr_n > 0) {
        const char *ct = get_str(c, "card_type");
        if (ct && !strcmp(ct, "member_card")) {
            int activating_card = g->activating_card;
            if (activating_card >= 0) {
                int ids[RB_MAX_ZONE]; int n = zone_ids(g, pl, loc, ids, RB_MAX_ZONE);
                int matched = 0;
                for (uint32_t gi = 0; gi < gn->arr_n && !matched; gi++) {
                    const char *t = (gn->arr[gi].tag == RB_TAG_STR) ? gn->arr[gi].s : NULL;
                    if (!t) continue;
                    for (int i = 0; i < n; i++) {
                        if (ids[i] < 0) continue;
                        if (rb_card_matches_group_str(ids[i], t)) { matched = 1; break; }
                    }
                }
                return matched;
            }
            return 0;
        }
    }
    int ids[RB_MAX_ZONE]; int n = zone_ids(g, pl, loc, ids, RB_MAX_ZONE);
    if (n == 0) return 0;
    for(int i=0;i<n;i++){
        Card card; if(!rb_decode_card_by_index((uint32_t)ids[i],&card)) continue;
        const char *gname = rb_card_string(card.group_idx);
        const char *uname = rb_card_string(card.unit_idx);
        if (rb_ability_debug_enabled() && gname && (strstr(gname,"XX")||strstr(gname,"YY"))) {
            for(uint32_t gi=0;gi<gn->arr_n;gi++) if(gn->arr[gi].tag==RB_TAG_STR && gn->arr[gi].s)
                fprintf(stderr,"[grp] card=%s gname=%s uname=%s target=%s\\n",
                        card.name?card.name:"?", gname, uname?uname:"-", gn->arr[gi].s);
        }
        for(uint32_t gi=0;gi<gn->arr_n;gi++){
            const char *t = (gn->arr[gi].tag==RB_TAG_STR)?gn->arr[gi].s:NULL;
            if(!t) continue;
            if(rb_card_matches_group_str(ids[i], t)) { rb_free_card(&card); return 1; }
        }
        rb_free_card(&card);
    }
    return 0;
}

'''

lines = lines[:start_idx] + [new_func] + lines[end_idx:]
with open('src/ability/condition.c', 'w') as f:
    f.writelines(lines)
print(f"Replaced lines {start_idx+1} to {end_idx+1}")