# Q005: Same Card Number Different Rarity

## Test Objective
Test that cards with the same card number but different rarity are limited to 4 total copies in the main deck, not 4 of each rarity.

## Q&A Reference
**Question:** Can cards with same card number but different rarity be 4 each in main deck?
**Answer:** No, same card number means max 4 total regardless of rarity.

## Card Selection
Cards with the same base card number but different rarities (e.g., PL!N-bp1-001-R and PL!N-bp1-001-P).

**Primary Cards:** Cards with same base number, different rarities

## Initial Game State
N/A - This is a deck validation test, not a gameplay test.

## Expected Action Sequence

**Step 1: Validate invalid deck (2 R + 3 P = 5 total of same card number)**
- Engine function called: `DeckBuilder::validate_deck`
- Parameters: card_database, main_deck (2 R + 3 P + 55 others), energy_deck
- Expected output: validation.is_valid = false

**Step 2: Validate valid deck (2 R + 2 P = 4 total of same card number)**
- Engine function called: `DeckBuilder::validate_deck`
- Parameters: card_database, main_deck (2 R + 2 P + 56 others), energy_deck
- Expected output: validation.is_valid = true

## User Choices
None - deterministic validation

## Expected Final State
N/A - deck validation only

## Expected Engine Faults
None - this is a validation test

## Verification Assertions
1. Deck with 5 total of same card number (across rarities) fails validation
2. Deck with 4 total of same card number (across rarities) passes validation
3. No compilation errors
4. No runtime panics
