use crate::helpers::*;
use rabuka_engine::game_state::AbilityTrigger;

const MITSUKI_AUTO: &str = "{{jidou.png|自動}}{{turn1.png|ターン1回}}自分がエールしたとき、エールにより公開された自分のブレードハートを持たない『蓮ノ空』のカードを3枚まで控え室に置いてもよい。そうした場合、これにより控え室に置いた数に等しい枚数のエールを追加で行う。";
const HASUMARU_MEMBER: &str = "PL!HS-bp1-001-R"; // 日野下花帆 (蓮ノ空, has blade heart)
const HASUMARU_MEMBER_NO_BH: &str = "PL!HS-bp1-010-N"; // 日野下花帆 (蓮ノ空, N rarity, no blade heart)
const HASUMARU_LIVE: &str = "PL!HS-bp6-027-L"; // 月夜見海月 (蓮ノ空, live card, has blade_heart + special_heart)
const HASUMARU_LIVE_NO_BH: &str = "PL!HS-bp1-019-L"; // 蓮ノ空 live, special_heart(score), NO blade_heart
const NON_HASUMARU_MEMBER: &str = "PL!-sd1-010-SD"; // 高坂穂乃果 (μ's, not 蓮ノ空)

fn trigger_mitsuki_auto(game: &mut TestGame) {
    let ability_id = format!("PL!HS-bp6-027-L_{}", MITSUKI_AUTO);
    game.state.trigger_auto_ability(
        ability_id,
        AbilityTrigger::Auto,
        "player1".to_string(),
        Some("PL!HS-bp6-027-L".to_string()),
        None,
        None,
        None,
    );
    game.state.process_pending_auto_abilities("player1");
}

fn setup_with_revealed(game: &mut TestGame, revealed: &[i16], mitsuki_id: i16) {
    for &cid in revealed {
        game.state.revealed_cards.push(cid);
    }
    game.state.player1.live_card_zone.cards.push(mitsuki_id);
    game.state.yell_occurred = true;
    let filler = game.new_id("PL!-sd1-010-SD");
    for _ in 0..20 {
        game.state.player1.main_deck.cards.push(filler);
    }
    game.state.current_phase = rabuka_engine::game_state::Phase::Main;
}

/// 蓮ノ空 member card without blade heart → can be placed in waitroom.
#[test]
fn q251_member_no_blade_can_move() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let member = game.new_id(HASUMARU_MEMBER_NO_BH);
    let mitsuki = game.new_id(HASUMARU_LIVE);
    setup_with_revealed(&mut game, &[member], mitsuki);
    trigger_mitsuki_auto(&mut game);

    assert!(
        game.has_pending_choice(),
        "Choice shown — up to N includes declining"
    );
    game.select_indices(&[0]);
    game.state.process_pending_auto_abilities("player1");
    assert!(
        game.state.player1.waitroom.cards.contains(&member),
        "蓮ノ空 member without blade heart moved to waitroom"
    );
}

/// 蓮ノ空 live card (no blade_heart but special_heart = [Score]) → CANNOT be placed.
/// has_blade_heart() now returns true for cards with special_heart, so the
/// "NOT has_blade_heart" filter excludes it (Q251 ruling: [Score] cards excluded).
#[test]
fn q251_live_special_heart_cannot_move() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let live = game.new_id(HASUMARU_LIVE_NO_BH);

    let cdb = game.db.clone();
    let card = cdb.get_card(live).unwrap();
    assert!(
        card.special_heart
            .as_ref()
            .is_some_and(|sh| !sh.hearts.is_empty()),
        "Test card must have special_heart"
    );
    assert!(
        card.blade_heart.is_none(),
        "Test card must have NO blade_heart"
    );

    let mitsuki = game.new_id(HASUMARU_LIVE);
    setup_with_revealed(&mut game, &[live], mitsuki);
    trigger_mitsuki_auto(&mut game);

    assert!(
        !game.has_pending_choice(),
        "No choice — live card with special_heart is excluded by has_blade_heart filter"
    );
    assert!(
        game.state.revealed_cards.contains(&live),
        "Live card remains in revealed_cards (not moved to waitroom)"
    );
    assert!(
        !game.state.player1.waitroom.cards.contains(&live),
        "Live card NOT in waitroom"
    );
}

/// 蓮ノ空 member WITH blade heart → filtered out (negation on has_blade_heart).
#[test]
fn q251_has_blade_heart_filtered() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let member_bh = game.new_id(HASUMARU_MEMBER);

    let cdb = game.db.clone();
    let card = cdb.get_card(member_bh).unwrap();
    assert!(card.blade_heart.is_some());

    let mitsuki = game.new_id(HASUMARU_LIVE);
    setup_with_revealed(&mut game, &[member_bh], mitsuki);
    trigger_mitsuki_auto(&mut game);

    assert!(
        !game.has_pending_choice(),
        "No choice — card with blade heart is excluded by negation"
    );
}

/// Non-蓮ノ空 card → filtered out by group_names.
#[test]
fn q251_wrong_group_filtered() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let non_hasu = game.new_id(NON_HASUMARU_MEMBER);
    let mitsuki = game.new_id(HASUMARU_LIVE);
    setup_with_revealed(&mut game, &[non_hasu], mitsuki);
    trigger_mitsuki_auto(&mut game);

    assert!(
        !game.has_pending_choice(),
        "No choice — non-蓮ノ空 card excluded by group filter"
    );
}

/// Mixed pool: 蓮ノ空 member (no blade) can move, 蓮ノ空 live ([Score]) cannot.
/// 2 cards in revealed, only 1 matches → auto-takes the matching one.
#[test]
fn q251_mixed_pool_only_member_can_move() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let member = game.new_id(HASUMARU_MEMBER_NO_BH);
    let live = game.new_id(HASUMARU_LIVE_NO_BH);
    let mitsuki = game.new_id(HASUMARU_LIVE);

    setup_with_revealed(&mut game, &[member, live], mitsuki);
    trigger_mitsuki_auto(&mut game);

    assert!(
        game.has_pending_choice(),
        "Choice shown — up to N includes declining"
    );
    game.select_indices(&[0]);
    game.state.process_pending_auto_abilities("player1");
    assert!(
        game.state.player1.waitroom.cards.contains(&member),
        "蓮ノ空 member moved to waitroom"
    );
    assert!(
        !game.state.revealed_cards.contains(&member),
        "Member removed from revealed"
    );
    assert!(
        game.state.revealed_cards.contains(&live),
        "Live card stays in revealed (excluded by has_blade_heart)"
    );
}

/// Skip/optional: player may decline to move cards.
/// With 4+ matching cards (more than max take_count=3), a prompt appears
/// and the user can select 0-3 cards or skip entirely.
#[test]
fn q251_skip_optional() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let m1 = game.new_id(HASUMARU_MEMBER_NO_BH);
    let m2 = game.new_id(HASUMARU_MEMBER_NO_BH);
    let m3 = game.new_id(HASUMARU_MEMBER_NO_BH);
    let m4 = game.new_id(HASUMARU_MEMBER_NO_BH);
    let mitsuki = game.new_id(HASUMARU_LIVE);
    setup_with_revealed(&mut game, &[m1, m2, m3, m4], mitsuki);
    trigger_mitsuki_auto(&mut game);

    assert!(game.has_pending_choice(), "Prompt: select up to 3 or skip");
    game.select_indices(&[]); // skip

    // After skipping, drain any remaining auto-resolve choices (e.g. second action)
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }
    assert!(
        game.state.revealed_cards.contains(&m1),
        "Card stays in revealed_cards after skip"
    );
    assert!(
        !game.state.player1.waitroom.cards.contains(&m1),
        "Card NOT moved to waitroom"
    );
}
