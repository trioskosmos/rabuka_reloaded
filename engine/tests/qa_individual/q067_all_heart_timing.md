# Q067: All Heart Timing

## Test Objective
Test that ALL heart (heart00) is only treated as any color during need heart verification, not during live start ability condition checks.

## Q&A Reference
**Question:** Can ALL heart be treated as any color for this ability?
**Answer:** No, ALL heart is only treated as any color during need heart verification, not during live start abilities.

## Card Selection
A live card with specific heart color requirements (PL!N-bp1-027-L "Solitude Rain").

**Primary Card:** PL!N-bp1-027-L

## Initial Game State
N/A - This is a timing verification test, not a gameplay test.

## Expected Action Sequence

**Step 1: Verify live card**
- Engine function called: `card_database.get_card(card_id)`
- Expected output: card.is_live() = true

**Step 2: Verify ALL heart timing**
- Engine function called: Heart color verification
- Expected output: ALL heart (heart00) does NOT satisfy specific color requirements in live start abilities

## User Choices
None - deterministic

## Expected Final State
N/A - timing verification only

## Verification Assertions
1. Card is a live card
2. ALL heart (heart00) is only treated as any color during need heart verification
3. ALL heart does NOT satisfy specific color requirements in live start abilities
4. No compilation errors
5. No runtime panics
