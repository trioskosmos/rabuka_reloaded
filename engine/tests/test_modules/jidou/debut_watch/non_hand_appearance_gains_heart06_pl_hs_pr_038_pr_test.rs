//! HS-PR-038 ab#0 「手札以外から登場したとき」
//!
//! 登場したとき、手札以外から登場したとき、ライブ終了までheart06を得る。
//!
//! The condition carries `appearance_source: "non_hand"`. A member played out
//! of the HAND also 「登場」s (the two main-phase placements at
//! turn/phases.rs:1115/1352 record their origin as `"hand"`), so it must NOT
//! gain the heart — only an arrival recorded from some other zone counts.
//! Mirror of the `appearance_source: "discard"` recipe in
//! `discard_appearance_draw_one_turn_once_pl_n_pr_033_pr_test.rs`.

use crate::helpers::*;
use rabuka_engine::ability::types::Choice;
use rabuka_engine::card::HeartColor;
use rabuka_engine::zones::MemberArea;

const SUBJECT: &str = "PL!HS-PR-038-PR";
/// 起動 (4E): 控え室からコスト4以下の『蓮ノ空』メンバー1人をステージの空きエリアに出す。
const IZUMI: &str = "PL!HS-bp6-016-R";
const FILLER: &str = "PL!-sd1-010-SD";

/// Drain the deploy chain: the waitroom candidate, the target area, and any
/// 自動 ordering prompt. Loop on the TYPE, not on pendency — an unknown prompt
/// ends the loop with its name instead of a blind `&[0]` absorbing it.
fn drain_deploy(v: &mut TestGame, subject: i16) {
    let mut guard = 0;
    loop {
        guard += 1;
        assert!(guard < 20, "deploy chain did not drain in 20 prompts");
        if !v.has_pending_choice() {
            break;
        }
        match v.get_pending_choice() {
            Choice::SelectCard { zone, .. } if zone == "discard" || zone == "waitroom" => {
                let idx = v
                    .state
                    .player1
                    .waitroom
                    .cards
                    .iter()
                    .position(|&c| c == subject)
                    .expect("the subject is the deploy candidate in the waitroom");
                v.select_indices(&[idx]);
            }
            Choice::SelectPosition { .. } => v.select_indices(&[0]),
            Choice::SelectAutoAbility { .. } => v.select_indices(&[]),
            other => panic!("unexpected prompt in the deploy chain: {:?}", other),
        }
    }
}

fn setup() -> TestGame {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    game.state.player1.hand.cards.clear();
    game.state.player1.waitroom.cards.clear();
    game.state.player1.main_deck.cards.clear();
    for _ in 0..40 {
        let f = game.id(FILLER);
        game.state.player1.main_deck.cards.push(f);
    }
    game.give_energy(6);
    game
}

/// 手札以外 — an arrival from the waitroom is outside the hand and gains.
#[test]
fn appearing_from_the_waitroom_gains_heart06() {
    let mut game = setup();
    let izumi = game.id(IZUMI);
    let subject = game.id(SUBJECT);
    game.state.player1.stage.stage = [izumi, -1, -1];
    game.state.player1.waitroom.cards.push(subject);

    game.activate_ability(izumi);
    drain_deploy(&mut game, subject);

    assert!(
        game.state.player1.stage.stage.contains(&subject),
        "precondition: the subject really arrived on stage from the waitroom"
    );
    assert!(
        !game.state.player1.waitroom.cards.contains(&subject),
        "precondition: the subject left the waitroom"
    );
    assert_eq!(
        game.state.mods.get_heart_modifier(subject, HeartColor::Heart06),
        1,
        "手札以外から登場したとき、ライブ終了までheart06を得る"
    );
    assert_eq!(
        game.state.player1.energy_zone.active_count(),
        2,
        "4E of 6 paid for the 起動"
    );
}

/// 手札から — a member played out of the hand 「登場」s too, but is not
/// 「手札以外から」and must gain nothing.
#[test]
fn a_member_played_from_hand_gains_nothing() {
    let mut game = setup();
    let subject = game.id(SUBJECT);
    game.state.player1.hand.cards.push(subject);
    game.state.player1.stage.stage = [-1, -1, -1];

    game.play_to_stage(subject, MemberArea::LeftSide);
    let mut guard = 0;
    loop {
        guard += 1;
        assert!(guard < 20, "play chain did not drain in 20 prompts");
        if !game.has_pending_choice() {
            break;
        }
        match game.get_pending_choice() {
            Choice::SelectAutoAbility { .. } => game.select_indices(&[]),
            other => panic!("unexpected prompt in the play chain: {:?}", other),
        }
    }

    assert!(
        game.state.player1.stage.stage.contains(&subject),
        "precondition: the member is on stage"
    );
    assert_eq!(
        game.state.mods.get_heart_modifier(subject, HeartColor::Heart06),
        0,
        "手札からの登場は「手札以外から登場したとき」ではない — no heart06"
    );
}
