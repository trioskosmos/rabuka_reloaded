# Full-Stack Audit & Prioritised Plan — 2026-08-23

Scope: `cards/abilities.json` + Python parser ecosystem + Rust engine, read end-to-end.
Goal framing: **the game must behave exactly as written in the Japanese ability text.**
Complements (does not replace) `CODE_AUDIT_2026-08-23.md`, `REFACTOR_BACKLOG.md`,
`ABILITY_PIPELINE.md`. Items already in those docs are marked *(known)*.

Corpus today: 2,526 cards / 1,565 with abilities / 2,011 abilities / **936 unique**.
Test suite: 409 files / ~2,575 tests, all green locally — **but nothing in CI runs them.**

---

## Part 0 — The three holes in the safety net (fix before anything else)

Everything below assumes we can refactor safely against the 2,575-test suite. We can't yet,
because:

| # | Hole | Evidence | Fix |
|---|------|----------|-----|
| H1 | **CI never builds or runs `cargo test --test run_all`** | `.github/workflows/` contains only coverage.yml + deploy-pages.yml | Add a CI job running the suite (Windows runner or container; suite is deterministic, no RNG) |
| H2 | **The parser-validation CI step has never actually run** | coverage.yml:41 calls `extract_card_abilities.py --validate-only`; that flag doesn't exist (argparse exits 2), and `\|\| echo "skipped"` swallows it | Add the flag or fix the workflow line; make failure loud |
| H3 | **Regression baselines are empty** | `cards/ability_extraction/validation_baseline.json` is `{}` → `--check` only fails on brand-new rule names, so issue-count regressions pass silently | Seed baseline with current counts; add `--strict` mode where a parse exception fails the build instead of shipping `{"actions": []}` |

Also cheap and worth doing first:
- **Strict choice-drain helper** (`drain_choices_strict`) in `engine/tests/helpers/mod.rs`: panics on
  unexpected choice types instead of the widespread `while has_pending_choice { select_indices(&[0]) }`
  auto-answer pattern that masks wrong prompts (e.g. `live_success_rules_test.rs:38-44`).
- **Whole-board zone snapshot assert helper** for both players — no test currently compares full state;
  collateral damage is invisible unless individually asserted.
- Uncomment the three commented-out checks in `run_all.rs:24-121` or delete them.

---

## Part 1 — Correctness issues (game ≠ Japanese text)

Ordered by how directly they falsify card behavior.

### C1. Silent decode fallbacks can turn any ability into a no-op
- `AbilityRef::resolve()` returns `Ability::default()` on decode failure (`ability_store.rs:64-72`);
  same at `vm.rs:157,166`.
- Unknown condition variant ⇒ ability dropped silently (`condition_decoder_gen.rs:1377`).
- Unknown filter string ⇒ `NoAbility` (`vm.rs:951-968`); unknown distinct ⇒ `CardName` (`vm.rs:930-948`);
  unknown zone ⇒ `Zone::Unknown` (`enums.rs:131`); unknown keyword ⇒ skipped with log only (`vm.rs:1554`).
- `read_condition_value` returns `None` for `TAG_OBJECT`; today every corpus type is mapped (verified),
  but **nothing enforces it** — one new handler emitting an unmapped `type` encodes as TAG_OBJECT and
  vanishes at runtime.

**Fix:** add a *decode-all-936-abilities* gate to the bytecode oracle tests (partly exists via
`bytecode_validation_test.rs` — extend it to fail on any default-fallback/log-skip path), and make the
compile-time tag maps (`COND_TO_VARIANT_TAG`/`ACTION_TO_VARIANT_TAG`, compile_abilities.py:364-473)
derived from `card.rs` serde renames instead of hand-maintained, asserted by `validate_schema.py`
(wired into CI — it currently isn't). Positional variant bytes mean inserting a `Condition` variant in
card.rs silently re-decodes every later condition as the wrong variant if artifacts go stale.

### C2. Hardcoded single-card rules inside generic code
- LL-bp7-001 alternative cost detected by shape-matching in two places + bespoke payment path
  (`modifiers.rs:899-906`, `phases.rs:1385-1410,1493-1537`).
- Play-time cost reduction hook detects abilities by condition-shape heuristics (`phases.rs:1270-1383`).
- Set-cost cleanup band-aid `remove_cost_modifier_set(card_id)` after every play (`phases.rs:1258`)
  would wipe legitimate set-cost constants applied during play.
- Draw-count=1 fixup injected into decoded DrawCard actions in the decoder (`vm.rs:1364-1380`) —
  data patching in the wrong layer; masks a compiler-side bug.
- Generic fallback hardcodes `"ブレードの数は3つになる"`→3 and `emma_punch` choice_type
  (parser.py:1777-1788); `card_overrides.py` re-implements parse_ability internals for Mari.

**Fix:** general 「プレイに際し…コストはNになる」 handler + move fixups into compiler/handlers.
Behavior-changing wave — needs golden-file harness (see Part 3 sequencing).

### C3. String-typed rules machinery that parses wrong rather than erroring
- `HeartColor::from_str` → `Heart00` fallback (card.rs:4001); `CardState::from_str` → Wait;
  `ZoneId::from_str` → Unknown; comparison/target enums likewise default instead of erroring.
- `prohibition_effects: SmallVec<String>` matched by substring (`tracking.rs:117`); restrictions
  serialized as comma-joined strings parsed back by `contains` (modifiers.rs:587) — commas in card
  names break parsing.
- Temporary-effect expiry dispatches on a string `effect_type` (`abilities.rs:2356+`); adding an
  effect type without touching this match leaks modifiers past expiry. `Duration::AsLongAs/Unless`
  are stubbed to behave like `ThisLive` (abilities.rs:2324-2330).

**Fix:** typed prohibition/restriction payloads + strict `TryFrom` at boundaries (lenient only at JSON
ingest). This is prerequisite for real AsLongAs semantics.

### C4. Trigger/timing heuristics encode rules text as code smells
- TAS scan gates: discard-location guard (`abilities.rs:410-437`), energy-zone comparison guard
  (:479-488), self-target+movement gate (:511-520); trigger subtype recognized via
  `condition.get_text().contains("すべて")` (:764).
- 「そうした場合」 consequence gating proxied by `was_moved == 0 && was_selected == 0`
  (`compound.rs:584-601`); ad-hoc `last_move_moved_any` recognizes only some consequence shapes
  (:411-422).
- Condition cache keyed by `format!("{:?}", condition)` (resolver.rs:242-254) — Debug-format keys,
  already caused stale-cache bugs patched by stripping conditions when saving pending actions
  (`compound.rs:507-535`).
- use_limit bookkeeping split over four call sites in `resolve_ability` (resolver.rs:852-1028) —
  fragile vs double-count/leak.

### C5. Known-unimplemented / under-implemented rules areas
- Phase-begin/end-of-turn triggers documented absent (`triggers.rs:70-76`) — none required by pool yet,
  but blocks future sets.
- Deck legality warn-only; no max-4-copies check (deck_builder.rs:84-119).
- Q118 placement-incomplete guard skips draws globally (`effects/mod.rs:54-69`).
- Double victory check in `check_timing` (actions.rs:1308 & 1340); refresh logic duplicated inline
  instead of calling `Player::refresh` (actions.rs:1276-1303 vs player.rs:545).

### C6. Untested correctness classes (from test audit)
- Simultaneous-trigger ordering between players / stack order — spot cases only.
- Opponent-as-actor: P2 activations need manual `set_active` trick, used by few modules; interrupt
  windows effectively untested.
- No determinism/replay or serialize↔deserialize round-trip tests; shuffle/RNG paths never exercised
  (tests hand-order decks).
- web_ui reads engine choice JSON with **zero JS tests** — contract drift unguarded.
- Helper pool falls back silently to template IDs on exhaustion → known modifier-sharing bugs
  (helpers/mod.rs:289-293).

### Data-layer gaps (abilities.json / qa_data.json)
- **176 abilities fully unreferenced by tests**; weakest sets cl1 2/10, PR 40/52, sd2 21/31, pb2 30/43;
  weakest triggers ライブ成功時 (89/122), 起動 (80/96), 常時 (100/117).
- 82 of 280 QA rulings have no `related_cards`, so coverage matching degrades to Q-id substring.
- 2 `is_null` abilities (PL!HS-PR-010-PR, PL!HS-bp1-019-L) — parser can't structure them at all.
- Coverage depth is per-*file*, not per-test; one comment mention counts as covered.

---

## Part 2 — Duplication & merge opportunities

### Python side (parser ecosystem)
*(known debt in PARSER_NOTES/PARSER_UNTANGLE_PLAN summarized; new finds flagged)*

| # | Merge | Notes |
|---|-------|-------|
| P1 | Extract script's inline copy of `parse_ability` → call the real one | Live copy lacks the stronger condition back-fill loop (parser.py:1281-1338) and `_validate_effect`; triple phase-gate-merge duplication. **Behavior-improving**, golden-gate it |
| P2 | Delete dead code: `parse_ability` dead twin, `FieldExtractor` (~180 lines, 0 callers), compile_abilities vocab/encode block (~260 lines :7-267), orphaned `_DEBUG_LOG` plumbing (no writer exists anywhere despite docstring claims) | Behavior-preserving |
| P3 | Finish Phase 8: convert remaining tuple-format `_ACTION_RULES` to `ActionRule` *(known, pending)* | Mechanical |
| P4 | Decide `segment_clauses`: wire in as THE splitter or delete + its 12-test suite *(deferred in plan)* | Either way kills aspirational architecture |
| P5 | Consolidate energy-under-member ownership: currently 4 code paths (parser.py:2124-2139, :3110-3112, :9758) | Byte-gated |
| P6 | Derive variant-tag maps + decoder field maps from card.rs; wire `validate_schema.py` into CI; make READER_MAP gaps errors not silent skips | Cross-language safety |
| P7 | Structured logging w/ card ids; stop dumping full MISSING-MECHANICS report on every run (parser.py:13534-13631) | DX |
| P8 | Fix typo'd/unreachable patterns (`"控え室か ら"` parser_utils.py:411) + duplicate rows | Preserving |
| P9 | Data-model cleanups when convenient: `cards[]` composite strings `"ID \| name (ab#N)"` → structured fields; `triggers` comma-string → array; unify `action:` vs `type:` discriminators and near-duplicates (`temporal`/`temporal_condition`, `appearance`/`appearance_condition`); collapse 3 parallel value/operator pairs (`cost_limit`, `cost_total`, `need_heart_total`) into typed comparator | Breaking format change — do once, with a version bump + regeneration |

### Rust side

| # | Merge | Notes |
|---|-------|-------|
| R1 | Movement tracking: make `MovementEvent` log the single source; delete hand-synced `recently_moved_cards` / `recently_moved_from_zone` / `cards_moved_this_turn` and backward-compat shims (types.rs:785-789 says the log was meant to replace them) | High value — removes a whole drift class feeding trigger conditions |
| R2 | Copy-paste extractions: need-heart restore ×2 (live.rs:52-70 / turn/triggers.rs:391-410), baton-protection scan ×3 (player.rs:296, phases.rs:1021, game_setup.rs:1345), b_heart07 yell decode ×2 (live.rs:1484 / phases.rs:449), refresh-in-check_timing, stage-index mapping ×~10 sites despite `MemberArea::to_index/from_index` existing, activation-position parsing ×4 (resolver.rs:331, condition.rs:889, condition/card.rs:3034, enums.rs:58) | Trivial diffs, pure win |
| R3 | Unify the four modifier layers (GameModifiers tables / constant-recalc shadow maps / temporary_effects string-revert / success-zone shadow maps) under one registry with owner+scope+duration metadata | Highest architectural win; enables real AsLongAs; big — needs characterization tests first |
| R4 | Single zone enum (`ZoneId` vs `ability::Zone` vs `Location` vs raw strings); total conversions | Medium blast radius |
| R5 | `SelectTargetBuilder` for 58 raw prompt sites + centralize 12 `pay_optional_cost` prompt constructions | Bilingual-prompt consistency |
| R6 | Collapse move_cards.rs `resolve_from_*` family (~1100 lines of near-siblings incl. looked_at/revealed_cards parallel pairs) behind a source-resolver abstraction | File is 148KB |
| R7 | Split god functions: `execute_sequential_effect` (~600L compound.rs:44-648), `recalculate_constants` (~600L), TAS scan (~360L), `handle_select_card` (~660L), `execute_live_victory_determination` (~850L), `handle_play_member_to_stage` double-baton inline (~365L) | Mechanical, improves reviewability |
| R8 | Proper condition-cache key (variant tag + normalized fields) replacing `format!("{:?}")` | After R7 touches compound.rs anyway |
| R9 | Dedup util-level helpers: two `fn norm`s (util.rs:510/559), zone-label fns ×3 (condition.rs:246, describe.rs:26/628), count_matching family (util.rs:1652-2290) | Low risk |
| R10 | Consolidate use_limit recording into one phased function | Gate on Q58-Q61 tests |

### Test-side merges
- Per-test depth attribution in `test_inventory.py` (attribute covering fns, ignore comments) before
  trusting L1/L2/+choice numbers to gate refactors.
- One canonical "run everything" doc listing test_inventory/find_bad_tests/gap_report/--check.

---

## Part 3 — Prioritisation

Guiding rule: **strengthen detection before moving behavior.** The suite is excellent but has the
Part-0 holes; several silent-fallback paths can eat a regression unnoticed.

### Wave 0 — Safety net (days, zero engine-behavior risk)
1. CI job running `cargo test --test run_all` (H1)
2. Fix `--validate-only` CI step (H2); seed validation baseline + `--strict` (H3)
3. Extend bytecode oracle: decode-all gate that fails on ANY silent fallback/skip path (C1 detection);
   derive variant-tag maps from card.rs + wire validate_schema.py into CI (P6)
4. `drain_choices_strict` helper + migrate blind drain loops; zone-snapshot helper
5. Strict `TryFrom` conversions for heart/state/zone/comparison enums (C3 partial) — may expose latent
   bad data; run full suite + inventory check after

### Wave 1 — Cheap structural wins (behavior-preserving)
6. R2 copy-paste extractions (pure dedup, trivial diffs)
7. P2 Python dead-code deletion; P8 pattern fixes
8. P3 ActionRule conversion; R9/R10 small consolidations
9. R1 movement-tracking unification (medium effort, removes drift class)

### Wave 2 — Parser consolidation (golden-file gated, per PARSER_UNTANGLE_PLAN loop)
10. P1: extract script calls real `parse_ability` (activates stronger back-fill → treat output diffs
    as improvements, review each)
11. Plan Phases 2-5 (single-pass extraction, merge tree walks, dissolve FIX blocks) exactly as written
12. P5 energy-under-member consolidation; decide segment_clauses (P4)

### Wave 3 — Rules fidelity (behavior-changing, needs Wave 0 + per-card regression tests)
13. General play-time/alternative-cost handler replacing LL-bp7-001 hardcodes + shape heuristics (C2);
    move draw-count fixup out of vm.rs; remove set-cost cleanup band-aid
14. Typed prohibition/restriction/expiry payloads (C3 core) — unlocks real
    「ライブ終了時まで」/as-long-as edge cases; then implement `Duration::AsLongAs/Unless` properly
15. Condition cache key fix (R8) + use_limit consolidation (R10)

### Wave 4 — Big architecture (only after Waves 0-3)
16. R3 unified modifier registry (one revert path; delete shadow maps)
17. R6/R7 splits (move_cards resolver family, god functions)
18. R4 single zone enum
19. P9 abilities.json format v2 (structured cards[], triggers[], discriminator unification)

### Continuous (parallel track)
- Test-gap burn-down: prioritize the 176 unreferenced abilities (cl1/PR/sd2/pb2 sets; ライブ成功時×33,
  ライブ開始時×56 gaps) using BATCH-style plan docs; backfill qa_data `related_cards` (82 rulings);
  add opponent-as-actor + simultaneous-trigger-ordering matrix tests; one replay/determinism test;
  one engine↔web_ui choice-contract pinning test.

---

## Appendix — Notable facts worth remembering

- `vm.rs` is not a VM: tagged binary-JSON serialization, eagerly decoded per ability, cached per slot.
  The name misleads.
- `engine/src/core/pool.rs` is a memory pool, not test RNG; there is no RNG in tests at all —
  everything is deterministic by construction (shuffling never exercised).
- `build.rs` only does staleness/size checks; real baking lives in compile_abilities.py which also
  emits SNES bank chunks.
- All 936 unique-ability entries share identical top-level shape; no duplicate texts; ids ↔ cards.json
  linkage is exact (1565 = 1565, no orphans).
- `generated_by`/`parser_version:"1.0"`/`input_hash:null` metadata is vestigial — provenance isn't
  actually recorded (ABILITY_PIPELINE.md asks for reproducibility inputs).
- Dead-but-dangerous: `ActionType::ConditionalOptional` is dead as input but alive as internal routing
  tag (see REFACTOR_BACKLOG §1b) — don't naively remove.
