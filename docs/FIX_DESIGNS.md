# Fix designs — allocation targets

> **Design only. No code changed.** Companion to `docs/ALLOCATION_TARGETS.md`
> (the measurements) and `docs/ENGINE_AUDIT_FINDINGS.md` (correctness audit).
>
> Ordered by measured impact. Each entry states the mechanism, the exact change,
> what it is gated on, and — where I could not determine it from the code alone —
> the counter that settles it before anyone starts.

---

## Fix 1 — delete a dead 118-allocation computation on the choice path

**Impact: ~118 allocations per choice prompt. The largest single item found.**
**Confidence: certain.**

`ability/resolver.rs:924-942`

```rust
#[cfg(feature = "serde_support")]
{
    let mut json = choice.to_frontend_json();          // ~62 allocs
    if let Some(ref mut j) = json {
        if let Some(entry) = gs.ability_queue.current_entry() {
            if let Some(ref effect) = entry.ability.effect {
                if let Some(ref maker) = effect.choice_maker_any() {
                    if let Some(obj) = j.as_object_mut() {
                        obj.insert("choice_maker".to_string(),
                                   serde_json::Value::String(maker.to_string()));
                    }
                }
            }
        }
        gs.inject_choice_ability_context(j);            // ~56 more
    }
}                                    // <-- `json` drops here. Never stored, never returned.
```

`json` is a block-local `let mut`. It is mutated by two helpers and then dropped.
`inject_choice_ability_context(&self, &mut Value)` takes `&self` on `GameState` and
a mutable reference to the value — it cannot store it anywhere, and it cannot
mutate `gs`. So the whole block is a complete dead computation.

It costs:
- `serde_json::to_value(self)` over `Choice` — for `SelectCard`, ~24 fields
  including 8 `String`s and 4 `Option<Vec<_>>`;
- `inner.as_object().cloned()` at `ability/types.rs:742` — **a deep clone of the
  entire serialised object**, purely so the variant can be unwrapped;
- `Value::String(description.clone())` at `:745` — a second copy of the prompt;
- `obj.insert("title".into(), …)` and `.into()` on every key — a `String` per key;
- `choice_maker.to_string()`, plus `inject_choice_ability_context`'s own
  `full_text.clone()` and `describe_effect_en`.

**Change:** delete the block. If the `choice_maker` / ability-context injection
is genuinely wanted, it belongs in `GameState::get_pending_choice_json`
(`core/game_state/abilities.rs:2475`), which is the API-boundary caller that
*does* return the value to a frontend. That function already calls both
`to_frontend_json()` and `inject_choice_ability_context` — so the logic exists
where it is needed, and the `resolver.rs` copy is a duplicate that throws its
result away.

**Do not** simply gate it on `logging_enabled()`. It has nothing to do with
logging; it builds frontend JSON. Delete it, or move the injection into
`get_pending_choice_json` if the fields are missing there.

**Gate:** full suite; confirm no test asserts on the discarded value (nothing
can, it was dropped); confirm `get_pending_choice_json`'s output still carries
`choice_maker` and the ability context. **Risk: essentially none.** This is the
highest-confidence, highest-impact item in the audit.

---

## Fix 2 — establish whether the ability decode cache is working (26 allocs/action)

**Impact: up to 26 allocations + 4,530 bytes per action, 26% with `get_ability`.**
**Confidence: mechanism certain, cause unresolved. Measure first.**

`ability_store.rs:45-62` is correct as written — a cache hit does
`Arc::clone`, which is an atomic increment and **never allocates**. Only the
miss path `Arc::new(self.decode())` allocates, at ~66 bytes.

The trace attributes 26 allocations to `AbilityRef::resolve`, so either:

- **(a)** the cache is genuinely missing 26 times per action, or
- **(b)** `decode()` is inlined into `resolve` and the tracer is folding
  `get_ability`'s allocations into `resolve`'s frame.

There is no cache reset anywhere in `engine/src` — I checked — and the cache is a
process-global `OnceLock<Vec<OnceLock<Arc<Ability>>>>` sized `NUM_ABILITIES = 985`.
So (a) would mean something upstream is not producing stable ability indices, or
far more distinct abilities are touched per action than expected.

**Step 1 — the counter that settles it.** Add two relaxed atomics:

```rust
static RESOLVE_HIT:  AtomicU64;   // slot.get() returned Some
static RESOLVE_MISS: AtomicU64;   // Arc::new(self.decode())
```

and expose them beside `alloc_count()`. Print both next to `total_actions` in the
bench summary. **One run answers it.** If `MISS / (HIT+MISS)` is above a few
percent, the cache — recorded as a completed win in `PERF_ENGINE_COMBINED.md` — is
not in effect on desktop and this becomes the project's top priority. If it is
near zero, the trace attribution is (b) and the real target is `get_ability`
itself.

**Do this before writing any code for it.** A refactor aimed at the wrong one of
these two costs days.

**If (a) — likely cause to investigate:** `AbilityRef(u16)` indices are assigned
by the generator; if a single logical ability maps to several indices, or the
stage/hand iteration order causes a fresh `AbilityRef` per card copy, the slot
index would churn. Check whether `Card::abilities` is rebuilt anywhere (a
`Vec<AbilityRef>` copied per `Card` clone would not churn indices, but a
re-decode into fresh indices would).

**If (b) — `get_ability`'s own allocations.** `vm.rs:249-260` decodes an
`Ability` from bytecode: a `Vec<AbilityRef>`, an `Option<Box<CompoundBranch>>`,
`Option<Arc<EffectKind>>`, and every `Vec<String>` / `Option<Box<…>>` in the
filter. ~4,530 bytes over 30 calls = ~150 bytes each, consistent with an
`Ability` tree skeleton rather than the deep filter. The filter is behind
`Arc` and shared, so the per-decode cost is mostly the tree spine. Reducing it
means a cheaper decoded representation, which is a real project — do not attempt
it before the counter says the cache is missing.

**Side note found while reading this:** `vm.rs:251-252` puts
`crate::timer::Timer::start("vm::get_ability")` inside
`#[cfg(not(feature = "no_std"))]` rather than behind `profiling`. In a build with
`alloc_tracker` but not `profiling`, that call still runs its body. It is cheap
when the `profiling`-gated parts compile out, but the cfg is inconsistent with
every other instrumentation site and should be `#[cfg(feature = "profiling")]`.

---

## Fix 3 — the TAS scan's 19 tiny allocations

**Impact: 19 allocations, 38 bytes, for one scan.** Almost all of them are 1–3
bytes each, i.e. `SmallVec` spills and one-element boxes rather than real work.
**Confidence: high that a fix exists; needs one traced run to locate.**

`trigger_auto_abilities_for_player_with_event` — the code I changed this session.
19 allocations at ~2 bytes average is a density problem, not a size problem: the
function does a board scan and enqueues, and almost everything it allocates is a
degenerate collection.

Candidates, in the order I'd check them:
- the `SmallVec<[i16; 4]>::from_slice(moved_cards)` I introduced at
  `abilities.rs:958` — allocates on every batch over 4 cards. Since the
  `TriggerEvent` deletion the callee only ever needs the batch as a slice, so a
  `SmallVec<[i16; 16]>` (or just passing `&[i16]` down to
  `just_completed_batch_matches`, which is the only consumer that copies)
  removes it;
- `just_completed_batch_matches(&moved_cards)` at `abilities.rs:597` taking
  `&[i16]` and doing a comparison per card;
- the enqueue path building a `MovementEvent` per movement — `MovementEvent`
  carries a `String` (`core/types.rs:955-970`), so every movement batch allocates
  once per card. `PlayerId(Arc<str>)` instead removes it; that is catalogue §6.7
  and needs a serde check because the field crosses the arena protocol.

**Gate:** a second `RABUKA_TRACE_ALLOC_AT` run at an action index with a *higher*
scan count, to confirm which of these dominates. 19 allocations is worth chasing
only once you know they are 19 cheap allocations rather than 3 expensive ones.

---

## Fix 4 — two `Vec`s that should be `SmallVec`

**Impact: 10 allocations, 1,856 bytes per action. Mechanical. Confidence: high.**

| Site | Evidence | Change |
| --- | --- | --- |
| `RawVec<AbilityFilterBranch>::grow_one` | 8 allocations, 1,024 B | a `Vec` growing 8× from empty; `SmallVec<[AbilityFilterBranch; N]>` |
| `RawVec<MemberContribution>::grow_one` | 2 allocations, 832 B | 416 B per push — a `Vec` of large elements built by `push`; `SmallVec<[MemberContribution; 4]>` |

The codebase already has this idiom and applies it correctly elsewhere — zone
storage in `core/player.rs` is entirely inline, `waited_stage_cards` is
`SmallVec<[i16;3]>`. These two are the same pattern missing.

**Determine the inline capacity from the real bound, not a guess** — `MemberContribution`
at 416 B/element means even a `SmallVec<[_; 4]>` is 1.6 KB inline, so a small
`N` matters more than usual. If the bound is genuinely unbounded (a turn can
produce many contributions), keep the `Vec` and reserve once instead of growing
8 times.

**Gate:** tests; allocation delta with `total_actions` fixed. **Risk: none.**

---

## Fix 5 — `EffectKind::from_action`'s `to_lowercase()`

**Impact: 3 allocations, 29 bytes per action. Confirmed real. Confidence: high.**

`core/card.rs:1263-1268` starts `let a = action.to_lowercase();` before a 60-arm
match on `action: ActionType` — a fieldless enum whose `to_str()` already returns
a canonical `&'static str`. Three allocations per action, confirmed by the tracer.

**Change:** match on `action` directly, with both the canonical wire spelling and
its lowercase form as arms; fall back to `eq_ignore_ascii_case` for mixed-case
input if any exists.

**Gate:** a corpus test asserting `from_action(s)` and
`from_action(&s.to_lowercase())` agree for every ability in the blob —
`abilities_gen::NUM_ABILITIES` is 985, so the test is cheap and exhaustive.
**Risk: none**, given that test.

---

## Fix 6 — `enqueue_printed_ability`'s identity string

**Impact: 2 allocations, 17 bytes per action, but ~150-byte strings.**
**Confidence: high that the string is avoidable; this one can change gameplay.**

`core/game_state/abilities.rs` enqueues with
`format!("{}_{}", card_no, ability.full_text)` and the consumer
(`abilities.rs:1232-1253`) `strip_prefix`es the card number and compares the
remainder to each ability's `full_text` — re-deriving, by string comparison, an
index the caller already had from `enumerate()`.

**Change:** carry `(card_id: i16, ability_index: usize)` through
`trigger_auto_ability`. `abilities_to_trigger` becomes
`SmallVec<[(i16, usize); 4]>` instead of `SmallVec<[(String, String, i16); 4]>`.

**⚠ Two things to handle explicitly, not silently:**

1. **This can change game outcomes.** Two abilities on one card with identical
   `full_text` but different `count` or `target` currently **collide**, and
   `find_printed_ability` returns the first. A `(card, index)` key makes that
   impossible. So the change is strictly a fix, but it is a *behavioural* one and
   must be gated on `total_actions` plus a win-rate comparison — not on the test
   suite. Report the collision as a separate bug so the behaviour change is
   reviewable on its own.
2. `AbilityRef::to_arc` is documented as "returns `self.resolve()`", and
   `find_printed_ability` resolves each ability twice. Trivial follow-on.

**Risk: medium.** This is the only fix on the list where I would not expect the
test suite alone to be sufficient evidence.

---

## Fix 7 — `generate_possible_actions`, ~75 allocations per action

**Impact: 378 allocations for 5 actions. The largest per-action budget found,
and not covered by the execution trace.** Confidence: high on the main cause.

`game/game_setup.rs:2100-2119`:

```rust
let activatable: Vec<(usize, crate::Arc<crate::card::Ability>)> = card.abilities.iter()
    .enumerate()
    .map(|(printed_index, ar)| (printed_index, ar.resolve()))     // Arc clone per ability
    .chain(game_state.gained_card_abilities.get(&card_id)
        .into_iter().flatten().enumerate()
        .map(|(gained_index, gained)| (…, crate::Arc::new(gained.clone()))))  // deep Vec<Ability> clone
    .collect();
```

Three defects:
1. the `Vec` itself, built to extend a lifetime the borrow checker could be
   satisfied without (the loop only reads `&GameState`);
2. `ar.resolve()` per ability, purely to extend a lifetime — `AbilityRef` is a
   `Copy` `u16`, and the loop can call `resolve()` inside instead, only for
   abilities that survive the two cheap gates below;
3. `crate::Arc::new(gained.clone())` — a **deep copy of a `Vec<Ability>`** into a
   fresh `Arc`, and the whole `chain` is constructed even when
   `gained_card_abilities` has no entry for the card (the common case).

**Change:** hold `SmallVec<[AbilityRef; 4]>` and resolve inside the loop; gate
the gained chain behind `if let Some(gained) = …get(&card_id)`. Changing
`gained_card_abilities` to `Vec<Arc<Ability>>` turns the deep clone into an
`Arc` bump and also removes an allocation from `find_gained_activation`.

Also in the same function: `game_setup.rs:2087` does
`area_name.parse().unwrap_or(MemberArea::Center)`, and `MemberArea::from_str`
(`core/zones.rs:141-152`) has an `_ => Err(format!("Invalid area: {}", s))` arm — so
a parse failure allocates a `String` to discard. `area_name` is always one of
three literals from `stage_positions`, and `MemberArea::from_index(slot)` already
exists and returns `Option` with no allocation.

**Gate:** `total_actions` fixed (this changes which actions are offered, so the
trajectory is the gate). **Risk: low-medium.**

---

## Fix 8 — the measurement command itself

**Impact: makes every other number in this document trustworthy. One line.**

```
- cargo build --release --features "profiling alloc_tracker" --bin sim_bench
+ cargo build --release --features "alloc_tracker" --bin sim_bench
```

`profiling` adds 902,530 allocations per 200-game run — 16.6% — of the profiler
measuring itself, which is why `PERF_ENGINE_COMBINED.md`'s 10,228,233 baseline
and this session's 5,442,137 both disagree with the real 4,539,607.

`profiling` is for flamegraphs. `alloc_tracker` is for counts. They are not
composable, and the docs currently tell people to compose them.

**Also:** re-baseline. Every allocation figure recorded under the old command
needs re-measuring, and `docs/PERF_ENGINE_OPTIMIZATION_CATALOG.md` §19.3/§19.10
should be corrected to say the flat measurements were partly this.

---

## User-facing safety assessment — do this before writing any of the above

Checked 2026-09-30 against the real consumer chain, not assumed. **The headline:
Fix 1 is the only one that touches user-facing output at all, and it is safe —
but only because `choice_maker` has zero consumers. That was worth verifying
rather than assuming, because `get_pending_choice_json` does *not* contain the
logic that resolver.rs was duplicating.**

### The choice-JSON chain, traced end to end

```
Choice  →  Choice::to_frontend_json()        ability/types.rs:734
             ↓ serde_json::Value
GameState::get_pending_choice_json()          core/game_state/abilities.rs:2475
             ↓ + inject_choice_ability_context()
game::display::…                              game/display.rs:1386, :1812
             ↓ reads pc.get("selection_cards")
display payload "pending_choice"  →  web server  →  web_ui/js/components/ChoiceView.js
```

`display.rs` reads **`selection_cards`**. `ChoiceView.js` reads exactly four
fields: `ability_text` (×3), `description` (×3), `title` (×2), `prompt_en` (×1).

| Field | Produced by | Read by the UI? |
| --- | --- | --- |
| `selection_cards` | `to_frontend_json` | **yes** — `display.rs:1386` |
| `ability_text` | `inject_choice_ability_context` | **yes** — `ChoiceView.js` |
| `prompt_en` / `prompt_ja` | `inject_choice_ability_context` | **yes** — `ChoiceView.js` |
| `title` | `to_frontend_json` | **yes** — `ChoiceView.js` |
| `description` | `to_frontend_json` | **yes** — `ChoiceView.js` |
| **`choice_maker`** | **resolver.rs:932 only** | **no — nothing, anywhere** |

**Conclusion for Fix 1.** The block at `resolver.rs:924-942` is the *only*
producer of `choice_maker`, and no consumer exists for that field: not
`ChoiceView.js`, not any other file under `web_ui/`, not the Rust display path.
Every field the UI does read is produced by `to_frontend_json` or
`inject_choice_ability_context`, and **both are called by
`get_pending_choice_json`**, which is untouched. Deleting the resolver.rs block
therefore removes no user-visible field.

**Still check before landing:** the native/console UIs (`platforms/*`) consume
the same display payload. I verified the Rust producer chain and `web_ui/`; I did
not audit every platform's own UI code for a raw read of `choice_maker`, though
none of them can read a field the shared Rust producer stops emitting without a
corresponding Rust-side reference — and there is none.

### Per-fix verdict

| Fix | Touches the user-facing game? | Verdict |
| --- | --- | --- |
| **1** dead choice JSON | Yes — the choice prompt payload | **Safe**, conditional on `choice_maker` staying unconsumed. Loss is one field nobody reads. |
| **2** decode-cache counter | No | **Safe.** Counter only, zero behaviour change. |
| **3** TAS scan `SmallVec` | No | **Safe.** Internal, no wire, no prompt. |
| **4** `SmallVec` for `AbilityFilterBranch` / `MemberContribution` | **Both are serialised** — but **safe** | See below. |
| **5** `EffectKind::from_action` | No | **Safe.** Internal enum dispatch. |
| **6** ability identity key | Not the UI, but **gameplay** | **Needs the full gate** — see below. |
| **7** `generate_possible_actions` | **Yes — which actions are offered** | **Needs `total_actions` fixed.** |
| **8** measurement command | No | **Safe.** No behaviour change at all. |

### Fix 4 deserves its own note — it looked like a wire change and is not

Both target types are serialised:

```rust
// core/card.rs
#[cfg_attr(feature = "serde_support", derive(Serialize, Deserialize))]
pub struct AbilityFilterBranch { … }

// core/types.rs
#[cfg_attr(feature = "serde_support", derive(serde::Serialize, serde::Deserialize))]
pub struct MemberContribution { … }
```

So `Vec<T>` → `SmallVec<[T; N]>` *looks* like the §6.1-A2 trap. **It is not, and
the distinction matters:**

- `HashMap<K,V>` → `[T; 8]` changes the JSON **shape** (object → array). Breaks
  the wire. That is why §6.1-A2 is blocked.
- `Vec<T>` → `SmallVec<[T; N]>` serialises as a **sequence in both cases** — the
  JSON is byte-identical. No wire change.

`smallvec/serde` is enabled through `serde_support` (`Cargo.toml:40`, confirmed
resolved in `Cargo.lock`), and if it ever were *not*, the `Serialize` derive would
fail to compile rather than silently changing the output. **Safe by failure.**

### Fixes 6 and 7 are user-facing in the gameplay sense, not the UI sense

Neither touches a rendered field, but both change what the player can do:

- **Fix 6** replaces `format!("{}_{}", card_no, full_text)` with
  `(card_id, ability_index)`. Two abilities sharing a `full_text` **currently
  collide** and the first wins; the key makes that impossible. That is a fix, but
  it changes which abilities activate, so it needs `total_actions` **plus** a
  win-rate comparison. Report the collision separately so the behavioural change
  is reviewable on its own.
- **Fix 7** changes `generate_possible_actions`, which decides what the action
  list contains. `total_actions` is the gate, and the full suite.

Neither may be validated on the test suite alone — `PERF_ENGINE_COMBINED.md` §6
records a scripted match going 28 s → 90 s while `loopback_full_match_stays_in_
lockstep` stayed green, because that test only checks the two link sides agree
with each other, not against a golden.

---

## Sequencing

| order | fix | why here |
| --- | --- | --- |
| 1 | **Fix 8** (measurement) | everything else is measured against it, and it is one line |
| 2 | **Fix 1** (dead choice JSON) | certain, ~118 allocs/choice, essentially zero risk |
| 3 | **Fix 2 counter** | decides whether the project's largest documented win is real |
| 4 | **Fix 4, 5** | mechanical, no design questions |
| 5 | **Fix 7** | biggest per-action budget; needs the trajectory gate |
| 6 | **Fix 3** | needs one more traced run to localise |
| 7 | **Fix 6** | behavioural change; needs the full §1.1 gate |

Do not start Fix 2's *refactor* before its counter reports. That is the mistake
this session already made once, and it is why the counter is step 3 rather than
part of step 3.
