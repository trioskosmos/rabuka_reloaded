/// BP07 C3: PL!SP-bp7-001-R 澁谷かのん ab#1 (自動).
///
/// 自動：このメンバーがステージから控え室に置かれたとき、バトンタッチしていた場合、
/// このカードをそのバトンタッチで登場したメンバーの下に置く。
///
/// (Auto) When THIS member is placed from the stage into the waitroom, IF it was
/// baton-touched (i.e. it was the replaced member), place THIS card under the
/// member that appeared via that baton-touch.
///
/// Nuances pinned here:
///   - Positive: a member baton-touches over 澁谷かのん → she is placed UNDER the
///     arriving member (not left in the waitroom).
///   - Negative (no baton touch): 澁谷かのん goes stage→waitroom by any other means
///     (discard effect / self-cost) → ab#1 does NOT fire; she stays in the waitroom.
///   - The host is the SPECIFIC member that arrived in her slot.
///   - When 澁谷かのん is the ARRIVER (she baton-touches in, is NOT displaced), her
///     ab#1 does NOT re-place herself.
use crate::helpers::*;
use rabuka_engine::zones::MemberArea;

const KANON: &str = "PL!SP-bp7-001-R"; // 澁谷かのん (Liella!)
const ARRIVER: &str = "PL!SP-sd1-004-SD"; // 平安名すみれ (Liella!)
const FILLER: &str = "PL!-sd1-010-SD";

fn setup_deck_and_energy(game: &mut TestGame) {
    let filler = game.id(FILLER);
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
    }
    game.give_energy(25);
}

/// Play `arriver` onto 澁谷かのん's occupied center area via baton touch, draining
/// any auto-ability prompts (borrowed from bp7_kanon_under_member_blade_test).
fn baton_touch_over(game: &mut TestGame, arriver: i16) {
    game.state.player1.hand.cards.push(arriver);
    game.play_to_stage(arriver, MemberArea::Center);
    while game.has_pending_choice() {
        let required = game.state.get_pending_choice().is_some_and(|c| {
            matches!(
                c,
                rabuka_engine::ability::types::Choice::SelectCard {
                    count: 1,
                    allow_skip: false,
                    ..
                }
            )
        });
        if required {
            game.select_indices(&[0]);
        } else {
            game.select_indices(&[]);
        }
    }
}

fn under_center(game: &TestGame) -> Vec<i16> {
    game.state.player1.stage.get_under_cards(MemberArea::Center).to_vec()
}

/// Positive: baton-touching over 澁谷かのん places her under the arriving member.
#[test]
fn kanon_baton_touch_places_under_arriver() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let kanon = game.id(KANON);
    game.state.player1.stage.stage[1] = kanon;
    setup_deck_and_energy(&mut game);

    let arriver = game.id(ARRIVER);
    baton_touch_over(&mut game, arriver);

    assert_eq!(game.state.player1.stage.stage[1], arriver, "arriver occupies center");
    assert!(
        under_center(&game).contains(&kanon),
        "ab#1 should place 澁谷かのん under the arriving member; under={:?}",
        under_center(&game)
    );
    assert!(
        !game.state.player1.waitroom.cards.contains(&kanon),
        "baton-touched 澁谷かのん must be under the arriver, not in the waitroom"
    );
}

/// Negative: 澁谷かのん goes stage→waitroom WITHOUT baton touch (discard effect).
/// ab#1 (baton_touch_trigger) must NOT fire → she stays in the waitroom.
#[test]
fn kanon_no_baton_touch_stays_in_waitroom() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let kanon = game.id(KANON);
    game.state.player1.stage.stage[1] = kanon;
    setup_deck_and_energy(&mut game);

    // Remove her from the stage and put her in the waitroom by a NON-baton-touch
    // means (simulating a discard effect), then scan auto abilities.
    game.state.player1.stage.stage[1] = -1;
    game.state.player1.waitroom.cards.push(kanon);
    let pid = game.state.player1.id.clone();
    rabuka_engine::turn::TurnEngine::trigger_auto_abilities_for_player(&mut game.state, &pid);
    game.state.process_pending_auto_abilities(&pid);
    game.drain_auto_ability_choices();

    assert!(
        game.state.player1.waitroom.cards.contains(&kanon),
        "non-baton-touch removal must leave 澁谷かのん in the waitroom"
    );
    assert!(
        under_center(&game).is_empty(),
        "no member under center — ab#1 must not fire without baton touch"
    );
}

/// When 澁谷かのん baton-touches IN (is the arriver, not displaced), her own ab#1
/// must NOT re-place her under anything — she stays on stage.
#[test]
fn kanon_as_arriver_not_displaced() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    // A filler member occupies center; 澁谷かのん will baton-touch in over it.
    let displaced = game.id(FILLER);
    game.state.player1.stage.stage[1] = displaced;
    setup_deck_and_energy(&mut game);

    let kanon = game.id(KANON);
    game.state.player1.hand.cards.push(kanon);
    game.play_to_stage(kanon, MemberArea::Center);
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    assert_eq!(
        game.state.player1.stage.stage[1], kanon,
        "澁谷かのん should be on center after baton-touching in"
    );
    // 澁谷かのん (the arriver) must NOT be placed under anyone.
    assert!(
        !under_center(&game).contains(&kanon),
        "the arriver 澁谷かのん must not be placed under herself"
    );
}

/// The displaced member is placed under the SPECIFIC arriver in its slot. A member
/// in a different slot must NOT receive it.
#[test]
fn kanon_placed_under_specific_arriver_slot() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let kanon = game.id(KANON);
    let other = game.id(ARRIVER); // another Liella! on the right slot
    game.state.player1.stage.stage = [other, kanon, -1];
    setup_deck_and_energy(&mut game);

    let arriver = game.id(ARRIVER);
    baton_touch_over(&mut game, arriver); // baton touch into CENTER over kanon

    assert!(
        under_center(&game).contains(&kanon),
        "kanon should be under the center arriver; under={:?}",
        under_center(&game)
    );
    // The right-slot member must have nothing under it.
    let under_right = game.state.player1.stage.get_under_cards(MemberArea::RightSide).to_vec();
    assert!(
        !under_right.contains(&kanon),
        "kanon must NOT go under the right-slot member; under_right={:?}",
        under_right
    );
}
