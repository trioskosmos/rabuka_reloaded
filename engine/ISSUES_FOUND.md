# Engine Issues Found

## Build & Runtime Issues

### 1. Exit Code 1 on Successful Build/Test
- **File**: Various (build output)
- **Issue**: Commands return exit code 1 even when they succeed (all tests pass, build completes)
- **Severity**: High - Breaks CI/CD pipelines
- **Details**: `cargo build`, `cargo test`, `cargo run` all complete successfully but return exit code 1

## Compilation Warnings

### 2. Unused Variable: `group`
- **File**: [src/ability/util.rs](src/ability/util.rs#L166)
- **Line**: 166
- **Issue**: Variable `group` assigned but never used
- **Fix**: Prefix with underscore: `_group`
- **Code**: `let group = group_name.unwrap_or("None");`

### 3. Invalid drop() of Reference
- **File**: [src/ability/effects/state.rs](src/ability/effects/state.rs#L98)
- **Line**: 98
- **Issue**: Calling `drop()` on a reference (`&mut Player`) which does nothing
- **Fix**: Use `let _ = player;` instead
- **Code**: `drop(player);`

## Test Warnings

### 4. Unused Imports in Tests
- **Files**: Multiple test files
- **Details**:
  - [tests/test_modules/mirai_ticket_test.rs](tests/test_modules/mirai_ticket_test.rs#L2): `MemberArea`
  - [tests/test_modules/yoshiko_center_ability_test.rs](tests/test_modules/yoshiko_center_ability_test.rs#L2): `TurnEngine`
  - [tests/test_modules/yoshiko_debug_move_test.rs](tests/test_modules/yoshiko_debug_move_test.rs#L2): `TurnEngine`
  - [tests/test_modules/yoshiko_debug_test.rs](tests/test_modules/yoshiko_debug_test.rs#L2): `TurnEngine`
  - [tests/test_modules/yoshiko_detailed_test.rs](tests/test_modules/yoshiko_detailed_test.rs#L4): `TurnEngine`
  - [tests/test_modules/yoshiko_fixed_test.rs](tests/test_modules/yoshiko_fixed_test.rs#L2): `TurnEngine`
  - [tests/test_modules/yoshiko_main_effect_only_test.rs](tests/test_modules/yoshiko_main_effect_only_test.rs#L2): `TurnEngine`
  - [tests/test_modules/yoshiko_single_target_test.rs](tests/test_modules/yoshiko_single_target_test.rs#L2): `TurnEngine`

### 5. Unused Variables in Tests
- **File**: [tests/test_modules/himeno_test.rs](tests/test_modules/himeno_test.rs#L197)
- **Line**: 197
- **Variable**: `filler`
- **File**: [tests/test_modules/mirai_ticket_test.rs](tests/test_modules/mirai_ticket_test.rs#L21)
- **Line**: 21
- **Variable**: `live`
- **File**: [tests/test_modules/mirai_ticket_test.rs](tests/test_modules/mirai_ticket_test.rs#L22)
- **Line**: 22
- **Variable**: `center`

### 6. Unused Function
- **File**: [tests/test_modules/performance_pipeline_test.rs](tests/test_modules/performance_pipeline_test.rs#L22)
- **Line**: 22
- **Function**: `advance_to_live_start`

## Clippy Warnings (Code Quality)

### 7. Missing Default Implementation for CardDatabase
- **File**: [src/core/card.rs](src/core/card.rs#L179)
- **Issue**: `new()` method exists but no `Default` trait implementation
- **Recommendation**: Implement `Default` trait to follow Rust conventions

### 8. Missing Default Implementation for GameModifiers
- **File**: [src/core/game_modifiers.rs](src/core/game_modifiers.rs#L51)
- **Issue**: `new()` method exists but no `Default` trait implementation
- **Recommendation**: Implement `Default` trait

### 9. Inefficient or_insert_with Usage
- **File**: [src/core/card_loader.rs](src/core/card_loader.rs#L94)
- **Issue**: Using `or_insert_with(Vec::new)` instead of `or_default()`
- **Fix**: Replace with `.or_default()`

