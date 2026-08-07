/// Q274 — 松浦果南 PL!S-bp7-003-R＋ ab#1 option 1 (ウェイトしない).
///
/// 登場：以下から1つを選ぶ。
///  ・ライブ終了時まで、自分のステージにいるブレードの数が3つ以下の『Aqours』の
///    メンバーは、相手の効果によってはウェイトしない。
///
/// Official QA Q274 ("相手がメンバーをウェイトにする効果で、ウェイトしないメンバーを
/// 選ぶことは可能ですか？") Answer: はい、可能です。
///
/// Q274 rule: an OPPONENT may still *select* your wait-immune member as the target of
/// their wait effect. "Cannot be waited" ≠ "cannot be selected". The member is still
/// offered as a legal target choice; only the *application* of the wait is suppressed.
///
/// These tests drive the real engine rule end-to-end with 朝香果林 (PL!N-bp7-004-R, whose
/// 起動 waits an opponent member with blade <= energy_under+1):
///   1. The wait-immune member is STILL OFFERED in the opponent's wait-target choice
///      (its stage index is present in the SelectCard filtered_indices).
///   2. Selecting the wait-immune member leaves it ACTIVE (the wait is suppressed).
///   3. On the same board, selecting a NON-immune member still waits it (immunity is
///      scoped to the protected Aqours member, not the whole wait).
///   4. Negative control: without any recorded immunity, the same member IS waited
///      (proving the blocking is caused by the immunity, not a broken wait effect).
use crate::helpers::*;
use crate::test_modules::bp7_wait_immunity_helpers::p1_establish_wait_immunity;

const KANAN: &str = "PL!S-bp7-003-R\u{ff0b}"; // 松浦果南 — Aqours, blade 2 (immune, blade ≤ 3)
const KARIN: &str = "PL!N-bp7-004-R"; // 朝香果林 — waits opp blade ≤ (energy_under + 1)
const NON_IMMUNE: &str = "PL!-sd1-010-SD"; // 高坂穂乃果 — μ's blade 1, NOT Aqours → not immune
const ENERGY: &str = "LL-E-001-SD";

fn set_active(game: &mut TestGame, p1_active: bool) {
    game.state.player1.is_first_attacker = p1_active;
    game.state.player2.is_first_attacker = !p1_active;
}

fn is_waited(game: &TestGame, id: i16) -> bool {
    game.state.mods.get_orientation_modifier(id).as_deref() == Some("wait")
}

/// Player2 activates 朝香果林 on their stage. The wait first pays the cost (place 1
/// energy from the energy zone under her), then presents the opponent-stage wait target
/// choice. When that "stage" SelectCard appears, we capture its filtered_indices (the
/// offered stage positions) and select the member whose stage position == `select_pos`.
/// Returns the offered stage positions (the members the opponent COULD pick).
fn run_karin_wait(game: &mut TestGame, select_pos: usize) -> Vec<usize> {
    let karin = game.id(KARIN);
    game.state.player2.stage.stage[1] = karin;
    let e = game.id(ENERGY);
    game.state.player2.energy_zone.cards.push(e);

    game.activate_ability(karin);
    let mut offered: Vec<usize> = Vec::new();
    let mut guard = 0;
    while game.has_pending_choice() && guard < 25 {
        guard += 1;
        let choice = game.get_pending_choice().clone();
        match choice {
            // The wait TARGET choice: a SelectCard over the opponent's stage. This is
            // the moment Q274 hinges on: the members offered to the opponent.
            rabuka_engine::ability::types::Choice::SelectCard {
                zone, filtered_indices, ..
            } if zone == "stage" => {
                let fi = filtered_indices.unwrap_or_default();
                offered = fi.clone();
                // select_pos is a stage position → map it to its index in the offered set.
                let sel = fi.iter().position(|&p| p == select_pos).unwrap_or(0);
                game.select_indices(&[sel]);
            }
            // The activation cost: place the single energy under 朝香果林.
            rabuka_engine::ability::types::Choice::SelectCard { .. } => {
                game.select_indices(&[0]);
            }
            rabuka_engine::ability::types::Choice::SelectTarget { .. } => {
                game.select_choice_option(1);
            }
            _ => {
                game.select_indices(&[]);
            }
        }
    }
    offered
}

/// 1. Q274 exact case: the wait-immune member is STILL OFFERED to the opponent and,
/// when selected, simply is not waited. Player1's 果南 (Aqours, blade 2) is wall at
/// center (stage index 1); a non-immune μ's member sits at index 0. Both board targets
/// of 朝香果林's wait (blade ≤ 2). We select 果南 (the immune one).
#[test]
fn q274_immune_target_still_offered_and_not_waited() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    // Player1 establishes wait-immunity on their own 果南 (at center / index 1).
    let kanan = p1_establish_wait_immunity(&mut game);
    let non_immune = game.id(NON_IMMUNE);
    game.state.player1.stage.stage[0] = non_immune; // blade 1, in-window, not immune

    assert!(
        game.state.wait_immune_members.iter().any(|(m, o)| *m == kanan && o == "p1"),
        "果南 must be recorded wait-immune for p1"
    );

    // Player2 (opponent) becomes active and activates 朝香果林's wait.
    set_active(&mut game, false);
    let offered = run_karin_wait(&mut game, /*select*/ 1 /* kanan's stage index */);

    // Q274 core: the wait-immune member was OFFERED as a legal target choice.
    assert!(
        offered.contains(&1),
        "wait-immune 果南 (stage index 1) must still be OFFERED to the opponent; offered={:?}",
        offered
    );
    assert!(
        offered.contains(&0),
        "non-immune member (index 0) should also be offered; offered={:?}",
        offered
    );
    // ...and despite being selected, the wait is suppressed.
    assert!(
        !is_waited(&game, kanan),
        "selecting the wait-immune member must NOT wait it (Q274); mod={:?}",
        game.state.mods.get_orientation_modifier(kanan)
    );
}

/// 2. On the same immune board, choosing the NON-immune member still waits it — the
///    immunity is scoped to the affected member only, not the whole wait effect.
#[test]
fn q274_non_immune_pick_is_still_waited() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let kanan = p1_establish_wait_immunity(&mut game);
    let non_immune = game.id(NON_IMMUNE);
    game.state.player1.stage.stage[0] = non_immune;

    set_active(&mut game, false);
    let offered = run_karin_wait(&mut game, 0 /* non-immune μs stage index */);

    assert!(offered.contains(&0), "non-immune member should be offered; offered={:?}", offered);
    assert!(
        is_waited(&game, non_immune),
        "the non-immune member selected by the opponent must still be waited"
    );
    // The immune member was not selected here, so it stays active.
    assert!(!is_waited(&game, kanan), "unselected immune member stays active");
}

/// 3. Negative control: with NO immunity recorded, the same test → the SAME member is
///    actually waited. This proves the block in test #1 comes from the immunity, not a
///    suppressed wait that excludes the member entirely.
#[test]
fn q274_without_immunity_the_member_is_waited() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    // Mirror test #1's board exactly (果南 at center + a non-immune μ's member) but
    // WITHOUT recording any immunity — place her card on the stage directly.
    let kanan = game.id(KANAN);
    let non_immune = game.id(NON_IMMUNE);
    game.state.player1.stage.stage = [non_immune, kanan, -1];
    assert!(game.state.wait_immune_members.is_empty(), "no immunity expected");

    set_active(&mut game, false);
    let offered = run_karin_wait(&mut game, 1);

    assert!(offered.contains(&1), "果南 should be selectable; offered={:?}", offered);
    assert!(
        is_waited(&game, kanan),
        "with no immunity, selecting 果南 (blade 2 ≤ window 2) MUST wait her — the block in test #1 is due to immunity"
    );
}