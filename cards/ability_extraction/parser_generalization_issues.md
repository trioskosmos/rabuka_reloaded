# Parser Generalization Issues

## Issues Found

### 1. Magic Numbers for Character Name Length ✅ FIXED
**Location**: Lines 220, 1186
**Problem**: Hardcoded length check `<= 10` for character names is hyperspecific.
**Fix**: Defined as constant `MAX_CHARACTER_NAME_LENGTH = 10` at module level.

### 2. Hardcoded Array Indices ✅ FIXED
**Location**: Multiple locations
**Problem**: Direct array indexing assumes arrays always have at least 2 elements.
**Fix**: Added bounds checking before accessing indices (e.g., `if len(parts) > 0`).

### 3. Hardcoded Split Limits ✅ FIXED
**Location**: Lines 309, 1356, 1548, 1593, 1614, 1654, 1666, 1693, 1867, 1883, 1938
**Problem**: Split limit of 1 is hardcoded throughout.
**Fix**: Defined constant `SPLIT_LIMIT = 1` and used consistently.

### 4. Hardcoded Pattern Strings
**Location**: Lines 102-110 (structural markers)
**Problem**: Some inline strings are not constants.
**Impact**: Inconsistency makes code harder to maintain.
**Fix**: Extract all hardcoded Japanese strings to constants at module level.

### 5. Hyperspecific Character Name Filtering
**Location**: Lines 217-220
**Problem**: Assumes ability names always contain `{{` and character names don't.
**Impact**: Will fail for edge cases.
**Fix**: Use more robust filtering based on actual data patterns or configurable rules.

### 6. Hardcoded Condition Markers ✅ FIXED
**Location**: Line 99
**Problem**: List contains duplicates ('場合' and '場合、').
**Fix**: Removed duplicates: `CONDITION_MARKERS = ['場合、', 'とき、', 'なら、']`.

### 7. Missing Error Handling
**Location**: Throughout the parser
**Problem**: Many operations assume success (e.g., regex matches, array access).
**Impact**: Will crash on unexpected input formats.
**Fix**: Add try-except blocks and validation checks.

### 8. Hardcoded Pattern Arrays
**Location**: Lines 12-96 (pattern arrays)
**Problem**: Patterns are hardcoded and not easily extensible.
**Impact**: Adding new patterns requires modifying code.
**Fix**: Load patterns from external configuration file or database.

### 9. Hardcoded Resource Type Checks
**Location**: Line 805
**Problem**: Resource type strings are hardcoded.
**Impact**: Adding new resource types requires code changes.
**Fix**: Define resource types as constants or enums.

### 10. Hardcoded Action Type Strings
**Location**: Throughout parse_action function
**Problem**: Action type strings are hardcoded throughout.
**Impact**: Typos can cause silent failures; hard to maintain.
**Fix**: Use enums or constants for all action types.

## Completed Fixes

✅ **High Priority Items Completed:**
1. Defined `MAX_CHARACTER_NAME_LENGTH = 10` constant
2. Added bounds checking for array accesses in split operations and categorized array access
3. Replaced all hardcoded `split(..., 1)` with `SPLIT_LIMIT` constant
4. Removed duplicate condition markers

✅ **Verification:**
- Parser successfully regenerated abilities.json
- Output unchanged: 1325 abilities, 609 unique
- No regressions introduced

## Remaining Recommendations

### Medium Priority
5. Load patterns from external configuration
6. Add comprehensive error handling
7. Improve character name filtering logic

### Low Priority
8. Add unit tests for edge cases
9. Document all magic numbers and their rationale
10. Use enums for action types and condition types
