# Q006: Different Card Numbers

## Test Objective
Test that cards with the same name/ability but different card numbers can each have 4 copies in the main deck.

## Q&A Reference
**Question:** Can cards with same name/ability but different card numbers be 4 each in main deck?
**Answer:** Yes, if card numbers differ, can use 4 of each.

## Card Selection
Two different member cards with different card numbers.

**Primary Cards:** Any two member cards with different card numbers

## Initial Game State
N/A - This is a deck validation test, not a gameplay test.

## Expected Action Sequence

**Step 1: Validate deck with 4 copies of each different card number**
- Engine function called: `DeckBuilder::validate_deck`
- Parameters: card_database, main_deck (4 of card1 + 4 of card2 + 52 others), energy_deck
- Expected output: validation.is_valid = true, no errors

## User Choices
None - deterministic validation

## Expected Final State
N/A - deck validation only

## Expected Engine Faults
None - this is a validation test

## Verification Assertions
1. Deck with 4 of each different card number passes validation
2. No compilation errors
3. No runtime panics
