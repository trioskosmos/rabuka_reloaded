/// BP07 CLEAN-G8: PL!S-bp7-022-L 恋になりたいAQUARIUM ab#0 (常時).
///
/// 自分のエールは、デッキの上から行う代わりにデッキの下から行う。
/// (Your yell is performed from the BOTTOM of the deck instead of the top.)
///
/// The parser already emits custom{yell_source_modifier, yell_source:deck_bottom};
/// the engine had no support — the yell (cheer) always revealed from the top.
/// These tests drive the real cheer reveal and assert the revealed cards come
/// from the deck bottom when the live card is in play.
use crate::helpers::*;

const AQUARIUM: &str = "PL!S-bp7-022-L";
const DECK_TOP: &str = "PL!S-sd1-001-SD"; // 渡辺曜 (Aqours member) — top filler
const DECK_BOTTOM: &str = "PL!N-bp1-001-R"; // 上原歩夢 (虹ヶ咲) — bottom filler

/// The reveal pool is set-based — the yell must reveal the bottom N cards
/// (the reveal order within the pool has no game semantics). Compare as sets.
fn same_set(a: &[i16], b: &[i16]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut sa = a.to_vec();
    let mut sb = b.to_vec();
    sa.sort_unstable();
    sb.sort_unstable();
    sa == sb
}

/// Player1 has 恋になりたいAQUARIUM in the live zone and a deck of 6 cards:
/// indices 0-2 are the TOP three, indices 3-5 are the BOTTOM three.
fn setup(game: &mut TestGame) -> Vec<i16> {
    let aquarium = game.id(AQUARIUM);
    game.state.player1.live_card_zone.cards.push(aquarium);

    let top = [game.id(DECK_TOP), game.id(DECK_TOP), game.id(DECK_TOP)];
    let bottom = [game.id(DECK_BOTTOM), game.id(DECK_BOTTOM), game.id(DECK_BOTTOM)];
    game.state.player1.main_deck.cards.clear();
    game.state.player1.main_deck.cards.extend_from_slice(&top);
    game.state.player1.main_deck.cards.extend_from_slice(&bottom);
    bottom.to_vec()
}

/// The yell reveals from the deck BOTTOM when 恋になりたいAQUARIUM is in play.
#[test]
fn aquarium_yell_from_bottom() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());
    let bottom = setup(&mut game);

    // Refresh constant effects so the yell-source modifier registers.
    game.state.recalculate_constants();

    let pid = game.state.player1.id.clone();
    game.state.perform_cheer_check(&pid, 3).unwrap();

    let revealed: Vec<i16> = game.state.resolution_zone.cards.iter().copied().collect();
    assert!(
        same_set(&revealed, &bottom),
        "yell should reveal the 3 BOTTOM cards, got {:?}",
        revealed
    );
}

/// With no AQUARIUM in play, the yell reveals from the TOP (default).
#[test]
fn no_aquarium_yell_from_top() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let top = [
        game.id(DECK_TOP),
        game.id(DECK_TOP),
        game.id(DECK_TOP),
        game.id(DECK_BOTTOM),
        game.id(DECK_BOTTOM),
        game.id(DECK_BOTTOM),
    ];
    game.state.player1.main_deck.cards.clear();
    game.state.player1.main_deck.cards.extend_from_slice(&top);
    game.state.recalculate_constants();

    let pid = game.state.player1.id.clone();
    game.state.perform_cheer_check(&pid, 3).unwrap();

    let revealed: Vec<i16> = game.state.resolution_zone.cards.iter().copied().collect();
    assert_eq!(
        revealed,
        top[..3].to_vec(),
        "without AQUARIUM the yell reveals the TOP cards, got {:?}",
        revealed
    );
}

/// The opponent's AQUARIUM must NOT affect this player's yell source.
#[test]
fn opponent_aquarium_does_not_change_own_yell() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    // Opponent (p2) has the AQUARIUM; p1 has a plain deck.
    let aquarium = game.id(AQUARIUM);
    game.state.player2.live_card_zone.cards.push(aquarium);
    let top = [
        game.id(DECK_TOP),
        game.id(DECK_TOP),
        game.id(DECK_TOP),
        game.id(DECK_BOTTOM),
        game.id(DECK_BOTTOM),
        game.id(DECK_BOTTOM),
    ];
    game.state.player1.main_deck.cards.clear();
    game.state.player1.main_deck.cards.extend_from_slice(&top);
    game.state.recalculate_constants();

    let pid = game.state.player1.id.clone();
    game.state.perform_cheer_check(&pid, 3).unwrap();

    let revealed: Vec<i16> = game.state.resolution_zone.cards.iter().copied().collect();
    assert_eq!(
        revealed,
        top[..3].to_vec(),
        "only the OWN player's AQUARIUM changes the yell source"
    );
}

/// Yell from the bottom reveals fewer than blade_count when the deck has fewer
/// cards than the blade count (deck exhausts mid-yell).
#[test]
fn aquarium_yell_from_bottom_short_deck() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let aquarium = game.id(AQUARIUM);
    game.state.player1.live_card_zone.cards.push(aquarium);
    // Deck has only 2 cards (both bottom-ish).
    let only = [game.id(DECK_BOTTOM), game.id(DECK_BOTTOM)];
    game.state.player1.main_deck.cards.clear();
    game.state.player1.main_deck.cards.extend_from_slice(&only);
    game.state.recalculate_constants();

    let pid = game.state.player1.id.clone();
    game.state.perform_cheer_check(&pid, 5).unwrap();

    let revealed: Vec<i16> = game.state.resolution_zone.cards.iter().copied().collect();
    assert!(
        same_set(&revealed, &only),
        "short deck reveals only the 2 available bottom cards, got {:?}",
        revealed
    );
}

/// Yell from the bottom, then yell again — the SECOND yell reveals the NEXT
/// bottom cards (the deck is consumed from the bottom up, not reset).
#[test]
fn aquarium_yell_from_bottom_two_yells() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let aquarium = game.id(AQUARIUM);
    game.state.player1.live_card_zone.cards.push(aquarium);
    // 6 cards: bottom 3 are B, middle 3 are T.
    let t = game.id(DECK_TOP);
    let b = game.id(DECK_BOTTOM);
    game.state.player1.main_deck.cards.clear();
    game.state.player1.main_deck.cards.extend_from_slice(&[t, t, t, b, b, b]);
    game.state.recalculate_constants();

    let pid = game.state.player1.id.clone();
    // First yell: blade 2 → reveals 2 from the bottom (the last 2 B cards).
    game.state.perform_cheer_check(&pid, 2).unwrap();
    let first: Vec<i16> = game.state.resolution_zone.cards.iter().copied().collect();
    game.state.resolution_zone.cards.clear();

    // Second yell: blade 1 → reveals the next-from-bottom (the 3rd B card).
    game.state.perform_cheer_check(&pid, 1).unwrap();
    let second: Vec<i16> = game.state.resolution_zone.cards.iter().copied().collect();

    assert_eq!(first.len(), 2, "first yell reveals 2");
    assert_eq!(second.len(), 1, "second yell reveals 1");
    assert!(
        first.iter().all(|&c| c == b) && second.iter().all(|&c| c == b),
        "both yells consume from the bottom (B cards) first, got first={:?} second={:?}",
        first,
        second
    );
    // After 3 B cards are gone, the deck is down to the 3 T cards.
    assert_eq!(
        game.state.player1.main_deck.cards.len(),
        3,
        "3 bottom cards consumed across the two yells"
    );
}

/// Removing 恋になりたいAQUARIUM from the live zone reverts the yell to the top.
#[test]
fn aquarium_leaves_zone_reverts_to_top() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let aquarium = game.id(AQUARIUM);
    game.state.player1.live_card_zone.cards.push(aquarium);
    let top = [
        game.id(DECK_TOP),
        game.id(DECK_TOP),
        game.id(DECK_TOP),
        game.id(DECK_BOTTOM),
        game.id(DECK_BOTTOM),
        game.id(DECK_BOTTOM),
    ];
    game.state.player1.main_deck.cards.clear();
    game.state.player1.main_deck.cards.extend_from_slice(&top);
    game.state.recalculate_constants();

    // AQUARIUM leaves the zone → refresh → yell back to top.
    game.state.player1.live_card_zone.cards.clear();
    game.state.recalculate_constants();

    let pid = game.state.player1.id.clone();
    game.state.perform_cheer_check(&pid, 3).unwrap();

    let revealed: Vec<i16> = game.state.resolution_zone.cards.iter().copied().collect();
    assert_eq!(
        revealed,
        top[..3].to_vec(),
        "after AQUARIUM leaves, the yell reveals the TOP cards again"
    );
}

/// The AQUARIUM in the SUCCESS zone (after a successful live) still applies.
#[test]
fn aquarium_in_success_zone_applies() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let aquarium = game.id(AQUARIUM);
    game.state.player1.success_live_card_zone.cards.push(aquarium);
    let top = [
        game.id(DECK_TOP),
        game.id(DECK_TOP),
        game.id(DECK_TOP),
        game.id(DECK_BOTTOM),
        game.id(DECK_BOTTOM),
        game.id(DECK_BOTTOM),
    ];
    game.state.player1.main_deck.cards.clear();
    game.state.player1.main_deck.cards.extend_from_slice(&top);
    game.state.recalculate_constants();

    let pid = game.state.player1.id.clone();
    game.state.perform_cheer_check(&pid, 3).unwrap();

    let revealed: Vec<i16> = game.state.resolution_zone.cards.iter().copied().collect();
    assert!(
        same_set(&revealed, &top[3..].to_vec()),
        "AQUARIUM in the success zone still forces yell-from-bottom, got {:?}",
        revealed
    );
}

