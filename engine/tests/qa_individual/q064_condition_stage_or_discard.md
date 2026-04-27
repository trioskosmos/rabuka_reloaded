# Q064: Condition Stage or Discard

## Test Objective
Test that a live start ability condition "5+ different named Liella! members in stage AND discard zone" is satisfied by having 5+ different named members in discard zone alone (even if none on stage).

## Q&A Reference
**Question:** If you have 5+ different named Liella! members in discard zone (none on stage), is condition met?
**Answer:** Yes, condition is met.

## Card Selection
A live card with the ability (PL!SP-bp1-026-L "夢咲きSUNNY"). 5+ different named Liella! member cards.

**Primary Card:** PL!SP-bp1-026-L
**Supporting Cards:** 5+ different named Liella! members

## Initial Game State
N/A - This is a condition verification test, not a full gameplay test.

## Expected Action Sequence

**Step 1: Find Liella! member cards**
- Engine function called: Filter cards by group == "Liella!" and is_member()
- Expected output: List of Liella! member cards

**Step 2: Verify unique names**
- Engine function called: Extract unique names from member cards
- Expected output: 5+ unique names

**Step 3: Verify condition is satisfied**
- Engine function called: Condition evaluation for live start ability
- Expected output: Condition is true (5+ different named members in discard zone satisfies "stage AND discard zone")

## User Choices
None - deterministic

## Expected Final State
N/A - condition verification only

## Expected Engine Faults
None - this is a condition verification test

## Verification Assertions
1. At least 5 Liella! members found
2. At least 5 different names among those members
3. Condition "5+ different named Liella! members in stage AND discard zone" is satisfied by discard zone alone
4. No compilation errors
5. No runtime panics
