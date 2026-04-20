# Pattern Validation Findings

## Critical Issue: Source vs Destination Confusion

**The problem:** My SOURCE_PATTERNS incorrectly mapped "自分の控え室から" to just "discard" without distinguishing it as a SOURCE.

**Validation results:**
- `控え室から` (FROM discard): 75 occurrences - this is a SOURCE
- `控え室に置く` (TO discard): 141 occurrences - this is a DESTINATION
- `ウェイトにする` (TO wait): 34 occurrences - this is a DESTINATION (wait state)

**Correct mappings:**
| Japanese | Direction | Code |
|----------|-----------|------|
| 控え室から | SOURCE | source: 'discard' |
| 控え室に置く | DESTINATION | destination: 'discard' |
| ウェイトにする | DESTINATION | destination: 'wait' |
| 手札に加える | DESTINATION | destination: 'hand' |
| カードを引く | ACTION | action: 'draw' (implies deck→hand) |
| 登場させる | DESTINATION | destination: 'stage' |

## Code → Multiple Japanese Mappings

**One code concept maps to multiple Japanese expressions:**

### DRAW (deck → hand)
- `カードを引く` (3 occurrences)
- `手札に加える` (79 occurrences)
- `カードをN枚引く` (0 - template)

### DISCARD (any → discard)
- `控え室に置く` (141 occurrences)
- `ウェイトにする` (34 occurrences) - wait state variant

### DEPLOY (any → stage)
- `登場させる` (19 occurrences)
- `ステージに置く` (0 occurrences)

### ACTIVATE ENERGY
- `アクティブにする` (24 occurrences)

## Source Patterns (FROM)

| Japanese | Code | Occurrences |
|----------|------|-------------|
| デッキから | source: 'deck' | 23 |
| デッキの上から | source: 'deck_top' | 78 |
| 手札から | source: 'hand' | 14 |
| ステージから | source: 'stage' | 22 |
| 控え室から | source: 'discard' | 75 |
| エネルギーデッキから | source: 'energy_deck' | 23 |

## Destination Patterns (TO)

| Japanese | Code | Occurrences |
|----------|------|-------------|
| 手札に加える | destination: 'hand' | 79 |
| 手札に | destination: 'hand' | 138 |
| 控え室に置く | destination: 'discard' | 141 |
| ウェイトにする | destination: 'wait' | 34 |
| 登場させる | destination: 'stage' | 19 |
| デッキの上に置く | destination: 'deck_top' | 4 |

## Contradictions (Expected)

28 abilities have both FROM discard and TO discard. This is correct:
- Example: "ステージから控え室に置く：自分の控え室からライブカードを1枚手札に加える"
- Cost: stage → discard
- Effect: discard → hand

## Key Insight

The user's point is correct: I must consider:
1. **Japanese → code mapping** (what does this Japanese phrase mean?)
2. **Code → multiple Japanese mapping** (what are all the ways to express this code concept?)

For example:
- Code concept: "source: discard"
- Japanese expressions: "控え室から", "控え室にある", "自分の控え室から", "相手の控え室から"

- Code concept: "destination: discard"
- Japanese expressions: "控え室に置く", "ウェイトにする" (wait state variant)

## Required Fix

The parser must:
1. Distinguish SOURCE patterns (FROM) from DESTINATION patterns (TO)
2. Handle multiple Japanese expressions for the same code concept
3. Not assume "控え室" always means the same thing - depends on context (from vs to)
4. Parse direction (from vs to) before mapping to code
