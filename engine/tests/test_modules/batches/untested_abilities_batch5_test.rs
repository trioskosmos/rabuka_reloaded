/// Untested abilities — batch 5, plus cross-card interaction coverage.
///
///   - PL!N-sd2-025-P ワンダーメイツ ライブ開始時: activate 1 虹ヶ咲 member —
///     INTERACTION: the activated member becomes able to use its own 起動
///     ability in the same live (waited members cannot activate).
///   - PL!-bp4-001-R 高坂穂乃果 ライブ開始時: stage cost total < opponent's →
///     draw 1.
///   - PL!SP-bp4-009-R 鬼塚夏美 常時: same comparison as_long_as → ブレード3つ.
///   - PL!N-bp3-002-R 中須かすみ ライブ開始時: optional discard → specify a
///     heart colour → ANOTHER 虹ヶ咲 member gains it until live end.
///   - PL!HS-bp2-011-PR 村野さやか 登場: mill top 5 to the waitroom.
use crate::helpers::*;
use rabuka_engine::card::HeartColor;
use rabuka_engine::core::types::AbilityTrigger;
use rabuka_engine::game_setup::{self, ActionType};

const FILLER: &str = "PL!-sd1-010-SD";
const CLEAN_KOTORI: &str = "PL!-pb1-021-PR"; // cost 5, no abilities

fn trigger_auto(game: &mut TestGame, cid: i16, trigger: AbilityTrigger, trigger_str: &str) {
    let card = game.db.get_card(cid).unwrap();
    let ab = card
        .resolved_abilities()
        .find(|a| a.triggers.as_deref() == Some(trigger_str))
        .expect("card should have the requested trigger ability");
    let pid = game.state.player1.id.clone();
    game.state.trigger_auto_ability(
        format!("{}_{}", card.card_no, ab.full_text),
        trigger,
        pid.clone(),
        Some(card.card_no.to_string()),
        Some(cid),
        None,
        None,
    );
    game.state.activating_card = Some(cid);
    game.state.process_pending_auto_abilities(&pid);
}

fn use_ability_offers(game: &TestGame, cid: i16) -> usize {
    game_setup::generate_possible_actions(&game.state)
        .into_iter()
        .filter(|a| {
            a.action_type == ActionType::UseAbility
                && a.parameters.as_ref().and_then(|p| p.card_id) == Some(cid)
        })
        .count()
}

// ====================================================================
// ワンダーメイツ × waited 近江彼方.
// NOTE: waiting does NOT gate 起動 activation (rules 7.7.2.1 has no
// orientation precondition; Q248) — it only suppresses blades at cheer time.
// The combo value here: ワンダーメイツ re-activates a member who spent
// herself earlier in the turn.
// ====================================================================
#[test]
fn wondermates_activates_waited_nijigasaki_enabling_activation() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let kanata = game.id("PL!N-bp1-006-R＋"); // 虹ヶ咲, 起動
    let wondermates = game.id("PL!N-sd2-025-P");
    let outsider = game.id(FILLER); // μ's — not a legal target

    // Kanata waits (as if she used a self-wait effect earlier).
    game.state.player1.stage.stage = [kanata, wondermates, outsider];
    game.state.mods.add_orientation_modifier(kanata, "wait");
    game.state.mods.add_orientation_modifier(outsider, "wait");

    let filler = game.id(FILLER);
    fill_decks(&mut game, filler);
    trigger_auto(
        &mut game,
        wondermates,
        AbilityTrigger::LiveStart,
        "ライブ開始時",
    );
    while game.has_pending_choice() {
        game.select_indices(&[0]);
    }

    assert!(
        is_active(&game, kanata),
        "ワンダーメイツ activates the waited 虹ヶ咲 member"
    );
    assert!(
        is_waited(&game, outsider),
        "non-虹ヶ咲 members are not touched"
    );
    assert!(
        use_ability_offers(&game, kanata) >= 1,
        "her 起動 remains usable this turn"
    );
}

fn is_active(game: &TestGame, cid: i16) -> bool {
    rabuka_engine::ability::util::orientation_matches_state(
        game.state.mods.get_orientation_modifier(cid),
        "active",
    )
}

fn is_waited(game: &TestGame, cid: i16) -> bool {
    game.state.mods.get_orientation_modifier(cid) == Some("wait")
}

// ====================================================================
// PL!-bp4-001-R 高坂穂乃果 — stage cost total comparison at live start.
// ====================================================================
#[test]
fn honoka_bp4001_draws_only_when_stage_total_is_cheaper() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let honoka = game.id("PL!-bp4-001-R"); // cost 9
    let big = game.id("PL!S-bp5-009-R"); // 黒澤ルビィ cost 15
    let small = game.id(CLEAN_KOTORI); // cost 5

    let filler = game.id(FILLER);
    fill_decks(&mut game, filler);

    // Cheaper: my total 9 < opponent 15 → draw.
    game.state.player1.stage.stage[1] = honoka;
    game.state.player2.stage.stage[1] = big;
    let before = game.state.player1.hand.cards.len();
    trigger_auto(&mut game, honoka, AbilityTrigger::LiveStart, "ライブ開始時");
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }
    assert_eq!(
        game.state.player1.hand.cards.len(),
        before + 1,
        "stage total 9 < opponent 15 → draw 1"
    );

    // Now more expensive: my 9 > opponent 5 → no draw.
    game.state.player2.stage.stage[1] = small;
    let before2 = game.state.player1.hand.cards.len();
    trigger_auto(&mut game, honoka, AbilityTrigger::LiveStart, "ライブ開始時");
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }
    assert_eq!(
        game.state.player1.hand.cards.len(),
        before2,
        "stage total 9 > opponent 5 → no draw"
    );
}

// ====================================================================
// PL!SP-bp4-009-R 鬼塚夏美 — same comparison as a blade constant.
// ====================================================================
#[test]
fn natsumi_bp4009_blade_constant_tracks_cost_total_race() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let natsumi = game.id("PL!SP-bp4-009-R"); // cost 9
    let big = game.id("PL!S-bp5-009-R"); // cost 15
    let small = game.id(CLEAN_KOTORI); // cost 5

    game.state.player1.stage.stage[1] = natsumi;
    game.state.player2.stage.stage[1] = big;
    game.state.recalculate_constants();
    assert_eq!(
        game.state.mods.get_blade_modifier(natsumi),
        3,
        "my total 9 < opponent 15 → ブレード3つ"
    );

    game.state.player2.stage.stage[1] = small;
    game.state.recalculate_constants();
    assert_eq!(
        game.state.mods.get_blade_modifier(natsumi),
        0,
        "my total 9 > opponent 5 → blades gone"
    );
}

// ====================================================================
// PL!N-bp3-002-R 中須かすみ — optional discard, specify colour, grant to
// ANOTHER 虹ヶ咲 member until live end (exclude_self enforced).
// ====================================================================
#[test]
fn kasumu_bp3002_grants_chosen_heart_to_other_nijigasaki_only() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let kasumu = game.id("PL!N-bp3-002-R"); // 中須かすみ QU4RTZ (虹ヶ咲)
    let other = game.id("PL!N-bp1-006-R＋"); // 近江彼方 (虹ヶ咲)
    let outsider = game.id("PL!S-bp5-009-R"); // Aqours — never targeted

    game.state.player1.stage.stage = [other, kasumu, outsider];
    game.add_to_hand(game.id(FILLER));

    trigger_auto(&mut game, kasumu, AbilityTrigger::LiveStart, "ライブ開始時");

    // Optional cost 「手札を1枚控え室に置いてもよい」 is presented directly as
    // a skippable hand SelectCard — pick the single card to pay.
    assert!(
        game.has_pending_choice(),
        "cost prompt appears"
    );
    game.select_indices(&[0]);

    // Specify heart colour.
    assert!(
        game.has_pending_choice(),
        "heart colour specification must be asked"
    );
    game.select_option(4); // some colour; identity checked below

    // Exactly one colour was granted — to the OTHER 虹ヶ咲 member only.
    let colours = [
        HeartColor::Heart01,
        HeartColor::Heart02,
        HeartColor::Heart03,
        HeartColor::Heart04,
        HeartColor::Heart05,
        HeartColor::Heart06,
    ];
    let granted_other: Vec<_> = colours
        .iter()
        .filter(|&&c| game.state.mods.get_heart_modifier(other, c) > 0)
        .collect();
    assert_eq!(
        granted_other.len(),
        1,
        "exactly one specified colour granted to the other 虹ヶ咲 member"
    );
    for c in colours {
        assert_eq!(
            game.state.mods.get_heart_modifier(kasumu, c),
            0,
            "exclude_self: かすみ herself gains nothing"
        );
        assert_eq!(
            game.state.mods.get_heart_modifier(outsider, c),
            0,
            "non-虹ヶ咲 members gain nothing"
        );
    }
}

// ====================================================================
// PL!HS-bp2-011-PR 村野さやか — 登場 デッキの上から5枚控え室に置く。
// ====================================================================
#[test]
fn sayaka_hsbp2011_mills_top_five() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let sayaka = game.id("PL!HS-bp2-011-PR");

    game.state.player1.stage.stage[1] = sayaka;
    let filler = game.id(FILLER);
    fill_decks(&mut game, filler);
    let deck_before = game.state.player1.main_deck.cards.len();
    let waitroom_before = game.state.player1.waitroom.cards.len();

    trigger_auto(&mut game, sayaka, AbilityTrigger::Debut, "登場");

    assert_eq!(
        game.state.player1.waitroom.cards.len(),
        waitroom_before + 5,
        "five cards milled to the waitroom"
    );
    assert_eq!(
        game.state.player1.main_deck.cards.len(),
        deck_before - 5,
        "deck shrank by five from the top"
    );
}
