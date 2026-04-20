# Comprehensive Rule Set - Based on Analysis of 609 Abilities

## Pattern Frequencies (from actual data)

| Pattern | Count | Frequency |
|---------|-------|-----------|
| cost_effect_separator (：) | 206 | 34% |
| conditional_bai (場合) | 259 | 43% |
| sequential_comma (、) | varies | high |
| compound_katsu (かつ) | 20 | 3% |
| duration_kagiri (かぎり) | 33 | 5% |
| duration_live_end (ライブ終了時まで) | varies | medium |
| per_unit_tsuki (につき) | varies | low |
| optional_may (もよい) | varies | medium |
| sequential_sono_go (その後) | 15 | 2% |
| choice (以下から1つを選ぶ) | 9 | 1.5% |
| conditional_toki (とき) | 43 | 7% |
| bullet_points (・) | 33 | 5% |
| parenthetical_notes (（）) | 24 | 4% |
| newlines (\n) | 33 | 5% |

## Action Verbs (sorted by frequency)

| Verb | Count | Context |
|------|-------|---------|
| 置く | 400+ | 控え室に置く, ステージに置く, etc. |
| 加える | 79 | 手札に加える |
| 引く | 65 | カードを引く |
| 得る | varies | ブレードを得る, ハートを得る, 能力を得る |
| 登場させる | 19 | ステージに登場させる |
| ウェイトにする | 34 | ウェイトにする |
| アクティブにする | 24 | エネルギーをアクティブにする |
| 見る | varies | カードを見る |
| 選ぶ | varies | カードを選ぶ |
| 発動させる | varies | 能力を発動させる |
| 公開する | varies | 手札を公開する |

## Locations (sorted by frequency)

| Location | Count | Context |
|----------|-------|---------|
| 控え室 | 400+ | 控え室に置く, 控え室から, 控え室にある |
| ステージ | 300+ | ステージに置く, ステージから, ステージにいる |
| 手札 | 200+ | 手札に置く, 手札から, 手札にある |
| デッキ | 100+ | デッキの上, デッキから |
| エネルギーデッキ | 50+ | エネルギーデッキから |
| エネルギー置き場 | 30+ | エネルギー置き場にある |
| ライブカード置き場 | 20+ | ライブカード置き場にある |
| 成功ライブカード置き場 | 10+ | 成功ライブカード置き場にある |

## Card Types (sorted by frequency)

| Card Type | Count | Context |
|-----------|-------|---------|
| メンバーカード | 100+ | メンバーカード |
| ライブカード | 80+ | ライブカード |
| エネルギーカード | 60+ | エネルギーカード |
| ドルチェストラ | 5+ | ドルチェストラ (rare) |

## Operators (sorted by frequency)

| Operator | Count | Meaning |
|----------|-------|---------|
| 以上 | 50+ | greater than or equal |
| 以下 | 40+ | less than or equal |
| より少ない | 5+ | less than |
| より多い | 3+ | greater than |

## Properties (sorted by frequency)

| Property | Count | Context |
|----------|-------|---------|
| コスト | 89 | コスト4以下 |
| ブレード | 129 | ブレードを得る, ブレードが3つ以上 |
| スコア | 108 | スコアを＋１する |
| ハート | 100 | ハートを得る |

## Top Groups (from actual data)

| Group | Count | Unit |
|-------|-------|------|
| Aqours | 40+ | Love Live! Sunshine!! |
| μ's | 30+ | Love Live! |
| Liella! | 25+ | Love Live! Superstar!! |
|虹ヶ咲 | 20+ | Love Live! Nijigasaki |
| Printemps | 15+ | μ's subunit |
| Lily White | 10+ | μ's subunit |
| BiBi | 10+ | μ's subunit |
| A-RISE | 8+ | Nijigasaki subunit |
| DiverDiva | 5+ | Nijigasaki subunit |
| QU4RTZ | 5+ | Nijigasaki subunit |

## Core Grammatical Rules

### Rule 1: Cost-Effect Structure
**Pattern:** `[cost] ： [effect]`
**Frequency:** 34% (206/609)
**Examples:**
- `このメンバーをステージから控え室に置く：自分の控え室からライブカードを1枚手札に加える。`
- `手札を1枚控え室に置いてもよい：自分のデッキの上からカードを3枚見る。`

**Sub-rules:**
- Cost can be optional: `～てもよい`
- Cost can be energy: `{{icon_energy.png|E}}`
- Cost can be card movement: `[source]から[destination]に置く`
- Effect can be conditional: `[condition]場合、[action]`

### Rule 2: Conditional Structure
**Pattern:** `[condition] 場合、[action]` or `[condition] とき、[action]`
**Frequency:** 50% (302/609)
**Examples:**
- `自分のステージに『Aqours』のメンバーがいる場合、ブレードを得る。`
- `ライブ開始時、カードを1枚引く。`

**Condition components:**
- Target: 自分の, 相手の
- Location: 控え室, 手札, ステージ, デッキ
- Card type: メンバーカード, ライブカード, エネルギーカード
- Group: 『group name』
- Property: コスト, スコア, ブレード, ハート
- Operator: 以上, 以下, より少ない, より多い
- Value: number
- Count: number + 枚/人

### Rule 3: Sequential Actions
**Pattern:** `[action1]、[action2]` or `[action1]。その後、[action2]`
**Frequency:** High
**Examples:**
- `カードを1枚引き、手札を1枚控え室に置く。`
- `カードを3枚見る。その後、1枚を手札に加え、残りを控え室に置く。`

**Sub-rules:**
- Comma separator: `、` for sequential actions
- Explicit marker: `その後` for explicit sequencing
- Variable reference: `これにより引いた枚数と同じ枚数`

### Rule 4: Choice Structure
**Pattern:** `以下から1つを選ぶ\n・[option1]\n・[option2]`
**Frequency:** 1.5% (9/609)
**Examples:**
- `以下から1つを選ぶ。\n・相手のステージにいるメンバーをウェイトにする。\n・カードを1枚引く。`

**Sub-rules:**
- Bullet point separator: `・`
- Each option can have its own condition
- Some choices allow multiple: `代わりに1つ以上を選ぶ`

### Rule 5: Compound Conditions
**Pattern:** `[condition1] かつ [condition2] 場合、[action]`
**Frequency:** 3% (20/609)
**Examples:**
- `自分のステージに『Aqours』のメンバーがおり、かつエネルギーが7枚以上ある場合、エネルギーをアクティブにする。`
- `スコアが相手より高く、かつ自分のステージにメンバーがいる場合、ブレードを得る。`

**Sub-rules:**
- Operator: `かつ` (and)
- Each condition can be complex
- Can combine different condition types

### Rule 6: Duration Modifiers
**Pattern:** `[condition] かぎり、[action]` or `ライブ終了時まで、[action]`
**Frequency:** 5-10%
**Examples:**
- `このメンバーの下にエネルギーカードが2枚以上置かれているかぎり、スコアを＋１する。`
- `ライブ終了時まで、ブレードを得る。`

**Sub-rules:**
- Duration: `かぎり` (as long as)
- Duration: `ライブ終了時まで` (until end of live)
- Duration: `このターンの間` (during this turn)
- Duration: `～場合のみ` (only if)

### Rule 7: Per-Unit Modifiers
**Pattern:** `[target] [count]につき、[action]`
**Frequency:** Low but important
**Examples:**
- `自分のステージにいるメンバー1人につき、カードを1枚引く。`
- `成功ライブカード置き場にあるカード1枚につき、ハートを1つ得る。`

**Sub-rules:**
- Per-unit marker: `につき` (per)
- Can reference variable count: `これにより引いた枚数と同じ枚数`

### Rule 8: Optional Actions
**Pattern:** `[action] てもよい` or `[action] ことができる`
**Frequency:** Medium
**Examples:**
- `手札を1枚控え室に置いてもよい`
- `カードを手札に加えてもよい`

**Sub-rules:**
- Optional marker: `てもよい` (may)
- Optional marker: `ことができる` (can)
- Can apply to cost or effect

### Rule 9: Card Movement Patterns
**Pattern:** `[verb] [object] [source] [destination] [count] [modifiers]`

**Common patterns:**
- Draw: `カードをN枚引く` (source: deck, destination: hand, implied)
- Add to hand: `[card_type]をN枚手札に加える` (destination: hand)
- Discard: `[source]からN枚控え室に置く` (destination: discard)
- Wait: `[source]からN枚ウェイトにする` (destination: wait)
- Deploy: `[source]からステージに登場させる` (destination: stage)
- Look at: `[source]からN枚見る` (no movement)

**Source patterns:**
- `自分のデッキの上から` (from top of own deck)
- `自分の控え室から` (from own discard)
- `自分の手札から` (from own hand)
- `自分のステージから` (from own stage)
- `相手の控え室から` (from opponent's discard)

**Destination patterns:**
- `手札に加える` (add to hand)
- `控え室に置く` (place in discard)
- `ウェイトにする` (to wait)
- `ステージに登場させる` (deploy to stage)
- `デッキの上に置く` (place on top of deck)

### Rule 10: Resource Gain/Loss
**Pattern:** `[resource]を[operation]`

**Resources:**
- `ブレード` (blade)
- `ハート` (heart)
- `スコア` (score)
- `エネルギー` (energy)

**Operations:**
- `得る` (gain)
- `支払う` (pay)
- `＋N` (add N)
- `－N` (subtract N)

**Sub-rules:**
- Icon repetition indicates count: `ICONICON` = 2
- Heart selection: `ICONかICONかICONのうち、Nつを選ぶ`
- Duration applies: `ライブ終了時まで、ブレードを得る`

### Rule 11: State Manipulation
**Pattern:** `[target]を[state]にする`

**States:**
- `ウェイト状態` (wait state)
- `アクティブ状態` (active state)
- `登場している` (deployed)

**Sub-rules:**
- `ウェイトにする` = move to wait
- `アクティブにする` = activate
- State can be condition: `ウェイト状態のメンバー`

### Rule 12: Ability Activation
**Pattern:** `[target]の[ability]を発動させる`

**Examples:**
- `このカードの登場能力を発動させる`
- `控え室に置いたメンバーカードの登場能力1つを発動させる`

**Sub-rules:**
- Can trigger from discard
- Can trigger from hand
- Can trigger specific ability types
- May require cost payment

### Rule 13: Ability Gain
**Pattern:** `「[ability text]」を得る`

**Examples:**
- `「常時ライブの合計スコアを＋１する。」を得る`
- `「ブレードを得る」を得る`

**Sub-rules:**
- Ability text in quotes: 「...」
- Can be conditional
- Can have duration

### Rule 14: Group Conditions
**Pattern:** `『group name』の[card type]`

**Examples:**
- `『Aqours』のメンバー`
- `『Liella!』のライブカード`

**Sub-rules:**
- Group name in brackets: 『...』
- Can be card type specific
- Can be location specific

### Rule 15: Baton Touch
**Pattern:** `[group]のメンバーからバトンタッチして登場`

**Examples:**
- `『Liella!』のメンバーからバトンタッチして登場`
- `バトンタッチして登場しており、かつ...`

**Sub-rules:**
- Implies cost comparison
- State marker: `おり` indicates completed state
- Can be part of compound condition

### Rule 16: Parenthetical Notes
**Pattern:** `（note text）`

**Examples:**
- `（ウェイト状態のメンバーが持つブレードは、エールで公開する枚数を増やさない。）`
- `（登場能力がコストを持つ場合、支払って発動させる。）`

**Sub-rules:**
- Contains exceptions
- Contains clarifications
- Contains cost requirements
- Contains cleanup instructions

### Rule 17: Variable References
**Pattern:** `[variable reference]`

**Examples:**
- `これにより引いた枚数と同じ枚数`
- `その中から好きな枚数`
- `残りを控え室に置く`

**Sub-rules:**
- `これにより` (by this)
- `その中から` (from those)
- `残り` (the rest)
- `好きな` (any/favorite)

### Rule 18: Count Modifiers
**Pattern:** `[count][modifier]`

**Modifiers:**
- `まで` (up to)
- `以上` (at least)
- `以下` (at most)
- `のみ` (only)
- `ずつ` (each)

**Examples:**
- `2人まで` (up to 2 people)
- `3枚以上` (at least 3 cards)
- `1枚のみ` (only 1 card)
- `1人ずつ` (1 person each)

### Rule 19: Position Requirements
**Pattern:** `[position]にいる`

**Positions:**
- `センターエリア` (center area)
- `左サイドエリア` (left side area)
- `右サイドエリア` (right side area)
- `ドルチェストラエリア` (dollchestra area)

**Sub-rules:**
- Can be condition
- Can be trigger
- Can be requirement

### Rule 20: Score Modification
**Pattern:** `[scope]スコアを[operation]`

**Examples:**
- `ライブの合計スコアを＋１する`
- `このカードのスコアを＋５する`
- `スコアを－１する`

**Sub-rules:**
- Scope: ライブの (live's), このカードの (this card's)
- Operation: ＋N, －N
- Can be conditional
- Can have duration

### Rule 21: Position Change (NEW - discovered in exploratory analysis)
**Pattern:** `[target]をポジションチェンジする`

**Examples:**
- `このメンバーをポジションチェンジしてもよい。(このメンバーを今いるエリア以外のエリアに移動させる。そのエリアにメンバーがいる場合、そのメンバーはこのメンバーがいたエリアに移動させる。)`

**Sub-rules:**
- Moves member to different area
- Swaps with member in target area
- Optional: てもよい
- Parenthetical note explains swap mechanic

### Rule 22: Position-Specific Conditions (NEW - discovered in exploratory analysis)
**Pattern:** `[position]に登場しているなら、[action]`

**Examples:**
- `ステージの左サイドエリアに登場しているなら、カードを2枚引く。`

**Sub-rules:**
- Position: 左サイドエリア (left side), 右サイドエリア (right side), センターエリア (center)
- Condition marker: ～なら (if)
- Can be trigger condition

### Rule 23: Cost Sum Conditions (NEW - discovered in exploratory analysis)
**Pattern:** `[property]の合計が、[values]のいずれかの場合、[action]`

**Examples:**
- `公開したカードのコストの合計が、10、20、30、40、50のいずれかの場合、能力を得る。`

**Sub-rules:**
- Property: コスト (cost), スコア (score)
- List of values: 10、20、30、40、50
- Condition: いずれかの場合 (if any of)
- Can trigger ability gain

### Rule 24: Energy Count Conditions (NEW - discovered in exploratory analysis)
**Pattern:** `自分のエネルギーが[count]枚以上ある場合、[action]`

**Examples:**
- `自分のエネルギーが11枚以上ある場合、ライブカードを1枚手札に加える。`
- `自分のエネルギーが7枚以上ある場合、エネルギーカードをウェイト状態で置く。`

**Sub-rules:**
- Target: 自分の (self)
- Object: エネルギー (energy)
- Count: N枚
- Operator: 以上 (at least)
- Condition marker: 場合

### Rule 25: Heart Icon Selection (NEW - discovered in exploratory analysis)
**Pattern:** `ICONかICONかICONのうち、Nつを選ぶ`

**Examples:**
- `{{heart_01.png|heart01}}か{{heart_03.png|heart03}}か{{heart_06.png|heart06}}のうち、1つを選ぶ。`

**Sub-rules:**
- Multiple heart icons listed
- Selection: Nつを選ぶ (choose N)
- Can be followed by per-unit modifier
- Can have duration

### Rule 26: Live Card Count Conditions (NEW - discovered in exploratory analysis)
**Pattern:** `自分のライブ中のカードが[count]枚以上あり、その中に[group]のライブカードを[count]枚以上含む場合、[action]`

**Examples:**
- `自分のライブ中のカードが3枚以上あり、その中に『虹ヶ咲』のライブカードを1枚以上含む場合、ブレードを得る。`

**Sub-rules:**
- Scope: 自分のライブ中の (during own live)
- Total count: N枚以上
- Group condition: group card count
- Compound condition with group

### Rule 27: Card Reveal Actions (NEW - discovered in exploratory analysis)
**Pattern:** `[source]にある[card_type]を[count]枚公開する`

**Examples:**
- `手札にあるメンバーカードを好きな枚数公開する`

**Sub-rules:**
- Source: 手札, 控え室, etc.
- Card type: メンバーカード, etc.
- Count: 好きな枚数 (any number)
- Action: 公開する (reveal)
- Can be cost for ability gain

### Rule 28: Cost-Based Ability Activation (NEW - discovered in exploratory analysis)
**Pattern:** `このカードのコストが[count]以上になった場合、[action]`

**Examples:**
- `このカードのコストが10以上になった場合、ライブの合計スコアを＋１する。`

**Sub-rules:**
- Target: このカード (this card)
- Property: コスト (cost)
- Condition: became N or more
- Can be trigger
- Can be conditional

### Rule 29: Area-Specific Deploy (NEW - discovered in exploratory analysis)
**Pattern:** `[position]に登場しているなら、[action]`

**Examples:**
- `ステージの左サイドエリアに登場しているなら、カードを2枚引く。`

**Sub-rules:**
- Position: specific area
- Condition: if deployed there
- Can be alternative trigger

### Rule 30: Deck Refresh (NEW - discovered in exploratory analysis)
**Pattern:** `デッキがリフレッシュしていた`

**Examples:**
- (found in parenthetical notes in some abilities)

**Sub-rules:**
- Indicates deck was refreshed
- Can be condition
- Usually cleanup instruction

## Structural Templates (from actual data)

### Template 1: Simple draw + discard
```
カードをN枚引き、手札をN枚控え室に置く。
```
**Count:** 5 abilities

### Template 2: Energy activation
```
エネルギーをN枚アクティブにする。
```
**Count:** 2 abilities

### Template 3: Compound condition with group
```
自分のステージのエリアすべてにGROUPのメンバーが登場しており、かつ名前が異なる場合、QUOTEを得る。
```
**Count:** 2 abilities

### Template 4: Cost: effect with card movement
```
このメンバーをステージから控え室に置く：自分の控え室から[card_type]をN枚手札に加える。
```
**Count:** Multiple variants

### Template 5: Optional cost with look + add + discard
```
手札をN枚控え室に置いてもよい：自分のデッキの上からカードをN枚見る。その中からN枚を手札に加え、残りを控え室に置く。
```
**Count:** 1 ability (complex)

### Template 6: Energy cost with duration resource gain
```
ICON支払ってもよい：ライブ終了時まで、ICONICONを得る。
```
**Count:** Multiple variants

### Template 7: Look + reorder + discard
```
自分のデッキの上からカードをN枚見る。その中から好きな枚数を好きな順番でデッキの上に置き、残りを控え室に置く。
```
**Count:** 1 ability

### Template 8: Heart selection with per-unit
```
ICONかICONかICONのうち、Nつを選ぶ。ライブ終了時まで、自分の成功ライブカード置き場にあるカードN枚につき、選んだハートをNつ得る。
```
**Count:** 1 ability (very complex)

## Parsing Strategy

Based on this analysis, the parser should:

1. **First pass:** Identify structural markers
   - `：` → split cost and effect
   - `場合`, `とき` → split condition and action
   - `かつ` → split compound conditions
   - `その後` → split sequential actions
   - `以下から1つを選ぶ` → identify choice
   - `かぎり` → identify duration
   - `につき` → identify per-unit
   - `、` → identify sequential actions
   - `なら` → identify position-specific conditions
   - `いずれかの場合` → identify list conditions

2. **Second pass:** Parse components within each segment
   - Extract target (自分の, 相手の)
   - Extract location (控え室, 手札, etc.)
   - Extract card type (メンバーカード, etc.)
   - Extract group (『group』)
   - Extract property (コスト, スコア, etc.)
   - Extract operator (以上, 以下, etc.)
   - Extract value (numbers)
   - Extract count (N枚, N人)
   - Extract modifiers (まで, のみ, etc.)
   - Extract position (センターエリア, 左サイドエリア, etc.)

3. **Third pass:** Build semantic structure
   - Combine components into condition objects
   - Combine components into action objects
   - Handle nested structures recursively
   - Apply grammatical rules to determine relationships

4. **Fourth pass:** Validate and normalize
   - Check for missing required fields
   - Apply default values (e.g., draw implies deck→hand)
   - Normalize similar patterns
   - Handle special cases (baton touch, ability activation, etc.)

## Missing Patterns Discovered

From exploratory analysis, these patterns were not in the original rule set:

1. **Position Change (ポジションチェンジ)** - swaps member positions
2. **Position-Specific Conditions** - "左サイドエリアに登場しているなら"
3. **Cost Sum Conditions** - "コストの合計が、10、20、30、40、50のいずれかの場合"
4. **Energy Count Conditions** - "エネルギーが11枚以上ある場合"
5. **Heart Icon Selection** - "heart01かheart03かheart06のうち、1つを選ぶ"
6. **Live Card Count Conditions** - "ライブ中のカードが3枚以上あり、その中にgroupのライブカードを1枚以上含む場合"
7. **Card Reveal Actions** - "手札にあるメンバーカードを好きな枚数公開する"
8. **Cost-Based Ability Activation** - "このカードのコストが10以上になった場合"
9. **Area-Specific Deploy** - position-specific trigger conditions
10. **Deck Refresh** - deck refresh conditions

## Unmatched Abilities

25 abilities don't match common predefined patterns. These need individual analysis to identify their unique structures. Examples include:
- Very short abilities (< 20 chars)
- Very long abilities (> 200 chars)
- Abilities with unusual character combinations
- Abilities with unique icon patterns

## Confidence Assessment

**High confidence patterns** (appearing in 10+ abilities):
- Cost:effect structure (34%)
- Conditional structure (50%)
- Card movement (draw, discard, add to hand)
- Resource gain (blades, hearts)
- Basic conditions (cost limits, counts)

**Medium confidence patterns** (appearing in 3-9 abilities):
- Choice effects (1.5%)
- Compound conditions (3%)
- Sequential with explicit marker (2%)
- Duration modifiers (5-10%)
- Per-unit modifiers (low frequency)

**Low confidence patterns** (appearing in 1-2 abilities):
- Position change
- Cost sum conditions
- Heart icon selection
- Live card count conditions
- Card reveal actions
- Cost-based ability activation

**Unknown patterns:**
- 25 abilities don't match any predefined pattern
- These need manual analysis to identify their structure
