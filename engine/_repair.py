import subprocess, difflib, os, sys

os.chdir(r'C:\Users\trios\OneDrive\Documents\rabuka_reloaded')

EDITS = {
 'engine/src/ability/choice.rs': [
   ('_context: &ExecutionContext,', 'context: &ExecutionContext,'),
   ('handle_selection_epilogue(gs, _context)', 'handle_selection_epilogue(gs, context)'),
   ('        _context: ExecutionContext,\n', ''),
   ('self.handle_select_target(gs, target, &selected, context)', 'self.handle_select_target(gs, target, &selected)'),
 ],
 'engine/src/ability/compound.rs': [('_repeat', 'repeat_idx')],
 'engine/src/ability/condition.rs': [('_before', 'before')],
 'engine/src/turn/actions.rs': [
   ('_choice_card_no', 'choice_card_no'),
   ('let _e1 =', 'let e1 ='), ('let _e2 =', 'let e2 ='),
   ('if _e2 > 0 || _e1 > 0 {', 'if e2 > 0 || e1 > 0 {'),
   ('let _o1 =', 'let o1 ='), ('let _o2 =', 'let o2 ='),
   ('if _o1 > 0 || _o2 > 0 {', 'if o1 > 0 || o2 > 0 {'),
 ],
 'engine/src/turn/phases.rs': [
   ('        let mut placed_ids: Vec<i16> = Vec::new();\n', ''),
   ('        drop(player);\n', ''),
   ('_card_indices: Option<Vec<usize>>,', 'card_indices: Option<Vec<usize>>,'),
   ('= _card_indices {', '= card_indices {'),
 ],
}

QA_EDITS = [
 ('''    let _stage_member_id = game_state
        .player1
        .stage
        .stage
        .iter()
        .find(|&&id| id != -1)
        .unwrap();

''', ''),
 ('    let _card_index = action_params.card_index.unwrap();\n',
  '    action_params.card_index.expect("play action must carry a card index");\n'),
 ('''    let _touched_member = game_state
        .player1
        .stage
        .stage
        .iter()
        .find(|&&id| id != -1)
        .copied();

''', ''),
 ('''    let _initial_cheer_checks_done = game_state.cheer_checks_done;

    // Perform cheer checks''', '    // Perform cheer checks'),
 ('''    let _sakura_id = card_database
        .get_card_id("PL!N-bp1-003-R+")
        .expect("Card not found (PL!N-bp1-003-R+)");''',
  '''    card_database
        .get_card_id("PL!N-bp1-003-R+")
        .expect("Card not found (PL!N-bp1-003-R+)");'''),
 ('    let _game_state = GameState::new(player1, player2, card_database.clone());',
  '    GameState::new(player1, player2, card_database.clone());'),
 ('''    let _player1 = Player::new("p1".to_string(), "Player 1".to_string(), true);
    let mut player2''', '    let mut player2'),
 ('''    let mut player1 = Player::new("p1".to_string(), "Player 1".to_string(), true);
    let _player2 = Player::new("p2".to_string(), "Player 2".to_string(), false);

    // Setup discard with live cards from different groups''',
  '''    let mut player1 = Player::new("p1".to_string(), "Player 1".to_string(), true);

    // Setup discard with live cards from different groups'''),
 ('''    let mut player1 = Player::new("p1".to_string(), "Player 1".to_string(), true);
    let _player2 = Player::new("p2".to_string(), "Player 2".to_string(), false);

    // Setup deck with cards''',
  '''    let mut player1 = Player::new("p1".to_string(), "Player 1".to_string(), true);

    // Setup deck with cards'''),
]
EDITS['engine/src/qa_test_suite.rs'] = QA_EDITS

report = []
for path, edits in EDITS.items():
    h = subprocess.run(['git','show','HEAD:'+path], capture_output=True).stdout.decode('utf-8')
    r = h
    for old,new in edits:
        n = r.count(old)
        r = r.replace(old,new)
        report.append(f'{path}: {old[:50]!r} x{n}')
    # damaged current copy
    cur = open(path,'rb').read().decode('utf-8', errors='replace')
    d = list(difflib.unified_diff(cur.splitlines(), r.splitlines(),
                                  'damaged_current', 'rebuilt', lineterm='', n=1))
    report.append(f'==== DIFF {path}: {len(d)} diff lines ====')
    report.extend(d[:400])
open(os.path.join('engine','_repair_report.txt'),'w',encoding='utf-8').write('\n'.join(report))
print('report written, total lines:', len(report))
