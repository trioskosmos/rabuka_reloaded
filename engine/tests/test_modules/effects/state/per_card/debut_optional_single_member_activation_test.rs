use crate::helpers::*;
use rabuka_engine::zones::MemberArea;

fn debut_activation(card_no: &str, selected: Option<usize>) {
    let mut game = TestGame::new(load_real_database());
    let source = game.id(card_no);
    assert_eq!(game.db.get_card(source).unwrap().card_no, card_no);
    let left = game.new_id("PL!-sd1-010-SD");
    let center = game.new_id("PL!-sd1-010-SD");
    let enemy = game.new_id("PL!-sd1-010-SD");
    let filler = game.id("PL!-sd1-010-SD");
    fill_decks(&mut game, filler);
    game.add_to_stage(MemberArea::LeftSide, left);
    game.add_to_stage(MemberArea::Center, center);
    game.state.player2.stage.stage[0] = enemy;
    for member in [left, center, enemy] {
        game.state.mods.add_orientation_modifier(member, "wait");
    }
    game.give_energy(10);
    game.state.player1.hand.cards.push(source);
    let deck = game.state.player1.main_deck.cards.clone();
    game.play_to_stage(source, MemberArea::RightSide);
    game.assert_select_card("stage", 1, true);
    game.select_indices(&selected.into_iter().collect::<Vec<_>>());
    game.drain_choices_strict(&[], &[]);
    assert_eq!(game.state.player1.stage.stage, [left, center, source]);
    assert_eq!(game.state.mods.get_orientation_modifier(left), Some("wait"));
    assert_eq!(
        game.state.mods.get_orientation_modifier(center),
        Some(if selected == Some(1) {
            "active"
        } else {
            "wait"
        })
    );
    assert_eq!(
        game.state.mods.get_orientation_modifier(enemy),
        Some("wait")
    );
    assert_eq!(game.state.player1.main_deck.cards, deck);
    assert!(game.state.player1.waitroom.cards.is_empty());
    assert!(game.state.player1.hand.cards.is_empty());
    assert_eq!(game.state.player1.energy_zone.active_count(), 6);
}

#[test]
fn debut_activates_only_selected_own_member_chika_bp3_010() {
    debut_activation("PL!S-bp3-010-N", Some(1));
}

#[test]
fn debut_declined_activation_preserves_waited_members_chika_bp3_010() {
    debut_activation("PL!S-bp3-010-N", None);
}

#[test]
fn debut_activates_only_selected_own_member_riko_bp3_011() {
    debut_activation("PL!S-bp3-011-N", Some(1));
}

#[test]
fn debut_declined_activation_preserves_waited_members_riko_bp3_011() {
    debut_activation("PL!S-bp3-011-N", None);
}
