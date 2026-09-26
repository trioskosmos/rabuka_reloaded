use crate::helpers::*;
use rabuka_engine::ability::types::Choice;

fn pay_optional_costs_selecting_last_hand_card(game: &mut TestGame) {
    let mut guard = 0;
    while game.has_pending_choice() && guard < 30 {
        guard += 1;
        match game.get_pending_choice() {
            Choice::SelectAutoAbility { .. } => game.select_indices(&[]),
            Choice::SelectCard { zone, .. } if zone == "hand" => {
                let n = game.state.player1.hand.cards.len();
                if n > 0 { game.select_indices(&[n - 1]); } else { break; }
            }
            _ => game.select_option(1),
        }
    }
}

fn fill_both_main_decks(game: &mut TestGame, filler: i16) {
    for _ in 0..20 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }
}

#[test]
fn live_start_pay_energy_grants_blade_to_other_group_member() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let self_member = game.id("PL!N-sd1-001-SD");
    let other_niji = game.new_id("PL!N-bp4-007-R\u{ff0b}");
    game.assert_card_identity(self_member, "PL!N-sd1-001-SD");
    game.assert_card_identity(other_niji, "PL!N-bp4-007-R\u{ff0b}");
    game.state.player1.stage.stage = [self_member, other_niji, -1];
    let fid2 = game.id_ref("PL!-sd1-010-SD");
    fill_both_main_decks(&mut game, fid2);
    game.give_energy(15);

    // The LiveStart fires and should target the OTHER 虹ヶ咲 member.
    for _ in 0..7 {
        game.pass();
        pay_optional_costs_selecting_last_hand_card(&mut game);
    }

    // 自分のステージにいるほかの『虹ヶ咲』のメンバーはブレードを得る — exactly one
    // blade, on the OTHER member only.
    assert_eq!(
        game.state.mods.get_blade_modifier(other_niji),
        1,
        "ほかの『虹ヶ咲』のメンバー gains exactly one blade"
    );
    assert_eq!(
        game.state.mods.get_blade_modifier(self_member),
        0,
        "ほかの — the source herself must NOT be included"
    );
}
