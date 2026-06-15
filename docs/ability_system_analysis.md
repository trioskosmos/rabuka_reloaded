# Ability System Analysis

## Current Architecture

```
Card text (Japanese) → Parser (Python) → abilities.json → Engine (Rust) → Game state
```

- 762 abilities total
- ~50 have QA entries with tests
- 69 have identified parser/engine gaps

## The Problem

The parser outputs a flat JSON bag of `Option<...>` fields (113 on `AbilityEffect`). Sequential ability steps can't reference each other's outputs. The engine works around this with global state (`gs.revealed_cards`, `gs.moved_cards`) but only for patterns the author anticipated.

When a new pattern appears — "set cost to chosen card's original cost minus 1" — there's no generic way to express it in the JSON. The parser drops the nuance, and the engine has no handler for it.

## Identified Gaps (69 abilities)

### Category 1: Pronoun/card references across steps (~55 abilities)

Card text says "これにより公開したカード" (the card revealed by this) or "選んだメンバー" (the member chosen) — a reference to the result of a previous step.

**Parser output:** Sequential actions with no linking.

**What's needed:** Each step can produce a named result (`step_1_result`), and later steps can reference it.

**Example:**

PL!HS-bp5-005-R 徒町小鈴:
```
手札の『DOLLCHESTRA』のカードを1枚控え室に置いてもよい：
自分のステージにいる『DOLLCHESTRA』のメンバー1人を選ぶ。
ライブ終了時まで、このメンバーのコストは、
選んだメンバーが元々持つコストより1低い値に等しくなる。
```

Parser outputs `original_value: true` + `action: "select"` but misses "cost minus 1" math and doesn't link the selected card's base cost to the cost_set effect.

### Category 2: Cross-position comparison (~3 abilities)

PL!S-bp5-002-R+ 桜内梨子:
```
右サイドエリアと左サイドエリアにいるメンバーのコストが同じ場合
```

Parser outputs `comparison_type: "equality"` with `position: "left_side"` but drops the comparison target (`right_side`). No `comparison_target` field in the JSON.

### Category 3: Dynamic values from previous steps (~8 abilities)

PL!N-bp5-003-R 桜坂しずく:
```
自分の控え室にあるライブカードを1枚選び、
そのカードのスコアに等しい数のEを支払ってもよい。
```

Pay energy equal to a chosen card's score. The parser splits into sequential steps but doesn't carry the dynamic value forward.

## The "Proper" Way

### Option A: Typed Action IR (intermediate representation)

Replace the 113-field `AbilityEffect` with ~15 typed action constructors, each with exactly the fields it needs:

```rust
enum AbilityAction {
    // Gain
    GainBlade { count: u32, color: Option<HeartColor>, duration: Duration, targets: TargetSpec },
    GainHeart { count: u32, color: HeartColor, duration: Duration, targets: TargetSpec },

    // Movement
    Discard { count: u32, source: Zone, filter: CardFilter, optional: bool },
    Draw { count: u32, destination: Zone },
    MoveCard { source: Zone, dest: Zone, count: u32, filter: CardFilter, optional: bool },

    // State
    ChangeState { state: CardState, targets: TargetSpec, filter: CardFilter },
    ModifyScore { value: i32, duration: Duration, targets: TargetSpec },
    ModifyCost { value: i32, operation: Op, targets: TargetSpec, duration: Duration },

    // Compound
    Sequential(Vec<AbilityAction>),
    Conditional { condition: Condition, then: Box<AbilityAction>, otherwise: Option<Box<AbilityAction>> },
    Choice { options: Vec<AbilityAction> },

    // Steps that produce named results (the key addition)
    Select { filter: CardFilter, count: u32, output: StepRef },
    Reveal { source: Zone, count: u32, output: StepRef },
}
```

**The key addition:** `StepRef` — a reference to the output of a previous step:

```rust
struct StepRef(String);  // e.g. "step_1"

// Usage in later actions:
MoveCard {
    source: Zone::Waitroom,
    dest: Zone::Hand,
    count: 1,
    filter: CardFilter {
        name_constraint: Some(NameRef::Step("reveal_1")),  // "use name from step reveal_1"
        ..Default::default()
    },
    ..
}

ModifyCost {
    value: ValueRef::StepScore("select_1", -1),  // "selected card's score minus 1"
    operation: Op::Set,
    targets: TargetSpec::Selector("select_1"),    // "the card from step select_1"
    ..
}
```

**Benefits:**
- Wrong field combinations don't compile
- Cross-step references are explicit and type-checked
- Each action variant is a self-contained unit with its own test
- The engine becomes `match` on a Rust enum, not string comparison

**Cost:** Parser needs a code generator backend. 762 abilities need conversion (but mechanical — each is a direct mapping from JSON).

### Option B: Keep JSON, add a shared step namespace

Keep the current JSON format but add a `step_results` map between sequential actions:

```json
{
  "action": "sequential",
  "steps": [
    {
      "id": "discard_1",
      "action": "move_cards",
      "source": "hand",
      "destination": "discard",
      "count": 1,
      "filter": { "group_names": ["DOLLCHESTRA"] }
    },
    {
      "id": "select_1",
      "action": "select",
      "count": 1,
      "target": "self",
      "zone": "stage",
      "filter": { "group_names": ["DOLLCHESTRA"] }
    },
    {
      "action": "modify_cost",
      "target": { "ref": "select_1" },
      "value": { "ref_score": "select_1", "offset": -1 },
      "operation": "set",
      "duration": "live_end"
    }
  ]
}
```

**Benefits:**
- Backward compatible with existing JSON (steps without `id` work as before)
- Parser changes are incremental
- Engine changes are localized to handlers that read `value`/`target` references

**Cost:** More fields on `AbilityEffect` (`id`, `ref`, `ref_score`). Not type-safe — invalid references are runtime errors.

### Recommendation

**Option B is more practical.** It doesn't require rewriting 762 abilities or the parser's backend. The parser already generates sequential actions — it just needs to:
1. Assign `id` fields to steps that produce results
2. Emit `ref` / `ref_score` fields on downstream steps that reference those results
3. Emit `comparison_target: "right_side"` for cross-position equality checks

The engine changes are also incremental:
- `execute_sequential_effect` passes a `step_results: HashMap<String, StepOutput>` between steps
- Handlers that read `value`/`target` resolve `ref` / `ref_score` against `step_results`
- `location_condition` handler gets a `comparison_target` field for cross-position checks

This covers all 69 identified gaps without rewriting anything.
