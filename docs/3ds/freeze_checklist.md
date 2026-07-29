# 3DS Port Freeze — Possible Causes Checklist

**Symptom:** When playing a card with a kidou ability to stage on the 3DS port, the game freezes — only the "pass" option is visible and no button presses work. The web server works fine.

---

## 1. `settle_3ds` loops too long / watchdog timeout

- `settle_3ds` loops up to 500 iterations, calling `advance_phase` for automatic phases
- It yields to `_3ds_main_loop()` (aptMainLoop) every 10 iterations
- If `advance_phase` keeps creating automatic phases without hitting a pending choice, it loops 500 times
- The 3DS OS watchdog may time out if aptMainLoop isn't called frequently enough
- **Hypothesis:** `advance_phase` transitions through automatic phases (Active → Energy → Draw → Main) and during one of these transitions, a kidou ability's auto-trigger creates a pending choice, but `has_pending_choice()` doesn't detect it correctly on 3DS

## 2. Pending choice not detected by `has_pending_choice()` on 3DS

- `has_pending_choice()` checks `ability_queue.is_waiting_for_choice().is_some()`
- If the pending choice is created but `is_waiting_for_choice()` returns `None` on 3DS, the game would try to auto-settle instead of showing the choice overlay
- **Hypothesis:** The ability queue's pending choice state is not properly synchronized on 3DS, possibly due to a difference in how the `no_std`/`std` feature flags affect the ability queue implementation

## 3. `generate_possible_actions` returns only Pass when a pending choice exists

- When `has_pending_choice()` is true, `generate_possible_actions` should return choice actions (e.g., `ChoiceOption` for `SelectAutoAbility`)
- If it returns only the Pass action, the acts_cache would show only "pass"
- **Hypothesis:** `generate_possible_actions` has a bug where it doesn't correctly handle `SelectAutoAbility` or other choice types on 3DS, possibly due to a missing `#[cfg(feature = "3ds")]` conditional or a platform-specific difference in how the choice is constructed

## 4. `choice_image_mode` rendering code has a 3DS-specific bug

- When `choice_image_mode` is true and there's a pending choice, the rendering code at lines 4787-4821 draws the choice grid
- The code accesses `gs.ability_queue.current_entry()`, `gs.get_pending_choice()`, and iterates over `display_order` and `acts_cache`
- **Hypothesis:** The rendering code has a null pointer dereference, division by zero, or out-of-bounds access that only manifests on 3DS hardware (e.g., the `opt_map` HashMap lookup fails, or `display_order` is empty when it shouldn't be)

## 5. Input handling for choices is broken on 3DS

- The DPAD navigation code (lines 2640-2735) and touch input code (lines 3467-3533) handle choice selection
- If the input handling doesn't work, the player can't interact with the choice overlay
- **Hypothesis:** The touch input coordinates are wrong when a pending choice exists, or the DPAD navigation code has a logic error that causes `cur` to always point to the Pass action

## 6. `_3ds_main_loop()` (aptMainLoop) not called frequently enough

- The 3DS OS requires `aptMainLoop()` to be called regularly to prevent watchdog timeouts
- The main loop calls it once per frame, but the Step::Play handler is very large (6000+ lines)
- On 3DS hardware (ARM11 ~268 MHz), the Step::Play handler might take too long per frame
- **Hypothesis:** When a pending choice exists, the rendering code does extra work (drawing the choice grid, highlighting cards) that causes the frame to take too long, triggering the 3DS OS watchdog

## 7. `acts_cache` not regenerated after pending choice is created

- `acts_cache` is regenerated when `dirty || redraw` is true (line 3785)
- After an action is executed, `dirty = true` and `redraw = true` are set (line 3081)
- If these flags are not set correctly after a pending choice is created, the old acts_cache (with only Pass) would persist
- **Hypothesis:** The `dirty` and `redraw` flags are not being set to true after `process_pending_auto_abilities` creates a pending choice, so the acts_cache is not regenerated with the choice actions

## 8. `advance_phase` creates an infinite loop of automatic phases

- If `advance_phase` transitions from one automatic phase to another without ever reaching a non-automatic phase, `settle_3ds` would loop 500 times
- After 500 iterations, it breaks and returns, but the game state might be inconsistent
- **Hypothesis:** A kidou ability's effect causes `advance_phase` to keep cycling through automatic phases (e.g., Active → Energy → Draw → Main → Active → ...) without ever reaching a state where `has_pending_choice()` is true

## 9. Memory exhaustion on 3DS hardware

- The 3DS has limited shared RAM (~256MB)
- The choice grid rendering code allocates HashMaps, Vecs, and Strings for each frame
- On 3DS hardware, memory allocation might fail or be very slow
- **Hypothesis:** The `opt_map` HashMap or other allocations in the choice rendering code fail on 3DS due to memory pressure, causing the game to freeze

## 10. The `handle_use_ability` function blocks or loops on 3DS

- `handle_use_ability` calls `trigger_auto_ability` and `process_pending_auto_abilities`
- If `process_pending_auto_abilities` gets stuck in a loop on 3DS (e.g., due to a platform-specific difference in the ability queue), the game would freeze
- **Hypothesis:** The `process_pending_auto_abilities` function has a loop that doesn't terminate on 3DS because the `MAX_AUTO_RECURSION` guard or the `reprocess_counts` guard doesn't work correctly on 3DS

---

## Most Likely Causes (ranked by probability)

1. **#3** — `generate_possible_actions` returns only Pass when a pending choice exists (explains "only pass visible")
2. **#7** — `acts_cache` not regenerated after pending choice is created (same symptom as #3)
3. **#4** — `choice_image_mode` rendering code has a 3DS-specific crash (explains freeze)
4. **#6** — `_3ds_main_loop()` not called frequently enough (explains 3DS-specific freeze)
5. **#2** — Pending choice not detected by `has_pending_choice()` on 3DS (explains why choice overlay doesn't appear)

---

## Recommended Debugging Steps

1. Add debug output (`_3ds_debug_print`) to `settle_3ds` to log when it's called and how many iterations it runs
2. Add debug output to `generate_possible_actions` to log what it returns when a pending choice exists
3. Add debug output to the choice rendering code to verify it's being reached
4. Test with `choice_image_mode` toggled off (R button) to see if the text action list shows the choice options
5. Compare the `has_pending_choice()` result on 3DS vs web server after activating a kidou ability