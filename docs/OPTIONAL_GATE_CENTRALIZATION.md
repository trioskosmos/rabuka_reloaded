# Optional-effect gate centralization

Status: DONE (single-gate + shared prompt helper), with open design notes on the
self-gating allowlist (see "The allowlist is the weak point").

## The problem

`AbilityEffect.optional: Option<bool>` (`engine/src/core/card.rs`) is a shared
field carried by **every** effect variant. Semantically it means "置いてもよい /
〜してもよい" — the player may *choose* to do the effect or skip it. But nothing
honored it centrally; each handler independently decided whether to offer a
Skip/Do prompt, so the "may" was silently dropped on effects whose handler never
read `optional`.

## What was built

### 1. Central gate (`engine/src/ability/effects/mod.rs`)

Immediately before the `match action_type` dispatch in `execute_effect`:

```
if effect.optional == Some(true)
    && !is_optional_self_gating_action(action_type)   // allowlist
    && self.offer_optional_skip(gs, "効果を実行しますか？（オプション）")
{
    let mut eff = effect.clone();
    eff.optional = Some(false);
    self.pending_optional_effect = Some(eff);          // exact clone for re-entry
    return Ok(());
}
```

Any effect with `optional:true` that is **not** in the self-gating allowlist gets
one Skip/Do prompt. On **accept**, `handle_optional_cost_payment`
(`cost.rs`) re-executes the stored clone (`optional` already forced off) → the
handler runs non-optionally, not re-gated. On **skip** it does nothing.

### 2. Generic accept re-entry (`cost.rs`)

`handle_optional_cost_payment` branches first on `pending_optional_effect`:
skip clears state + resumes; accept runs `execute_effect(gs, &eff)` on the stored
clone, then resumes. This replaced the `PlaceEnergyUnderMember`-only `is_effect_optional`
re-entry. The result uses the existing `ChoiceResult::Skip` /
`ChoiceRoute::OptionalCost` / `pay_optional_cost:skip_optional_cost` machinery.

### 3. Shared prompt helper (`resolver.rs` `offer_optional_skip`)

One builder for the Skip/Do prompt that also records `choice_card_no =
Some(ChoiceRoute::OptionalCost)` and returns `false` (don't emit) if an optional
prompt is already pending. Used by the central gate **and** the two branch-specific
misc.rs gates (reveal deck-top, placement `energy_deck`), removing three identical
~10-line blocks.

## What was intentionally NOT centralized

- **Branch-specific gates stay in `misc.rs`** (reveal deck-top; `place_energy_under_member`
  `energy_deck`). They trigger on a *source/destination condition inside a handler*,
  not on a whole action type. A whole-action gate cannot reproduce that; the top-level
  `PlaceEnergyUnderMember` is also in the self-gating list, so its own path handles it
  (this is the card the original `bp7` test covers).
- **Optional *cost*** (`cost.rs`) is a separate axis (pay-or-skip a cost) and stays
  in the cost engine.
- **Mid-flow select/reveal** (`look.rs`, `compound.rs`) — "look at N, may select up to M".
  These are granular, not whole-effect skips; a single gate would change their behavior.

## Resource / speed analysis (does this cost anything?)

**No meaningful cost; it's cheaper over time.**

- The gate is **O(1)** per dispatch: `matches!` lowers to a jump-table compare and
  `offer_optional_skip` short-circuits on an `Option` equality before allocating.
- **Zero allocation in the hot path.** The only `effect.clone()` happens when the
  player *accepts* a genuinely-optional leaf effect — a rare, human-timed event,
  once per turn at most.
- Proportional to number of effects dispatched (tiny per turn); dwarfed by zone
  lookups / card matching / modifier recalcs.
- Net: fewer duplicated branches and prompt sites than before → less code to
  maintain and fewer procedure points.

## The allowlist is the weak point

`is_optional_self_gating_action` is a hardcoded **negative** allowlist
(opt-out: "gate everything except these"). That aggressive default is what
auto-catches dropped `optional`, but it means: **every time a new self-gating action
type is added, someone must remember to add it here**, or the gate double-prompts
it. This produced the 55 → 6 → 3 failure cascade during implementation
(`PositionChange`, `ChangeState`, `DrawCard`, `InvalidateAbility`, … all had to be
added as the tests surfaced them).

Alternatives, best first:

- **Co-located marker (recommended).** Each handler exports a small constant
  (`const SELF_GATES_OPTIONAL: bool`) or `fn`, so the "does this handler read
  `optional`" knowledge lives next to the code that must stay consistent with it.
  The gate stays central; the policy moves to where the risk is. Mechanical to
  apply, no behavior change.
- **Exhaustive + disjoint test (guardrail).** A test enumerating *all* `ActionType`
  variants asserting each is exactly "self-gating" or "centrally gated". Adding a
  variant then forces a conscious classification instead of silent drift. Cheap;
  pair with the marker refactor.
- **Positive allowlist (opt-in).** Gate only known-safe leaf types. Cannot
  double-gate any handler, but also stops auto-catching the original defect for
  future cards — a new leaf that drops `optional` silently regresses. Trade safety
  for automatic coverage.

Status: the hardcoded list is currently in place; the co-located-marker refactor
(+ exhaustive test) is the recommended follow-up.

## Tests

- `bp7_ai_energy_under_member_optional_test.rs` — accept places, skip does nothing,
  solo-虹ヶ咲 target works.
- Full engine suite (`cargo test -p rabuka_engine`): 2235 passed / 0 failed.