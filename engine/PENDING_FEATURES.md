# Pending Engine Features and Implementation Tasks

This document tracks all pending unimplemented features and stub implementations that need to be completed.

## Critical Issues

### 1. Fix `draw_card` vs `draw` Naming Mismatch
- **Severity**: HIGH
- **Description**: abilities.json uses "draw_card" but engine expects "draw"
- **Impact**: All draw abilities will fail to execute
- **Fix**: Add "draw_card" as alias in match statement in ability_resolver.rs
- **Status**: ✅ DONE
- **Test Required**: ✅ Yes - test that draw_card actions execute correctly

## Stub Implementations (Need Full Implementation)

### 2. `position_change` Handler
- **Current State**: Basic implementation (only swaps center/left)
- **Required**: Full implementation for all position changes
- **Status**: ❌ TODO
- **Test Required**: ✅ Yes - comprehensive position change tests

### 3. `appear` Handler
- **Current State**: Stub implementation (just logs)
- **Required**: Full implementation for card appearance logic
- **Status**: ❌ TODO
- **Test Required**: ✅ Yes - test appear effect

### 4. `choice` Handler
- **Current State**: Basic implementation (always picks first option)
- **Required**: Full implementation with player choice resolution
- **Status**: ❌ TODO
- **Test Required**: ✅ Yes - test choice resolution with different options

### 5. `set_card_identity` Handler
- **Current State**: Stub implementation (just logs)
- **Required**: Full implementation for card identity changes
- **Status**: ❌ TODO
- **Test Required**: ✅ Yes - test card identity changes

### 6. `restriction` Handler
- **Current State**: Stub implementation (just logs)
- **Required**: Full implementation for ability restrictions
- **Status**: ❌ TODO
- **Test Required**: ✅ Yes - test restriction effects

### 7. `re_yell` Handler
- **Current State**: Stub implementation (just logs)
- **Required**: Full implementation for re-yell mechanics
- **Status**: ❌ TODO
- **Test Required**: ✅ Yes - test re-yell effect

### 8. `modify_cost` Handler
- **Current State**: Stub implementation (just logs)
- **Required**: Full implementation for cost modification
- **Status**: ❌ TODO
- **Test Required**: ✅ Yes - test cost modification effects

## Unknown Implementation Quality (Need Testing)

### 9. `sequential` Handler
- **Current State**: Implemented but needs testing
- **Status**: ❌ TODO - Add comprehensive tests
- **Test Required**: ✅ Yes - test sequential effect execution

### 10. `conditional_alternative` Handler
- **Current State**: Implemented but needs testing
- **Status**: ❌ TODO - Add comprehensive tests
- **Test Required**: ✅ Yes - test conditional alternative logic

### 11. `look_and_select` Handler
- **Current State**: Implemented but needs testing
- **Status**: ❌ TODO - Add comprehensive tests
- **Test Required**: ✅ Yes - test look and select mechanics

### 12. `look_at` Handler
- **Current State**: Implemented but needs testing
- **Status**: ❌ TODO - Add comprehensive tests
- **Test Required**: ✅ Yes - test look at effect

### 13. `reveal` Handler
- **Current State**: Implemented but needs testing
- **Status**: ❌ TODO - Add comprehensive tests
- **Test Required**: ✅ Yes - test reveal mechanics

### 14. `select` Handler
- **Current State**: Implemented but needs testing
- **Status**: ❌ TODO - Add comprehensive tests
- **Test Required**: ✅ Yes - test selection mechanics

### 15. `modify_required_hearts_global` Handler
- **Current State**: Implemented but needs testing
- **Status**: ❌ TODO - Add comprehensive tests
- **Test Required**: ✅ Yes - test global heart modification

### 16. `modify_yell_count` Handler
- **Current State**: Implemented but needs testing
- **Status**: ❌ TODO - Add comprehensive tests
- **Test Required**: ✅ Yes - test yell count modification

### 17. `place_energy_under_member` Handler
- **Current State**: Implemented but needs testing
- **Status**: ❌ TODO - Add comprehensive tests
- **Test Required**: ✅ Yes - test energy placement under member

### 18. `activation_cost` Handler
- **Current State**: Implemented but needs testing
- **Status**: ❌ TODO - Add comprehensive tests
- **Test Required**: ✅ Yes - test activation cost mechanics

### 19. `play_baton_touch` Handler
- **Current State**: Implemented but needs testing
- **Status**: ❌ TODO - Add comprehensive tests
- **Test Required**: ✅ Yes - test baton touch play mechanics

## Summary

- **Total Tasks**: 19
- **Critical**: 1
- **Stub Implementations**: 7
- **Need Testing**: 11
- **Completed**: 0

## Progress Tracking

- [ ] Task 1: Fix draw_card naming mismatch
- [ ] Task 2: Implement position_change fully
- [ ] Task 3: Implement appear fully
- [ ] Task 4: Implement choice fully
- [ ] Task 5: Implement set_card_identity fully
- [ ] Task 6: Implement restriction fully
- [ ] Task 7: Implement re_yell fully
- [ ] Task 8: Implement modify_cost fully
- [ ] Task 9: Test sequential handler
- [ ] Task 10: Test conditional_alternative handler
- [ ] Task 11: Test look_and_select handler
- [ ] Task 12: Test look_at handler
- [ ] Task 13: Test reveal handler
- [ ] Task 14: Test select handler
- [ ] Task 15: Test modify_required_hearts_global handler
- [ ] Task 16: Test modify_yell_count handler
- [ ] Task 17: Test place_energy_under_member handler
- [ ] Task 18: Test activation_cost handler
- [ ] Task 19: Test play_baton_touch handler
