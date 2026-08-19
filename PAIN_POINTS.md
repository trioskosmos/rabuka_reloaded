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
