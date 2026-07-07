# Rabuka 3DS — Hang Debugging at Phase::Active / recalculate_constants

## Symptoms

- Game proceeds through setup correctly
- First time `Phase::Active` is reached, the game hangs
- `[LOOP] phase=Active` is printed on top screen
- `PHASE_ACTIVE:1 reset_keyword_tracking OK` is printed
- `PHASE_ACTIVE:2 recalculate_constants OK` is NEVER printed
- No debug output from inside `recalculate_constants()` appears (despite `tdbg!` calls)
- Confirmed: the hang is INSIDE `GameState::recalculate_constants()`

## Execution Context

```
settle_3ds()                              # engine_3ds/src/bin/rabuka_3ds.rs:465
  └─ advance_phase(gs)                    # engine/src/turn/phases.rs:62
       └─ Phase::Active arm               # phases.rs:81
            ├─ reset_keyword_tracking()    # OK — returns
            ├─ recalculate_constants()     # HANGS — never returns
            └─ (never reached)
```

## Research Findings

### 1. Cooperative Threading Model

Horizon OS (3DS) uses `SCHED_FIFO` — threads only yield when they explicitly call an OS function
(`aptMainLoop()`, `svcSleepThread()`, `svcWaitSynchronization()`, etc.). There is NO preemption.

**Impact**: If `recalculate_constants()` calls any function that internally waits on a
synchronization primitive (Mutex::lock, etc.), and that primitive was never released, the thread
hangs permanently.

Source: https://github.com/rust3ds/ctru-rs/wiki/System-Flaws
> "Horizon OS uses a cooperative threading model... without any sort of yielding, the loop will
> permanently lock the main thread! Beware of deadlocks."

### 2. AtomicBool on ARMv6K (3DS)

The 3DS CPU is ARM11 MPCore (ARMv6K). Rust's `AtomicBool` is a 1-byte atomic type.

- ARMv6 introduced `ldrex`/`strex` for 32-bit word atomics (available)
- ARMv6K adds `ldrexb`/`strexb` for 8-bit byte atomics (should be available on ARM11 MPCore)
- ARMv7 makes byte/halfword atomics mandatory

**However**: The Rust target spec for `armv6k-nintendo-3ds` may not properly enable the `+v6k`
feature flag in LLVM for atomic codegen. If LLVM doesn't know about `ldrexb`/`strexb`, it falls
back to a `__sync_` or `__atomic_` library call. On the 3DS, these library functions may not exist
or may crash (since devkitARM doesn't ship libatomic).

**Evidence from embedded-hal issue #598**: `AtomicBool::compare_exchange` is documented as not
working on ARMv6-M targets. A similar issue likely affects ARMv6K for certain atomic operations.

The first line of `recalculate_constants()` is:
```rust
if crate::ability::debug::ABILITY_DEBUG.load(std::sync::atomic::Ordering::Relaxed) {
```

`ABILITY_DEBUG` is `static AtomicBool`. On ARMv6K without proper LLVM target features, this
`load(Relaxed)` may compile to a library call that deadlocks or crashes.

**Fix applied**: `#[cfg(not(feature = "3ds"))]` gated the AtomicBool load. On 3DS, it's skipped.

### 3. Stack Overflow on 3DS

The 3DS default thread stack size is typically 32KB (0x8000) or 64KB (0x10000). The
`recalculate_constants()` function:
- Is ~530 lines of Rust code
- Creates ~10 HashMap/Vec/String local variables (each with ~40-56 byte stack footprint)
- Calls deeply into `ConditionContext::evaluate_condition()` which can recurse through
  ability evaluation chains
- Contains a `for` loop over `entries` that calls `collect_constant_stage_effects()` and
  creates `AbilityResolver` instances on the stack

If the total stack usage exceeds the thread's stack limit, the ARM11 CPU raises a Data Abort
exception. Horizon OS's default exception handler for userland applications is not user-visible —
it effectively hangs the process without any error message.

**Suspect**: The function creates local `HashMap` objects in release mode. Even though HashMap
data is heap-allocated, the HashMap handle struct (~48 bytes) plus the surrounding loop state
and deeply nested evaluator calls could overflow a small stack.

### 4. `_3ds_tdbg` C Function Not Producing Visible Output

The `tdbg!` macro in `modifiers.rs` calls `_3ds_tdbg()` which does:
```c
void _3ds_tdbg(const char *msg) {
    consoleSelect(&top_console);
    printf("%s\n", msg);
    consoleSelect(&bot_console);
}
```

Despite this, no RC:0..RC:14b markers appear on screen. Possible reasons:
- The function never executes (stack overflow in prologue)
- `printf` is buffered and never flushed before the hang
- `consoleSelect` doesn't actually redirect `printf` on the 3DS (console uses a custom FILE
  that may not be controlled by `consoleSelect` on all builds)

## Current Diagnostic Approach

### Stub Strategy

All 8 calls to `recalculate_constants()` in `phases.rs` are replaced with `tdbg!` markers:
```
PHASE_ACTIVE:2 RECALC_SKIPPED    # Phase::Active
PHASE_ENERGY:1 SKIPPED            # Phase::Energy
PHASE_DRAW:1 SKIPPED              # Phase::Draw
PHASE_MAIN:1 SKIPPED              # Phase::Main
PHASE_LIVE:1 SKIPPED              # LiveCardSetSecondAttacker
PHASE_EXEC:1 SKIPPED              # execute_main_phase_action
PHASE_AUTO:1 SKIPPED              # process_pending_auto_abilities
PHASE_AUTO2:1 SKIPPED             # process_pending_auto_abilities
```

If the game proceeds past `PHASE_ACTIVE:2 RECALC_SKIPPED`, the hang is CONFIRMED inside
`recalculate_constants()`.

### Next Steps (when confirmed)

1. **Reduce stack pressure**: Split `recalculate_constants()` into smaller sub-functions
   that don't create all HashMaps/state on a single stack frame
2. **Verify ARMv6K atomics**: Use `AtomicU32` instead of `AtomicBool` everywhere on 3DS
   (32-bit atomics use native `ldrex`/`strex` which work on ARMv6K)
3. **Add explicit yields**: Call `aptMainLoop()` periodically inside long-running functions
   to keep the OS watchdog happy and test cooperative threading assumptions
4. **Try debug build**: A non-optimized build has different stack layout and might not
   trigger the overflow

## Key Files

| File | Purpose |
|------|---------|
| `engine/src/core/game_state/modifiers.rs` | Contains `recalculate_constants()` — THE HANG LOCATION |
| `engine/src/turn/phases.rs` | Contains `advance_phase()` — calls recalculate_constants |
| `engine_3ds/src/bin/rabuka_3ds.rs` | Main loop, `settle_3ds()`, screen selection |
| `engine_3ds/src/ctru_shim.c` | C shim — `_3ds_init`, `consoleSelect`, `_3ds_tdbg` |
| `engine/src/ability/debug.rs` | `pub static ABILITY_DEBUG: AtomicBool` |
| `engine_3ds/build.rs` | Build script — compiles C shim, links libctru |
