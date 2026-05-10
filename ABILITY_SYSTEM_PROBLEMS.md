# Ability System Problems - Concrete Examples

## 1. Data Structure Inconsistencies

### Missing/Null Fields
```json
// Line 285 - Inconsistent null handling
{
  "triggers": null,           // Should be required
  "is_null": true,            // Contradicts having data
  "cost": null,
  "effect": null
}

// Line 46 - Proper structure
{
  "triggers": "起動",         // Proper string value
  "is_null": false,
  "cost": { ... },
  "effect": { ... }
}
```

### Field Redundancy
```json
// Multiple fields for same purpose
{
  "count": 2,                    // Primary count
  "value": 2,                    // Duplicate
  "resource_icon_count": 2,       // Another duplicate
}
```

## 2. Implementation Architecture Problems

### Fragile String-to-Enum Mapping
```rust
// effects.rs:27-85 - Brittle pattern matching
fn from_action(s: &str) -> Self {
    match s {
        "sequential" => Self::Sequential,
        // ... 40+ string matches
        _ => { eprintln!("Unknown effect action: '{}'", s); Self::DoNothing }  // Silent failure!
    }
}
```

### Massive Effect Handler (1,600+ lines)
```rust
// execute_effect() violates single responsibility
pub fn execute_effect(&mut self, effect: &AbilityEffect) -> Result<(), String> {
    // 200+ lines of setup...
    match action {
        EffectAction::Sequential => self.execute_sequential_effect(...),
        // ... 40+ branches with different parameter patterns
        EffectAction::Custom => self.execute_custom(effect, &action_str),
    }
}
```

### Custom Action Workarounds
```rust
// Lines 278-304 - Hacky pattern matching
fn execute_custom(&mut self, effect: &AbilityEffect, action_str: &str) -> Result<(), String> {
    // Runtime action mutation!
    if effect.placement_order.as_deref() == Some("any_order") {
        let mut routed = effect.clone();
        routed.action = "move_cards".into();
        return self.execute_move_cards(&routed);
    }
    
    // String parsing heuristics!
    if action_str.contains("枚数を") && action_str.contains("増やす") {
        return self.execute_modify_limit("increase", 1);
    }
    
    eprintln!("Unhandled custom action: {}", action_str);  // Silent failure
    Ok(())
}
```

## 3. Data Mapping Issues

### Missing JSON Actions
```json
// Actions not in EffectAction enum
{
  "action": "reveal_per_group",     // No handler!
}
{
  "action": "conditional_on_result", // No handler!
}
```

### Inconsistent Parameter Semantics
```json
// Same field, different meanings
{
  "action": "gain_resource",
  "heart_colors": ["heart01", "heart03"],  // Choice options
}
{
  "action": "modify_required_hearts", 
  "heart_colors": ["heart02"],            // Which heart to modify
}
{
  "action": "reveal",
  "heart_colors": ["heart01", "heart06"], // Filter criteria
}
```

### Inconsistent Handler Patterns
```rust
// Lines 211-223 - Different parameter extraction
EffectAction::Draw => 
    self.execute_draw(effect, effect.count_or(1), ...),

EffectAction::GainResource => 
    self.execute_gain_resource(effect, effect.resource.as_deref().unwrap_or(""), 
        effect.resource_icon_count.unwrap_or(effect.count_or(1)), ...),

EffectAction::GainAbility => 
    self.execute_gain_ability(effect.ability_gain.as_deref().filter(|s| !s.is_empty())
        .or_else(|| if effect.text.is_empty() { None } else { Some(effect.text.as_str()) })
        .unwrap_or(""), ...),  // Complex fallback logic!
```

## 4. Performance and Maintainability

### File Size Issues
```bash
# abilities.json: 1MB+, 24,599 lines
# 1,399 abilities across 645 unique types
# All loaded into memory - no lazy loading
```

### Repeated Database Lookups
```rust
// Same pattern in multiple handlers
let card_db = self.game_state.card_database.clone();  // Expensive clone!
let filter = util::CardFilter { ... };  // Duplicated logic
```

### Code Duplication
```rust
// Similar validation scattered across files
// cost.rs:28, effects.rs:340, resolver.rs:70
if available < count {
    return Err(format!("Not enough cards: need {}, have {}", count, available));
}
```

## 5. Critical Impact

### Silent Failures
- Unknown actions become `DoNothing` 
- Custom actions unhandled with just error log
- Missing validation for complex nested actions

### Maintenance Burden
- Adding new action requires modifying 3+ files
- Parameter extraction logic inconsistent
- No centralized validation

### Performance Issues
- 1MB file loaded entirely at startup
- Repeated expensive database clones
- No caching mechanism

## Recommendations

1. **Schema Validation**: Strict JSON schema with required fields
2. **Refactor Effects**: Split 1,600-line method into focused handlers  
3. **Typed Actions**: Replace string matching with proper type system
4. **Data Access Layer**: Implement caching and lazy loading
5. **Consistent Parameters**: Standardize field semantics
6. **Error Handling**: Comprehensive validation and recovery
7. **Testing Framework**: Automated ability execution tests
