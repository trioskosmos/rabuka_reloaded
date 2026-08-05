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

### B4. `PL!SP-bp7-005-R＋` 葉月 恋 ab#0 — gap: [energy_deck] — ⚠️ second trigger dropped

Japanese:
> 自動：このメンバーが登場するか、自分のエネルギーがエネルギー置き場からエネルギーデッキに置かれたとき、自分のエネルギーデッキから、エネルギーカードを1枚ウェイト状態で置く。…

Parsed:
```json
{"condition":{"type":"appearance_condition","appearance":true,
  "trigger_event":{"type":"appearance","location":"stage"},
  "location":"stage","target":"self","card_type":"member_card"},
 "action":"sequential":[move_cards{source:energy_deck,destination:energy_zone,state_change:wait},
                        restriction{cannot_active,delayed}]}
```
- ✅ The effect (energy_deck → energy_zone wait + cannot_active) is correct; the `energy_deck` **source** is handled.
- ❌ **"…か、自分のエネルギーがエネルギー置き場からエネルギーデッキに置かれたとき" is dropped.** The trigger is an OR: (a) this member appears, **or** (b) energy moved from energy_zone to energy_deck. Only (a) was captured. This is a compound-trigger bug, not an energy_deck-source bug.
- Fix: condition needs a compound OR of appearance + zone_change(energy_zone→energy_deck).

### B5. `PL!N-bp7-006-R＋` 近江彼方 ab#1 — gap: [card_property] — ⚠️ wrong condition location, OR branch lost

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

### B6. `PL!N-bp7-028-L` Cooking with Love ab#0 — gap: [under_member, card_property] — ⚠️ wrong condition location, OR branch lost

Japanese:
> ライブ開始時：自分の控え室に『虹ヶ咲』のライブカードと、ブレードハートを持たない『虹ヶ咲』のメンバーカードがある場合、自分の控え室にあるすべてのカードをシャッフルし、デッキの下に置いてもよい。そうしたとき、…すべての『虹ヶ咲』のメンバーは heart01を得る。

Parsed:
```json
{"condition":{"type":"location_condition","card_type":"member_card","location":"stage",
  "negation":true,"card_property":"has_blade_heart","group_names":["虹ヶ咲"],"target":"self"},
 "action":"gain_resource","resource":"heart","heart_colors":["heart01"],
 "all":true,"duration":"live_end","shuffle":true}
```
- ✅ `card_property:has_blade_heart + negation`; `shuffle:true`; `gain_resource{heart01, all, live_end}`.
- ❌ **`location:"stage"` is wrong** — the text says "自分の控え室に…ある場合" (in **discard**), not stage.
- ❌ **"ライブカード**と**…メンバーカード" AND-branch is collapsed to `card_type:member_card` only** — the live-card requirement is lost.
- `under_member` flag: no メンバーの下 anywhere in the text → spurious.
- Fix: condition location → discard; represent the compound (live card present AND member card present without blade heart).

---

## Group C — Structure bugs (parser emits wrong action or drops the effect)

### C1. `PL!S-bp7-009-R` 黒澤ルビィ ab#0 — gap: [lose_resource] — ❌ effect becomes `custom`

Japanese:
> 常時：このメンバーの正面のエリアにいるコスト4以下のメンバーは、ブレードを1つ**失う**。

Parsed:
```json
{"cost_limit":4,"cost_limit_operator":"<=","count":1,"card_type":"member_card",
 "position":"front","action":"custom"}
```
- ✅ Target filter `position:front + cost_limit<=4` parsed.
- ❌ **"ブレードを1つ失う" is not represented** → `action:"custom"`. No negative resource, no `lose_resource`. This is a continuous 常時 modifier (opponent's front, cost≤4 member loses 1 blade while this is out), so it needs a continuous negative blade modifier, not a one-shot.
- Engine: `gain_resource` already supports negative amounts (`effects/misc.rs:1404` `blades_to_add = if is_negative`); blade modifiers exist via `add_blade_modifier_with_trace`. **Engine can apply it once the parser emits the effect.** → Parser-only fix (see full analysis for shape).

### C2. `PL!N-bp7-005-R` 宮下 愛 ab#0 — gap: [under_member, energy_deck, distinct_name] — ❌ choice option broken

Japanese:
> 登場：自分のステージに名前の異なる『DiverDiva』のメンバーが2人いる場合、以下から1つを選ぶ。・エネルギーを2枚アクティブにする。・自分のエネルギーデッキから、エネルギーカード1枚を自分のステージにいる『虹ヶ咲』のメンバーの下に置く。

Parsed:
```json
{"condition":{"type":"location_condition","target":"self","distinct":"card_name",
  "location":"stage","group_names":["DiverDiva"]},
 "action":"choice","options":[
   {"action":"change_state","state_change":"active","count":2,"card_type":"energy_card"},
   {"action":"move_cards","source":"stage","count":1,"card_type":"energy_card",
    "target":"self","group_names":["虹ヶ咲"],"destination":null}]}
```
- ✅ Top-level `distinct:card_name` (名前の異なる DiverDiva) handled; option 1 fine.
- ❌ **Option 2 is broken**: `source:"stage"` is wrong (should be `"energy_deck"`), `destination:null` is wrong (should be `"under_member"`), and it should use `place_energy_under_member` (like A4) or `move_cards` with `destination:"under_member"`. Compare A4 which produced the correct shape.
- Fix: in choice-subtree handling, recognize "エネルギーデッキから…メンバーの下に置く" → `place_energy_under_member{source:energy_deck, destination:under_member}`.

### C3. `PL!SP-bp7-001-R` 澁谷かのん ab#1 — gap: [under_member] — ❌ effect becomes `custom`

Japanese:
> 自動：このメンバーがステージから控え室に置かれたとき、バトンタッチしていた場合、このカードをそのバトンタッチで登場したメンバーの下に置く。

Parsed:
```json
{"condition":{"type":"location_condition","location":"discard",
  "locations":["discard","stage"],"card_type":"member_card","self_target":true,"target":"self"},
 "card_type":"member_card","action":"custom","destination":null,"self_target":true}
```
- ✅ Trigger + condition (stage→discard) parsed.
- ❌ **The action "そのバトンタッチで登場したメンバーの下に置く" is not represented** → `action:"custom", destination:null`. Existing baton-touch patterns only match "バトンタッチして登場"/"からバトンタッチ"; this departing-member perspective ("バトンタッチしていた場合 … その…メンバーの下に置く") matches nothing.
- Engine: baton-touch tracking exists (`baton_touch_arriving_card_ids`, `baton_touch_replaced_member_id`) and `under_member` placement exists, but no resolver path re-places the departed card under the arriving member. → **Overlap** (parser must emit the move; engine needs the wiring). See full analysis.

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

### C5. `PL!N-bp7-004-R` 朝香果林 ab#0 — gap: [under_member] — ❌ effect mis-typed + condition dropped

Japanese:
> 起動：エネルギー置き場にあるエネルギー1枚をこのメンバーの下に置く：相手のステージにいる、元々持つブレードの数がこのメンバーの下にあるエネルギーカードの枚数に1を足した数以下のメンバー1人をウェイトにする。

Parsed:
```json
"cost":{"type":"place_energy_under_member","destination":"under_member","count":1,
  "card_type":"member_card"},
"effect":{"source":"under_member","state_change":"wait","count":1,"card_type":"member_card",
  "target":"opponent","action":"place_energy_under_member","original_value":true,
  "energy_count":1,"target_member":"this_member"}
```
- ✅ Cost (energy under this member) correct — though `card_type:"member_card"` on cost is a type-slip (should be energy_card).
- ❌ **Effect `action` is wrong**: `"place_energy_under_member"` — the actual effect is to **wait (ウェイトにする) one opponent member**. It should be `change_state{state_change:wait}`.
- ❌ **The blade-limit condition is dropped**: "元々持つブレードの数が【下にあるエネルギー枚数+1】以下のメンバー1人" — neither the `blade_limit` nor the dynamic comparison to energy-under count is represented.
- Fix: effect → `change_state{state_change:wait, target:opponent, member_card, blade_limit:dynamic(energy_under+1), <=}`.

### C6. `PL!S-bp7-004-R` 黒澤ダイヤ ab#0 — gap: [under_member, baton_touch, both_targets] — ❌ target mis-parsed, "選んだカード以外" lost

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

### C7. `PL!S-bp7-004-R` 黒澤ダイヤ ab#1 — gap: [under_member, placement_order] — ⚠️/❌ look source + rest placement wrong

Japanese:
> ライブ開始時：自分のデッキの**下**からカードを3枚見る。その中から好きな枚数を好きな順番でデッキの**下**に置き、残りを控え室に置く。

Parsed:
```json
{"action":"look_and_select",
 "look_action":{"action":"look_at","count":3,"target":"self"},   // ❌ no source (should be deck_bottom)
 "select_action":{"action":"select_cards","discard_remaining":true,
   "reveal":false,"destination":"discard",                       // ❌ should have deck_bottom placement
   "text":"好きな枚数を好きな順番でデッキの下に置き、残りを控え室に置く"}}
```
- ❌ `look_action` has **no `source`** — it should be `deck_bottom` (デッキの下から見る).
- ❌ `select_action` only has `destination:"discard"` + `discard_remaining:true` — the "好きな順番で**デッキの下**に置き" part is missing entirely (no `destination:deck_bottom`, no `placement_order`).
- `under_member` flag spurious (it's デッキの下, not メンバーの下).
- Fix: look_action `source:deck_bottom`; select_action needs both a deck_bottom placement (any_order) and a discard remaining.

### C8. `PL!S-bp7-008-R` 小原鞠莉 ab#0 — gap: [under_member, placement_order] — ❌ "残りをデッキの下" parsed as discard

Japanese:
> 登場：自分のデッキの上からカードを3枚見る。その中から好きな枚数を好きな順番でデッキの上に置き、残りを**好きな順番でデッキの下**に置く。

Parsed:
```json
{"action":"look_and_select",
 "look_action":{"action":"look_at","source":"deck_top","count":3,"target":"self"},
 "select_action":{"action":"select_cards","discard_remaining":true,
   "destination":"deck_top","reveal":false,
   "text":"好きな枚数を好きな順番でデッキの上に置き、残りを好きな順番でデッキの下に置く"}}
```
- ✅ look from `deck_top`, selected → `deck_top`.
- ❌ **"残りを好きな順番でデッキの下に置く" is parsed as `discard_remaining:true`** — the discard zone is wrong; remaining must go to **deck_bottom** in chosen order, not to the discard. There's no `placement_order` on the discard_remaining either.
- `under_member` flag spurious.
- Fix: select_action needs `destination:deck_top` (any_order) for selected AND `deck_bottom` (any_order) for remaining — the current `look_and_select` schema only supports one destination + discard_remaining.

### C9. `PL!HS-PR-035-PR` 百生吟子 ab#0 — gap: [placement_order] — ❌ "相手のデッキの下に置く" parsed as select

Japanese:
> 登場：相手の控え室にあるメンバーカードを3枚選び、相手のデッキの下に好きな順番で置いてもよい。そうした場合、相手のステージにいる元々持つブレードの数が3つ以下のメンバー1人をウェイトにする。

Parsed:
```json
"action":"sequential","actions":[
  {"action":"select","source":"discard","count":3,"card_type":"member_card",
   "target":"opponent","optional":true,"placement_order":"any_order"},   // ❌ select, no destination
  {"action":"change_state","source":"selected_cards","state_change":"wait","count":1,
   "card_type":"member_card","target":"opponent","original_value":true,
   "blade_limit":3,"blade_limit_operator":"<="}]}
```
- ✅ `placement_order:any_order` present; ✅ wait effect with `blade_limit:3, <=`.
- ❌ **First step is `action:"select"` with no destination** — it should be a `move_cards` to opponent's `deck_bottom`. Selecting from discard ≠ placing them under the opponent's deck.
- Fix: step 1 → `move_cards{source:discard, destination:deck_bottom, target:opponent, count:3, optional, placement_order:any_order}`.

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
| B4 | PL!SP-bp7-005-R＋ 葉月 恋 ab#0 | energy_deck | ⚠️ | compound trigger (energy_zone→energy_deck) dropped |
| B5 | PL!N-bp7-006-R＋ 近江彼方 ab#1 | card_property | ⚠️ | condition `location:"stage"` wrong (should be preceding_moved); live-card OR lost |
| B6 | PL!N-bp7-028-L Cooking with Love ab#0 | under_member, card_property | ⚠️ | condition `location:"stage"` wrong (should be discard); live-card AND lost |
| C1 | PL!S-bp7-009-R 黒澤ルビィ ab#0 | lose_resource | ❌ | blade-loss → `action:"custom"` |
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

### CLEAN-G2. `destination: null` on "…の下に置く" (member-under placement)

- D1 `PL!S-bp7-005-R＋` 渡辺 曜 ab#0 — "自分の控え室にあるメンバーカード1枚を、自分のステージにいるメンバー1人の**下に置く**" →
  `move_cards{source:"discard", destination:null}` (line 6351). Should be `destination:"under_member"`.

Same class as C2 (choice-subtree) but here at top level — so the fix is a plain
`move_cards` under-member rule, not only a choice-option rule.

### CLEAN-G3. gain_resource missing the "メンバーカードが下に置かれている" condition

- D2 `PL!S-bp7-005-R＋` 渡辺 曜 ab#1 — "自分のステージにいる、**メンバーカードが下に置かれている**『Aqours』のメンバーは、ブレードを得る" →
  `gain_resource{blade, self, Aqours}` with **no condition at all** (line 6368). The
  "has a member card underneath" filter is dropped, so the blade is granted
  unconditionally to all Aqours members. Needs a `condition{location:under_member}`-style
  gate (same field gap as B1/B2 but the *subject* filter, not the per-unit source).

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

### CLEAN-G10. "元々持つハートがすべてheart04になる" → `custom`

- D15 `PL!S-bp7-024-L` ときめき分類学 ab#0 — "ライブ終了時まで、自分のステージにいる『Aqours』のメンバー1人は、**元々持つハートがすべてheart04になる**。" →
  `action:"custom"` (line 36479) with `heart_colors:[heart04], original_value:true`. This is a
  `set_heart_type` effect (compare C4's shape) but it fell to custom.

### CLEAN-G11. "より多くのブレードを持つ" max-comparison missing

- D16 `PL!N-bp7-027-L` オードリー ab#0 — "そのメンバーが、自分と相手のステージにいる**ほかのすべてのメンバーより多くのブレードを持つ**場合、このカードのスコアを＋１する。" →
  condition `location_condition{stage, exclude_self, scope:both, all}` (line 36953) with **no
  blade comparison** — the "has more blade than all others" predicate is not represented.

### CLEAN-G12. "ライブカード置き場から手札に戻す" → `custom`

- D17 `PL!N-bp7-030-L` Cheer Mode ab#1 — "このカードを**ライブカード置き場から手札に戻す**。" →
  `action:"custom"` (line 37104). No `move_cards{live_card_zone→hand, self}` emitted.

### CLEAN-G13. Optional add-to-hand action missing (Like a Treasure)

- D18 `PL!N-bp7-031-L` Like a Treasure ab#1 — "それらのカードの中から『虹ヶ咲』の**ライブカードを1枚手札に加えてもよい**。そうしたとき、このカードのスコアを＋１する。" →
  parsed as `compound condition + modify_score` only (line 37150). The **move to hand is
  absent** — the effect just scores, never actually adds the card.

### CLEAN-G14. Blade-limit filter misparsed as a resource gain (Fire Bird)

- D19 `PL!N-sd2-026-P` Fire Bird ab#0 — "自分のステージにいる**ブレードを4つ以上持つ**『虹ヶ咲』のメンバー1人は、ライブ終了時まで、heart02×2を得る。" →
  parsed as `sequential[gain_resource{blade, count:4}, gain_resource{heart, count:4}]` (lines 22470-22494).
  The **blade-4 filter became a blade+4 grant**, and the heart count 2 became 4. Severely wrong.

### CLEAN-G15. Triple-name cost condition → only one name survives

- D20 `LL-bp7-001-R＋` 国木田花丸&優木せつ菜&嵐 千砂都 ab#0 — "自分の手札から「国木田花丸」と「優木せつ菜」と「嵐千砂都」のメンバーカードを**それぞれ1枚ずつ**控え室に置いてもよい。そうしたとき、このカードのコストは10になる。" →
  `condition{location_condition, characters:[嵐千砂都], locations:[discard,hand], count:1, operator:"="}` + `modify_cost{characters:[嵐千砂都]}` (lines 37766-37787).
  Only「嵐千砂都」survives; 国木田花丸 and 優木せつ菜 are dropped, the
  "それぞれ1枚ずつ" (1 of each) is lost, and `modify_cost` has **no value:10 / operation:set**.

### CLEAN-G16. Shuffle-to-deck-bottom action folded into condition

- D21 `PL!SP-bp7-028-L` 未来の音が聴こえる ab#0 — "自分の控え室にある『Liella!』のメンバーカードを**9枚選び、シャッフルし、デッキの一番下に置いてもよい。そうしたとき、**…すべてのメンバーはブレードを得る。" →
  parsed as `condition{group_condition, shuffle:true, count:9} + gain_resource{blade, all}` (lines 37700-37723).
  The **"そうしたとき"** structure is flattened: the shuffle+place-under is a condition, not an
  action, and the follow-up gain applies without checking the result actually happened.

### CLEAN-G17. Trigger "エネルギーがメンバーの下に置かれたとき" under-parsed

- D24 `PL!SP-bp7-016-N` 葉月 恋 ab#0 and D25 `PL!SP-bp7-005-R＋` 葉月 恋 ab#1 — "自分のカードの効果によって、自分の**エネルギー置き場にエネルギーが置かれたとき**" →
  `condition{comparison_condition, location:energy_zone, resource_type:energy, movement:moved}` (lines 37364, 7075).
  The trigger is expressed as a fuzzy comparison, not a `trigger_event{zone_change,
  destination:energy_zone}`. Borderline but a real trigger-modeling gap (compare 桜小路きな子 ab#1
  which did produce a `zone_change` trigger_event).

### CLEAN-G18. Color-diversity condition collapsed to a plain card count

- D27 `PL!N-bp7-020-N` エマ・ヴェルデ ab#0 — "それらのメンバーカードの中に**2種類以上のブレードハートの色**がある場合" →
  `condition{card_count_condition, card_type:member_card, count:1, >=}` (line 36786). The "2種類以上"
  (≥2 distinct blade-heart colors) requirement is lost; any 1 member card satisfies it.
  Compare Colorful Dreams ab#1 which correctly produced `unit:"types"`.

### CLEAN-G19. Select excludes self for a "this member + 1 other" target

- D26 `PL!S-bp7-005-R＋` 渡辺 曜 ab#2 — "このメンバー**と**自分のステージにいるほかの『Aqours』のメンバー1人を選ぶ。それらが持つ登場能力それぞれ1つを発動させる。" →
  the first select is `{source:"stage", count:1, exclude_self:true, group:Aqours}` (line 6406)
  and the `activate_ability` count is 1. The text selects **this member plus one other** (two
  targets) and activates *their* 登場 abilities; the JSON only ever targets the single non-self
  member, so this member's 登場 ability would never fire.

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
| 52 | LL-bp7-001-R＋ 国木田花丸&優木せつ菜&嵐千砂都 ab#0 | ❌ D20 | only 嵐千砂都 survives; modify_cost no value/op |
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

## Recommended order (updated)

1. Group B (B1–B6) — smallest field-gap fixes. **B1, B2 DONE (2026-08-05).**
2. **CLEAN-G1** (6 abilities) — recurring `source:"hand"`→`deck_bottom`. **DONE (2026-08-05): parser + engine DeckBottom draw branch + optional-pay routing; tests in `bp7_deck_bottom_source_test.rs`.**
3. **C4** (桜坂しずく ab#0) — under-member move + heart-copy. **DONE (2026-08-05): `_try_place_under_heart_copy` + `heart_copy` modifier; tests in `bp7_heart_copy_test.rs`.**
4. CLEAN-G5 + D20 (4 abilities) — character-name conditions (`characters` array). **CLEAN-G5 DONE (2026-08-05): parser character extraction + `preceding_moved` condition/move source; tests in `bp7_character_name_condition_test.rs`. D20 (鬼塚夏美 yell-source) still open.**
5. Group C parser-only (C1, C2, C5–C9).
6. CLEAN-G2/G3/G4/G8/G9/G10/G11/G12/G13/G16/D27 (one-off structure fixes).
7. CLEAN-G6/G7/G14/G15/D26 (structure + dynamic-count/cost restructuring).
8. C3 + CLEAN-G17 (overlap / trigger-modeling) last, engine-aware.
