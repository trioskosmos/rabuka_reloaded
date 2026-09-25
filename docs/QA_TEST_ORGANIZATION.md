# QA Test Organization

Test names prioritize the behavior being proven over card IDs. Card numbers belong in assertions or file-level documentation when they identify the fixture, not in the filename.

## Naming

Use `q<NNN>_<behavior>_<kind>_test.rs` for a QA-specific regression test.

Examples:

- `q180_active_phase_still_activates_test.rs`
- `q126_stage_to_waitroom_does_not_trigger_test.rs`
- `q134_baton_touch_waited_occupant_test.rs`

For behavior families shared by multiple QA entries, use the family name first and include the QA ID in the test function:

- `card_filter_ability_or_no_ability_test.rs`
- `live_start_draw_timing_test.rs`

Recommended test-function suffixes:

- `_positive`
- `_negative`
- `_boundary`
- `_ordering`
- `_refresh`
- `_same_instance`
- `_no_replay`

## QA index

| QA | Current coverage | Remaining edge focus |
|---|---|---|
| Q180 | `nico_cannot_activate_test.rs` | restriction expiry and unrelated baton paths |
| Q165 | `ll_joint_test.rs` | mixed-name combinations and duplicate instances |
| Q163 | `activate_wait_other_group_draw_q163_test.rs` | empty own stage and multiple opponent targets |
| Q160 | `miyashita_ai_test.rs` | member leaves before/after the triggering debut |
| Q141 | `energy_and_member_under_test.rs` | under-energy when owner leaves by effect |
| Q140 | `energy_and_member_under_test.rs` | stage-to-hand and stage-to-discard parity |
| Q134 | `baton_touch_test.rs` | already-deployed occupant restriction |
| Q126 | `chisato_move_test.rs` | stage-to-waitroom and effect-caused movement |
| Q122 | `debut_top3_reorder_and_discard_activation_test.rs` | refresh timing versus same-turn discard |
| Q120 | `revealed_live_hand_le7_draw_q120_test.rs` | exact draw delta and duplicate live identity |
| Q111 | `wien_yell_count_test.rs` | blade changes after the reduction snapshot |
| Q116 | `dream_with_you_test.rs` | reduced yell count below the blade threshold |
| Q101 | `special_blade_heart_rules_test.rs` | refresh and exhausted-source termination |
| Q95 | `fuyumari_test.rs` | same-name decoy versus exact card instance |
| Q90 | `ll_bp1_001_test.rs` | joint-card name in live-card selection |
| Q88 | `action_coverage_test.rs` | illegal free actions outside abilities |
| Q89 | `multiname_card_test.rs` | group metadata and unit-field invariants |

## Maintenance rule

When a QA test is added or moved, update this table in the same change. Keep fixture card IDs in the test body. A renamed file must continue to expose its QA ID through at least one test function or file-level QA marker.
