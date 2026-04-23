# Conceptual Model Corrections

## Critical Finding: ウェイト (Wait) is a STATE, not a LOCATION

**From rules.txt section 5.2:**
> カードを'アクティブにする'または'ウェイトにする'指示がある場合、指定されたカードの向きをその指示に応じて、アクティブ状態かウェイト状態にします。

**From rules.txt section 4.3.2.2:**
> ウェイト状態のカードは、そのカードのマスターから見て横向きになるように置きます。

**Correct understanding:**
- ウェイト (wait) = card STATE (horizontal orientation)
- アクティブ (active) = card STATE (vertical orientation)
- These are ORIENTATION states, not locations

**Incorrect assumption:**
- I mapped "ウェイトにする" to `destination: 'wait'`
- This is wrong - wait is a state change, not a destination

**Correct model:**
- "ウェイトにする" = change card state to wait (horizontal)
- "アクティブにする" = change card state to active (vertical)
- These can apply to cards in various locations (stage, energy zone)

## 登場させる (Deploy) is NOT usually attached to baton pass

**Findings from examination:**
- Total abilities with 登場させる: 19
- With バトンタッチ: 1 (5%)
- Without バトンタッチ: 18 (95%)

**User's assumption:** "登場させる I think is usually attached to baton pass? maybe always."
**Actual:** Only 1 out of 19 has baton touch. The assumption is incorrect.

**Sources for 登場させる:**
- 控え室から: 10
- 手札から: 8
- ステージから: 5
- デッキから: 0

## Draw and Discard as Card Movement

**Correct conceptual model:**
All card movements should be modeled as:
```
action: move_cards
source: [location]
destination: [location]
count: [number]
state_change: [optional state change]
```

**Examples:**
- Draw: `source: 'deck', destination: 'hand', count: 1`
- Discard: `source: 'hand', destination: 'discard', count: 1`
- Wait: `action: 'change_state', state: 'wait'` (no location change)
- Deploy: `source: 'discard', destination: 'stage', count: 1`

**Code → Multiple Japanese mappings:**

### DRAW (deck → hand)
- `カードを引く` (3 occurrences)
- `手札に加える` (79 occurrences)
- `カードをN枚引く` (template)

### DISCARD (any → discard zone)
- `控え室に置く` (141 occurrences) - move to discard zone
- `ウェイトにする` (34 occurrences) - change to wait state (NOT discard!)

### STATE CHANGES
- `アクティブにする` (24 occurrences) - change to active state
- `ウェイトにする` (34 occurrences) - change to wait state

### DEPLOY (any → stage)
- `登場させる` (19 occurrences)
- `ステージに置く` (0 occurrences)

## Correct Pattern Mappings

| Japanese | Type | Code |
|----------|------|------|
| 控え室から | SOURCE | source: 'discard' |
| 控え室に置く | DESTINATION | destination: 'discard' |
| ウェイトにする | STATE CHANGE | action: 'change_state', state: 'wait' |
| アクティブにする | STATE CHANGE | action: 'change_state', state: 'active' |
| 手札に加える | DESTINATION | destination: 'hand' |
| カードを引く | ACTION | action: 'draw' (implies deck→hand) |
| 登場させる | DESTINATION | destination: 'stage' |

## Required Parser Changes

1. Separate STATE CHANGE patterns from LOCATION patterns
2. ウェイトにする should parse as state change, not destination
3. Draw should be modeled as card movement (deck→hand)
4. Discard should be modeled as card movement (any→discard zone)
5. Handle state changes that occur WITH location changes (e.g., "ウェイト状態で置く")
