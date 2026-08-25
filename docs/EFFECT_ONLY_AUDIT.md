# effect_only flag audit — push_movement_event call sites

**Status: COMPLETE (2026-08-25).** All 19 real call sites classified;
one inconsistency found and fixed (cost.rs optional-cost drain was `true`,
flipped to `false` to match choice.rs:544 and the rules-corpus convention:
**cost payments are player actions, not card effects**).

## Convention

- `true`  => event caused by CARD EFFECT execution (arms 「カードの効果によって」 triggers).
- `false` => event caused by cost payment, rule step, or phase action.

## Final classifications

| Site | Effect | Classification |
|---|---|---|
| ability/choice.rs:544 | optional-cost hand discard | false ✓ (canonical R1 comment lives here) |
| ability/choice.rs:871 | under_member placement from choice | true ✓ |
| ability/cost.rs:1343 | optional-cost ACCEPT full-hand drain | **false — FLIPPED from true** |
| effects/misc.rs:2924 | position change swap legs | true ✓ |
| effects/misc.rs:3149 | single position change | true ✓ |
| effects/misc.rs:3311/3320 | swap pair pushes | true ✓ |
| effects/misc.rs:3402/3411 | swap pair pushes | true ✓ |
| effects/misc.rs:3493/3502 | activating-card reposition + target | true ✓ |
| effects/misc.rs:3572 | formation plan loop | true ✓ |
| effects/state.rs:709 | energy_deck -> zone placement | true ✓ |
| move_cards.rs:56 | under_member -> energy_zone | true ✓ |
| move_cards.rs:2579 | generic move_cards effect dispatch | true ✓ |
| move_cards.rs:3169 | look-and-select finalize moves | true ✓ |
| move_cards.rs:3682 | energy_zone -> under_member | true ✓ |
| turn/actions.rs:1428 | live-resolution zone moves | false ✓ (rule step) |
| turn/phases.rs:1045 | double-baton replaced member | false ✓ (rule step) |
| turn/phases.rs:1133 | baton-touch replaced member | false ✓ (rule step) |
| turn/phases.rs:1529 | mulligan-style hand -> waitroom | false ✓ (rule step) |

Non-call-site grep hits excluded: game_state/mod.rs:166 (field doc),
game_state/modifiers.rs:1191 (fn definition).

## Residual notes

- The `true` population is homogeneous (all inside resolver effect
  execution), which makes a future R1 consolidation straightforward:
  effect-executed pushes can derive the flag from the execution context
  instead of receiving it as a parameter.
- cost.rs:1343 flip verified against full suite (2912/0): no test pinned
  the old value; the HS-pb1-003-R each_time watcher keys off
  preceding_moved membership, not this bit.
