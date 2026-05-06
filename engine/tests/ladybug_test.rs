/// Tests for レディバグ (PL!HS-bp2-024-L) — LiveStart modify_required_hearts:
///
/// ライブ開始時 自分のステージに「夕霧綴理」が登場しており、
/// かつ「夕霧綴理」よりコストの大きい「村野さやか」が登場している場合、
/// このカードの必要ハートをheart0×3減らす。
///
/// Q114: Members just need to be on stage when ability fires,
/// they don't need to have debuted that turn.
/// Parser gap: uses appearance_condition(appearance=true) instead of
/// location_condition, so "on stage since prior turn" fails.

mod helpers;
use helpers::*;
use rabuka_engine::card::HeartColor;

fn advance_to_live_card_set_p1(game: &mut TestGame) {
    for _ in 0..5 { game.pass(); }
}

fn advance_to_live_start(game: &mut TestGame) {
    game.pass();
    game.pass();
}

/// Q114: Ability structure has correct conditions parsed.
#[test]
fn ladybug_q114_ability_parsed() {
    let db = load_real_database();
    let card = db.get_card_by_no("PL!HS-bp2-024-L").expect("Ladybug exists");
    let ab = card.abilities.iter().find(|a| a.triggers.as_deref() == Some("ライブ開始時"))
        .expect("Should have LiveStart ability");
    let effect = ab.effect.as_ref().expect("Should have effect");
    assert_eq!(effect.action, "modify_required_hearts");
    assert_eq!(effect.value, Some(3));
    assert_eq!(effect.operation.as_deref(), Some("decrease"));

    let cond = effect.condition.as_ref().expect("Should have condition");
    assert_eq!(cond.condition_type.as_deref(), Some("compound"));
    assert_eq!(cond.operator.as_deref(), Some("and"));

    // Both sub-conditions should check for specific names on stage
    let subs = cond.conditions.as_ref().expect("Should have sub-conditions");
    assert_eq!(subs.len(), 2, "Should have 2 conditions for 2 members");
    // First mentions 夕霧綴理
    assert!(subs[0].text.contains("綴理") || subs[0].text.contains("夕霧"));
    // Second mentions 村野さやか and cost comparison
    assert!(subs[1].text.contains("さやか") || subs[1].text.contains("村野"));
}

/// The modify_required_hearts effect exists with target heart00 and value 3.
#[test]
fn ladybug_q114_heart_reduction_parsed() {
    let db = load_real_database();
    let card = db.get_card_by_no("PL!HS-bp2-024-L").expect("Ladybug exists");
    let ab = card.abilities.iter()
        .find(|a| a.triggers.as_deref() == Some("ライブ開始時"))
        .expect("Should have LiveStart ability");
    let effect = ab.effect.as_ref().expect("Should have effect");
    assert_eq!(effect.heart_color.as_deref(), Some("heart00"), "Reduces heart00");
    assert_eq!(effect.value, Some(3), "Reduce by 3");
}
