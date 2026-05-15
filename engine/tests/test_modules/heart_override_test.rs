use crate::helpers::*;
use rabuka_engine::card::HeartColor;

/// Kanan (PL!S-pb1-003-R) has base hearts: heart02:1, heart04:4, heart05:1 = 6 total.
/// Stage has only Kanan. No other cards contribute hearts.
/// Before: {♥02:1, ♥04:4, ♥05:1}. After: all converted → {♥04:6}.
#[test]
fn kanan_heart_override_exact_color_conversion() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());
    let kanan = game.id("PL!S-pb1-003-R");

    // Only Kanan on stage, no other member contributes hearts
    game.state.player1.stage.stage = [-1, kanan, -1];

    // ── Before override — verify exact heart distribution ──
    let before = game.state.player1.calculate_stage_hearts(
        &game.state.card_database,
        &game.state.mods.heart_color_multiplier,
    );
    assert_eq!(
        before.hearts.get(&HeartColor::Heart02),
        Some(&1),
        "Kanan has 1 heart02 before override"
    );
    assert_eq!(
        before.hearts.get(&HeartColor::Heart04),
        Some(&4),
        "Kanan has 4 heart04 before override"
    );
    assert_eq!(
        before.hearts.get(&HeartColor::Heart05),
        Some(&1),
        "Kanan has 1 heart05 before override"
    );
    assert_eq!(before.hearts.values().sum::<u32>(), 6);

    // ── Inject override: ALL hearts become heart04 ──
    game.state
        .mods
        .heart_color_multiplier
        .insert(kanan, HeartColor::Heart04);

    // ── After override — verify exact conversion ──
    let after = game.state.player1.calculate_stage_hearts(
        &game.state.card_database,
        &game.state.mods.heart_color_multiplier,
    );
    assert_eq!(
        after.hearts.get(&HeartColor::Heart02),
        None,
        "heart02 should be 0 after override"
    );
    assert_eq!(
        after.hearts.get(&HeartColor::Heart05),
        None,
        "heart05 should be 0 after override"
    );
    assert_eq!(
        after.hearts.get(&HeartColor::Heart04),
        Some(&6),
        "ALL 6 hearts count as heart04 after override"
    );
    assert_eq!(
        after.hearts.len(),
        1,
        "Only 1 color (heart04) remains after override"
    );
    assert_eq!(
        after.hearts.values().sum::<u32>(),
        6,
        "Total heart count unchanged at 6"
    );

    // ── Clear override — verify original distribution restored ──
    game.state.mods.heart_color_multiplier.clear();
    let restored = game.state.player1.calculate_stage_hearts(
        &game.state.card_database,
        &game.state.mods.heart_color_multiplier,
    );
    assert_eq!(
        restored.hearts.get(&HeartColor::Heart02),
        Some(&1),
        "heart02 restored to 1 after clear"
    );
    assert_eq!(
        restored.hearts.get(&HeartColor::Heart04),
        Some(&4),
        "heart04 restored to 4 after clear"
    );
    assert_eq!(
        restored.hearts.get(&HeartColor::Heart05),
        Some(&1),
        "heart05 restored to 1 after clear"
    );
    assert_eq!(restored.hearts.len(), 3, "All 3 original colors restored");
    assert_eq!(restored.hearts.values().sum::<u32>(), 6);
}
