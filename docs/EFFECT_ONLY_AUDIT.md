# effect_only flag audit — push_movement_event call sites

Generated 2026-08-25 (grep-level scan; each site still needs semantic review).
`effect_only=true`  => event caused by a CARD EFFECT (arms 「カードの効果によって」 triggers).
`effect_only=false` => event caused by cost payment / rule step / phase action.

Rule of thumb from the rules corpus: costs are NOT card effects for
「カードの効果によって」 purposes; anything inside resolver effect execution IS.

| Site | File | Current flag | Notes |
|---|---|---|---|| ability\choice.rs:544 | ability\choice.rs | variable — REVIEW | || ability\choice.rs:871 | ability\choice.rs | variable — REVIEW | || ability\cost.rs:1343 | ability\cost.rs | true | || ability\move_cards.rs:56 | ability\move_cards.rs | true | || ability\move_cards.rs:2579 | ability\move_cards.rs | variable — REVIEW | || ability\move_cards.rs:3169 | ability\move_cards.rs | true | || ability\move_cards.rs:3682 | ability\move_cards.rs | variable — REVIEW | || ability\effects\misc.rs:2924 | ability\effects\misc.rs | true | || ability\effects\misc.rs:3149 | ability\effects\misc.rs | true | || ability\effects\misc.rs:3311 | ability\effects\misc.rs | variable — REVIEW | || ability\effects\misc.rs:3320 | ability\effects\misc.rs | variable — REVIEW | || ability\effects\misc.rs:3402 | ability\effects\misc.rs | variable — REVIEW | || ability\effects\misc.rs:3411 | ability\effects\misc.rs | variable — REVIEW | || ability\effects\misc.rs:3493 | ability\effects\misc.rs | variable — REVIEW | || ability\effects\misc.rs:3502 | ability\effects\misc.rs | variable — REVIEW | || ability\effects\misc.rs:3572 | ability\effects\misc.rs | true | || ability\effects\state.rs:709 | ability\effects\state.rs | true | || core\game_state\mod.rs:166 | core\game_state\mod.rs | variable — REVIEW | || turn\actions.rs:1428 | turn\actions.rs | variable — REVIEW | || turn\phases.rs:1045 | turn\phases.rs | variable — REVIEW | || turn\phases.rs:1133 | turn\phases.rs | variable — REVIEW | || turn\phases.rs:1529 | turn\phases.rs | false | |
