use crate::helpers::*;
use rabuka_engine::zones::MemberArea;

/// Test to verify filtering logic is working correctly
#[test]
fn group_member_filter_excludes_activating_member() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let yoshiko = game.id("PL!S-bp3-006-R＋");
    let chika = game.id("PL!S-bp2-001-R");
    let riko = game.id("PL!S-bp2-002-R");

    // Place them on stage
    game.add_to_stage(MemberArea::Center, yoshiko);
    game.add_to_stage(MemberArea::LeftSide, chika);
    game.add_to_stage(MemberArea::RightSide, riko);

    let stage_data = game.player().stage.stage;
    let card_db = game.state.card_database.clone();

    assert!(
        game.state.card_database.get_card(chika).is_some(),
        "Chika card should exist in DB"
    );
    assert!(
        game.state.card_database.get_card(riko).is_some(),
        "Riko card should exist in DB"
    );

    // Test the filter that should be used by the ability
    let filter = rabuka_engine::ability::util::filter_from_parts_full(
        Some("member_card"),
        Some("Aqours"),
        None,
        None,
        None,
        None,
        None,
        Some(yoshiko), // exclude_self
        None,
        None,
        None, // exclude_characters
    );

    // Get matching indices
    let matching_indices =
        rabuka_engine::ability::util::matching_indices(&stage_data, &card_db, &filter, true);

    // Stage: [Chika (Aqours), Yoshiko (Aqours, excluded), Riko (Aqours)]
    // Filter: member_card, Aqours, exclude_self=yoshiko
    // Expected matches: Chika (index 0) and Riko (index 2), NOT Yoshiko (index 1)
    assert_eq!(
        matching_indices,
        vec![0usize, 2],
        "filter should match Chika (idx=0) and Riko (idx=2), excluding Yoshiko (idx=1)"
    );
    assert_eq!(
        stage_data[matching_indices[0]], chika,
        "first match should be Chika"
    );
    assert_eq!(
        stage_data[matching_indices[1]], riko,
        "second match should be Riko"
    );
}
