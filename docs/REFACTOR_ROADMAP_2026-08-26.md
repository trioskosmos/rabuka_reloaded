# Refactor & Test Roadmap — 2026-08-26 (deep audit)

Derived from full reads of rules.txt structure, cards/abilities.json marker
scans, engine duplication mapping, and parser-pipeline architecture audit.
Supersedes the stale entries in RULES_GAP_ANALYSIS.md where noted.

## A. Correctness risks (do first — these are bug factories)

### A1. Three divergent stage-heart pipelines ★ highest priority
| Impl | Order applied |
|---|---|
| `zones.rs::get_available_hearts` | override → copy → base → multiplier → additive mods |
| `player.rs::calculate_stage_hearts` | same shape, but override path **duplicated internally** (already drifted once — the 9.9 bug we fixed lived here) |
| `turn/live.rs` member-contribution loop (~1576–1685) | copy → blades → mods → multiplier → override (**different order**) |

Two of the three run during a single live resolution (`live.rs:392/399` call
B; `live.rs:1743/2602` call A). The heart_override-swallowed-additives bug we
fixed existed *because* of this triplication; the ordering divergence is a
live correctness risk for any card combining copy+multiplier+override.

**Rewrite:** single `HeartPipeline::compute(stage, mods…) -> BaseHeart` (+ a
per-member detail view feeding `MemberContribution`). Pin with a
characterization test FIRST: same board through all three entry points must
produce identical multisets (today it may not — that diff IS the test).

### A2. Blade set-rule re-implemented 4×
Canonical `zones.rs::total_blades`; inline copies at `live.rs:1624`,
`condition/card.rs:3044`, `condition/card.rs:2789`. Extract
`effective_blade(card_id) -> u8`. Same characterization-test-first approach.

### A3. `resolve_target_player` mut/non-mut divergence
`abilities.rs:2308` (_mut) vs `:2346` — non-mut falls back through
activating_card/owner_of_card, mut does not; both silently default unknowns
(incl. `"both"`) to player1. 100+ call sites. Make the two arms identical,
return Result or an explicit Owner enum, log-and-default only at the UI edge.

### A4. need-heart satisfaction computed by hand twice inside live.rs
(~218–255 allocation scoring, ~613–700 pass/fail) instead of calling
`card.rs::check_heart_requirement`. Fold into A1's rewrite.

## B. Engine refactors (mechanical, medium value)

- **B1. Split `execute_live_victory_determination` (882 lines)** — contains
  its own heart math + need-heart scorer + trigger ordering; splitting forces A1/A4.
- **B2. Movement-event recording choke point** — 25+ raw-string
  `push_movement_event` callsites (misc.rs, move_cards.rs, phases.rs,
  actions.rs, choice.rs, cost.rs); make it take `(Zone, Zone)` typed args so
  `"energy"|"energy_zone"`-style aliases die at the boundary. Persisted
  strings are pattern-matched later — alias drift = silently dead watchers.
- **B3. Collapse four parallel zone vocabularies** (`ability/enums::Zone`,
  `core/types::ZoneId`, `bot/encoding::ZoneId`, `core/card::Location`) into
  one conversion layer.
- **B4. Typed errors**: 104 `Result<_, String>` in ability/ — callers match on
  English prose ("Cannot baton touch - no member…"). Error enum.
- **B5. Candidate-pool builders ×5** (`game_setup.rs:579/:1353`,
  `choice.rs:402`, `look.rs:356/607`, `state.rs:20`) share filter logic that
  will drift — extract shared pool builder.
- **B6. Dead code decisions**: `ExclusionZone` fully modeled but ZERO cards
  use 除外/バインド (keep — future sets); `repeat_prompt_choice()` doc claims
  to be the single construction point yet is dead — verify & delete;
  consolidate bot strategy v2→v5 layer cake; add ONE no_std target to CI so
  gated stubs can't rot.

## C. Parser/pipeline hardening (each ships with its own guard)

1. **CI hole: `effect_decoder_gen.rs` has NO freshness check** — coverage.yml
   diffs only the condition decoder. A Rust type missing from the effect
   READER_MAP is *silently skipped* at decode. Add the second diff + wire
   `validate_schema.py` into CI or delete it (currently implies safety,
   delivers none, checked nowhere).
2. **Golden snapshot for abilities.json** — today the only shape guards are
   validation-count baselines; a walker reshuffle ships unnoticed until a
   gameplay test trips. Commit a golden (diff ignoring generated_at) or
   hash-pin; regenerate deliberately.
3. **Version the bytecode blob** — abilities.bin.z has no magic/header; a
   stale blob paired with fresh code fails far from the cause. Prepend
   magic+version, assert in vm.rs; make build.rs staleness warnings errors.
4. **Dissolve `_register_action` (767-line inline table)** into declarative
   data rows like CONDITION_PATTERNS — greppable/diffable, no logic change.
5. **Retire zero-hit handlers** (7 confirmed: those_cards_add_hand_optional,
   opponent_after_conditional, sou_shinakatta, heart_choice,
   kore_niyori_cascade, baton_touch_effect, energy_under_member) via the
   repo's own disable→regen→removal-diff methodology; fold ~20 single-hit
   handlers into `_EFFECT_RULES` rows over time.
6. **`_fill_defaults` ownership split** (PARSER_NOTES structural issue #1):
   move extraction into parse_action, leave defaults-only. Reduces the 3–5×
   field-extraction surface that makes walker-guard changes risky.
7. **Do NOT touch**: `_walk`↔`_propagate_context` merge, FIX 10/12/13a/13b
   dissolution — removal-diffed as live with tiny blast radii.

## D. Test-frontier additions (from rules mass × card data)

- §9 is 382 lines (~23% of the rulebook) — check-timing cascade (9.5),
  play-procedure (9.6), auto-ability processing (9.7), replacement effects
  (9.10), LKI (9.11), source identification (9.12) deserve the same
  phase-machine treatment §7 of TEST_HARDENING_PLAN gave the turn machine.
- Replacement effects (9.10.2): multiple replacements on one event =
  affected-party chooses order — only "each-applies-once" exists; write the
  choose-order scenario when ≥2 replacement cards coexist (data check needed).
- Q39/Q34/Q33/Q31/Q29 rulings still unpinned (flagged in TEST_COVERAGE.md).
- Cross-seat mirror coverage per TEST_HARDENING_PLAN §5 ledger — two planned
  rows remain (SP-bp5-027-L conditional gate, S-bp7-025-L option carry-over).
- Timestamp-ordering layer collisions (9.9.1.7): the two singleton set-cards
  (PL!S-bp3-019-L score=4, LL-bp7-001-R＋ cost=10) are the only future
  collision candidates — pin them individually NOW so later stacks are
  detectable.

## E. Facts worth remembering

- Exclusion zone: engine-complete, card-empty (rule 4.13).
- Mulligan: fully interactive implementation, rulebook-only (6.2.1.6).
- 手札に加える = biggest effect family (119 unique abilities / 309 cards).
- Surplus hearts: 17 cards, heavyweight support — asymmetry OK.
- Trigger labels well-centralized (triggers.rs); residual raw substring
  checks at game_state/modifiers.rs:1030/1666 should use TriggerKind.
