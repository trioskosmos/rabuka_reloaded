//! Catalogue §4.11 gate: the trigger masks cached on `Ability`
//! must agree with a fresh parse of the `triggers` text for
//! every ability in the bytecode. `has_trigger` and the
//! action-generation position gate answer from these caches,
//! so a disagreement is a gameplay-visible wrong verdict.

#[cfg(feature = "bytecode_abilities")]
mod trigger_mask_corpus {
    use rabuka_engine::ability::vm::{ability_count, get_ability};
    use rabuka_engine::card::Ability;
    use rabuka_engine::core::zones::{
        check_trigger_position, MemberArea, TriggerPositionMask,
    };
    use rabuka_engine::triggers::{parse_triggers, TriggerKind, TriggerMask};

    const ALL_KINDS: [TriggerKind; 8] = [
        TriggerKind::Activation,
        TriggerKind::Auto,
        TriggerKind::Constant,
        TriggerKind::Debut,
        TriggerKind::LiveStart,
        TriggerKind::LiveSuccess,
        TriggerKind::Main,
        TriggerKind::BatonTouch,
    ];

    fn expected_trigger_mask(ability: &Ability) -> TriggerMask {
        let mut mask = TriggerMask::EMPTY;
        if let Some(triggers) = ability.triggers.as_deref() {
            for kind in parse_triggers(triggers) {
                mask = mask.union(TriggerMask::of(kind));
            }
        }
        mask
    }

    #[test]
    fn trigger_mask_agrees_with_parse_for_every_ability() {
        let total = ability_count();
        assert!(total > 0, "bytecode ability store is empty");
        for index in 0..total {
            let Some(ability) = get_ability(index).ok() else {
                continue;
            };
            let expected = expected_trigger_mask(&ability);
            assert_eq!(
                ability.trigger_mask(),
                expected,
                "ability #{index}: cached trigger mask disagrees with parse_triggers"
            );
            for kind in ALL_KINDS {
                assert_eq!(
                    ability.has_trigger(kind),
                    expected.contains(kind),
                    "ability #{index} {kind:?}: has_trigger disagrees with the parsed mask"
                );
            }
        }
    }

    #[test]
    fn trigger_position_mask_agrees_with_keyword_scan() {
        let total = ability_count();
        assert!(total > 0, "bytecode ability store is empty");
        for index in 0..total {
            let Some(ability) = get_ability(index).ok() else {
                continue;
            };
            let triggers = ability.triggers.as_deref();
            let expected = TriggerPositionMask::parse(triggers.unwrap_or(""));
            assert_eq!(
                ability.trigger_position_mask(),
                expected,
                "ability #{index}: cached position mask disagrees with the keyword scan"
            );
            for area in MemberArea::ALL {
                assert_eq!(
                    ability.trigger_position_allows(area),
                    check_trigger_position(triggers, area),
                    "ability #{index} {area:?}: position gate disagrees with check_trigger_position"
                );
            }
        }
    }
}
