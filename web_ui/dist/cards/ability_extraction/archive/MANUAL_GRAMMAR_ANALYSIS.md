# Manual Grammatical Analysis of 20 Longest Abilities

## Ability 1 (248 chars)
**Text**: 自分がエールしたとき、エールにより公開された自分のカードが持つブレードハートの中に{{heart_01.png|heart01}}、{{heart_02.png|heart02}}、{{heart_03.png|heart03}}、{{heart_04.png|heart04}}、{{heart_05.png|heart05}}、{{heart_06.png|heart06}}、{{icon_all.png|ハート}}のうち、3種類以上ある場合、ライブ終了時まで、{{heart_01.png|heart01}}を得る。6種類以上ある場合、さらにライブ終了時まで、「{{jyouji.png|常時}}ライブの合計スコアを＋１する。」を得る。

**Structure**:
- **Condition**: "自分がエールしたとき、エールにより公開された自分のカードが持つブレードハートの中に...3種類以上ある場合"
  - Nested condition: "6種類以上ある場合"
- **Effect 1**: "ライブ終了時まで、{{heart_01.png|heart01}}を得る"
- **Effect 2**: "さらにライブ終了時まで、「...」を得る" (conditional on 6+ types)

**Grammatical patterns**:
- Particle usage: "の中に" (in), "のうち" (among), "を得る" (gain object)
- Conditional structure: "場合" triggers effect
- Duration modifier: "ライブ終了時まで" (until live end)
- Nested conditionals: second condition modifies first effect

**Key insight**: This has **nested conditionals** where the second condition adds an additional effect. The parser needs to handle conditional stacking.

---

## Ability 2 (238 chars)
**Text**: 自分のステージに『蓮ノ空』のメンバーがいる場合、このカードを成功させるための必要ハートは、{{heart_01.png|heart01}}{{heart_01.png|heart01}}{{heart_00.png|heart0}}か、{{heart_04.png|heart04}}{{heart_04.png|heart04}}{{heart_00.png|heart0}}か、{{heart_05.png|heart05}}{{heart_05.png|heart05}}{{heart_00.png|heart0}}のうち、選んだ1つにしてもよい。

**Structure**:
- **Condition**: "自分のステージに『蓮ノ空』のメンバーがいる場合"
- **Effect**: "このカードを成功させるための必要ハートは...選んだ1つにしてもよい"
  - Choice structure: "Aか、Bか、Cのうち、選んだ1つ"

**Grammatical patterns**:
- Particle usage: "に" (to/in), "か" (or), "のうち" (among)
- Choice structure: "AかBかCのうち、選んだ1つ" (choose one from A, B, or C)
- Optional modifier: "してもよい" (may do)

**Key insight**: This is a **cost modification** ability, not a standard cost/effect. It changes the card's required hearts conditionally. The parser needs to distinguish between cost modification and standard effects.

---

## Ability 3 (237 chars)
**Text**: 自分の、ステージと控え室に名前の異なる『Liella!』のメンバーが5人以上いる場合、このカードを使用するためのコストは{{heart_02.png|heart02}}{{heart_02.png|heart02}}{{heart_03.png|heart03}}{{heart_03.png|heart03}}{{heart_06.png|heart06}}{{heart_06.png|heart06}}になる。\n(必要ハートを確認する時、エールで出た{{icon_b_all.png|ALLブレード}}は任意の色のハートとして扱う。)

**Structure**:
- **Condition**: "自分の、ステージと控え室に名前の異なる『Liella!』のメンバーが5人以上いる場合"
  - Compound location: "ステージと控え室に" (stage AND discard)
  - Modifier: "名前の異なる" (different names)
- **Effect**: "このカードを使用するためのコストは...になる" (cost modification)
- **Parenthetical**: Rule clarification

**Grammatical patterns**:
- Particle usage: "と" (and), "に" (in), "が" (subject marker)
- Compound location: "AとBに" (in A and B)
- State change: "になる" (becomes)
- Parenthetical for rules: ( ) contains game rule clarification

**Key insight**: Another **cost modification** ability with compound location ("stage and discard"). The parser needs to handle "AとBに" as a compound location condition.

---

## Ability 4 (232 chars)
**Text**: エールにより公開された自分のカードの中にライブカードが2枚以上あるか、自分のステージにいるメンバーが持つハートの中に{{heart_01.png|heart01}}、{{heart_04.png|heart04}}、{{heart_05.png|heart05}}、{{heart_02.png|heart02}}、{{heart_03.png|heart03}}、{{heart_06.png|heart06}}のうち合計5種類以上あるか、このターンに自分のステージにいるメンバーがエリアを移動している場合、このカードのスコアを＋１する。

**Structure**:
- **Condition**: "Aか、Bか、Cの場合" (A or B or C case)
  - A: "エールにより公開された自分のカードの中にライブカードが2枚以上ある"
  - B: "自分のステージにいるメンバーが持つハートの中に...合計5種類以上ある"
  - C: "このターンに自分のステージにいるメンバーがエリアを移動している"
- **Effect**: "このカードのスコアを＋１する"

**Grammatical patterns**:
- Particle usage: "の中に" (in), "か" (or), "を" (object), "に" (to)
- OR conditionals: "Aか、Bか、Cの場合" (if A or B or C)
- Temporal condition: "このターンに" (this turn)
- Action: "エリアを移動している" (moved area)

**Key insight**: This has **multiple OR conditions** joined by "か". The parser needs to handle "Aか、Bか、Cの場合" as a single compound OR condition, not three separate conditions.

---

## Ability 5 (229 chars)
**Text**: 自分のステージにいる『虹ヶ咲』のメンバーが持つ{{heart_01.png|heart01}}、{{heart_04.png|heart04}}、{{heart_05.png|heart05}}、{{heart_02.png|heart02}}、{{heart_03.png|heart03}}、{{heart_06.png|heart06}}のうち1色につき、このカードのスコアを＋１する。\n(エールをすべて行った後、エールで出た{{icon_draw.png|ドロー}}1つにつき、カードを1枚引く。)

**Structure**:
- **Condition**: "自分のステージにいる『虹ヶ咲』のメンバーが持つ...のうち1色につき" (per 1 color among)
- **Effect 1**: "このカードのスコアを＋１する"
- **Effect 2** (parenthetical): "エールをすべて行った後、エールで出た...1つにつき、カードを1枚引く"

**Grammatical patterns**:
- Particle usage: "に" (in), "が" (subject), "のうち" (among), "につき" (per)
- Per-unit modifier: "1色につき" (per 1 color)
- Sequential timing: "エールをすべて行った後" (after completing all cheers)
- Per-unit effect: "1つにつき" (per 1)

**Key insight**: This has a **per-unit modifier** ("につき") that applies to the effect. Also has a parenthetical that contains a separate sequential effect. The parser needs to handle per-unit modifiers and parenthetical effects.

---

## Ability 6 (244 chars)
**Text**: 自分の成功ライブカード置き場かライブ中のライブカードの中に、必要ハートに含まれる{{heart_01.png|heart01}}が3の『虹ヶ咲』のライブカードがある場合、ライブ終了時まで、自分のステージにいる{{heart_06.png|heart06}}を持つ『虹ヶ咲』のメンバー1人は{{heart_06.png|heart06}}{{heart_06.png|heart06}}{{heart_06.png|heart06}}{{heart_06.png|heart06}}を得る。

**Structure**:
- **Condition**: "自分の成功ライブカード置き場かライブ中のライブカードの中に...がある場合"
  - OR location: "成功ライブカード置き場かライブ中のライブカードの中に"
  - Card specification: "必要ハートに含まれる{{heart_01.png|heart01}}が3の『虹ヶ咲』のライブカード"
- **Effect**: "ライブ終了時まで、自分のステージにいる{{heart_06.png|heart06}}を持つ『虹ヶ咲』のメンバー1人は...を得る"
  - Target specification in effect: "自分のステージにいる{{heart_06.png|heart06}}を持つ『虹ヶ咲』のメンバー1人"

**Grammatical patterns**:
- Particle usage: "か" (or), "の中に" (in), "が" (subject), "は" (topic), "を" (object)
- OR location: "AかBの中に" (in A or B)
- Complex card specification: "必要ハートに含まれるXがYのZ" (Z with X being Y in required hearts)
- Target specification in effect: "AにあるBのC人" (C people of B in A)

**Key insight**: Complex **card specification** with nested conditions ("required hearts containing X being Y"). The effect also has a target specification ("members with X in location"). The parser needs to handle nested specifications.

---

## Ability 7 (240 chars)
**Text**: 自分のステージにいるメンバーが持つハートの中に{{heart_01.png|heart01}}、{{heart_02.png|heart02}}、{{heart_03.png|heart03}}、{{heart_04.png|heart04}}、{{heart_05.png|heart05}}、{{heart_06.png|heart06}}がすべてある場合、ライブ終了時まで、{{icon_blade.png|ブレード}}{{icon_blade.png|ブレード}}を得る。

**Structure**:
- **Condition**: "自分のステージにいるメンバーが持つハートの中に...がすべてある場合"
- **Effect**: "ライブ終了時まで、{{icon_blade.png|ブレード}}{{icon_blade.png|ブレード}}を得る"

**Grammatical patterns**:
- Particle usage: "に" (in), "が" (subject), "の中に" (in), "を" (object)
- "すべてある" (all present) - universal quantifier
- Duration modifier: "ライブ終了時まで"

**Key insight**: This uses "すべてある" (all present) as a condition. The parser needs to distinguish "3種類以上ある" (3 or more types exist) from "すべてある" (all exist).

---

## Ability 8 (234 chars)
**Text**: {{icon_energy.png|E}}{{icon_energy.png|E}}このメンバーをステージから控え室に置く：自分の手札からコスト13以下の「優木せつ菜」のメンバーカードを1枚、このメンバーがいたエリアに登場させる。その後、自分のエネルギー置き場にあるエネルギー1枚をそのメンバーの下に置く。（メンバーの下に置かれているエネルギーカードではコストを支払えない。メンバーがステージから離れたとき、下に置かれているエネルギーカードはエネルギーデッキに置く。）

**Structure**:
- **Cost**: "{{icon_energy.png|E}}{{icon_energy.png|E}}このメンバーをステージから控え室に置く"
- **Effect**: Sequential actions
  - Action 1: "自分の手札からコスト13以下の「優木せつ菜」のメンバーカードを1枚、このメンバーがいたエリアに登場させる"
  - Action 2: "その後、自分のエネルギー置き場にあるエネルギー1枚をそのメンバーの下に置く"
- **Parenthetical**: Rule clarification

**Grammatical patterns**:
- Particle usage: "を" (object), "から" (from), "に" (to), "の" (possessive), "の下に" (under)
- Sequential marker: "その後" (after that)
- Source-destination: "ステージから控え室に" (from stage to discard)
- Cost limit: "コスト13以下" (cost 13 or less)
- Specific card: "「優木せつ菜」のメンバーカード" (member card named "優木せつ菜")
- Relative location: "このメンバーがいたエリア" (the area this member was in)
- Under placement: "そのメンバーの下に置く" (place under that member)

**Key insight**: This has **sequential actions** ("その後") and complex source/destination patterns. Also has specific card selection by name and relative location ("the area this member was in"). The parser needs to handle sequential actions and relative locations.

---

## Ability 9 (216 chars)
**Text**: {{icon_energy.png|E}}{{icon_energy.png|E}}{{icon_energy.png|E}}{{icon_energy.png|E}}{{icon_energy.png|E}}{{icon_energy.png|E}}支払ってもよい：ライブ終了時まで、{{icon_blade.png|ブレード}}{{icon_blade.png|ブレード}}{{icon_blade.png|ブレード}}を得る。

**Structure**:
- **Cost**: "{{icon_energy.png|E}}{{icon_energy.png|E}}{{icon_energy.png|E}}{{icon_energy.png|E}}{{icon_energy.png|E}}{{icon_energy.png|E}}支払ってもよい"
- **Effect**: "ライブ終了時まで、{{icon_blade.png|ブレード}}{{icon_blade.png|ブレード}}{{icon_blade.png|ブレード}}を得る"

**Grammatical patterns**:
- Particle usage: "を" (object)
- Optional cost: "支払ってもよい" (may pay)
- Duration modifier: "ライブ終了時まで"

**Key insight**: Simple optional energy cost with duration effect. This is a basic pattern the current parser handles.

---

## Ability 10 (212 chars)
**Text**: 自分のライブ中のライブカードの必要ハートの中に{{heart_01.png|heart01}}、{{heart_02.png|heart02}}、{{heart_03.png|heart03}}、{{heart_04.png|heart04}}、{{heart_05.png|heart05}}、{{heart_06.png|heart06}}がそれぞれ1以上含まれるかぎり、{{icon_all.png|ハート}}を得る。

**Structure**:
- **Condition**: "自分のライブ中のライブカードの必要ハートの中に...がそれぞれ1以上含まれるかぎり"
- **Effect**: "{{icon_all.png|ハート}}を得る"

**Grammatical patterns**:
- Particle usage: "の中に" (in), "が" (subject), "を" (object)
- "それぞれ1以上" (each 1 or more) - universal quantifier with count
- Duration marker: "かぎり" (as long as)

**Key insight**: Uses "かぎり" (as long as) as a duration marker, and "それぞれ1以上" (each 1 or more) as a condition. The parser needs to handle "かぎり" as a duration modifier.

---

## Ability 11 (209 chars)
**Text**: 自分のエネルギー置き場にあるエネルギー1枚をこのメンバーの下に置いてもよい。そうした場合、カードを1枚引き、ライブ終了時まで、自分のステージにいるメンバーは{{icon_blade.png|ブレード}}{{icon_blade.png|ブレード}}を得る。（メンバーの下に置かれているエネルギーカードではコストを支払えない。メンバーがステージから離れたとき、下に置かれているエネルギーカードはエネルギーデッキに置く。）

**Structure**:
- **Optional action**: "自分のエネルギー置き場にあるエネルギー1枚をこのメンバーの下に置いてもよい"
- **Conditional on optional**: "そうした場合" (if so)
- **Effect**: "カードを1枚引き、ライブ終了時まで、自分のステージにいるメンバーは...を得る"
- **Parenthetical**: Rule clarification

**Grammatical patterns**:
- Particle usage: "に" (in), "を" (object), "の下に" (under)
- Optional action: "してもよい"
- Conditional on optional: "そうした場合" (if did so / in that case)
- Sequential effect: "カードを1枚引き、..." (draw card, then...)

**Key insight**: This has an **optional action with a conditional follow-up** ("そうした場合"). The parser needs to handle "optional action + if so condition" structure.

---

## Ability 12 (206 chars)
**Text**: エールにより公開された自分の『虹ヶ咲』のメンバーカードが持つハートの中に{{heart_01.png|heart01}}、{{heart_02.png|heart02}}、{{heart_03.png|heart03}}、{{heart_04.png|heart04}}、{{heart_05.png|heart05}}、{{heart_06.png|heart06}}がある場合、このカードのスコアを＋１する。

**Structure**:
- **Condition**: "エールにより公開された自分の『虹ヶ咲』のメンバーカードが持つハートの中に...がある場合"
- **Effect**: "このカードのスコアを＋１する"

**Grammatical patterns**:
- Particle usage: "により" (by means of), "の" (possessive), "が" (subject), "の中に" (in), "を" (object)
- Passive/causal: "エールにより公開された" (revealed by cheer)

**Key insight**: Uses "により" (by means of) to indicate causation. The condition checks cards "revealed by cheer". The parser needs to handle "XによりY" (Y by means of X).

---

## Ability 13 (202 chars)
**Text**: {{center.png|センター}}自分の成功ライブカード置き場に{{icon_score.png|スコア}}を持つ『μ's』のカードが1枚ある場合、ライブ終了時まで、「{{jyouji.png|常時}}ライブの合計スコアを＋１する。」を得る。2枚以上ある場合、代わりに「{{jyouji.png|常時}}ライブの合計スコアを＋２する。」を得る。（この能力はセンターエリアに登場した場合のみ発動する。）

**Structure**:
- **Condition**: "自分の成功ライブカード置き場に...を持つ『μ's』のカードが1枚ある場合"
- **Effect 1**: "ライブ終了時まで、「...」を得る"
- **Alternative condition**: "2枚以上ある場合"
- **Effect 2**: "代わりに「...」を得る"
- **Parenthetical**: Activation condition

**Grammatical patterns**:
- Particle usage: "に" (in), "を持つ" (having), "が" (subject), "を" (object)
- Ability gain: "「...」を得る" (gain ability "...")
- Alternative effect: "代わりに" (instead)
- Activation condition in parenthetical

**Key insight**: This has **alternative effects** based on count thresholds ("1枚ある" vs "2枚以上ある"). Uses "代わりに" (instead) to indicate the alternative. The parser needs to handle threshold-based alternative effects.

---

## Ability 14 (198 chars)
**Text**: 以下から1つを選ぶ。
・自分のステージにいるこのメンバー以外の『Aqours』のメンバー1人は、ライブ終了時まで、{{icon_blade.png|ブレード}}を得る。
・自分のステージにいる『SaintSnow』のメンバー1人をポジションチェンジさせる。(このメンバーを今いるエリア以外のエリアに移動させる。そのエリアにメンバーがいる場合、そのメンバーはこのメンバーがいたエリアに移動させる。)

**Structure**:
- **Choice marker**: "以下から1つを選ぶ。" (choose one from the following)
- **Option 1**: "自分のステージにいるこのメンバー以外の『Aqours』のメンバー1人は、ライブ終了時まで、{{icon_blade.png|ブレード}}を得る。"
- **Option 2**: "自分のステージにいる『SaintSnow』のメンバー1人をポジションチェンジさせる。"
  - Parenthetical explanation of position change

**Grammatical patterns**:
- Particle usage: "から" (from), "を" (object), "以外" (except), "は" (topic)
- Choice structure: bullet points with "・"
- Except modifier: "このメンバー以外" (except this member)
- Position change: "ポジションチェンジさせる"

**Key insight**: This is a **choice effect** with bullet points. The parser needs to handle "以下から1つを選ぶ" followed by bullet-pointed options. Also has "以外" (except) modifier.

---

## Ability 15 (197 chars)
**Text**: {{center.png|センター}}このメンバーをウェイトにし、手札を1枚控え室に置く：このメンバー以外の『Aqours』のメンバー1人を自分のステージから控え室に置く。そうした場合、自分の控え室から、そのメンバーのコストに2を足した数に等しいコストの『Aqours』のメンバーカードを1枚、そのメンバーがいたエリアに登場させる。（この能力はセンターエリアに登場している場合のみ起動できる。）

**Structure**:
- **Cost**: "{{center.png|センター}}このメンバーをウェイトにし、手札を1枚控え室に置く"
  - Sequential cost actions: "ウェイトにし、...置く"
- **Effect**: "このメンバー以外の『Aqours』のメンバー1人を自分のステージから控え室に置く。そうした場合、..."
  - Action 1: Move member to discard
  - Conditional: "そうした場合" (if so)
  - Action 2: Bring out member with calculated cost
- **Parenthetical**: Activation condition

**Grammatical patterns**:
- Particle usage: "を" (object), "に" (to), "以外" (except), "から" (from), "の" (possessive)
- Sequential cost: "Aし、B" (do A, then B)
- Except modifier: "以外"
- Conditional on action: "そうした場合"
- Cost calculation: "コストに2を足した数に等しいコスト" (cost equal to cost + 2)

**Key insight**: Has **sequential cost actions** ("Aし、B") and cost calculation in the effect. The parser needs to handle sequential costs and mathematical cost calculations.

---

## Ability 16 (196 chars)
**Text**: 自分のステージにいるメンバーが持つハートの中に{{heart_01.png|heart01}}、{{heart_02.png|heart02}}、{{heart_03.png|heart03}}、{{heart_04.png|heart04}}、{{heart_05.png|heart05}}、{{heart_06.png|heart06}}がすべてある場合、このカードのスコアを＋１する。

**Structure**:
- **Condition**: "自分のステージにいるメンバーが持つハートの中に...がすべてある場合"
- **Effect**: "このカードのスコアを＋１する"

**Grammatical patterns**:
- Same as Ability 7

**Key insight**: Duplicate of Ability 7 pattern.

---

## Ability 17 (195 chars)
**Text**: {{icon_energy.png|E}}{{icon_energy.png|E}}支払ってもよい：自分のステージにコスト9以上の『EdelNote』のメンバーがいる場合、以下から1つを選ぶ。
・自分の控え室からコスト4以下の『EdelNote』のメンバーカードを1枚、メンバーのいないエリアに登場させる。
・このカードの必要ハートを{{heart_06.png|heart06}}減らす。

**Structure**:
- **Cost**: "{{icon_energy.png|E}}{{icon_energy.png|E}}支払ってもよい"
- **Condition**: "自分のステージにコスト9以上の『EdelNote』のメンバーがいる場合"
- **Choice effect**: "以下から1つを選ぶ"
  - Option 1: Move member to empty area
  - Option 2: Reduce required hearts

**Grammatical patterns**:
- Particle usage: "を" (object), "に" (to), "から" (from), "の" (possessive)
- Cost limit: "コスト9以上" (cost 9 or more)
- Choice structure with bullet points
- State change: "減らす" (reduce/decrease)

**Key insight**: Conditional choice effect. The choice is only available if the condition is met. The parser needs to handle "condition + choice effect" structure.

---

## Ability 18 (193 chars)
**Text**: 手札を1枚控え室に置いてもよい：自分のデッキの上からカードを3枚見る。その中から1枚を手札に加え、残りを控え室に置く。{{live_start.png|ライブ開始時}}{{icon_energy.png|E}}{{icon_energy.png|E}}支払ってもよい：ライブ終了時まで、{{icon_blade.png|ブレード}}{{icon_blade.png|ブレード}}を得る。

**Structure**:
- **Cost**: "手札を1枚控え室に置いてもよい"
- **Effect 1**: "自分のデッキの上からカードを3枚見る。その中から1枚を手札に加え、残りを控え室に置く"
  - "その中から" (from among them) pattern
- **Effect 2** (separate trigger): "{{live_start.png|ライブ開始時}}{{icon_energy.png|E}}{{icon_energy.png|E}}支払ってもよい：..."

**Grammatical patterns**:
- Particle usage: "を" (object), "に" (to), "から" (from), "の中から" (from among)
- "その中から" (from among them) - look and select pattern
- Sequential actions: "見る。その中から...加え、残りを...置く"
- Multiple triggers on same card

**Key insight**: This has **"その中から" (from among them)** pattern for look-then-select. Also has multiple triggers on the same card. The parser needs to handle "look at cards, then select from among them" pattern.

---

## Ability 19 (193 chars)
**Text**: 自分のステージに{{heart_02.png|heart02}}を4つ以上持つメンバーがいる場合、このカードのスコアを＋２し、必要ハートは{{heart_02.png|heart02}}{{heart_02.png|heart02}}{{heart_02.png|heart02}}{{heart_02.png|heart02}}{{heart_02.png|heart02}}になる。

**Structure**:
- **Condition**: "自分のステージに{{heart_02.png|heart02}}を4つ以上持つメンバーがいる場合"
- **Effect**: "このカードのスコアを＋２し、必要ハートは...になる"
  - Sequential effects: "スコアを＋２し、必要ハートは...になる"
  - State change: "必要ハートは...になる" (required hearts become...)

**Grammatical patterns**:
- Particle usage: "に" (in), "を" (object), "が" (subject), "は" (topic)
- Sequential effects: "Aし、B" (do A, then B)
- State change: "は...になる" (becomes)

**Key insight**: Has **sequential effects** ("Aし、B") and state change for required hearts. The parser needs to handle sequential effects and state changes.

---

## Ability 20 (189 chars)
**Text**: 手札の『蓮ノ空』のカードを2枚控え室に置いてもよい：{{heart_01.png|heart01}}か{{heart_04.png|heart04}}か{{heart_05.png|heart05}}か{{heart_06.png|heart06}}のうち、1つを選ぶ。ライブ終了時まで、自分のステージにいるこのメンバー以外の『蓮ノ空』のメンバー1人は、選んだハートを2つ得る。

**Structure**:
- **Cost**: "手札の『蓮ノ空』のカードを2枚控え室に置いてもよい"
- **Effect**: "{{heart_01.png|heart01}}か{{heart_04.png|heart04}}か{{heart_05.png|heart05}}か{{heart_06.png|heart06}}のうち、1つを選ぶ。ライブ終了時まで、..."
  - Choice: "AかBかCかDのうち、1つを選ぶ"
  - Effect uses choice: "選んだハートを2つ得る"

**Grammatical patterns**:
- Particle usage: "の" (possessive), "を" (object), "に" (to), "か" (or), "のうち" (among)
- Choice with "か" and "のうち"
- Reference to choice: "選んだハート" (the chosen heart)

**Key insight**: This has a **choice that affects the effect** ("choose one, then gain 2 of the chosen"). The parser needs to track the choice and reference it later in the effect.

---

# Summary of Key Grammatical Patterns

## 1. Nested Conditionals
- "Aの場合、B。Cの場合、さらにD" (if A, then B. if C, additionally D)
- Need to handle conditional stacking

## 2. Cost Modification
- "コストは...になる" (cost becomes...)
- "必要ハートは...にしてもよい" (required hearts may become...)
- Distinguish from standard effects

## 3. Compound Locations
- "AとBに" (in A and B)
- "AかBの中に" (in A or B)

## 4. OR Conditionals
- "Aか、Bか、Cの場合" (if A or B or C)
- Multiple conditions joined by "か"

## 5. Per-Unit Modifiers
- "Xにつき" (per X)
- "1つにつき" (per 1)
- Affects effect based on count

## 6. Complex Card Specifications
- "必要ハートに含まれるXがYのZ" (Z with X being Y in required hearts)
- Nested specifications

## 7. Sequential Actions
- "その後" (after that)
- "Aし、B" (do A, then B)
- In both costs and effects

## 8. Relative Locations
- "このメンバーがいたエリア" (the area this member was in)
- "そのメンバーの下に置く" (place under that member)

## 9. Optional Action with Conditional Follow-up
- "Aしてもよい。そうした場合、B" (may do A. if so, B)
- "そうした場合" = conditional on optional action

## 10. Alternative Effects
- "1枚ある場合、X。2枚以上ある場合、代わりにY"
- Threshold-based alternatives with "代わりに"

## 11. Choice Effects
- "以下から1つを選ぶ" + bullet points
- Choice that affects effect ("選んだX" = chosen X)

## 12. "その中から" Pattern
- "カードを3枚見る。その中から1枚を..." (look at 3 cards. from among them, 1 to...)
- Look-then-select pattern

## 13. Universal Quantifiers
- "すべてある" (all exist)
- "それぞれ1以上" (each 1 or more)
- Distinguish from existential quantifiers

## 14. Duration Markers
- "ライブ終了時まで" (until live end)
- "かぎり" (as long as)

## 15. Except Modifiers
- "以外" (except)
- "このメンバー以外" (except this member)

## 16. Cost Calculation
- "コストに2を足した数に等しいコスト" (cost equal to cost + 2)
- Mathematical expressions in cost

## 17. Ability Gain
- "「...」を得る" (gain ability "...")
- Quoted ability names

## 18. State Changes
- "になる" (become)
- "減らす" (reduce)
- Not just move_cards or gain_resource
