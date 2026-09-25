use crate::helpers::*;
use rabuka_engine::zones::MemberArea;

fn baton_touch_over(game: &mut TestGame, kanon: i16, arriver: i16) {
    let _ = kanon;
    game.state.player1.hand.cards.push(arriver);
    game.play_to_stage(arriver, MemberArea::Center);
    while game.has_pending_choice() {
        // Default: skip any optional choice; required discards pick the first card.
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

#[test]
fn baton_displaced_self_placed_under_liella_arriver_grants_host_blade() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    // 澁谷かのん on center stage.
    let kanon = game.id("PL!SP-bp7-001-R");
    game.state.player1.stage.stage[1] = kanon;

    // Filler deck so debut "draw" prompts resolve cleanly.
    let filler = game.id("PL!-sd1-010-SD");
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
    }
    game.give_energy(25);

    // Baton-touch a Liella! arriver (平安名すみれ) over 澁谷かのん.
    let arriver = game.id("PL!SP-sd1-004-SD");
    baton_touch_over(&mut game, kanon, arriver);

    assert_eq!(
        game.state.player1.stage.stage[1], arriver,
        "arriver should occupy center after baton-touch"
    );

    // ab#1: 澁谷かのん should now be UNDER the arriver, not sitting in the waitroom.
    let under = game.state.player1.stage.get_under_cards(MemberArea::Center);
    assert!(
        under.contains(&kanon),
        "ab#1 should place 澁谷かのん under the arriving member; under={:?}",
        under
    );
    assert!(
        !game.state.player1.waitroom.cards.contains(&kanon),
        "澁谷かのん must be under the arriver (ab#1), not in the waitroom"
    );

    // ab#0: the Liella! arriver host now gains a blade.
    let blade_mod = game.state.mods.get_blade_modifier(arriver);
    assert_eq!(
        blade_mod, 1,
        "host Liella! member with 澁谷かのん underneath → 1 blade, got {}",
        blade_mod
    );
}
