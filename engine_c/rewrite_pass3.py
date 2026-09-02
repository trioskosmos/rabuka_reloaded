import re, sys

p = 'tools/gen_tests.py'
src = open(p, encoding='utf-8').read()

# Replace the entire pass-3 block (from the "# --- pass 3:" comment through
# "# --- pass 4b:" comment) with a simpler, robust token-based scan.
start = src.index('    # --- pass 3: add missing int declarations')
end = src.index('    # --- pass 4b: inline g.id("X")')

new_pass3 = '''    # --- pass 3: add missing int declarations for undeclared bare locals ---
    # Robust token scan: for each function, collect every identifier used in
    # real (non-comment) code that is not declared, not a known global, and
    # not a C keyword/type.  Inject `int VAR = 0;` after the TestGame tg;
    # declaration so the C compiles.  This is a safety net — most vars are
    # already declared by the line-based transpiler's `let` handling.
    known = {"tg","tg2","failures","RB_ZONE_HAND","RB_ZONE_STAGE","RB_PHASE_MAIN","RB_PHASE_ACTIVE","RB_PHASE_ENERGY","RB_PHASE_DRAW","RB_PHASE_LIVE_SET","RB_PHASE_PERFORMANCE","RB_PHASE_VICTORY","RB_PHASE_OPENING","RB_PHASE_RPS","RB_HEART_PINK","RB_HEART_RED","RB_HEART_YELLOW","RB_HEART_GREEN","RB_HEART_BLUE","RB_HEART_PURPLE","RB_HEART_ORANGE","RB_HEART_ALL","RB_HEART_DRAW","RB_HEART_SCORE","RB_HEART_ANY","RB_MAX_CARD_IDS","RB_MAX_RECENTLY_MOVED","int","Card","TestGame","void","static","if","for","while","return","CHECK","CHECK_EQ","CHECK_EQ_STR","test_id","test_add_to_deck","test_add_to_hand","test_add_to_discard","test_add_to_live","test_add_to_success","test_add_to_energy","test_add_to_deck_pl","test_add_to_revealed","test_add_to_stage","test_play_to_stage","test_activate_ability","test_give_energy","test_spend_energy","test_recalc","test_clear_mods_for_card","test_set_live_card","test_has_pending_choice","test_pending_choice_count","test_pending_choice_type","test_get_blade_modifier","test_get_score_modifier","test_get_cost_modifier","test_get_heart_modifier","test_zone_has_id","test_zone_has_card_no","test_filler_hand","test_insert_deck_top","test_set_energy_active","test_place_under","test_drain_auto_choices","test_answer_play_cost_choice","rb_mods_get_cost","rb_mods_get_score","rb_mods_get_blade","rb_mods_get_heart","rb_mods_set_orientation","rb_advance_phase","rb_has_pending_choice","rb_resume_with_choice","rb_drain_ability_queue","rb_trigger_live_start","rb_queue_current_entry","rb_queue_is_empty","rb_phase_name","rb_load","rb_unload","rb_find_card_by_no","rb_decode_card_by_index","rb_card_is_live","rb_card_is_energy","rb_owner_of_card","rb_zone_of_str","rb_parse_heart_color","rb_heart_index","rb_give_energy","rb_pass","rb_perform_live","rb_record_event","rb_fire_recorded_auto","rb_queue_trigger_abilities","rb_use_count","rb_use_limit_reached","rb_pos_change_for_player","rb_misc_position_destinations","printf","fprintf","strstr","strcmp","stderr","__FILE__","__LINE__","i","l","score","idx","hc","need","copy","guard","shizuku","ally","shi","honoka","member","card","mira1","mira2","ren","koko","natsume","e1","e2","n2","mia_discarded","total_heart02","id","hb","got_blade","edel1","edel2","edel3","before","swc","c1","c2","b0","b1","b2","hand_before","filler","dive_p1","setsuna","live","trapper","p1_member","p1_live","p2_member","outsider","yoshiko","mari","keke","liella","genki","awaken","mw","p2_live","v","g","g2","game2","self","db","game","s","NIJI_LIVES","FILLER","DIVE","P1_MEMBER","P1_LIVE","P2_MEMBER","TRAPPER","MIRAKURA_RURINO","HIMEKO","KOKO","NON_MIRAKURA_MEMBER","LIVE","MEMBER","LIVECARD","ENERGY","HAND","STAGE","WAITROOM","DISCARD","DECK","SUCCESS","LIVES","HANDCARD","STAGECARD","ENERGYCARD","WAITROOMCARD","DISCARDCARD","DECKCARD","SUCCESSCARD","LIVESCARD","HANDCARD","STAGECARD","ENERGYCARD","WAITROOMCARD","DISCARDCARD","DECKCARD","SUCCESSCARD","LIVESCARD"}
    keywords = {"int","void","char","float","double","long","short","unsigned","signed","const","static","struct","union","enum","if","else","for","while","do","return","break","continue","switch","case","default","sizeof","typedef","goto","true","false","NULL","void","auto","register","volatile","extern","inline","restrict"}
    final_lines = text2.splitlines()
    func_ranges = []
    cur = None
    start = 0
    for idx, ln in enumerate(final_lines):
        if re.match(r'\s*static void gen_\w+\(', ln):
            if cur is not None:
                func_ranges.append((cur, start, idx-1))
            cur = re.match(r'\s*static void (gen_\w+)\(', ln).group(1)
            start = idx
    if cur is not None:
        func_ranges.append((cur, start, len(final_lines)-1))
    for fname, s, e in func_ranges:
        body = "\n".join(final_lines[s:e+1])
        declared = set(re.findall(r'\bint\s+(\w+)\b', body))
        declared.update(re.findall(r'\bCard\s+(\w+)\b', body))
        declared.update(re.findall(r'\bchar\s+(\w+)\b', body))
        declared.add('tg'); declared.add('tg2')
        # tokens used in real (non-comment) code
        real = re.sub(r'//.*', '', body)
        real = re.sub(r'/\\*.*?\\*/', '', real, flags=re.DOTALL)
        used = set(re.findall(r'[A-Za-z_]\\w*', real))
        # tokens used as the first word of a statement that is NOT a known call
        # (i.e. not `test_*(&tg` / `rb_*` / `CHECK` / `int` / `tg.state` / `if`)
        missing = set()
        for tok in used:
            if tok in known or tok in declared or tok in keywords:
                continue
            if not re.match(r'^[A-Za-z_]\\w*$', tok):
                continue
            # skip tokens that are actually function calls (followed by '(')
            if re.search(r'\b' + re.escape(tok) + r'\s*\\(', real):
                continue
            # skip tokens that are field accesses (preceded by '.')
            if re.search(r'\\.' + re.escape(tok) + r'\b', real):
                continue
            missing.add(tok)
        if missing:
            insert_idx = None
            for idx in range(s, e+1):
                if 'TestGame tg;' in final_lines[idx]:
                    insert_idx = idx
                    break
            if insert_idx is not None:
                indent = "    "
                for var in sorted(missing):
                    if var in declared:
                        continue
                    final_lines.insert(insert_idx+1, f"{indent}int {var} = 0; // auto-fix missing decl")
                    insert_idx += 1
                    declared.add(var)
                    e += 1
    # --- pass 4b: inline g.id("X") / v.id("X") / g2.id("X") / g.new_id("X") ->'''

src = src[:start] + new_pass3 + src[end:]
open(p, 'w', encoding='utf-8').write(src)
import py_compile; py_compile.compile(p, doraise=True); print('gen_tests rewritten + compiles')