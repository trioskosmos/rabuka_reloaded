# Q007: Energy Deck Duplicates

## Test Objective
Test that energy deck can have any number of the same card (no 4-copy limit).

## Q&A Reference
**Question:** How many same cards can be used in energy deck?
**Answer:** Any number of same cards (can use 12 of same card).

## Card Selection
Any energy card.

**Primary Card:** PL!-EN-001 (Energy Card)
- Card ID: PL!-EN-001
- Card Name: Energy Card
- Why this card: Standard energy card for testing duplicate rules

## Initial Game State
N/A - This is a deck validation test, not a gameplay test.

## Expected Action Sequence

**Step 1: Validate energy deck with 12 copies of same card**
- Engine function called: `DeckBuilder::validate_deck`
- Parameters: card_database, main_deck (valid composition), energy_deck (12 of same card)
- Expected output: validation.is_valid = true, no errors

## User Choices
None - deterministic validation

## Expected Final State
N/A - deck validation only

## Verification Assertions
1. Energy deck with 12 of same card passes validation
2. No compilation errors
3. No runtime panics
