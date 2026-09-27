use crate::helpers::*;
use crate::test_modules::support::ability_trigger_and_deck_setup::*;

#[test]
fn discard_to_activate_opponent_recovers_live_from_waitroom() {
    let db = load_real_database();
    let mut g = TestGame::new(db);
    let c = g.id("PL!-bp5-111-R");
    let f = g.id("PL!-sd1-010-SD");
    g.state.player1.stage.stage = [-1, c, -1];
    let opp = g.id("PL!-sd1-001-SD");
    g.state.player2.stage.stage = [-1, opp, -1];
    g.state.mods.add_orientation_modifier(opp, "wait");
    // A live card to recover from the waitroom when the activated member is
    // the opponent's.
    let live = g.id("PL!-sd1-020-SD");
    g.state.player1.waitroom.cards.push(live);
    g.state.player1.hand.cards.push(f);
    fill_both_main_decks(&mut g, f);
    g.give_energy(5);
    g.activate_ability(c);
    let mut guard = 0;
    while g.has_pending_choice() && guard < 10 {
        match g.pending_choice_type().as_deref() {
            Some("SelectCard") => {
                g.select_indices(&[0]);
            }
            Some("SelectTarget") => {
                g.select_option(0);
            }
            _ => {
                g.select_indices(&[]);
            }
        }
        guard += 1;
    }
    // The waited opponent member was activated by the ability.
    assert_eq!(
        g.state.mods.get_orientation_modifier(opp),
        Some("active"),
        "waited opponent member must be active after activation"
    );
    // Activating an OPPONENT member recovers 1 live card from the waitroom.
    assert!(
        g.state.player1.hand.cards.contains(&live),
        "live card must be recovered from waitroom to hand"
    );
}
