use crate::helpers::*;
use rabuka_engine::core::types::AbilityTrigger;

const KANON: &str = "PL!SP-pb1-001-PR"; // 澁谷かのん — 『Liella!』, unit CatChu!
const SUMIRE: &str = "PL!SP-bp1-004-PR"; // 平安名すみれ — 『Liella!』, unit CatChu!
/// 嵐 千砂都 — 『Liella!』, unit 5yncri5e!, and a THIRD name distinct from both above.
///
/// The id here used to be `PL!SP-bp1-014-PR`, WHICH DOES NOT EXIST. The lenient
/// card-id fallback substituted `PL!SP-bp1-014-N` (嵐 千砂都) and the test passed
/// anyway — so the "three DISTINCT names" premise this test is named for was never
/// actually pinned, and a distinct-NAME count is exactly the claim a substituted
/// card can silently invalidate. The id is now spelled out and identity-asserted
/// below, so a future edit cannot quietly restore the substitution.
///
/// The negatives for this condition (fewer than three names, a non-『Liella!』 name,
/// and the count holding while no 『Liella!』 live is revealed) live in
/// `livesuccess_revealed_distinct_name_threshold_test.rs`.
const CHISATO: &str = "PL!SP-bp1-014-N";
const KEKE: &str = "PL!SP-bp4-006-R";
const LIELLA_LIVE: &str = "PL!SP-bp1-023-L";

#[test]
fn sp_bp4_006_live_success_with_three_distinct_liella_revealed_retrieves_liella_live_to_hand() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let me = game.id(KEKE);
    game.state.player1.stage.stage[0] = me;

    let members = [game.id(KANON), game.id(SUMIRE), game.id(CHISATO)];
    // The premise this test's name claims, asserted rather than assumed.
    for (id, card_no) in members.iter().zip([KANON, SUMIRE, CHISATO]) {
        game.assert_card_identity(*id, card_no);
        game.assert_card_in_group(*id, "Liella!", "each revealed member is 『Liella!』");
    }
    let names: std::collections::HashSet<String> = members
        .iter()
        .map(|id| game.db.get_card(*id).unwrap().name.to_string())
        .collect();
    assert_eq!(
        names.len(),
        3,
        "precondition: the three revealed members must carry three DISTINCT names \
         (got {names:?})"
    );

    for id in members {
        game.state.revealed_cards.push(id);
    }
    let liella_live = game.id(LIELLA_LIVE);
    game.state.revealed_cards.push(liella_live);
    let hand_before = game.state.player1.hand.cards.len();

    fire_trigger(&mut game, me, AbilityTrigger::LiveSuccess, "ライブ成功時");

    assert_eq!(
        game.state.player1.hand.cards.len(),
        hand_before + 1,
        "『Liella!』 live card retrieved from revealed cards to hand"
    );
    assert!(
        game.state.player1.hand.cards.contains(&liella_live),
        "the retrieved card is the revealed 『Liella!』 live card"
    );
}
