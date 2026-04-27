# Q004: Main Deck Duplicates

## Test Objective
Test that cards with the same card number are considered the same card, with a maximum of 4 copies allowed in the main deck. Card number excludes rarity symbol.

## Q&A Reference
**Question:** How many same cards can be used in main deck?
**Answer:** Cards with same card number are same card, max 4 of same card. Card number excludes rarity symbol.

## Card Selection
A member card with multiple rarities available.

**Primary Card:** PL!N-bp1-001-R (星空 凛)
- Card ID: PL!N-bp1-001-R
- Card Name: 星空 凛
- Rarity: R
- Alternative rarities: PL!N-bp1-001-SR, PL!N-bp1-001-SSR
- Why this card: Has multiple rarities to test same card number rule

## Initial Game State
N/A - This is a deck validation test, not a gameplay test.

## Expected Action Sequence

**Step 1: Validate deck with 4 copies of same card number**
- Engine function called: `DeckBuilder::validate_deck`
- Parameters: card_database, main_deck (4 copies of same card + 56 others), energy_deck
- Expected output: validation.is_valid = true, no errors

**Step 2: Validate deck with 5 copies of same card number**
- Engine function called: `DeckBuilder::validate_deck`
- Parameters: card_database, main_deck (5 copies of same card + 55 others), energy_deck
- Expected output: validation.is_valid = false, error about maximum 4 copies

## User Choices
None - deterministic validation

## Expected Final State
N/A - deck validation only

## Expected Engine Faults
None - this is a validation test

## Verification Assertions
1. Deck with 4 copies of same card number passes validation
2. Deck with 5 copies of same card number fails validation
3. Error message mentions "maximum is 4"
4. No compilation errors
5. No runtime panics
