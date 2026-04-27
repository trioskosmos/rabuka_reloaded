# Q062: Combined Name Parsing

## Test Objective
Test that cards with combined names like "A&B" are correctly parsed as having both names "A" and "B".

## Q&A Reference
**Question:** Do cards with names like "A&B" have both names "A" and "B"?
**Answer:** Yes, they have each name.

## Card Selection
A card with a combined name (e.g., LL-bp1-001-R+ "上原歩夢＆澁谷かのん＆日野下花帆").

**Primary Card:** LL-bp1-001-R+

## Initial Game State
N/A - This is a card parsing test, not a gameplay test.

## Expected Action Sequence

**Step 1: Get card names from combined name card**
- Engine function called: `card_database.get_card_names(card_id)`
- Parameters: card_id
- Expected output: Vec containing all individual names

## User Choices
None - deterministic

## Expected Final State
N/A - card parsing only

## Verification Assertions
1. Card has all expected individual names
2. Name count matches expected (3 for "&" card)
3. No compilation errors
4. No runtime panics
