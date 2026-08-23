# Full-Stack Audit & Prioritised Plan — 2026-08-23

## Progress log (update as fixes land)

| Date | Commit | What |
|---|---|---|
| 08-24 | 20094f5a | W0/H1+H2+H3: real `--validate-only` flag added to extract_card_abilities.py, coverage.yml no longer swallows failure, new `.github/workflows/engine-tests.yml` runs `cargo test --test run_all` on push. Baseline seeded (= `{}` — semantic validation currently reports **0 issues** on the whole corpus, so any future issue fails `--check`). Baseline suite: **2591 passed / 0 failed** (~1.4s after build — cheap enough to gate every step) |
| 08-24 | 213590a4 | P2: deleted FieldExtractor (~180L, 0 callers), `_DEBUG_LOG`/`set_debug_section` plumbing + stale docstring claims, compile_abilities.py dead vocab/ENCODE/norm/encode block (~260L). Output byte-identical |
| 08-24 | e040e30a | P8: removed typo pattern `控え室か ら` (0 corpus hits) + later duplicate rows in SOURCE/DESTINATION_PATTERNS (earlier guards kept — they precede over-matching patterns like `からライブカード`). Byte-identical |
| 08-24 | e98c1ef8 | Phase-1 leftover: deleted unused `segment_clauses` Stage-A IR (+helpers `_segment_sentence`/`_CONDITION_MARKERS_IR`/`_LINK_PREFIXES`, header comment, test file ~230L). Kept `_ir_depth_scan`/`_split_sentences_nesting`/`_find_depth0`/`_split_marker_depth0` (live callers). Byte-identical |
| 08-24 | — | Discovery: PARSER_NOTES "Phase 8: 33 tuple-format _ACTION_RULES entries" is STALE — all registrations already go through the ActionRule-normalizing `_register_action`; dispatch has no TypeError workaround. Phase 8 = done |
| 08-24 | 2121cacd | P1: extract script now calls real `parse_ability` (deleted the weaker inline copy). Fixes needed along the way: `normalize()` collapsed `\n`-bullets and broke choice parsing → added `normalize_multiline`; back-fill loop was re-scanning cost text and double-gating existing conditions → now effect-text-only, only when no condition exists, leading-gate-only. Net 46/936 abilities improved (leading gates captured + `_clean` of empty/default fields). Suite 2591/0. **This closes audit item P1 and retires the dead `parse_ability` twin** |
| 08-24 | 2d159cd0 | Phase-1 leftover closed: `_try_phase_gate` now delegates to `extract_phase_gate` (corpus output unchanged — the flagged behavioral diffs never fired on this corpus) |
| 08-24 | f1f6a1ed | Special rules: 「ブレードの数はNつになる」 generalized from hardcoded count=3; extra_checks block documented as position-sensitive by design (E2b verdict) |
| 08-24 | 6a414e0a | Special rules: generic `_try_play_time_cost_set` handler for 「プレイに際し…コストはNになる」 (priority -10); LL-bp7-001 override deleted. Output matches engine contract |
| 08-24 | cfbe79f8 | Engine side of LL-bp7-001 de-hardcoded: `play_time_alt_cost_chars` (any N/any char-count), k-slot backtracking assignment replaces fixed [Vec;3], prompts derived from card data. 17-test card harness incl. fuzz green |
| 08-24 | 2b452d27 | vm.rs draw-count=1 fixup removed — redundant with `effects/draw.rs` `count_or(1)` default. Suite 2591/0 |
| 08-24 | aab5f54b | R2: shared `restore_performance_need_heart_modifiers` replaces the copy-pasted block in live-victory + live-success flows |
| 08-24 | 44a7830d | R2: baton-protection scan unified (player.rs / phases.rs / game_setup.rs → `ability::util::has_cannot_baton_touch_protection`) |
| 08-24 | ce245103 | R2: single `activation_position_index` helper (3 copies with divergent unknown-token policies preserved per site) |
| 08-24 | e377673c | **F1 describe-parity gate landed**: new test walks all effect nodes of all 936 abilities, fails on any raw-text fallback in EN or JA. Fixed the 8 gaps it found: templates for select_number, modify_yell_source, suppress_ability_trigger (both langs) and reduce_live_card_set_limit EN arm (the audit's i18n bug). 1683 nodes now fully templated |
| 08-24 | c56c4039..606c4dc0 | R2/R9 dedups: single `norm_group_name` (2 copies); dead `ActionType::SetCardIdentityAllRegions` removed per REFACTOR_BACKLOG 1a; empty `impl AbilityCost{}` dropped. Note: audit's "Orientation enum dead" claim is STALE — display.rs uses it |
| 08-24 | 0fd0d1c2 | **F2 validation rules landed**: per_unit scaling, cannot_restriction scope, effect_self_clamp. Baseline records exactly 1 known gap (bp5-010 score clamp) as regression floor. The other two rules pass clean on the whole corpus |
| 08-24 | b97b5621 | **F3 fidelity report landed**: `describe_dump` bin + `cards/describe_fidelity_report.py`. Immediately surfaced + fixed 2 real describe bugs: LL-bp7-001 rendered as 「Eを3増やす」 (modify_cost arm used additive count instead of set value) and restriction arms rendering raw tokens. Remaining low-overlap entries are F4-class (conditions not rendered by describe) |
| — | — | Deferred with rationale: R1 movement-tracking unification (54 refs/12 files; direct writes to recently_moved_* are scratch-channel assignments from choice/move handlers — needs characterization tests before touching); owner-resolution boilerplate (~17 sites but borrow contexts differ per site) |
| 08-24 | d5e30686..7d17afe5 | Phase 5 FIX-block triage started (empirical removal + byte-diff per block): **FIX 6 removed** (opponent_action flattening has no producer anymore); **FIX 2 & FIX 3 verified load-bearing** and documented (removal changes 4 resp. 2 corpus abilities). Lesson: these blocks are mostly live compensations for handler-emitted shapes — dissolution requires fixing the producers first, exactly as PARSER_UNTANGLE_PLAN warned |
| 08-24 | c71b8937..7716714e | Phase 5 triage continued: FIX 7/7b, 9, 9b all verified LOAD-BEARING (1–2 abilities each) and documented; FIX 14 comment corrected (stats-only counter, never did source inference). PARSER_NOTES.md now carries the full triage table + Phase 8 marked done |
| 08-24 | 3bd92579 | Test-gap burn-down: batch 10 covers 3 depth-none abilities with positive+negative cases (higher-cost-member draw gate, success-zone comparison score, distinct-name KALEIDOSCORE count). depth=none 164→159, covered cards 600→615, suite 2598/0 |
| — | — | R8 condition-cache keys assessed & deferred: `format!("{:?}")` is a complete content-addressing scheme over the struct; changing it risks subtle cache-hit changes for no correctness gain |

Verification loop used per step: regen abilities.json → byte-diff vs pre-step copy
(only `generated_at`/`engine_commit` may differ) → python parser tests → `cargo test --test run_all`.

---

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

## Part 4 — Ability text ↔ code expressiveness audit

How faithfully each representation layer carries the meaning of the Japanese text, with concrete
traced abilities. Layers: **JA text → parser JSON → bytecode/EffectKind → handlers**.

### 4.1 The same sentence has multiple encodings (and handlers must sniff all of them)

**「〜につき」 per-unit scaling** — 66 occurrences. `per_unit_type` is a soup of Japanese counters,
English zone names, and pseudo-zones, resolved by *sniffing other fields*:

| per_unit_type value in corpus | count | resolved as (util.rs:2108-2121, :2167-2184) |
|---|---|---|
| `"枚"` | 26 | Hand — *unless* `filter.card_type == member_card`, then UnderMember |
| `"人"` | 24 | Stage members |
| `"discard"` | 8 | actually Waitroom (控え室) — naming drift baked into data |
| `"live_card_zone"`, `"energy_deck"`, `"group_name"`, `"heart_colors"`, `"下"`, `"つ"` | ~10 | special-cased per handler |
| `None` (with per_unit=true) | 2 | falls through `_ => return 1` |

The real zone sometimes lives in a *different field* (`location`), so the same sentence
「自分の成功ライブカード置き場にあるカード1枚につき」 is encoded zone-in-per_unit_type for some cards
and counter+location for others; misc.rs:524-586, score.rs:33-88, state.rs:1522-1532 each re-derive
it differently. **Improvement:** a typed `PerUnit { zone: Zone, filter: CardFilter, n: u16 }` node;
one resolver; migrate the 66 abilities via golden-gated regeneration.

**「そうした場合／そうしたとき」 consequences** — 44 occurrences, **three different encodings**:
- implicit gate on `sequential` (31): compound.rs:584-601 sets `condition_failed` when the gating
  action moved *and* selected nothing (`was_moved == 0 && was_selected == 0`). Works for the traced
  cases (PL!-bp3-004-R＋ optional discard→retrieve; PL!S-bp3-006-R＋ cost-move→debut-with-cost+2),
  but it's a proxy: any future consequence shape that neither moves nor selects silently always-fires.
- explicit `conditional_on_optional` (7): PL!N-bp7-011-R＋ etc. — a second, better shape for the
  identical semantics.
- `conditional_alternative` / ad-hoc `last_move_moved_any` (compound.rs:411-422): a third path that
  only recognizes modify_score and two move shapes as "consequences".
  **Improvement:** one first-class `Consequent { gate: PreviousActionSucceeded, effect }` node;
  delete the was_moved/was_selected heuristics.

**Referential clauses 「これにより〜したカード」** — no structural home at all. Threading happens
through game-state globals: `mods.last_cost_moved_card_ids` written at choice.rs:558/:741 and
move_cards.rs:2916, read at misc.rs:933-948; plus resolver-scoped `moved_cards`/`selected_cards`.
E.g. PL!HS-bp1-005-PR 「これにより置いた枚数分カードを引く」 and PL!HS-pb1-003-R
「その枚数に1を足した枚数のカードを引く」 are implemented entirely off these globals. One global
overwrite mid-chain = wrong draw count, with nothing pointing from the ability JSON to that fact.

### 4.2 Duration semantics

Corpus durations: `live_end` ×251, `as_long_as` ×62(71 nodes), `this_turn` ×1, `unless` ×1.
- The 62 「〜かぎり」 abilities (e.g. PL!SP-bp4-005-R＋ 「エネルギーが10枚以上あるかぎり」) are
  常時-style constants handled by `recalculate_constants` re-evaluation — correct behavior, but their
  `Duration::AsLongAs` tag is never what executes.
- `check_expired_effects` (abilities.rs:2324-2330) stubs AsLongAs/Unless to expire at live end. Today
  no handler appears to store an as_long_as *temporary* effect (the tag is dead weight), but the trap
  is armed: the first triggered 「〜するかぎり」 temporary bonus will silently become ThisLive.
  **Improvement:** either implement condition-re-eval expiry or make decode reject
  duration=as_long_as outside constant context (loud, not silent).

### 4.3 use_limit 「この能力は1ターンにN回まで」

Worse than documented: recording is spread over **6 sites across 2 files**, not 4 in one —
resolver.rs:646-675 (guard helper), :760-774 (pre-cost check), :836-1028 (four recording paths:
early-record-if-condition-met, activation-failure, post-choice, post-effect), *plus*
choice.rs:3034-3123 and :3249-3260 (optional-effect variants record inside choice resume).
Traced example PL!N-bp7-006-R＋ (起動, ターン1回... use_limit semantics) depends on which of the six
paths its choice flow takes. **Improvement:** single `record_use(phase)` funnel; phases named after
the rule reason (activation_started / effect_completed / optional_accepted).

### 4.4 Draw-count fixup is masking missing structure (vm.rs:1364-1380)

Exactly **2 corpus abilities** hit it (PL!HS-bp1-005-PR, PL!HS-pb1-003-R) — both are dynamic draws
whose true count comes from `last_cost_moved_card_ids`. The decoder injects `count=1` so generic code
doesn't divide-by-none, but the *real* semantic (draw N where N = cards discarded) exists nowhere in
the JSON. **Improvement:** a `DynamicCount::FromPreviousMove{offset}` variant (dynamic_count.rs
already exists as a home); then delete the fixup.

### 4.5 What describe.rs says about expressiveness

`describe_effect_en/ja` template-render EffectKind back to text — the closest thing to a round-trip
fidelity check:
- Structural coverage: **EN 929/936, JA 931/936** fall through to raw-text fallback
  (describe.rs:504/:1122); 7 EN / 5 JA abilities have no template at all.
- Bug found: `reduce_live_card_set_limit` has a JA arm (describe.rs:967) but **no EN arm** — English
  prompts show raw Japanese for those cards.
- **Conditions are never rendered anywhere** — 47% of effects carry conditions, so even
  "templated" descriptions omit half the sentence. Choice options dropped too ("Choose 1").
- No bin/test dumps all descriptions today; a ~40-line loop over `get_ability(0..NUM_ABILITIES)`
  would enable a normalized diff report against `full_text` (strip `{{icons}}`, digits→N).

### 4.6 Dead expressive capacity vs forced-generic shapes

- `ActionType` has ~66 variants; the parser produces 44. **20 dead variants** incl. `shuffle`,
  `discard_card`, `set_cost`, `custom` (=the Default!), `reveal_per_group`, … — several still decoded
  and handled engine-side (card.rs:1107-1170). Parser-side capacity that was built but never wired.
- Conversely ~15 sub-node `type` strings (`area_move`, `baton_touch`, `per_unit`, `temporal_count`,
  `zone_change`, …) live *outside* both ActionType and ConditionType enums — stringly-typed shadow
  schema that decoders skip silently when unknown (see C1).
- `EffectKind` is only 14 coarse variants; nearly all meaning lives in `EffectFilter`. Fine as design,
  but it means enum-count lints overstate expressiveness.

### 4.7 Text markers without structural counterparts (quantified)

| Marker | occurrences | covered? |
|---|---|---|
| 「〜につき」 | 66 | 58 yes; **8 uncovered** (parenthetical post-yell bonuses riding on perform_yell/score nodes — e.g. PL!N-bp1-029-L Eutopia) |
| 「その後」 | 30 | 29/30 via sequential |
| 「「X」以外」 exclusion lists | 44 | mostly covered (PARSER_UNTANGLE_PLAN note partially stale — 『group』以外/「name」以外 ARE parsed at parser.py:5719/:6520/:6850); remaining gaps: 選んだカード以外-shuffles, 手札以外から登場, Q&A それ以外 branches (~6 abilities) |
| 「この効果では…ない」 self-clamps | 9 (1 real) | **0** — e.g. PL!N-bp5-010-R's 「スコアは０未満にならない」 clamp has no structural home; check if hardcoded in score.rs |
| 「できない」 negation | 11 (5 non-parenthetical) | covered via restriction/negation/conditional_negation |
| 「〜ごとに」「最大まで」「直後」 | 0 | not in this game's dialect |

### 4.8 Fidelity CI ladder (new recommendations, ranked)

These complement Part 3's waves — they're detection tooling specifically for *text↔code fidelity*:

1. **F1 — describe-parity engine test** (hours): loop 0..NUM_ABILITIES asserting (a) no node hits the
   describe fallback arm, (b) EN/JA arm-set equality (catches the reduce_live_card_set_limit class).
   Gates every future parser→enum addition automatically.
2. **F2 — three new `_validate_semantic` rules** (a day): につき-without-per-field (8 hits now),
   non-parenthetical できない-without-restriction, この効果で-self-clamp. Baseline-seed them (H3).
3. **F3 — describe-dump + normalized diff report** (non-gating): the bin above + python normalizer;
   weekly top-N worst matches. Also directly improves UI prompts (describe feeds choices at
   game_state/abilities.rs:1871/:1894).
4. **F4 — typed PerUnit + Consequent nodes** (Wave-3 work, golden-gated): dissolves the sniffing in
   util.rs/misc.rs/score.rs/state.rs and the was_moved/was_selected proxy. This is the single biggest
   *expressiveness* upgrade available.
5. **F5 — dead-capacity lint**: extend test_inventory.py to emit ActionType×corpus tables flagging
   unused variants (like ABILITY_MATRIX.md) so dead variants get deleted-or-wired decisions instead
   of accumulating.

### 4.9 Traced-example verdict table (spot checks)

| Ability | Text pattern | Code path | Verdict |
|---|---|---|---|
| PL!-bp3-004-R＋ | optional discard、そうした場合 retrieve | sequential + implicit gate (compound.rs:584) | faithful; mechanism fragile |
| PL!S-bp3-006-R＋ | cost-move、そうした場合 debut w/ cost+2 in same area | sequential + was_moved gate + area shim | approximate — area tracking rides last_area_move shims |
| PL!N-bp7-011-R＋ | optional discard、そうしたとき self-retrieve ×2 | conditional_on_optional (explicit) | faithful — but proves 2nd encoding exists |
| PL!HS-bp1-005-PR | draw = number discarded | last_cost_moved globals + decoder fixup | unrepresentable in JSON (works via globals) |
| PL!HS-pb1-003-R | draw = discarded+1 | same globals | same |
| PL!-bp3-004-R＋ (登場) | draw per member on stage | per_unit_type:"人" | faithful |
| PL!SP-bp2-009-R＋ | blade per 2 cards in hand | per_unit_type:"枚",count=2 | faithful but counter-string typed |
| PL!N-bp5-010-R | score floor clamp この効果では | ? (no structural node) | needs verification vs score.rs hardcode |
| 62× 「かぎり」 constants | conditional constant bonus | recalculate_constants path | faithful; Duration::AsLongAs stub latent-trap |

Items marked "needs verification" should become targeted tests before any Wave-3 refactor touches
their handler.

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
