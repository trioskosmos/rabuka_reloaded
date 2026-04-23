# Ability Patterns Audit

## Overview
- Total abilities: 609
- Max length: 322 characters
- Min length: 0 characters
- Avg length: 85.9 characters
- Long abilities (>150 chars): 47

## Punctuation Patterns

### High-frequency punctuation
- **、** (comma): 965 occurrences - Separates clauses, list items, sequential actions
- **。** (period): 948 occurrences - Sentence endings
- **『』** (single quotes): 226/225 occurrences - Group/unit names (e.g., 『Liella!』, 『Aqours』)
- **：** (colon): 206 occurrences - Separates cost from effect
- **「」** (double quotes): 78/77 occurrences - Character names, ability names
- **（）** (parentheses): 24/24 occurrences - Notes, clarifications, exceptions

### Punctuation usage patterns
1. **Cost delimiter**: `：` separates cost from effect (206 occurrences)
2. **Sentence structure**: `。` ends sentences (948 occurrences)
3. **Clause separation**: `、` separates:
   - Sequential actions: "カードを2枚引き、手札を1枚控え室に置く"
   - List items: "ハート01、ハート02、ハート03のうち"
   - Conditions: "場合、とき、かぎり" clauses
4. **Group markers**: `『』` for unit/group names
5. **Character markers**: `「」` for specific character names
6. **Notes**: `（）` for parenthetical notes and exceptions

## Trigger Types

### Single triggers (most common)
- ライブ開始時: 176
- 登場: 175
- 起動: 68
- ライブ成功時: 65
- 常時: 65
- 自動: 40

### Combined triggers (rare)
- ライブ開始時, 登場: 6
- Position-specific triggers (left_side, right_side): 9 total

## Cost Patterns

### Cost types
- move_cards: 126 (card movement costs)
- pay_energy: 54 (energy payment costs)
- none: 25 (no cost)

### Cost variations
1. **Energy costs**: `{{icon_energy.png|E}}` icons (1-3 energy typical)
2. **Discard costs**: "手札を1枚控え室に置く"
3. **Optional costs**: "～てもよい" (may do X)
4. **Self-discard**: "このメンバーをステージから控え室に置く"
5. **Member to wait**: "ウェイトにする"
6. **Combined costs**: Multiple cost types in sequence

## Effect Action Types

### Primary actions
- move_cards: 277 (card movement)
- gain_resource: 151 (resource gain)
- activate_energy: 24 (energy activation)

### Move cards destinations
- hand (add to hand, draw)
- discard (discard, wait)
- stage (deploy)
- deck_top/deck_bottom (deck manipulation)
- member_under (energy under member)

### Resource types
- heart (heart icons)
- blade (blade icons)
- score (score modification)
- energy (energy activation)

## Keyword Analysis

### Condition keywords
- 場合: 259 (conditional "if/when")
- とき: 43 (timing "at the time of")
- かぎり: 33 (duration "as long as")
- 好きな: 26 (choice "favorite/any")
- かつ: 20 (compound "and")
- その後: 15 (sequential "after that")
- 以下から1つを選ぶ: 9 (choice "choose one from below")
- また: 2 (alternative "also/or")
- または: 2 (alternative "or")

### Resource mentions
- ブレード: 129 (blades)
- スコア: 108 (score)
- ハート: 100 (hearts)
- コスト: 89 (cost)
- エネルギー: 75 (energy)

### Card movement keywords
- 控え室に置く: 141 (place in discard)
- 手札に加える: 79 (add to hand)
- 引く: 65 (draw)
- ウェイトにする: 34 (to wait)
- 登場させる: 19 (deploy)
- デッキの上に置く: 4 (place on top of deck)

## Complex Patterns

### Structural patterns
- newline: 33 (multiline abilities)
- conditional: 259 (if/when conditions)
- timing: 43 (at the time of)
- compound: 20 (and conditions)
- sequential: 15 (after that)
- choice: 9 (choose one)
- alternative: 2 (or)

## Long Ability Structures (>150 chars)

### Common patterns in long abilities
1. **Complex conditions**: Multiple conditions with "かつ" (and)
2. **Heart type selections**: Multiple heart icons with "のうち" (among)
3. **Cost modifications**: "コストX以下" with group conditions
4. **Multi-part effects**: Sequential actions with "その後"
5. **Choice effects**: "以下から1つを選ぶ" with bullet points
6. **Parenthetical notes**: Exceptions and clarifications in （）

### Example long ability patterns
```
自分がエールしたとき、エールにより公開された自分のカードが持つブレードハートの中に
{{heart_01.png|heart01}}、{{heart_02.png|heart02}}、{{heart_03.png|heart03}}のいずれかがある場合、
ライブ終了時まで、自分のステージにいるメンバー1人は{{icon_blade.png|ブレード}}を得る。
```

## Game Effect/Resource Manipulation Patterns

### Resource gain
1. **Heart gain**: `{{heart_xx.png|heartxx}}を得る`
2. **Blade gain**: `{{icon_blade.png|ブレード}}を得る`
3. **Score modification**: `スコアを＋Xする`
4. **Energy activation**: `エネルギーをアクティブにする`

### Resource cost
1. **Heart cost**: `{{heart_xx.png|heartxx}}を支払う`
2. **Energy cost**: `{{icon_energy.png|E}}` icons
3. **Card cost**: Discard specific cards

### Score manipulation
1. **Add score**: `スコアを＋Xする`
2. **Reduce score**: `スコアを－Xする`
3. **Score comparison**: "スコアが相手より高い/低い"

### Cost manipulation
1. **Cost reduction**: `コストを－Xする`
2. **Cost increase**: `コストを＋Xする`
3. **Cost setting**: `コストはXになる`

## Condition Patterns

### Member conditions
1. **Count conditions**: "メンバーX人以上/以下"
2. **Cost conditions**: "コストX以上/以下のメンバー"
3. **Group conditions**: "『Group』のメンバー"
4. **State conditions**: "ウェイト状態の/アクティブ状態の"
5. **Position conditions**: "センターエリア/左サイド/右サイド"
6. **Presence conditions**: "いる/いない"

### Card conditions
1. **Count conditions**: "カードX枚以上/以下"
2. **Type conditions**: "メンバーカード/ライブカード/エネルギーカード"
3. **Location conditions**: "手札/控え室/ステージ/デッキ"
4. **Cost conditions**: "コストX以上/以下"

### Resource conditions
1. **Heart conditions**: "ハートXつ以上/以下"
2. **Blade conditions**: "ブレードXつ以上/以下"
3. **Energy conditions**: "エネルギーX枚以上/以下"
4. **Score conditions**: "スコアX以上/以下"

### Trigger conditions
1. **Movement triggers**: "エリアを移動した"
2. **Deploy triggers**: "登場した"
3. **Discard triggers**: "ステージから控え室に置かれた"
4. **Cheer triggers**: "エールにより公開された"

## What is Possible (Based on Data)

### Multi-action abilities
- Sequential actions with "その後" (after that)
- Compound conditions with "かつ" (and)
- Choice effects with "以下から1つを選ぶ" (choose one)
- Alternative actions with "または" (or)

### Complex targeting
- Opponent targeting: "相手のステージにいる"
- Self targeting: "自分のステージにいる"
- Both players: "自分と相手の"
- Specific characters: "「Character Name」"

### Variable extraction
- Counts: "X人/X枚"
- Cost limits: "コストX以上/以下"
- Heart types: Multiple heart icons
- Groups: "『Group Name』"
- Positions: "センターエリア/左サイド/右サイド"

### State manipulation
- Wait state: "ウェイトにする"
- Active state: "アクティブにする"
- Deploy: "登場させる"
- Discard: "控え室に置く"

### Duration modifiers
- "ライブ終了時まで" (until end of live)
- "このターンの間" (during this turn)
- "かぎり" (as long as)

### Optional actions
- "～てもよい" (may do X)
- "～ことができる" (can do X)
- "～しなくてもよい" (may not do X)

### Per-unit modifiers
- "X人につき" (per X people)
- "X枚につき" (per X cards)
- Multiplier effects based on count

### Special patterns
1. **Baton touch**: "バトンタッチして登場"
2. **Deck refresh**: "デッキがリフレッシュしていた"
3. **Unique name**: "名前が異なる"
4. **Unique cost**: "コストが異なる"
5. **Highest cost**: "最も大きいコストを持つ"
6. **Center position**: "センターエリアにいる"

## Missing/Underrepresented Patterns

### Patterns that may exist but are rare
- Per-unit effects with complex conditions
- Multi-card selection with exclusions
- Nested conditions (conditions within conditions)
- State-based triggers beyond movement/deploy
- Resource transfer between players
- Temporary effect stacking
- Conditional cost payment
- Counter effects
- Zone-specific restrictions

### Potential unhandled cases
1. **Multi-line choice effects** with complex options
2. **Nested parenthetical notes** with multiple exceptions
3. **Compound triggers** with position requirements
4. **Sequential effects** with intermediate conditions
5. **Variable counts** based on game state
6. **Dynamic cost modification** based on conditions
7. **Resource conversion** (e.g., hearts to blades)
8. **Zone-specific abilities** (e.g., only in certain positions)
