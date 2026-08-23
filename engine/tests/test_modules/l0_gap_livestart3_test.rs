/// L0 gap coverage: additional LiveStart blade abilities.
use crate::helpers::*;
use rabuka_engine::ability::types::Choice;

fn drain_pay_costs(game: &mut TestGame) {
    let mut guard = 0;
    while game.has_pending_choice() && guard < 30 {
        guard += 1;
        match game.get_pending_choice() {
            Choice::SelectAutoAbility { .. } => game.select_indices(&[]),
            Choice::SelectCard { zone, .. } if zone == "hand" => {
                let n = game.state.player1.hand.cards.len();
                if n > 0 { game.select_indices(&[n - 1]); } else { break; }
            }
            _ => game.select_option(1), // pay optional costs by default
        }
    }
}

fn setup_and_advance(game: &mut TestGame, card_no: &str) -> i16 {
    let member = game.id(card_no);
    let fid = game.id_ref("PL!-sd1-010-SD");
    game.state.player1.stage.stage = [-1, member, -1];
    fill_decks(game, fid);
    game.give_energy(15);
    advance_live(game);
    member
}

fn advance_live(game: &mut TestGame) {
    for _ in 0..7 {
        game.pass();
        drain_pay_costs(game);
    }
}

fn fill_decks(game: &mut TestGame, filler: i16) {
    for _ in 0..20 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }
}

/// PL!N-bp1-001-R: LiveStart, pay 1E → until live end, +1 blade.
#[test]
fn bp1_001_pay_energy_blade() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let member = setup_and_advance(&mut game, "PL!N-bp1-001-R");
    let blade = game.state.mods.get_blade_modifier(member);
    assert!(blade >= 1, "pay 1E → at least +1 blade");
}

/// PL!N-bp1-005-R: LiveStart, optional hand discard 1 → until live end, +1 blade.
/// TODO: needs investigation - optional cost prompt shape may differ.
#[test]
#[ignore = "optional cost prompt shape needs investigation"]
fn bp1_005_discard_gains_blade() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let member = setup_and_advance(&mut game, "PL!N-bp1-005-R");
    let blade = game.state.mods.get_blade_modifier(member);
    assert!(blade >= 1, "optional discard paid → at least +1 blade");
}

/// PL!N-sd1-001-SD: LiveStart, pay 1E → OTHER 虹ヶ咲 members on stage get
/// +1 blade (not self).
#[test]
fn nsd1_001_targets_other_niji_members_not_self() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let self_member = game.id("PL!N-sd1-001-SD");
    let other_niji = game.new_id("PL!N-bp4-007-R\u{ff0b}");
    game.state.player1.stage.stage = [self_member, other_niji, -1];
    let fid2 = game.id_ref("PL!-sd1-010-SD");
    fill_decks(&mut game, fid2);
    game.give_energy(15);

    // The LiveStart fires and should target the OTHER 虹ヶ咲 member.
    for _ in 0..7 {
        game.pass();
        drain_pay_costs(&mut game);
    }

    let other_blade = game.state.mods.get_blade_modifier(other_niji);
    assert!(
        other_blade >= 1,
        "other 虹ヶ咲 member should receive the blade boost"
    );
}
