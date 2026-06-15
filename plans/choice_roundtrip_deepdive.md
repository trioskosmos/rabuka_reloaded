# Deep Dive: Choice Round-Trip — The Mess vs The Proper Way

## Why This Matters

The choice round-trip is the single worst architectural problem in the engine.
Items 1–5 in `engine_issues.md` are all symptoms of it. The `current_changes.md`
document literally says *"stopping because: position choice for 'stage' breaks
15+ existing tests"* — this is why. The choice system is too fragile to touch.

---

## The Mess: Current System

### Data Flow: 3 Fragmented State Locations

When a choice is created, state is stored in THREE places that must be kept in sync:

```
┌─────────────────────────────────────────────────────────────────┐
│                        AbilityResolver                          │
│  (ephemeral — created per call, dropped after)                   │
│                                                                  │
│  • pending_choice: Option<Choice>                                │
│  • selected_cards: Vec<i16>                                      │
│  • selected_card_ids: Vec<i16>                                   │
│  • moved_cards: Vec<i16>                                         │
│  • looked_at_cards: Vec<i16>                                     │
│  • pending_stage_cards: Vec<(i16, String)>                       │
│  • execution_context: ExecutionContext                           │
│  • last_effect_target: Option<String>                            │
│  • sub_choice_created: bool                                      │
└───────────────────────┬─────────────────────────────────────────┘
                        │  must manually save/restore
                        ▼
┌─────────────────────────────────────────────────────────────────┐
│                      AbilityQueueEntry                           │
│  (persistent — survives across requests)                         │
│                                                                  │
│  • execution_context: Option<ExecutionContext>                   │
│  • selected_card_ids: Vec<i16>              ← DUPLICATE         │
│  • pending_stage_cards: Vec<(i16, String)>   ← DUPLICATE        │
│  • last_effect_target: Option<String>         ← DUPLICATE       │
│  • cost_paid: bool                                               │
│  • effect_started: bool                                          │
│  • pending_commands: Vec<Command>                                │
│  • pending_choice_result: Option<ChoiceResult>                   │
│  • choice_card_no: Option<String>                                │
│  • selected_area: Option<String>                                 │
└───────────────────────┬─────────────────────────────────────────┘
                        │  some fields duplicated here too
                        ▼
┌─────────────────────────────────────────────────────────────────┐
│                         GameState                                │
│                                                                  │
│  • looked_at_cards: Vec<i16>               ← DUPLICATE          │
│  • revealed_cost_cards: Vec<i16>           ← DUPLICATE          │
│  • activating_card: Option<i16>            ← DUPLICATE          │
│  • pending_choice (from store_pending_choice) ← DUPLICATE       │
└─────────────────────────────────────────────────────────────────┘
```

### The Lifecycle (11 steps of pain)

```
Step 1: effect execution → sets resolver.pending_choice
Step 2: store_pending_choice() → serializes choice to JSON on gs
Step 3: process_current_ability() → pause_for_choice() on queue
         queue state = WaitingForChoice
Step 4: RESOLVER IS DROPPED ◄── all its state is gone
Step 5: Manually save ResolverState to queue entry fields
         (selected_card_ids, execution_context, last_effect_target, etc.)
Step 6: Frontend polls GET api/game-state → gets pending_choice JSON
Step 7: User clicks → POST api/execute-action
Step 8: resume_with_choice() in actions.rs:
Step 9:   CREATE A FRESH AbilityResolver::new() ◄── reads from queue entry
Step 10:  resolver.provide_choice_result() → does the thing
Step 11:  Read resolver state BACK, save to queue entry again
          DROP THE RESOLVER AGAIN ◄── if there's another choice, goto 9
```

### Concrete Bugs This Causes

**Bug 1: `handle_select_card` has two zone matches that collide.**

`choice.rs:395` (the cost handler) and `choice.rs:664` (the effect handler).
Both match `zone == "discard"`. The cost handler at line 395 has:

```rust
if allow_skip && !indices.is_empty() {
    match zone {
        "discard" => {
            // ... cost handling ...
            self.clear_choice_state(gs);
            return self.resume_pending_commands(gs);  // ← EARLY RETURN
        }
        // ...
    }
}
```

The effect handler at line 664 also matches `"discard"` (line 793), but it
*never runs* because the cost handler already returned. This is how the
position choice for stage got "eaten": the code entered the cost path's
discard arm, called `execute_selected_cards_from_zone` (which placed cards),
then `return Ok(())` prevented the position choice from ever being created.

**Bug 2: `finalize_choice` is never called for SelectCard choices.**

The match arm at `provide_choice_result:186`:

```rust
) => self.handle_select_card(...),
//      ^^^^^^^^^^^^^^^^^^^^^^^^
//      This returns the Result from handle_select_card DIRECTLY.
//      finalize_choice is only called WITHIN handle_select_card,
//      and only if the zone match doesn't return early.
```

The cost handler at line 395 has `return Ok(())` which skips `finalize_choice`
entirely. But `finalize_choice` is where `resume_pending_commands` is called,
which runs the sequential actions that were deferred by the choice. So when a
card selection is made and the cost handler early-returns, the sequential
actions (like "then place the selected card on stage") never run.

**Bug 3: `clear_choice_state` wipes `self.pending_choice`.**

`clear_choice_state` at line 1203 does `self.pending_choice = None`. It's used
indiscriminately. If a position choice was **just** created by
`place_card_with_position_choice` in the same call, `clear_choice_state` wipes
it before it can be stored. This happens in the `"discard"` cost arm (line 502):
`self.clear_choice_state(gs)` then `return self.resume_pending_commands(gs)`.

**Bug 4: Resolver state is lost between calls because new fields aren't added
to the save/restore cycle.**

The `ResolverState` struct has 6 fields. The `AbilityResolver` has ~20 fields.
When someone adds a new field to the resolver (like `sub_choice_created` or
`pending_stage_cards`), they need to remember to:
1. Add it to `ResolverState::from_resolver()`
2. Add it to `ResolverState::apply()`
3. Add it to `AbilityResolver::new()` (reads from queue entry)
4. Add explicit sync code in `resume_queue_with_choice()` for any field
   that's not covered by `ResolverState`

Current fields that ARE saved: `execution_context`, `selected_card_ids`,
`selected_area`, `moved_cards`, `looked_at_cards`, `last_effect_target`.

Current fields that are NOT saved (and lose state across calls):
- `sub_choice_created` — manually synced via extra code
- `pending_stage_cards` — manually synced via extra code
- `revealed_cost_cards` — manually synced via `res.rev` in actions.rs:412
- `is_reveal_cost`, `last_draw_count`, `looked_at_total_count`, `duration_effects` — lost

**Bug 5: `store_pending_choice` writes to `gs.pending_choice` (or rather
`inject_choice_ability_context` on the JSON), but `actions.rs` checks
`resolver.get_pending_choice()` which reads `self.pending_choice` on the
*new* resolver. If `store_pending_choice` was set on the *old* resolver
(before it was dropped), the new resolver doesn't have it.**

The workaround at `actions.rs:397-399`:

```rust
// Resolver's pending_choice takes priority; fallback to gs.pending_choice
// in case the resolver was dropped before store_pending_choice was called.
let new_choice = resolver.get_pending_choice().cloned();
```

This comment literally documents the desync. The "fallback to gs.pending_choice"
doesn't actually exist in the code — it just uses `resolver.get_pending_choice()`.
But `store_pending_choice` wrote to the JSON, not to a field. So the desync
means sometimes choices are silently dropped.

### Why Each Bug Exists (Root Cause)

Every single one of these bugs traces back to the same root cause:

**The resolver is destroyed and recreated, requiring manual state save/restore
across 3 locations, which is inevitably incomplete.**

---

## The Proper Way: Persistent Resolver + Async Generator

### Option A: Persistent Resolver (incremental fix)

Keep the resolver alive across choice boundaries instead of destroying it.

```
┌─────────────────────────────────────────────────────────────────┐
│                      AbilityQueueEntry                           │
│                                                                  │
│  • resolver: AbilityResolver          ◄── NEW: owned, persistent │
│  • cost_paid: bool                                              │
│  • effect_started: bool                                         │
│  • pending_commands: Vec<Command>                                │
│  • pending_choice_result: Option<ChoiceResult>                   │
│  • choice_card_no: Option<String>                                │
│  • selected_area: Option<String>                                 │
└─────────────────────────────────────────────────────────────────┘
```

Changes:

**1. Move `AbilityResolver` into `AbilityQueueEntry`:**

```rust
pub struct AbilityQueueEntry {
    pub id: AbilityId,
    pub card_no: String,
    pub player_id: String,
    pub ability: Ability,
    pub ability_index: usize,
    pub card_id: Option<i16>,
    pub trigger_type: AbilityTrigger,
    pub completed: bool,
    pub cost_paid: bool,
    pub cost_paid_index: usize,
    pub pending_choice_result: Option<ChoiceResult>,
    pub choice_card_no: Option<String>,
    pub conditional_choice: Option<String>,
    pub pending_commands: Vec<Command>,
    pub selected_area: Option<String>,

    // NEW: one field replaces 10 scattered fields
    pub resolver: Option<AbilityResolver>,
}
```

**2. `AbilityResolver` becomes a self-contained unit:**

```rust
pub struct AbilityResolver {
    pub pending_choice: Option<Choice>,
    pub looked_at_cards: Vec<i16>,
    pub card_database: Arc<CardDatabase>,
    pub duration_effects: Vec<(String, String)>,
    pub current_ability: Option<Ability>,
    pub activating_card_id: Option<i16>,
    pub execution_context: ExecutionContext,
    pub current_effect: Option<AbilityEffect>,
    pub revealed_cost_cards: Vec<i16>,
    pub is_reveal_cost: bool,
    pub last_draw_count: u32,
    pub looked_at_total_count: usize,
    pub selected_cards: Vec<i16>,
    pub selected_card_ids: Vec<i16>,
    pub selected_area: Option<String>,
    pub moved_cards: Vec<i16>,
    pub last_effect_target: Option<String>,
    pub sub_choice_created: bool,
    pub pending_stage_cards: Vec<(i16, String)>,
    pub pipeline: EffectPipeline,
    // NO ResolverState — all fields persist naturally
}
```

**3. Remove `ResolverState` entirely.** Nothing is saved or restored. The
resolver's fields are just there.

**4. Remove duplicated fields from `AbilityQueueEntry`:**
- `execution_context` → lives on resolver
- `selected_card_ids` → lives on resolver
- `pending_stage_cards` → lives on resolver
- `last_effect_target` → lives on resolver

**5. `actions.rs:resume_queue_with_choice()` becomes simple:**

```rust
fn resume_queue_with_choice(
    game_state: &mut GameState,
    choice: Choice,
    result: ChoiceResult,
) -> Result<(), String> {
    game_state.ability_queue.resume_with_choice(result.clone());
    let entry = game_state.ability_queue.current_entry_mut().unwrap();
    let resolver = entry.resolver.as_mut().unwrap();

    resolver.pending_choice = Some(choice);
    resolver.provide_choice_result(game_state, result)?;

    if let Some(new_choice) = resolver.get_pending_choice().cloned() {
        game_state.ability_queue.pause_for_choice(new_choice);
    } else {
        // completion logic (same as before but simpler)
    }
    Ok(())
}
```

No `ResolverState::from_resolver()`. No `ResolverState::apply()`. No manual
sync of `looked_at_cards`, `revealed_cost_cards`, `selected_cards` back to
queue fields. The resolver retains all its state naturally.

**6. `AbilityResolver::new()` becomes a one-time constructor:**

```rust
pub fn new(card_database: Arc<CardDatabase>, activating_card_id: Option<i16>) -> Self {
    AbilityResolver {
        pending_choice: None,
        looked_at_cards: Vec::new(),
        card_database,
        duration_effects: Vec::new(),
        current_ability: None,
        activating_card_id,
        execution_context: ExecutionContext::None,
        current_effect: None,
        revealed_cost_cards: Vec::new(),
        is_reveal_cost: false,
        last_draw_count: 0,
        looked_at_total_count: 0,
        selected_cards: Vec::new(),
        selected_card_ids: Vec::new(),
        selected_area: None,
        moved_cards: Vec::new(),
        last_effect_target: None,
        sub_choice_created: false,
        pending_stage_cards: Vec::new(),
        pipeline: EffectPipeline::new(card_database),
    }
}
```

No reading from queue entry. No conditional restores. Just defaults.

**7. `store_pending_choice()` stays the same** (it serializes to JSON for the
frontend), but it now reads from the persistent resolver instead of a
doomed-to-be-dropped one.

### Option B: Async Generator (bigger rewrite, cleaner)

Make the entire ability execution a single async function that `yield`s on
choice. This is the ideal architecture but requires a larger rewrite.

```rust
// Pseudocode — the entire ability execution is one function
async fn resolve_ability(gs: &mut GameState, ability: &Ability) -> Result<(), String> {
    // Cost payment (may yield for card selection)
    if let Some(ref cost) = ability.cost {
        pay_cost(gs, cost).await?;  // may yield Choice
    }

    // Effect execution (may yield multiple times)
    if let Some(ref effect) = ability.effect {
        execute_effect(gs, effect).await?;  // may yield Choice
    }

    Ok(())
}

// The async executor
impl GameState {
    pub async fn process_current_ability(&mut self) {
        let entry = self.ability_queue.current_entry().unwrap();
        match entry.resolver.resolve_ability(self).await {
            Ok(()) => self.ability_queue.complete_current(),
            Err(e) => { /* handle */ }
        }
    }
}
```

Rust doesn't have async generators natively, but the `genawaiter` crate or a
manual state machine can implement this pattern. The key insight: instead of
save/restore state across calls, the function's own stack frame holds the
state naturally through `await` points.

### Why the Proper Way Fixes the Bugs

| Bug | Root cause | How persistent resolver fixes it |
|-----|-----------|----------------------------------|
| 1. Two zone matches collide | Cost handler returns early, skipping effect handler | Resolver is the same object, both paths share state, no save/restore boundary to skip |
| 2. `finalize_choice` never called | Early return skips epilogue | Persistent resolver means cost handling and effect handling are in the same call chain — can't skip each other |
| 3. `clear_choice_state` wipes choice | Indiscriminate clearing of a field on the resolver | Same root issue — if resolver persists, callers are more careful about field lifetimes |
| 4. New fields lose state across calls | `ResolverState` is manually maintained and always incomplete | Fields live on the resolver naturally, no manual save/restore cycle to forget |
| 5. `store_pending_choice` desync | Old resolver vs new resolver have different `pending_choice` | There's only ONE resolver. `self.pending_choice` is always the current one. |

## Three Options Compared

### Option A: Persistent Resolver

Keep the resolver alive on the queue entry instead of destroying/recreating it.

```
Current:  create → use → save state → DROP → recreate → restore → use → save → DROP
Option A: create → use → park → use → park → use → DONE
```

**How it works:** The resolver moves from an ephemeral stack variable into the
`AbilityQueueEntry` as an `Option<AbilityResolver>`. When a choice is created,
the resolver stays alive — its fields are directly accessible. When the choice
resolves, `resume_queue_with_choice` borrows it from the queue entry.

**Problem: Rust borrow checker.** The resolver is INSIDE `GameState.ability_queue`,
but its methods take `&mut GameState`. You can't have both at once:

```rust
// Can't compile — game_state is already borrowed by current_entry_mut()
fn resume(game_state: &mut GameState) {
    let resolver = game_state.ability_queue.current_entry_mut()
        .and_then(|e| e.resolver.as_mut()).unwrap();
    resolver.provide_choice_result(game_state, result);  // ERROR
}
```

**Workaround:** Take the resolver out, drop the borrow, use it, put it back:

```rust
fn resume(game_state: &mut GameState) -> Result<(), String> {
    let mut resolver = game_state.ability_queue.take_resolver().unwrap();
    let result = resolver.provide_choice_result(game_state, result);
    game_state.ability_queue.set_resolver(resolver);
    result
}
```

This `take → use → put_back` pattern is needed everywhere the resolver is
used with the game state. It works but is error-prone — an early `return`
or `?` can leak the resolver out of the queue if the put-back is missed.

**Also NOT fixed:** The two-zone-match bug in `handle_select_card`. That's a
control flow bug within a single call — the cost handler at line 395 returns
early before the effect handler at line 664 runs. Making the resolver
persistent doesn't change this. You'd still need to fix the bug separately.

**Verdict:** Eliminates the save/restore cycle but adds borrow-checker friction
and doesn't fix the control flow. Net positive, but the ergonomics aren't great.

---

### Option B: Async Generator

Rewrite ability execution as a single async function that yields at choice points.

```rust
async fn resolve_ability(gs: &mut GameState, ability: &Ability) -> Result<(), String> {
    pay_cost(gs, ability.cost.as_ref()).await?;   // may yield Choice
    execute_effect(gs, ability.effect.as_ref()).await?;  // may yield Choice
    Ok(())
}

async fn execute_effect(gs: &mut GameState, effect: &AbilityEffect) -> Result<(), String> {
    match effect.action.as_str() {
        "sequential" => {
            for sub in effect.compound.actions.iter() {
                execute_effect(gs, sub).await?;  // may yield Choice
            }
        }
        "draw" => draw_cards(gs, effect).await,
        "select_cards" => {
            let cards = prompt_select_cards(gs, effect).await;  // yields Choice
            process_cards(gs, cards).await;
        }
        // ...
    }
}
```

Each `.await` is a suspension point. The Rust compiler generates a state
machine that saves locals across suspension points. No manual save/restore.

**Problem: `&mut GameState` across await points.** Async Rust requires that
references that live across `.await` points be `Send` (for multi-threaded
executors) or at least not conflict with other borrows. Since the engine is
single-threaded, this is manageable with a single-threaded executor.

**Problem: Recursive async.** Effect execution is deeply recursive
(sequential → conditional → draw → sequential → ...). Deeply recursive async
functions in Rust can have large hidden state machines. Each nested `.await`
expands the state machine. With 50+ effect types, this could blow up.

**Problem: Every effect function becomes async.** Even trivial effects like
`modify_score` that never yield. ~50 functions need signature changes.
Function pointers and trait objects become harder to use.

**Problem: Async infrastructure.** Need either a runtime (tokio, which the
web server already uses via actix) or a custom single-threaded executor.
The engine is currently sync. Adding async to the engine means either:
- Mixing sync engine with async web (current state, fine)
- Making the engine async too (big change, affects everything)

**Problem: Testing.** Tests drive the engine synchronously. Making the engine
async means test functions become `async fn` or need `block_on`. This changes
every test.

**Verdict:** Cleanest architecture but the highest cost. This is "rewrite it
properly" territory. The borrow checker + async + recursive effects is a
painful combination in Rust. Not recommended as a next step.

---

### Option C: Explicit Continuation Enum (recommended middle ground)

Encode the execution's suspension point as an explicit enum on the queue entry.
Instead of a generic resolver with ~20 fields (some saved, some not), have a
tight enum where each variant holds exactly the fields that suspension point
needs.

**Core idea:**

```rust
/// The ENTIRE possible paused state of an ability execution
pub enum AbilityContinuation {
    /// No ability being processed
    Idle,

    /// Sequential compound actions paused mid-way
    Sequential {
        actions: Vec<AbilityEffect>,
        index: usize,
        repeat_remaining: u32,
        selected_card_ids: Vec<i16>,
        moved_cards: Vec<i16>,
    },

    /// Waiting for card selection during cost payment
    PayCostSelectCards {
        cost: AbilityCost,
        zone: Zone,
        count: usize,
        already_indices: Vec<usize>,     // already selected in THIS round
        accumulated_ids: Vec<i16>,       // accumulated across sequential prompts
    },

    /// Waiting for card selection during effect execution
    EffectSelectCards {
        effect: Box<AbilityEffect>,
        zone: Zone,
        count: usize,
        card_type: Option<CardType>,
        accumulated_ids: Vec<i16>,
        /// Remaining sequential actions after this choice resolves
        pending_effects: Vec<AbilityEffect>,
    },

    /// Waiting for position choice when placing on stage
    SelectPosition {
        card_ids: Vec<i16>,
        state_change: Option<State>,
        target: String,
        pending_effects: Vec<AbilityEffect>,
    },

    /// Waiting for a yes/no or option choice
    SelectTarget {
        target_type: TargetType,
        options: Vec<String>,
        pending_effects: Vec<AbilityEffect>,
    },

    /// Waiting for heart color selection
    SelectHeartColor {
        count: u32,
        options: Vec<String>,
    },

    /// Waiting for order selection (put looked-at cards on deck in order)
    SelectOrder {
        looked_at_cards: Vec<i16>,
    },

    /// ... more variants as needed, one per distinct suspension type
}

/// The queue entry becomes simpler
pub struct AbilityQueueEntry {
    pub id: AbilityId,
    pub card_no: String,
    pub player_id: String,
    pub ability: Ability,
    pub card_id: Option<i16>,
    pub trigger_type: AbilityTrigger,
    pub completed: bool,
    pub cost_paid: bool,
    pub pending_choice: Option<Choice>,        // for the frontend
    pub continuation: AbilityContinuation,     // replaces ResolverState + 10 fields
}
```

**How execution works:**

```rust
impl GameState {
    /// Advance ability execution one step. Returns a Choice if one was created.
    pub fn step_ability(&mut self) -> Result<Option<Choice>, String> {
        let entry = self.ability_queue.current_entry_mut()?;
        match std::mem::replace(&mut entry.continuation, AbilityContinuation::Idle) {
            AbilityContinuation::Idle => {
                // Start executing the ability from scratch
                let ability = entry.ability.clone();
                drop(entry);
                self.start_ability(ability)  // returns new continuation
            }
            AbilityContinuation::Sequential { actions, index, .. } => {
                // Resume sequential execution from saved position
                self.run_sequential(actions, index, ...)
            }
            AbilityContinuation::EffectSelectCards { effect, accumulated_ids, pending_effects } => {
                // The player just chose cards, run the effect with those cards
                self.run_effect_with_selection(effect, accumulated_ids, pending_effects)
            }
            // ... one arm per variant
        }
    }
}
```

**The two-zone-match bug CANNOT EXIST** because each continuation variant has
EXACTLY one handler. There's no "first check if it's a cost, then check if
it's an effect" ambiguity. The state knows what it is.

**Key distinctions from Option A:**

| | Option A | Option C |
|---|---|---|
| State location | Resolver fields (all ~20) | Enum variants (each has ~3-5 fields) |
| Save/restore | None, but fields are always there | None, each variant drops when transitioning |
| Two-zone-match | Not fixed | Impossible by construction |
| Borrow checker | Painful (resolver inside GameState) | Fine (enum on queue entry, no methods on it) |
| New choice types | Add a field, might forget to persist | Add a variant, naturally complete |
| Code readability | Same messy flow | Explicit state machine traceable in logs |

**The `handle_select_card` function disappears.** Instead of a 700-line
monster with two zone matches, you have:
- `handle_cost_card_selection()` — handles cost card choices (~100 lines)
- `handle_effect_card_selection()` — handles effect card choices (~100 lines)
- `handle_position_selection()` — handles position choices (~30 lines)

Each is called by the appropriate `AbilityContinuation` arm. No shared state,
no early returns, no `allow_skip` collisions.

**Where the state lives:** The continuation enum is on the queue entry, which
is inside `GameState`. Methods that process it take `&mut GameState` and DON'T
need a separate resolver object. The `step_ability` function is on `GameState`
itself, or on the `AbilityQueue`.

**What about accumulated state across sequential steps?** Things like
`selected_cards`, `moved_cards`, `last_effect_target` — these are stored in
the `Sequential` variant's fields. When `Sequential` transitions to
`EffectSelectCards`, the accumulated data moves with it. When
`EffectSelectCards` transitions back to `Sequential`, the data is there.

**Does this work with deeply nested sequential effects?** Yes. Each time the
execution nests into a sub-effect, it pushes a new frame. But actually — you
can handle this without a stack. The key insight: when the player makes a
choice, the remaining sequential actions are stored in `pending_effects`.
When the choice resolves, the continuation runs the selected action (which
may itself create sub-choices), then `pending_effects` is re-stored for the
next choice. This is exactly how `compound.rs` works today with
`set_pending_commands` — Option C just makes every effect type use the same
pattern instead of only sequential effects.

**Testing:**

```rust
#[test]
fn test_card_selection_choice() {
    let mut gs = GameState::new();
    gs.ability_queue.current_entry_mut().unwrap().continuation =
        AbilityContinuation::EffectSelectCards {
            effect: Box::new(effect),
            zone: Zone::Hand,
            count: 2,
            accumulated_ids: vec![],
            pending_effects: vec![],
        };
    
    // Provide the choice result directly
    let choice = Choice::SelectCard { zone: "hand", count: 2, .. };
    let result = ChoiceResult::CardSelected { indices: vec![0, 1] };
    gs.resolve_choice(choice, result).unwrap();
    
    // Assert expected outcome
    assert_eq!(gs.active_player().waitroom.len(), 2);
}
```

You can construct ANY intermediate state directly without jumping through
hoops. No need to queue an ability, pay costs, etc. — just set the
continuation and feed it a choice.

---

## Recommendation

| Criterion | Option A (Persistent) | Option B (Async) | Option C (Continuation) |
|-----------|----------------------|------------------|------------------------|
| Eliminates save/restore | ✅ | ✅ | ✅ |
| No borrow-checker pain | ❌ take/put-back everywhere | ⚠️ &mut across await | ✅ plain enum |
| Fixes two-zone-match | ❌ separate fix needed | ✅ linear flow | ✅ impossible by design |
| Readability | ❌ same messy flow | ✅ reads like sync code | ⚠️ explicit but verbose |
| Effort | 2-3 days | 1-2 weeks | 4-6 days |
| Risk | Low | High (changes everything) | Medium |
| Test impact | Low | High (need async runtime) | Low (construct states) |
| New effect types | Same pattern | Trivial | May need new variant |

**If you want to ship fast:** Fix the bugs directly + Option A (3-4 days total).
The borrow checker pain is manageable with the take/put-back pattern.

**If you want to fix it properly:** Option C. It eliminates the root cause
(the ambiguous "resolver with 20 fields") rather than patching around it.
The explicit state machine is more code, but each piece is simple and provably
correct. The two-zone-match bug literally cannot exist.

**Option B is overkill for this codebase.** Async Rust is powerful but the
ergonomic cost (recursive async, `&mut` across await, test infrastructure)
isn't justified when Option C gives the same correctness guarantees with
plain Rust enums.

---

## Appendix: The Two-Zone-Match Bug in Detail

This is the specific bug that caused development to stop. Here is the exact
flow:

### When the bug fires

1. A card ability says: "discard 2 cards from hand, then place up to 1 member
   card from discard on stage, choosing the position"

2. The cost system creates a `SelectCard` choice for `zone: "hand"`, `count: 2`

3. User selects 2 cards → `provide_choice_result` → `handle_select_card`

4. Inside `handle_select_card(zone = "hand", count = 2, ...)`:

   a. Enters the cost handler at line 395:
      `if allow_skip && !indices.is_empty() {`

   b. `match zone { "hand" => { ... } }` — moves cards from hand to discard ✓

   c. Falls through to `self.clear_choice_state(gs)` (line 657)

   d. `if gs.ability_queue.has_pending_commands() { return self.resume_pending_commands(gs); }`
      — runs the next sequential action: "select from discard to stage"

   e. `resume_pending_commands` runs `Command::Effect(effect)` for the
      "place on stage" step

   f. `execute_effect` runs, which eventually creates a `SelectCard` choice
      for `zone: "discard"`

   g. Inside the same `handle_select_card` call (re-entered via step e):
      Enters the cost handler at line 395 again

   h. `match zone { "discard" => { ... } }` — the COST handler matches first!

   i. `is_select_action` is false for a discard-to-stage move, so it falls
      into the `else` branch: `self.execute_selected_cards_from_zone(gs, "discard", ...)`

   j. This places the card, BUT does NOT create a position choice (position
      choice is handled in the EFFECT handler, lines 900-963, which never runs)

   k. `self.clear_choice_state(gs)` at line 502
   l. `return self.resume_pending_commands(gs)` at line 503 — resumes, finds
      nothing, returns Ok

   m. Back in the outer cost handler (step d): returns Ok

5. Card is on stage but at the WRONG position (always center, default).

### Why the fix isn't "just delete the cost handler"

Because the cost handler has special logic for:
- Sequential card selection (re-prompt if not enough cards selected)
- Optional cost tracking (`optional_cost_was_paid` flag)
- Cost-specific validation (card type/group/character matching)
- Reveal handling for reveal costs

The effect handler has DIFFERENT special logic for:
- Position choice on stage placement
- `is_select_action` flag for card reference without movement
- `selected_cards` accumulation for subsequent effects

These two handlers can't be separated by a return early boundary. They need to
be merged into a single match where both cost and effect considerations are
checked in one pass, with `finalize_choice` called exactly once.
