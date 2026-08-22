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

