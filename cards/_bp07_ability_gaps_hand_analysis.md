# BP07 / NSD02 Ability Gap Audit — Full Text-vs-JSON Comparison Report

Date: 2026-08-05
Scope: all **31** abilities that the automated regex analyzer
(`_new_abilities_analysis.json` → `_gaps_readable.txt`) flagged as `needs_parser_work`.

Method: for every flagged ability I pulled its `full_text` (the real Japanese) and its
actual `effect` JSON from `cards/abilities.json`, then compared clause-by-clause.

Legend:
- ✅ **OK** — parser output faithfully represents the Japanese. Flag was spurious.
- ⚠️ **Field gap** — structure is right but a field is missing/wrong (e.g. `location`, target).
- ❌ **Structure bug** — parser emitted wrong action type or dropped the effect.

---

## Group A — Parsed correctly (16 of 31 flags are false positives)

### A1. `PL!S-bp7-007-R＋` 国木田花丸 ab#1 — gaps: [under_member, placement_order] — ✅ OK

Japanese:
> 自分の控え室から『Aqours』のメンバーカードを3枚まで好きな順番でデッキの下に置く。ライブ終了時まで、これによりデッキに置かれたカード1枚につき、ブレードを得る。

Parsed:
```json
{"action":"sequential","actions":[
  {"action":"move_cards","source":"discard","destination":"deck_bottom",
   "placement_order":"any_order","count":3,"max":true,"group_names":["Aqours"]},
  {"action":"gain_resource","resource":"blade","count":1,"per_unit":true,
   "per_unit_count":1,"per_unit_type":"枚","per_unit_source":"previous_moved_cards",
   "duration":"live_end"}]}
```
Comparison:
- "3枚まで好きな順番でデッキの下に置く" → `count:3, max:true, placement_order:any_order, destination:deck_bottom` ✅
- "1枚につきブレードを得る" → `per_unit + per_unit_source:previous_moved_cards` ✅
- The `under_member` flag is meaningless here (no メンバーの下 in the text). **Spurious.**

### A2. `PL!N-bp7-006-R＋` 近江彼方 ab#0 — gaps: [placement_order] — ✅ OK

Japanese:
> E：自分のデッキの上からカードを4枚見る。その後、それらを好きな順番でデッキの上に置く。

Parsed: `cost{pay_energy:1}` + `sequential[look_at{deck_top,4}, move_cards{destination:deck_top, placement_order:any_order, source:looked_at}]`
- "好きな順番でデッキの上に置く" → `placement_order:any_order, destination:deck_top` ✅
**Spurious.**

### A3. `PL!N-bp7-007-R＋` 優木せつ菜 ab#0 — gaps: [under_member] — ✅ OK

Japanese:
> 常時：このメンバーの下にあるエネルギーカード1枚につき、heart02を得る。

Parsed: `gain_resource{heart, heart_colors:[heart02], count:1, per_unit, card_type:energy_card, location:under_member}`
- "このメンバーの下にあるエネルギーカード" → `location:under_member, card_type:energy_card` ✅
**Spurious.**

### A4. `PL!N-bp7-007-R＋` 優木せつ菜 ab#2 — gaps: [under_member, energy_deck] — ✅ OK

Japanese:
> ライブ成功時：自分のエネルギーデッキから、エネルギーカード1枚をこのメンバーの下に置く。

Parsed:
```json
{"action":"place_energy_under_member","source":"energy_deck","destination":"under_member",
 "count":1,"card_type":"energy_card","target":"self","energy_count":1}
```
- "エネルギーデッキから…メンバーの下に置く" → `source:energy_deck, destination:under_member, action:place_energy_under_member` ✅
This is the **reference example** the other energy-deck-under-member cards should be producing. **Spurious.**

### A5. `PL!N-bp7-011-R＋` ミア・テイラー ab#1 — gaps: [under_member] — ✅ OK (acceptable)

Japanese:
> 常時：このカードをプレイする際、自分の控え室にあるすべてのメンバーカードをシャッフルし、デッキの下に置いてもよい。そうしたとき、このカードのコストは２減る。

Parsed: `condition{location_condition, location:discard, locations:[discard,deck], card_type:member_card, shuffle:true, all:true}` + `modify_cost{operation:subtract, value:2, shuffle:true, all:true}`
- The shuffle-to-deck-bottom is expressed via the `shuffle:true` flag + `locations:[discard,deck]`, cost reduction via `modify_cost`. No explicit `move_cards`, but the shuffle flag carries the intent and the engine implements it. `under_member` flag has no basis in the text. **Spurious / acceptable.**

### A6. `PL!SP-bp7-003-R＋` 嵐 千砂都 ab#1 — gaps: [under_member] — ✅ OK

Japanese:
> 常時：このメンバーの下にメンバーカードが3枚以上置かれているかぎり、ライブの合計スコアを＋１する。

Parsed: `condition{card_count_condition, count:3, ">=", card_type:member_card, location:under_member}` + `modify_score{value:1, operation:add}`
- "このメンバーの下に3枚以上" → `location:under_member, count:3, >=` ✅ **Spurious.**

### A7. `PL!SP-bp7-003-R＋` 嵐 千砂都 ab#2 — gaps: [under_member] — ✅ OK

Japanese:
> 起動：手札のコストが10か20のメンバーカードを1枚公開する：これにより公開したカードをこのメンバーの下に置く。その後、カードを2枚引く。

Parsed: `cost{reveal, source:hand}` + `sequential[move_cards{source:revealed_cards, destination:under_member, dynamic_count:previous_reveal}, draw_card{2}]`
- "公開したカードをこのメンバーの下に置く" → `source:revealed_cards, destination:under_member` ✅ **Spurious.**

### A8. `PL!SP-bp7-006-R＋` 桜小路きな子 ab#0 — gaps: [energy_deck] — ✅ OK

Japanese:
> 登場：エネルギー置き場にあるエネルギー1枚をエネルギーデッキに置いてもよい：自分の控え室にある『Liella!』のメンバーカードを1枚手札に加える。

Parsed: `cost{move_cards, destination:energy_deck, count:1, optional:true}` + `effect{move_cards discard→hand, member_card, Liella!}`
- "エネルギー置き場…をエネルギーデッキに置いてもよい" → `cost{destination:energy_deck, optional:true}` ✅ **Spurious.**

### A9. `PL!SP-bp7-006-R＋` 桜小路きな子 ab#1 — gaps: [energy_deck] — ✅ OK

Japanese:
> ライブ成功時：センター このターン、自分のエネルギーがエネルギー置き場からエネルギーデッキに置かれていた場合、ライブの合計スコアを＋１する。

Parsed: `condition{card_count_condition, trigger_event{zone_change, source:energy_zone, destination:deck}, temporal:this_turn, position:center}` + `modify_score{value:1, position:center}`
- "このターン…エネルギー置き場からエネルギーデッキに置かれていた場合" → `trigger_event{zone_change, source:energy_zone→deck}, temporal:this_turn` ✅ **Spurious.**

### A10. `PL!SP-bp7-007-R＋` 米女メイ ab#0 — gaps: [energy_deck] — ✅ OK

Japanese:
> ライブ開始時：エネルギー置き場にあるエネルギー2枚をエネルギーデッキに置いてもよい：ライブ終了時まで、ブレード×3を得る。

Parsed: `cost{move_cards, destination:energy_deck, count:2, optional:true}` + `gain_resource{blade, count:3, duration:live_end}` ✅ **Spurious.**

### A11. `PL!SP-bp7-007-R＋` 米女メイ ab#1 — gaps: [energy_deck] — ✅ OK

Japanese:
> ライブ成功時：自分のエネルギーデッキから、エネルギーカードを2枚ウェイト状態で置く。それらのエネルギーカードは、次のターンのアクティブフェイズにアクティブしない。

Parsed: `sequential[move_cards{source:energy_deck, destination:energy_zone, state_change:wait, count:2}, restriction{restriction_type:cannot_active, delayed:true}]`
- "エネルギーデッキから…ウェイト状態で置く" → `source:energy_deck, state_change:wait` ✅
- "次のターンアクティブしない" → `restriction{cannot_active, delayed:true}` ✅ **Spurious.**

### A12. `PL!SP-bp1-026-L` 未来予報ハレルヤ！ ab#0 — gaps: [distinct_name] — ✅ OK

Japanese:
> ライブ開始時：自分の、ステージと控え室に名前の異なる『Liella!』のメンバーが5人以上いる場合、このカードを成功させるための必要ハートは heart02×2 heart03×2 heart06×2 になる。

Parsed: `condition{location_condition, distinct:card_name, locations:[discard,stage], count:5, >=, unit:人, group_names:[Liella!]}` + `modify_required_hearts{heart_colors:[heart02,heart03,heart06], operation:set, count:2, distinct:card_name, value:2}`
- "名前の異なる…5人以上" → `distinct:card_name, count:5, >=, unit:人` ✅ **Spurious.**

### A13. `PL!N-bp7-026-L` Just Believe!!! ab#1 — gaps: [card_property] — ✅ OK

Japanese:
> ライブ成功時：エールにより公開された自分のカードの中に、ブレードハートを持たないメンバーカードが2枚以上ある場合、このカードのスコアを＋１する。

Parsed: `condition{card_count_condition, count:2, >=, negation:true, location:revealed_cards, card_type:member_card, card_property:has_blade_heart}` + `modify_score{value:1, operation:add}`
- "ブレードハートを持たない" → `negation:true, card_property:has_blade_heart` ✅ **Spurious.**

### A14. `PL!SP-bp7-004-R` 平安名すみれ ab#0 — gaps: [placement_order, card_property] — ✅ OK

Japanese:
> ライブ開始時：自分の控え室から『Liella!』のメンバーカード3枚を好きな順番でデッキの一番下に置いてもよい。これによりデッキの下に置いたカードの中にブレードハートを持たないメンバーカードが1枚以上ある場合、ライブ終了時まで、ブレード×2を得る。

Parsed: `conditional_on_result{primary_effect{move_cards discard→deck_bottom, placement_order:any_order, count:3, optional:true}, result_condition{card_count_condition, >=1, negation:true, card_property:has_blade_heart, source:preceding_moved}, followup{gain_resource{blade,2,live_end}}}`
- "好きな順番で" → `placement_order:any_order` ✅; "ブレードハートを持たない" → `negation+card_property` ✅ **Spurious.**

### A15. `PL!SP-bp7-026-L` Dears ab#0 — gaps: [energy_deck] — ✅ OK

Japanese:
> ライブ開始時：エネルギー置き場にあるエネルギー1枚をエネルギーデッキに置いてもよい：自分のステージに「葉月恋」がいる場合、カードを2枚引き、手札を1枚控え室に置く。

Parsed: `cost{move_cards, destination:energy_deck, count:1, optional:true}` + `condition{location_condition, characters:[葉月恋], location:stage}` + `sequential[draw_card{2}, move_cards{hand→discard,1}]` ✅ **Spurious.**

### A16. `PL!SP-bp7-027-L` What a Wonderful Dream!! ab#0 — gaps: [energy_deck] — ✅ OK

Japanese:
> ライブ開始時：エネルギー置き場にあるエネルギー1枚をエネルギーデッキに置いてもよい：自分のエネルギーが相手より多い場合、このカードのスコアを＋１する。

Parsed: `cost{destination:energy_deck, count:1, optional:true}` + `condition{comparison_condition, resource_type:energy, >, comparison_target:opponent}` + `modify_score{+1}` ✅ **Spurious.**

---

## Group B — Field-level gaps (structure OK, one field missing/wrong)

### B1. `PL!N-bp7-003-R＋` 桜坂しずく ab#1 — gap: [under_member] — ✅ FIXED (parser + engine)

Japanese:
> ライブ終了時まで、このメンバーの下に置かれている名前の異なるメンバーカード1枚につき、ブレードを得る。

Parsed:
```json
{"action":"gain_resource","resource":"blade","count":1,"per_unit":true,
 "per_unit_count":1,"per_unit_type":"枚","card_type":"member_card",
 "distinct":"card_name","duration":"live_end"}
```
- ✅ `distinct:card_name` (名前の異なる) is handled.
- ❌ **"このメンバーの下に置かれている" is dropped** — no `location:under_member`, so the per-unit count is not scoped to cards under the member. Without it, the parser's meaning is "count all distinct-name member cards anywhere".
- ✅ **Fixed**: added `location:"under_member"` to the `gain_resource` node (parser `_try_per_unit` now recognizes `下に置かれている` placement verbs, not only `下にある`). Engine dedups by distinct name for the per-unit count. Tests: `bp7_under_member_per_unit_blade_test.rs` (`shizuku_*`).

### B2. `PL!SP-bp7-003-R＋` 嵐 千砂都 ab#0 — gap: [under_member] — ✅ FIXED (parser + engine)

Japanese:
> 常時：このメンバーの下に置かれているメンバーカード1枚につき、ブレードを得る。

Parsed:
```json
{"action":"gain_resource","resource":"blade","count":1,"per_unit":true,
 "per_unit_count":1,"per_unit_type":"枚","card_type":"member_card"}
```
Identical defect to B1 — **"このメンバーの下に置かれている" dropped**, no `location:under_member`. 
- ✅ **Fixed**: same parser fix as B1 (recognize `下に置かれている` → `location:under_member`), no `distinct` (counts every member card under). Engine scopes the constant per-unit count to cards under the member only. Tests: `bp7_under_member_per_unit_blade_test.rs` (`chika_*`).

### B3. `PL!SP-bp7-001-R` 澁谷かのん ab#0 — gap: [under_member] — ⚠️ missing `location:under_member` on condition

Japanese:
> 常時：このカードが『Liella!』のメンバーの下に置かれているかぎり、そのメンバーはブレードを得る。

Parsed:
```json
{"condition":{"group_names":["Liella!"],"card_type":"member_card",
  "self_target":true,"type":"group_condition"},
 "duration":"as_long_as","conditional":true,
 "action":"gain_resource","resource":"blade","count":1}
```
- ✅ `group_names:[Liella!]`, `as_long_as` captured.
- ❌ **"メンバーの下に置かれている" dropped** — the condition does not say the *this card* must be `location:under_member`. This is a persistent "as long as X is under a Liella! member" effect; without the location the condition is vacuous.
- Fix: add `location:"under_member"` to the condition node.

### B4. `PL!SP-bp7-005-R＋` 葉月 恋 ab#0 — gap: [energy_deck] — ✅ DONE (parser + engine + tests)

Japanese:
> 自動：このメンバーが登場するか、自分のエネルギーがエネルギー置き場からエネルギーデッキに置かれたとき、自分のエネルギーデッキから、エネルギーカードを1枚ウェイト状態で置く。…

Fixed (2026-08-06):
- **Parser**: the generic `_try_or` split now emits a compound `or_condition` whose leg1 is a **bare** `appearance_condition` (no `card_type`) and leg2 is a `card_count_condition` with `trigger_event{zone_change, source:energy_zone, destination:energy_deck}`. `_try_or` also aggregates a top-level `trigger_event{type:"or", events:[…]}` from the legs so the engine prefilters on real events.
- **Parser invariant**: self-appearance card_type is stripped centrally by `_strip_self_appearance_card_type` (applied over the whole effect tree in `process_abilities`), so the engine's self-trigger guard works — the appearance leg requires a real debut.
- **Engine**: `resolve_moved_cards_source` now accepts the `-1` anonymous energy resource for `resource_type=="energy"` zone-change conditions (energy_zone↔energy_deck).
- **Tests**: `engine/tests/test_modules/bp7_ren_both_trigger_test.rs` (11 tests) — appearance leg, energy_zone→energy_deck leg, WAIT-not-active, once-per-turn with both legs, empty deck, no-trigger, opponent energy move, other-member appearance, multiple moves single-fire, wrong source zone, and a verdict-tree assertion via the `assert_ability!` helper.

Next: B5 近江彼方 ab#1 (compound OR condition source + live-card OR branch).

### B5. `PL!N-bp7-006-R＋` 近江彼方 ab#1 — gap: [card_property] — 🔧 IN PROGRESS (tests written first; wrong condition location + OR branch lost)

Japanese:
> 起動：デッキの上からカードを3枚控え室に置く：これにより控え室に置いたカードの中に『虹ヶ咲』のライブカードかブレードハートを持たない『虹ヶ咲』のメンバーカードがある場合、以下から1つを選ぶ。…

Parsed:
```json
{"condition":{"type":"location_condition","card_type":"member_card","location":"stage",
  "text":"これにより控え室に置いたカードの中に『虹ヶ咲』のライブカードかブレードハートを持たない『虹ヶ咲』のメンバーカードがある場合",
  "negation":true,"card_property":"has_blade_heart","group_names":["虹ヶ咲"],"target":"self"},
 "action":"choice","options":[...]}
```
- ✅ `card_property:has_blade_heart + negation` represents "ブレードハートを持たない".
- ❌ **`location:"stage"` is wrong** — the condition is about cards *just placed into the discard* ("これにより控え室に置いたカードの中に"), i.e. the cost's moved cards (source should be `preceding_moved`/recently moved, location discard), not the stage.
- ❌ **The OR branch "ライブカード**か**…メンバーカード" is lost** — only `card_type:member_card` survived; the "ライブカード" alternative is gone.
- Fix: condition source → preceding_moved; card-type check must be an OR of live_card / member_card(no blade heart).

### B6. `PL!N-bp7-028-L` Cooking with Love ab#0 — gap: [under_member, card_property] — ✅ FIXED (parser + engine)

Japanese:
> ライブ開始時：自分の控え室に『虹ヶ咲』のライブカードと、ブレードハートを持たない『虹ヶ咲』のメンバーカードがある場合、自分の控え室にあるすべてのカードをシャッフルし、デッキの下に置いてもよい。そうしたとき、…すべての『虹ヶ咲』のメンバーは heart01を得る。

Parsed:
```json
{"action":"conditional_on_optional",
 "condition":{"type":"compound","operator":"and","conditions":[
   {"type":"card_count_condition","card_type":"live_card","location":"discard","group_names":["虹ヶ咲"],"count":1,"operator":">=","target":"self"},
   {"type":"card_count_condition","card_type":"member_card","location":"discard","card_property":"has_blade_heart","negation":true,"group_names":["虹ヶ咲"],"count":1,"operator":">=","target":"self"}
 ]},
 "optional_action":{"action":"move_cards","source":"discard","destination":"deck_bottom","all":true,"shuffle":true,"target":"self"},
 "conditional_action":{"action":"sequential","actions":[
   {"action":"move_cards","source":"discard","destination":"deck_bottom","all":true,"shuffle":true,"target":"self"},
   {"action":"gain_resource","resource":"heart","heart_colors":["heart01"],"card_type":"member","all":true,"duration":"live_end","group_names":["虹ヶ咲"],"target":"self"}
 ]},
 "conditional_negation":false}
```
- ✅ `card_property:has_blade_heart + negation`; `shuffle:true`; `gain_resource{heart01, all, live_end}`.
- ✅ **Fixed**: condition now checks **discard** (控え室), as an **AND** of (虹ヶ咲 live card present) + (虹ヶ咲 member **without** blade heart present), via `compound`/`and` of two `card_count_condition`s. The してもよい gate is a `conditional_on_optional` (Skip/Pay); accepting runs the sequential [shuffle all discard → deck bottom, all 虹ヶ咲 members gain heart01].
- `under_member` flag: no メンバーの下 anywhere in the text → spurious.
- Engine fixes: `execute_gain_resource` no longer hijacks a multi-target gain (has `group_names`/`all`) onto the single activating card — it routes to the computed `heart_targets`. Parser: `_walk_extract_heart_colors` no longer inherits parent-context heart colors onto blanket `all:true` `move_cards`, and `_clean_action_list` doesn't propagate `heart_colors` onto plain `move_cards`. Tests: `bp7_cooking_with_love_test.rs` (`cooking_*`).

---

## Group C — Structure bugs (parser emits wrong action or drops the effect)

### C1. `PL!S-bp7-009-R` 黒澤ルビィ ab#0 — gap: [lose_resource] — ✅ FIXED (parser + engine + tests)

Japanese:
> 常時：このメンバーの正面のエリアにいるコスト4以下のメンバーは、ブレードを1つ**失う**。

Parsed:
```json
{"action":"gain_resource","sign":"negative","resource":"blade","count":1,"cost_limit":4,"cost_limit_operator":"<=","position":"front"}
```
- ✅ **Parser fix**: Un-indented `cost_limit` / `cost_limit_operator` extraction in `parser.py` and updated `_set_lose_resource_fields` so negative blade debuffs emit `action:"gain_resource"`, `sign:"negative"`, `cost_limit:4`, `cost_limit_operator:"<="`, `position:"front"`.
- ✅ **Deserializer fix**: Updated `EffectFilter` position deserialization in `card.rs` and `cost_limit_operator` parsing in `AbilityEffect::from_value`.
- ✅ **Engine fix**: Updated `recalculate_constant_blade_modifiers()` in `modifiers.rs` to handle `position: "front"` (targeting opponent mirrored slot `2 - slot_idx`) and `sign: "negative"`.
- ✅ **Tests**: 9 unit tests passing in `bp7_ruby_front_blade_test.rs`.

### C2. `PL!N-bp7-005-R` 宮下 愛 ab#0 — gap: [under_member, energy_deck, distinct_name] — ✅ FIXED (parser + engine)

Japanese:
> 登場：自分のステージに名前の異なる『DiverDiva』のメンバーが2人いる場合、以下から1つを選ぶ。・エネルギーを2枚アクティブにする。・自分のエネルギーデッキから、エネルギーカード1枚を自分のステージにいる『虹ヶ咲』のメンバーの下に置く。

Parsed:
```json
{"condition":{"type":"location_condition","target":"self","distinct":"card_name",
  "count":2,"operator":">=","unit":"人","location":"stage","group_names":["DiverDiva"]},
 "action":"choice","options":[
   {"action":"change_state","state_change":"active","count":2,"card_type":"energy_card"},
   {"action":"place_energy_under_member","source":"energy_deck","destination":"under_member",
    "count":1,"energy_count":1,"card_type":"energy_card","target":"self","group_names":["虹ヶ咲"]}]}
```
- ✅ **Fixed**: the `distinct:card_name` condition now carries `count:2` (from "2人" — `_try_distinct` previously only parsed "N以上"; added a `(\d+)人いる|ある` fallback), so 1 DiverDiva member no longer satisfies it.
- ✅ **Fixed**: option 2 now emits `place_energy_under_member{source:"energy_deck", destination:"under_member", group_names:["虹ヶ咲"]}` (parser_utils gained `energy_deck` source + `メンバーの下に置く` under_member destination patterns).
- ✅ **Engine**: `execute_place_energy_under_member_impl` now handles `source:"energy_deck"` — draws from the energy deck and places under the activating member when it matches the group filter (else the first matching member). Tests: `bp7_ai_choice_under_member_test.rs` (`ai_*`).

### C3. `PL!SP-bp7-001-R` 澁谷かのん ab#1 — gap: [under_member] — ✅ FIXED (behavior verified + edge tests)

Japanese:
> 自動：このメンバーがステージから控え室に置かれたとき、バトンタッチしていた場合、このカードをそのバトンタッチで登場したメンバーの下に置く。

Parsed (fixed):
```json
{"condition":{"type":"movement_condition","movement":"baton_touch","target":"self",
  "baton_touch_trigger":true,"trigger_event":{"type":"baton_touch","tense":"past","location":"discard"}},
 "destination":"under_member","card_type":"member_card","action":"move_cards",
 "self_target":true,"count":1}
```
- ✅ The baton-touch-trigger movement condition + `move_cards{destination:under_member, self_target}` now represent the departing-member perspective ("バトンタッチしていた場合 … その…メンバーの下に置く").
- ✅ Engine: when a member is baton-touched over, the displaced member goes to the waitroom and ab#1 moves it under the arriving member (the one in its slot).
- ✅ Tests: `bp7_kanon_baton_touch_replace_test.rs` (`kanon_*`) — positive placement, no-baton-touch stays in waitroom, kanon-as-arriver not displaced, slot-specific host.


### C4. `PL!N-bp7-003-R＋` 桜坂しずく ab#0 — gap: [under_member] — ✅ FIXED (parser + engine)

Japanese:
> 起動：デッキの上からカードを5枚控え室に置く：自分の控え室にあるコスト17以下の『虹ヶ咲』のメンバーカード1枚を**このメンバーの下に置く**。そうしたとき、ライブ終了時まで、このメンバーが元々持つハートは、これにより下に置いたメンバーカードが持つハートと同じになる。

Parsed (fixed):
```json
"cost":{"move_cards","deck_top→discard",5},
"effect":{"action":"sequential","conditional":true,"actions":[
  {"action":"move_cards","source":"discard","destination":"under_member",
    "cost_limit":17,"cost_limit_operator":"<=","count":1,"card_type":"member_card",
    "group_names":["虹ヶ咲"]},
  {"action":"set_heart_type","heart_type":null,"ref_value":"placed_under",
    "original_value":true,"self_target":true,"card_type":"member_card","duration":"live_end"}
]}
```
- ✅ Cost parsed. ✅ The placing-under-member move is now emitted as `move_cards{destination:under_member, cost≤17 虹ヶ咲 member, count:1}` as the first sequential step.
- ✅ The heart-set is the second step (`ref_value:"placed_under"` = copy the hearts of the card just placed under by the preceding move). Engine added `heart_copy` modifier (`GameModifiers.heart_copy: target member → source card`), applied in live `calculate_stage_hearts`/`get_available_hearts`/`player_perform_live`/`check_live_success`. This member's original hearts now equal the placed card's hearts.
- Parser fix: new `_try_place_under_heart_copy` handler (matches "…をこのメンバーの下に置く。そうしたとき、…ハートは…と同じになる"), registered before `_try_conditional_sequential`.

### C5. `PL!N-bp7-004-R` 朝香果林 ab#0 — gap: [under_member] — ✅ FIXED (behavior verified + tests)

Japanese:
> 起動：エネルギー置き場にあるエネルギー1枚をこのメンバーの下に置く：相手のステージにいる、元々持つブレードの数がこのメンバーの下にあるエネルギーカードの枚数に1を足した数以下のメンバー1人をウェイトにする。

Parsed (fixed):
```json
"cost":{"type":"place_energy_under_member","destination":"under_member","count":1,
  "source":"energy_zone","card_type":"energy_card"},
"effect":{"action":"change_state","state_change":"wait","count":1,"card_type":"member_card",
  "target":"opponent","original_value":true,
  "blade_limit_from_energy_under":true,"blade_limit_offset":1,"blade_limit_operator":"<="}
```
- ✅ **Fixed**: cost is `place_energy_under_member` from `energy_zone`; effect is `change_state{state_change:wait, target:opponent, original_value:true, blade_limit_from_energy_under:true, blade_limit_offset:1, blade_limit_operator:"<="}`.
- ✅ Engine correctly computes the dynamic limit `energy_under(this member) + 1` (after the cost) and compares each opponent's ORIGINAL blade.
- ✅ Tests: `bp7_karin_wait_blade_limit_test.rs` (`karin_*`) — asserts the cost places energy under 朝香果林, a blade-1 member is waited at limit 2, a blade-4 member is not, and (dynamic proof) a blade-4 member IS waited when pre-seeded energy raises the limit to 5.

### C6. `PL!S-bp7-004-R` 黒澤ダイヤ ab#0 — gap: [under_member, baton_touch, both_targets] — ✅ FIXED (parser + engine)

Japanese:
> 登場：『Aqours』のメンバーからバトンタッチして登場した場合、自分と相手はそれぞれ、自身の手札のカードを3枚まで選び、選んだカード以外のカードをシャッフルし、自身のデッキの下に置く。その後、自分と相手はそれぞれカードを3枚引く。

Parsed (abridged):
```json
"action":"sequential",
"actions":[
  {"condition":{"type":"movement_condition","movement":"baton_touch","baton_touch_trigger":true,
     "trigger_event":{"type":"baton_touch","tense":"past","location":"stage"},"group_names":["Aqours"]},
   "action":"sequential","actions":[
     {"action":"shuffle","source":"hand","count":3,"target":"energy_deck",   // ❌ target should be both
      "multiple_targets":true,"max":true,"shuffle":true,"group_names":["Aqours"]},
     {"action":"move_cards","destination":"deck_bottom","source":"hand","card_type":"card",
      "count":1,"shuffle":true}]},
  {"action":"draw_card","count":3,"target":"both","multiple_targets":true}]}
```
- ✅ `baton_touch_trigger + trigger_event{baton_touch, past}` (baton-touch flag is actually handled); `multiple_targets` on the draw.
- ❌ **`target:"energy_deck"` on the shuffle step is wrong** — should be `target:"both"` (自分と相手はそれぞれ). A random zone leaked in as the target.
- ❌ **"選んだカード以外のカード" (cards *other than* the selected up-to-3) is not expressed** — the parser just shuffles hand with `max:3` and then moves 1; the "keep 3 chosen, shuffle+put the rest under deck" semantics are not captured.
- `under_member` flag: no メンバーの下 in the text → spurious.
- Fix: the two players each select up to 3 hand cards; shuffle the *remaining* hand to deck bottom; then both draw 3. Needs a selection-then-move-excluding-selected structure.

**FIXED (parser + engine):**
- Parser: the "『X』のメンバーからバトンタッチして登場した場合" gate was being dropped (the extract pipeline uses `parse_effect`/`_normalize_effect_tree`, not `parse_ability`). Added `_attach_baton_touch_from_group_condition` (called from `_normalize_effect_tree`) which attaches the `movement_condition{baton_touch, baton_touch_trigger, group_names:[X]}` as the effect's condition and strips the leaked `baton_touch_trigger`/`group_names` off the action steps (they are a gate, not a target filter).
- Engine: the baton_touch condition's `group_names` now reach the resolver (was decoding to None before because the strip removed them from the condition); the gate correctly rejects a baton-touch source whose group isn't 『Aqours』.
- The keep-≤3/shuffle-rest-under/draw-3 both-player structure is `select{keep_shuffle_under, max, target:both}` + `draw_card{3, both}`.
- Tests: `bp7_dia_both_hand_reorder_test.rs` (`c6_*`) — incl. new `c6_non_aqours_baton_touch_does_not_fire` (source-group gate).

### C7. `PL!S-bp7-004-R` 黒澤ダイヤ ab#1 — gap: [under_member, placement_order] — ✅ FIXED (parser + engine)

Japanese:
> ライブ開始時：自分のデッキの**下**からカードを3枚見る。その中から好きな枚数を好きな順番でデッキの**下**に置き、残りを控え室に置く。

Parsed (fixed):
```json
{"action":"look_and_select",
 "look_action":{"action":"look_at","count":3,"target":"self","source":"deck_bottom"},
 "select_action":{"action":"select_cards","discard_remaining":true,
   "reveal":false,"destination":"deck_bottom","placement_order":"any_order",
   "any_number":true,
   "text":"好きな枚数を好きな順番でデッキの下に置き、残りを控え室に置く"}}
```
- ✅ **Fixed**: `look_action` now has `source:deck_bottom`; `select_action` now has `destination:deck_bottom` + `placement_order:any_order` + `any_number` (added the deck_BOTTOM pattern to `_build_look_select_actions`, mirroring the existing deck_top one).
- ✅ **Engine**: `execute_look_at` now handles `Zone::DeckBottom` by DRAINING the bottom N cards from the deck (previously it fell into the generic `zone_cards` copy branch, leaving the cards in the deck and duplicating them). Kept cards go back on the deck bottom, the rest to the waitroom.
- Tests: `bp7_dia_look_bottom_select_test.rs`.

### C8. `PL!S-bp7-008-R` 小原鞠莉 ab#0 — gap: [under_member, placement_order] — ✅ FIXED (parser + engine)

Japanese:
> 登場：自分のデッキの上からカードを3枚見る。その中から好きな枚数を好きな順番でデッキの上に置き、残りを**好きな順番でデッキの下**に置く。

Parsed (fixed):
```json
{"action":"look_and_select",
 "look_action":{"action":"look_at","source":"deck_top","count":3,"target":"self"},
 "select_action":{"action":"select_cards",
   "destination":"deck_top","placement_order":"any_order","any_number":true,
   "reveal":false,
   "remainder_destination":"deck_bottom","remainder_placement_order":"any_order",
   "text":"好きな枚数を好きな順番でデッキの上に置き、残りを好きな順番でデッキの下に置く"}}
```
- ✅ **Fixed**: `select_action` now carries `destination:deck_top` (any_order) for the selected cards AND `remainder_destination:deck_bottom` (any_order) for the rest — the rest goes to the deck bottom, NOT the discard.
- ✅ **Engine**: added `remainder_destination` / `remainder_placement_order` effect fields (decoder + `EffectFilter` + getters) and made `handle_select_cards_looked_at` honor `remainder_destination` when placing the remaining cards. This also activates the previously-latent `deck_top`-remainder pattern.
- Tests: `bp7_mari_look_top_split_test.rs`.

### C9. `PL!HS-PR-035-PR` 百生吟子 ab#0 — gap: [placement_order] — ✅ FIXED (parser)

Japanese:
> 登場：相手の控え室にあるメンバーカードを3枚選び、相手のデッキの下に好きな順番で置いてもよい。そうした場合、相手のステージにいる元々持つブレードの数が3つ以下のメンバー1人をウェイトにする。

Parsed (fixed):
```json
"action":"sequential","actions":[
  {"action":"select","source":"discard","count":3,"card_type":"member_card",
   "target":"opponent","optional":true,"placement_order":"any_order"},
  {"action":"move_cards","source":"selected_cards","destination":"deck_bottom",
   "count":0,"all":true,"target":"opponent"},
  {"action":"change_state","state_change":"wait","count":1,
   "card_type":"member_card","target":"opponent","original_value":true,
   "blade_limit":3,"blade_limit_operator":"<="}]}
```
- ✅ **Fixed**: the parser now inserts a `move_cards{source:selected_cards, destination:deck_bottom, target:opponent, all:true}` step after the `select` for "デッキの下に好きな順番で置いてもよい" (previously it only handled "デッキの一番上"). Also the move step now carries the select's `target` (was None, so the engine looked in the wrong player's discard).
- Tests: `bp7_ginko_select_discard_deck_bottom_test.rs` (3 opponent-discard cards → opponent deck bottom).

---

## Summary table

| # | Card / ability | Flags | Verdict | Real defect |
|---|---|---|---|---|
| A1 | PL!S-bp7-007-R＋ 国木田花丸 ab#1 | under_member, placement_order | ✅ OK | none (spurious) |
| A2 | PL!N-bp7-006-R＋ 近江彼方 ab#0 | placement_order | ✅ OK | none (spurious) |
| A3 | PL!N-bp7-007-R＋ 優木せつ菜 ab#0 | under_member | ✅ OK | none (spurious) |
| A4 | PL!N-bp7-007-R＋ 優木せつ菜 ab#2 | under_member, energy_deck | ✅ OK | none (reference shape) |
| A5 | PL!N-bp7-011-R＋ ミア・テイラー ab#1 | under_member | ✅ OK | none (acceptable) |
| A6 | PL!SP-bp7-003-R＋ 嵐 千砂都 ab#1 | under_member | ✅ OK | none (spurious) |
| A7 | PL!SP-bp7-003-R＋ 嵐 千砂都 ab#2 | under_member | ✅ OK | none (spurious) |
| A8 | PL!SP-bp7-006-R＋ 桜小路きな子 ab#0 | energy_deck | ✅ OK | none (spurious) |
| A9 | PL!SP-bp7-006-R＋ 桜小路きな子 ab#1 | energy_deck | ✅ OK | none (spurious) |
| A10 | PL!SP-bp7-007-R＋ 米女メイ ab#0 | energy_deck | ✅ OK | none (spurious) |
| A11 | PL!SP-bp7-007-R＋ 米女メイ ab#1 | energy_deck | ✅ OK | none (spurious) |
| A12 | PL!SP-bp1-026-L 未来予報ハレルヤ！ ab#0 | distinct_name | ✅ OK | none (spurious) |
| A13 | PL!N-bp7-026-L Just Believe!!! ab#1 | card_property | ✅ OK | none (spurious) |
| A14 | PL!SP-bp7-004-R 平安名すみれ ab#0 | placement_order, card_property | ✅ OK | none (spurious) |
| A15 | PL!SP-bp7-026-L Dears ab#0 | energy_deck | ✅ OK | none (spurious) |
| A16 | PL!SP-bp7-027-L What a Wonderful Dream!! ab#0 | energy_deck | ✅ OK | none (spurious) |
| B1 | PL!N-bp7-003-R＋ 桜坂しずく ab#1 | under_member, distinct_name | ✅ | DONE — gain_resource now has `location:under_member` + distinct-name dedup |
| B2 | PL!SP-bp7-003-R＋ 嵐 千砂都 ab#0 | under_member | ✅ | DONE — gain_resource now has `location:under_member` |
| B3 | PL!SP-bp7-001-R 澁谷かのん ab#0 | under_member | ⚠️ | condition missing `location:under_member` |
| B4 | PL!SP-bp7-005-R＋ 葉月 恋 ab#0 | energy_deck | ✅ | DONE — OR (appearance | energy_zone→energy_deck) + no card_type on self-appearance; 11 tests |
| B5 | PL!N-bp7-006-R＋ 近江彼方 ab#1 | card_property | ⚠️ | condition `location:"stage"` wrong (should be preceding_moved); live-card OR lost |
| B6 | PL!N-bp7-028-L Cooking with Love ab#0 | under_member, card_property | ⚠️ | condition `location:"stage"` wrong (should be discard); live-card AND lost |
| C1 | PL!S-bp7-009-R 黒澤ルビィ ab#0 | lose_resource | ✅ | FIXED — parser emits `gain_resource` sign:negative, engine handles position:front targeting |
| C2 | PL!N-bp7-005-R 宮下 愛 ab#0 | under_member, energy_deck, distinct_name | ❌ | choice option source/destination wrong (energy_deck→under_member) |
| C3 | PL!SP-bp7-001-R 澁谷かのん ab#1 | under_member | ❌ | baton-touch re-place → `action:"custom"` |
| C4 | PL!N-bp7-003-R＋ 桜坂しずく ab#0 | under_member, heart_copy | ✅ | DONE — sequential move→under + heart_copy (ref_value="placed_under") |
| C5 | PL!N-bp7-004-R 朝香果林 ab#0 | under_member | ❌ | effect action `place_energy_under_member` (should be change_state wait); blade-limit dropped |
| C6 | PL!S-bp7-004-R 黒澤ダイヤ ab#0 | under_member, baton_touch, both_targets | ❌ | shuffle `target:"energy_deck"` wrong; "選んだカード以外" lost |
| C7 | PL!S-bp7-004-R 黒澤ダイヤ ab#1 | under_member, placement_order | ❌ | look `source` missing; rest→deck_bottom missing |
| C8 | PL!S-bp7-008-R 小原鞠莉 ab#0 | under_member, placement_order | ❌ | rest→deck_bottom parsed as discard_remaining |
| C9 | PL!HS-PR-035-PR 百生吟子 ab#0 | placement_order | ❌ | step1 `select` (no destination) instead of move_cards→deck_bottom |

## Final tally

- **16 / 31 are false positives** — the parser already produced correct structure; the regex analyzer was matching Japanese surface strings it couldn't decompose.
- **6 / 31 are field-level gaps** (Group B): all are "add `location:under_member`" or "fix condition `location` + restore an OR/AND branch". Pure parser work.
- **9 / 31 are structure bugs** (Group C). Of these:
  - 7 are **parser-only** fixes (C1, C2, C4, C5, C6, C7, C8, C9 — the last three need `look_and_select`/selection-schema support for dual destinations).
  - 1 is a **parser+engine overlap** (C3 澁谷かのん ab#1 — baton-touch re-place under arriving member; engine lacks the resolver path).

## Recommended order

1. Group B first (smallest, highest value): add `location:"under_member"` in the three gain_resource/condition rules (B1–B3), fix the compound-trigger OR (B4), and fix condition source/location + OR/AND branches (B5–B6).
2. Group C parser-only: C1 (negative gain_resource / lose_resource), C2 (choice-subtree energy_deck→under_member), C5 (change_state + blade-limit), C6 (both-target + exclude-selected), C7–C9 (look_and_select dual destination / select→move). **C4 DONE (2026-08-05, parser + engine heart_copy).**
3. C3 last: prototype the parser emission, then confirm engine resolver wiring with `run_qa_tests.rs`.

---

# Part 1b — Engine runtime fixes (each_time watcher use-limit)

Date: 2026-08-06.
These are **not** parser gaps — the parser/JSON was faithful. They are runtime
infinite-loop / use-tracking bugs in the ability engine, surfaced by the
`ren_test::ren_ab1_two_copies_both_trigger` test (`PL!SP-bp5-005-R＋` 葉月 恋 ab#1:
"自分のカードが1枚以上いずれかの領域から控え室に置かれるたび、E支払ってもよい。
そうした場合、それらのカードの中から1枚手札に加える", auto, once per turn).

## RT1. each_time discard watcher re-queued forever (runaway loop) — FIXED

Root cause chain (in order of how deep the bug hid):
1. **Trigger condition counts stale batch cards.** The old-format
   `preceding_moved` condition (`card_count_condition`, `location:"discard"`) has
   no `destination`/`locations` array, so it counts **all** cards in the movement
   batch regardless of where they moved. After ab#1 recovers a card *to hand*
   (discard→hand), the re-scan's batch still contains the original milled cards
   (which remain in discard), so the "card was placed into discard" condition
   still passes → ab#1 re-triggers on its own effect's movement.
2. **use_limit was enforced only on completion, not at enqueue.** `trigger_auto_
   abilities_for_player_with_event` enqueued the ability every time the condition
   passed; `use_limit=1` was only checked later (in `resolve()`), so a used-but-
   not-yet-completed ability got re-queued, re-executed, re-moved a card, and
   re-triggered itself — flooding the queue (~+100 entries per pass) until a
   `u8` counter overflowed (`movement_event_counter`, then `turn_limited_
   abilities_used`).
3. **The optional-cost handler recorded the wrong ability's use.** In
   `choice.rs` `handle_conditional_optional`, when the player chose to pay, it
   recorded the use against the **first** ability on the card with a `use_limit`
   (for Ren, ab#0 the mill) instead of the **currently resolving** ability
   (ab#1). So ab#1 was never marked as used at pay-time — too late to matter.

Fixes (engine only, all tests green: **2027 passed**):
- `trigger_auto_abilities_for_player_with_event`: skip enqueueing an auto ability
  whose `use_limit` is already fully consumed this turn. Declined (unused)
  abilities are not recorded, so they still re-trigger (Q233 preserved).
- `choice.rs` `handle_conditional_optional`: record use against
  `entry.ability_index` (the actual ability being resolved), not the first
  use-limited ability on the card.
- Added safety **timeouts** so any residual runaway loop aborts instead of
  hanging/overflowing: `process_current_ability` and `handle_conditional_optional`
  each abort the queue past 200k invocations (`AbilityQueue::clear()` added).
- The `u8` counters (`movement_event_counter`, `turn_limited_abilities_used`) were
  left as-is; the loop is now prevented at the source so they can't overflow.

Note: the initial root-cause candidate — making the `preceding_moved` condition
validate the destination zone (only count cards still in the target zone) — was
**attempted and reverted**. It breaks the documented contract in
`on_hand_to_discard_test.rs` ("the preceding_moved path does NOT validate the
destination zone — it counts cards from recently_moved_cards by type/property")
and regressed 3 Rurino tests. The correct fix is the enqueue-time use_limit gate.

### RT1 hardening — single source of truth for use_limit (2026-08-06)

The follow-up refactor removed the scattered, inconsistent use-limit handling
that made RT1 hard to find and easy to regress:

- **One recorder.** `GameState::record_ability_use(key)` is now the *only* method
  that mutates `turn_limited_abilities_used`. The ~8 ad-hoc call sites that each
  hand-rolled `turn_limited_abilities_used.entry(key).or_insert(0) += 1` (resolver
  `r831/r937/r981/r1014/r1037`, choice `c2880/c3011`, actions `a935`) now all
  delegate to it. It guarantees once-per-activation (via the current queue entry's
  `use_limit_recorded`) and saturates the `u8` count so a runaway caller can never
  overflow it again.
- **One gate.** The trigger-time gate (`ability_has_remaining_uses`, used by the
  enqueue scan) and the resolution-time gate (used in `resolver.rs`) both read the
  same accessor, so they can never disagree about whether an ability still has
  uses left. This is the invariant that prevents the each_time re-queue loop.
- **Key derived from the current entry.** The optional-cost handler records the use
  against `entry.ability_index` — the ability actually being resolved — instead of
  scanning the card for the *first* ability with a use_limit. That removes the
  "recorded the wrong ability (ab#0 instead of ab#1)" bug class entirely.
- **No debug spam.** The temporary `[REN_UL]`/`[REN_GATE]` `eprintln!`
  instrumentation is removed (use-limit events are `log::debug!` behind the
  existing `ABILITY_DEBUG` flag). The two safety timeouts
  (`process_current_ability`, `handle_conditional_optional` → `AbilityQueue::clear()`)
  remain as defense-in-depth.


---

# Part 2 — Clean-abilities audit (the 63 "clean" abilities)

Date: 2026-08-05 (continued session).
Scope: the **63** abilities the regex analyzer marked `new_abilities_clean`
(`gaps: []`). The previous audit (Part 1) only covered the 31 flagged abilities.
This audit re-reads **every one of the 63** against its actual JSON in
`cards/abilities.json` — "clean" only meant *no regex gap keyword matched*, not
*parsed correctly*.

Result: **25 of the 63 have a genuine defect.** The 38 remaining are genuinely fine.

Same legend as Part 1:
- ✅ **OK** — JSON faithfully represents the Japanese.
- ⚠️ **Field gap** — structure right, a field missing/wrong.
- ❌ **Structure bug** — wrong action type / dropped effect.

## New defect groups discovered in the "clean" set

### CLEAN-G1. "デッキの下から / デッキの一番下" → `source: "hand"` (recurring bug, 6 abilities) — ✅ FIXED

The phrase "自分のデッキの下から…控え室に置く" (and "デッキの一番下のカード") is
parsed as a move **from the hand** instead of from `deck_bottom`. This is a real
bug affecting the whole BP07 N-block. Text clearly says the card comes from the
bottom of the deck.

Affected (all confirmed in JSON):
- D7  `PL!S-bp7-006-R` 津島善子 ab#0 — was `"source": "hand"` → now `deck_bottom` ✅
- D8  `PL!S-bp7-008-R` 小原鞠莉 ab#1 — was `"source": "hand"` → now `deck_bottom` ✅
- D9  `PL!S-bp7-020-L` HAPPY PARTY TRAIN ab#1 — was `"source": "hand"` → now `deck_bottom` ✅
- D11 `PL!S-bp7-011-N` 桜内梨子 ab#0 — was `"source": "hand"` → now `deck_bottom` ✅
- D13 `PL!S-bp7-015-N` 津島善子 ab#0 — was `"source": "hand"` → now `deck_bottom` ✅
- D14 `PL!S-bp7-017-N` 小原鞠莉 ab#0 — was `"source": "hand"` → now `deck_bottom` ✅

Fixed (parser + engine + tests):
- **Parser**: `SOURCE_PATTERNS` in parser_utils.py gained `デッキの一番下のカードを` and
  `デッキの下から` → `deck_bottom` (placed before the generic `デッキの上から`).
- **Engine**: `resolve_cards_from_source` in move_cards.rs had a `Deck|DeckTop`
  branch but NO `DeckBottom` arm — the source silently resolved to nothing, so
  zero cards were ever moved. Added `Zone::DeckBottom` arm that pops from the
  deck end via a new `MainDeck::draw_bottom()` (zones.rs). Optional deck-bottom
  moves (e.g. 鞠莉 "…置いてもよい") also needed the yes/no prompt: reused the
  `pay_optional_cost` choice routing, and `handle_optional_cost_payment`'s
  `is_deck_top` gate now also accepts `deck_bottom` so paying the optional move
  re-runs it with the card actually drawn.
- **Tests**: `engine/tests/test_modules/bp7_deck_bottom_source_test.rs` (4 tests)
  covering 津島善子 PL!S-bp7-006-R (live start, bottom 3 → discard + Aqours
  heart04 follow-up), 津島善子 PL!S-bp7-015-N (live start, bottom 1), and
  小原鞠莉 PL!S-bp7-008-R (optional "一番下" discard). All verify the BOTTOM
  cards (not top / not hand) leave the deck.
- All 10 deck_bottom source nodes in the DB verified legitimate (7 move→discard,
  2 look_at, 1 yell-source). No unintended re-parses.

### CLEAN-G2. `destination: null` on "…の下に置く" (member-under placement) — ✅ FIXED

- D1 `PL!S-bp7-005-R＋` 渡辺 曜 ab#0 — "自分の控え室にあるメンバーカード1枚を、自分のステージにいるメンバー1人の**下に置く**" →
  now `move_cards{source:"discard", destination:"under_member", target:"self", count:1}` (was `destination:null`).

### CLEAN-G3. gain_resource missing the "メンバーカードが下に置かれている" condition — ✅ FIXED (parser + engine)

- D2 `PL!S-bp7-005-R＋` 渡辺 曜 ab#1 — "自分のステージにいる、**メンバーカードが下に置かれている**『Aqours』のメンバーは、ブレードを得る" →
  now `gain_resource{blade, count:1, all:true, group_names:[Aqours], requires_under_card:true}`.
- **Parser**: `_mark_under_card_gain` (from `_normalize_effect_tree`) sets `all:true` + `requires_under_card:true` on a gain whose text says "メンバーカードが下に置かれている…メンバーは…を得る".
- **Engine**: added `requires_under_card` effect field (decoder + `EffectFilter` + getter). `recalculate_constant_blade_modifiers` now resolves `all:true` gains to every matching group member on the ability card's side, and — when `requires_under_card` — only those with a MEMBER card underneath (energy doesn't count).
- **Tests**: `bp7_watanabe_under_card_blade_test.rs` (6 cases: member-under → blade, no-under → none, energy-under → none, non-Aqours → none, self → none, two hosts both gain).

### CLEAN-G4. Protection / "ウェイトしない" → `custom`

- D3 `PL!S-bp7-003-R＋` 松浦果南 ab#1 — choice option 1 "相手の効果によっては**ウェイトしない**" →
  `action:"custom"` (line 6305) with `blade_limit:3 <=` captured but no protection primitive.
- Same class as C1/C3 — the parser has no continuous-modifier emission for wait-immunity.

### CLEAN-G5. Character-name condition reduced to "any card in hand" (names dropped) — ✅ FIXED

- D4 `PL!S-bp7-007-R＋` 国木田花丸 ab#0 — "これによって「**津島善子**」か「**黒澤ルビィ**」を手札に加えた場合" →
  `condition{comparison_condition, location:"hand", count:1, >=}` (line 6471) — the two
  names are gone; the condition would pass even if the added card were anyone. ✅ now emits `characters:["津島善子","黒澤ルビィ"]`.
- D22 `PL!S-bp7-001-R` 高海千歌 ab#0 — "これにより「**桜内梨子**」か「**渡辺曜**」を手札に加えた場合" →
  same reduction (line 20999). ✅ now emits `characters:["桜内梨子","渡辺曜"]`.
- D8b `PL!S-bp7-008-R` 小原鞠莉 ab#1 — follow-up "それが「**松浦果南**」か「**黒澤ダイヤ**」の場合" →
  `type:"custom"` (line 21284) — names also lost. ✅ now emits
  `condition{location_condition, source:"preceding_moved", characters:["松浦果南","黒澤ダイヤ"]}`.

Fixed:
- **Parser**: `_extract_generic_fields` now extracts 「A」か「B」 character names in any
  conditional/result phrase (previously only the `のうち` pattern). `_infer_condition_type`
  resolves a characters-only condition (「それが「X」か「Y」の場合」) to a
  `location_condition` with `source:"preceding_moved"`, count≥1, target self.
- **Parser follow-up source**: for "それを手札に加える" conditional follow-ups, the
  move_cards action now emits `source:"preceding_moved"` instead of `source:"discard"`
  so it targets the SPECIFIC card just placed — not any matching discard card.
- **Engine**: `resolve_cards_from_source` gained a `preceding_moved` source handler that
  pulls from `self.moved_cards` (the current sequential's own moves) filtered by the
  action's `characters`.
- **Tests**: `engine/tests/test_modules/bp7_character_name_condition_test.rs` (5 tests):
  bottom 果南 → hand; bottom ダイヤ → hand; bottom すみれ → stays in waitroom;
  skip optional → nothing moves; and the key edge case — discard already holds a 果南
  but the placed card is すみれ → the pre-existing 果南 must NOT be grabbed.

### CLEAN-G6. Unresolvable `dynamic_count` references

- D5 `PL!N-bp7-007-R＋` 優木せつ菜 ab#1 — "自分のエネルギーが6枚より多いかぎり、**その差に等しい数**のheart02を得る" →
  `dynamic_count{reference:"その差", mode:"equals"}` (line 6764). "その差" is prose, not a
  computable reference. The engine cannot resolve it. Should be `energy_count - 6`.
- D23 `PL!N-bp7-026-L` Just Believe!!! ab#0 — select count uses
  `dynamic_count{reference:"自分のステージにいる…これにより控え室に置いたカードの枚数", mode:"equals"}`
  (line 21872) — the whole clause became the reference; should be `previous_cost_moved`.

Fix: bind the cost's moved-card count (and energy-difference) to real references.

### CLEAN-G7. Optional-cost structure dropped (ミア・テイラー class)

- D6 `PL!N-bp7-011-R＋` ミア・テイラー ab#0 — "このカードがデッキから控え室に置かれたとき、**手札を1枚控え室に置いてもよい。そうしたとき、**控え室からこのカードを手札に加える。" →
  parsed as `condition{zone_change→discard} + move_cards{discard→hand, self}` (line 6817).
  The **optional discard-cost + "そうしたとき"** structure is dropped entirely; the JSON
  describes an unconditional "when discarded, add this card to hand", which is not the text.

### CLEAN-G8. "エールをデッキの下から行う" → both branches `custom`

- D10 `PL!S-bp7-022-L` 恋になりたいAQUARIUM ab#0 — "自分のエールは、デッキの上から行う**代わりにデッキの下から行う**。" →
  `conditional_alternative{primary_effect{custom}, alternative_effect{custom}}` (lines 21417-21431).
  Neither branch parses to a yell-source modification.

### CLEAN-G9. Formation-change action dropped

- D12 `PL!S-bp7-012-N` 松浦果南 ab#0 — "…**フォーメーションチェンジしてもよい**。この効果によって『SaintSnow』のメンバーが**移動した場合**、…ブレード×2を得る。" →
  parsed `sequential` with **only** the conditional gain (line 36056). The
  `position_change`/formation-change move is missing entirely; the "移動した場合"
  condition has no preceding action to observe.

### CLEAN-G10. "元々持つハートがすべてheart04になる" → `custom` — ✅ FIXED

- D15 `PL!S-bp7-024-L` ときめき分類学 ab#0 — "ライブ終了時まで、自分のステージにいる『Aqours』のメンバー1人は、**元々持つハートがすべてheart04になる**。" →
  now `action:"set_heart_type"`, `heart_type:"heart04"`, `original_value:true`, `group_names:["Aqours"]`, `count:1`, `target_count:1`, `duration:"live_end"` (was `custom`).


### CLEAN-G11. "より多くのブレードを持つ" max-comparison missing

- D16 `PL!N-bp7-027-L` オードリー ab#0 — "そのメンバーが、自分と相手のステージにいる**ほかのすべてのメンバーより多くのブレードを持つ**場合、このカードのスコアを＋１する。" →
  condition `location_condition{stage, exclude_self, scope:both, all}` (line 36953) with **no
  blade comparison** — the "has more blade than all others" predicate is not represented.

### CLEAN-G12. "ライブカード置き場から手札に戻す" → `custom` — ✅ FIXED

- D17 `PL!N-bp7-030-L` Cheer Mode ab#1 — "このカードを**ライブカード置き場から手札に戻す**。" →
  now `move_cards{source:"live_card_zone", destination:"hand", self_target:true, card_type:"live_card", count:1}` (was `custom`).


### CLEAN-G13. Optional add-to-hand action missing (Like a Treasure) — ✅ FIXED (parser + engine, 9 gameplay tests)

- D18 `PL!N-bp7-031-L` Like a Treasure ab#1 — "それらのカードの中から『虹ヶ咲』の**ライブカードを1枚手札に加えてもよい**。そうしたとき、このカードのスコアを＋１する。" →
  now `conditional_on_optional{optional_action: move_cards{source:"those_cards", destination:"hand", card_type:"live_card", group_names:["虹ヶ咲"], count:1}, conditional_action: sequential[move_to_hand, modify_score{+1}]}` (was only a `modify_score` — the move to hand was absent).
- **Parser**: new `_try_those_cards_add_hand_optional` handler (Tier 2) emits the conditional_on_optional for "それらのカードの中から…手札に加えてもよい。そうしたとき、…".
- **Engine refactor** (2026-08-06): `source:"those_cards"` was silently resolving from the **whole discard pile** instead of the cards that actually moved. Now it:
  - resolves only the explicit `trigger_moved_cards` captured on the queue entry at enqueue time;
  - when the trigger moved cards but **none matched** the filters, moves **nothing** (returns an empty move) — it no longer grabs a pre-existing qualifying card that wasn't part of the trigger batch (fixed the "those cards" semantic);
  - still falls through to the historic discard-pile "pick card" resolution when **no** `trigger_moved_cards` was recorded (Q252 / Riko legacy path preserved).
- **Engine**: `condition/card.rs` — a `Location` condition with `destination` set but `source` None is now a **movement** condition (`is_new_movement`), and an empty source matches any source. This is what made G13's each_time ("a card is placed into your discard by a live-success ability") actually fire — it was parsed with `destination:discard` but `source:None`, so it never matched any deck→discard movement.
- **Engine**: added `AbilityResolver.last_move_moved_any`; a `modify_score` step that directly follows a `those_cards`→hand move only applies when the move **actually added a card** ("そうしたとき" = +1 only when you add).
- **Tests**: `bp7_like_a_treasure_optional_test.rs` — now **9 real-gameplay each_time edge cases** (was 3 JSON-shape tests): accept → hand + exactly +1 score; skip → nothing + 0; no 虹ヶ咲 live card → nothing + 0; multiple 虹ヶ咲 live → exactly ONE + +1; duplicate copies → one; turn-limit (ターン1回) → second mill offers nothing; **pre-existing 虹ヶ咲 live card already in waitroom (not part of the mill) → NOT added**; a 虹ヶ咲 MEMBER (non-live) card → not added (card_type filter); no movement at all → does not fire. The `assert_ability!` verdict helper is used on failure.


### CLEAN-G14. Blade-limit filter misparsed as a resource gain (Fire Bird) — ✅ FIXED

- D19 `PL!N-sd2-026-P` Fire Bird ab#0 — "自分のステージにいる**ブレードを4つ以上持つ**『虹ヶ咲』のメンバー1人は、ライブ終了時まで、heart02×2を得る。" →
  now `gain_resource{heart, heart_colors:[heart02], count:2, card_type:member_card, group_names:["虹ヶ咲"], target_count:1, duration:"live_end", blade_limit:4, blade_limit_operator:">="}` (was a misparse into `gain_resource{blade, count:4}` + `gain_resource{heart, count:4}`).


### CLEAN-G15. Triple-name cost condition → only one name survives — ✅ FIXED

- D20 `LL-bp7-001-R＋` 国木田花丸&優木せつ菜&嵐 千砂都 ab#0 — "自分の手札から「国木田花丸」と「優木せつ菜」と「嵐千砂都」のメンバーカードを**それぞれ1枚ずつ**控え室に置いてもよい。そうしたとき、このカードのコストは10になる。" →
  was `condition{location_condition, characters:[嵐千砂都], locations:[discard,hand], count:1, operator:"="}` + `modify_cost{characters:[嵐千砂都]}`.
  Only「嵐千砂都」survived; 国木田花丸 and 優木せつ菜 dropped, the
  "それぞれ1枚ずつ" (1 of each) was lost, and `modify_cost` had no value:10 / operation:set.

Fixed (parser + engine):
- **Parser**: new `_try_character_each` handler (registered Tier 1) turns
  "「A」と「B」と「C」…をそれぞれ1枚ずつ控え室に置いた" into a compound AND of one
  per-character `location_condition{characters:[name], location:"discard", count:1, operator:">="}`.
  `_handle_cost_modification` now parses "コストはNになる" as `operation:"set", value:N`
  with `source/location:"hand", card_type:"member_card"`. Also stopped `_enrich_characters`
  from leaking the condition's character names onto a self cost-set effect, and prevented
  `parse_ability` from double-compounding the re-derived trigger condition.
- **Engine**: `count_cards_with_filters` (used by card-count conditions) now applies
  `condition.characters` to the filter (was silently dropped). `ModifierEntry` supports a
  `set` cost override (コストはNになる) distinct from additive deltas:
  `GameModifiers.constant_cost_set_bonuses`, `set_cost_modifier`, `get_cost_modifier_set`,
  and the play-cost path (`move_card_from_hand_to_stage`) applies the set override when
  present. Stale set values are cleared on recalc. `set+additive` stacking preserved
  (verified by q127/special_color stacking tests).
- **Tests**: `engine/tests/test_modules/ll_bp7_001_triple_member_test.rs` (12 tests):
  6 constant-cost conditions (0/1/2/3 named + duplicate character + named-in-hand),
  2 GAMEPLAY play-cost checks (costs 10 with all three in discard, costs 15 without),
  debut adds live card, debut with no live card, live-success adds member, live-success
  ignores live cards. Play-cost verified via actual energy spent.

### CLEAN-G16. Shuffle-to-deck-bottom action folded into condition — ✅ FIXED (parser)

- D21 `PL!SP-bp7-028-L` 未来の音が聴こえる ab#0 — "自分の控え室にある『Liella!』のメンバーカードを**9枚選び、シャッフルし、デッキの一番下に置いてもよい。そうしたとき、**…すべてのメンバーはブレードを得る。" →
  now `conditional_on_optional{optional_action: move_cards{source:"discard", destination:"deck_bottom", count:9, card_type:"member_card", group_names:["Liella!"], shuffle:true, placement_order:"any_order"}, conditional_action: sequential[move_to_bottom, gain_resource{blade, all}]}` (was a `group_condition{shuffle:true,count:9}` + `gain_resource` — the move never actually happened).
- **Parser**: new `_try_discard_shuffle_to_bottom_optional` handler (Tier 2) emits the conditional_on_optional.
- **Tests**: `bp7_mirai_no_oto_optional_test.rs` — **5 real-gameplay** edge cases (was 3 JSON-shape tests): accept → 9 Liella! discard→deck_bottom in any order + blade gain; skip → nothing; too few Liella! cards → nothing; mixed Liella!/non-Liella! discard → only Liella!; turn-limit / no-optional. Driven through the real TAS scan, not JSON inspection.


### CLEAN-G17. Trigger "エネルギーがメンバーの下に置かれたとき" under-parsed

- D24 `PL!SP-bp7-016-N` 葉月 恋 ab#0 and D25 `PL!SP-bp7-005-R＋` 葉月 恋 ab#1 — "自分のカードの効果によって、自分の**エネルギー置き場にエネルギーが置かれたとき**" →
  `condition{comparison_condition, location:energy_zone, resource_type:energy, movement:moved}` (lines 37364, 7075).
  The trigger is expressed as a fuzzy comparison, not a `trigger_event{zone_change,
  destination:energy_zone}`. Borderline but a real trigger-modeling gap (compare 桜小路きな子 ab#1
  which did produce a `zone_change` trigger_event).

### CLEAN-G18. Color-diversity condition collapsed to a plain card count — ✅ FIXED (parser + engine)

- D27 `PL!N-bp7-020-N` エマ・ヴェルデ ab#0 — "それらのメンバーカードの中に**2種類以上のブレードハートの色**がある場合" →
  now `card_count_condition{card_type:member_card, count:2, operator:">=", unit:"types", source:"preceding_moved"}` (was `count:1 >=` of any member card — color diversity lost).
- **Parser**: added the `(\d+)種類以上の.+?色がある場合` pattern (→ `unit:"types"`) and `source:"preceding_moved"` for "それらの…色がある場合" (scope to the milled cards).
- **Engine**: `resolve_moved_cards_source` now honors `unit:"types"` — it counts DISTINCT blade-heart colors among the moved member cards instead of card count.
- **Tests**: `bp7_emma_color_diversity_test.rs` (5 gameplay edge cases): 2 distinct colors → heart04, 3 distinct → heart04, all same color → no, only 1 member milled → no, members with no blade heart → no.


### CLEAN-G19. Select excludes self for a "this member + 1 other" target — ✅ FIXED (parser)

- D26 `PL!S-bp7-005-R＋` 渡辺 曜 ab#2 — "このメンバー**と**自分のステージにいるほかの『Aqours』のメンバー1人を選ぶ。それらが持つ登場能力それぞれ1つを発動させる。" →
  the select was `{source:"stage", count:1, exclude_self:true, group:Aqours}` (only the other member) and the `activate_ability` also `exclude_self:true`, so this member's 登場 ability never fired.
- **Fixed**: `_fix_select_self_and_other` (from `_normalize_effect_tree`) — when the effect text has "このメンバーと…を選ぶ", the `select` becomes `count:2` with `exclude_self` removed, and the follow-up `activate_ability` also drops `exclude_self` (so THIS member's 登場 ability is targeted too).
- **Tests**: `bp7_watanabe_select_self_and_other_test.rs` (the select is over 2 candidates including this member).

## Full per-card verdict table (all 63)

| # | Card / ability | Verdict | Defect |
|---|---|---|---|
| 1 | PL!S-bp7-003-R＋ 松浦果南 ab#0 | ✅ | — (look top → optional deck_bottom) |
| 2 | PL!S-bp7-003-R＋ 松浦果南 ab#1 | ❌ D3 | option1 ウェイトしない → `custom` |
| 3 | PL!S-bp7-005-R＋ 渡辺 曜 ab#0 | ❌ D1 | `destination:null` (should be under_member) |
| 4 | PL!S-bp7-005-R＋ 渡辺 曜 ab#1 | ❌ D2 | under-card condition dropped |
| 5 | PL!S-bp7-005-R＋ 渡辺 曜 ab#2 | ❌ D26 | select only the "other" member; this member's 登場 ability dropped |
| 6 | PL!S-bp7-007-R＋ 国木田花丸 ab#0 | ✅ | D4 FIXED — 津島善子/黒澤ルビィ characters on condition |
| 7 | PL!N-bp7-007-R＋ 優木せつ菜 ab#1 | ❌ D5 | dynamic_count "その差" unresolvable |
| 8 | PL!N-bp7-011-R＋ ミア・テイラー ab#0 | ❌ D6 | optional discard-cost + そうしたとき dropped |
| 9 | PL!N-bp7-011-R＋ ミア・テイラー ab#2 | ✅ | — (discard→deck_top optional) |
| 10 | PL!SP-bp7-005-R＋ 葉月 恋 ab#1 | ⚠️ D25 | trigger as comparison, not zone_change event |
| 11 | PL!SP-bp7-007-R＋ 米女メイ ab#2 | ✅ | — (energy>opp → active 6) |
| 12 | PL!-PR-020-PR 高坂穂乃果 ab#0 | ✅ | — (gain_ability + condition) |
| 13 | PL!-PR-021-PR 矢澤にこ ab#0 | ✅ | — (energy==7 → blade2) |
| 14 | PL!S-bp7-001-R 高海千歌 ab#0 | ✅ | D22 FIXED — 桜内梨子/渡辺曜 characters on result_condition |
| 15 | PL!S-bp7-006-R 津島善子 ab#0 | ✅ | D7 FIXED — デッキの下 → `deck_bottom` |
| 16 | PL!S-bp7-008-R 小原鞠莉 ab#1 | ✅ | D8+D8b FIXED — deck-bottom source + preceding_moved character condition |
| 17 | PL!S-bp7-020-L HAPPY PARTY TRAIN ab#0 | ✅ | — (all active → reduce hearts) |
| 18 | PL!S-bp7-020-L HAPPY PARTY TRAIN ab#1 | ✅ | D9 FIXED — デッキの下 → `deck_bottom` |
| 19 | PL!S-bp7-022-L 恋になりたいAQUARIUM ab#0 | ❌ D10 | yell-from-bottom → both branches `custom` |
| 20 | PL!S-bp7-022-L 恋になりたいAQUARIUM ab#1 | ✅ | — (revealed heart02/04/05 → +1) |
| 21 | PL!N-bp7-025-L Colorful Dreams! ab#0 | ✅ | — (1 虹ヶ咲 member → blade) |
| 22 | PL!N-bp7-025-L Colorful Dreams! ab#1 | ✅ | — (3+ heart types revealed → +1) |
| 23 | PL!N-bp7-026-L Just Believe!!! ab#0 | ⚠️ D23 | select dynamic_count reference unresolvable |
| 24 | PL!SP-bp7-008-R 若菜四季 ab#0 | ✅ | — (wait self → draw 1) |
| 25 | PL!SP-bp7-008-R 若菜四季 ab#1 | ✅ | — (area move while wait → active) |
| 26 | PL!SP-bp7-009-R 鬼塚夏美 ab#0 | ✅ | — (left/right side → heart02) |
| 27 | PL!SP-bp7-009-R 鬼塚夏美 ab#1 | ✅ | — (center, blade≤2 opp → wait) |
| 28 | PL!N-sd2-026-P Fire Bird ab#0 | ❌ D19 | blade4 filter → blade+4; heart 2 → 4 |
| 29 | PL!SP-PR-024-PR 平安名すみれ ab#0 | ✅ | — (yell + score-icon card → heart06) |
| 30 | PL!S-bp7-011-N 桜内梨子 ab#0 | ✅ | D11 FIXED — デッキの下 → `deck_bottom` |
| 31 | PL!S-bp7-012-N 松浦果南 ab#0 | ❌ D12 | formation-change action missing |
| 32 | PL!S-bp7-014-N 渡辺 曜 ab#0 | ✅ | — (opp energy>self → heart02) |
| 33 | PL!S-bp7-015-N 津島善子 ab#0 | ✅ | D13 FIXED — デッキの下 → `deck_bottom` |
| 34 | PL!S-bp7-016-N 国木田花丸 ab#0 | ✅ | — (3+ members → heart02/04/05) |
| 35 | PL!S-bp7-017-N 小原鞠莉 ab#0 | ✅ | D14 FIXED — デッキの一番下 → `deck_bottom` |
| 36 | PL!S-bp7-024-L ときめき分類学 ab#0 | ❌ D15 | 元々のハートがheart04になる → `custom` |
| 37 | PL!S-bp7-025-L Guilty Night, Guilty Kiss! ab#0 | ✅ | — (choice: wait≤2 / draw1) |
| 38 | PL!N-bp7-020-N エマ・ヴェルデ ab#0 | ❌ D27 | "2種類以上のブレードハートの色" → only `count:1 >=` of member cards, color-diversity lost |
| 39 | PL!N-bp7-024-N 鐘 嵐珠 ab#0 | ✅ | — (R3BIRTH 3 → heart01) |
| 40 | PL!N-bp7-027-L オードリー ab#0 | ❌ D16 | blade-max comparison missing |
| 41 | PL!N-bp7-030-L Cheer Mode ab#1 | ❌ D17 | live-card-zone→hand → `custom` |
| 42 | PL!N-bp7-031-L Like a Treasure ab#0 | ✅ | — (deck_top 3 → discard) |
| 43 | PL!N-bp7-031-L Like a Treasure ab#1 | ❌ D18 | optional add-to-hand action missing |
| 44 | PL!SP-bp7-013-N 唐 可可 ab#0 | ✅ | — (KALEIDOSCORE 3 → heart06+blade) |
| 45 | PL!SP-bp7-014-N 嵐 千砂都 ab#0 | ✅ | — (area move → blade2) |
| 46 | PL!SP-bp7-015-N 平安名すみれ ab#0 | ✅ | — (E optional + CatChu 3 → draw) |
| 47 | PL!SP-bp7-016-N 葉月 恋 ab#0 | ⚠️ D24 | trigger as comparison, not zone_change event |
| 48 | PL!SP-bp7-020-N 鬼塚夏美 ab#0 | ✅ | — (energy>opp → blade2) |
| 49 | PL!SP-bp7-025-L Memories ab#0 | ✅ | — (嵐千砂都 → blade) |
| 50 | PL!SP-bp7-028-L 未来の音が聴こえる ab#0 | ❌ D21 | 9枚選びシャッフル→下 folded into condition |
| 51 | PL!SP-bp7-028-L 未来の音が聴こえる ab#1 | ✅ | — (all revealed Liella! → +1) |
| 52 | LL-bp7-001-R＋ 国木田花丸&優木せつ菜&嵐千砂都 ab#0 | ✅ | D20 FIXED — compound 1-of-each + cost set to 10 |
| 53 | LL-bp7-001-R＋ 国木田花丸&優木せつ菜&嵐千砂都 ab#1 | ✅ | — (discard live card → hand) |
| 54 | PL!N-sd2-001-SD2 上原歩夢 ab#0 | ✅ | — (E2 → 虹ヶ咲 live card to hand) |
| 55 | PL!N-sd2-006-SD2 近江彼方 ab#0 | ✅ | — (wait 虹ヶ咲 optional → blade2) |
| 56 | PL!N-sd2-010-SD2 三船栞子 ab#0 | ✅ | — (draw 2) |
| 57 | PL!N-sd2-010-SD2 三船栞子 ab#1 | ✅ | — (wait member → optional discard → active+blade2) |
| 58 | PL!N-sd2-013-SD2 上原歩夢 ab#0 | ✅ | — (虹ヶ咲 only → opp blade≤2 wait) |
| 59 | PL!N-sd2-015-SD2 桜坂しずく ab#0 | ✅ | — (wait + discard → draw) |
| 60 | PL!N-sd2-017-SD2 宮下 愛 ab#0 | ✅ | — (E optional → active 1) |
| 61 | PL!N-sd2-019-SD2 優木せつ菜 ab#0 | ✅ | — (heart05) |
| 62 | PL!N-sd2-019-SD2 優木せつ菜 ab#1 | ✅ | — (opp cost≤2 → wait) |
| 63 | PL!N-sd2-021-SD2 天王寺璃奈 ab#0 | ✅ | — (opp cost≤4 → wait) |

## Clean-set tally

- **38 / 63 genuinely fine** (parser produced faithful structure).
- **25 / 63 have a genuine defect**, dominated by:
  - **6×** the `source:"hand"` deck-bottom bug (CLEAN-G1) — one parser fix, high blast radius. **FIXED (2026-08-05).**
  - **3×** character-name conditions reduced to "any card" or `custom` (CLEAN-G5). **FIXED (2026-08-05).**
  - **2×** unresolvable `dynamic_count` (CLEAN-G6).
  - **2×** energy-placed trigger modeled as comparison instead of zone_change (CLEAN-G17).
  - **1× each** for the rest: under-member destination null (G2), missing under-card gate (G3),
    wait-immunity custom (G4), optional-cost structure (G7), yell-source custom (G8),
    formation-change dropped (G9), set-heart custom (G10), blade-max comparison (G11),
    live-card-return custom (G12), missing add-to-hand (G13), Fire Bird misparse (G14),
    triple-name cost (G15), shuffle-to-bottom as condition (G16), select misses self (G19),
    color-diversity collapsed (G18).

## Combined grand total (flagged 31 + clean 63 = 94)

- **16** false positives (all in the flagged set).
- **15 + 25 = 40 genuine defects**:
  - Flagged set: 6 field gaps (B1–B6) + 9 structure bugs (C1–C9) = 15.
  - Clean set: 25 new findings (D1–D27).
- So the regex analyzer's "clean/needs_work" split was **not** meaningful for
  correctness: 25 of the 63 "clean" abilities are actually broken.

## Status as of 2026-08-06

**Done so far (with tests):**
- Flagged set: **B1, B2, B3, B4, B6, C1–C9** (14 / 15).
- Clean set: **CLEAN-G1, G2, G3, G4, G5, G7, G9, G10, G12, G13, G14, G15, G16, G18, G19** (15 / 19 groups).

**Newly confirmed done this session:**
- **B3** (澁谷かのん ab#0) — the `location:under_member` condition was already fixed in `abilities.json`; 11 gameplay tests in `bp7_kanon_under_member_blade_test.rs`.
- **B5** (近江彼方 ab#1) — `or_condition` + `source:preceding_moved` already fixed; 9 gameplay tests in `bp7_kanata_choice_test.rs`.
- **G4 parser** (松浦果南 ab#1) — now emits `restriction{cannot_wait_by_effect}` (was `custom`). **Engine enforcement still missing.**
- **G7** (ミア ab#0) — new parser `_try_discard_hand_recover_self_optional` → `conditional_on_optional`; engine `last_move_moved_any` recorded on every move finalize + sequential gate generalized to self-recover; 5 gameplay tests in `bp7_mia_optional_recover_test.rs`.
- **G9** (果南 ab#0) — `_try_kore_niyori_result` now recognizes `この効果によって` and attaches a leading gate condition to the primary; engine `evaluate_group_condition` `all_members` now matches ANY listed group (`『Aqours』か『SaintSnow』`); 3 gameplay tests in `bp7_kanan_formation_change_test.rs`.
- **G4 (engine)** (松浦果南 ab#1) — `cannot_wait_by_effect` now enforced: `GameState.wait_immune_members` records (member_id, owner_id); `execute_change_state` member_op wait path drops members immune against the effect's controller (opponent wait blocked, self allowed). Verified against **5 diverse wait abilities** (朝香果林 / 矢澤にこ / 高坂穂乃果 / 西木野真姫 / 園田海未) in their existing test files + shared helper `bp7_wait_immunity_helpers`.

**Remaining to fix (4 items):**
| Item | Defect |
|---|---|
| CLEAN-G4 (engine) `PL!S-bp7-003-R＋` 松浦果南 ab#1 | parser emits `restriction{cannot_wait_by_effect}`, but `execute_restriction` does not enforce wait-immunity |
| CLEAN-G6 `PL!N-bp7-007-R＋` 優木せつ菜 ab#1 + `PL!N-bp7-026-L` Just Believe!!! ab#0 | unresolvable `dynamic_count` ("その差") |
| CLEAN-G8 (engine) `PL!S-bp7-022-L` 恋になりたいAQUARIUM ab#0 | parser emits `custom{yell_source_modifier, deck_bottom}` — engine support unverified |
| CLEAN-G11 `PL!N-bp7-027-L` オードリー ab#0 | "more blade than all others" max-comparison missing |
| CLEAN-G17 `PL!SP-bp7-016-N` 葉月 恋 ab#0 + `PL!SP-bp7-005-R＋` 葉月 恋 ab#1 | energy-placed trigger modeled as comparison, not `zone_change` |

## Recommended order (updated)

1. Group B (B1–B6) — smallest field-gap fixes. **B1–B6 DONE.**
2. **CLEAN-G1** (6 abilities) — recurring `source:"hand"`→`deck_bottom`. **DONE (2026-08-05): parser + engine DeckBottom draw branch + optional-pay routing; tests in `bp7_deck_bottom_source_test.rs`.**
3. **C4** (桜坂しずく ab#0) — under-member move + heart-copy. **DONE (2026-08-05): `_try_place_under_heart_copy` + `heart_copy` modifier; tests in `bp7_heart_copy_test.rs`.**
4. CLEAN-G5 + D20 (4 abilities) — character-name conditions (`characters` array). **DONE.**
5. Group C parser-only (C1, C2, C5–C9). **All DONE.**
6. **CLEAN-G13** (Like a Treasure ab#1). **DONE (2026-08-06): `those_cards`/movement-condition/score-gate refactor; 9 gameplay tests in `bp7_like_a_treasure_optional_test.rs`.**
7. **CLEAN-G16/G18** (未来の音 / エマ color-diversity). **DONE — JSON-shape tests replaced with real gameplay edge cases.**
8. **CLEAN-G7** (ミア ab#0). **DONE (2026-08-06): parser `_try_discard_hand_recover_self_optional` + engine `last_move_moved_any` gating; 5 gameplay tests in `bp7_mia_optional_recover_test.rs`.**
9. **CLEAN-G9** (果南 ab#0). **DONE (2026-08-06): parser `この効果によって` gate + engine `all_members` any-group; 3 gameplay tests in `bp7_kanan_formation_change_test.rs`.**
10. Next: **G4 engine enforcement** (wait-immunity), then **G11 blade-max comparison**, **G6 dynamic_count**, **G17 trigger-modeling**, **G8 yell-source engine**. These are engine-heavy.
