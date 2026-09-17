use crate::helpers::*;
use crate::test_modules::support::ability_trigger_and_deck_setup::*;

#[test]
fn look_per_group_member_discards_remainder_without_score_for_member_reveal() {
    let db = load_real_database();
    let mut g = TestGame::new(db);
    let l = g.id("PL!N-bp3-028-L");
    let n = g.id("PL!N-bp1-001-R");
    let f = g.id("PL!-sd1-010-SD");
    g.state.player1.stage.stage = [n, n, n];
    g.state.player1.hand.cards.push(l);
    fill_both_main_decks(&mut g, f);
    g.give_energy(5);
    trigger_printed_ability_and_resolve_choices(&mut g, l, "ライブ開始時");
    while g.has_pending_choice() {
        g.select_indices(&[0]);
    }
    // Deck was all fillers (no live card): the reveal cannot grant score.
    assert_eq!(
        g.state.mods.get_score_modifier(l),
        0,
        "no live card revealed means no score bonus"
    );
    // 3 cards were looked at (one per Niji member); at most 1 stays on deck,
    // so the waitroom must have received the rest of the peeked cards.
    assert!(
        g.state.player1.waitroom.cards.len() >= 2,
        "peeked cards (3 minus at most 1 kept) must go to the waitroom"
    );
}
