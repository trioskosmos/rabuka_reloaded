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

### Current work directive (user, 08-24)
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
