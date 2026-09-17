use crate::helpers::*;

// ====================================================================
// PL!SP-bp4-018-N (起動):
// 「このメンバーをステージから控え室に置く：
//   自分の控え室から『Liella!』のカードを1枚手札に加える。」
// ====================================================================

#[test]
fn sp_bp4_018_activation_self_to_waitroom_recovers_liella_card() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let me = game.id("PL!SP-bp4-018-N"); // CatChu! member
    game.state.player1.stage.stage[0] = me;

    // A Liella! member (澁谷かのん, CatChu! sub-unit) sits in the waitroom.
    let liella = game.id("PL!SP-pb1-001-PR");
    game.state.player1.waitroom.cards.push(liella);

    game.activate_ability(me);
    assert!(
        game.has_pending_choice(),
        "waitroom retrieval must be prompted (self now in waitroom = 2 candidates)"
    );
    assert_eq!(
        game.pending_choice_type().as_deref(),
        Some("SelectCard"),
        "expected SelectCard retrieval prompt"
    );
    game.select_indices(&[0]);

    assert!(
        game.state.player1.waitroom.cards.contains(&me),
        "this member moved to the waitroom"
    );
    assert!(
        game.state.player1.hand.cards.contains(&liella),
        "Liella! card retrieved to hand"
    );
}
