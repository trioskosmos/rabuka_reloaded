import importlib.util, sys, re
spec = importlib.util.spec_from_file_location('gt', 'tools/gen_tests.py')
gt = importlib.util.module_from_spec(spec)
spec.loader.exec_module(gt)

# Build a body that mirrors the failing function and see what pass 3 does.
# We can't easily call _postprocess, so test the candidate logic directly.
body = """
    int guard = 0;
    test_has_pending_choice(&tg);
    // TODO: guard += 1;
    if (idx >= 0) {
    // reuse tg; test_game_new(&tg);
    rb_resume_with_choice(&tg.state, i);
    // TODO: mia_discarded = true;
    // TODO: } else if *count > 0 {
    rb_resume_with_choice(&tg.state, 0);
    // TODO: eprintln!(
    // TODO: "VERIFY P2 snap: success={} total_score={}",
    // TODO: s.success, s.total_score
    // TODO: );
    i = 0;
    l = 0;
    // TODO loop (degraded): for (i, l) in s.lives.iter().enumerate() {
"""
# Replicate the pass-3 candidate logic
known = {"tg","tg2","failures","RB_ZONE_HAND","RB_ZONE_STAGE","RB_PHASE_MAIN","RB_PHASE_ACTIVE","RB_PHASE_ENERGY","RB_PHASE_DRAW","RB_PHASE_LIVE_SET","RB_PHASE_PERFORMANCE","RB_PHASE_VICTORY","RB_PHASE_OPENING","RB_PHASE_RPS","RB_HEART_PINK","RB_HEART_RED","RB_HEART_YELLOW","RB_HEART_GREEN","RB_HEART_BLUE","RB_HEART_PURPLE","RB_HEART_ORANGE","RB_HEART_ALL","RB_HEART_DRAW","RB_HEART_SCORE","RB_HEART_ANY","RB_MAX_CARD_IDS","RB_MAX_RECENTLY_MOVED","int","Card","TestGame","void","static","if","for","while","return","CHECK","CHECK_EQ","CHECK_EQ_STR","test_id","test_add_to_deck","test_add_to_hand","test_add_to_discard","test_add_to_live","test_add_to_success","test_add_to_energy","test_add_to_deck_pl","test_add_to_revealed","test_add_to_stage","test_play_to_stage","test_activate_ability","test_give_energy","test_spend_energy","test_recalc","test_clear_mods_for_card","test_set_live_card","test_has_pending_choice","test_pending_choice_count","test_pending_choice_type","test_get_blade_modifier","test_get_score_modifier","test_get_cost_modifier","test_get_heart_modifier","test_zone_has_id","test_zone_has_card_no","test_filler_hand","test_insert_deck_top","test_set_energy_active","test_place_under","test_drain_auto_choices","test_answer_play_cost_choice","rb_mods_get_cost","rb_mods_get_score","rb_mods_get_blade","rb_mods_get_heart","rb_mods_set_orientation","rb_advance_phase","rb_has_pending_choice","rb_resume_with_choice","rb_drain_ability_queue","rb_trigger_live_start","rb_queue_current_entry","rb_queue_is_empty","rb_phase_name","rb_load","rb_unload","rb_find_card_by_no","rb_decode_card_by_index","rb_card_is_live","rb_card_is_energy","rb_owner_of_card","rb_zone_of_str","rb_parse_heart_color","rb_heart_index","rb_give_energy","rb_pass","rb_perform_live","rb_record_event","rb_fire_recorded_auto","rb_queue_trigger_abilities","rb_use_count","rb_use_limit_reached","rb_pos_change_for_player","rb_misc_position_destinations","printf","fprintf","strstr","strcmp","stderr","__FILE__","__LINE__","i","l","score"}
declared = set(re.findall(r'\bint\s+(\w+)\b', body))
print('declared:', declared)
candidates = set()
for m in re.finditer(r'[\(\,\s]\s*(\w+)\s*[,\)\;]', body):
    tok = m.group(1)
    if tok in known or tok in declared or tok.isdigit() or len(tok) < 1: continue
    if tok in ("NULL","true","false"): continue
    if re.search(r'\b'+re.escape(tok)+r'\s*\(', body): pass
    if re.search(r'test_\w+\(&tg.*\b'+re.escape(tok)+r'\b', body) or re.search(r'rb_\w+.*\b'+re.escape(tok)+r'\b', body) or re.search(r'CHECK.*\b'+re.escape(tok)+r'\b', body) or re.search(r'if\s*\(.*\b'+re.escape(tok)+r'\b', body) or re.search(r'rb_resume_with_choice\([^,]+,\s*'+re.escape(tok)+r'\b', body):
        candidates.add(tok)
for m in re.finditer(r'^\s*(\w+)\s*=', body, re.MULTILINE):
    tok = m.group(1)
    if tok not in declared and tok not in known: candidates.add(tok)
print('candidates:', candidates)