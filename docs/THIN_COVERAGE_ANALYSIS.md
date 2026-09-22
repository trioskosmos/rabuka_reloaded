# Thin-coverage march — status and analysis

Method per card: verify identity + full printed text in `cards/cards.json`, read
every covering test, find the closest well-covered abilities (same clause shapes)
and their test files, write gameplay tests asserting the FULL printed behavior
plus hostile edges, fix engine/parser gaps end-to-end. "Covered" below means
*behavior asserted*, not card-referenced.

Status legend: ✅ asserted / ❌ untested / ⚠️ partially / 🔧 engine or parser fix.

## DONE

### `PL!SP-bp1-009-R` Natsumi (was 1 test → 5)
Full behavior (offer, 1E pay, draw+discard, limit, no re-offer) + hostile edges
(unaffordable, exact cost, next-turn reset, direct double-fire refused at the
resolver). Engine already correct — pure test gap.

### `PL!SP-bp1-010-R` Wien (was 1 → 5) 🔧 ENGINE FIX
Found: mandatory-unpayable activations were offered and fizzled at resolution
(Ok + in-queue error) instead of refused. Fixed with `validate_mandatory_cost`
(upfront, read-only; optional/any-number/max + PayEnergy excluded) wired into
offer generation and `handle_use_ability`. 7 existing tests had encoded the
fizzle — each re-verified against its card text (all mandatory) and converted
to refusal asserts. Skip-at-pay for optional costs (wakana/umi Q228) untouched.

### `PL!S-bp7-011-N` Riko (was ZERO behavior tests → 4)
Card was only mill fodder. Now: all-Aqours (unwait + 2 blades + limit), mixed
(stays wait, no blades), already-wait refused, empty-deck mills-nothing-grants-
nothing (no vacuous bonus). Blade EXPIRY at live end still unasserted (global
gap §7).

### `PL!SP-pb2-020-R` Natsumi yell (was negative-only → 4 + Q264)
Positive path was never executed. Now: discard Liella live → +2 yells
(2 blades each — count carefully), decline, no-eligible auto-skip, use_limit
blocks second yell. Trigger condition is revealed_cards non-empty (Q264).

### `PL!N-PR-025-PR` Setsuna (was 1 direction → 4) 🔧 PARSER + ENGINE FIXES
- Parser: "このメンバーか、ほかのメンバーがバトンタッチして登場" was split so
  the self leg became a static presence check (always true once staged) — plain
  debuts fired the watcher. Fixed with `_fix_distributed_baton_arrival`
  (regen diff: exactly this ability).
- Engine: `just_completed_ability_key` was never cleared after the drain, so no
  turn2+ auto could re-fire across actions. Now cleared at full drain end
  (matches the set/scan/clear discipline in `complete_and_scan`).
- Tests: self arrival, turn2 budget (2 draws, 3rd refused, use count == 2),
  normal debut quiet, opponent-stage arrival quiet (driven via P2's main phase).

### `PL!SP-sd2-006-SD2` Kinako (was 1 → 3)
Kasumi mirror: payment accounting (2E + discard), empty-hand refusal, no-Liella-
live termination (offer+fizzle or no offer, gains nothing).

### `PL!HS-bp6-016-R` Izumi (was 1 → 4)
Payment (4E) + limit pins added; cost-9 member stays (effect fizzles, cost paid —
correct: target absence ≠ cost failure); non-Hasu ignored; full stage
terminates unchanged.

### `PL!SP-bp4-010-R` Wien (was 2 → 4)
Placed-energy orientation asserted waited (not just inferred); already-wait
refused at offer AND activation (THE self_cost threading trap for the new
validator — holds); 0-energy refused without waiting self.

### `PL!N-bp7-023-N` Mia (was 1 → 4)
Natsumi mirror at 2-card scale: limit pins, already-wait refusal, empty-hand
forced full-discard, next-turn fresh-instance offer.

## IN PROGRESS

### `PL!N-bp7-022-N` Shioriko (was accept-only → +6 tests, 3 failing pre-regen)
🔧 PARSER GAP (fix written, awaiting regen): the optional discard + そうしたとき
follow-up were SILENTLY DROPPED — parsed effect is a bare unconditional
change_state (cost null). The old accept test passed vacuously (free reactivate,
no discard ever paid). New handler `_try_discard_hand_reactivate_optional`
(Tier 2, Shioriko-specific per G7/G13/G16 precedent) builds the proven
`sequential + conditional:true` shape (pb2-020 contract): optional hand discard
→ activate the waited Niji member.
- Tests added: decline, non-live-phase, non-Niji, empty-hand auto-skip,
  turn1 second-wait, self-wait. Three fail pre-regen as expected (no prompt /
  free reactivate) — they are the fix's acceptance criteria.
- Residual imprecision (documented, not fixed): with MULTIPLE waited Niji the
  consequence filters + prompts instead of tracking the exact trigger subject
  (`triggering_member_id` is dead plumbing — no scan populates it). Single-
  waited case (the realistic one) auto-resolves correctly.

## REMAINING

### `PL!S-bp7-020-L` HAPPY PARTY TRAIN (L2, 4 tests — best of the nine)
Both abilities have positive + negative pins. Missing:
- ❌ STACKING: A and B both true → reduced twice? Combined value unasserted.
- ❌ B with EMPTY deck → mill-shortfall + condition-on-no-card path.
- ❌ A with opponent waited but own all active → own-stage gate precision.
- ❌ reduction honored at performance end-to-end (modifier asserted only).

## Open questions (under investigation)
- q94/q171 + s2_pb1_006 failures (`left: 4, right: 2` — double blades): did the
  just_completed fix expose tests that encoded the old suppression, or over-fire?
  Card texts pending verification.
- PR-025 budget test fails ONLY in full-suite parallel runs
  ("Only member cards can be placed on stage") while passing targeted —
  possible shared-global contamination or order dependence. Pending.

## Overall testing gaps (beyond the 9 cards)

1. **Fodder-only coverage counts as covered.** bp7-011 proved L0 ("card
   referenced") can mean "mill seed". Trust `depth`, not L0.
2. **Mandatory-cost refusal is brand new (`651a2d78`).** Wait-self / discard /
   energy costs need the already-paid-state case everywhere; opportunistic
   conversions done, no systematic pass yet.
3. **Filter-aware validation still raw-count.** `validate_mandatory_cost`
   counts zone cards without cost filters ("discard a Liella! card" + 3
   non-Liella validates Ok, fizzles at pay). Deferred follow-up.
4. **Optional-decline (`てもよい`) matrix is per-card ad hoc.** Accept usually
   tested; decline often not.
5. **Exact-cost / unaffordable edges exist for 3 cards.** Highest-value cheap
   asserts for every activation card.
6. **Hostile double-fire pinned for 3 cards** (Kasumi, Natsumi, Wien-010) +
   jidou-budget pins (PR-025, pb2-020). `use_limit_gate` path still deserves a
   systematic pass.
7. **Duration expiry almost never asserted** ("until live end" gains checked at
   grant, not expiry).
8. **Deck-shortfall paths** covered sporadically (bp4-010 empty energy deck ✔,
   Riko empty deck ✔) but not systematically.

March order executed: bp7-011 → pb2-020 → PR-025 → sd2-006 → bp6-016 →
bp4-010 → bp7-023 → bp7-022 → (remaining) bp7-020.

---

## Energy work (user directive: card identity doesn't matter, but for COSTS
## active-vs-wait is the main factor — menus must respect it)

Model today: `EnergyZone { cards, active_energy_count }` with the positional
convention "indices `[0..active)` are active" maintained by hand (~78 readers,
~12 writers), plus a parallel per-card orientation-mod system. Nothing
reconciles the two.

Changes (all behavior-preserving except stated):
- `util::{active,waited}_energy_indices` — one shared definition of the
  offerable sets; any-number pay menus (cost + reprompt) unified onto it.
- Under-member placement menu now states pile composition
  ("(active: A, waited: W)" EN+JA); both states remain selectable (correct).
- Deactivate menu offers ACTIVE-only indices + shows counts (was: any matched
  index, including already-waited).
- Counter honesty in selection handlers: removing waited cards no longer
  decrements; tapping already-waited is a no-op for the count (was: blind
  subtract → desync, spendable phantom energy).
- Activate auto-pick takes waited-only (was: first-N matched regardless of
  state — could "activate" already-active cards and overcount past the
  number of cards in the zone).

Fallout (in progress): ~12 existing activation tests pinned the phantom
overcount (active counter exceeding cards in zone, e.g. 5 active of 3 cards).
As-written, "activate N" with nothing waited is a no-op (same principle as
Q137 for wait). Fixing those tests to stage genuinely-waited energy so they
pin REAL activation instead of asserting no-ops.

---

## Appendix: stupid in the repo (verified, ranked by danger)

Found while chasing the budget-test flake (test-harness id allocator handing
out colliding/data-less ids). All read-verified, 2026-09-21.

### Actually dangerous
1. **Two null conventions for cards** — `Option<i16>` in APIs but `-1`
   sentinels in `[i16; 3]` stage slots, so every stage scan needs a manual
   `id != -1` filter (`zones.rs:385`, `condition/card.rs`, everywhere). One
   forgotten filter evaluates ghost card `-1`.
2. **Energy is two coupled fields** — `EnergyZone { cards,
   active_energy_count: u8 }` (`zones.rs:593`). Nothing enforces
   `active ≤ len`; `saturating_add/sub` clamp silently; `set_active_count`
   sets anything. Even test setup maintains the invariant by hand.
3. **Prohibition is substring matching** — `is_action_prohibited` does
   `prohibition_effects.iter().any(|e| e.contains(action))`
   (`tracking.rs:114`). Over-broad strings ban more than written.
4. **Identity fallbacks iterate HashMaps** — `get_card_id` steps 3/5
   (`card.rs:593,604`) return first `starts_with`/`contains` match in
   RandomState order: which card you get varies per process on mismatch.
   Exact keys are deterministic; sloppy input resolves to a random card.
5. **Four ability representations must agree** — JP text → `abilities.json` →
   bytecode blob → decoded structs via generated files. Build warns of
   staleness; drift means playing last week's cards.
6. **Trigger-guard lifetimes are tribal knowledge** —
   `just_completed_ability_key` + `this_batch_triggered_ability_ids` +
   recently-moved/appeared/state cleared in ~7 places with different scopes.
   Turn-2 refires were silently swallowed by it (fixed via batch-scoped
   guard + `just_completed_moved`).

### Fragile by design
7. **~440 `Option` fields** across `Ability`/`AbilityEffect` (`card.rs`) —
   every mechanic is `None`-means-default folklore.
8. **Zone names are aliased strings** — `"waitroom"`/`"discard"`,
   `"energy"`/`"energy_zone"` matched ad hoc per site despite the enum.
9. **Two clocks + attacker flags** — `Phase` (~10 variants) + `TurnPhase` +
   `is_first_turn`/`turn_number`/first-second-attacker bools.
10. **Global `GAME_STATE: Mutex<Option<…>>`** (`main.rs:31`).
11. **Test-id allocator** — pool of 11 + synthetic fallback ids with no card
    data; pre-seed fix they collided with random pool cards per process
    (the budget flake). Now seeded past the pool; still data-less past 11 —
    tests must keep same-card `new_id` under pool size (only the budget
    test violated it: 24 burns).

### Fix first, in order
Energy per-card state → `-1` sentinel removal → typed prohibition → trigger
event identity. (1+2 mechanical; 4 surgery.)
