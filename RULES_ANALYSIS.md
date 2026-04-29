# Rules.txt and QA Data Analysis - Missing Engine Features

**Status**: 📋 **ANALYSIS COMPLETE** - Additional requirements identified  
**Last Updated**: 2026-04-29  

---

## 📖 RULES.TXT KEY FINDINGS

### Game Flow Requirements
| Section | Feature | Current Status | Implementation Need |
|---------|---------|----------------|---------------------|
| 7.4-7.8 | Turn phases (Active, Energy, Draw, Main, Live) | ⚠️ Partial | Complete phase system |
| 8.3.11 | **エール (Cheer/Yell)** - Reveal cards from deck | ❌ Missing | Cheer/yell mechanics |
| 8.3.12 | Draw cards based on ブレードハート icons | ❌ Missing | Heart icon processing |
| 8.3.14 | **ライブ所有ハート** (Live Owned Hearts) calculation | ❌ Missing | Heart aggregation system |
| 8.3.15 | **必要ハート** (Required Hearts) validation | ❌ Missing | Heart requirement checking |
| 8.4 | Live victory/defeat determination | ❌ Missing | Win condition logic |
| 9.5 | **チェックタイミング** (Check Timing) system | ❌ Missing | Timing/trigger system |
| 9.6.3 | **選択 (Selection)** mechanics with targeting | ❌ Missing | Target selection system |

### Critical Missing Mechanics

#### 1. **エール (Cheer/Yell) System** (8.3.11)
```
"手番プレイヤーは、自身のメインデッキの一番上のカードを解決領域に移動する処理を、
前述の合計ブレード数と同じ回数繰り返します。この処理を'エール'と呼びます。"
```
- **What**: Reveal top cards equal to total blade count
- **Where**: Move to `解決領域` (Resolution Area)
- **Missing**: Resolution area, blade counting, cheer execution

#### 2. **Heart Processing System** (8.3.12-8.3.15)
```
"アイコン 1 つにつき、手番プレイヤーはカードを 1 枚引きます。"
"各 アイコンは、任意の色のハートアイコン 1 つとして扱うことができます。"
```
- **What**: Process heart icons from cheered cards
- **Missing**: Icon extraction, heart color handling, wildcards

#### 3. **Check Timing System** (9.5)
```
"チェックタイミングとは、ゲーム中で発生したルール処理や自動能力の
プレイを行う時点を指します。"
```
- **What**: Priority system for triggered abilities
- **Missing**: Trigger queue, priority resolution, state stack

#### 4. **Selection/Targeting System** (9.6.3)
```
"カードや能力に'～選び'や'～選ぶ'と書かれている場合、解決の際に、
その指示があった段階でそこで示された選ぶべきカードやプレイヤー等を選択します。"
```
- **What**: Player choice mechanics for targeting
- **Missing**: Choice prompts, validation, targeting logic

---

## ❓ QA_DATA.JSON KEY FINDINGS

### Common Question Categories
| Category | Count | Missing Engine Feature |
|----------|-------|------------------------|
| Heart calculations | ~30% | Heart processing system |
| Cost payment | ~20% | Cost validation system |
| Card movement | ~15% | Zone transfer logic |
| Timing/trigger | ~15% | Check timing system |
| Selection/target | ~10% | Choice system |
| Win conditions | ~10% | Victory logic |

### Specific Examples Requiring Implementation

#### Q237: Card Name Matching
```json
"能力でPL!HS-sd1-018-SD「Dream Believers（104期Ver.）」を公開しました。
その場合、控え室からPL!HS-bp1-019-L「Dream Believers」を手札に加えることはできますか？"
```
**Answer**: "いいえ、できません。"
- **Requirement**: Exact card name matching (not partial)
- **Missing**: Card name comparison logic

#### Q235: Group Name Resolution  
```json
"LL-bp1-001-R+「上原歩夢＆澁谷かのん＆日野下花帆」とPL!SP-bp1-001-R「澁谷かのん」と
PL!HS-bp1-001-R「日野下花帆」をそれぞれ手札に加えられますか？"
```
**Answer**: "はい、LL-bp1-001-R+を『虹ヶ咲』のカードとして選ぶことで可能です。"
- **Requirement**: Group name (『虹ヶ咲') resolution
- **Missing**: Group name to card mapping

#### Q234: Deck Validation
```json
"自分のデッキが2枚しかない状態でこの能力のコストを支払えますか？"
```
**Answer**: "いいえ、できません。デッキが3枚以上必ず必要です。"
- **Requirement**: Pre-action validation
- **Missing**: Cost feasibility checking

#### Q233: Trigger Stacking
```json
"カードが控え室に置かれ、この自動能力が発動しましたが、を支払いませんでした。
その場合、そのターン中にまたカードが控え室に置かれたとき、この能力は発動しますか？"
```
**Answer**: "はい、発動します。"
- **Requirement**: Multiple trigger handling
- **Missing**: Trigger state tracking

---

## 🚨 NEW CRITICAL MISSING FEATURES

### From Rules Analysis
1. **Resolution Area Management** - Cards revealed during cheer
2. **Blade Counting System** - Total blades from all members  
3. **Heart Icon Processing** - Extract and process heart icons
4. **Wild Heart Handling** - icons as any color
5. **Live Success Determination** - Heart requirement validation
6. **Victory Condition Logic** - 3+ cards vs 2- cards
7. **Phase Management** - Complete turn phase system
8. **Check Timing Implementation** - Priority-based trigger resolution

### From QA Analysis
1. **Exact Card Name Matching** - No partial matching
2. **Group Name Resolution** - 『グループ名』 to cards
3. **Cost Validation** - Check feasibility before payment
4. **Trigger State Tracking** - Multiple instances per turn
5. **Position Change Targeting** - Who chooses movement
6. **Deck Bottom Placement** - When deck is too small
7. **Baton Touch Cost Reduction** - Member discard for cost
8. **Score Calculation** - Base score + modifiers

---

## 🔧 UPDATED IMPLEMENTATION PRIORITY

### Phase 0: Foundation (Critical - Week 1)
1. **Resolution Area** - Core game mechanic
2. **Check Timing System** - Trigger handling foundation
3. **Selection/Choice System** - User interaction foundation
4. **Zone Transfer Validation** - Movement rules

### Phase 1: Core Mechanics (Week 1-2)
5. **エール (Cheer) System** - Blade counting + card reveal
6. **Heart Processing** - Icon extraction + wildcards
7. **Cost Validation** - Pre-action checking
8. **Card Name/Group Resolution** - Targeting accuracy

### Phase 2: Game Flow (Week 2-3)
9. **Turn Phase System** - Complete flow implementation
10. **Live Success Logic** - Heart requirement checking
11. **Victory Conditions** - Win/lose determination
12. **Baton Touch Mechanics** - Cost reduction system

### Phase 3: Advanced Features (Week 3+)
13. **Trigger Stacking** - Complex ability interactions
14. **Position Change** - Target selection rules
15. **Score Modifiers** - Dynamic score calculation
16. **All Placeholder Handlers** - Complete ability support

---

## 📊 UPDATED IMPACT ASSESSMENT

| Category | Before | After Foundation | After Full |
|----------|--------|-------------------|------------|
| Working Abilities | ~300 | ~350 | ~580 |
| Broken Abilities | ~200 | ~150 | ~20 |
| Game Mechanics | 30% | 50% | 95% |
| Rule Compliance | 40% | 60% | 98% |

**Key Insight**: Rules analysis reveals ~50% more missing features than just ability handlers. The game needs core systems (timing, zones, validation) before abilities can work properly.

---

## 🎯 GETTING TO A WORKING GAME (REVISED)

### Minimum Viable Product (2 weeks)
1. Resolution area + cheer system
2. Basic check timing  
3. Heart processing
4. Cost validation
5. Core ability handlers (look_and_select, choice, etc.)

### Full Implementation (3-4 weeks)
1. Complete turn phase system
2. All mechanics from rules.txt
3. QA compliance for all edge cases
4. Full ability support (580+ working)

### Result
- **Before**: Game crashes on most abilities
- **After MVP**: Basic playable game with core mechanics
- **After Full**: Fully compliant with official rules

---

## 📝 NOTES

- Many "abilities" are actually rule mechanics (cheer, timing, validation)
- Rules.txt reveals the game is more complex than just ability execution
- QA data shows edge cases that proper implementations must handle
- Some abilities require UI integration (choices, targeting)
- The engine needs game state tracking beyond just cards

---

*This analysis shows the engine needs foundational game systems before abilities can work correctly.*
