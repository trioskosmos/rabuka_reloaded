import re
with open('engine/tests/gameplay_test.rs', 'r', encoding='utf-8') as f:
    content = f.read()

# Keep everything up to (but not including) the first Distortion test
idx = content.find('fn distortion_q97')
header = content[:idx]

# Remove any misplaced advance_to_live_start calls from the header
# (from my broken Python script above)
# The header should have exactly ONE advance_to_live_start function definition
# and the advance_to_live_card_set_p1 function

new_tests = r'''
// ====================================================================
//  ディストーション (PL!SP-pb1-023-L) — sequential conditional ability
// ====================================================================
// ライブ開始時:
//   自分のステージに名前の異なる『CatChu!』のメンバーが2人以上いる場合
//     → エネルギーを6枚までアクティブにする。
//   その後:
//     自分のエネルギーがすべてアクティブ状態の場合
//       → このカードのスコアを＋１する。
//
// NOTE: Phase::Active activates ALL energy for both players. Tests that need
// wait energy must set it up AFTER advance_to_live_card_set_p1 completes.
// ====================================================================

fn assert_score(game: &TestGame, expected: i32) {
    let live_card_id = game.state.player1.live_card_zone.cards[0];
    assert_eq!(game.state.get_score_modifier(live_card_id), expected);
}

fn assert_energy(game: &TestGame, active: usize, total: usize) {
    assert_eq!(game.state.player1.energy_zone.active_energy_count, active);
    assert_eq!(game.state.player1.energy_zone.cards.len(), total);
}

// ── Q97: CatChu!不足でも全エネルギーがアクティブならスコア＋１ ─────

#[test]
fn distortion_q97_all_active_no_catchu_score_plus_1() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let distortion = game.id("PL!SP-pb1-023-L");
    let filler = game.id("PL!-sd1-010-SD");
    game.state.player1.hand.cards.push(distortion);
    game.state.player1.hand.cards.push(filler);
    game.give_energy(3);
    assert_energy(&game, 3, 3);
    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(distortion);
    advance_to_live_start(&mut game);
    assert!(!game.has_pending_choice());
    assert_energy(&game, 3, 3);
    assert_score(&game, 1);
}

// ── Q96: スコア＋１は永続、あとでウェイトにしても戻らない ──────────

#[test]
fn distortion_q96_score_permanent_after_energy_used() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let distortion = game.id("PL!SP-pb1-023-L");
    let filler = game.id("PL!-sd1-010-SD");
    game.state.player1.hand.cards.push(distortion);
    game.state.player1.hand.cards.push(filler);
    game.give_energy(3);
    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(distortion);
    advance_to_live_start(&mut game);
    let live = game.state.player1.live_card_zone.cards[0];
    assert_eq!(game.state.get_score_modifier(live), 1);
    // Simulate using an energy (make it wait)
    game.state.player1.energy_zone.active_energy_count =
        game.state.player1.energy_zone.active_energy_count.saturating_sub(1);
    assert!(game.state.player1.energy_zone.active_energy_count
        < game.state.player1.energy_zone.cards.len());
    // Condition was checked at resolution time, not continuously
    assert_eq!(game.state.get_score_modifier(live), 1);
}

// ── Basic: CatChu! condition met, 4 wait refreshed → all active → +1 ─

#[test]
fn distortion_basic_energy_refresh_with_catchu() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let distortion = game.id("PL!SP-pb1-023-L");
    let filler = game.id("PL!-sd1-010-SD");
    let catchu_a = game.id("PL!SP-sd1-001-SD");
    let catchu_b = game.id("PL!SP-sd1-004-SD");
    let energy_id = game.id("LL-E-001-SD");
    game.state.player1.hand.cards.push(distortion);
    game.state.player1.hand.cards.push(filler);
    game.state.player1.stage.stage[0] = catchu_a;
    game.state.player1.stage.stage[2] = catchu_b;
    game.give_energy(3);
    advance_to_live_card_set_p1(&mut game);
    // Add wait energy AFTER phase advancement (Active phase activates all energy)
    for _ in 0..4 { game.state.player1.energy_zone.cards.push(energy_id); }
    assert_energy(&game, 3, 7);
    game.set_live_card(distortion);
    advance_to_live_start(&mut game);
    assert!(!game.has_pending_choice());
    assert_energy(&game, 7, 7);
    assert_score(&game, 1);
}

// ── No wait: CatChu! met, all already active → +1 only ──────────────

#[test]
fn distortion_no_refresh_when_no_wait_energy() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let distortion = game.id("PL!SP-pb1-023-L");
    let filler = game.id("PL!-sd1-010-SD");
    let catchu_a = game.id("PL!SP-sd1-001-SD");
    let catchu_b = game.id("PL!SP-sd1-004-SD");
    game.state.player1.hand.cards.push(distortion);
    game.state.player1.hand.cards.push(filler);
    game.state.player1.stage.stage[0] = catchu_a;
    game.state.player1.stage.stage[2] = catchu_b;
    game.give_energy(6);
    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(distortion);
    advance_to_live_start(&mut game);
    assert!(!game.has_pending_choice());
    assert_energy(&game, 6, 6);
    assert_score(&game, 1);
}

// ── Max cap: 8 wait → only 6 refreshed → 2 remain → no +1 ──────────

#[test]
fn distortion_max_cap_8_wait_refresh_6_only() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let distortion = game.id("PL!SP-pb1-023-L");
    let filler = game.id("PL!-sd1-010-SD");
    let catchu_a = game.id("PL!SP-sd1-001-SD");
    let catchu_b = game.id("PL!SP-sd1-004-SD");
    let energy_id = game.id("LL-E-001-SD");
    game.state.player1.hand.cards.push(distortion);
    game.state.player1.hand.cards.push(filler);
    game.state.player1.stage.stage[0] = catchu_a;
    game.state.player1.stage.stage[2] = catchu_b;
    advance_to_live_card_set_p1(&mut game);
    for _ in 0..8 { game.state.player1.energy_zone.cards.push(energy_id); }
    assert_energy(&game, 0, 8);
    game.set_live_card(distortion);
    advance_to_live_start(&mut game);
    assert!(!game.has_pending_choice());
    assert_energy(&game, 6, 8);
    assert_score(&game, 0);
}

// ── Exact max boundary: 6 wait → all refreshed → all active → +1 ────

#[test]
fn distortion_exact_max_boundary_6_wait_all_refreshed() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let distortion = game.id("PL!SP-pb1-023-L");
    let filler = game.id("PL!-sd1-010-SD");
    let catchu_a = game.id("PL!SP-sd1-001-SD");
    let catchu_b = game.id("PL!SP-sd1-004-SD");
    let energy_id = game.id("LL-E-001-SD");
    game.state.player1.hand.cards.push(distortion);
    game.state.player1.hand.cards.push(filler);
    game.state.player1.stage.stage[0] = catchu_a;
    game.state.player1.stage.stage[2] = catchu_b;
    advance_to_live_card_set_p1(&mut game);
    for _ in 0..6 { game.state.player1.energy_zone.cards.push(energy_id); }
    assert_energy(&game, 0, 6);
    game.set_live_card(distortion);
    advance_to_live_start(&mut game);
    assert!(!game.has_pending_choice());
    assert_energy(&game, 6, 6);
    assert_score(&game, 1);
}

// ── Same-name CatChu! → distinct condition NOT met → no refresh ─────

#[test]
fn distortion_same_name_catchu_condition_not_met() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let distortion = game.id("PL!SP-pb1-023-L");
    let filler = game.id("PL!-sd1-010-SD");
    let same_name = game.id("PL!SP-sd1-001-SD");
    let energy_id = game.id("LL-E-001-SD");
    game.state.player1.hand.cards.push(distortion);
    game.state.player1.hand.cards.push(filler);
    game.state.player1.stage.stage[0] = same_name;
    game.state.player1.stage.stage[2] = same_name;
    game.give_energy(3);
    advance_to_live_card_set_p1(&mut game);
    for _ in 0..4 { game.state.player1.energy_zone.cards.push(energy_id); }
    assert_energy(&game, 3, 7);
    game.set_live_card(distortion);
    advance_to_live_start(&mut game);
    assert!(!game.has_pending_choice());
    assert_energy(&game, 3, 7);
    assert_score(&game, 0);
}

// ── Q103: 7 wait, two Distortions → only one gets +1 ──────────────

#[test]
fn distortion_q103_two_triggers_only_one_plus_1() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let distortion = game.id("PL!SP-pb1-023-L");
    let filler = game.id("PL!-sd1-010-SD");
    let catchu_a = game.id("PL!SP-sd1-001-SD");
    let catchu_b = game.id("PL!SP-sd1-004-SD");
    let energy_id = game.id("LL-E-001-SD");
    game.state.player1.hand.cards.push(distortion);
    game.state.player1.hand.cards.push(distortion);
    game.state.player1.hand.cards.push(filler);
    game.state.player1.stage.stage[0] = catchu_a;
    game.state.player1.stage.stage[2] = catchu_b;
    advance_to_live_card_set_p1(&mut game);
    for _ in 0..7 { game.state.player1.energy_zone.cards.push(energy_id); }
    assert_energy(&game, 0, 7);
    game.set_live_card(distortion);
    game.set_live_card(distortion);
    advance_to_live_start(&mut game);
    assert!(!game.has_pending_choice());
    assert_energy(&game, 7, 7);
    let total_score: i32 = game.state.player1.live_card_zone.cards.iter()
        .map(|&cid| game.state.get_score_modifier(cid))
        .sum();
    assert_eq!(total_score, 1,
        "Total score across all live cards should be exactly +1 (Q103)");
}
'''

# Fix: the header might have a corrupted advance_to_live_start. Let's strip it
# and put the correct version in the new_tests section.
header = header.rstrip()
# Remove any existing advance_to_live_start function definition
if 'fn advance_to_live_start' in header:
    # Find and remove it
    idx2 = header.find('fn advance_to_live_start')
    # Find the next function after it
    idx3 = header.find('\nfn ', idx2 + 1)
    if idx3 == -1:
        idx3 = len(header)
    header = header[:idx2] + header[idx3:]

# Remove any existing assert_score or assert_energy from header (they'll be in new_tests)
for func in ['fn assert_score', 'fn assert_energy']:
    idx2 = header.find(func)
    if idx2 >= 0:
        idx3 = header.find('\nfn ', idx2 + 1)
        if idx3 == -1:
            idx3 = len(header)
        header = header[:idx2] + header[idx3:]

# Remove the advance_to_live_card_set_p1's comment that another version exists
# Also clean up any misplaced advance_to_live_start calls

with open('engine/tests/gameplay_test.rs', 'w', encoding='utf-8') as f:
    f.write(header)
    f.write('\n')
    f.write(new_tests)

print('Written: header=' + str(len(header)) + ' chars + tests')
