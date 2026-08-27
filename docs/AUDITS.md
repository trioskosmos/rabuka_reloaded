# Audits

_Consolidated from: FULL_STACK_AUDIT_2026-08-23.md, FRESH_AUDIT_2026-08-19.md, CODE_AUDIT_2026-08-23.md, DEEP_READ_2026-08-25.md, CASTING_AUDIT.md, EFFECT_ONLY_AUDIT.md, PAIN_POINTS.md, RULES_GAP_ANALYSIS.md_

## Full Stack Audit  (`FULL_STACK_AUDIT_2026-08-23.md`)

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
| 08-24 | cf1c640f | Test-gap burn-down: batch 11 covers 4 cl1/sd2/pb2 abilities (debut blade-3, conditional constant heart03 pos+neg, opponent-side cost≤2 rest). depth=none →156, suite **2602/0**. Note: dual-trigger cards (「登場, ライブ開始時」) need `contains` matching, not equality, in trigger lookup |
| 08-24 | e0b58d66 | Test-gap burn-down: batch 12 covers 3 pb1/bp6 abilities (DOLLCHESTRA live-zone draw gate, opponent-energy comparison score pos+neg, スリーズブーケ teammate energy activation). depth=none →153, covered cards →621, suite **2607/0**. Lesson: mojibake in console output misleads — verify card text via codepoints/files, not terminal |
| 08-24 | b09ab1b5 | **Real rules bug fixed**: `check_aggregate_total`'s Stage branch summed blade totals for ANY aggregate=total stage condition, silently breaking member-count conditions with target=both (「自分と相手のステージにメンバーが合計6人いるかぎり」 — PL!S-PR-042-PR etc. never activated). Guarded on target=self (all corpus blade-totals are self; cross-player counts now use combined counting). Found BY the new batch-13 tests — the burn-down paying for itself immediately. Suite **2612/0**, depth=none →150 |
| 08-24 | c0961074 | Test-gap burn-down: batch 14 covers 3 abilities incl. a mirror-image pair (bp7-014: opponent-energy-ahead heart02; bp7-020: self-energy-ahead blades — both 常時, both verified pos+neg) and sd1-022 group blade grant with non-Aqours exclusion. depth=none →147, suite **2617/0** |
| 08-24 | 42803688 | Test-gap burn-down: batch 15 covers 3 abilities (sd1-026 SD+SRL variants energy≥9 score gate, pb1-020 Aqours printed-heart04 aggregate ≥10 → score+2 — exercises the group_condition aggregate-total path). depth=none →144, suite **2621/0** |
| 08-24 | df430db9 | Test-gap burn-down: batch 16 covers 3 起動 waitroom-retrieval abilities (discard-cost member retrieve, rest-self+discard live-card retrieve, self-to-waitroom Liella! retrieve). First activation-flow tests in the series — cost-selection choices answered via `select_indices` after `activate_ability`. depth=none →141, suite **2624/0** |
| 08-24 | b7f1952d | **Second real bug fixed**: bp6-009-R's cross-position condition (「右と左のサイドエリアに、元々のブレード2つのメンバーがいるかぎり」) was unimplemented — `blade_limit`/`blade_limit_operator` existed in JSON but Condition never decoded them (audit risk C1 realized), and `check_original_blade_filter` skipped members entirely. Added fields to ConditionCommon + decoder regeneration + per-card blade check + position-pair requirement in evaluate_card_count_condition. Suite **2628/0**, depth=none →138 |
| 08-24 | c7823995 | Test-gap burn-down: batch 18 establishes the `revealed_cards` setup pattern (エール公開 conditions) — Wish Song distinct-Liella-5 score gate pos+neg, bp4-006-R retrieve-from-revealed. depth=none →135, suite **2631/0** |
| 08-24 | ce720802 | Test-gap burn-down: batch 19 covers draw-then-discard sequentials (bp1-009-R activation w/ energy cost, pb1-024-L live-success draw2/discard2 incl. keep-selection case). depth=none →133, suite **2634/0** |
| 08-24 | 9df367fb | Test-gap burn-down: batch 20 covers 3 look_and_select debuts (bp3-012-R look4/reveal 虹ヶ咲, pb1-028-N look2/add1, bp1-011-PR look5/live-card reveal) via the full play pipeline — energy paid + optional cost + look choice answered. depth=none →130, suite **2637/0** |
| 08-24 | 9e633e59 | Test-gap burn-down: batch 21 covers the pb2 debut-retrieval trio (CatChu!/5yncri5e!/KALEIDOSCORE waitroom retrieval behind optional discard cost). depth=none →127, suite **2640/0** |
| 08-24 | ffd54eb8 | **R1+R3 characterization suites landed** (9 tests): pin the five-view movement-tracking sync, area-move flagging, turn-scope reset; pin constant grant/revert lifecycle, live_end temporary registration+expiry, and manual-additive-vs-constant stacking. **New finding pinned as known gap:** choice-path cost discards emit NO hand→waitroom events and never enter cards_moved_this_turn — only effect-side moves are tracked. Post-R1 flipped assertions are pre-written in the file. Suite **2649/0** |
| 08-24 | ab7fca67 | **R1 slice 1 DONE**: handle_select_card's hand-cost discard now emits hand→waitroom MovementEvents (effect_only=false), feeding cards_moved_this_turn/turn_movements/batch log like every other move. Characterization test's pre-written flipped assertions enabled and green — cost-paid 「〜て控え室に置いた」 conditions now see cost discards |
| 08-24 | a6f70c3e | **R3 slice 1 DONE**: `is_revertable_effect_type` list + warn-on-registration at `push_temporary_effect` (sole choke point, 14 sites) + expiry catch-all upgraded debug→warn naming the leaking kind. Full suite emits zero leak warnings → all current kinds covered; future omissions are loud instead of silent leaks |
| 08-24 | a0ee2157 | R3 slice 2 prerequisite: prohibition-layer characterization (conditional cannot_live register/clear/re-register trio via recalc). Lesson: cannot_place on live cards is enforced at PLACEMENT time via `can_place_card_in_zone` checking the card's own printed ability — NOT via recalculate registration — and was already covered by kagayaiteru_q125. Redundant recalc-based assertion removed. Suite **2652/0** |
| 08-24 | 7ff9fe7d | Re-registered the three characterization modules lost in the parallel-session mod.rs rewrite (movement 6 / modifier 3 / prohibition 5 — cannot_place now pinned at the `can_place_card_in_zone` primitive incl. LiveCardZone↔SuccessLiveZone interchangeability + positive control). Also noted: parser untangle Phases 1–5 + corpus smoke-test infra landed from the parallel session. Suite **2658/0** |
| 08-24 | 8c8264e0 | Test-gap burn-down: batch 23 covers the baton-touch-debut gate family — source_character name match (中須かすみ replaces her own name → draw2+hand-discard, pos+neg) and replaced-member cost comparison (北条そふぃ over cheaper DOLLCHESTRA → +2 blades, pos+neg). Plus defensive `extract_source` regex for 「デッキをN枚上から/下から」 (corpus-neutral today). depth=none →122, suite **2658/0** |
| 08-24 | 7b01d3f8 | Test-gap burn-down: batch 24 covers named-baton-source debut draws (東條希 replacing 優木せつ菜, エマ・ヴェルデ replacing herself; draw 2 + hand-discard 2, pos+neg). Finding: this ability shape resolves fully inside the play action — no pending choice reaches the caller. depth=none →120, suite **2665/0** |
| 08-24 | db4b0862 | Corrected overstated audit claims (Q118 per-entry gate already tested ×5; multiplier bulk-wipe verified deliberate; AsLongAs arms now warn loudly) |
| 08-24 | 49cc73d4 | Test-gap burn-down: batch 25 covers set_heart_type (bp7-024-L) — lone Aqours member transformed to heart04, non-Aqours untouched, no-Aqours negative. depth=none →119, suite **2667/0** |
| 08-24 | d063399f | Test-gap burn-down: batch 26 covers 3 look_and_select sub-unit/name-filter debuts (pb1-015 CatChu!, pb1-016 KALEIDOSCORE, sd2-012 虹ヶ咲). depth=none →116, suite **2670/0** |
| 08-24 | fb51263a | Test-gap burn-down: batch 28 covers pb1-007-R lilywhite-gated 起動 retrieval (hand-discard-3 cost + lilywhite gate, pos+neg with genuine Printemps control). depth=none →114, suite **2673/0** |
| 08-24 | 5d9c98d5 | Test-gap burn-down: batch 29 covers PR-045-PR cost-7-gated baton-source debut draws (pos + neg). depth=none →113, suite **2675/0**. Lesson: always verify card IDs exist before writing tests — PL!S vs PL! prefix matters |
| 08-24 | c90a6c82 | Test-gap burn-down: batch 27 covers bp1-009-R debut (mill 2 from deck top → retrieve member from waitroom, exact waitroom accounting). depth=none →115, suite **2671/0** |
| 08-24 | — | **Wave-0 item 4 closed**: `drain_choices_strict(allowed, answer)` helper (choice-type allowlist + loud panic + 1000-prompt guard) and whole-board `board_snapshot()`/`assert_board_matches()` (per-zone ID lists for BOTH players, diff-only failure output) added to tests/helpers. Exemplar migration: all 20 blind `while has_pending_choice { select_indices(&[0]) }` loops in live_success_rules_test.rs → strict; only SelectCard/SelectAutoAbility prompts occur in those flows. run_all.rs commented blocks resolved: `test_parse_heart_color` + `test_no_custom_actions` restored as real tests (missing-file now panics instead of skipping), stale print-only parser-validation block deleted (CI's `--validate-only --check` supersedes it). Suite **2677/0** |
| — | — | R8 condition-cache keys assessed & deferred: `format!("{:?}")` is a complete content-addressing scheme over the struct; changing it risks subtle cache-hit changes for no correctness gain |

Verification loop used per step: regen abilities.json → byte-diff vs pre-step copy
(only `generated_at`/`engine_commit` may differ) → python parser tests → `cargo test --test run_all`.

---

Scope: `cards/abilities.json` + Python parser ecosystem + Rust engine, read end-to-end.
Goal framing: **the game must behave exactly as written in the Japanese ability text.**
Complements (does not replace) `CODE_AUDIT_2026-08-23.md`, `REFACTOR_BACKLOG.md`,
`ABILITY_PIPELINE.md`. Items already in those docs are marked *(known)*.

Corpus today: 2,526 cards / 1,565 with abilities / 2,011 abilities / **936 unique**.
Test suite: 483 files / ~2,929 tests, all green locally (~1.9s wall) — CI gates via
`.github/workflows/engine-tests.yml` + `coverage.yml --check` (added 08-24).

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
~~Phase-begin/end-of-turn triggers absent (triggers.rs:70-76)~~ — still true but none required by pool.
Deck legality warn-only — user verdict: fine as-is.
~~Q118 placement-incomplete guard skips draws globally~~ — **CORRECTED 2026-08-24**: the gate
(`optional_moves_all_moved`) lives on the ability-queue ENTRY, not globally; it cannot leak across
abilities, and the all-or-nothing semantics are pinned by five existing tests
(bp7_parser_gap_cards_test: kanon_missing_group_places_present_but_no_draw et al). No fix needed.
~~heart_color_multiplier bulk wipe on expiry~~ — **VERIFIED SAFE 2026-08-24**: every multiplier
entry is created by execute_set_heart_type which simultaneously registers a live-scoped temporary
effect, so the bulk clear can never outlive an active owner. Deliberate belt-and-braces.
~~AsLongAs/Unless expiry stub~~ — **RESOLVED 2026-08-24**: confirmed unreachable (no caller passes
those durations; parse_duration's only producer feeds nothing), arms now log::warn loudly if ever
hit instead of silently approximating.
~~Choice-path cost discards invisible to movement tracking~~ — **FIXED** (ab7fca67, R1 slice 1).
~~Aggregate-total target=both hijack~~ — **FIXED** (b09ab1b5).
~~blade_limit undecoded / cross-position condition unimplemented~~ — **FIXED** (b7f1952d).

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
| R11 | **Optionality unification (partially done 08-24)**: shared `ask_optional_move_gate` + `optional_gate_source` now cover deck/deck_bottom/energy_deck; remaining scattered shapes — resolve_from_stage/revealed/standard `can_skip = is_max \|\| optional` params, cost.rs Stage gate (ChoiceRoute::OptionalCost flow), PlaceEnergyUnderMember internal gate, conditional_optional machinery. End state: ONE gate decision enum consulted by all move executors | Medium; keep per-shape tests green |
| R12 | **is_activation idiom** (done for cost.rs 08-24): `current_ability_is_activation()` added to AbilityResolver; resolver.rs:728/944 compare passed-in abilities (different shape) and could share a static helper | Trivial remainder |
| R13 | **cost_limit+operator pair extraction**: `.cost_limit_operator_any()` appears at 22 sites across 10 files, half paired with `cost_limit_any()` into ChoiceBuilder::cost_limit(limit, String). Add `ChoiceBuilder::cost_limit_from(&AbilityEffect)` + swap ~11 builder-style sites; the as_deref comparison sites stay as-is | Mechanical |
| R14 | **Bilingual zone-label pairs**: zone_label + zone_label_ja fetched together at ~32 sites (describe.rs ×26, look.rs ×4, move_cards.rs ×2); one `zone_labels(zone) -> (&str, &str)` helper halves the call sites | Low risk |
| R15 | **filter_from_parts_full positional-None calls**: 6 callers pass up to 5 trailing Nones by position — builder pattern or struct-update would kill the silent-misordering class (CLEAN-G15/D20 already bit once) | R6 adjacent |
| R16 | **Name-normalization sprawl (survey 08-24, partially done)**: `dedupe_by_normalized_name` helper now replaces 3 distinct-name blocks; `MoveSourceContext::player_mut` replaces the use_p2 pick at 4 clean sites (stage/selected_cards/revealed keep inline picks — interleaved gs-field borrows). REMAINING: two opposite-direction `normalize_card_no` impls (card.rs uppercase/fullwidth→half vs deck_parser.rs +/!→FULLWIDTH — document or unify); `normalize_member_name` (phases.rs) strips spaces; ~8 inline normalize_name-contains checks in condition.rs/state.rs/score.rs/card.rs should route through util's card_matches_name_fragments | Medium |
| R17 | **sort+dedup pairs** at cost.rs:945, display.rs:1310, web_server.rs:1543, live.rs:2060/2219 → one `sort_dedup_ids(&mut Vec<i16>)` helper | Trivial |

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

### Post-R1 agenda (from 08-25 trigger-scoping work)

R1 (movement-tracking unification) stopped being theoretical this session:
the S2/S3 scoping implementation had to be threaded through five overlapping
channels and three guard lifetimes, and every misstep traced back to the same
root. What R1 must absorb, concretely:

| # | Pain (observed 08-25) | Consequence today | Post-R1 shape |
|---|---|---|---|
| P1 | Movement recorded in **5 places** (`recently_moved_cards`, `batch_movements`, `turn_area_movements`, `position_change_events`, entry `trigger_moved_cards`/`snapshot_movements`) with different lifetimes | Every consumer picks the wrong one eventually; TAS windows gated on `recently_moved.is_some()` silently skip pure-position-change batches | One append-only event log; all queries derived |
| P2 | Dedupe guards (`this_batch_triggered_ability_ids`, `just_completed_ability_key`) cleared at **inconsistent sites** (process-loop end vs RWC windows vs never) | Stale key **permanently suppressed** an each_time watcher's future triggers (found via S2 pin); clearing too eagerly double-fires | Guard = (ability key, event id) pairs; expires by construction when its event ages out of the log |
| P3 | Cause-player attribution exists **only on MovementEvent** | Opponent-effect scoping (S1/S2) implementable for stage moves + energy placements only; appearance/state-change families stay blind | Add causer to every emitted event kind at emit time |
| P4 | Energy placement tracked as a **bool flag** (`last_energy_placed_by_effect`) + player string | No per-card/per-count info; snapshots must be plumbed manually through entries | Query the log like any other zone change |
| P5 | `entry_snapshot_*` getter proliferation (~6 ad-hoc filters over `snapshot_movements`) | Each new feature adds another bespoke getter; filters drift | Typed query API over entry snapshot (by zone pair, by card, latest-n) |
| P6 | Position-change executors bypass `push_movement_event` in some paths (opponent-facing drags recorded nothing until hooked) | Cross-side forced moves invisible to ALL trigger classes, not just S2 | Single emission point enforced; executor cannot forget |

Sequencing note: `fire_opponent_cause_watchers_for_move` /
`_energy_watchers` and their turn-scoped dedupe
(`opp_cause_fired_keys`) were deliberately written against
`push_movement_event` only, so they survive the unification untouched —
afterwards they reduce to "query log for events caused by opponent matching
marker", and P2/P3 dissolve entirely.

Also blocked-on-R1 (smaller):
- Per-move identity for each_time dedupe (today: turn-scoped set ⇒ marked
  watchers arm at most once/turn even across distinct moves)
- Simultaneous two-player trigger ordering pins need stable event ids to be
  assertable deterministically

Additional finds from the same trench work (not R1-gated):

| # | Finding | Why it matters |
|---|---|---|
| A1 | ~~`condition_is_event_based` does not recurse into composites~~ **CORRECTED 08-25**: recursion into `Condition::Compound` exists (abilities.rs ~306); the multi-fire observed during S2 debugging traced to **guard lifetimes (P2)**, not classification | No code gap — kept as a pin: `trigger_scope` + combination tests now cover composite watchers' enqueue counts |
| A2 | Debug output has **two independent switches**: gated logs check the `ABILITY_DEBUG` atomic (set by `TestGame::new` unless `--test-threads` appears in argv), plain `log::debug!` needs `RUST_LOG`. Running with `--test-threads=1` silently disables half the diagnostics | Debugging sessions lose the most valuable traces for no reason; derive one switch from `RUST_LOG` |
| A3 | The `--test-threads` argv sniff itself is brittle (`-j`, env vars, CI runners all bypass it) | Same fix as A2 |
| A4 | `effect_only` flag on MovementEvents is set inconsistently by callers (cost discards false, hook placements true, some executors unclear) | 「カードの効果によって」 scoping depends on this bit; worth a per-caller audit when R1 lands |
| A5 | Test choice-answering has three channels (`select_indices` / `select_option` / `select_generated`) with overlapping failure modes — wrong channel yields silent no-ops ("Unknown source position") or raw-index garbage | One `answer_choice(game, idx)` dispatcher keyed off the pending choice's variant would kill a recurring test-bug class |
| A6 | Corpus smoke parallelization swaps the process-global panic hook for its duration; panics in OTHER tests running concurrently in that window print nothing | Narrow race, cosmetic — note for whoever touches the smoke harness |

Also open, cheap: corpus sweep for additional 「自分のカードの効果によって」/
「〜でも発動する。」 shapes — **swept 08-25, none found** beyond the pinned
S1×4 / S2×7 sets; remaining 効果によって hits are condition/restriction-side
clauses already listed above.

| # | Finding | Why it matters |
|---|---|---|
| A7 | `gained_card_abilities` registration **stacks duplicates** — refiring the same LiveStart/LiveSuccess grant re-pushes an identical gained ability and re-applies its immediate score | Unreachable through phase-driven dispatch today (just_completed/this_batch guards), but one guard regression away from score inflation. Hardening: make registration idempotent per (card_id, full_text) or per (source_card, full_text) when distinct sources must stack |

QA-ruling coverage updated: Q37/Q138/Q139 pinned in
`qa_rulings_pins_test.rs` (Q139 under-energy follows area moves; Q138
under-energy unpayable; Q37 single-resolution per timing). Remaining
uncovered rulings are procedural definitions (Q31/Q33/Q34/Q39) with no
engine decision to assert.

### Continuous (parallel track)
- ~~Test-gap burn-down: prioritize the 176 unreferenced abilities~~ **DONE at L0** —
  936/936 abilities on 771/771 cards referenced (batch 55 closed the list). Remaining
  quality axes: inferred-depth tail (48×L1, 28×L2), the 08-25 scoping/conjunction
  directive above, backfill qa_data `related_cards` (82 rulings; 8 rulings still
  test-unreferenced: Q29/Q31/Q33/Q34/Q37/Q39/Q138/Q139);
  one replay/determinism test; one engine↔web_ui choice-contract pinning test.

### Current work directive (user, 08-25) — ACTIVE

**Theme: 自動 abilities × trigger-source scoping × ability combination.**
The next fidelity frontier is not more single-ability burn-down (L0 gaps = 0); it is
WHOSE card effect is allowed to activate a trigger, and how abilities behave in
combination.

Corpus facts (measured 08-25):

- 75 unique 自動 abilities; all condition-gated; 17 are multi-action sequentials.
- **Trigger-source scoping is a three-arm dimension the parser likely drops today:**

| Arm | Text shape | Uniques | Correct semantics |
|---|---|---|---|
| S1 own-only | 「自分のカードの効果によって、…たとき」 | 4 自動 | fires ONLY when own card's effect caused the event |
| S2 both-sides | 「…たとき。(対戦相手の/相手のカードの効果でも発動する。)」 | 7 自動 | fires from EITHER player's card effect |
| S3 unscoped | plain 「エリアを移動したとき」「〜が置かれたとき」 | majority of 自動 | check rules.txt: default scope = ? |

- Notable S1 case watches the OPPONENT's stage (「自分のカードの効果によって、相手のステージの…
  メンバーがウェイト状態になったとき」 PR-family draw): event location ≠ effect owner side.
- Condition-side cousins (same scoping axis, different clause family):
  「相手の効果によってはウェイトしない」 restriction;
  「自分の『Liella!』のカードの効果によって…移動していた場合」 LiveSuccess score gate;
  「自分の『虹ヶ咲』のカードの効果によって…アクティブにしていた場合」 LiveStart score gate.
- Combination surface: **405 cards carry >1 ability** (364×2, 41×3); 自動 is ab#1 on
  16 cards; S2/S1 triggers interleave with OPPONENT chains by definition.

Work items, in order:

1. **DONE 08-25 probe — parser state**: the scoping clauses ARE decoded.
   - S1: `trigger_event.self_effect_only = true` survives into Condition; engine
     CONSUMES it in condition/state.rs (area-move + energy-placed arms read
     `get_self_effect_only()`). Machinery exists — untested.
   - S2: the parenthetical survives verbatim as `effect.parenthetical:
     ["対戦相手のカードの効果でも発動する。"]` but NOTHING in the engine reads it for
     trigger dispatch (only unrelated activation-position uses exist). S2 triggers
     today behave however the unscoped path behaves — likely own-side or
     any-source by accident. **This is the primary gap.**
2. **Engine arm for S2** end-to-end per AGENTS.md: resolve event cause-side at
   trigger dispatch (MovementEvent already carries `cause_player_id` +
   `cause_card_id`; energy-placed events carry cause too), gate on
   effect.parenthetical containing 発動する+相手 → allow opponent-caused;
   default unscoped (S3) semantics decided FROM rules/rules.txt + qa_data.json,
   not from current behavior. Then bytecode regen + golden-diff.
3. **Test matrix per scoping arm** (pos/neg/edge, reusing batch idioms):
   - S1: own-effect fires ✓; OPPONENT-effect same event MUST NOT fire ✗; own
     non-matching event ✗; turn1-limit consumed by own then opponent event ✗.
   - S2: own ✓ AND opponent ✓; non-matching ✗; turn-limit shared across sides.
   - S3: pin whatever the rules-corpus default turns out to be (consult
     rules/rules.txt + qa_data.json before writing expectations).
   - Cross-side event location: own effect waiting an OPPONENT member (S1 draw) —
     pos, plus own-member-waited-by-opponent neg.
4. **Combination/conjunction matrix** (supersedes old directive item 3):
   - auto(ab#1) + sibling 起動/常時 on the same card: firing one must not consume the
     other's use_limit or leave stale temporary modifiers (16 ab#1-auto cards first).
   - S2 auto firing inside an OPPONENT's ability chain: queue ordering, choice
     ownership (who answers the prompt), recalc timing.
   - two-player simultaneous-trigger ordering pin (already wanted by Continuous).
5. Feed every finding back as parser/engine tickets per the 08-24 loop below.

### Current work directive (user, 08-24) — standing rules
1. **Finish the missing tests, one by one, with care** — every new test covers positive AND
   negative cases plus edge cases (empty zones, boundary counts, wrong-type/wrong-name
   exclusions, sibling-ability interference). Take inspiration from the existing corpus of
   ~450 test files; follow `engine/tests/WRITING_TESTS.md`.
2. **Text-similarity search FIRST.** Before writing a new test, grep `cards/abilities.json`
   and the existing suite for abilities with similar Japanese text (same clause shapes:
   「〜かぎり」, 「そうした場合」, 「エールにより公開された…」, per-unit counts, name-substring
   gates). A sibling ability with an existing test gives the setup idiom, the choice-drain
   pattern AND a known-good engine path; adapt it instead of deriving from scratch. If the
   sibling exists but is untested too, they share a batch.
3. **Then write conjunction tests** — abilities working in combination (two constants stacking,
   trigger ordering between two players, temporary expiry vs re-registration, cost reductions +
   optional costs in one play chain).
4. **Consult the rules corpus per card** — `cards/qa_data.json` rulings and `rules/rules.txt`
   are part of the spec alongside the Japanese text; when they disagree with behavior, that is
   a bug.
5. **Never assume — observe.** Run the specific test with
   `$env:RUST_LOG="debug"; cargo test --test run_all <name> -- --nocapture` and read the
   condition-verdict/trace output before concluding anything about engine internals.
6. **Feed findings back into refactoring** — tests that expose parser/decoder/handler weakness
   become refactor tickets (P-items/R-items above); fix producers, not tests.

Immediate targets (from the C1 decode-audit triage):
- [x] PL!N-bp4-010-R＋ ab#1 — 「それと同じカード名のカードが成功ライブカード置き場にある場合」
      reference_card gate: **real over-trigger bug found + fixed** (heart04 granted on
      different-name success-zone card); pos/neg/empty pins in decode_audit_behavior_pins_test.
      Replaces the assertion-free issue7_mifune_live_start_select_and_check.
- [x] PL!HS-sd1-018-SD ab#0 — **real over-trigger bug found + fixed** (score+1 fired on ANY
      waitroom live card; condition-side card_names was dropped at decode). Pins: pos,
      wrong-name neg, 2-member neg, empty-waitroom neg, 104期Ver substring edge.
- [x] PL!SP-bp2-001-R＋ ab#0 — negative case pinned: no invalidatable Liella! → no recovery.
- [x] PL!N-bp7-011-R＋ ab#1 — already pinned by bp7_mia_play_cost_reduction_test (3 tests).

| 08-24 | 7ca499f9 | Decode-audit follow-through: **two real over-trigger bugs fixed** (bp4-010-R+ ab#1 heart04 name gate; sd1-018 waitroom card_names gate) via ConditionCommon.card_names/reference_card + regenerated decoder; decode_audit instrumentation made generator-emitted + CI freshness check; 9 gameplay behavior pins (mifune ×3, dream believers ×5, kanon neg ×1). Suite **2688/0**, depth=none →113 |
| 08-24 | 977cb135 | Batch 30 (13 tests) + **third real bug fixed — has_moved/not_moved inversion**: the compiler stripped `"type":"has_moved"` as a tag without preserving it, so `Condition::Movement{movement:None}` classified EVERY moved-gate as NotMoved (gate inverted: fired when standing still, blocked after moving). Fix at all three layers: compile_abilities.py injects `movement` field from type tag (mirrors or_condition→operator precedent); enums classification maps `Some("not_moved")`→NotMoved; deep-compare oracle normalizes moved types before serde. Found via batch-30 千砂都 test drawing 2-without-move / 1-after-move — exactly inverted. Tests: 葉月恋 dual energy thresholds 5/6/7/8 boundary ×4, 桜坂しずく hand-cost−2 success-zone gate pos/neg/wrong-group, sweet&sweet holiday μ's-success-zone draw pos/neg/wrong-group, 千砂都 area-move draw pos/neg. Suite **2700/0**, covered cards 659→663, depth=none →109 |
| 08-24 | 29223c7a | Batch 31 (11 tests) + **bugs 4 & 5 fixed**: (4) cost-range filter 「コスト4以上9以下」 was never enforced on revealed-cards retrieval — CardFilter gained `cost_limit_max` + matches() bound + resolver wiring (cl1-009 boundary pins: 4✓ 9✓ 10✗ wrong-group✗); (5) optional stage-move costs couldn't be declined — `resolve_from_stage` hardcoded can_skip=false and nothing gated the effect; now mirrors the optional-energy pattern (pay/skip gate → re-execute with optionality stripped / skip sets optional_cost_result=false). fuyumari tests updated for the new gate sequence. Suite **2711/0**, covered cards 663→668, depth=none →104 |
| 08-24 | d97ec486 | Batch 32 (8 tests) + **bug 5, third instance**: resolve_from_energy_deck ignored optionality (HOT PASSION!! auto-placed + always drew for opponent). Fix mirrors deck_top gate precedent; handle_optional_cost_payment marks decided ONLY for sequential steps with an EnergyDeck first action (PlaceEnergyUnderMember owns its own gate — marking it double-executed). Tests: 夏めきRain gate pos/neg, せつ菜 opponent-success pos/neg, 希 look-3 reorder, Aqours x2 to deck bottom (non-Aqours stays), HOT PASSION accept/decline. AGENTS.md: truncation ban + debug-env scope. Suite **2719/0**, covered cards 668->673, depth=none ->99 |
| 08-24 | b827fc23 | Batch 33 (8 tests): waitroom-debut gates — bp6-016-N look3/take1/rest-to-waitroom + bp6-011-N draw2/discard1, gated on `card_appearance_source=="discard"` (simulated via record_card_appearance; no corpus discard→stage deploy effect exists to exercise the real pipeline). Edges: hand-debut neg, unrecorded fail-closed, deck<look-count, single-card deck (short draw + unconditional discard), empty deck+hand noop — all pass first try, gate is sound. Suite **2727/0**, covered cards 673→675, depth=none →97 |
| 08-25 | 3cf2f6fa | **Bugs 6–9 fixed via batch 34** (5 tests, PR-032/PR-044 deficit mill): (6) parser kept static count=1 beside waitroom_count_below_base dynamic_count → mill moved 1 not the shortfall (fix: drop count on canonicalize); (7) 「これにより控え室に置いたカードの中から」 parsed source=hand (fix: SOURCE_PATTERN → those_cards); (8) those_cards sequential sub-actions fell back to whole-waitroom scan (fix: fall back to ability's own earlier moves); (9) optional plain-dest single-candidate recovery auto-executed + skipped source removal = dual-zone leak (fix: declinable SelectCard for plain-dest optional; dtob keeps Q252 flow). Plus global test watchdog RABUKA_TEST_TIMEOUT_SECS=300. Suite **2732/0**, depth=none →95 |
| 08-25 | 3edbdbfa | **Bug 10 (C1-class) fixed via batch 35** (8 tests, original-value hearts): get_original_value() on Condition::Location read only legacy sub_checks, bare-key original_value in ConditionCommon was invisible → 「元々持つハートより多い」 gates mis-evaluated; also resolve_zone_card_count hardcoded respect_original_value=false, and parser emitted '>=' making per-card check non-strict. Fixes: getter falls back to common; respect flag honored; 'より多い' → '>' at parse. Tests: PR-028 Echoes Beyond pos/neg/edges + pb1-029 全方位キュン♡ tiered draw & need-heart00 −2. Suite **2740/0**, depth=none →93 |
| 08-25 | fc44d9aa | **Bug 11 fixed via batch 36** (5 tests): handle_play_member_to_stage cleared ALL baton tracking per-play, so same-turn baton plays never accumulated — 「このターン中にバトンタッチして登場したメンバーが2人以上」 (Mirage Voyage / ココン東西) could never fire after a 2nd play. Fix: clear_play_scoped_baton_touch() resets play-scoped fields only; turn-scoped arriving ids + counts clear at Active-phase boundary. Old pin updated to corrected split. Suite **2745/0**, depth=none →91 |
| 08-25 | ebfc2895 | **Bug 12 fixed via batch 37** (8 tests): apply_blade_resource empty-targets fallback granted blades to the ACTIVATING card even for exclude_self effects — 「ほかのメンバーはブレードを得る」 boosted Honoka herself when nobody else was staged. Fix: fallback requires exclude_self unset/false. Tests: bp3-006-R success-zone per-card +2 blades ×3 edges; pb1-010-R other-member blades incl. lone-member noop; SP-bp4-010-R self-wait activation placing WAITED energy + empty-deck noop. Suite **2753/0**, depth=none →88 |
| 08-25 | a51e6907 | **Bug 13 half-fixed via batch 38** (7 tests + 1 ignored): 「エネルギーをエネルギーデッキに置いてもよい」 costs had no pay/skip gate — declining impossible, gated score+1 fired anyway. Fix: Zone::Energy in optional_gate_source + gate in resolve_from_zone Energy arm + energy arm in handle_optional_cost_payment. REMAINING (ignored test documents it): on ACCEPT the energy movement is lost to cost-resume interleaving ({E} activation cost clobbers the selection) — R10/R11 ticket. Tests: bp5-013-N heart04 aggregate exact-4/3/empty; bp7-027-L decline neg, behind neg, LiveSuccess waited-energy placement. Suite **2759+1ign**, depth=none →85 |
| 08-25 | 1364d9c3 | Batch 39 (4 tests): chosen-color heart transformation - pb1034/pb1036 SelectHeartColor -> set_heart_type(selected). Protocol verified end-to-end: select_option(idx) is the required answer channel (select_indices silently defaults to option 0 - documented footgun). All three pb1034 colors + twin heart06 pinned, bystander isolation. Suite **2763/0+1ign**, depth=none ->83 |
| 08-25 | a73f5b00 | Batch 40 (2 passing + 3 ignored tickets): bp3-021-L waitroom recovery & sd1-009 reveal-recovery expose **bugs 14/15** - the implicit 「そうした場合」 gate (was_moved/was_selected proxy, Part 4.1) mis-evaluates when the optional move defers to a selection prompt: accept path blocks the blade despite successful placement; decline path applies it anyway; reveal->deck_top_or_bottom drops the card on resume. Documented as ignored tests with reasons = F4/Wave-3 Consequent-node refactor ticket. Suite **2765/0+4ign**, depth=none ->81 |
| 08-25 | e5dcb2cf | **Bug 16 fixed via batch 41** (6 tests): no_ability_type ability-filter matched nothing, so 星空凛 bp4-014-N's gate passed unconditionally. Three layers: parser captured icon FILENAMES not JA alt-texts; condition lacked location (defaulted to stage); engine NoAbilityType arm checked only the activating card instead of scanning the zone for a card lacking the triggers. All three fixed. Tests: bp5-017-N heart05 aggregate twin ×3; bp4-014-N pos/neg/empty ×3. Suite **2771/0+4ign**, depth=none ->79 |
| 08-25 | 36e724ce | Batch 43 (4 tests, all green): bp4-011-N/bp4-017-N center-area blade grants behind self-wait cost (area scoping pos + non-center neg, twin +1 variant); bp6-013-N success-zone score-sum >=6 gates mu-s live retrieval (pos + empty neg). Suite **2781/0+4ign**, depth=none ->73 |
| 08-25 | 6bfc072a | Batch 44 (5 tests, all green): pb2-036-N right-side / pb2-037-N left-side debut draw2+discard2; bp7-023-N self-wait activation draw2+discard2 (both drawn reach hand, both discards land in waitroom); bp7-013-N KALEIDOSCORE-trio constant heart06+blade pos/neg. Suite **2786/0+4ign**, depth=none ->69 |
| 08-25 | 9b1261c5 | Batch 47 (5 passing + 1 ignored): pb2-047-L all-Liella gate waits enemy cost<=2 member (pos/neg); bp5-010-N mill3 + A-RISE retrieval (pos/no-ARISE neg); bp7-022-N empty-deck noop pinned. Accept-path energy->deck movement still lost to cost-resume clobber (bug 13 family) - ignored test w/ reason. Suite **2800/0+5ign**, depth=none ->58 |
| 08-25 | b4746b59 | Batch 48 (5 passing + 1 ignored): cl1-002-CL optional-energy DOLLCHESTRA retrieval (accept/decline); cl1-008-CL self-to-waitroom activation retrieving Hasunosora card; bp5-026-L below-threshold neg. **Bug 17 documented**: stage-scoped current-heart-total aggregation over a group unimplemented (parses to plain member-count) - F4 ticket. Suite **2804/0+6ign**, depth=none ->55 |
| 08-25 | eab5787a | Batch 49 (5 tests, all green): cl1-010-CL cost>=10 Hasunosora +2 blades (pos/cost-5 neg); cl1-004-CL binary choice debut both arms (mill-3 / wait-enemy cost<=2); bp6-030-L draw1+hand-discard1 deterministic end state. Suite **2809/0+6ign**, depth=none ->52 |
| 08-25 | 5b283478 | Batch 46 (6 tests, all green): PR-029-PR optional-energy -> heart01 (pay/decline); bp4-024-L mu-s member +1 blade (pos/Aqours-only neg); bp5-013-N mill-3 all-member -> +2 blades (pos/live-among neg). Suite **2795/0+4ign**, depth=none ->61 |
| 08-25 | 0ccd170d | Batch 45 (3 tests, all green): bp7-025-L named grant - staged Chisato +1 blade (other-member neg); bp7-019-N >=3 staged 5yncri5e! -> waitroom live retrieval (pos/neg). Note: Chisato stored name has a space. Suite **2789/0+4ign**, depth=none ->65 |
| 08-25 | 285b9ab3 | Batch 50 (6 passing + 2 ignored): **Bug 18 half-fixed** - resolve_from_revealed_cards dropped cost_limit_operator so 「コスト9以上」 retrieval offered below-threshold cards (also fixed same pattern in resolve_source_revealed_cards). Pinned: sd2-005-SD2 specify-color +2 (choice protocol); pb2-030-N transform twin; cl1-012-CL tie-score pos WITH cost>=9 filter. Bug 19 documented: tie-score comparison itself unenforced (9 vs 8 still retrieved) - ignored test w/ reason, F4 ticket. Suite **2813/0+7ign**, depth=none ->50 |
| 08-25 | ddead2d3 | Batch 51 (3 tests, all green): pb1-016-R named-member (朝香果林) look2 fetch; bp4-006-R success-zone score>=3 gates look5 mu-s fetch (pos/score-0 neg). Suite **2816/0+7ign**, depth=none ->48 |
| 08-25 | 135c39c4 | **Bugs 14, 15, 19 FIXED**: (14) resolve_gain_resource blade_targets no longer lets preceding step selection leak into fresh-filter targeting + new deferred_conditional_gate flag on AbilityResolver consumed by choice answer handler (empty/skipped answer drops remaining actions); (15) Reveal cost arm in pay_cost lacked group_names filter — every hand card was reveal-eligible; (19) test setup wrong not engine bug — calculate_live_score needs player.stage_hearts populated via calculate_stage_hearts. omoi/ruby/cl1012 all pass un-ignored. Suite **2820/0+3ign** |
| 08-25 | f6a762ac | **Bug 17 FIXED**: parser now emits aggregate=total on group_condition for 「ハートの総数」 patterns — engine dispatches to sum_group_hearts_in_stage (base heart sums of matching Liella! members via series matching). Test setup also fixed (was calling recalculate_constants instead of firing trigger). Suite **2821/0+2ign**, depth=none stays 48 |
| 08-25 | 7f0a55bc | **Bug 13 FULLY RESOLVED** — energy zone→deck cost auto-take: when resolve_from_zone dispatches Energy with destination=energy_deck, takes from zone end directly (fungible). Pay/skip gate still fires on first pass for optional; auto-take fires on re-entry. wien_q262 updated (no intermediate prompt expected). **ZERO ignored tests** — all bugs fixed. Suite **2823/0+0ign**, depth=none ->48 |
| 08-25 | 3efc9aef | **Batch 52 + bug 20 fixed end-to-end** — parser gap (bp7-023-L tiered energy comparison) FIXED not documented: new _try_energy_ahead_alternative handler emits sequential[move, conditional_alternative] with comparison_type=energy_relative; engine ComparisonType::EnergyRelative + evaluation branch ((opp_active-self_active) vs count). Also resolver fall-through fix: activation_condition_parsed no longer early-returns past the main condition gate (bp4-017 family was granting unconditionally). Batch 52: 17 tests / 8 abilities. Suite **2840/0+0ign**, depth=none ->40 |
| 08-25 | f560c998 | **Hard tier (tests-first)** — 15 tests written from printed text for the 4 riskiest abilities, then fixed: 227 cost-mirror (parser set_from_reference + Operation variant + COST_SELF branch), 726 has_moved group filter, 840 choice group-propagation, compiler negative-int desync bug found & fixed. Suite **2855/0+0ign**, depth=none ->35 |
| 08-25 | c37ccd95 | **Batch 53** — look-and-select family: 14 tests / 9 abilities (group-filter looks lilywhite/5yncri5e!/DOLLCHESTRA/Liella, named-member 璃奈/嵐珠 look2, cost-gated looks 起動+opt登場, opt{E}{E}{E} live retrieval, mill3->live recovery). Pure idiom reuse, zero engine changes. Suite **2869/0+0ign**, depth=none ->25 |
| 08-25 | (b54) | **Batch 54** — yell-revealed retrievals x4, state/formation x4, dual-trigger + specify-color twins: 12 tests / 11 abilities. Suite **2881/0+0ign**, depth=none ->15 |
| 08-25 | 4eec599d | Batch 42 (6 tests, all green): bp5-015-N all-six-colors collective gate (pos via Honoka+Kanan coverage, missing-colors neg); bp7-015-N exactly-3 CatChu! + optional energy -> draw (pos/neg); bp7-018-N optional live-card discard cost -> look5/take1 (accept + decline). No engine changes needed. Suite **2777/0+4ign**, depth=none ->76 |
| 08-25 | (b55) | **Batch 55** (parallel session) — final depth=none burn-down: s1013/s1014 look4 debuts, PR-017 self-exile 起動, N-bp5-028 stage-heart02 live gates, SP-bp5-021/023/024 lives, HS-pb1-020/026, HS-bp6-028. During bring-up: encoding corruption of the new file repaired via cp932 round-trip + Edit-tool rewrites; 3 test bugs fixed against printed text (card_no needs fullwidth ＋ `PL!S-bp5-001-R＋`; SP-bp5-023 revealed pool must hold a live with an actual score icon — `has_score_icon()` reads special_heart.score, Next SPARKLING!! has none; SP-bp5-024 rewritten to printed semantics — moved-members gain the chosen heart, NOT live-scales-by-success-zone-count) |
| 08-25 | (perf) | Corpus smoke test parallelized (std::thread::scope worker pool, deterministic chunk order): every_card_executes_without_panicking 1.93s → ~0.53s; full-suite wall ~3.6s → ~1.9s. Remaining per-test >1s entries are just first-wave attribution of the one-time ~1s DB load (OnceLock), not real work |
| 08-25 | (b21fix) | **Bug 21 fixed end-to-end — under_member host targeting**: N-bp3-025-L Awakening Promise's 「そうした場合、そのメンバーは…」 gain targeted EVERY stage member matching the card_type filter (correct only by accident when one member staged). New `GameMods.last_under_move_host_ids` recorded by `move_from_under_member`, consumed by `resolve_gain_resource_targets` for Heart+per_unit+per_unit_type=energy_deck, cleared in clear_effect_tracking; guard test awakening_targets_energy_owner_member_only (two staged members each holding under-energy — only the owner gains). Also REVERTED a parallel-session `GR_SELF_PER_UNIT` early-return in misc.rs that landed self-targeted per-unit heart gains on the activating LIVE card — contradicts printed text for both N-bp3-025-L (そのメンバー gains) and SP-bp5-024-L (moved members gain); its [GAIN_MULT] debug log kept. If reintroduced, awakening_targets_energy_owner_member_only is the failing guard. Suite **2895/0**, wall ~1.9s |
| 08-25 | (b56) | **Batch 56 — ability-COMBINATION tests** (`ability_combination_test.rs`, 7 tests): same-card sibling chains per the 08-25 directive. bp5-111-R 起動 position-change → 自動 waits opponent blade≤2 (pos + empty-stage neg); HS-pb1-003-R 登場 hand-discard → 自動 heart01+blade per discard (pos + no-discard neg, hand accounting exact); SP-bp7-005-R＋ 自動×2 cascade off debut (deck→zone waited placement IS the own-effect energy_placed event arming ab#1; ターン1回 respected); N-bp3-005-R＋ shared-counter pair — playing it as 3rd debut fires draw-to-five AND LiveStart grants 常時 score+1 (asserted via recalc + p1_constant_total_score_bonus), with a threshold-divergence pin at 2 debuts (auto silent / constant grants). Refactor: `fire_trigger` hoisted to tests/helpers (was copy-pasted in batch55). Lesson: multi-ability tests must budget EACH ability's cost/turn-limit — give_energy must cover play costs + activation costs together. Suite **2902/0**, wall ~1.8s |
| 08-25 | (s2) | **S2 trigger-source scoping IMPLEMENTED** — 「(対戦相手のカードの効果でも発動する。)」 now works, driven by REAL two-seat plays (`Side`/`set_active_side` helpers route P2 through `execute_main_phase_action`; the engine pipeline was already active-player-aware — only test helpers were p1-hardcoded). New `fire_opponent_cause_watchers_for_move`, called from `push_movement_event` on stage→stage moves whose cause ≠ moved card's owner; marker via new `AbilityEffect::fires_on_opponent_effects()` accessor (parenthetical was decoded but dead). Turn-scoped dedupe `opp_cause_fired_keys` shared by hook + generic TAS prevents double-fire per move. Tests: `trigger_scope_test.rs` 4-test matrix incl. the parenthetical pin (opponent-caused move fires +2 more) and S3 default-scope pin (no-parenthetical stays silent on opponent cause). DX: `scan_autos_both` helper hoisted. **Known limits**: hook covers stage→stage only (energy_placed arm pending); dedupe is turn-scoped so a marked watcher arms at most once per turn. Suite **2906/0**, wall ~1.9s |
| 08-25 | (s1s2) | **Trigger-scoping matrix COMPLETE (6 pins)**: energy-placed S2 arm added — `fire_opponent_cause_energy_watchers`, hooked into push_movement_event for opponent-caused zone→energy placements (SP-bp4-016-N heart06; its comparison_condition needs no cause gate at resolution, hook alone suffices); S1 negative pin added (SP-bp7-005-R＋ ab#1 self_effect_only stays silent on opponent-caused placement — existing state.rs:908 check confirmed live). Matrix: S1 own-only ✓neg-opp / S2 both-sides ✓opp / S3 unscoped ✓own+✓silent-opp. Remaining known limit: marked watchers arm at most once per turn (dedupe is turn-scoped; per-move identity would need richer keys). Suite **2908/0**, wall ~1.8s |

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

---

## Fresh Audit  (`FRESH_AUDIT_2026-08-19.md`)

# Fresh Audit — Engine + Parser + Japanese Ability Texts (2026-08-19)

> No prior MDs consulted. Findings from fresh code reads + Python text-mining scripts over `cards/cards.json` (2526 cards, 2137 raw ability lines) and `cards/abilities.json` (936 unique abilities).

Scripts used (ephemeral, deleted after run):
- `audit_tmp.py` — corpus stats, stranded-field diff, bracket-gap, per-unit/optional/duration coverage, opcode collision check, decoder drift
- `jp_mine_tmp.py` — n-gram verb frequencies after stripping `{{icon|label}}`, raw-vs-parsed gap percentages per phrase

---

## 1. Engine — silent success is the biggest bullshit

### 1.1 `_ => true` / `_ => Ok(())` makes broken cards pass

The engine systematically replaces `todo!` with a permissive default, so malformed or unknown abilities *succeed* instead of failing loudly. The worst spots:

- `engine/src/ability/effect_decoder_gen.rs:207`, `engine/src/ability/condition_decoder_gen.rs:185`, `engine/src/ability/vm.rs:207` — unknown JSON field → `skip_value(); return Some(true)`. A typo'd key in `abilities.json` compiles and is ignored at runtime.
- `engine/src/ability/condition.rs:333-411` → `409: _ => true` — `check_phase_gate` only handles `main / active_phase / live_phase`. Any other `phase` string (e.g. `draw_phase`, `end_phase` if the parser ever emits it) passes unconditionally.
- `engine/src/ability/condition.rs:450-534` — `evaluate_condition` dispatch. Several `Condition` variants have no branch and fall through to the generic verdict path; e.g. `AlwaysTrue`, temporal `skip_phase_gate`, `yell_trigger` outside `Movement` — decoded but never checked.
- `engine/src/ability/cost.rs:956` / `164`: `pay_cost_inner: _ => Ok(())`, `validate_cost: _ => Ok(())` — unknown `ActionType` as cost is *free*.
- `engine/src/ability/dynamic_count.rs:129: _ => 0` — unknown `count_type` returns 0 cards/energy instead of error, so `draw X` silently draws 0.

**Fix:** add `debug_assert!(false)` + `log::warn!` and return `Err` in `#[cfg(debug_assertions)]` for every catch-all. At minimum make bytecode decoder emit a warning count that `cargo test` asserts is zero. Currently CI can go green with stranded fields.

### 1.2 Opcode collisions in the contract

`cards/ability_schema.json` reuses opcodes across unrelated actions:

- `0`: `sequential` vs `choice`
- `20`: `move_cards` vs `rotation`
- `21`: `pay_energy` vs `set_cost`
- `28`: `shuffle` vs `repeat_procedure`
- `35`: `set_cost_to_use` vs `set_blade_count`
- `41`: `conditional_alternative` vs `choose_required_hearts`

`engine/src/ability/enums.rs` maps opcode → `ActionType` via `from_u8`; collisions mean one of the pair can never be round-tripped through bytecode. The bytecode compiler (`cards/compile_abilities.py`) works around this by matching on *name*, not opcode, but any future `no_std` deserializer that dispatches on opcode alone will alias.

**Fix:** assign unique opcodes (or make opcode an internal detail and dispatch purely on `EffectKind` string in bytecode — already what `EffectKind::from_action` does). Remove `opcode` from the public schema or make CI assert uniqueness.

### 1.3 `ModifyYellSource` and `Custom` are compiled no-ops

- `engine/src/ability/effects/mod.rs:404` `ModifyYellSource => Ok(())` with comment "applied by `refresh_yell_sources` during `recalculate_constants`". The one-shot path does nothing, so a card that says `自分のエールは、デッキの上から行う代わりにデッキの下から行う` (`PL!S-bp7-022-L | 恋になりたいAQUARIUM`) has no effect if checked via `execute_effect` in a non-constant context. Only the `常時` recalc path applies it — fragile split.
- `engine/src/ability/effects/mod.rs:400` `DoNothing => Ok(())` and `misc.rs:130` custom fallback `log::debug!("Unhandled custom action") → Ok(())`. Any parser `custom` silently does nothing and still consumes `use_limit`.

**Fix:** make `Custom` return `Err("unhandled action")` in debug builds; add a test that `abilities.json` contains zero `custom` / `do_nothing` (currently 0 `do_nothing` but the *path* exists and will hide future regressions).

### 1.4 Handler metadata in `ability_schema.json` is stale

Every `actions.*.handler` is a `file:line` string like `effects/mod.rs:287` that drifted after refactors (`execute_move_cards` now lives in `move_cards.rs`/`draw.rs`, not line 287). Nothing validates these. They are displayed in `docs/ABILITY_MATRIX.md` as if accurate.

**Fix:** generate `handler` from `grep -n "pub fn execute_" engine/src -r` or drop the field.

### 1.5 Hardcoded assumptions in hot paths

- `engine/src/ability/resolver.rs:362-372` activation_position check hardcodes `left→0, center→1, right→2` and assumes stage length 3. Aliases `left_side/right_side` only work because of string split normalization; a future `front` position will break.
- `engine/src/ability/resolver.rs:1101-1130` `apply_modify_cost_to_ability_cost` only handles `operation=="subtract" && per_unit && per_unit_type=="group_name"`. Other `modify_cost` ops (increase, `blade_limit`, `non_stackable`) are parsed but ignored.
- `engine/src/ability/condition/state.rs:309,341,384,395,667,878,1004` all fall back to `_ => true` for state checks — unknown `state`/`from_state` values pass.
- `engine/src/ability/cost.rs:648-690` `has_skip_prompt` only treats `PayEnergy`/`ChangeState(self_cost)` as binary prompts — sequential cost with two `move_cards` legs gets auto-paid without a choice.

---

## 2. Parser — priority is order, order is fragile

### 2.1 First-match-wins shadowing (known but still biting)

`cards/ability_extraction/parser.py:1798-2530` `_ACTION_RULES` is documented "order = priority" but there is no numeric priority — insertion order *is* priority. Classic bugs:

- `parser.py:1908-1918` catch-all `move_cards` (`source+destination && "選ぶ" not in t`) shadows `select`. Tests explicitly mark `KNOWN_BUG`: `cards/ability_extraction/tests/test_parse_action.py:49,59,127` expect `select` but get `move_cards`. Fix is to add `exclude_any=["選び","選ぶ","選択"]` or move `select` rule above.
- `parser.py:2226` `choice` (`以下から1つを選ぶ`) vs `2234` `select` (`選ぶ`) — `select` fires first, so `choice` is never reached for texts that contain both phrases.
- `parser.py:2104-2113` generic `置く` catch-all shadows `shuffle+move` and `place_energy_under_member`; only works because `shuffle` rule at `1798` stays above it — invisible invariant.
- `parser.py:1085` `_cost_verb_choice` regex `r"(.*(?:支払う|置く|加える|公開する))か(.+)"` greedy split loses `AかBかC` alternatives beyond the first `か`.

**Fix:** give `_ACTION_RULES` explicit numeric priorities + a `cargo test`-like `--check-order` that fails when a more specific rule is after a more general one (can be computed by subset check on `match` strings). Or at least sort rules by specificity length.

### 2.2 Splitters diverge

- `parser.py:600-629` `split_cost_effect` skips `：` inside `（）`/`()` and `"` but not `『』`/`「」`/`{{}}`. A cost like `「A：B」` splits at the inner colon. Quote depth toggle `quote_depth+=1 if==0 else -1` also fails on `"` nested inside `「」`.
- `parser.py:632-643` `split_condition_action` looks for `場合/とき/なら+、` without paren depth, so `A場合、B場合、C` splits at the wrong `場合、`. It also ignores `たび、` even though handlers use `EACH_TIME_MARKER`.
- `cards/ability_extraction/extract_card_abilities.py:379-387` splits cost/effect on first `：` (`split("：",1)`) *without* paren depth, while `parser.py:split_cost_effect` *does* depth-check. Two different split strategies for the same text — they diverge on `（コストはA：B）効果`.

**Fix:** unify on one `split_cost_effect` implementation; add depth for `「」『』{{}}（）()` and fuzz-test against all `cards.json` lines (assert `split(join(parts))==original`).

### 2.3 Normalization is split-brained

- `parser.py:486-491` `normalize()` only does `'→『』` and `ライブ終了まで→ライブ終了時まで`, never calls `normalize_fullwidth_digits`/`strip_suffix_period`/`normalize_whitespace`.
- `parser.py:1328` `parse_effect` normalizes full-width digits + strips parenthetical + trailing `。`, but `parser.py:1596` `parse_condition` only strips parenthetical — so `３枚以上` in a condition vs `3枚以上` in an effect diverge; `extract_count` then mis-counts.
- `cards/ability_extraction/parser_utils.py:77-83` vs `parser.py:489` — full-width `＋−－` translation tables differ (one maps `−` (U+2212), the other `－` (U+FF0D)) — only one fires depending on path.

**Fix:** single `canonicalize(text)` called at the top of `parse_condition`, `parse_effect`, `parse_cost`, and `extract_card_abilities.extract_trigger`.

### 2.4 Duplicated zone tables

`cards/ability_extraction/parser_utils.py:440-516` `SOURCE_PATTERNS`/`DESTINATION_PATTERNS` vs `parser.py:702-809` `_extract_basic_cost_fields` and `2789-2823` `parse_action` duplicate zone inference. Adding `success_live_zone` to one list but not the other silently diverges (already happened: `success_live_zone` only in `SOURCE_PATTERNS`).

**Fix:** `parser.py` should import `SOURCE_PATTERNS`/`DESTINATION_PATTERNS` from `parser_utils` as single source of truth.

### 2.5 Fields parsed but never consumed

- `parser.py:1373` `activation_condition_parsed` (from `起動できる` parenthetical) is stored as `ActivationConditionParsed` in `engine/src/core/card.rs:848` but engine only checks `activation_position` gate at `resolver.rs:740` — the condition is never evaluated (only debug-printed).
- `parser_utils.py:851` `FieldExtractor` computes `blade_count`/`multiple_targets`/`non_stackable` but `update_dict` never writes `blade_count` — caller `parse_action` drops it.
- `parser.py:735` `cost["all"]=True` for `手札をすべて控え室に置く` — `compile_abilities.py:320` encodes it as bool, but Rust `AbilityCost` has no `all` field (only `AbilityEffect.all`), so bytecode carries an unused key.
- `parser.py:1606-1626` after a handler succeeds, `parse_condition` re-extracts `cost_limit` via `extract_cost_limit` (not `with_operator`) and can overwrite `>=` with `=` .

---

## 3. Japanese text handling — coverage holes with numbers

Raw-vs-parsed gap (from `jp_mine_tmp.py`):

| Phrase | Raw lines | Unique abs with phrase | Parsed with flag | Gap |
|---|---|---|---|---|
| `につき` (per-unit) | 132 | 66 | 57 | **9 (13% miss)** |
| `まで` (up-to / duration) | 575 | 256 | 210 | **46 (17% miss)** — `ライブ終了時まで` often lacks `duration:live_end` |
| `かぎり` (as long as) | 127 | 65 | 63 | 2 |
| `代わりに` (instead) | 16 | 10 | 0 | **10 (100% miss)** — no `replacement/restriction` mapping |
| `として扱う` (treat as) | 17 | 6 | 1 | **5 (83% miss)** — only 1 has `SetCardIdentity/treat_as` |
| `次の〜フェイズ` (next phase) | 17 | 8 | 8 | 0 |
| `以下から1つを選ぶ` | 39 | 19 | 19 | 0 |
| `もよい` (may) | 690 | 276 | 265 | 11 (mostly ok via `optional:true`) |

Concrete misses:

- **`まで` / duration gap 46:** many `〜まで` texts set no `duration` because `_strip_duration_prefix` (`parser.py:292-297`) only strips a *prefix* at `text.startswith(pat)`. A mid-sentence `ライブ終了時まで、〜を得る` does not match and loses `duration`. Compare `modify_required_hearts_global` etc. that correctly use duration as prefix but `gain_resource` with trailing duration fails.
- **`として扱う` 5 misses:** `すべての領域にあるこのカードは『X』として扱う` (`PL!HS-bp2-020-L` etc.) and `必要ハートを確認する時、エールで出たALLブレードは任意の色のハートとして扱う` — currently `is_null:true` notes with no effect. Engine has `SetCardIdentity` but parser only emits it for one of six variants. The note case should be a `常時` restriction/heart-replacement, not discarded.
- **`代わりに` 10 misses, 100%:** `エールをデッキの下から行う代わりに…` (`PL!S-bp7-022-L`) is the sole hit for `modify_yell_source`; the other 9 `〜の代わりに` texts produce no `replacement`/`treat_as` and compile to `custom`/`choice` that does nothing.

### 3.1 Parenthetical notes become `is_null` and are silently dropped

`cards/ability_extraction/extract_card_abilities.py:214-234` lines starting `（`/`(` are appended to previous ability *or* if standalone become `is_null:true` with empty `triggerless_text`. Two abilities in the DB are in this bucket:

- `(必要ハートを確認する時、エールで出たALLブレードは任意の色のハートとして扱う。)` — `PL!HS-PR-010-PR` etc. (6 cards share this)
- `(エールで出たスコア1つにつき、成功したライブのスコアの合計に1を加算する。)` — `PL!HS-bp1-019-L`

Both have real game effect (`need_heart` substitution, score bonus) but engine ignores `is_null` entries entirely. The second one *is* parsed as a parenthetical attached to the previous `live_start` ability when that ability exists on the same card, but standalone on `PL!HS-bp1-019-L` has no preceding trigger and is lost.

**Fix:** don't use `is_null`; parse parenthetical as its own `常時` effect with `ability_filter`/`modify_required_hearts`/`modify_score` per-unit.

### 3.2 Malformed brackets

One card (`rainbow` family) has `『虹ヶ咲」のメンバー` (opening `『` closed by `」`) — `parser_utils.py:362` `GROUP_PATTERN` `『(.+?)』` misses it; the fallback `『([^』」]+)」` at `364` catches it but `check_group_filters.py` and `engine` normalize differently, so the group filter is sometimes `虹ヶ咲` and sometimes `虹ヶ咲」` leftover.

### 3.3 `か` (or) is ubiquitous (1701 lines) but not structural

Texts like `{{heart_01}}か{{heart_03}}か{{heart_06}}のうち1つを選ぶ` are correctly parsed as `choice`, but `コストが10か20のメンバー` (`extract_cost_values` at `parser_utils.py:650`) only handles the narrow `コストがNかM` + `のメンバー` pattern. Other `AかB` lists (e.g. `スコアかコストが高い`) fall through to `compound` with no operator, producing `custom` in one known case:

- `PL!N-sd2-007-P`: `カードを1枚引く。このターン、相手もライブを成功している場合、さらに…` — the `custom` condition with `temporal_scope=this_turn` is the only `type:custom` in the entire `abilities.json`. Root cause: `「このターン、相手もライブを成功している場合」` triggers the *complex condition* path (`これにより/その結果`) at `parser.py:646` but `かつこれにより` guard causes early `return None`, and no other handler claims it.

### 3.4 `すべての領域` / `すべての3領域` handling is ad-hoc

Two abilities (`PL!HS-bp1-003-R`, `PL!HS-bp1-019-L` via parenthetical) use `すべての領域` to mean hand+stage+discard — parser sets `all_areas` stranded field, engine has no handler for it. `ability_schema.json` lacks `all_areas`.

---

## 4. Contract & bytecode drift

- **Stranded fields: 92 keys** appear in `abilities.json` but not in `ability_schema.json`: `ability_filter`, `ability_filter_triggers`, `action_reference`, `all_areas`, `allow_occupied_stage`, `blade_limit_offset`, `cost_comparison`, `conditional_action`, `comparison_target`, `source_position`, `target_event`, `temporal_scope`, `yell_source`, etc. They are emitted by the parser, carried through `compile_abilities.py` (which just serializes whatever JSON is given), and then *skipped* by the decoder (`skip_value`). They are invisible bugs.
- **Decoder drift:** `engine/src/ability/effect_decoder_gen.rs:192` decodes `or_card_types, cost_limit, distinct` for `select` even though `ability_schema.json:1155` `select` declares only `source/target/destination/count/card_type/group_names/exclude_self`. Schema validation would reject the real data; the decoder accepts it — schema is not the source of truth.
- **Stale `handler` line numbers** in schema (see §1.4) mean `ABILITY_MATRIX.md` / `TEST_COVERAGE.md` attribute coverage to wrong functions.

**Fix:** make `cargo test --test run_all` + `python cards/ability_extraction/tests/*.py` + `python cards/test_inventory.py --check` also run a `validate_schema.py` that asserts every key in `abilities.json` is declared in `ability_schema.json` (or explicitly in an `allowlist` for generic `text/type`). Generate `handler` lines from `ctags`/`grep`.

---

## 5. Small but real

- `engine/src/ability/condition/card.rs:648,686,801,832,1018,3430` — multiple `Zone::from_str` fallbacks `return true` for unknown zones. Add a test with a fake zone to ensure it fails.
- `engine/src/ability/look.rs:83` — `matching_count==0` in `look_and_select` moves all `looked_at_cards` to waitroom without choice. No log. Add rule log for this edge.
- `engine/src/ability/effects/state.rs:905-951` `execute_set_cost` defaults to `hand` when `card_type` is not `Live`/`Member`; `energy_card` path silently targets `hand`.
- `cards/ability_extraction/extract_card_abilities.py:379` splits cost/effect on first `：` without depth — differs from `parser.py:split_cost_effect` depth-aware split. Unify.
- `cards/ability_extraction/tests/test_parse_action.py:49,59,127,141` are marked as KNOWN_BUG and still failing; they document real player-visible misparses (`山札から選び` → `move_cards` not `select`).
- `engine/src/ability/condition_decoder_gen.rs:1355` unknown condition variant `return None` → caller treats as "no condition" → ability becomes unconditional. Should at least `log::warn`.

---

## 6. What to fix first (max impact / min risk)

1. **Opcode uniqueness CI** (`ability_schema.json`) — one-liner script, catches aliasing forever.
2. **Decoder unknown-field warning** (`effect_decoder_gen.rs:207` / `condition_decoder_gen.rs:185` / `vm.rs:207`) — count skipped fields and assert zero in tests; surfaces all 92 stranded fields.
3. **Move `select` before catch-all `move_cards`** + fix `split_cost_effect` bracket depth (`parser.py:600`) — fixes the oldest KNOWN_BUG with no engine change.
4. **`代わりに` / `として扱う` coverage** — add 2 `ActionRule`/`EffectPattern` rows + `SetCardIdentity` / `modify_yell_source` promotion; knocks 100% and 83% gaps to 0.
5. **Duration strip for mid-sentence `まで`** — change `_strip_duration_prefix` to `search` not `startswith`, or add `per_unit_type`/`duration` propagation in `_normalize_effect_tree` (`parser.py:1326`).

---

## Appendix — how to reproduce

```powershell
# 1. Verb frequencies (raw JP)
python -c "import json,re,collections; c=json.load(open('cards/cards.json',encoding='utf-8')); cnt=collections.Counter(); [cnt.update([p for p in ['置く','得る','引く'] if p in re.sub(r'\{\{[^|]+\|[^}]+}}','',v.get('ability',''))]) for v in c.values()]; print(cnt)"

# 2. Gap counts (raw vs parsed)
python cards/ability_extraction/tests/test_parse_action.py  # shows 4 KNOWN_BUG

# 3. Stranded fields
python -c "import json; s=json.load(open('cards/ability_schema.json',encoding='utf-8')); a=json.load(open('cards/abilities.json',encoding='utf-8')); ..."
# or just run the audit script that was used:
#   python audit_tmp.py   (ephemeral — recreated from FRESH_AUDIT_2026-08-19.md § "Scripts used")
#   python jp_mine_tmp.py
```

---

## Fixes Applied — Follow-up (2026-08-19)

This section documents what was actually changed after the audit, why the old code was wrong, and why some proposed fixes were reverted.

### Fix 1: Opcode collisions (`cards/ability_schema.json`) — DONE

**Before:** 13 opcode collisions (e.g. `0: sequential/choice`, `20: move_cards/rotation`, `21: pay_energy/set_cost`, `28: shuffle/repeat_procedure`, `35: set_cost_to_use/set_blade_count`, `41: conditional_alternative/choose_required_hearts`, plus `29: restriction/reveal_until_chosen_card`, `17: activation_cost/activation_restriction` etc.). `engine/src/ability/enums.rs: ActionType::from_str` dispatches on *name*, so collisions were invisible at runtime, but any future `no_std` bytecode dispatch on `opcode` alone would alias two actions and make `ability_schema.json` invalid as contract.

**Why wrong:** Schema claimed to be "single source of truth" but violated its own uniqueness invariant. `cards/compile_abilities.py` worked around it by ignoring opcode, hiding the bug.

**After:** Reassigned all duplicate opcodes to fresh values `43..57` (next free after max `42`). Verified `58` actions → `58` unique opcodes. CI should assert uniqueness (`python -c "assert len(set(opcodes))==len(opcodes)"`).

**File:** `cards/ability_schema.json:252` (`choice:0→43`, `rotation:20→44`, `set_cost:21→45`, `shuffle:28→46`, `set_cost_to_use:35→47`, `choose_required_hearts:41→48`, `sequential:0→49`, `restriction:29→50`, etc.)

### Fix 2: Silent decoder swallowing (`engine/src/ability/*_decoder_gen.rs`, `cards/generate_*.py`) — DONE

**Before:** `effect_decoder_gen.rs:207`, `condition_decoder_gen.rs:185`, `vm.rs:207` all did `_ => { bc.skip_value()?; return Some(true); }`. A typo'd key in `abilities.json` (e.g. `cost_limt`) would compile to bytecode, decode would skip the value and *succeed* — the ability would silently do the wrong thing. Same for unknown condition variant `condition_decoder_gen.rs:303: _ => return None` → caller treated as "no condition" → ability became unconditional. 92 stranded fields (keys in JSON not in schema) were carried through `compile_abilities.py` and then dropped without trace.

**Why wrong:** Decoder was designed as permissive for forward-compat, but without logging it became a correctness hole. CI (2349 tests) stayed green while parser emitted ad-hoc fields like `all_areas`, `yell_source`, `temporal_scope` that were invisible bugs.

**After:** Generators now emit `log::warn!("[bytecode] unknown effect field: {}", key)` and `log::warn!("[bytecode] unknown condition field: {}", key)` plus `unknown condition variant` warn. With `RUST_LOG=warn` in CI, any stranded field now surfaces. Follow-up: add `cargo test` that asserts warning count == 0.

**Files:** `cards/generate_effect_decoder.py:312`, `cards/generate_condition_decoder.py:267`, `engine/src/ability/effect_decoder_gen.rs:207`, `engine/src/ability/condition_decoder_gen.rs:185,303`

### Fix 3: `look.rs` silent discard (`engine/src/ability/look.rs:83`) — DONE

**Before:** `look_and_select` with `matching_count==0` did `take(&mut looked_at_cards); waitroom.add_card` for all cards and returned `Ok(())` with no log. A deck-search that filtered for `『Aqours』` but found none would silently dump all 5 looked-at cards to waitroom — player had no feedback why the choice never appeared. Same path swallowed `followup_action` handling.

**Why wrong:** Silent success on edge case made debugging impossible; `ability_queue` tests that checked `choice pool` leaks could not distinguish "no match" from "bug".

**After:** Added `log::warn!("[look] no matching cards among {} looked-at; discarding all to waitroom (effect: {})")`. No behavioral change, but now diagnosable under `RUST_LOG=debug`.

**File:** `engine/src/ability/look.rs:83-96`

### Fix 4: Parser structural fixes — DONE (dangerous but proven safe)

**What we changed:**

* `parser.py:486` `normalize()` now also calls `normalize_fullwidth_digits()` + whitespace collapse. **Before:** `３枚` (fullwidth) in conditions mismatched `3枚` in effects; `extract_count` with `\d+` missed fullwidth, causing `cost_limit`/`count` to be `None` for 12 cards that use fullwidth in `cards.json`. After: both sides canonicalize, counts agree.

* `parser.py:602` `split_cost_effect()` now tracks depth for `「」『』` + `{{}}` in addition to `（）()`. **Before:** a cost like `「A：B」` or `{{center.png|センター}}：効果` could split at the inner colon. In practice no current card has `：` inside `「」`/`{{}}`, so old code was accidentally correct, but the fix is forward-safe and was verified: `2349 passed` after change.

* `extract_card_abilities.py:40,381` now imports and uses `split_cost_effect` instead of naive `split("：",1)`. **Before:** two implementations diverged; a future card with `（コストはA：B）効果` would split differently depending on code path. After: single source of truth.

**Why these were considered dangerous:** Any change to the splitter changes the `cost_text`/`effect_text` boundary for every card. Downstream `_extract_basic_cost_fields` and `parse_condition` compensate for the old boundary; a "more correct" split can shift a card into a different handler and break golden tests. We verified by applying each in isolation + `cargo test --test run_all` (see § Fixes Applied — Dangerous fixes deep dive below) — these three stayed at `2349 passed`.

**Files:** `parser.py:486,602`, `extract_card_abilities.py:40,381`, regenerated `cards/abilities.json` (936 unique, 0 mismatches) + `engine/src/ability/abilities_gen.rs`

### Fix 5: `として扱う` ALL-blade parenthetical — DONE (narrow, safe)

**Before:** `cards/ability_extraction/extract_card_abilities.py:214` treated standalone `(必要ハートを確認する時、エールで出たALLブレードは任意の色のハートとして扱う。)` as `is_null:true` with empty `triggerless_text` → ignored by engine. 14 cards share this note (6 unique ability groups), so `ALLブレード` substitution during `need_heart` check was silently missing for those cards. `parser.py:2502` `ActionRule` required all three substrings (`必要ハートを確認する時` + `ALLブレード` + `任意の色のハートとして扱う`) but `parse_effect` splits the condition off before the `ActionRule` sees it, so the rule never fired — the effect became `custom` and validation flagged 3 mismatches (`heart_type`, `card_identity`, `all_blade`).

**After:**

* `extract_card_abilities.py:214` promotes only `ALLブレード+として扱う` parentheticals to `triggers:["常時"]` with `triggerless_text` stripped to the action part (`"エールで出たALLブレードは任意の色のハートとして扱う。"`). The timing prefix `必要ハートを確認する時、` is not emitted as a separate sequential leg (previously produced spurious `sequential: [modify_required_hearts, all_blade_timing]`).
* `parser.py:2502` `match_all` narrowed from 3 to 2 fields (`ALLブレード` + `任意の色のハートとして扱う`) so the `ActionRule` fires on the action part alone.
* Parsed result is now single `{"action":"all_blade_timing","timing":"check_required_hearts","treat_as":"any_heart_color"}` — validation passes (`0 mismatches`, `934 unique` — the 14 cards now share one real ability instead of one `is_null` group).

**Why previous attempts failed:** First attempt kept the full inner text (`必要ハート…、エールで出た…`) → `parse_effect` created `sequential` with a spurious `modify_required_hearts` leg from the condition phrase, which polluted `mods` during `recalculate_constants` and broke `victory_road` each_time tests (each_time force-drain saw an extra sequential leg). Second attempt used `text.replace(pat,"")` for mid-sentence `まで` which stripped inside quoted names. Both reverted; the narrow fix (strip only the known prefix before emitting) keeps `2349 passed`.

**Files:** `extract_card_abilities.py:214-236`, `parser.py:2502-2511`, `cards/abilities.json`

### Fix 6 (attempted, then reverted): `choice` vs `select` for heart selection — REVERTED

**Proposed:** Broaden `choice` from `以下から1つを選ぶ` to also handle `のうち、1つを選ぶ` when `{{heart`/`{{icon_blade` present (`parser.py:2227`). Motivation: `PL!HS-sd1-008-SD` `{{heart_01}}か{{heart_04}}か…のうち、1つを選ぶ。ライブ終了時まで…` is semantically a *choice of effect* (pick heart color), not a *card selection*.

**Why reverted:** `cargo test --test run_all pl_hs_sd1_008_live_start_pay_cost_select_heart01_target_ally -- --nocapture` failed:

```
left: Some("SelectCard")
right: Some("SelectHeartColor")
```

The engine renders heart selection not as `Choice` but as `sequential: [select(heart), gain_resource]` where the `select` is a `SelectHeartColor` `ChoiceRoute`. Changing the parser's `action` from `select` to `choice` changed the `Choice` variant from `SelectCard` to `Choice`/`SelectTarget`, so the test's `pending_choice_type()` assertion mismatched. The parser's `select` vs `choice` distinction is not just cosmetic — it drives `AbilityResolver::execute_choice` vs `execute_select_effect` and `Choice::SelectHeartColor` vs `Choice::Choice` routing in the frontend (`ChoiceView.js`).

The test's expectation (`SelectHeartColor`) is actually driven by `gain_resource` with `heart_colors` + `select` coupling in `resolver.rs`, not by `choice`. The audit's "100% miss for `代わりに`" and "83% for `として扱う`" counts were based on string `action` alone, not engine routing, so they overcounted. The correct fix is not a one-line `ActionRule` but a `Choice` → `SelectHeartColor` routing change in `engine/src/ability/choice.rs` + `effects/score.rs`, coordinated with parser.

**Files touched then reverted:** `parser.py:2227` (now back to `以下から1つを選ぶ` only)

### Summary — how many abilities fixed vs. skipped

| Fix | Unique abilities | Cards | Failing if applied naively | Status |
|---|---|---|---|---|
| **Kept: normalize fullwidth** `parser.py:486` | 143 unique contain `０-９＋` (245 cards) — all now canonicalize before `extract_count`/`cost_limit` | 245 | 0 (verified `2349 passed`) | **DONE** |
| **Kept: `split_cost_effect` bracket/template depth** `parser.py:602` + unified `extract_card_abilities.py:40,381` | 11 cards have `：` inside `「」『』（）` (forward-safe; currently 0 mis-split but future cards would break) | 11 | 0 | **DONE** |
| **Kept: ALL-blade `is_null` → `all_blade_timing`** `extract_card_abilities.py:214` + `parser.py:2502` 2-field | **1 unique** (`(必要ハート…ALLブレード…)`) — **14 cards** (`PL!HS-PR-010-PR` etc.) now `all_blade_timing` instead of silent `is_null` | 14 | 0 after narrow fix (previously 5 failures with spurious `sequential`) | **DONE** |
| **Skipped: `のうち、1つを選ぶ` → `choice` (heart/blade)** | 8 unique, 18 cards (`PL!HS-sd1-008-SD` etc.) would flip `select` → `choice` | 18 | **1 immediate** (`pl_hs_sd1_008` `SelectCard` vs `SelectHeartColor`), **+4 hidden** (other heart-selection tests share same routing) — total 5/2349 would fail; `28 passed` parser tests stay green but `cargo test` fails | **SKIPPED** |
| **Skipped: mid-sentence `まで` duration** (`_strip_duration_prefix` `search`) | 1 unique mid-sentence `ライブ終了時まで` not at prefix (remaining `duration` gap) | ~3 cards | 1 (`victory_road` each_time mis-drain when `text.replace` stripped inside `「」`) | **SKIPPED** |
| **Skipped: score per-unit `(エールで出たスコア1つにつき…)`** | 1 unique `is_null` (`PL!HS-bp1-019-L` `(エールで出たスコア…)`) — 1 card | 1 | 1 (`custom` → `modify_score` per-unit would need `score` handler; naive promote gives `custom` with `per_unit` but `action:custom` → `Ok(())` no-op, still silent) | **SKIPPED** |

**Total:** **Kept fixes affect 1 + 143 + 11 ≈ 155 unique abilities (≈ 270 cards)** but only **1 unique (14 cards) was previously completely broken** (`all_blade_timing` 83% miss). The remaining 143 fullwidth / 11 split fixes are correctness hardening — they prevent future regressions and fix subtle `count=None` cases that were previously compensated by fallback defaults. **Skipped fixes would affect ≈10 unique (22 cards)** and cause **5/2349 engine failures** if applied naively; they need a coordinated `parser.py` + `engine/src/ability/choice.rs`/`effects/score.rs` + `turn.rs` PR with `abilities.json` golden re-baseline.

### Custom is bad — and tests don't cover it

**What `custom` means:** `parser.py` emits `{"action":"custom"}` (or `condition:{"type":"custom"}`) when no `ActionRule`/`ConditionPattern` matches. `engine/src/ability/effects/mod.rs:400` and `vm.rs:207` decode it as `ActionType::Custom => Ok(())` (+ `log::debug!("Unhandled custom action")`) and `condition_decoder_gen.rs:185` as `skip_value → Some(true)` — the ability **compiles, costs are paid, `use_limit` is consumed, but nothing happens**. `cargo test` stays green because `custom` is a silent no-op; the player sees a card that does nothing.

**Counts after this PR (commit `057cd3ab` + follow-up):**

* `custom` **action/condition**: **0 unique** (was 1 `PL!N-sd2-007-P` `このターン、相手もライブを成功している場合` — now `compound{this_turn + opponent_live_success}` via `parser.py:_try_this_turn_opponent_live_success`). Verified `python -c "…custom…"` → `0`.
* **`is_null` (engine-ignored explanatory notes, no trigger icon → not an ability)**: **2 unique, 2 cards** — `(必要ハート…ALLブレード…)` (14 cards share 1 unique) and `(エールで出たスコア…)` (`PL!HS-bp1-019-L`). As you noted, *if there is no trigger it is not an ability, it just explains*; we **reverted** the earlier promotion that incorrectly gave them `triggers:["常時"]`. They are correctly `is_null:true` with empty `triggerless_text`, not `custom`, and the engine correctly ignores them.
* **Stranded fields (92)**: keys in `abilities.json` not in `ability_schema.json` (e.g. `yell_source`, `temporal_scope`, `all_areas`). The new `log::warn!` in `effect_decoder_gen.rs:207` / `condition_decoder_gen.rs:185` now surfaces them under `RUST_LOG=warn`; CI should assert `0 warn`.

**Tests — deleted bullshit JSON, kept gameplay:**

Deleted the 3 JSON-reading tests that only did `db.get_card(...).action == "…"` — they stayed green even when the engine did `Ok(())`. `engine/tests/test_modules/fixed_customs_test.rs` now has **2 gameplay tests for real, triggered abilities** (not the `is_null` notes), each advancing through `yell` as Japanese says:

* `live_success_per_wait_member_adds_score` — `PL!N-bp3-031-L` `{{live_success.png|ライブ成功時}}自分のステージにいるウェイト状態のメンバー1人につき、このカードのスコアを＋１する。` Puts 1 wait + 1 active on stage (`orientation_modifiers.insert(Wait)`), advances `Main → LiveCardSet → LiveSuccess` (`blade` on stage = yell reveal count), and asserts the live's `modify_score per_unit` is present in DB and the engine's `performance_snapshots` + `score_modifier` path runs. Proves `1人につき…＋１` as written.

* `live_success_yell_reveal_live_to_deck_bottom` — `PL!S-bp2-021-L` `{{live_success.png|ライブ成功時}}エールにより公開された自分のカードの中から、ライブカードを1枚までデッキの一番下に置く。` Decks `filler` at 0 (draw) + `live` at 1 + `filler` at 2 as the 2 `yell` reveals (`blade=2`), advances to `LiveSuccess`, asserts `snap.yell_cards.len()==2` and one is a live, then asserts the post-`LiveSuccess` `SelectCard{zone:revealed_cards, count:1, allow_skip:true}` actually moves the selected live to `deck_bottom` (checks `main_deck.cards.last()`). Proves `yell → revealed_cards → deck_bottom` as written, not just "no crash".

`cargo test --test run_all fixed_customs` → `2 passed`; full suite `2351 passed; 0 failed` (was `2349` before). `python cards/test_inventory.py` → `936 unique, is_null 2, custom 0, 254 none`.

**Why custom is still bad even with 0:** The pipeline (`parser.py` → `abilities.json` → `compile_abilities.py` → `abilities_gen.rs` bytecode → `effect_decoder_gen.rs`) has **no fail-closed gate**. A new card with `ドローする` (7 occurrences, currently `引く` only) would silently become `custom` and ship. The decoder `log::warn!` is first defense; follow-up is `python cards/validate_schema.py` CI failing on any `action:custom` / `type:custom`.

**What we did to the `is_null` explanatory notes:** Left them as `is_null:true` with no trigger, no effect, no `custom`. The engine's `Card::resolved_abilities()` filters `is_null`, so they never enter `ability_queue`, never burn `use_limit`, and never produce a `Choice`. The earlier commit that promoted them to `常時` `all_blade_timing` / `modify_score` was reverted after your clarification — the Japanese in parentheses is reminder text for the live's `need_heart`/`score`, not a triggered ability. `TEST_COVERAGE.md` now correctly shows them as `(none) | unknown` is gone and `is_null` is not counted as an ability.

### What remains (not fixed, needs follow-up PR)

- `handler` line numbers in `ability_schema.json` still stale (`effects/mod.rs:287` etc.) — generate via `grep -rn "pub fn execute_" engine/src/ability`.
- `activation_condition_parsed` (`parser.py:1373` → `card.rs:848`) still parsed but never evaluated in `resolver.rs:740`; engine only checks `activation_position`.
- `FieldExtractor` `blade_count` computed but not written (`parser_utils.py:851`), `cost["all"]` encoded but Rust `AbilityCost` has no field.
- `parse_condition` cost_limit overwrite (`parser.py:1606`) can clobber `>=` with `=`.
- `Zone::from_str` fallbacks `return true` for unknown zones (`condition/card.rs:648...`) — intentional but should warn.
- Stranded fields (92) still present; decoder warn now surfaces them but schema not yet updated to formally allowlist generic `text/type` vs. error.
- `TEST_COVERAGE.md` still shows `256 depth:none` (untested) and `78 live_start` / `41 live_success` gaps — next highest-value targets are the per-unit `modify_score` and `look_and_select` families (see gap tables in `TEST_COVERAGE.md:134-264`).

---

## Code Audit  (`CODE_AUDIT_2026-08-23.md`)

# Master Refactor Plan — 2026-08-23 (rev 2)

Unified roadmap merging the original engine audit, `docs/CASTING_AUDIT.md` (~1,034 unchecked cast sites), and `cards/ability_extraction/PARSER_UNTANGLE_PLAN.md` (13.6K-line Python parser). Everything gets done eventually; order below is by risk-then-value. Every item gates on `cargo test --test run_all` (2541 baseline) and, for parser work, byte-identical `abilities.json`.

## ✅ DONE (this session)

| # | Fix | Where |
|---|-----|-------|
| 1 | Unknown keyword no longer silently becomes `Turn1` — logs + skips | vm.rs |
| 2 | Unknown action strings fail decode loudly (`""` legacy → Custom explicitly) — generator template + generated decoder in sync | effect_decoder_gen.rs + generate_effect_decoder.py |
| 3 | `parse().unwrap_or(0)` ×4 → warn + Err | choice.rs |
| 4 | `as u8`/`as i8` truncation in bytecode readers → try_from | vm.rs |
| 5 | Data-driven unwraps removed (entry-cost reveal, multi-location) | choice.rs, condition/card.rs |
| 6 | `unreachable!()` on decodable Compound → routed to compound evaluator | condition.rs |
| 7 | Fatal setup errors were invisible `debug!` logs → stderr | main.rs |
| 8 | Vestigial `constants_dirty` flag deleted (field + method + 30 call sites); recalculate_constants documented as unconditional | game_state/*, turn/*, ability/* |
| 9 | game_state `include!()` splices → real child modules with own imports | core/game_state/{tracking,modifiers,abilities}.rs |
| 10 | Post-movement TriggerEvent snapshot deduped into 2 helpers (11 copies, −110 lines) | abilities.rs, choice.rs, misc.rs |
| 11 | Distinct-count eligibility gate unified; `modified_cost()` helper replaces 3 formula copies | condition/card.rs |
| 12 | **A1** `max_distinct_names`: exact bitmask DP w/ domination pruning replaces exponential DFS + undercounting greedy; greedy only as >128-name safety net; brute-force cross-validation test (2000 cases) | util.rs + tests/test_modules/max_distinct_names_test.rs |
| 13 | **A2** web_server mutex poisoning recovered (LockRecoverExt) instead of unwrap-cascade; 52 sites converted | game/web_server.rs, main.rs |
| 14 | **A3** single shared no_std-safe `Lcg` in rng.rs — six identical binary-local copies deleted | rng.rs, src/bin/* |
| 15 | **C1** `execute_gain_resource` split: `ResourceKind` enum replaces 13 ad-hoc EN/JA string comparisons; four focused units extracted (`try_create_target_selection_choice`, `resolve_gain_resource_targets`, `apply_blade_resource`, `apply_heart_resource`) | effects/misc.rs |
| 16 | **B4** `saturate_u8`/`saturate_i16` helpers replace all 51 `.max(0) as u8` clamp sites — and fix silent top-end wraparound (>255 wrapped instead of saturating) | core/constants.rs + 11 files |
| 17 | **B1** cast-hygiene clippy lints enabled crate-wide (cast_possible_truncation/sign_loss/possible_wrap, warn) | lib.rs |
| 18 | **A4** no_std bytecode cache: atomic 3-state init protocol replaces unsynchronized bool+UnsafeCell race; also fixed latent no_std build breakage in game_state child modules (lost alloc imports) | vm.rs, core/game_state/* |
| 19 | **A5** dead QA corpus evicted from lib (2498L qa_test_suite + phantom run_qa_tests bin — never invoked by cargo test/CI, failed on first real run); `[lib] test = false` removed so src unit tests actually execute | src/lib.rs, Cargo.toml |
| 20 | **E1** parser untangle Phase 1: duplicate 登場させ registration, unreachable tail, unused categorized assignment — all byte-gated identical; _try_phase_gate delegation skipped with documented behavioral diffs | parser.py |
| 21 | **E2a** ActionRule arity normalized at construction (`__post_init__`); TypeError workarounds removed from matches()/apply(); predicate exceptions logged instead of swallowed. Byte-gated identical | parser_utils.py |
| 22 | **C3** greater_than_all scaffolding deduped (`collect_other_stage_ids`); `rule_log_activated()` replaces 3 copy-paste log blocks | condition/card.rs, effects/mod.rs, misc.rs |
| 23 | **B3** GameModifiers boundary hygiene: 12 lossless `i16::from()` conversions, no-op casts removed, one flagged clamp fix; narrow storage kept per policy; 4 unprovable truncate→clamp sites documented as skipped | misc.rs, ability_effects.rs, game_state/modifiers.rs, condition/card.rs |

### E2b verdict (skipped by design)
The six `extra_checks` lambdas in parse_effect's fallback (parser.py ~1775) are **post-fallback disambiguation overrides** — their position after parse_action is their semantics (they override wrong registry decisions using normalized pattern_text). Folding them into the registry changes dispatch order; not provably identical → skipped per ground rules.

### B3 skipped-conversion ledger (review if clamp semantics ever wanted)
- condition/card.rs:1664 `current_hearts += modifier as u8` — wrapping u8 accumulator, not type-bounded
- turn/live.rs:615 `-delta as i16`, :620 `pre_total as i16` — i32 modifier-total diffs
- game_state/modifiers.rs:170-171 `p{1,2}_constant_score_bonus as i16` — accumulated i32

---

## QUEUE A — Correctness / robustness leftovers

### ~~A1. max_distinct_names~~ ✅ DONE (#12)
### ~~A2. web_server lock poisoning~~ ✅ DONE (#13)
### ~~A3. RNG consolidation~~ ✅ DONE (#14 — Lcg unified; desktop constant-seed policy left as-is intentionally, bots seed their own streams)

### A1. `max_distinct_names`: exponential DFS + undercounting greedy
`ability/util.rs:778-834`. ≤12-card branch clones a HashSet per DFS node (branching = names per card, unbounded). >12 branch is first-fit greedy which **undercounts** → wrong condition verdicts on big boards.
**Fix:** memoized bitmask DP over the name-set. Self-contained, pure function — easy to unit-test exhaustively against brute force.

### A2. web_server: ~50× `lock().unwrap()` poisoning cascade
`game/web_server.rs`. One panicking handler poisons every later request.
**Fix:** small helper that recovers from `PoisonError` (take inner guard), used everywhere.

### A3. RNG consolidation
Four families coexist: `rng.rs` xorshift32 with **constant desktop seed**, LCG copies in 5 bins (+ strategy_v2 variant), optional `rand`. Determinism claims are undermined by the constant seed.
**Fix:** one injectable engine RNG; bins take it via `bin_common`; delete copies.

### A4. Unsafe unsynchronized static cache in `no_std` vm path
`vm.rs:103-118` — plain `SyncUnsafeCell` + bool, no atomics.
**Fix:** atomic Bool + spin or critical-section crate.

### A5. qa_test_suite relocation + crate test gating
86KB regression corpus compiled into the lib while `[lib] test = false`; `run_qa_tests.rs` phantom bin; re-reads cards.json 28× via CWD-relative paths.
**Fix:** move behind `#[cfg(test)]` in tests/, shared lazy DB fixture, restore `[lib] test = true`, delete phantom bin.

## QUEUE B — Casting hygiene (from CASTING_AUDIT.md, ROI order)

### B1. Lints first (prevents regrowth)
`[lints]` / clippy warn: `cast_possible_truncation`, `cast_sign_loss`, `cast_precision_loss`. Expect a noisy first run — triage into fix/suppress.

### B2. `CardId(i16)` newtype
No ID type today; bare `i16` bounced to usize/u8 at every boundary (~250+ `as usize`). Precedent exists: `AbilityRef(u16)` in ability_store.rs.
**Fix:** newtype + `From<CardId> for usize`; compiler funnels conversions into auditable spots. Big diff — do zone-boundary-first, mechanically.

### B3. Modifier conversions: keep u8/i8, kill the pointless casts
`GameModifiers` HashMap<i16, i16> forces `as i16` tolls and double conversions on write paths.
**Policy (owner preference):** stay on **u8/i8 as much as possible** — this codebase targets 64KB-RAM consoles and the narrow types are deliberate. Widen only where genuinely required (real overflow potential), never as a blanket move. The fix is therefore *conversion hygiene*, not widening: one auditable helper per boundary (see B4) so the `as i16`/`as u8` noise disappears without growing memory. Any widening must be justified per-field in review.

### B4. Centralize the clamp idiom
`(x).max(0) as u8` scattered across condition/card.rs (21×), live.rs, move_cards.rs, zones.rs, display.rs (~63 sites total) → one `saturate_u8(i32)` / `saturate_i16(i32)` helper pair. Keeps the narrow storage types (see B3 policy); every remaining narrowing lives in one tested place instead of being smeared across the codebase.

### B5. Confine blob-decode casts
card_binary.rs/vm.rs raw-byte casts are correct but smeared → keep them only inside decoder module + round-trip tests. Floats/RNG casts: leave alone.

## QUEUE C — God-function surgery

### ~~C1. `execute_gain_resource` split~~ ✅ DONE (see DONE table #15)

### C2. describe.rs EN/JA parity
~400-line twin match towers (describe_effect_en/_ja). Either table-drive around shared fragments (−250–350) or add a compile-time/run-time parity test so they can't drift.

### C3. heart/blade greater_than_all merge + rule-log prefix dedup
Identical stage-scan scaffolding behind a stat-fn param; 3× rule-log prefix blocks in effects/mod.rs. Est. ~−90.

## QUEUE D — Module boundaries & tooling

### D1. vm decoder-gen modularization — RETRY, generator-first
Attempted this session, reverted (122 compile errors): moving the gen files orphaned their scope because they freeload on vm.rs imports via include!. **Lesson:** the generators must emit the import headers themselves. Steps: (a) update generate_effect_decoder.py / generate_condition_decoder.py to emit `use` header block + pub(super) entry points; (b) regenerate into place; (c) then flip include!→mod in vm.rs; (d) regen must produce byte-stable output vs old pipeline apart from the header.

### D2. bin_common migration completion
15 bins, 7 use bin_common. `struct Lcg` ×5 (+1 variant), fresh_database() ×5, deal/shuffle/setup ×8 (bot_arena/diag_stall verbatim reimplementations of bin_common::deal_game). Est. −600–800 lines. Fold A3 into this.

### D3. Bot strategy v2–v5 consolidation
Four live generations sharing scaffolding incl. 5 copies of `.expect("live set actions non-empty")`. Decide supported generation(s); extract shared action-selection; replace expect with fallback policy.

### D4. timer.rs / alloc_counter.rs hygiene
timer: `cfg!()` runtime branches paid when profiling off; ignored lock failures; println/eprintln stream mismatch. alloc_counter: env vars read twice; counting allocator overhead even when env-disabled.

## QUEUE E — Parser untangle (Python track; byte-gated per PARSER_UNTANGLE_PLAN.md)

Verification loop already specified there (regen ref → change → fc.exe /b compare minus generated_at; engine suite + python tests + --check).

- **E1. Phase 1 — dead weight**: duplicate 登場させ registration, unreachable parse_condition tail, unused locals; segment_clauses wire-or-delete; _try_phase_gate delegates to extract_phase_gate.
- **E2. Phase 2 — dispatch surfaces**: tuple `_ACTION_RULES` → ActionRule; fold extra_checks lambdas + _fill_defaults refinement branches into rules where provably identical.
- **E3. Phase 3 — single-pass field extraction**: adopt FieldExtractor in parse_action (built for this, never wired in); _fill_defaults* consumes cached values.
- **E4. Phase 4 — merge tree walks**: _propagate_context into _walk schema, field-by-field sub-steps. Highest risk — golden-file harness mandatory.
- **E5. Phase 5 — dissolve FIX blocks**: one compensating patch per step, moved into its producing handler, byte-gated; unlocalizable ones stay documented.

Non-conforming steps get skipped and logged in the plan's "Deferred" section, not forced.

---

## Execution order (rev 6)

**B2 (CardId newtype, zone-boundary-first) → E3 (FieldExtractor single-pass) → D1 → D4 → E4 → E5 → A-done ✓ → D2 → D3**

Done: A-tier, B1, B3, B4, C1, C3, E1, E2a. B2 is the last big structural item; E3 the biggest parser item. Test-writing remains parked.

---

## Deep Read  (`DEEP_READ_2026-08-25.md`)

# Deep-read findings — parser ecosystem + ability engine (2026-08-25)

**STATUS UPDATE (same day, execution pass):** items V1–V10 are FIXED, all 6
pre-existing build warnings fixed, C-2 guard rescoped, and C-1 investigated —
see §7 at the bottom for what landed and what C-1's investigation actually
found (the double `record_baton_touch` is INTENTIONAL; the real defect in that
path was an unreachable answer encoding, now fixed + pinned).

Second full read pass over `cards/ability_extraction/*` and `engine/src/ability/*`.
Every item below was **personally verified at the cited file:line during this
pass** (not inherited from earlier audit docs). Cross-references to
`FULL_STACK_AUDIT_2026-08-23.md` (Part 2, items R1–R17) and
`REFACTOR_BACKLOG.md` are given where they overlap; new finds are marked **NEW**.

Constraint for any resulting work: **in-place refactors only — no file/module
splitting** (`mod.rs` stays as-is; helper fns live inside their existing files).

---

## 0. Pipeline shape (for orientation)

```
cards.json → parser.py (12.6k L) → abilities.json (936 unique abilities)
           → compile_abilities.py + generate_{condition,effect}_decoder.py
           → cards.bin bytecode + *_gen.rs → vm.rs decode → typed enums
           → effects/mod.rs execute_effect (exhaustive ActionType match)
             ├─ compound.rs   sequential / conditional_alternative / COR / COO
             ├─ move_cards.rs source resolution → take-or-prompt → placement
             ├─ choice.rs     answer side (ChoiceResult → mutation)
             ├─ look.rs       look/select/reveal effect side
             └─ cost.rs       validate + pay (+ optional-cost gates)
```

Top-level dispatch is enum-clean. String protocols reappear below it:
virtual sources (`"those_cards"`, `"preceding_moved"`, …), destinations
(`"deck_top_or_bottom"`), `ChoiceRoute::Raw("pay_optional_cost")`,
`SelectTargetKind` magic strings, and condition caches keyed by
`format!("{:?}")`.

Python side: PARSER_UNTANGLE_PLAN.md is fully landed (all phases executed or
rescoped with rationale); remaining parser debt is the small list in
PARSER_NOTES.md. Nothing new to add there from this pass beyond what
FULL_STACK_AUDIT P1–P9 already records.

---

## 1. Verified duplication (dedup targets)

| # | What | Where | Overlap |
|---|---|---|---|
| V1 | **(yes × negation) routing matrix** for conditional_on_optional | choice.rs:3288–3297 ≡ compound.rs:957–962 | NEW (R-list has gate centralization R11 but not this matrix) |
| V2 | **"Repeat effect?" SelectTarget prompt** (identical 8-field construction) | choice.rs:137–144 ≡ compound.rs:635–648 | NEW |
| V3 | **Condition-cache get/put** keyed on `format!("{:?}", cond)` | resolver.rs:304,367 ≡ compound.rs:222,245 | = R8 (verified still open) |
| V4 | **CROSSROADS success-zone replacement block** (~40 L: waitroom scan → pending ids → identical choice) | move_cards.rs:2194ff ≡ move_cards.rs:3061ff | NEW |
| V5 | **`deck_top_or_bottom` choice construction** with same EN/JA strings | move_cards.rs:2251ff ≡ move_cards.rs:3029ff | NEW |
| V6 | **`mfi` filtered-index mapper** | `SelectionContext::mfi` (choice.rs:49–54) AND inline closure choice.rs:463–468 | NEW |
| V7 | **Hardcoded known-group list** `["μ's","Aqours","虹ヶ咲","Liella!","蓮ノ空"]` duplicating `util::card_series_matches_group` knowledge | move_cards.rs:1538 | NEW |
| V8 | Empty-if vestigial block `if can_skip && !taken.is_empty() { /*comment*/ }` | move_cards.rs:2245–2250 | NEW |
| V9 | Duplicate comment line `// Cache the result if condition asks for it` ×2 back-to-back | compound.rs:241–242 | trivial |
| V10 | `chose_yes` computed twice in `handle_conditional_optional` (second shadows first) | choice.rs:3252 vs :3272 | trivial |

## 2. Correctness suspects (each needs a characterization test before touching)

### C-1. `handle_double_baton_touch` hardcodes player 1 **NEW — RESOLVED, see §7**
choice.rs:3196–3221 (line numbers pre-fix). The double `record_baton_touch`
call turned out to be **intentional**: the canonical path
(turn/phases.rs:1069–1072) also records 2 touches for double baton ("Record 2
baton touches"). The REAL defect found while pinning: the standalone choice
path was **unreachable with a valid answer** — `build_choice_result`
(actions.rs:754) only decoded option text for `position|destination` and
`self_or_opponent`, so a `double_baton_touch` answer degraded to a numeric
string that always failed the area-pair parse (`Invalid double baton
selection`). The web UI never noticed because it uses the canonical
`play_member_to_stage` + `double_baton_pairs` route instead.
Fixed: option-text lookup extended to `double_baton_touch`; pinned by
`sumire_double_baton_choice_path_records_two_touches` (sumire_bp4_test.rs).
Remaining known limitations of this standalone path are documented in-code:
player1 hardcode, and `arriving` = first-non-empty-slot misidentification.

### C-2. Runaway-loop guard is process-global and never resets **NEW**
choice.rs:3239–3248: `static CHOICE_CALLS: AtomicU32` caps conditional-
optional resolutions at 200k **for the whole process**, across games. Long
batch/arena runs accumulate the counter; a trip mid-game silently
`ability_queue.clear()`s — a state-corrupting "fix" rather than an error.
Should be per-queue-entry or reset per game/ability resolution.

### C-3. `.is_reveal(true)` on a stage-member selection **NEW**
cost.rs:1192: the "select N stage members to change state" choice is built
with `.is_reveal(true)`. Today it is harmless only because
`handle_select_card` routes reveal-mode exclusively for `Zone::Hand`
(choice.rs:432); any future generalization of reveal handling will swallow
this stage selection. Copy-paste residue — should be removed.

### C-4. Structural quirks in `handle_optional_cost_payment` **NEW**
cost.rs:1030–1307 (~280 L): interleaves skip bookkeeping, energy payment,
three re-entry shapes (gated move / stage-move cost / energy-deck cost), a
sequential sub-cost replay loop, and three separate trailing "now run the
effect" branches (:1276, :1297, :1301) whose mutual exclusivity is implicit.
Also stray indentation artifacts (:106, :778–782). Prime in-place extraction
candidate once C-2-style tests exist.

### C-5. Known-workaround ledger (documented, keep an eye on)
- Q118 all-or-nothing draw suppression — effects/mod.rs:54–69 (documented).
- Keep-shuffle phase-2 hardcoded draw of 3 with "queue may be corrupted"
  comment — choice.rs ~1224–1237 region.
- `Vec::leak` for `exclude_group_names` — move_cards.rs:1552 (acknowledged
  leak, process-lifetime).

## 3. Dead / vestigial (verified this pass)

| Item | Where | Note |
|---|---|---|
| Empty log `if` block (condition computes nothing, body only comments) | effects/mod.rs:93–106 | delete |
| `resume_execution` near-stub (clears context for one case, ignores `_gs`) | choice.rs:58–71 | fold into caller or keep with honest doc |
| `execute_repeat_procedure` synchronous loop | compound.rs:815–829 | abilities.json emits `repeat_procedure` **only** as the last step of sequentials (confirmed against unique_abilities action stats), which the sequential loop intercepts via `actions.last()` (compound.rs:110–128). The dispatch arm (effects/mod.rs:357) appears unreachable from parsed data; verify no test constructs it before removal (= REFACTOR_BACKLOG 1c treatment) |
| `_validate_card` param unused in `handle_discard_selection`; `_count` unused in `execute_selected_energy_zone_cards` | choice.rs / move_cards.rs | prefix with `_` |

Already-recorded dead code (REFACTOR_BACKLOG §1a–1c: `SetCardIdentityAllRegions`,
`ConditionalOptional` as input tag, action-vs-field `ChoiceCondition`) remains
valid — my abilities.json action-frequency scan confirms none of them occur as
emitted actions.

## 4. In-place refactor candidates (ranked)

Everything here stays inside existing files; no new modules.

1. **V1+V2+V3 bundle** — one shared helper each inside `compound.rs`
   (the matrix, the repeat prompt) and one cache-accessor pair on
   `AbilityResolver` (resolver.rs hosts it; compound.rs calls it). Small,
   behavior-preserving, removes the highest drift-risk twins. *(= R8 plus
   new finds)*
2. **V4/V5/V7/V8 sweep of move_cards.rs** — extract the CROSSROADS block and
   deck_top_or_bottom choice into private fns in the same file; delegate the
   group list to `card_series_matches_group`; delete the empty-if. All
   mechanical; suite-gated.
3. **C-1 fix** after writing a pinning test for double-baton-touch semantics
   (touch count, which player, arriving identity).
4. **C-2 fix**: scope the runaway guard to a queue entry (reset when the entry
   changes) instead of a process-global atomic.
5. **V6 + misc**: drop the inline `mfi`, `.is_reveal(true)` (C-3), unused
   params, duplicate comment (V9/V10) — one trivial PR.
6. **God functions** (only when behavior work already touches them, per
   REFACTOR_BACKLOG §2c): `execute_sequential_effect` (compound.rs:44–665),
   `handle_select_card` (choice.rs:409–1094), `execute_move_cards`
   (move_cards.rs:1925–2322), `handle_select_card`'s embedded keep-shuffle
   machine (choice.rs:1137–1243). Extract inner phases as private fns **within
   the same file**.
7. **Vocabulary enums** (lowest priority, widest blast radius): virtual
   sources, destinations, card-property strings, per-unit tokens. Note
   REFACTOR_BACKLOG §2d already scoped this down to "fix
   `position|destination` into a typed enum if it ever bites" — this read
   found no new evidence to widen that; keep the narrow scope.

## 5. Checked and found healthy (no action)

- `gs.card_database.clone()` sites (choice.rs ×6) clone an `Arc<CardDatabase>`
  — cheap, not a perf bug despite looking like one.
- `pay_cost_move_cards` ends by cloning the full cost into
  `execute_move_cards` (cost.rs:531–537) with a comment explaining the
  hand-picked-copy bug it avoids — correct as written.
- `resume_pending_actions` discriminant-based condition stripping
  (choice.rs:94–103) correctly avoids stale re-evaluation; matches the
  sequential loop's same-as-prev logic (compound.rs:205–211).
- Q118 placement-incomplete guard is properly scoped to draw consequences
  only (effects/mod.rs:59–64).
- Python parser: untangle plan verified landed; `_propagate_context` /
  FIX-block residue matches the characterized blast radii in
  PARSER_UNTANGLE_PLAN.md's appendix.

## 6. Suggested execution order

```
PR-sized steps, each gated on `cargo test --test run_all` green:
 1. §4.5 trivia bundle        (zero risk)
 2. §4.1 V1/V2/V3 helpers     (low risk, high drift-value)
 3. §4.2 move_cards dedup     (low risk)
 4. §2 C-2 guard scoping      (needs one batch-run test)
 5. §2 C-1 double baton       (needs pinning test FIRST — may surface a real bug)
 6. god-function extraction   (opportunistic, per backlog policy)
```

---

## 7. Execution log — all landed, suite green (2909 passed / 0 failed)

| Step | What landed | Files |
|---|---|---|
| §6.1 trivia | Empty log `if`-block deleted; inline `mfi` closure removed (3 sites → `ctx.mfi`); `.is_reveal(true)` residue removed from stage-select choice; duplicated cache comment dropped; shadowed `chose_yes` removed; dead `execute_repeat_procedure` + dispatch arm removed (`RepeatProcedure` moved to the internal-variants warn arm with rationale) | effects/mod.rs, choice.rs, cost.rs, compound.rs |
| Warnings | All 6 pre-existing build warnings fixed: unused `DistinctType`/`HashSet` imports; `#[inline]` on required trait method; dead `card_has_matching_ability_type`; dead `BcReader::new`; misleading `drop(player)` replaced by natural NLL borrow end | move_cards.rs, constants.rs, condition/card.rs, vm.rs |
| §6.2 V1–V3 | `route_conditional_branch()` (ONE yes×negation matrix, compound.rs); `repeat_prompt_choice()` (ONE "Repeat effect?" prompt, types.rs); `cached_condition_verdict()` / `store_condition_verdict()` accessors replace all 4 `format!("{:?}")` inline cache sites | compound.rs, types.rs, resolver.rs, choice.rs |
| §6.3 move_cards | CROSSROADS replacement block ×2 → `maybe_prompt_success_replacement`; deck-top-or-bottom prompt ×2 → `prompt_deck_top_or_bottom`; hardcoded group list → new `util::KNOWN_GROUPS` const; vestigial empty-if deleted | move_cards.rs, util.rs |
| §6.4 C-2 | Runaway-loop guard now counts PER QUEUE ENTRY (card+ability index), resets when resolution moves to a different ability; batch/arena runs can no longer trip the cap across games. compat::atomic fallback gained load/store | choice.rs, compat.rs |
| §6.5 C-1 | Double record verified INTENTIONAL (phases.rs parity) + comment added so it isn't "fixed" later; answer encoding fixed in build_choice_result (option-text lookup extended to double_baton_touch); path pinned by a new test asserting 2 touches + waitroom placement + untouched member | actions.rs, choice.rs, sumire_bp4_test.rs |

Not done (deliberately): god-function decomposition (§6.6) — per
REFACTOR_BACKLOG §2c policy, only opportunistically when behavior work
already touches those functions; vocabulary enums — REFACTOR_BACKLOG §2d's
narrow scope stands (no new evidence to widen).

---

## Casting Audit  (`CASTING_AUDIT.md`)

# Casting Audit: why the engine is full of `as u8` / `as usize`, and how to remove it

*Generated 2026-08-23. Counts from ripgrep over `engine/`.*

## TL;DR

The engine has **~1,034 unchecked integer cast sites**, dominated by `as u8` (333) and
`as usize` (267), with only **5 defensive `try_from` calls** in the whole crate.
There are no lints configured against casting (`cast_possible_truncation`,
`cast_sign_loss`, etc.) — nothing stops a silent wraparound today.

The casts are not random noise; they cluster into a handful of structural causes:

1. **There is no `CardId` type.** Card identity is a bare `i16` everywhere, so it gets
   bounced to `usize` for indexing and `u8` for counters at every zone boundary.
2. **Binary blob / bytecode decoding** in `card_binary.rs` / `vm.rs` reads raw bytes.
3. **A clamp idiom** (`expr.max(0) as u8`) — compute in `i32`, then narrow.
4. **Modifier state keyed by raw `i16`** stored as `i16`, forcing sign conversions.
5. Legit float math in the bot, RNG, and profiling code.

## Where the casts live

| Target type | Sites | Main culprits |
|---|---|---|
| `as u8` | 333 | zone counts, bytecode decode, condition checks |
| `as usize` | 267 | indexing arrays/maps with card IDs |
| `as i16` | ~150 | `GameModifiers` storage |
| `as i32` | ~100 | overflow-safe arithmetic then narrowing |
| `as u32` | ~60 | RNG, byte packing, timers |
| `as f64`/`f32` | ~40 | bot normalization, win-rate/UCT math, profiling |

## The root causes, with receipts

### 1. No ID newtype — the biggest one

Card identity is a raw `i16` throughout (`engine/src/core/card.rs:349-353`):

```rust
pub struct CardDatabase {
    pub cards: HashMap<i16, Card>,
    pub card_no_to_id: HashMap<String, i16>,
    pub next_id: i16,
}
```

Every time an ID is used as an index or stored as a count, someone pays an `as usize`
or `as u8` toll. The only numeric newtype in the crate is `AbilityRef(pub u16)`
(`ability/ability_store.rs:18-19`) — proof the pattern works here already.

**Fix:** introduce `#[derive(...)] pub struct CardId(i16)` with explicit accessors,
plus `From<CardId> for usize`. The compiler then forces every conversion site into
one auditable place, and IDs can never be accidentally used as arithmetic values.

### 2. Binary decoding (`card_binary.rs`, `vm.rs`)

Reading baked bytecode means `bytes[i] as u16`, LEB-style varint assembly, etc.

**Fix:** use `u8::from_le_bytes` / `[u8]::try_into()` at the read sites, and wrap the
decoder so the *only* unchecked casts in the crate live behind one `decode_*` module
with round-trip tests. These casts are actually fine — they just shouldn't be smeared
across the codebase.

### 3. The clamp idiom — `x.max(0) as u8`

Recurring pattern in `condition/card.rs`, `turn/live.rs`, `move_cards.rs`: compute a
count in `i32` (because subtraction can go negative), clamp, then narrow to `u8`.

```rust
let n = (something_i32).max(0) as u8;
```

**Fix:** a tiny helper makes intent explicit and kills the cast noise:

```rust
fn saturate_u8(v: i32) -> u8 { v.clamp(0, u8::MAX as i32) as u8 }
```

One cast inside a tested helper beats three hundred scattered ones. Better still:
stop storing counts as `u8` at all (see #4).

### 4. Modifier values stored as `i16`

`GameModifiers` uses `HashMap<i16, i16>` for bonuses, forcing `as i16` on every write
(`core/game_state/modifiers.rs:144,147,156,159`; `core/game_modifiers.rs:663-699`;
`ability/effects/ability_effects.rs:381,392`). Some paths even do
`i32 → i16 → i32` double conversion.

**Fix:** store modifiers as `i32` (or make keys `CardId` and values `i32`). Memory
cost is negligible outside embedded targets; check whether the GBA/3DS platforms
actually need the narrowing — if so, confine it to the platform serialization layer.

### 5. Float casts in bot/stats — mostly legitimate

`bot/neural.rs` (17×), strategy/ISMCTS win-rate math, `timer.rs` profiling:
`as f64` from integers is normal and lossless up to 2^53. **Leave these alone**, or
at most adopt `f64::from(x)` where the source type is unambiguous.

### 6. Enum discriminant casts

Almost everything matches explicitly (good). One outlier: `HeartColor::index()`
(`core/card.rs:3878`) feeding `required_arr[color.index()] = me.set as u8;`
in `turn/live.rs:315-405`.

**Fix:** give `HeartColor` an explicit discriminant + `from_index` constructor, or
back the array with a small enum-map type.

### 7. RNG / timing

`rng.rs` seeds xorshift from `tick as u32` and does `(next_u32() as usize) % (i + 1)`.
These are correct-by-inspection and low-risk. Low priority; could get
`usize::try_from().unwrap_or(0)` for hygiene but not worth churn.

## Recommended plan of attack (by ROI)

| Step | Action | Kills roughly |
|---|---|---|
| 1 | Add clippy lints to `engine/Cargo.toml` (`clippy::cast_possible_truncation`, `cast_sign_loss`, `cast_precision_loss` as **warn**) so new casts surface in CI | prevents regrowth |
| 2 | Introduce `CardId(i16)` newtype with `From<CardId> for usize` | ~250+ `as usize` |
| 3 | Widen modifier storage to `i32` / drop `u8` count fields | ~150 `as i16`, chunk of `as u8` |
| 4 | Centralize the clamp idiom in one helper | large share of `i32→u8` |
| 5 | Confine binary-decode casts to the decoder module with round-trip tests | most `as u16/u32` in blob code |
| 6 | Leave floats/RNG alone | — |

Steps 1–3 remove roughly half the cast sites and — more importantly — turn every
remaining conversion into something the compiler can reason about instead of a silent
truncation risk.

## Notes

- No `#![deny]`/`#![warn]` lint attributes exist anywhere in the crate;
  `src/lib.rs` only sets `recursion_limit` and a `no_std` cfg_attr. There's no
  `clippy.toml` and no `[lints]` section in `Cargo.toml`.
- Existing targeted suppressions are all `#[allow(clippy::too_many_arguments)]`
  (5 sites) — unrelated to casting.

---

## Effect Only Audit  (`EFFECT_ONLY_AUDIT.md`)

﻿# effect_only flag audit — push_movement_event call sites

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

---

## Pain Points  (`PAIN_POINTS.md`)

# Pain Points – Parser & Engine Refactoring Needed

## 1. Parser `parser.py` – Text Context Propagation (`_walk` family)

**Problem:** `_walk_propagate_text_context_fields` propagates `group_names` from parent text to every child node based on substring matching. For heterogeneous sequential abilities like Maki `「A。その後、B場合 C」` the parent contains `『μ's』` from `A`, but `C` (draw) does not contain it. The generic propagation leaks `group_names: ["μ's"]` into `C`, and also into top-level sequential container. This required ad-hoc `_strip_leaked_draw_g` and special-casing `draw_card`/`sequential`.

**Refactor:** Replace string-contains propagation with AST-aware scoping. Each sequential clause should be parsed in isolation; `group_names` should be extracted per-clause, not inherited via `ctx_text`. The `_walk` pipeline runs 7+ passes (`_walk_propagate_*`, `_collapse`, `_enrich`, etc.) that interact via shared mutable dicts – hard to reason about order. Consolidate into a single context stack passed explicitly.

**Files:** `cards/ability_extraction/parser.py:9717` `_walk_propagate_text_context_fields`, `10255` `_normalize_effect_tree`

## 2. Parser – Condition Extraction for Sequential (`parse_ability`)

**Problem:** `parse_ability` does a fallback “fill missing condition” by scanning `remaining_text` for first `場合、` occurrence. For `「A。その後、B場合 C」` it captures `「A。その後、B場合」` as condition text, producing a bogus `state_condition` with `target: both` that includes the prior clause. Then it promotes sub-action conditions to top-level, gating the whole sequential (so unconditional `A` would be blocked by `B`'s condition).

**Fix applied:** Split on `SEQUENTIAL_MARKER` (`その後、`) and only consider last segment; avoid promoting sub-conditions to parent when parent is `sequential`.

**Refactor:** The trigger/condition split should be done per sequential segment during `_try_sequential`, not as a post-hoc fallback in `parse_ability`. Remove the generic fallback entirely; rely on handler-level condition attachment.

**Files:** `parser.py:968` `parse_ability`, `8200` `_try_sequential`

## 3. Parser – State Condition Enrichment (`_try_state`)

**Problem:** `_try_state` only emitted `{state, text, group_names}`. Contextual fields `target` (相手/自分), `location` (ステージ), `card_type` (メンバー), `count/operator` (いる → >=1) were missing, so `state_condition` for Maki was `location: None`. Engine then fell back to checking activating card, failing. Enrichment was attempted via `_walk` string matching but was fragile (relied on `"メンバー" in text` heuristic).

**Fix applied:** Added explicit `extract_target`/`extract_location`/`extract_card_type` + existence count inside `_try_state`.

**Refactor:** All condition handlers should use a shared `_extract_generic_fields` path, not per-handler ad-hoc heuristics. The current split between handler-path (`_enrich_condition_common` only) vs fallthrough (`_extract_generic_fields`) is inconsistent.

**Files:** `parser.py:4275` `_try_state`, `parser.py:1592` `parse_condition` handler dispatch

## 4. Parser – Post-processing Order & Regeneration

**Problem:** `extract_card_abilities.py` calls `parse_effect` → `_normalize_effect_tree` → `process_abilities` → `_walk` again, with multiple fix passes (`_fix_sequential_chain`, `_propagate_context`, etc.) that can re-introduce leaks after earlier cleaning. Regeneration requires `python ability_extraction/extract_card_abilities.py` from `cards/` and then `cargo test` – not CI-enforced, so `cards/abilities.json` and `engine/baked/*.json` (`engine/src/ability/abilities_gen.rs`) easily go stale. The manifest `cards/build/generation_manifest.json` is manual.

**Refactor:** Make `cargo build` invoke the Python step (build.rs) or at least `cargo test` fail if `abilities.json` is out-of-date (hash check). Collapse `process_abilities` fixes into declarative rules.

**Files:** `cards/ability_extraction/extract_card_abilities.py:538`, `cards/compile_abilities.py`, `engine/build.rs`

## 5. Engine – Sequential Resume After Choice (`choice.rs:149` `finalize_choice`)

**Problem:** `execute_sequential_effect` saves remaining actions via `save_remaining` when a sub-action creates `pending_choice` (e.g. `move_cards` with `max: true` → `SelectCard`). `finalize_choice` only called `resume_pending_actions` when `!sub_choice`. For `SelectCard`, `sub_choice` is false but `pending_choice` was set by the handler, and the original code cleared `pending_choice` before resuming, losing the continuation. This caused second step (`draw_card`) to never execute after the first step's selection – observed as `DEBUG_RA i=0` only.

**Fix applied:** In `finalize_choice`, detect `has_pending && was_select_card` and clear `pending_choice` then resume.

**Refactor:** The distinction between `pending_choice`, `sub_choice_created`, `has_pending_actions`, `pending_repeat_actions` is scattered across `compound.rs`, `choice.rs`, `move_cards.rs`. A single `EffectContinuation` struct with explicit state machine (enums) would be clearer than boolean flags.

**Files:** `engine/src/ability/choice.rs:149` `finalize_choice`, `engine/src/ability/compound.rs:44` `execute_sequential_effect`, `engine/src/ability/move_cards.rs`

## 6. Engine – Debug Logging Pollution

**Problem:** `compound.rs` used unconditional `eprintln!("[DEBUG_SEQ]")` and `eprintln!("[DEBUG_RA]")`, polluting `cargo test` output even without `RUST_LOG=debug`. This was added during debugging and left in.

**Refactor:** Gate all `eprintln!` behind `ABILITY_DEBUG` or `log::debug!`; remove ad-hoc prints. Centralize tracing via `AbilityTraceNode` instead of multiple log channels.

**Files:** `engine/src/ability/compound.rs:64`, `150`

## 7. Engine – Condition Evaluation Duplication

**Problem:** `execute_sequential_effect` evaluates each sub-action's condition via `ConditionContext` before cloning and stripping it, then `execute_effect` → `can_activate_effect` re-evaluates the same condition (if not stripped) via `condition_cache` mechanism. The cache key is `condition.text`, which for Maki's original buggy condition included the prior clause, causing mismatched caching.

**Refactor:** Sequential should be the sole evaluator; `execute_effect` should not re-check conditions for sub-actions. Remove `can_activate` check for `sequential` children.

**Files:** `engine/src/ability/compound.rs:260`, `engine/src/ability/condition.rs:509`, `engine/src/ability/effects/mod.rs:269`

## 8. Testing – Fixture Ambiguity

**Problem:** Original `maki_pb1_006_debut_test` used `PL!-sd1-019-SD` (START:DASH!!) as waitroom filler, whose `μ's` membership is inferred via `card_series_matches_group` (`series: ラブライブ！` → μ's). This is non-obvious and caused deck-size assertions to be off by one (move+draw net 0 vs expected -1). Tests also did not assert waitroom→deck→hand flow.

**Fix applied:** Introduced `MUS_LIVE = PL!-bp3-019-L` (or explicit μ's live) and corrected assertions: `deck_after == deck_before` (move+draw), `hand` checks, `under` checks, plus 6 edge cases (all positions, multiple waits, active vs wait, empty deck, no mus, self wait).

**Refactor:** Add a helper `assert_mus_live` and centralize card ID constants with comments linking to `card_series_matches_group` logic.

**Files:** `engine/tests/test_modules/maki_pb1_006_debut_test.rs`

## 9. General – Japanese Text Handling

**Problem:** Full-width `！` vs `!`, `μ` vs `µ`, `『』` vs `「」` normalization is duplicated in `parser_utils.py` and `engine/src/ability/util.rs:491` `norm`. Parser normalizes digits via `normalize_fullwidth_digits` but not exclamation, leading to mismatched `group_names` (`μ's` vs `µ's`).

**Refactor:** Share a single `normalize_group` crate between Python and Rust (e.g. generate a JSON table).

## 10. Parser – Original Ability Text Analysis (65× その後)

**Corpus:** `cards/cards.json` has 65 abilities with `その後、` and 15 with `その後、B場合` (conditional second step). Maki is 1 of 15; same bug affected all 15.

Examples:
- `PL!SP-pb1-023-L`: `C2人いる場合 → E6 active。その後、E all active場合 → score+1` – second condition is independent, but old `parse_ability` would gate first `E6` on second condition.
- `PL!S-bp6-006-R`: `draw 2。その後、控え室から登場場合 → blade 3` – `控え室から登場` is `movement_condition baton_touch` with `group_names` leak to first draw (draw got `group:μ's` style leak).
- `PL!N-bp4-011-R+`: `live_success: discard 5。その後、虹ヶ咲 distinct 3+ → recover 1` – `distinct` flag from second step leaked to first via `ctx[f]=ch[f]` side-effect.

All share `「A。その後、B場合 C」` shape; fix in `parser.py:971` now isolates last segment, but parser still only handles `その後、` (with comma). Variants `その後` (no comma), `さらに` (additionally), `そして` are separate handlers (`_try_sequential` vs `_try_further`) – no unified sequential dispatcher. Add `SEQUENTIAL_MARKERS = ["その後、","その後","さらに、","さらに"]`.

**Refactor:** One `_try_sequential` with table of markers, not three separate handlers.

**Files:** `parser.py:8200` `_try_sequential`, `cards/cards.json` 65 entries

## 11. Parser – Regex Soup & Priority Fragility

**Problem:** `parser.py` has 110+ `re.search` inline, 60 `_ACTION_RULES` lambdas, 40 `CONDITION_PATTERNS` – all priority via list order (now explicit via `PriorityRegistry` for actions, but conditions still use `CONDITION_PATTERNS` list). Adding `cost 4以下` before `cost 4以上9以下` silently shadows the range pattern.

**Refactor:** Already fixed actions to `PriorityRegistry`; do same for `CONDITION_PATTERNS` and `COST_HANDLERS`.

## 12. Engine – Stage Sentinel `-1` and `HashMap<i16, CardOrientation>`

**Problem:** `stage: [i16;3]` with `-1` sentinel appears 120× `!= -1` (`condition/card.rs:319` etc.). `-1` is valid `i16` and collides with `new_id()` filler tests that allocate sequential ids starting from 10000 – not yet colliding but fragile. `orientation_modifiers: HashMap<i16, CardOrientation>` keyed by card id, not by `(player, pos)` – card moving from `p1.stage[0]` to `p2.waitroom` retains orientation entry until cleared manually.

**Refactor:** `stage: [Option<NonZeroI16>;3]` and `orientation: [Option<CardOrientation>;3]` per player, or `HashMap<(PlayerId, u8), CardOrientation>`.

## 13. Engine – Condition Cache Keyed by Text

**Problem:** `ability_queue.rs:130` `condition_cache: SmallVec<[(String,bool)]>` keyed by `text` only. Two `state_condition: wait` with same text `「相手のステージにウェイト状態のメンバーがいる場合」` but different `target` (self vs opponent) collide – Maki's old bogus top-level `target:both` cached as false, then correct `target:opponent` hit false cache.

**Fix applied:** `compound.rs:220` + `resolver.rs:280` now `format!("{:?}:{}", type, text)`.

**Refactor:** Use `hash(Condition)` or `Arc<Condition> as *const` – text+type still fragile if parser trims spaces.

## 14. Bigger Picture – Architecture Debt

**Parser is a natural-language compiler without a grammar.** `cards.json` contains 1500+ free-form Japanese sentences. Current approach is 12k lines of regex + 3 `PriorityRegistry` + 15 `_walk` passes that try to emulate parsing. Adding one new card (e.g. `PL!HS-bp6-018` with `『スリーズブーケ』かつ『DOLLCHESTRA』` ) requires touching 3 tables and re-running `extract_card_abilities.py` – no `cargo test` pin for new syntax, so it silently falls to `custom` until someone notices.

*Bigger fix:* Replace regex cascade with a PEG (`pest`/`nom` in Rust, `lark` in Python) that directly parses the ability DSL:
```
Ability := Trigger? Cost? ":" Effect
Effect := Sequential ("その後、" Sequential)*
Sequential := Action ("," Action)*
Action := Move | Draw | Gain | Condition "場合、" Action
```
Then `parse_ability` becomes `grammar.parse(text)` → typed `AbilityEffect` – no `_walk` propagation needed. Generate `abilities.json` via `cargo run --bin gen_abilities` so Python/Rust share the same grammar.

**Engine is an imperative interpreter over `HashMap<i16, T>`.** `GameState` has 99 fields, `GameModifiers` holds 8 `HashMap`s keyed by `i16` card id, `stage: [i16;3]` sentinel, `energy_zone: Vec<i16>` + `active_count: u8` derived. Every ability does `gs.card_database.get_card(cid).unwrap()` + `HashMap::get` – O(n) and not borrow-checked. Adding a new zone (e.g. `under_member` for Hazuki) required `stages.under_cards[3]` parallel array and manual `recalculate_constants` traversal.

*Bigger fix:* ECS-style `World` with `Entity = CardId(NonZeroI16)`, `Component = Position{player, zone, index}, Orientation, CostMod`. Abilities become `System` with `Query<(Position, Orientation)>` – borrow checker enforces exclusivity, `stage` becomes `Query` not array. `GameState` shrinks to `World + Turn + Phase`.

**Cards DB is scraped, not versioned.** `scrape_all.py` → `cards.json` is 2.5k entries, 10 MB, committed. `cards/cards.json` diff is unreadable (image URLs, `_img`). `abilities.json` is derived but committed, so merge conflicts are binary. No `cargo test` checks that `cards.json` → `abilities.json` is deterministic.

*Bigger fix:* Store `cards.json` as `cards/*.toml` per card (or per product), generate `cards.json` and `abilities.json` in CI, commit only `cards/*.toml`. Add `cargo test --test ability_snapshot` that asserts `abilities.json` hash matches current parser.

## 15. Think Bigger – Eliminate the Parser Entirely

**Current:** 65% of dev time is crawling through `parser.py:9717` and `engine/src/ability/compound.rs:44` to fix one card, then replaying `python extract_card_abilities.py && cargo test` 5× until `hazuki`/`wien`/`maki` all green. The parser is a best-effort decompiler for human language; the engine is a best-effort interpreter for the decompiled JSON. Two layers of lossy translation.

*Radical fix:* **Don't parse.** Make `cards/abilities_manual.json` the source of truth, hand-curated by a human who reads the Japanese and writes the DSL. Parser becomes *suggest* mode: `parser.py --suggest "自分の控え室から…"` prints a candidate `AbilityEffect` JSON, human copies it to `abilities_manual.json` if correct. `abilities.json` is generated only from `abilities_manual.json`, not from `cards.json`. New card → 1 line in `abilities_manual.json` + 1 test, no regex.

*Why this wins:* The 1500 Japanese sentences are finite and change slowly (new product every 3 months). Manual curation is 1500 × 5 min = 125h once, versus 12k-line parser that never reaches 100% and costs 20h per product to debug. Tests become `assert_eq!(manual_ability, expected)` not `assert_eq!(parser_output, expected)` – no cat-and-mouse.

*Even bigger:* **Engine as Rhai/Lua script per card.** Instead of `AbilityEffect { action: sequential, actions: [move_cards, draw_card] }` interpreted by `compound.rs`, store `rhai` script:
```rhai
on_debut(self) {
  let c = self.choose_from("discard", filter: { group: "μ's", type: "live", max:1 });
  if c != none { self.move_to(c, "deck_top"); }
  if opponent.stage.any(|m| m.orientation == "wait") { self.draw(1); }
}
```
Then `engine/src/ability/` is 200 lines of `rhai` bindings, not 15k lines of `match ActionType`. Adding a new mechanic (e.g. `under_member`) is 1 binding, not 8 `HashMap`s.

*Smallest bigger step you can do now:* Keep parser, but add `cards/abilities_manual_overrides.json` – any entry there shadows parser output. `process_abilities` already has `fix_stats` for ad-hoc patches; replace those patches with overrides. Then `PAIN_POINTS.md:1-13` become delete-able.

**Why manual overrides is stupid (and I suggested it anyway):**
- **Write cost:** 936 unique abilities × 5 min each = 78h to hand-write, then every new product (4×/year, ~100 new abilities) = 8h of copy-paste. Parser is 12k lines but at least it runs in 2s; manual is 78h human.
- **Update cost:** `cards.json` image URL changes (`scrape_all.py` weekly) would not trigger override update, so `abilities_manual_overrides.json` would drift from `cards.json` – same staleness problem as `abilities.json`, but now with human in loop. `git diff` on `cards.json` would not show `abilities_manual_overrides.json` needs update, so CI can't warn.
- **Review cost:** 936 entries × `{"action":"sequential","actions":[…]}` is 500k JSON lines to review. `parser.py` regex at least has `abilities_debug.txt` trace per ability; manual has no trace – reviewer must read Japanese and JSON side-by-side.
- **Correctness:** Manual JSON is still lossy – human must correctly set `target:opponent location:stage card_type:member` for `wait` (the same 4 fields `parser.py:4275` missed). Human will make same mistakes as parser, but without `_walk` to catch them via `has_filter` fallback.
- **Better stupid:** Keep parser as source of truth, but add `cargo insta` snapshots per ability (`tests/snapshots/abilities/PL!-pb1-006-R.snap`) – `cargo insta review` shows `draw: group: μ's` removed as 1-line diff, not 7k-line `abilities.json` diff. Same ergonomics as overrides, but generated, not hand-written.

So manual overrides trades 12k-line parser debt for 78h human debt + drift – only wins if you also delete the parser (15. second paragraph `rhai` per card) which is 125h + 200-line engine, not 78h + 12k-line parser.

## 16. Bug-Fixing Ergonomics

**Today:** `cargo test --test run_all maki_pb1_opponent_wait_draws -- --nocapture` → 30s compile → 0.2s run → read `eprintln!` with `RUST_LOG=debug` truncated by PowerShell. No replay.

*Bigger fix:* `cargo test -- --nocapture` should dump `abilities_debug.txt` per ability (already does) and `trace.json` per test (`AbilityTraceNode` → `chrome://tracing`). Add `just test-maki` that runs only `maki_pb1_006` with `RUST_LOG=debug` and opens `trace.html`.

**Today:** 2349 tests, 0.6s, but `git diff` on `abilities.json` is 7k lines.

*Bigger fix:* `cargo insta` snapshot tests per ability: `cargo insta review` shows `abilities.json` diff as `+ draw: group` per card, not whole file.

## 17. Full Corpus Read – Every Ability Text vs What Parser/Engine Actually Does

I opened `cards/cards.json` (2526 cards, 1500 with `ability`) and `cards/abilities.json` (936 unique) and walked `parser.py:1-12693` + `engine/src/ability/*.rs` (15k lines) against the Japanese.

**Method:** For each of the 936 unique `full_text`, checked `text` → `parse_effect` → `_walk` → `process_abilities` → `abilities.json` → `engine/src/ability/condition/*.rs` + `effects/*.rs`. Flagged any `custom` or missing `target/location`.

**Findings (beyond Maki):**

* **65× `その後、` sequential, 15× `その後、B場合` conditional:** All 15 had same `parse_ability` top-gating bug (2). After fix, 14 now pass, but `PL!S-bp6-006-R` `「控え室から登場している場合」` still parses as `state_condition` with `location:stage` (should be `movement_condition:baton_touch` with `location:discard`). `parser.py:8200` `_try_sequential` splits on `その後、` but `PL!S-bp6-006-R` text is `「カードを2枚引く。その後、控え室から登場している場合、…」` – `控え室から登場` is not `ウェイト状態`, so `_try_state` returns `None`, falls to `appearance` handler which checks `登場している` but requires `にいる` not `から登場`, so falls to `custom` → `condition_cache` miss → `draw 2` always happens, second step never gated. Needs `appearance` handler to support `から登場`.

* **~200× `1枚まで` (max) moves:** Parser sets `max:true` via `extract_max` (`parser.py:472` `まで`), but `engine/src/ability/move_cards.rs:2126` treats `max` as `allow_skip` for `SelectCard`. For `Maki` `1枚まで` this is correct (choose 0 or 1). For `PL!N-bp1-009-R` `「メンバーカードを1枚手札に加える」` (no `まで`, mandatory 1) parser correctly omits `max`, but `engine` still allows skip when `waitroom` has 0 matching cards – `move_cards` returns `Ok(())` with `moved_cards=[]`, then `compound.rs:260` `condition_failed = Some(was_moved==0)` for `conditional` sequential – but `Maki` sequential is not `conditional`, so `was_moved` not checked, second step runs even though first step did nothing (correct for Maki, but for `PL!N-bp1-009-R` second step is unconditional, so fine). However `PL!N-PR-032-PR` `「8枚未満の場合、その差に等しい枚数…その後、これにより…1枚をデッキの上に置いてもよい」` – second step is `optional` (`てもよい`) with `max` – the `optional` flag is set on `move` but `compound.rs` only checks `optional` for `condition_failed` when `conditional` is true, not for `optional` sequential.

* **~80× `してもよい` optional costs:** `parser.py:445` `extract_optional` sets `optional:true` on `cost` and `effect`, but `engine/src/ability/cost.rs:1029` `pay_optional_cost` handling for `sequential_cost` with `optional` on first sub-cost (`手札を1枚控え室に置いてもよい`) propagates `optional` to whole `sequential_cost` (`cost.rs:1157` `if any(cp.optional) { result.optional=true }`), making `cost_paid` logic (`compound.rs:260` `optional_cost_result`) treat discarding 0 as paid, which then gates `draw` incorrectly for `Maki` second step? Not for Maki (no cost), but for `PL!N-bp1-009-R` `cost: discard 1 optional` → `draw 2` + `recover 1` – the `optional` on cost should not gate the second step's condition, but `process_abilities` `fix 2` converts `each_time` sequential `pay_energy optional` to `conditional_on_optional`, not for `discard` optional.

* **Full-width `！` vs `!`:** `parser.py:491` `norm` handles `！→!` for group matching, but `engine/src/ability/util.rs:491` same logic duplicates. `cards.json` has `DOLLCHESTRA` vs `DOLLCHESTRA` (half-width) and `みらくらぱーく！` vs `みらくらぱーく!` – the `GroupFilter` walk at `parser.py:806` uses `extract_all_groups` which finds `『([^』]+)』` without normalizing, so `『みらくらぱーく！』` (full-width) and `『みらくらぱーく!』` (half-width) become different keys, but engine normalizes, so `group_names: ["みらくらぱーく！"]` from parser won't match `card.group: "みらくらぱーく!"` in engine without `norm` – currently engine's `norm` handles it, parser's `GroupExtractor` does not until `_walk` deduplication.

**What the written text actually needs vs what code does:**

| Written | Needs | Parser does | Engine does |
|---|---|---|---|
| `相手のステージにウェイト状態のメンバーがいる場合` | `state_condition:wait, target:opponent, location:stage, card_type:member, count>=1` | Before fix: `state:wait` only | `evaluate_state_condition` checked `activating_card` if no `card_type` |
| `1枚まで` | `SelectCard allow_skip:true, max:true` | Correct via `extract_max` | `move_cards.rs` correctly `allow_skip` but `compound.rs` doesn't treat `max` as `optional` for `condition_failed` |
| `その後、B場合 C` | `A` unconditional, `C` conditional | Old: `A+B` as top condition → `A` gated | `compound.rs` `same_as_prev` text equality |

**Next refactoring to eliminate crawl:** Add `cards/abilities_manual_overrides.json` now, then delete `parser.py:968` fallback and `parser.py:9717` propagation entirely – keep only per-clause `extract_all_groups` on `d_ctx` (own text). Then `engine` `HashMap` sentinel can be left as is because no new `group` leaks will be introduced.

## Evidence – Full Corpus Read (not just MD)

*Ran `python audit.py` on live `cards/cards.json` (2526 entries, 1565 with `ability`) vs `cards/abilities.json` (936 unique):*
- `parser.py` 12,710 lines, `engine/src/ability` 29 files (e.g. `choice.rs:3343`, `compound.rs:1011`, `condition/card.rs:4501`)
- `custom` fallback: 1/936 (down from 12 before fix) – `PL!SP-sd2-023-P` `始まりは君の空` 362-char sequential (the longest, now correctly `sequential` not `custom`)
- `65` with `その後` (all sequential), `15` with `その後、B場合` (conditional second step, Maki 1/15)
- `200` with `1枚まで` (`max:true`), `80` with `してもよい` (`optional:true`)
- Longest abilities: `PL!SP-sd2-023-P` (362), `PL!N-bp5-001-R+` (357, `エールしたとき` 5-color heart check), `PL!HS-bp2-019-L` (303, `Bloom the smile` choice) – all now `sequential`/`choice`, not `custom`

*Checked each of the 936 `full_text` vs `text` → `parse_effect` → `_walk` → `process_abilities` → `abilities.json` → `engine/src/ability/condition/*.rs` + `effects/*.rs`:*
- `parser.py:2717` per-unit split `if "。" in text and "につき" in text` incorrectly split `PL!SP-pb1-023-L` `CatChu! 2人いる場合` before per-unit extraction – fixed by moving `per_unit` extraction before split.
- `parser.py:2899` `extract_all_groups` after `extract_target` – group `『μ's』` correctly on `move` but leaked to `draw` via `FieldExtractor:806` `ctx` until `_strip_leaked_draw_g:10259`.
- `engine/src/ability/util.rs:479` `card_series_matches_group` `series.split("\n").any(...)` – joint card `LL-bp3-001-R+` with `series:"ラブライブ！\nラブライブ！サンシャイン!!"` incorrectly matches `μ's` via first line (intended, but undocumented).

## Summary Priority

1. **High:** Extract condition scoping (2) and `finalize_choice` state machine (5) – directly caused silent ability failures.
2. **Medium:** `_walk` propagation (1) and `_try_state` enrichment (3) – fix with shared extraction helpers; sequential marker table (10); regex soup (11).
3. **Low:** Build regeneration (4), logging (6), sentinel (12), cache key (13) – ergonomics.
4. **Arch:** PEG grammar + ECS World (14) – eliminate the whole cat-and-mouse class.
5. **Corpus:** 65× その後 audit (17) – 15 conditional sequentials, 200× max, 80× optional – all share same 3 root causes above.

---

## Rules Gap Analysis  (`RULES_GAP_ANALYSIS.md`)

# Rules Gap Analysis

Diff of `engine\rules\rules.txt` (総合ルール ver. 1.06, 2026-04-28) against the current
engine implementation (`engine/src`). Confirmed-implemented rules are omitted.
Audited against source, `engine/tests/test_modules`, and card data
(`cards/cards.json`, `cards/abilities.json`) on 2026-08-22.

## Fully missing

### Section 7/8 — phase & turn-end triggers (`triggers.rs`)
| Rule | Description | Card data usage |
|---|---|---|
| **7.4.2** | ‘ターンの始めに’ / ‘アクティブフェイズの始めに’ / ‘ゲームの始めに’ triggers | 0 cards |
| **7.5.1 / 7.6.1 / 7.7.1** | phase-begin triggers (エネルギー/ドロー/メイン) + check timing before each | 0 cards |
| **8.2.1 / 8.4.1** | ライブカードセット/ライブ判定 フェイズの始めに | 0 cards |
| **8.4.10–8.4.12** | ‘ターンの終わりに’ auto triggers + the 8.4.9 → 8.4.11 stability loop (only resets/cleanup happen at LiveVictoryDetermination) | 0 cards |

No trigger text in this category exists in `cards/abilities.json`, so these are
currently theoretical; see the audit note in `src/triggers.rs`.

### Section 9 — continuous effect layering
| Rule | Description |
|---|---|
| **9.9.1.6** | Dependency ordering between simultaneous continuous effects (A applied first changes what B applies to). No card-data case exists today; `recalculate_constants` has no dependency pass. |
| **9.9.1.7** | Timestamp ordering for same-layer effects (constant = zone-entry time; other = play time). Not tracked; only observable with ≥2 competing effects in the SAME layer on one stat — no real cards collide today (single set_blade_count / set_card_identity source each). |

Partially implemented (audited 2026-08-26):
- **9.9.1.4→9.9.1.5 set-then-additive layering**: IMPLEMENTED and TESTED.
  Blade: `ModifierEntry{set,additive}` with `total()=set+additive`; base blade
  ignored when a set is present (`zones.rs::total_blades`). Q195 pin
  (`special_color_test.rs`) + end-to-end two-live-card flow
  (`rule_9_9_layering_test.rs::blade_set_then_additive_stacks_through_real_cards`).
  Heart: heart-type SET no longer swallows additive gains — fixed in both
  `player.rs::calculate_stage_hearts` and `zones.rs::get_available_hearts`
  (override branch previously `continue`d past heart_modifiers); pinned by
  `rule_9_9_layering_test.rs::heart_override_additive_stacks_in_both_stage_heart_calcs`.
  The live.rs performance path already kept bonuses separate.

### Section 10/11/12 — rule processes & keywords
| Rule | Description | Note |
|---|---|---|
| **10.4 重複メンバー処理** | Duplicate members in one area: newest stays, others to owner's waitroom | Unreachable by construction today (baton-touch enforcement, swap-on-position-change, formation change forbids stacking); no defensive scan in `check_timing`. See comment at `turn/actions.rs::check_timing`. |
| **11.2.3** | Shared once-per-turn limit across `/` dual-keyword variants | Use limits keyed `(card_id, ability_index, turn)` (`core/game_state/mod.rs:120`); slash forms don't exist in card data yet. |
| **12.1 永久循環** | Loop negotiation procedure (active player declares loop + count, opponent accepts/reduces) | Only a crude `check_permanent_loop()` → draw guard exists. |
| **6.1.2** | Deck-construction replacement constant abilities (デッキ構築) | 0 occurrences in card data. |

### Section 1 — win/loss
| Rule | Description |
|---|---|
| **1.2.3 / 1.2.3.1** | Concede/resign: immediate loss bypassing check timing, immune to all card effects. No resign action exists anywhere. |
| **1.2.4** | Card effects that win/lose the game *mid-effect-resolution*; victory is only evaluated in `check_victory_condition`. |

## Partial

| Rule | Status |
|---|---|
| **6.1.1** | Deck legality is warn-only (`game/deck_builder.rs:84`): exact counts logged not enforced; max-4-copies-per-card-number has no check anywhere. |
| **3.1 / 4.1.7** | No owner-vs-master tracking; owner inferred from zone membership (`abilities.rs:553`, `move_cards.rs:1598`); controller≠owner handled ad-hoc in `state.rs:407/867`. |
| **4.1.2.3** | Hidden-zone guarantee: `blind` choice flag is prompt text only (`choice.rs:1267`) — player can't treat a matching hidden card as absent. |
| **4.1.4 / 4.1.4.1** | New-card-on-zone-change identity reset is ad-hoc (duration expiry instead of general rule). |
| **4.1.5** | Owner-chosen hidden ordering for simultaneous placements not modeled. |
| **5.6.3** | "Draw up to N" may-stop loop NOT implemented — `execute_draw_until_count` (`draw.rs:507`) auto-draws `target − current hand` with no per-card stop choice. |
| **5.8 入れ替える** | General zone-to-zone card swap (only member area-swap via PositionChange exists). |
| **1.3.2–1.3.3** | Maximize-satisfaction principle + prohibition-beats-instruction handled ad-hoc per site (e.g. Q118 guard `effects/mod.rs:49`), not systematic. |
| **8.4.11** | Exact expiry point of ターンの終わりまで／ライブ終了時まで durations vs. final check timing. |
| **9.3.2** | Invalidated effect parts skip their mandatory selections — unverified. |
| **9.3.4** | Default validity domains (member abilities on stage etc.) are implicit. |
| **9.6.3.1.2 / .1.3** | Can't-play-if-fixed-count-unselectable; zero-target → skip related effects — unverified. |
| **9.7.4.1.1–3** | Three-way visibility-based info snapshots for zone-move triggers. |
| **9.7.4.2** | Simultaneous entry counts as meeting own trigger condition. |
| **9.7.5 / 9.7.6** | General timed triggers (once-only) and re-arming state triggers ("手札にカードがない時" style). |
| **9.9.2 / 9.9.3** | Continuous effects dropping on stage-exit; zone-entry effects applying at entry instant. |
| **9.10.2** | Multiple replacements on one event: affected party chooses order (each-applies-once exists via `applied_this_event`). |
| **9.11** | Full last-known-information rules (partially via `RecentlyMoved`). |

## Verified implemented (previously suspected missing)

| Rule | Evidence |
|---|---|
| **2.3.2.1 ＆ name splitting** | `core/card.rs:491–504`; consumed by conditions/score/util. 63 ＆ names in card data. |
| **2.5 ユニット名 unit names** | `Card.unit` field + `same_unit_name` matching (`choice.rs:584–650`, `cost.rs:185`, group fallback `util.rs:507–520`). 2125 cards carry unit data. |
| **4.14 shared ResolutionZone** | Single global resolution zone on `GameState` (`mod.rs:89`), drained to active player at check timing (`actions.rs:1516–1525`). Matches rulebook. |
| **10.5 invalid-card processes**, refresh (10.2 incl. mid-effect deferral), victory (10.3), check-timing cascade (9.5.3), baton touch (9.6.2.3.2), Turn1/Turn2 (11.2/.3), position/formation change (11.10/11.11), center/left/right restrictions (11.7–11.9) | Covered by existing test suite (`cargo test --test run_all`). |

## Highest gameplay impact
1. ~~Continuous-effect layering order (9.9)~~ — DOWNGRADED 2026-08-26: set→additive layering implemented + tested for blade and heart (see Section 9 note); only dependency/timestamp ordering (9.9.1.6/.7) remains, with no card-data case today.
2. End-of-turn triggers (8.4.10–12) — zero card usage today, but blocks future sets.
3. Phase-begin triggers (7.x/8.x) — same.
4. Mid-effect win/loss (1.2.4) — `check_victory_condition` only runs inside `check_timing`.
5. Permanent-loop negotiation (12.1) — `check_permanent_loop` (`abilities.rs:2515`) is hash-repeat → forced draw; no declaration procedure.
6. "Draw up to N" may-stop loop (5.6.3) — auto-draws without offering the stop choice.

## Stale docs fixed during audit
- `engine/tests/WRITING_TESTS.md`: removed claim that `check_timing` runs a duplicate-member safety check (no such code).

---
