/// Q275 — 松浦果南 PL!S-bp7-003-R＋ ab#1 option 1 (ウェイトしない) vs
/// セラス 柳田 リリエンフェルト PL!HS-bp6-007-R.
///
/// 果南 ab#1 option 1: ライブ終了時まで、自分のステージのブレード<=3 の 『Aqours』
/// メンバーは、相手の効果ではウェイトしない。
///
/// セラス (PL!HS-bp6-007-R) 自動・ターン1: 「自分のステージに『EdelNote』のメンバーが
/// 登場したとき、相手は、自分のステージにいる、アクティブ状態のメンバー1人を
/// ウェイトにする。」 (parsed as action=change_state, target="opponent",
/// action_by="opponent") — the OPPONENT of セラス must choose one of their OWN active
/// members to wait.
///
/// Official QA Q275 ("自分のステージにウェイトしないメンバーとそうでないメンバーが
/// いるとき、ウェイトしないメンバーを選ぶことは可能ですか？")
/// Answer: いいえ、必ずウェイトできるメンバーの中から選ぶ必要があります。
///
/// Q275 rule: when an effect forces YOU to wait a member, a wait-immune member is NOT a
/// legal choice — you must pick a waitable member. This is the inverse of Q274 (an
/// opponent freely picking a victim may still select the immune one).
///
/// engine fix: `execute_change_state` pre-filters candidates so the immune member is NOT
/// offered when the wait is a self-sacrifice selection (`effect.action_by == "opponent"`).
///
/// Note: when only ONE legal candidate remains, the effect auto-resolves (count==1) with
/// no prompt. To exercise the offered-choice filtering, each test keeps TWO+ legal
/// (non-immune) candidates alongside the immune one so a stage SelectCard prompt appears.
use crate::helpers::*;
use crate::test_modules::bp7_wait_immunity_helpers::p1_establish_wait_immunity;
use rabuka_engine::zones::MemberArea;

const CERAS: &str = "PL!HS-bp6-007-R"; // セラス 柳田 リリエンフェルト — EdelNote, cost 15
const NON_IMMUNE: &str = "PL!-sd1-010-SD"; // 高坂穂乃果 — μ's blade 1, NOT Aqours

fn set_active(game: &mut TestGame, p1_active: bool) {
    game.state.player1.is_first_attacker = p1_active;
    game.state.player2.is_first_attacker = !p1_active;
}

fn is_waited(game: &TestGame, id: i16) -> bool {
    game.state.mods.get_orientation_modifier(id).as_deref() == Some("wait")
}

/// Player2 plays セラス, whose 自動 fires when the EdelNote セラス appears on p2's own
/// stage, forcing p1 to wait one of p1's own active members. Drives that wait-target
/// choice and returns the OFFERED p1 stage indices (the members p1 may legally pick).
/// When the offered prompt is a stage SelectCard, its filtered_indices are recorded and
/// the member at `select` is chosen. Returns (offered_indices, selected_stage_pos).
fn run_ceras_forced_wait(game: &mut TestGame, select: usize) -> (Vec<usize>, usize) {
    set_active(game, false);
    let ceras = game.id(CERAS);
    game.state.player2.hand.cards.push(ceras);
    for _ in 0..20 {
        let e = game.id("LL-E-001-SD");
        game.state.player2.energy_zone.cards.push(e);
    }
    game.state.player2.energy_zone.add_active(20);
    game.play_to_stage(ceras, MemberArea::Center);

    let mut offered: Vec<usize> = Vec::new();
    let mut selected: usize = usize::MAX;
    let mut guard = 0;
    while game.has_pending_choice() && guard < 25 {
        guard += 1;
        let choice = game.get_pending_choice().clone();
        match choice {
            // The forced self-wait target choice: a SelectCard over p1's stage.
            rabuka_engine::ability::types::Choice::SelectCard {
                zone, filtered_indices, ..
            } if zone == "stage" => {
                let fi = filtered_indices.unwrap_or_default();
                if offered.is_empty() {
                    offered = fi.clone();
                }
                if fi.is_empty() {
                    game.select_indices(&[]);
                } else {
                    game.select_indices(&[select]);
                    selected = select;
                }
            }
            _ => {
                game.select_choice_option(0);
            }
        }
    }
    (offered, selected)
}

/// 1. Q275 exact case: 果南 is wait-immune (index 1) with two non-immune members on p1's
///    stage. セラス's forced wait does NOT offer the immune 果南 — only the waitable
///    members are offered; the picked non-immune is waited while 果南 stays active.
#[test]
fn q275_forced_wait_excludes_immune_and_waits_non_immune() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    // p1: 果南 (Aqours, blade 2) at center -> wait-immune for p1; two μ's (non-immune).
    let kanan = p1_establish_wait_immunity(&mut game); // stage index 1
    let a = game.id(NON_IMMUNE);
    let b = game.id(NON_IMMUNE);
    game.state.player1.stage.stage[0] = a;
    game.state.player1.stage.stage[2] = b;
    assert!(
        game.state.wait_immune_members.iter().any(|(m, o)| *m == kanan && o == "p1"),
        "果南 must be wait-immune for p1"
    );

    let (offered, _) = run_ceras_forced_wait(&mut game, 0);

    assert!(
        !offered.contains(&1),
        "wait-immune 果南 (stage index 1) must NOT be a legal forced-wait choice (Q275); offered={:?}",
        offered
    );
    assert!(
        offered.contains(&0) && offered.contains(&2),
        "the non-immune members (indices 0,2) must be offered; offered={:?}",
        offered
    );
    assert!(
        is_waited(&game, a),
        "the offered non-immune member must be waited by セラス's forced wait"
    );
    assert!(
        !is_waited(&game, kanan),
        "the immune 果南 must stay active (never a legal target)"
    );
}

/// 2. Negative control: with NO immunity recorded, the same forced wait OFFERS 果南 and
///    she IS waited — proving test #1's exclusion is caused by the wait-immunity, not a
///    broken セラス effect.
#[test]
fn q275_without_immunity_immune_member_is_offered_and_waited() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let kanan = game.id("PL!S-bp7-003-R\u{ff0b}");
    let a = game.id(NON_IMMUNE);
    let b = game.id(NON_IMMUNE);
    // Direct placement, no immunity recorded.
    game.state.player1.stage.stage[0] = a;
    game.state.player1.stage.stage[1] = kanan;
    game.state.player1.stage.stage[2] = b;
    assert!(game.state.wait_immune_members.is_empty(), "no immunity expected");

    let (offered, _) = run_ceras_forced_wait(&mut game, 1);

    assert!(
        offered.contains(&1),
        "without immunity 果南 (index 1) must be offered; offered={:?}",
        offered
    );
    assert!(
        is_waited(&game, kanan),
        "without immunity, the forced wait must wait 果南 — earlier exclusion is immunity-driven"
    );
}

/// 4. Single legal target auto-resolves (count==1, no prompt): immune 果南 + exactly one
///    non-immune → the non-immune is waited without a stage prompt, 果南 stays active.
#[test]
fn q275_single_legal_target_auto_resolves_no_prompt() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let kanan = p1_establish_wait_immunity(&mut game); // immune at index 1
    let a = game.id(NON_IMMUNE);
    game.state.player1.stage.stage[0] = a; // the single legal (non-immune) target

    let (offered, _) = run_ceras_forced_wait(&mut game, 0);
    assert!(
        offered.is_empty(),
        "with a single legal candidate the forced wait must auto-resolve with no prompt; offered={:?}",
        offered
    );
    assert!(is_waited(&game, a), "the single non-immune member is waited");
    assert!(!is_waited(&game, kanan), "immune 果南 stays active");
}

/// 3. Mixed control: with immunity, only the immune member excluded; with two non-immune
///    members both are offered. (Confirms the filter is scoped to the wait-immune entry,
///    not to "only one member may be offered".)
#[test]
fn q275_immunity_excludes_only_the_immune_member() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let kanan = p1_establish_wait_immunity(&mut game); // immune at index 1
    let a = game.id(NON_IMMUNE);
    let b = game.id(NON_IMMUNE);
    game.state.player1.stage.stage[0] = a;
    game.state.player1.stage.stage[2] = b;

    let (offered, _) = run_ceras_forced_wait(&mut game, 0);
    assert_eq!(
        offered,
        vec![0, 2],
        "only the non-immune members may be chosen; offered={:?}",
        offered
    );
    assert!(!is_waited(&game, kanan), "immune 果南 stays active");
    assert!(is_waited(&game, a), "the picked non-immune member is waited");
}
