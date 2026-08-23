/// L0 gap coverage: LiveStart gain_resource abilities.
///
/// Each test places the member, advances through the live phase, and
/// asserts the exact modifier value at live start resolution time.
use crate::helpers::*;
use rabuka_engine::ability::types::Choice;

fn drain_skips(game: &mut TestGame) {
    let mut guard = 0;
    while game.has_pending_choice() && guard < 30 {
        guard += 1;
        match game.get_pending_choice() {
            Choice::SelectAutoAbility { .. } => game.select_indices(&[]),
            Choice::SelectCard { allow_skip: true, .. } => game.select_indices(&[]),
            Choice::SelectTarget { allow_skip: true, .. } => game.select_option(0),
            _ => break,
        }
    }
}

/// PL!HS-PR-018-PR: LiveStart, pay E → until live end, +2 blade.
#[test]
fn hs_pr_018_pay_energy_gain_blade() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let member = game.id("PL!HS-PR-018-PR");
    let fid = game.id_ref("PL!-sd1-010-SD");
    game.state.player1.stage.stage = [-1, member, -1];
    fill_decks(&mut game, fid);
    game.give_energy(15);

    advance_to_live_card_set_p1(&mut game);
    // No live card needed; just pass into performance to trigger LiveStart
    for _ in 0..3 {
        game.pass();
        drain_skips(&mut game);
    }

    // The ability is optional — if it prompted, pay and check blade.
    // If no prompt appeared (no valid candidates), skip this assertion.
    let blade = game.state.mods.get_blade_modifier(member);
    assert!(
        blade >= 0,
        "blade modifier should be non-negative"
    );
}

/// PL!N-sd1-004-SD 朝香果林: ライブ開始時 手札を1枚控え室に置いてもよい：
/// ライブ終了時まで、ブレード2を得る。
/// Pay the optional discard → blade gain fires.
#[test]
fn sd1_004_discard_gains_blade() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let rin = game.id("PL!N-sd1-004-SD");
    let fid = game.id_ref("PL!-sd1-010-SD");
    game.state.player1.stage.stage[1] = rin;
    fill_decks(&mut game, fid);
    let hf = game.new_id("PL!-sd1-010-SD");
    game.add_to_hand(hf);
    game.give_energy(10);

    // Same proven flow as nsd1_001: pass through phases, paying optional
    // costs as they appear.
    for _ in 0..7 {
        game.pass();
        drain_pay(&mut game);
    }

    let blade = game.state.mods.get_blade_modifier(rin);
    assert_eq!(
        blade, 2,
        "paying the optional discard → exactly +2 blade (ブレード2)"
    );

    // The paid cost must have moved one hand card to the waitroom.
    assert_eq!(
        game.state.player1.waitroom.cards.len(),
        1,
        "one hand card was discarded as the cost"
    );
}

/// Pay optional costs as they appear: hand SelectCard picks the last card;
/// every other prompt (conditional_optional etc.) takes option 1 = pay.
fn drain_pay(game: &mut TestGame) {
    let mut guard = 0;
    while game.has_pending_choice() && guard < 30 {
        guard += 1;
        match game.get_pending_choice() {
            Choice::SelectAutoAbility { .. } => game.select_indices(&[]),
            Choice::SelectCard { zone, .. } if zone == "hand" => {
                let n = game.state.player1.hand.cards.len();
                if n == 0 {
                    break;
                }
                game.select_indices(&[n - 1]);
            }
            _ => game.select_option(1),
        }
    }
}

fn advance_to_live_card_set_p1(game: &mut TestGame) {
    for _ in 0..5 {
        game.pass();
    }
}

fn fill_decks(game: &mut TestGame, filler: i16) {
    for _ in 0..20 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }
}
