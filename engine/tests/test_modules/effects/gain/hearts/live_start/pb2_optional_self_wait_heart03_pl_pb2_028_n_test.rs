//! 高坂穂乃果 `PL!-pb2-028-N` (ライブ開始時)
//!
//! {{live_start.png|ライブ開始時}}このメンバーをウェイトにしてもよい：ライブ終了時まで、
//! {{heart_03.png|heart03}}を得る。
//! （ウェイト状態のメンバーが持つ{{icon_blade.png|ブレード}}は、エールで公開する枚数を
//! 増やさない。）
//!
//! The cost is 「このメンバーをウェイトにしてもよい」 — optional, and the PAYING side is
//! an orientation change, so paying and declining differ in observable state as well as
//! in the heart. The trailing parenthetical is a rules clarification about the yell,
//! not an effect, and is not asserted (docs §8.3).

use crate::helpers::*;
use rabuka_engine::card::HeartColor;
use rabuka_engine::core::types::AbilityTrigger;
use rabuka_engine::zones::MemberArea;

const HONOKA: &str = "PL!-pb2-028-N";

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

/// Play HONOKA, fire, and answer the optional self-wait cost with `pay`.
fn after_cost(pay: bool) -> (TestGame, i16) {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.new_id("PL!-sd1-010-SD");
    fill_decks(&mut game, filler);
    let honoka = game.id(HONOKA);
    game.assert_card_identity(honoka, HONOKA);
    game.state.player1.hand.cards.push(honoka);
    game.give_energy(20);
    game.play_to_stage(honoka, MemberArea::Center);

    fire_live_start(&mut game, honoka);
    if game.has_pending_choice() {
        if pay {
            game.select_option(1);
        } else {
            game.select_option(0);
        }
    }
    (game, honoka)
}

#[test]
fn paying_the_wait_gives_one_heart03() {
    let (game, honoka) = after_cost(true);
    assert_eq!(
        game.state.mods.get_heart_modifier(honoka, HeartColor::Heart03),
        1,
        "このメンバーをウェイトにしてもよい — paying grants +1 heart03"
    );
}

#[test]
fn paying_the_wait_actually_waits_her() {
    let (game, honoka) = after_cost(true);
    assert_eq!(
        game.state.mods.get_orientation_modifier(honoka),
        Some("wait"),
        "このメンバーをウェイトにし is part of the cost, so it must actually be paid"
    );
}

#[test]
fn declining_gives_no_heart_and_leaves_her_active() {
    let (game, honoka) = after_cost(false);
    assert_eq!(
        game.state.mods.get_heart_modifier(honoka, HeartColor::Heart03),
        0,
        "置いてもよい — declining the optional cost grants no heart"
    );
    assert_eq!(
        game.state.mods.get_orientation_modifier(honoka),
        None,
        "declining must not wait her either; the two halves of the cost move together"
    );
}

#[test]
fn no_heart_is_produced_off_the_trigger() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.new_id("PL!-sd1-010-SD");
    fill_decks(&mut game, filler);
    let honoka = game.id(HONOKA);
    game.state.player1.hand.cards.push(honoka);
    game.give_energy(20);
    game.play_to_stage(honoka, MemberArea::Center);
    game.state.recalculate_constants();

    assert_eq!(
        game.state.mods.get_heart_modifier(honoka, HeartColor::Heart03),
        0,
        "ライブ終了時まで — scoped to the live, not a permanent 常時"
    );
}

/// The heart is one, not per live set or per turn — it must not accumulate.
#[test]
fn the_heart_does_not_accumulate() {
    let (game, honoka) = after_cost(true);
    assert_eq!(
        game.state.mods.get_heart_modifier(honoka, HeartColor::Heart03),
        1,
        "{{heart_03}}を得る with no multiplier — exactly one, never more"
    );
}
