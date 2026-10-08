use crate::helpers::*;

fn fill_decks(game: &mut TestGame) {
    let filler = game.id("PL!-sd1-010-SD");
    for _ in 0..30 {
        game.state.player1.main_deck.cards.push(filler);
    }
    for _ in 0..30 {
        game.state.player2.main_deck.cards.push(filler);
    }
}

fn advance_to_live_card_set_p1(game: &mut TestGame) {
    for _ in 0..5 {
        game.pass();
    }
}

fn advance_to_live_start(game: &mut TestGame) {
    game.pass();
    game.pass();
}

/// Stage `members` left→right for P1, hand P1 `6` energy with `active` of them
/// active, then walk to live start. Returns the game so the caller can assert
/// the resulting active count.
///
/// `6` energy leaves at least 3 genuinely waited cards, so every ruling count
/// (0, 1, 2) below is reachable without the waited pool clamping the result.
fn run_with_stage(members: &[&str], active: u8) -> TestGame {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    fill_decks(&mut game);
    for (slot, member) in members.iter().enumerate() {
        game.state.player1.stage.stage[slot] = game.id(member);
    }
    game.give_energy(6);
    game.state.player1.energy_zone.set_active_count(active);
    assert_eq!(game.state.player1.energy_zone.active_count(), active);

    advance_to_live_card_set_p1(&mut game);
    advance_to_live_start(&mut game);
    // Two 米女メイ copies raise their ライブ開始時 in the same window, and the
    // engine asks which resolves first (「複数の自動能力が同時に発動しました」).
    // Answer in queue order; anything other than that prompt is a hard failure.
    game.drain_choices_strict(&["SelectAutoAbility"], &[]);
    game
}

fn assert_active(game: &mut TestGame, expected: u8, why: &str) {
    // Anything still queued here is an engine bug — the ability must have
    // fully resolved. An empty allow-list makes every prompt a hard failure
    // that prints the whole choice, and the call no-ops on a clean queue, so
    // it needs no `if has_pending_choice()` guard: such a guard can only skip
    // the assertion below, which is exactly what the soft-guard ratchet bans.
    game.drain_choices_strict(&[], &[]);
    assert_eq!(
        game.state.player1.energy_zone.active_count(),
        expected,
        "{}",
        why
    );
}

// ====================================================================
// PL!SP-pb2-018-R / PL!SP-pb2-018-P＋ 米女メイ
//   {{ライブ開始時}}自分のステージにいるこのメンバーと名前の異なる
//   『CatChu!』のメンバー1人につき、エネルギーを1枚アクティブにする。
//
// Official rulings Q281 / Q282 pin the counting rule down to exactly one
// reading:
//   * the candidate set is every CatChu! member on the stage whose printed
//     name differs from THIS card's name — the source card never counts;
//   * copies are NOT deduplicated: two prints of 澁谷かのん are two members
//     (Q281②③), so the `distinct: card_name` that 「名前の異なる」 normally
//     produces is wrong for this clause;
//   * each copy of this ability fires on its own, so two 米女メイ on stage
//     each activate their own count (Q282③).
// ====================================================================

const MEI: &str = "PL!SP-pb2-018-R";
const MEI_PPLUS: &str = "PL!SP-pb2-018-P＋";
const MEI_OTHER_PRINT: &str = "PL!SP-pb2-007-R";
const KANON: &str = "PL!SP-pb2-001-R";
const KANON_OTHER_PRINT: &str = "PL!SP-pb2-012-R";
const SUMIRE: &str = "PL!SP-pb2-004-R";
const KANON_SD: &str = "PL!SP-sd1-001-SD";
const SUMIRE_SD: &str = "PL!SP-sd1-004-SD";
const FILLER: &str = "PL!-sd1-010-SD";

/// Q281①: 米女メイ + 澁谷かのん + 平安名すみれ → 2 energy.
/// The pre-ruling engine counted 3 (it never dropped 米女メイ herself).
#[test]
fn q281_two_named_catchu_members_activate_two() {
    let mut game = run_with_stage(&[MEI, KANON, SUMIRE], 3);
    assert_active(&mut game, 5, "Q281①: 2 CatChu members ≠ 米女メイ → 3+2");
}

/// Q281②: two copies of the SAME 澁谷かのん print → 2 energy, not 1.
#[test]
fn q281_two_copies_of_one_print_activate_two() {
    let mut game = run_with_stage(&[MEI, KANON, KANON], 3);
    assert_active(&mut game, 5, "Q281②: copies are not deduplicated → 3+2");
}

/// Q281③: 澁谷かのん (001-R) + 澁谷かのん (012-R) — same name, different
/// prints — → 2 energy, not 1.
#[test]
fn q281_same_name_different_prints_activate_two() {
    let mut game = run_with_stage(&[MEI, KANON, KANON_OTHER_PRINT], 3);
    assert_active(&mut game, 5, "Q281③: same name, different prints → 3+2");
}

/// Q282①: 米女メイ + 澁谷かのん only → 1 energy (米女メイ never counts).
#[test]
fn q282_one_other_catchu_member_activates_one() {
    let mut game = run_with_stage(&[MEI, KANON, FILLER], 3);
    assert_active(&mut game, 4, "Q282①: only 澁谷かのん counts → 3+1");
}

/// Q282②: 米女メイ (018-R) + 澁谷かのん + 米女メイ (007-R) → 1 energy.
/// The second 米女メイ shares the source's name, so she is not counted.
#[test]
fn q282_other_mei_print_is_excluded_by_name() {
    let mut game = run_with_stage(&[MEI, KANON, MEI_OTHER_PRINT], 3);
    assert_active(&mut game, 4, "Q282②: 米女メイ (007-R) shares the name → 3+1");
}

/// Q282③: 米女メイ (018-R) + 澁谷かのん + 米女メイ (018-R copy) → 2 energy.
/// Both copies of this ability fire, each activating its own 1.
#[test]
fn q282_each_mei_copy_triggers_separately() {
    let mut game = run_with_stage(&[MEI, KANON, MEI], 3);
    assert_active(
        &mut game,
        5,
        "Q282③: both 米女メイ copies trigger, 1 each → 3+2",
    );
}

/// The source card alone (two empty stage slots) → 0 energy: a member cannot
/// satisfy 「このメンバーと名前の異なる」 against herself.
#[test]
fn mei_alone_activates_nothing() {
    let mut game = run_with_stage(&[MEI], 3);
    assert_active(&mut game, 3, "米女メイ alone → no CatChu member qualifies → 3");
}

/// 米女メイ + only non-CatChu! members → 0 energy.
#[test]
fn mei_with_only_non_catchu_members_activates_nothing() {
    let mut game = run_with_stage(&[MEI, FILLER, FILLER], 3);
    assert_active(&mut game, 3, "only Printemps filler → nothing counts → 3");
}

/// Three 米女メイ copies, no other CatChu! member → 0 energy from each of the
/// three triggers: every candidate shares the source's name.
#[test]
fn only_mei_copies_activate_nothing() {
    let mut game = run_with_stage(&[MEI, MEI, MEI], 3);
    assert_active(&mut game, 3, "all candidates are 米女メイ → 3");
}

/// The P＋ print carries the identical clause and must count identically.
#[test]
fn pplus_print_counts_like_the_r_print() {
    let mut game = run_with_stage(&[MEI_PPLUS, KANON, FILLER], 3);
    assert_active(&mut game, 4, "PL!SP-pb2-018-P＋ → 3+1");
}

/// 澁谷かのん/平安名すみれ SD prints are CatChu! too and count the same as
/// their pb2 prints.
#[test]
fn sd1_catchu_prints_count_like_pb2_prints() {
    let mut game = run_with_stage(&[MEI, KANON_SD, SUMIRE_SD], 3);
    assert_active(&mut game, 5, "SD prints are CatChu! members → 3+2");
}

/// 米女メイ not on stage → no live-start trigger at all.
#[test]
fn no_mei_on_stage_nothing_activates() {
    let mut game = run_with_stage(&[FILLER, FILLER, FILLER], 3);
    assert_active(&mut game, 3, "no source card → no trigger → 3");
}
