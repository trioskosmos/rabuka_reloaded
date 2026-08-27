//! B3 zone vocabulary conversion layer — the four vocabularies collapse into one conversion.

use rabuka_engine::ability::enums::Zone as AbilityZone;
use rabuka_engine::core::types::ZoneId;

#[test]
fn ability_zone_round_trips_through_zone_id() {
    for az in [AbilityZone::Stage, AbilityZone::Hand, AbilityZone::Deck, AbilityZone::Discard, AbilityZone::Energy, AbilityZone::LiveCardZone, AbilityZone::SuccessLiveZone] {
        let zid = ZoneId::from_ability_zone(az);
        let back = zid.to_ability_zone().expect("round-trip should exist");
        // Discard/Waitroom and Energy/EnergyZone alias — both map to same ability zone
        assert!(back == az || matches!((az, back), (AbilityZone::Discard, AbilityZone::Discard) | (AbilityZone::Energy, AbilityZone::Energy)));
    }
}

#[test]
fn alias_drift_dies_at_boundary() {
    // "energy" vs "energy_zone" both normalize to same ZoneId::Energy via typed wrapper
    assert_eq!(ZoneId::from_str("energy"), ZoneId::from_str("energy_zone"));
    // "discard" vs "waitroom" are distinct variants but `equivalent` treats them as same — typed wrapper normalizes via equivalent/matches_source
    assert!(ZoneId::from_str("discard").equivalent(&ZoneId::from_str("waitroom")));
    assert!(ZoneId::from_str("waitroom").equivalent(&ZoneId::from_str("discard")));
    // Unknown stays Unknown — caller gets loud log via push_movement_event drift detection
    assert_eq!(ZoneId::from_str("energy|energy_zone"), ZoneId::Unknown);
    assert_eq!(ZoneId::from_str(""), ZoneId::Unknown);
}
