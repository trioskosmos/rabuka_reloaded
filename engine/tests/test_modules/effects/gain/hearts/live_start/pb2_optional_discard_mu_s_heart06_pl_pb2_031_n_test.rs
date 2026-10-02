//! 園田海未 `PL!-pb2-031-N` (ライブ開始時)
//!
//! {{live_start.png|ライブ開始時}}手札の『μ's』のカードを1枚控え室に置いてもよい：
//! ライブ終了時まで、{{heart_06.png|heart06}}を得る。
//!
//! A flat +1 heart06 behind an OPTIONAL cost, so the two axes are separate: the cost is
//! paid or declined, and the heart appears or not. 「1枚」 is a single card, not a
//! per-card count, so discarding two must not give two hearts.

use crate::helpers::*;
use rabuka_engine::card::HeartColor;
use rabuka_engine::core::types::AbilityTrigger;
use rabuka_engine::zones::MemberArea;

const MAKI: &str = "PL!-pb2-031-N";
/// A μ's card for the optional discard. Any member will do — the cost filters on group.
const MUS_CARD: &str = "PL!-bp6-017-N"; // 小泉花陽, Printemps
/// Not μ's, so it must not be payable.
const NON_MUS: &str = "PL!S-bp5-012-N"; // 松浦果南, AZALEA

fn fire_live_start(game: &mut TestGame, cid: i16) {
    let ability_id = {
        let card = game.db.get_card(cid).unwrap();
        let ab = card
            .resolved_abilities()
            .find(|a| a.triggers.as_deref() == Some("ライブ開始時"))
            .unwrap_or_else(|| panic!("{} lacks a ライブ開始時 ability", card.card_no));
        format!("{}_{}", card.card_no, ab.full_text)
    };
    let card_no = game.db.get_card(cid).unwrap().card_no.to_string();
    let pid = game.state.player1.id.clone();
    game.state.activating_card = Some(cid);
    game.state.trigger_auto_ability(
        ability_id,
        AbilityTrigger::LiveStart,
        pid.clone(),
        Some(card_no),
        Some(cid),
        None,
        None,
    );
    game.state.process_pending_auto_abilities(&pid);
}

/// Stand MAKI with `hand` seeded; pay the optional cost; return (game, maki).
fn paid_with(hand: &[&str], pay: bool) -> (TestGame, i16) {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.new_id("PL!-sd1-010-SD");
    fill_decks(&mut game, filler);
    let maki = game.id(MAKI);
    game.assert_card_identity(maki, MAKI);
    for c in hand {
        game.state.player1.hand.cards.push(game.id(c));
    }
    game.state.player1.hand.cards.push(maki);
    game.give_energy(20);
    game.play_to_stage(maki, MemberArea::Center);

    fire_live_start(&mut game, maki);
    // The optional cost is a SelectCard on the hand; an empty selection declines it.
    // The prompt must be offered: the old `if has_pending_choice()` answered
    // nothing when it was absent, and both halves of this test (paid vs declined)
    // would then agree with an engine that simply never charged.
    assert_eq!(
        game.pending_choice_type().as_deref(),
        Some("SelectCard"),
        "『μ's』のカードを1枚控え室に置いてもよい — the optional discard must be offered"
    );
    if pay {
        game.select_indices(&[0]);
    } else {
        game.select_indices(&[]);
    }
    (game, maki)
}

#[test]
fn paying_the_discard_gives_one_heart06() {
    let (game, maki) = paid_with(&[MUS_CARD], true);
    assert_eq!(
        game.state.mods.get_heart_modifier(maki, HeartColor::Heart06),
        1,
        "手札の『μ's』のカードを1枚控え室に置いてもよい — paying the optional cost \
         grants +1 heart06"
    );
}

#[test]
fn declining_the_discard_gives_nothing() {
    let (game, maki) = paid_with(&[MUS_CARD], false);
    assert_eq!(
        game.state.mods.get_heart_modifier(maki, HeartColor::Heart06),
        0,
        "置いてもよい — the cost is optional, so declining grants no heart"
    );
}

/// 「1枚」 is a single card, not a per-card count.
#[test]
fn two_discarded_cards_still_give_one_heart() {
    let (game, maki) = paid_with(&[MUS_CARD, MUS_CARD], true);
    assert_eq!(
        game.state.mods.get_heart_modifier(maki, HeartColor::Heart06),
        1,
        "1枚 — one card is discarded for a flat +1, so a bigger hand is not worth more"
    );
}

#[test]
fn no_heart_appears_off_the_trigger() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.new_id("PL!-sd1-010-SD");
    fill_decks(&mut game, filler);
    let maki = game.id(MAKI);
    game.state.player1.hand.cards.push(maki);
    game.give_energy(20);
    game.play_to_stage(maki, MemberArea::Center);
    game.state.recalculate_constants();

    assert_eq!(
        game.state.mods.get_heart_modifier(maki, HeartColor::Heart06),
        0,
        "ライブ終了時まで — the heart is scoped to the live, so it is not a permanent \
         常時 that recalculate_constants would keep producing"
    );
}

/// The cost filters on 「『μ's』のカード」, so a hand with no μ's card cannot pay it.
#[test]
fn a_hand_with_no_mus_card_cannot_pay() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.new_id("PL!-sd1-010-SD");
    fill_decks(&mut game, filler);
    let maki = game.id(MAKI);
    let non_mus = game.id(NON_MUS);
    game.state.player1.hand.cards.push(non_mus);
    game.state.player1.hand.cards.push(maki);
    game.give_energy(20);
    game.play_to_stage(maki, MemberArea::Center);

    fire_live_start(&mut game, maki);
    // Negative: with no eligible card the cost has nothing it can pay with, so it
    // must not be offered at all. An empty `allowed` list turns any prompt into a
    // failure, which is the point — the old guard declined one if it appeared,
    // letting a wrongly-offered cost reach the heart assertion and pass.
    game.drain_choices_strict(&[], &[]);
    game.state.recalculate_constants();

    assert_eq!(
        game.state.mods.get_heart_modifier(maki, HeartColor::Heart06),
        0,
        "『μ's』のカード — the only hand card is an AZALEA member, so the optional \
         cost has nothing it can pay with and no heart is gained"
    );
}
