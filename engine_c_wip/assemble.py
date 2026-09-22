import re, glob

HEADER = r'''
/* === Assembled choice resolver (ports engine/src/ability/choice.rs) === */
typedef struct RbSelectionContext { int indices[RB_STAGE_SIZE]; int n; } RbSelectionContext;
typedef RbSelectionContext SelectionContext;
typedef struct RbExecutionContext { int dummy; } RbExecutionContext;
typedef struct RbAbilityResolver {
    GameState *gs;
    int actor;
    int host_cid;
    RbChoice pending_choice;
    int choice_card_no;
    int conditional_choice;
    int entry_choice_card_no;
    AbilityEffect *current_effect;
    AbilityEffect *entry_effect;
    void *exec_ctx;
    void *execution_context;
    int formation_plan[RB_STAGE_SIZE];
    int n_formation_plan;
    int selected_area;
    int selected_cards[RB_MAX_RECENTLY_MOVED];
    int n_selected_cards;
    int moved_cards[RB_MAX_RECENTLY_MOVED];
    int n_moved_cards;
    int activating_card;
    int sub_choice_created;
    int has_pending_choice;
    int has_pending_reprompt;
    int has_pending_reprompt_choice;
    int deferred_conditional_gate;
    int pending_deferred_costs[16];
    int n_pending_deferred_costs;
    int pending_reprompt_choice[16];
    int spawn_target;
    int spawn_target_set;
} RbAbilityResolver;
typedef RbAbilityResolver RbResolver;
typedef RbAbilityResolver AbilityResolver;
'''

STRIP_RES = [
    re.compile(r'typedef\s+struct\s+RbAbilityResolver\s*\{.*?\}\s*RbAbilityResolver\s*;', re.S),
    re.compile(r'struct\s+RbAbilityResolver\s*\{.*?\}\s*;', re.S),
    re.compile(r'typedef\s+struct\s+RbSelectionContext\s*\{.*?\}\s*RbSelectionContext\s*;', re.S),
    re.compile(r'struct\s+RbSelectionContext\s*\{.*?\}\s*;', re.S),
    re.compile(r'typedef\s+struct\s+RbExecutionContext\s*\{.*?\}\s*RbExecutionContext\s*;', re.S),
    re.compile(r'struct\s+RbExecutionContext\s*\{.*?\}\s*;', re.S),
    re.compile(r'typedef\s+RbAbilityResolver\s+RbResolver\s*;'),
    re.compile(r'typedef\s+RbAbilityResolver\s+AbilityResolver\s*;'),
    re.compile(r'typedef\s+RbSelectionContext\s+SelectionContext\s*;'),
]

SHIMS = r'''
int rb_get_card(int id, Card *out) { return rb_decode_card_by_index((uint32_t)id, out); }
int rb_card_db_unit(int id) { (void)id; return 0; }
int rb_ability_master_id(int id) { (void)id; return 0; }
int rb_choice_destination(const GameState *g, int *out) { (void)g;(void)out; return 0; }
int rb_compound_route_conditional_branch(const AbilityEffect *e) { (void)e; return 0; }
int rb_effect_answers_any(const AbilityEffect *e) { (void)e; return 0; }
int rb_effect_resource_on_select(const AbilityEffect *e) { (void)e; return 0; }
int rb_effect_alternative_count_type_any(const AbilityEffect *e) { (void)e; return 0; }
AbilityEffect *rb_entry_effect(GameState *g) { return g && g->queue.resume_eff ? g->queue.resume_eff : NULL; }
int rb_entry_destination(const GameState *g) { (void)g; return 0; }
int rb_entry_conditional_choice_effect(const GameState *g) { (void)g; return 0; }
int rb_resolver_build_choice_select_cards(RbAbilityResolver *self, GameState *g) { (void)self;(void)g; return 0; }
int rb_resolver_card_name(GameState *g, int id, char *out, int outsz) {
    Card c; if (rb_decode_card_by_index((uint32_t)id, &c)) { if(out&&outsz)snprintf(out,outsz,"%s",c.name?c.name:""); rb_free_card(&c); return 1;} return 0;
}
int rb_resolver_entry_effect(RbAbilityResolver *self) { (void)self; return 0; }
int rb_resolver_look_select_finalize_dest(GameState *g, int idx) { (void)g;(void)idx; return 0; }
int rb_resolver_spawn_target(RbAbilityResolver *self, GameState *g, int t) { (void)self;(void)g;(void)t; return 0; }
int rb_select_target_kind_from_str(const char *s) { (void)s; return 0; }
'''

frags = sorted(glob.glob('src/ability/choice_frag_*.c'))
parts = []
for f in frags:
    t = open(f, encoding='utf-8', errors='ignore').read()
    for r in STRIP_RES:
        t = r.sub('', t)
    parts.append(t)

def split_funcs(text):
    out = []
    pat = re.compile(r'(?:static\s+|inline\s+)?([A-Za-z_][\w\s\*]*?)\s+((?:rb_|Rb_|rb_sc_|rb_selection_)[A-Za-z0-9_]+)\s*\(([^;{]*?)\)\s*\{', re.S)
    for m in pat.finditer(text):
        start = m.start()
        depth = 0; j = m.end()-1
        while j < len(text):
            if text[j] == '{': depth += 1
            elif text[j] == '}':
                depth -= 1
                if depth == 0: break
            j += 1
        out.append((m.group(2), text[start:j+1]))
    return out

seen = set(); uniq = []
for txt in parts:
    for name, body in split_funcs(txt):
        if name in seen: continue
        seen.add(name); uniq.append((name, body))

protos = []
for name, body in uniq:
    sig = body[:body.index('{')].strip()
    sig = re.sub(r'^\s*(static|inline|extern)\s+', '', sig)
    protos.append(sig + ';')

import subprocess
base = subprocess.run(['git', 'show', 'HEAD:engine_c/src/ability/choice.c'],
                      cwd='.', capture_output=True).stdout.decode('utf-8', 'ignore')
body0 = base
for inc in ('#include "rabuka.h"', '#include <string.h>', '#include <stdio.h>'):
    body0 = body0.replace(inc, '', 1)

out = '/* ===== AUTO-ASSEMBLED from choice.rs port fragments ===== */\n'
out += '#include "rabuka.h"\n#include <string.h>\n#include <stdio.h>\n#include <stdlib.h>\n'
out += HEADER + '\n' + '\n'.join(protos) + '\n' + SHIMS + '\n'
out += '/* ---- original choice.c (mode dispatch) ---- */\n' + body0 + '\n'
out += '/* ---- ported choice.rs functions ---- */\n' + '\n\n'.join(b for _, b in uniq) + '\n'
open('src/ability/choice.c', 'w', encoding='utf-8').write(out)
print('assembled choice.c funcs=', len(uniq))
