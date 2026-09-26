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
            _ => game.select_option(1), // pay optional costs by default
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
fn live_start_optional_discard_gains_exactly_one_self_blade() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    // 宮下 愛: ライブ開始時 手札を1枚控え室に置いてもよい：ブレードを得る (×1)
    let member = game.id("PL!N-bp1-005-R");
    game.assert_card_identity(member, "PL!N-bp1-005-R");
    let fid = game.id_ref("PL!-sd1-010-SD");
    game.state.player1.stage.stage = [-1, member, -1];
    fill_both_main_decks(&mut game, fid);
    // The optional cost needs a hand card to discard — without one the
    // ability correctly self-declines.
    let hf = game.new_id("PL!-sd1-010-SD");
    game.add_to_hand(hf);
    game.give_energy(15);
    let waitroom_before = game.state.player1.waitroom.cards.len();

    // No live card is revealed in this fixture, so the ライブ開始時 window is
    // reached by walking the turn rather than by name — the number of passes is
    // what this setup needs, and the drain after each pass is what lets the
    // optional cost be paid. Converting this to `advance_to_phase` would need a
    // live in the zone, which changes the fixture.
    for _ in 0..7 {
        game.pass();
        pay_optional_costs_selecting_last_hand_card(&mut game);
    }

    // ブレード1枚 — exactly one, and the printed cost must have been paid.
    assert_eq!(
        game.state.mods.get_blade_modifier(member),
        1,
        "optional discard paid → exactly +1 blade"
    );
    assert_eq!(
        game.state.player1.waitroom.cards.len(),
        waitroom_before + 1,
        "手札を1枚控え室に置く — the cost must actually have been paid"
    );
}

#[test]
fn live_start_optional_discard_moves_one_hand_card_and_gains_two_self_blades() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let rin = game.id("PL!N-sd1-004-SD");
    let fid = game.id_ref("PL!-sd1-010-SD");
    game.state.player1.stage.stage[1] = rin;
    fill_both_main_decks(&mut game, fid);
    let hf = game.new_id("PL!-sd1-010-SD");
    game.add_to_hand(hf);
    game.give_energy(10);

    // Same proven flow as nsd1_001: pass through phases, paying optional
    // costs as they appear.
    for _ in 0..7 {
        game.pass();
        pay_optional_discard_stopping_on_empty_hand(&mut game);
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

fn pay_optional_discard_stopping_on_empty_hand(game: &mut TestGame) {
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
