/// Untested-abilities batch 18 — revealed-cards (エールで公開) conditions:
/// - PL!SP-bp4-026-L (ライブ成功時): score +1 with ≥5 distinct Liella! members revealed
/// - PL!SP-bp4-006-R (ライブ成功時): ≥3 distinct Liella! members revealed →
///   retrieve a Liella! live card from the revealed cards to hand
use crate::helpers::*;
use rabuka_engine::core::types::AbilityTrigger;

const KANON: &str = "PL!SP-pb1-001-PR"; // 澁谷かのん
const KEKE: &str = "PL!SP-bp1-004-PR"; // 唐可可
const REN: &str = "PL!SP-bp1-016-PR"; // 葉月 凛? (distinct name)
const SUMIRE: &str = "PL!SP-bp1-018-PR"; // 平安名すみれ
const WIEN: &str = "PL!SP-PR-017-PR"; // ウィーン・マルガレーテ
const CHISATO: &str = "PL!SP-pb1-014-PR"; // 嶋野あい? distinct Liella name

fn fire_trigger(game: &mut TestGame, cid: i16, trigger: AbilityTrigger, trig: &str) {
    let ability_id = {
        let card = game.db.get_card(cid).unwrap();
        let ab = card
            .resolved_abilities()
            .find(|a| a.triggers.as_deref() == Some(trig))
            .unwrap_or_else(|| panic!("card {} lacks a '{trig}' ability", card.card_no));
        format!("{}_{}", card.card_no, ab.full_text)
    };
    let card_no = game.db.get_card(cid).unwrap().card_no.to_string();
    let pid = game.state.player1.id.clone();
    game.state.trigger_auto_ability(
        ability_id,
        trigger,
        pid.clone(),
        Some(card_no),
        Some(cid),
        None,
        None,
    );
    game.state.activating_card = Some(cid);
    game.state.process_pending_auto_abilities(&pid);
}

// ====================================================================
// PL!SP-bp4-026-L Wish Song (ライブ成功時):
// 「エールにより公開されたカードの中に名前が異なる『Liella!』のメンバーカードが
//   5枚以上ある場合、このカードのスコアを＋１する。」
// ====================================================================

#[test]
fn wish_song_five_distinct_liella_revealed_scores() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let live = game.id("PL!SP-bp4-026-L");
    game.state.player1.live_card_zone.cards.push(live);

    for id in [game.id(KANON), game.id(KEKE), game.id(REN), game.id(SUMIRE), game.id(WIEN)] {
        game.state.revealed_cards.push(id);
    }

    fire_trigger(&mut game, live, AbilityTrigger::LiveSuccess, "ライブ成功時");

    assert_eq!(
        game.state.mods.get_score_modifier(live),
        1,
        "5 distinct Liella! members revealed -> score +1"
    );
}

#[test]
fn wish_song_four_distinct_liella_revealed_no_score() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let live = game.id("PL!SP-bp4-026-L");
    game.state.player1.live_card_zone.cards.push(live);

    for id in [game.id(KANON), game.id(KEKE), game.id(REN), game.id(SUMIRE)] {
        game.state.revealed_cards.push(id);
    }

    fire_trigger(&mut game, live, AbilityTrigger::LiveSuccess, "ライブ成功時");

    assert_eq!(
        game.state.mods.get_score_modifier(live),
        0,
        "only 4 distinct Liella! members -> no score"
    );
}

// ====================================================================
// PL!SP-bp4-006-R (ライブ成功時):
// 「エールにより公開されたカードの中に、名前が異なる『Liella!』のメンバーカードが
//   3枚以上ある場合、エールにより公開されたカードの中の『Liella!』のライブカードを
//   1枚手札に加える。」
// ====================================================================

#[test]
fn bp4006_three_distinct_members_retrieves_liella_live_from_revealed() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let me = game.id("PL!SP-bp4-006-R");
    game.state.player1.stage.stage[0] = me;

    for id in [game.id(KANON), game.id(KEKE), game.id(CHISATO)] {
        game.state.revealed_cards.push(id);
    }
    let liella_live = game.id("PL!SP-bp1-023-L"); // Liella! live card
    game.state.revealed_cards.push(liella_live);
    let hand_before = game.state.player1.hand.cards.len();

    fire_trigger(&mut game, me, AbilityTrigger::LiveSuccess, "ライブ成功時");

    assert_eq!(
        game.state.player1.hand.cards.len(),
        hand_before + 1,
        "Liella! live card retrieved from revealed cards to hand"
    );
    assert!(
        game.state.player1.hand.cards.contains(&liella_live),
        "the retrieved card is the revealed Liella! live card"
    );
}
