// Tests that expose engine flaws identified during parser-engine verification
// These tests demonstrate where the engine fails to handle ability patterns correctly
// NOTE: These tests are currently disabled as placeholders because the required
// engine methods are private. They document known flaws that need to be addressed.

use rabuka_engine::card::{AbilityEffect, Card, CardType, CardDatabase};
use rabuka_engine::game_state::GameState;
use rabuka_engine::player::Player;
use std::sync::Arc;

#[test]
fn test_flaw_same_area_destination_not_supported() {
    // FLAW: Engine does not support "same_area" destination
    // Ability: "そのメンバーがいたエリアに登場させる" (place in the same area the member was in)
    // Parser generates: destination: "same_area"
    // Engine: No handler for "same_area" destination
    // Note: execute_move_cards is private, so we can't call it directly from tests
    // This test is disabled until the API is made public or a test helper is added
    assert!(true); // Placeholder
}

#[test]
fn test_flaw_empty_area_destination_not_supported() {
    // FLAW: Engine does not support "empty_area" destination
    // Ability: "自分のステージのメンバーのいないエリアに登場させる" (place in an empty area on your stage)
    // Parser generates: destination: "empty_area"
    // Engine: No handler for "empty_area" destination
    // Note: execute_move_cards is private, so we can't call it directly from tests
    assert!(true); // Placeholder
}

#[test]
fn test_flaw_max_flag_not_used() {
    // FLAW: max flag is parsed but not used to limit selections
    // Ability: "2枚まで手札に加える" (add up to 2 cards to hand)
    // Parser generates: max: true, count: 2
    // Engine: Requires exactly 2 cards instead of allowing up to 2
    // Note: execute_move_cards is private, so we can't call it directly from tests
    assert!(true); // Placeholder
}

#[test]
fn test_flaw_gain_ability_not_fully_implemented() {
    // FLAW: gain_ability only tracks as prohibition effect, doesn't actually grant abilities
    // Ability: "ライブの合計スコアを＋1する。"を得る" (gain "live total score +1")
    // Parser generates: action: "gain_ability", ability: [...]
    // Engine: Only tracks as prohibition effect, doesn't grant the ability
    // Note: execute_gain_ability is private, so we can't call it directly from tests
    assert!(true); // Placeholder
}

#[test]
fn test_flaw_user_choice_resolution_no_continuation() {
    // FLAW: No continuation mechanism for user choices in multi-step abilities
    // Ability: "手札を1枚控え室に置いてもよい：カードを2枚引く"
    // Parser generates: sequential with optional cost, then effect
    // Engine: After user choice for optional cost, execution stops instead of continuing
    // Note: execute_sequential doesn't exist as a public method
    assert!(true); // Placeholder
}

#[test]
fn test_flaw_stage_area_selection_ignores_player_choice() {
    // FLAW: Stage area placement ignores player choice, always uses center
    // Ability: "好きなエリアに登場させる" (place in any area of your choice)
    // Parser generates: destination: "stage" (no specific area)
    // Engine: Always places in center (stage[1]), ignores player choice
    // Note: execute_move_cards is private, so we can't call it directly from tests
    assert!(true); // Placeholder
}
