# Rabuka Reloaded - Missing Engine Features

**Status**: 🚧 **INCOMPLETE** - Many abilities will fail or not work correctly  
**Last Updated**: 2026-04-29  
**Total Abilities**: 602 unique abilities affecting 1057 cards  

---

## 🚨 CRITICAL ISSUES (Game-Breaking)

### 1. **Missing `look_and_select` Handler**
- **Impact**: **CRITICAL** - Used in many abilities, completely broken
- **Error**: `"Unknown effect action: look_and_select"`
- **Affected Abilities**: All "look at X cards, choose Y, do Z" abilities
- **Example**: Ability #6 - "Look at 3 cards, choose any number, put on deck, rest to discard"

### 2. **Missing count/dynamic_count in ~130 Actions**
- **Impact**: **HIGH** - Runtime errors or incorrect behavior
- **Problem**: Actions with `count: null` but no `dynamic_count` fallback
- **Pattern**: Mostly in `select_action.actions` within `look_and_select` effects
- **Example**: "残りを控え室に置く" (move remaining cards) has no count field

### 3. **`move_from_looked_at_to_deck_top` Ignores Count**
- **Impact**: **HIGH** - Moves wrong number of cards
- **Problem**: Always moves ALL looked-at cards, ignores count parameter
- **Also**: Doesn't respect `any_number: true` flag for player choice

---

## 📋 COMPLETE MISSING HANDLERS (5 Total)

| Handler | Priority | Description | Usage Count |
|---------|----------|-------------|-------------|
| `look_and_select` | **CRITICAL** | Look at cards, then choose actions | Many |
| `change_state` | Medium | Game state transitions | TBD |
| `choose_required_hearts` | Medium | Heart selection mechanics | TBD |
| `pay_energy` | Medium | Energy cost payment | TBD |
| `play_baton_touch` | Medium | Baton pass mechanics | TBD |

---

## ⚠️ PLACEHOLDER-ONLY HANDLERS (19 Total)

These exist but only log messages - no actual implementation:

| Handler | Usage Count | Current Behavior | Required Implementation |
|---------|-------------|-------------------|------------------------|
| `choice` | 10 abilities | Logs options, no selection | User choice UI + result handling |
| `gain_ability` | 11 abilities | Logs, no ability granted | Temporary ability system |
| `appear` | 6 abilities | Logs, no card appears | Card creation/spawning |
| `modify_cost` | 6 abilities | Logs, cost unchanged | Cost modification system |
| `restriction` | 6 abilities | Logs, no restriction | Game restriction mechanics |
| `select` | 5 abilities | Logs, no selection | Card/player selection |
| `custom` | 3 abilities | Logs, custom logic ignored | Custom effect framework |
| `re_yell` | 2 abilities | Logs, no cheer effect | Cheer/yell mechanics |
| `draw_until_count` | 1 abilities | Logs, no cards drawn | Draw-to-size logic |
| `activation_cost` | 1 abilities | Logs, cost not paid | Special cost handling |
| `set_blade_count` | 1 abilities | Logs, counters unchanged | Blade counter system |
| `set_card_identity` | 1 abilities | Logs, no transformation | Card identity change |
| `set_cost` | 1 abilities | Logs, cost unchanged | Cost setting system |
| `set_score` | 1 abilities | Logs, score unchanged | Score manipulation |
| `activation_restriction` | 0 abilities | Unused | Activation restrictions |
| `invalidate_ability` | 0 abilities | Unused | Ability invalidation |
| `modify_limit` | 0 abilities | Unused | Limit modification |
| `reveal` | 0 abilities | Unused | Card reveal mechanics |
| `specify_heart_color` | 0 abilities | Unused | Heart color selection |

---

## ✅ FULLY IMPLEMENTED (16 Handlers)

These work correctly:
- `move_cards` - Card movement between zones
- `draw`/`draw_card` - Drawing cards
- `gain_resource` - Blade/heart gain
- `modify_score` - Score changes
- `modify_required_hearts` - Heart requirement changes
- `set_required_hearts` - Set heart requirements
- `modify_required_hearts_global` - Global heart changes
- `set_blade_type` - Blade type setting
- `set_heart_type` - Heart type setting
- `position_change` - Stage position changes
- `place_energy_under_member` - Energy placement
- `modify_yell_count` - Cheer count changes
- `look_at` - Look at cards (no movement)
- `sequential` - Multi-action sequences
- `discard_until_count` - Discard to hand size
- `conditional_alternative` - Conditional effects

---

## 📖 RULES.TXT ANALYSIS - CRITICAL GAME SYSTEMS MISSING

### Core Game Mechanics (Not Just Abilities)
| System | Rules Section | Status | Impact |
|--------|---------------|--------|--------|
| **エール (Cheer/Yell) System** | 8.3.11 | ❌ Missing | Core live phase mechanic |
| **Heart Processing** | 8.3.12-8.3.15 | ❌ Missing | Live success determination |
| **Check Timing System** | 9.5 | ❌ Missing | Trigger/ability resolution |
| **Resolution Area** | 8.3.11 | ❌ Missing | Card reveal zone |
| **Selection/Targeting** | 9.6.3 | ❌ Missing | Player choice system |
| **Turn Phase System** | 7.4-7.8 | ⚠️ Partial | Game flow foundation |
| **Victory Conditions** | 8.4 | ❌ Missing | Win/lose logic |

### Critical Missing Mechanics
1. **Blade Counting** - Sum blades from all stage members
2. **Card Reveal** - Move cards to resolution area during cheer
3. **Heart Icon Processing** - Extract/convert heart icons
4. **Wild Heart Handling** - icons as any color
5. **Live Success Validation** - Check heart requirements
6. **Priority System** - Active/non-active player order
7. **Cost Validation** - Check feasibility before payment

---

## ❓ QA_DATA.JSON ANALYSIS - EDGE CASES

### Common Requirements
| Category | Examples | Missing Feature |
|----------|----------|-----------------|
| **Exact Card Matching** | Q237 - No partial name matching | Card name comparison |
| **Group Resolution** | Q235 - 『虹ヶ咲』 to cards | Group name mapping |
| **Deck Validation** | Q234 - Need 3+ cards to pay cost | Pre-action checks |
| **Trigger Stacking** | Q233 - Multiple triggers per turn | State tracking |
| **Target Selection** | Q223 - Who chooses position | Player choice rules |
| **Score Calculation** | Q231 - Base + modifiers | Dynamic scoring |

---

## 🎯 REVISED IMPLEMENTATION PRIORITY

### Phase 0: Foundation Systems (Critical - Week 1)
1. **Resolution Area** - Cards revealed during cheer
2. **Check Timing System** - Trigger handling foundation  
3. **Selection/Choice System** - User interaction foundation
4. **Zone Transfer Validation** - Movement rules enforcement

### Phase 1: Core Mechanics (Week 1-2)
5. **エール (Cheer) System** - Blade counting + card reveal
6. **Heart Processing** - Icon extraction + wildcards
7. **Cost Validation** - Pre-action feasibility checking
8. **Card Name/Group Resolution** - Accurate targeting

### Phase 2: Ability Handlers (Week 2)
9. **`look_and_select` handler** - Many abilities broken
10. **Fix missing count/dynamic_count** - ~130 actions
11. **`choice` handler** - 10 abilities need selection
12. **`gain_ability` handler** - 11 abilities need granting

### Phase 3: Game Flow (Week 2-3)
13. **Turn Phase System** - Complete flow implementation
14. **Live Success Logic** - Heart requirement validation
15. **Victory Conditions** - Win/lose determination
16. **Baton Touch Mechanics** - Cost reduction system

### Phase 4: Complete Support (Week 3+)
17. **All remaining ability handlers** - Full compatibility
18. **QA Edge Cases** - All special situations
19. **Advanced Mechanics** - Complex interactions

---

## 🔧 TECHNICAL DETAILS

### Engine Files to Modify
- `engine/src/ability/executor.rs` - Main ability execution
- `engine/src/card.rs` - AbilityEffect struct (may need new fields)
- `engine/src/game_state.rs` - State tracking for new mechanics

### Key Functions to Add/Modify
```rust
// Missing completely
fn execute_look_and_select(...)
fn execute_change_state(...)
fn execute_choose_required_hearts(...)
fn execute_pay_energy(...)
fn execute_play_baton_touch(...)

// Need full implementation
fn execute_choice(...)           // Currently just logs
fn execute_gain_ability(...)    // Currently just logs
fn execute_appear(...)          // Currently just logs
// ... etc for all placeholders
```

### Data Structure Needs
- `any_number` flag handling in move_cards
- `dynamic_count` support for variable counts
- Choice/result tracking system
- Temporary ability system
- Card appearance/spawning system

---

## 📊 REVISED IMPACT SUMMARY

| Category | Before | After Foundation | After Full |
|----------|--------|-------------------|------------|
| Working Abilities | ~300 | ~350 | ~580 |
| Broken Abilities | ~200 | ~150 | ~20 |
| Game Mechanics | 30% | 50% | 95% |
| Rule Compliance | 40% | 60% | 98% |
| Core Systems | 0% | 70% | 100% |

**Key Insight**: Rules analysis reveals the game needs foundational systems (timing, zones, validation) before abilities can work properly.

**Estimated Work**: 3-4 weeks for full implementation
**Minimum Viable**: 2 weeks for foundation systems

---

## 🚀 GETTING TO A WORKING GAME (REVISED)

### Foundation Systems (Week 1-2)
1. Resolution area + check timing
2. Cheer system + heart processing
3. Selection/choice system
4. Cost validation

### Core Game Mechanics (Week 2-3)
1. Turn phase system
2. Live success logic
3. Victory conditions
4. Critical ability handlers

### Full Implementation (Week 3-4)
1. All ability handlers
2. QA edge cases
3. Advanced mechanics
4. Complete rule compliance

### Result
- **Before**: Game crashes on most abilities, no core mechanics
- **After Foundation**: Basic playable game with core systems
- **After Full**: Fully compliant with official rules, 95%+ abilities work

---

## 📝 NOTES

- Many abilities use `sequential` actions, so fixing individual handlers enables complex multi-step abilities
- The `any_number` flag is important for "choose any number" effects
- `dynamic_count` is needed for "remaining cards" and "player choice" effects
- Some abilities might need UI integration for user choices
- Test with actual card gameplay to validate implementations
- **Rules.txt reveals the game needs core systems before abilities can work**
- **QA data shows edge cases that proper implementations must handle**
- **Foundation systems (timing, zones, validation) are more critical than individual abilities**
- **The engine needs game state tracking beyond just cards**

---

## 📋 RELATED DOCUMENTS

- `RULES_ANALYSIS.md` - Detailed breakdown of rules.txt requirements
- `scan_report.txt` - Technical scan results of abilities.json
- `abilities.json` - Parsed ability data (602 unique abilities)
- `rules.txt` - Official game rules (Japanese)
- `qa_data.json` - Official Q&A with edge cases

---

*This document will be updated as features are implemented.*
