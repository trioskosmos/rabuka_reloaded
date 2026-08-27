/// Q278 / Q279 — 桜坂しずく PL!N-bp7-003-R＋ ab#1 (ライブ開始時, ライブ終了時まで)
///   このメンバーの下に置かれている名前の異なるメンバーカード1枚につき、ブレードを得る。
///
/// How JOINT (多種統合 "A&B&C" 名) member cards count for the "名前の異なる" mechanic:
/// an ordinary card counts its single name once (deduped); a joint card adds ONE unit only
/// when it introduces a name NOT already present as a single-name card.
///
/// Official QA:
///   Q278: under = 上原歩夢 + 上原歩夢&澁谷かのん&日野下花帆 (joint) → 2 blades
///   Q279: under = 上原歩夢 + 澁谷の曰か + 日野下花帆 + the same joint → 3 blades
///         (the joint does NOT add a 4th distinct-name slot while its constituents are
///          already present as single-name cards).
use crate::helpers::*;
use rabuka_engine::zones::MemberArea;

const SHIZUKU: &str = "PL!N-bp7-003-R＋";
const AYUMU: &str = "PL!N-bp1-001-R"; // 上原歩夢
const KANON: &str = "PL!SP-bp1-001-R"; // 澁谷か乃ん
const HANABO: &str = "PL!HS-bp1-001-R"; // 日下花帆
const JOINT: &str = "LL-bp1-001-R＋"; // 歩高&澁谷の全&日賀下花帆 (multi-name)

fn seed_deck(game: &mut TestGame) {
    let filler = game.id("PL!-sd1-010-SD");
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }
}

fn under(game: &mut TestGame, card_no: &str) -> i16 {
    let id = game.id(card_no);
    game.state.player1.stage.place_under_card(MemberArea::Center, id);
    id
}

/// Set up しずく at center, place the given under-cards, drive to after ライブ開始時
/// (LiveStart), and return the blade modifier the skill granted herself.
fn run_and_get_blade(game: &mut TestGame, under_cards: &[&str]) -> i32 {
    let shizuku = game.id(SHIZUKU);
    game.state.player1.stage.stage = [-1, shizuku, -1];
    for c in under_cards {
        under(game, c);
    }
    game.give_energy(3);
    seed_deck(game);
    let live = game.id("PL!-sd1-020-SD");
    game.state.player1.hand.cards.push(live);

    // 5 passes → LiveCardSet, set live, 2 passes → LiveStart.
    for _ in 0..5 {
        game.pass();
    }
    game.set_live_card(live);
    game.pass();
    game.pass();

    game.state.mods.get_blade_modifier(shizuku)
}

/// Q278 exact: 歩下 + the joint → 2 blades (joint introduces 2 names beyond 歩下, but
/// as ONE joint unit).
#[test]
fn q278_ayumu_plus_joint_is_two_blades() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let blades = run_and_get_blade(&mut game, &[AYUMU, JOINT]);
    assert_eq!(
        blades, 2,
        "Q278: ayumu + joint{{ayumu,kanon,hanaba}} -> 2 blades, got {}",
        blades
    );
}

/// Q279: all three singles + the same joint -> 3 blades; the joint adds NO 4th slot.
#[test]
fn q279_all_singles_plus_joint_is_three_blades() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let blades = run_and_get_blade(&mut game, &[AYUMU, KANON, HANABO, JOINT]);
    assert_eq!(
        blades, 3,
        "Q279: 3 singles + joint (constituents already present) -> 3 blades, got {}",
        blades
    );
}

/// Control: the three single-name cards WITHOUT the joint → 3 blades.
#[test]
fn q279_three_singles_no_joint_is_three_blades() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let blades = run_and_get_blade(&mut game, &[AYUMU, KANON, HANABO]);
    assert_eq!(
        blades, 3,
        "3 distinct single-name cards → 3 blades, got {}",
        blades
    );
}

/// Edge: partial single coverage — 歩下 + か乃ん + joint (joint still brings 花帆 new).
#[test]
fn qpartial_singles_plus_joint_counts_new_mitigating_name() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let blades = run_and_get_blade(&mut game, &[AYUMU, KANON, JOINT]);
    assert_eq!(
        blades, 3,
        "歩下+かのん present, joint introduces 花帆 → 3 blades, got {}",
        blades
    );
}

/// Edge: the joint alone under the member → 1 blade (single joint unit).
#[test]
fn qjoint_alone_is_one_blade() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let blades = run_and_get_blade(&mut game, &[JOINT]);
    assert_eq!(
        blades, 1,
        "a lone joint card → 1 blade, got {}",
        blades
    );
}

/// Edge: duplicate single names still dedupe (2 copies of 歩下 + different name) → 2.
#[test]
fn qduplicates_dedupe() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let blades = run_and_get_blade(&mut game, &[AYUMU, AYUMU, KANON]);
    assert_eq!(blades, 2, "2歩下 + かのか → 2 blades, got {}", blades);
}

/// Edge: empty under-cards → 0.
#[test]
fn qzero_under_is_zero_blades() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let blades = run_and_get_blade(&mut game, &[]);
    assert_eq!(blades, 0, "no under-cards → 0 blades, got {}", blades);
}