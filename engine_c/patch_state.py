import re, sys

path = r"C:\Users\trios\OneDrive\Documents\rabuka_reloaded\engine_c\src\ability\effects\state.c"
with open(path, "r", encoding="utf-8") as f:
    src = f.read()

new_func = '''void rb_effect_modify_hearts(GameState *g, int actor, AbilityEffect *e){
    /* Faithful mirror of engine/src/ability/effects/score.rs::execute_modify_required_hearts:
       apply add/set need_heart modifiers (operation: decrease/increase/set) to the
       target player's live cards, for each listed heart color, scaled by per_unit. */
    int value = e->count >= 0 ? e->count : 1;
    const char *op = "decrease"; int is_set = 0; int sign = -1;
    const char *grp = NULL; const char *loc = NULL; int per_unit = 0; int per_unit_count = 1;
    for (int i = 0; i < e->n_extra; i++) {
        if (!e->extra_k[i]) continue;
        if (!strcmp(e->extra_k[i], "operation") && e->extra_v[i]) {
            op = e->extra_v[i];
            if (!strcmp(op, "increase")) { sign = 1; is_set = 0; }
            else if (!strcmp(op, "set")) { sign = 1; is_set = 1; }
            else { sign = -1; is_set = 0; }
        } else if (!strcmp(e->extra_k[i], "group_names") || !strcmp(e->extra_k[i], "group_name")) {
            if (e->extra_v[i]) grp = e->extra_v[i];
        } else if (!strcmp(e->extra_k[i], "per_unit") && e->extra_v[i] && !strcmp(e->extra_v[i], "true")) {
            per_unit = 1;
        } else if (!strcmp(e->extra_k[i], "location")) {
            loc = e->extra_v[i];
        } else if (!strcmp(e->extra_k[i], "per_unit_count") && e->extra_v[i]) {
            per_unit_count = atoi(e->extra_v[i]);
        }
    }
    /* colors (default heart00) from heart_colors / heart_color (comma list) */
    int cols[8]; int nc = 0;
    for (int i = 0; i < e->n_extra && nc < 8; i++) {
        if (e->extra_k[i] && (!strcmp(e->extra_k[i], "heart_colors") || !strcmp(e->extra_k[i], "heart_color")) && e->extra_v[i]) {
            cols[nc++] = heart_color_of((AbilityEffect*)e, 0);
            break;
        }
    }
    if (nc == 0) cols[nc++] = 0;
    RbPlayer *Pp = (e->target && !strcmp(e->target, "opponent")) ? &g->p[actor ^ 1] : &g->p[actor];
    if (per_unit) {
        int units = Pp->live.n;
        if (loc && (!strcmp(loc, "success_live_zone") || !strcmp(loc, "live_zone") || !strcmp(loc, "success_live_card_zone")))
            units = Pp->success.n;
        if (per_unit_count < 1) per_unit_count = 1;
        value = value * (units / per_unit_count);
    }
    int who = (e->target && !strcmp(e->target, "opponent")) ? actor ^ 1 : actor;
    RbPlayer *P = &g->p[who];
    for (int i = 0; i < P->live.n; i++) {
        int cid = P->live.cards[i];
        if (grp && !rb_card_matches_group_str(cid, grp)) continue;
        for (int c = 0; c < nc; c++) {
            if (is_set) rb_mods_set_need_heart(&g->mods, cid, cols[c], value);
            else       rb_mods_add_need_heart(&g->mods, cid, cols[c], value * sign);
        }
    }
}
'''

# Replace the existing rb_effect_modify_hearts(...) { ... } (first top-level match).
pattern = re.compile(r'void\s+rb_effect_modify_hearts\(GameState\s*\*g,\s*int\s*actor,\s*AbilityEffect\s*\*e\)\s*\{.*?\n\}', re.DOTALL)
m = pattern.search(src)
if not m:
    print("FUNCTION NOT FOUND")
    sys.exit(1)
src = src[:m.start()] + new_func + src[m.end():]
with open(path, "w", encoding="utf-8") as f:
    f.write(src)
print("replaced rb_effect_modify_hearts OK")
