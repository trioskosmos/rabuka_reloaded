/// BP07 CLEAN-G18: PL!N-bp7-020-N エマ・ヴェルデ ab#0 (登場).
///
/// 登場：自分のデッキの上からカードを3枚控え室に置く。それらのメンバーカードの中に
/// 2種類以上のブレードハートの色がある場合、ライブ終了時まで、heart04を得る。
///
/// (Debut) Place the top 3 cards of your deck into the discard. If among those
/// member cards there are >= 2 DISTINCT blade-heart colors, until live end, gain
/// heart04.
///
/// Gameplay edge cases:
///   1. 2 distinct blade-heart colors among the milled members → heart04.
///   2. 3 distinct colors → heart04.
///   3. All milled members share 1 color → NO heart04.
///   4. Only 1 milled card is a member (others non-member) → NO heart04.
///   5. Milled members have no blade heart → NO heart04.
use crate::helpers::*;
use rabuka_engine::card::HeartColor;
use rabuka_engine::core::types::AbilityTrigger;

const EMMA: &str = "PL!N-bp7-020-N"; // エマ・ヴェルデ (member, cost 5), ab#0 登場
const KARIN_B05: &str = "PL!N-sd1-004-SD"; // 朝香果林 — blade heart b_heart05
const KANATA_B06: &str = "PL!N-sd1-006-SD"; // 近江彼方 — blade heart b_heart06
const HONOKA_B03: &str = "PL!-sd1-010-SD"; // 高坂穂乃果 — blade heart b_heart03
const NO_BLADE: &str = "PL!N-sd1-002-SD"; // 中須かすみ — no blade heart
const ENERGY: &str = "LL-E-001-SD";

fn trigger_debut(game: &mut TestGame, card_id: i16) {
    let card = game.db.get_card(card_id).unwrap();
    let ab = card
        .resolved_abilities()
        .find(|a| a.triggers.as_deref() == Some("登場"))
        .expect("card should have a 登場 ability");
    let pid = game.state.player1.id.clone();
    game.state.trigger_auto_ability(
        format!("{}_{}", card.card_no, ab.full_text),
        AbilityTrigger::Debut,
        pid.clone(),
        Some(card.card_no.to_string()),
        Some(card_id),
        None,
        None,
    );
    game.state.activating_card = Some(card_id);
    game.state.process_pending_auto_abilities(&pid);
    game.drain_auto_ability_choices();
}

/// エマ on stage with the given top-3 deck cards; returns emma's id.
fn setup(game: &mut TestGame, top3: [i16; 3]) -> i16 {
    let emma = game.id(EMMA);
    game.state.player1.stage.stage[1] = emma;
    game.state.player1.main_deck.cards.clear();
    for c in top3 {
        game.state.player1.main_deck.cards.push(c); // index 0 = top
    }
    // Filler deck below so the mill has depth.
    let f = game.id(NO_BLADE);
    for _ in 0..6 {
        game.state.player1.main_deck.cards.push(f);
    }
    emma
}

fn heart04(game: &TestGame, id: i16) -> i32 {
    game.state.mods.get_heart_modifier(id, HeartColor::Heart04)
}

/// 1. Two milled members with 2 distinct blade-heart colors → heart04 gained.
#[test]
fn emma_two_distinct_colors_gains_heart04() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let karin = game.id(KARIN_B05);
    let kanata = game.id(KANATA_B06);
    let filler = game.id(NO_BLADE);
    let emma = setup(&mut game, [karin, karin, kanata]); // colors {b05, b06} = 2 distinct

    trigger_debut(&mut game, emma);
    assert_eq!(
        heart04(&game, emma),
        1,
        "2 distinct blade-heart colors among milled members → heart04, got {}",
        heart04(&game, emma)
    );
    let _ = filler;
}

/// 2. Three milled members with 3 distinct colors → heart04 gained.
#[test]
fn emma_three_distinct_colors_gains_heart04() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let karin = game.id(KARIN_B05);
    let kanata = game.id(KANATA_B06);
    let honoka = game.id(HONOKA_B03);
    let emma = setup(&mut game, [karin, kanata, honoka]); // {b05,b06,b03} = 3 distinct

    trigger_debut(&mut game, emma);

    assert_eq!(
        heart04(&game, emma),
        1,
        "3 distinct blade-heart colors → heart04, got {}",
        heart04(&game, emma)
    );
}

/// 3. All milled members share ONE color → NO heart04.
#[test]
fn emma_all_one_color_no_heart04() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let karin = game.id(KARIN_B05);
    let emma = setup(&mut game, [karin, karin, karin]); // {b05} = 1 distinct

    trigger_debut(&mut game, emma);

    assert_eq!(
        heart04(&game, emma),
        0,
        "1 distinct blade-heart color → no heart04, got {}",
        heart04(&game, emma)
    );
}

/// 4. Only 1 of the 3 milled cards is a member → NO heart04 (needs >=2 distinct
/// among the member cards).
#[test]
fn emma_only_one_member_milled_no_heart04() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let karin = game.id(KARIN_B05);
    let energy = game.id(ENERGY);
    let emma = setup(&mut game, [karin, energy, energy]); // 1 member card

    trigger_debut(&mut game, emma);

    assert_eq!(
        heart04(&game, emma),
        0,
        "only 1 milled member card → no heart04, got {}",
        heart04(&game, emma)
    );
}

/// 5. Milled members have NO blade heart → NO heart04 (base hearts don't count).
#[test]
fn emma_members_without_blade_heart_no_heart04() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let no_blade = game.id(NO_BLADE);
    let emma = setup(&mut game, [no_blade, no_blade, no_blade]); // 0 blade-heart colors

    trigger_debut(&mut game, emma);

    assert_eq!(
        heart04(&game, emma),
        0,
        "no blade-heart colors → no heart04, got {}",
        heart04(&game, emma)
    );
}
