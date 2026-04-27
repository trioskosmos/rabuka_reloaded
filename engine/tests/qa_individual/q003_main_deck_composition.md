# Q003: Main Deck Composition

## Test Objective
Test that main deck must have specific composition: 48 member cards + 12 live cards = 60 total (half deck: 24 member + 6 live = 30 total). Validates deck composition rules.

## Q&A Reference
**Question:** Can member and live cards be combined in any ratio for main deck?
**Answer:** No, must be specific counts. 48 member, 12 live, total 60 (half deck: 24 member, 6 live, total 30).

## Card Selection
Use specific cards from the card database for validation.

**Primary Cards:** 
- Member cards: PL!N-bp1-001-R (星空 凛), PL!N-bp1-002-R (高海千歌), PL!N-bp1-003-R (桜内梨子), etc.
- Live cards: PL!N-bp1-011-R (START:True colors), PL!N-bp1-012-R (DreamLand), etc.

**Specific Card IDs for Testing:**
- Use 48 unique member card IDs from available cards
- Use 12 unique live card IDs from available cards
- Example member IDs: PL!N-bp1-001-R, PL!N-bp1-002-R, PL!N-bp1-003-R, PL!N-bp1-004-R, PL!N-bp1-005-R
- Example live IDs: PL!N-bp1-011-R, PL!N-bp1-012-R, PL!N-bp1-013-R, PL!N-bp1-014-R, PL!N-bp1-015-R

## Initial Game State
N/A - This is a deck validation test, not a gameplay test.

## Expected Action Sequence

**Step 1: Validate full deck (48 member + 12 live = 60)**
- Engine function called: `DeckBuilder::validate_deck`
- Parameters: card_database, main_deck (48 member + 12 live), energy_deck
- Expected output: validation.is_valid = true, no errors

**Step 2: Validate invalid deck (wrong composition)**
- Engine function called: `DeckBuilder::validate_deck`
- Parameters: card_database, main_deck (50 member + 10 live), energy_deck
- Expected output: validation.is_valid = false, errors about incorrect composition

**Step 3: Validate half deck (24 member + 6 live = 30)**
- Engine function called: `DeckBuilder::validate_deck`
- Parameters: card_database, main_deck (24 member + 6 live), energy_deck
- Expected output: validation.is_valid = true, no errors

## User Choices
None - deterministic validation

## Expected Final State
N/A - deck validation only

## Expected Engine Faults
None - this is a validation test

## Verification Assertions
1. Valid full deck (48+12) passes validation
2. Invalid deck (50+10) fails validation
3. Valid half deck (24+6) passes validation
4. No compilation errors
5. No runtime panics
