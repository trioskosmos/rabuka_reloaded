//! 中須かすみ `PL!N-PR-033-PR` (自動・ターン1回)
//!
//! 自分の控え室からメンバーが登場したとき、カードを1枚引く。
//!
//! The condition carries `appearance_source: "discard"` — a member arriving from
//! ANY other zone (hand, deck, baton touch off the hand) is not 「控え室から」
//! and must not draw. The revival recipe is the one already proven against
//! `PL!S-bp6-008-R` in `debut_draw_two_blade_only_when_revived_from_discard_*`.

use crate::helpers::*;
use rabuka_engine::zones::MemberArea;

const KASUMI: &str = "PL!N-PR-033-PR";
/// 起動 (E E): このメンバーをステージから控え室に置く：自分の控え室からコスト17以下の
/// 『Aqours』のメンバーカードを1枚、このメンバーがいたエリアに登場させる。
const MARI: &str = "PL!S-bp6-008-R";
/// Abilityless 『Aqours』 member at cost 4 — qualifies for the revival.
const REVIVED: &str = "PL!S-bp2-014-N";
/// Abilityless member whose name is neither 星空 凛 nor 小泉花陽.
const FILLER: &str = "PL!-sd1-010-SD";

fn drain_auto(v: &mut TestGame) {
    while v.has_pending_choice() {
        match v.pending_choice_type().as_deref() {
            Some("SelectAutoAbility") => v.select_indices(&[0]),
            _ => v.select_indices(&[]),
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
    game.give_energy(20);
    game
}

/// The trigger source is the waitroom — a member arriving from there draws.
#[test]
fn a_member_appearing_from_the_waitroom_draws_one() {
    let mut game = setup();

    let kasumi = game.id(KASUMI);
    let mari = game.id(MARI);
    let revived = game.id(REVIVED);
    game.state.player1.stage.stage = [kasumi, mari, -1];
    game.state.player1.waitroom.cards.push(revived);
    assert_eq!(
        game.state.player1.hand.cards.len(),
        0,
        "precondition: hand starts empty"
    );

    game.activate_ability(mari);
    assert!(
        game.has_pending_choice(),
        "008's revival needs a SelectCard prompt off the waitroom"
    );
    let idx = game
        .state
        .player1
        .waitroom
        .cards
        .iter()
        .position(|&c| c == revived)
        .expect("the revival candidate is in the waitroom");
    game.select_indices(&[idx]);
    drain_auto(&mut game);

    assert!(
        game.state.player1.stage.stage.contains(&revived),
        "precondition: the member actually arrived on stage from the waitroom"
    );
    assert_eq!(
        game.state.player1.hand.cards.len(),
        1,
        "自分の控え室からメンバーが登場したとき、カードを1枚引く"
    );
}

/// Same arrival, a second time in the SAME turn — ターン1回 stops the second draw.
#[test]
fn only_the_first_waitroom_arrival_in_a_turn_draws() {
    let mut game = setup();

    let kasumi = game.id(KASUMI);
    let mari_a = game.id(MARI);
    let revived_a = game.id(REVIVED);
    game.state.player1.stage.stage = [kasumi, mari_a, -1];
    game.state.player1.waitroom.cards.push(revived_a);

    game.activate_ability(mari_a);
    let idx = game
        .state
        .player1
        .waitroom
        .cards
        .iter()
        .position(|&c| c == revived_a)
        .expect("the revival candidate is in the waitroom");
    game.select_indices(&[idx]);
    drain_auto(&mut game);

    assert_eq!(
        game.state.player1.hand.cards.len(),
        1,
        "precondition: the first waitroom arrival drew"
    );

    // mari leaves the stage as her own cost, so exactly one slot is open for a
    // second reviver — and a second waitroom arrival is waiting to use it.
    let revived_b = game.new_id(REVIVED);
    let mari_b = game.new_id(MARI);
    game.state.player1.waitroom.cards.push(revived_b);
    game.state.player1.hand.cards.push(mari_b);
    game.give_energy(40);

    let free = game
        .state
        .player1
        .stage
        .stage
        .iter()
        .position(|&s| s == -1)
        .expect("mari_a's own cost freed a stage slot");
    let area = rabuka_engine::zones::MemberArea::from_index(free).expect("0..3");
    game.play_to_stage(mari_b, area);
    drain_auto(&mut game);
    assert!(game.state.player1.stage.stage.contains(&mari_b));

    game.activate_ability(mari_b);
    let idx = game
        .state
        .player1
        .waitroom
        .cards
        .iter()
        .position(|&c| c == revived_b)
        .expect("the second revival candidate is in the waitroom");
    game.select_indices(&[idx]);
    drain_auto(&mut game);

    assert!(
        game.state.player1.stage.stage.contains(&revived_b),
        "precondition: the second member really arrived from the waitroom"
    );
    assert_eq!(
        game.state.player1.hand.cards.len(),
        1,
        "ターン1回 — the second waitroom arrival of the same turn must not draw"
    );
}

/// 「控え室から」 — a member played out of the HAND is not a waitroom arrival.
#[test]
fn a_member_played_from_hand_draws_nothing() {
    let mut game = setup();

    let kasumi = game.id(KASUMI);
    let from_hand = game.id(REVIVED);
    game.state.player1.stage.stage = [kasumi, -1, -1];
    game.state.player1.hand.cards.push(from_hand);

    game.play_to_stage(from_hand, MemberArea::Center);
    drain_auto(&mut game);

    assert!(game.state.player1.stage.stage.contains(&from_hand));
    assert_eq!(
        game.state.player1.hand.cards.len(),
        0,
        "手札からの登場は「自分の控え室から」ではない — no draw"
    );
}
