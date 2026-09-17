# Repository audit — 2026-09-17

## Scope and status

Requested deliverable: document defects, inefficiencies, maintenance hazards, and confusing design **without applying fixes**. This report is the only intended repository write. No builds, tests, generators, deployments, or remediation scripts were run for this audit. Three reviewers disclosed temporary inventory listings outside the repository in the preapproved temporary directory; no implementation/probe files were created by any reviewer, and no repository code was changed by the audit.

**Status: first consolidated pass.** This is **not** an exhaustive file-by-file audit. Findings are source-inspection based; no runtime reproduction was performed. Coverage ledgers distinguish full reads, targeted reads, mechanical inventories, generated assets, and unreviewed files; an inventory entry is not a clean bill of health. Substantive areas left unaudited include large parts of the Python card parser, most UI rendering/modals/CSS, repetitive engine test bodies, C engine ability/turn implementations, generated asset consistency, and all hardware/toolchain behavior. A C reviewer additionally reported findings targeting stale generator paths, CD-i streaming tables, Genesis stack sizing, and test selection handling; those were not yet tabulated here and remain in the reviewers' results.

Initial tracked inventory: **4,848 paths**. Root files: 12; `.github`: 5; `ai_design`: 2; `android`: 5; `cards`: 45; `docs`: 19; `engine`: 921; `engine_c`: 70; `pc_relay`: 3; `platforms`: 379; `tools`: 24; `training`: 2; `web_ui`: 3,361. This includes 3,258 frontend images, 36 object files, four executable paths, and at least 65 files in explicitly named vendor directories. These counts describe tracked paths, including files deleted in the working tree, not files semantically reviewed.

Excluded from first-party source review: `.git`, nested `.kilo/worktrees`, ignored toolchain/build caches, downloaded `research` trees, source-image collections, and local test-output dumps. Generated tables/data, images, binaries, and third-party sources are identified separately rather than represented as manually verified.

Snapshot caveat: the checkout already contained modifications to bot code, C engine code, documentation, binaries, and untracked analysis scripts. Additional files appeared during inventory. Findings describe the inspected working-tree versions, not a frozen commit. Initial HEAD: `a20f5c19`; a later read-only check showed `1ab04279` after intervening commits, with parser/test/C changes still pending. These concurrent changes were not made by the audit coordinator. File references must be rechecked against any later checkout before remediation.

Severity: **High** = core correctness/build/data-loss hazard; **Medium** = materially wrong behavior, reliability/performance or verification gap; **Low** = bounded quality/documentation issue. Confidence is source-level unless stated otherwise. Suggested directions are notes only, not work performed.

## Build, CI, launchers, and repository hygiene

### B01 — High — Fresh-checkout engine CI lacks required generated inputs

- Evidence: `.github/workflows/engine-tests.yml:25-42` checks out the repository, restores an optional cache, and runs Cargo without generating card/deck artifacts. `engine/src/lib.rs:51` unconditionally includes the deck module; `engine/src/decks_cards_gen.rs:8-40` embeds `engine/baked/decks/*.bin`. `.gitignore:182-186` excludes those blobs, and `git ls-files engine/baked` lists JSON data but no deck binaries.
- Impact: a clean checkout cannot satisfy these `include_bytes!` paths; the Cargo target cache does not create the missing source inputs. The documented test command also assumes unmentioned preparation.
- Direction: define and invoke one reproducible artifact-preparation step before clean builds/tests. Not executed here.

### B02 — Medium — Ordinary Cargo builds rewrite tracked test source and suppress filesystem errors

- Evidence: `engine/build.rs:16-80,97-105` recursively regenerates test `mod.rs` files during every applicable build-script invocation; line 66 discards write errors. `read_dir(...).ok()?` and `entries.flatten()` at lines 18-21 also hide traversal failures.
- Impact: building the application can change the working tree; a read-only or partly inaccessible test tree can silently retain stale module declarations or omit newly discovered tests. Build-script work is unrelated to whether the current target is a test.
- Direction: separate generation/checking from normal source-tree mutation, and surface failures. No build was run because it would violate this audit's no-code-change constraint.

### B03 — Medium — Claimed generated-artifact freshness enforcement is not enforcement

- Evidence: `engine/build.rs:127-181` uses modification times, sets `stale`, then discards it. Lines 155-164 search the entire manifest text for an input byte-length string rather than verifying a digest. `cards/build/generation_manifest.json:5-16` has input hashes and output byte counts, not the claimed input-size field. `.github/workflows/engine-tests.yml:40-42` does not turn these warnings into failures.
- Impact: stale artifacts can compile; freshly checked-out timestamps can create misleading warnings. Comments claiming hard errors/CI denial overstate the actual protection.
- Direction: compare declared input/output hashes and make failure policy explicit; eliminate the unrelated size-substring heuristic.

### B04 — Medium — Docker dependency-cache stage is defeated by copying the whole repository first

- Evidence: `Dockerfile:14-23` copies the entire repository before the supposed dependency-only layer. `src` therefore already exists; `mkdir src` fails, the `|| true` branch hides that failure, and the trailing `rm -rf src` still runs under shell left-associative `&&`/`||` evaluation. Real sources are copied again at line 26.
- Impact: the intended warm-up build is skipped and unrelated repository changes invalidate the dependency layer. Errors are deliberately hidden. This finding is an efficiency/reliability issue, not a claim the final build always fails.
- Direction: construct a real dependency-only stage with explicit successful commands and without unconditional error suppression.

### B05 — Medium — Windows launcher builds WASM into one target directory and reads another

- Evidence: `start.bat:21-22` exports `CARGO_TARGET_DIR=C:\rust_targets`; the child WASM Cargo build at lines 68-69 inherits it. The binding command at line 75 instead reads `platforms\wasm\target\wasm32-unknown-unknown\release\rabuka_wasm.wasm`. The inspected WASM Cargo config supplies linker flags, not a target-directory override.
- Impact: the launcher either fails to refresh bindings or feeds an old local WASM artifact to wasm-bindgen while the new artifact resides elsewhere. The warning path continues running the server.
- Direction: use the same explicit target directory for compilation and binding generation.

### B06 — Withdrawn after verification — Rust action target input is supported

The initial suspicion about `.github/workflows/deploy-pages.yml:24` was disproved: the upstream `dtolnay/rust-toolchain` action definition explicitly accepts `target` as an alias of `targets` and uses either input. This is **not a finding**. Upstream action metadata was fetched read-only during the audit; no workflow change is warranted on this basis.

### B07 — Medium — Server deployment triggers omit inputs copied into the server image

- Evidence: `.github/workflows/deploy-server.yml:6-11` and `.github/workflows/deploy-fly.yml:6-11` omit `web_ui/**` and `tools/**`; `Dockerfile:32,49,52` consumes the deck-baking tool, frontend, and deck files.
- Impact: frontend/deck/tool-only commits can leave the hosted backend image stale even though they change image contents or generation behavior.
- Direction: align deployment path filters with the Docker build's actual inputs.

### B08 — Medium — Deployment success can be reported for a rejected Render API request

- Evidence: `.github/workflows/deploy-server.yml:48-51` uses plain `curl` without an HTTP-error failure option or response validation.
- Impact: an HTTP 4xx/5xx response can leave the step green although no deployment started. Network failures still produce nonzero exit status; this finding concerns HTTP responses.
- Direction: fail on unsuccessful HTTP status and validate the deployment response.

### B09 — Medium — Fly deployment has no checked-in application configuration

- Evidence: `.github/workflows/deploy-fly.yml:27` runs `flyctl deploy` from the root without `--app` or `--config`; inventory and a `**/fly.toml` search found no Fly configuration.
- Impact: a fresh runner lacks the application/service/port configuration normally required by this command. A token alone does not identify these settings in the workflow.
- Direction: make application identity and deploy configuration explicit. Not exercised against Fly.

### B10 — Medium — CI path filters do not cover all generator changes they purport to verify

- Evidence: `.github/workflows/coverage.yml:5-23` names the condition decoder generator but not `cards/generate_effect_decoder.py`, although lines 55-58 check the latter's output. The workflow itself and `cards/cards.json` are also absent. `.github/workflows/engine-tests.yml:5-17` omits several compiler/parser/build-data inputs under `cards`.
- Impact: changes to an omitted input alone can skip the relevant verification job.
- Direction: derive trigger paths from the pipeline dependency graph or use broader directory coverage.

### B11 — Medium — Docker context admits local metadata and heavyweight unrelated trees

- Evidence: `.dockerignore:1-33` omits `.git`, `.kilo`, `.opencode`, and `research`; `Dockerfile:15` copies the complete admitted context.
- Impact: local Docker builds can transfer unrelated history, session/worktree material, and downloaded research into the build context/intermediate layer; this also wastes I/O/cache space. This does not establish that any secret is present or published.
- Direction: explicitly limit build inputs and exclude local-only metadata/reference trees.

### B12 — Medium — Default Docker image omits the card images advertised by its standalone quick start

- Evidence: `.dockerignore:1-2` removes card WebPs; `README.md:77-82` presents a standalone Docker build/run without documenting the separate asset host required by that exclusion.
- Cross-reference: `web_ui/js/constants.js:48-70` preserves local `img/` paths rather than routing them to a CDN; `web_ui/js/state.js:381` derives local card-WebP mappings and `web_ui/js/components/CardRenderer.js:226` constructs local WebP candidates.
- Impact: the standalone image omits assets the frontend requests locally. Static Pages hosting has those images, but that does not satisfy the advertised standalone Docker path.
- Confidence: source-level integration defect; no container/browser reproduction performed.

### B13 — Medium — Auto-continue script types into an unverified foreground window

- Evidence: `auto_complete.bat:25-36` ignores `AppActivate`'s success result, sends Escape, raw message text, and Enter; the default target title is empty (line 11). The loop repeats indefinitely (lines 52-55).
- Impact: changing focus or a failed activation can submit a long instruction to an unrelated application. `SendKeys` also interprets special characters in custom text rather than guaranteeing literal entry.
- Direction: require verified target identity and safe literal text delivery; never rely on focus alone.

### B14 — Low — Root dependency audit is an empty scan presented as an audit

- Evidence: `DEPENDENCY_AUDIT.md:7-16` reports zero files and zero functions, then zero missing/unresolved symbols. The repository inventory contains a substantive `engine_c` tree.
- Impact: the artifact conveys no dependency assurance and can be mistaken for a clean result.
- Direction: mark failed/empty collection distinctly and retain one clearly scoped authoritative report.

### B15 — Low — README's benchmark and installation guidance is stale

- Evidence: `README.md:73-74,180` says `cargo bench`, but `engine/Cargo.toml:125-131` requires feature `profiling` for the performance bench. `README.md:88` contains a literal placeholder repository owner; line 135 lists a directory absent from the tracked root inventory. Lines 145 and `engine/Cargo.toml:41` describe xorshift64 while `engine/src/rng.rs:14-23` implements xorshift32.
- Impact: copy/paste setup can fail, benchmark commands skip the intended target, and architecture descriptions mislead maintenance.
- Direction: validate documented commands and paths against the current tree.

### B16 — Low — AGENTS contains contradictory recovery instructions

- Evidence: `AGENTS.md:10` prohibits Git reversion unless asked, while line 34 prescribes stash/checkout as baseline verification. Line 21 mandates fixing every gap, conflicting with a documentation-only engagement unless the current user request is explicitly given priority.
- Impact: repository guidance can drive unintended working-tree changes or scope expansion.
- Direction: state precedence and safe read-only baseline alternatives. No stash, checkout, or code fix was performed here.

## Shared Rust infrastructure

### R01 — Medium — Disabled profiling still obtains a clock timestamp on every timer construction

- Evidence: `engine/src/timer.rs:31-36` calls `Instant::now()` when `profiling` is off. Numerous hot-path entry points construct timers, including phase advancement, timing checks, action generation, and live performance.
- Impact: nonprofiling gameplay/simulation still pays clock-call overhead; exact runtime cost has not been benchmarked.
- Direction: compile timestamp storage/construction out when profiling is disabled.

### R02 — Medium — Profiling call stack is process-global rather than thread-local

- Evidence: `engine/src/timer.rs:14-15,38-40,55-71` shares one mutex-protected label stack across all threads. Drop pops only a matching last label; path capture and pop take separate locks.
- Impact: overlapping threads can be recorded as nested calls, and out-of-order drops leave stale frames. Locking prevents a data race but not semantic corruption of the profile.
- Direction: keep per-thread/per-task stacks and aggregate samples separately.

### R03 — Medium — Flamegraph output sums inclusive durations as if they were exclusive samples

- Evidence: `engine/src/timer.rs:53,74-81` records each timer's full elapsed duration, including children; lines 97-109 sum these inclusive values and lines 125-133 emit each as a folded-stack weight.
- Impact: nested work is counted multiple times in percentages/flamegraph totals, skewing optimization decisions.
- Direction: track child elapsed time or use a sampling/exclusive-time representation; label inclusive reports accurately.

### R04 — Medium — Allocation guard's reported interval peak is not an interval peak

- Evidence: `engine/src/alloc_counter.rs:104-112,135-142,158-166` subtracts two lifetime high-water marks to report `peak bytes`.
- Impact: a period that allocates heavily but never exceeds a prior lifetime peak reports zero peak growth; an increase above an old peak is also not the interval's absolute maximum.
- Direction: label this as lifetime-peak growth or track an actual scoped peak.

### R05 — Low — Allocation counters count failed allocations as live allocated bytes

- Evidence: `engine/src/alloc_counter.rs:41-56` increments live/total/peak counters before checking `System.alloc`'s returned pointer.
- Impact: failed allocations overstate telemetry if a caller handles a null allocation; ordinary abort-on-OOM paths may never print a report.
- Direction: distinguish allocation attempts from successfully allocated bytes.

### R06 — Medium — Engine randomness is shared across independent games and simulations

- Evidence: `engine/src/rng.rs:74-81,109-121` has one process-wide mutable RNG; each shuffle swap locks and consumes it separately. Game deck shuffles, refreshes, and bot determinization use this API.
- Impact: concurrent or interleaved simulations consume the same stream; seeding one game resets another's stream, and cloned states do not isolate randomness. Deterministic action/seed replay depends on unrelated scheduling and RNG consumers. The per-instance `Lcg` does not remove these global engine calls.
- Direction: make gameplay RNG part of per-game state and separate rollout streams. No probabilistic behavior was benchmarked or reproduced.

### R07 — Medium — no_std RNG exposes unsynchronized global mutation through safe APIs

- Evidence: `engine/src/rng.rs:138-155` supplies a blanket unsafe `Sync` implementation for `UnsafeCell`, then mutates the global state through public safe functions without synchronization.
- Impact: soundness relies on every no_std embedding being single-threaded and nonreentrant. That invariant is not enforced by the public API; concurrent/interrupt use would race.
- Status: conditional safety hazard, not evidence of an observed race on a particular console.
- Direction: enforce/document the execution model or use an appropriate synchronization/ownership design.

### R08 — Low — Trigger normalization has two inconsistent matching contracts

- Evidence: `engine/src/triggers.rs:54-83` parses exact comma-separated tokens, while `canonical_trigger` at lines 89-105 uses substring matching and omits `Main`/`BatonTouch` cases recognized by the parser. `trigger_to_texticon` at lines 139-147 maps any unrecognized trigger to a constant-ability badge.
- Impact: metadata can report `unknown` or a mismatched kind, and unknown/composite triggers can display a misleading constant badge. This is a diagnostic/UI consistency issue, not proof of incorrect activation dispatch.
- Direction: reuse parsed trigger kinds and represent unrecognized/multiple kinds explicitly.

## Coverage ledger — coordinator portion

`Full` means the entire file was read during this audit, not that all behavior was executed or proved correct. `Targeted` means selected content was inspected. `Inventory` means existence/category only. Files not listed in a reviewer ledger remain unreviewed.

| Files | Review depth |
|---|---|
| `AGENTS.md`, `README.md`, `DEPENDENCY_AUDIT.md` | Full |
| `Dockerfile`, `.dockerignore`, `.gitignore`, `.gitattributes`, `cloudbuild.yaml`, `render.yaml`, `start.bat`, `auto_complete.bat` | Full |
| `.github/workflows/coverage.yml`, `.github/workflows/engine-tests.yml`, `.github/workflows/deploy-pages.yml`, `.github/workflows/deploy-server.yml`, `.github/workflows/deploy-fly.yml` | Full |
| `engine/Cargo.toml`, `engine/.cargo/config.toml`, `engine/build.rs` | Full |
| `engine/src/lib.rs`, `engine/src/rng.rs`, `engine/src/timer.rs`, `engine/src/triggers.rs`, `engine/src/alloc_counter.rs` | Full |
| `engine/src/decks_cards_gen.rs` | Full generated wrapper read; embedded blobs not decoded |
| `cards/build/generation_manifest.json` | Full metadata read; hashes not recomputed |
| `platforms/wasm/.cargo/config.toml` | Full cross-reference read |
| Root `Cargo.lock`, `engine/Cargo.lock` | Inventory only; dependency vulnerability audit not performed |

## Ability execution, queue, and turn engine

Reviewer scope: all 35 files under `engine/src/ability`, `engine/src/ability_queue.rs`, and `engine/src/turn`. Static inspection at HEAD `1ab04279`; assigned source paths had no pending diff at the reviewer's final check. No builds/tests/probes were run. **Source-proven branch behavior is not proof that the current generated card corpus exercises every branch.**

| ID | Severity | Evidence | Issue and impact | Direction only |
|---|---|---|---|---|
| AB01 | High | `engine/src/ability_queue.rs:385-389`; `engine/src/core/game_state/abilities.rs:1358-1388,1536-1564`; `engine/src/turn/actions.rs:934-943,967-974` | Queue compaction removes completed entries without remapping outstanding ordering options. Completed active entry A followed by nonactive B/C leaves option indices 1/2 after B/C move to 0/1: choosing B promotes C. | Stable identifiers or complete suspended-index remapping. |
| AB02 | High | `engine/src/ability/choice.rs:75-107`; `engine/src/ability/compound.rs:227-229` | Resumption removes remaining conditions by enum discriminant, not predicate equality. Distinct thresholds of the same condition variant become interchangeable after a pause, allowing an ineligible later effect. Corpus reachability unverified. | Carry exact predicate identity/result. |
| AB03 | Medium | `engine/src/ability/condition/state.rs:1249-1257`; `engine/src/ability/condition.rs:523,548-560` | Opponent-choice helper applies negation, then generic evaluation applies it again; negated and positive conditions collapse to the same result. | One owner for negation. |
| AB04 | High | `engine/src/ability/effects/draw.rs:16-69,427-437,487-497` | Filtered draw increments its counter only for matches and returns rejects to the same deck without bounding attempts. A nonempty deck with insufficient matches can loop forever. | Bound candidate scanning and preserve rejected-card order/destination. |
| AB05 | High | `engine/src/ability/effects/draw.rs:460-509`; `engine/src/ability/util.rs:2379-2396` | Distinct filtering happens after removal from deck/discard; rejected duplicate-name cards are never restored or placed in another zone. | Validate before removal or restore every rejection. |
| AB06 | High | `engine/src/ability/effects/state.rs:756-865`; `engine/src/ability/choice.rs:898-904`; `engine/src/ability/move_cards.rs:3671-3730,3818-3837` | Energy-choice producer retains the target player, but resumption discards it and resolves self. Opponent indices can consequently mutate the activating player's energy. | Preserve target identity and index mapping end to end. |
| AB07 | Medium | `engine/src/turn/triggers.rs:11-129`; `engine/src/turn/phases.rs:1174-1180` | Debut receives only printed `card_no` and picks the first matching stage member, not the newly played instance. Multiple copies can resolve effects or position checks against the old member. | Pass arriving instance ID. |
| AB08 | Medium | `engine/src/turn/phases.rs:1017-1043`; `engine/src/turn/actions.rs:172-181` | Double-baton branch spends energy before checking target protection; rejecting a protected replacement leaves energy spent. Exposed-action generation may constrain reachability but does not make the API transactional. | Validate before payment. |
| AB09 | Low | `engine/src/ability/condition.rs:279-325,565-570`; `engine/src/ability/log.rs:65-69,79-84` | Disabled logging still builds condition descriptions/verdicts because zero buffer length passes the construction guard; the finished result is discarded. Avoidable allocation and traversal, not benchmarked. | Gate or lazily construct diagnostics. |

### Ability/turn file coverage

All names in the next table are relative to `engine/src/`; each listed file has its stated status individually. **Full = entire content read, not behavior proved.**

| Depth | Files and ranges |
|---|---|
| Full (13) | `ability/ability_store.rs`, `ability/condition.rs`, `ability/condition/compound.rs`, `ability/debug.rs`, `ability/dynamic_count.rs`, `ability/effects/ability_effects.rs`, `ability/effects/draw.rs`, `ability/effects/mod.rs`, `ability/log.rs`, `ability/mod.rs`, `ability/types.rs`, `ability_queue.rs`, `turn/triggers.rs` |
| Full module declaration | `turn/mod.rs` |
| Targeted | `ability/choice.rs:1-1174`; `ability/compound.rs:1-760`; `ability/condition/card.rs:1-130`; `ability/condition/state.rs:1249-1355`; `ability/cost.rs:1-330`; `ability/describe.rs:635-789` |
| Targeted | `ability/effects/misc.rs:263-335`; `ability/effects/score.rs:650-818`; `ability/effects/state.rs:721-947`; `ability/enums.rs:1-285`; `ability/look.rs:849-1182`; `ability/move_cards.rs:3671-3876` |
| Targeted | `ability/resolver.rs:1-433`; `ability/util.rs:1860-2242,2350-2397`; `ability/vm.rs:1-284`; `turn/actions.rs:30-205,558-1341`; `turn/live.rs:1254-1498`; `turn/phases.rs:894-1255` |
| Generated inventory only | `ability/abilities_gen.rs`, `ability/condition_decoder_gen.rs`, `ability/effect_decoder_gen.rs` |

The reviewer's stated totals (13 full / 19 partial / 3 generated) differ from the individually listed classifications (14 full / 18 partial / 3 generated, counting the fully read `turn/mod.rs`). The per-file entries above, rather than the aggregate claim, define coverage. Unread parts of targeted files remain unreviewed. Supporting queue processing read: `core/game_state/abilities.rs:1301-1564`.

## Platform ports, PC relay, Android

Static source review; **no hardware, network, compiler, or packaging reproduction**. High confidence denotes explicit source contradictions, not observed runtime failure. Conditional allocator/hardware issues are labeled below. This reviewer also disclosed two temporary inventory files (`audit_tree.txt`, `audit_dirs.txt`) outside the repository, and initial enumeration of cache/vendor directories before filtering; no repository code was changed.

| ID | Severity | Evidence | Issue and impact | Direction only |
|---|---|---|---|---|
| P01 | High | `platforms/3ds/src/ctru_shim.c:2016-2030,2044-2068` | Audio allocation uses samples × channels × sample width, but decoded bytes become `nsamples` without dividing by sample width; later refill uses mutated count. Overstated DSP length and mono refill beyond allocation. | Separate byte capacity from frame count. |
| P02 | High | `platforms/3ds/src/ctru_shim.c:2170,2180-2187` | Detached audio thread is retained, joined, and freed, although it can exit independently. Lifetime misuse under libctru's ownership contract; installed SDK implementation not checked. | Consistent joinable or detached ownership. |
| P03 | High | `platforms/3ds/src/uds.rs:246-277`; `platforms/3ds/src/game/input.rs:523-550` | Decoder checks initial bytes but then directly indexes/slices later fields. Incomplete network input can panic rather than return a parse error. | Checked cursor/length validation throughout. |
| P04 | High | `platforms/3ds/src/pc_transport.rs:48-70`; `platforms/3ds/src/uds.rs:129-153,216-243`; `pc_relay/src/main.rs:204-205,241-243,340`; `platforms/3ds/src/game/input.rs:526,558-575` | 3DS sends tagged binary while relay reads JSON; DeckSync parsing is nested behind ActionSync success; WebSocket actions lack UDP forwarding; gameplay reception/ACK/retry still uses UDS directly. PC crossplay has no compatible bidirectional path. | Shared versioned codec and transport abstraction for all traffic. |
| P05 | High | `platforms/wii/src/display.rs:17-20` | Display text is passed as the format parameter of variadic printf. Formatting characters are interpreted rather than printed literally, risking invalid variadic access. Current triggering assets not established. | Fixed literal format or nonvariadic text output. |
| P06 | Medium | `platforms/3ds/src/uds.rs:235-242,276-284`; `platforms/3ds/src/game/input.rs:543-558` | Decoder does not advance offset after the optional ability index; reads the wrong action sequence and undermines ordering/ACK matching. | Checked decoder cursor plus round-trip cases. |
| P07 | Medium | `pc_relay/src/main.rs:212-239,307-340` | Client identity is not bound to action ownership; sequence is acknowledged without duplicate/order checking; execution errors are discarded before broadcast/ACK. | Enforce session/player ownership and accepted-action sequencing. |
| P08 | Medium | `pc_relay/src/main.rs:68-73,116-122` | DeckSync energy lists are ignored in favor of defaults, potentially changing composition and instance order. | Build every zone from validated synchronized data. |
| P09 | Medium | `pc_relay/src/main.rs:158,290-314,355-360,400-404` | Unbounded output queue and missing connection/handshake limits; switching sessions leaves old membership and disconnect cleans only the latest. Slow clients and stale memberships retain memory. | Bounded queues and explicit owned membership lifecycle. |
| P10 | Medium | `platforms/gba/src/sram.rs:42-53` | Ordinary copy/slice access does not enforce byte-wide volatile cartridge SRAM operations. Hardware/compiler-dependent save correctness hazard. | Platform SRAM routines or byte-wide volatile access. |
| P11 | Medium | `platforms/dc/src/lib.rs:17-30`; `platforms/ds/src/lib.rs:36-54`; `platforms/wii/src/lib.rs:15-25`; `platforms/cdi/src/lib.rs:40-59`; `platforms/snes/src/lib.rs:19-32` | C wrappers ignore Layout alignment; arenas align offsets without guaranteeing backing-address alignment. Conditional GlobalAlloc contract violation for over-aligned requests. | Align actual returned addresses. |
| P12 | Medium | `platforms/jaguar/src/lib.rs:31-44`; `platforms/cdi/src/lib.rs:47-61`; `platforms/snes/src/lib.rs:22-34` | Bump deallocation is a no-op with no reviewed reset, so transient/repeated matches permanently consume finite heaps. Exhaustion point unmeasured. | Reclaiming allocator or safely scoped arena reset. |
| P13 | Medium | `platforms/snes/src/sneslib.rs:100-129` | DMA writes 16-bit source offsets but not the source-bank register. Reads can use the wrong bank. Caller reachability incompletely reviewed. | Program full DMA source address. |
| P14 | Medium | `platforms/genesis/src/bin/rabuka_genesis.rs:17-20,43-49`; `platforms/genesis/link.ld:32-47` | Native startup points stack at RAM bottom and omits declared data copy/BSS clearing. Alternate C/assembly pipelines not reviewed. | Linker-defined stack/heap and correct startup initialization. |
| P15 | Medium | `platforms/3ds/src/setup.rs:975-976,1152-1155` | QR previews slice UTF-8 strings at byte 40 without checking a character boundary; multibyte text can panic. | Character-boundary truncation. |
| P16 | Medium | `platforms/wii/src/lib.rs:93-117` | UI retains a mutable display borrow across a second direct mutable display call, then is used again. Source-level borrow-check failure; no compiler run. | Access through current owner or end the borrow. |
| P17 | Medium | `platforms/3ds/src/bin/rabuka_3ds.rs:233-240`; `platforms/3ds/src/net.rs:8`; `platforms/3ds/src/transport.rs:1`; `platforms/3ds/src/lib.rs:5` | Duplicate non-3DS mains; unconditional net module imports a feature-gated transport module. Host/default build path inconsistent. | One host entry and aligned feature gates. |
| P18 | Medium | `platforms/genesis/build_wsl.sh:9-12`; `platforms/dc/build_dc_wamr.sh:94-96`; `platforms/psp/build_psp.bat:9-11`; `platforms/3ds/build_3ds.bat:146-151` | Cargo piped to tail without pipefail, uncollected child compilation failures, unchecked bake, or proceeding after RomFS failure can package stale outputs. | Propagate every stage's failure and publish isolated staged artifacts. |
| P19 | Medium | `android/create_package.bat:25-30`; `android/create_package.sh:58-73`; `android/setup_termux.sh:37-45` | Malformed batch variable syntax and Bash heredoc in cmd file; shell archive layout differs from instructions, stray EOF under set -e, setup clones placeholder rather than installing extracted tree. | One explicit package/install layout and native shell syntax. |
| P20 | Low | `android/start_android.sh:36-37,85-107` | Broad tunnel termination can stop unrelated processes; cleanup excludes normal exit/server failure. | Clean only owned processes on all exit paths. |
| P21 | Low | `platforms/3ds/build.rs:15-23,73-77,102`; `platforms/3ds/rust-toolchain.toml:2`; `platforms/3ds/build_3ds.bat:26-40` | Rerun inputs omit C/headers and watch wrong relative image path; Cargo and batch choose different dated toolchains. | Complete input tracking and one toolchain source. |

### Platform/relay/Android coverage ledger

Each comma-separated filename has the stated status individually. Paths are relative to the directory in the first column. Remaining files in these directories are **unreviewed**, even if inventoried.

| Directory | Fully read files |
|---|---|
| `android` | `create_package.bat`, `create_package.sh`, `setup_termux.sh`, `start_android.sh` |
| `pc_relay` | `Cargo.toml`, `src/main.rs` |
| `platforms/3ds` | `Cargo.toml`, `build.rs`, `build_3ds.bat`, `rust-toolchain.toml`, `src/lib.rs`, `src/net.rs`, `src/pc_transport.rs`, `src/transport.rs`, `src/uds.rs`, `src/util.rs`, `src/bin/rabuka_3ds.rs`, `src/game/mod.rs` |
| `platforms/gba` | `Cargo.toml`, `build_gba.bat`, `src/sram.rs`, `src/link.rs` |
| `platforms/ds` | `Cargo.toml`, `build.rs`, `src/lib.rs`, `src/nds_shim.c` |
| `platforms/dc` | `build_dc.bat`, `build_dc_wamr.sh`, `link_dc.sh`, `src/lib.rs` |
| `platforms/genesis` | `build_wsl.sh`, `link.ld`, `src/lib.rs`, `src/bin/rabuka_genesis.rs` |
| `platforms/jaguar` | `build_jaguar.bat`, `src/lib.rs`, `src/display.rs` |
| `platforms/ps1` | `build_ps1.bat`, `src/lib.rs` |
| `platforms/psp` | `Cargo.toml`, `build_psp.bat`, `src/lib.rs` |
| `platforms/snes` | `build_snes.sh`, `build_snes.bat`, `src/hardware.rs`, `src/lib.rs`, `src/sneslib.rs` |
| `platforms/wii` | `build_wii.bat`, `build.rs`, `entry.c`, `src/lib.rs`, `src/display.rs`, `src/input.rs` |
| `platforms/wasm` | `Cargo.toml`, `src/lib.rs`, `src/game_api.rs`, `src/rng_api.rs` |
| `platforms/cdi` | `src/lib.rs` |
| `platforms/sdl` | `main.c` |

Partial reads: `platforms/3ds/src/ctru_shim.c:1450-2209`, `platforms/3ds/src/setup.rs:1-1367`, `platforms/3ds/src/game/input.rs:500-609`. Remaining ranges are unreviewed.

Not substantively reviewed: `android/README_ANDROID.md`; most platform rendering/input/deck-builder/overlay code outside the named full reads; 3DS `src/ui/*`, `src/game/{action_list,overlays,render}.rs`, `src/{ffi,i18n,lang,steps}.rs`, `scripts`, `ctru-sys`, supporting Python/test scripts and RomFS configuration; GBA old display copies and most gameplay/display modules; Dreamcast/Jaguar alternate WASM pipelines; Genesis `c`/assembly/libc; SNES target/linker/startup files other than those explicitly listed. Platform `.cargo` configurations were not reviewed by this reviewer; the coordinator separately read the WASM config.

Inventory only: platform lockfiles, baked decks and fonts/art tables, compressed/binary resources, outputs, vendor packages and runtime support. Copied 3DS QR/camera code was not reviewed. Hardware correctness, vendor modifications, generated-asset consistency, and engine save-codec interoperability remain unverified.

## Core state, game APIs, server, persistence

Static review at `1ab04279`; assigned paths had no diff at the reviewer's final check. No runtime reproduction. Security entries describe defensive boundary defects only; deployment exposure and severity depend on whether untrusted clients can reach the server.

| ID | Severity | Evidence | Finding / impact | Direction only |
|---|---|---|---|---|
| CG01 | High | `engine/src/game/web_server.rs:2994,3004` | Last-session departure reacquires the same nonreentrant broadcast mutex while retaining its guard and the global room lock; normal cleanup self-deadlocks. | End guard scopes before cleanup; one lock order. |
| CG02 | High | `engine/src/game/web_server.rs:803,997-1008,1251,2409-2420` | Viewing takes room then game locks; execution takes game then room locks. Concurrent operations can deadlock the shared registry. | Release registry before locking game handles. |
| CG03 | High | `engine/src/game/web_server.rs:1223-1275`; mutating handlers in same file | Session validation fails open and sandbox/admin mutations lack a consistent mode/ownership boundary. Competitive room integrity is not uniformly enforced. | Central fail-closed authorization for every mutation. |
| CG04 | High | `engine/src/game/web_server.rs:2833-2872` | Returning-player recovery relies on display-name equality rather than a recovery credential. | Authenticate recovery independently of usernames. |
| CG05 | High | `engine/src/game/web_server.rs:1006-1013,1108-1212,1489-1490`; `engine/src/game/display.rs:1065-1072,1318-1337,1789` | Hidden-state policy differs between full/delta/debug/spectator responses; ancillary maps and pending-selection fields retain private information despite hiding card objects. | Perspective-specific allow-listed response models across every path. |
| CG06 | Medium | `engine/src/game/web_server.rs:1234-1251` | Authorization, undo snapshot, and mutation use separate lock acquisitions with no exclusive-lock revalidation. Concurrent requests can act on a newer state than checked/snapshotted. | Atomic check/snapshot/mutation plus version precondition. |
| CG07 | Medium | `engine/src/game/web_server.rs:2433-2445,2470-2484,2514-2523` | Each SSE disconnect removes the room sender even when other subscribers remain, separating remaining clients from later notifications. | Subscriber-aware channel lifecycle and generation checks. |
| CG08 | Medium | `engine/src/game/web_server.rs:298-300,379-384,3016-3047,3572` | WebSocket relay registration handlers exist but no lifecycle path populates the initially empty session map. Broadcasts have no registered destinations. | Register/unregister actor sessions explicitly. |
| CG09 | Medium | `engine/src/game/web_server.rs:2627-2632,2737` | Four-letter room ID inserted without collision checking can overwrite a live room. Probabilistic occurrence. | Atomic vacant-entry allocation and collision retries. |
| CG10 | Medium | `engine/src/game/web_server.rs:1776-1909` | Debug mutation saves its undo snapshot after applying the mutation; undo restores the changed state. | Capture pre-mutation state transactionally. |
| CG11 | Medium | `engine/src/game/web_server.rs:2142-2154` | Import ignores input, replaces current global game with empty players, and returns success while related history/version/cache bookkeeping remains inconsistent. | Unsupported error without mutation until transactional import exists. |
| CG12 | Medium | `engine/src/main.rs:31,38-79,181-184,275` | CLI state is process-local but each invocation executes one command and exits; later commands cannot retain init. Simplified dispatcher also reports success for unsupported actions. | Persistent loop/IPC/state and canonical dispatcher. |
| CG13 | Medium | `engine/src/core/card_binary.rs:42-68,103-134,185-191,299-323` | Compact decoder's length checks do not cover fixed fields and count-derived slices; incomplete/corrupt blobs can panic. External-data feature broadens applicability. | Checked lengths/arithmetic and structured decode errors. |
| CG14 | Medium | `engine/src/core/card.rs:403,453-478`; `engine/src/game/deck_builder.rs:45-71` | Signed 16-bit instance counter increments without representable-ID exhaustion handling; unrestricted construction can overflow or create negative sentinel-like IDs. Legal decks alone do not approach boundary. | Fallible allocation and input limits. |
| CG15 | Medium | `engine/src/game/deck_builder.rs:69-124`; `engine/src/game/web_server.rs:2556-2603` | Permissive construction logs missing cards/invalid composition but returns success; competitive setup uses it without a strict boundary. Sandbox permissiveness itself is intentional. | Separate strict validation and propagate setup errors. |
| CG16 | Medium | `engine/src/core/game_modifiers.rs:40-45,463-471`; `engine/src/core/stats_pipeline.rs:196-200,223-224` | Zero means no absolute override, making set-to-zero indistinguishable from absent. API defect; no named-card failure established. | Represent override presence separately. |
| CG17 | Medium | `engine/src/core/stats_pipeline.rs:60-89,166-172,187-200` | Detail helpers ignore/clamp negative modifiers while effective totals apply reductions; consumers receive inconsistent stat representations. Downstream gameplay impact not fully traced. | One signed resolved calculation for detail and totals. |
| CG18 | Medium | `engine/src/game/web_server.rs:679-685,822-827,1329-1331`; `engine/src/game/display.rs:1355-1362` | Active rooms retain cloned states/frame histories without bounds and duplicate historical display data. Memory/serialization grow with play length; no timings measured. | Bounded retention, checkpoints/deltas, pagination. |
| CG19 | Low | `engine/src/game/sav.rs:111-117,136-161` | Encoder accepts embedded NUL but decoder stops at first NUL; accepted fields do not round-trip. NUL is valid UTF-8 despite contrary comment. | Validate interior NUL and padding. |
| CG20 | Low | `engine/src/core/zones.rs:291-358` | Formation API sequentially swaps rather than assigning simultaneously; reciprocal moves undo each other and late errors can leave partial changes. No production caller found. | Validate and construct from original layout transactionally. |
| CG21 | Low | `engine/src/core/card_binary.rs:568-614`; `engine/src/core/card_loader.rs:28-31`; `engine/src/core/mod.rs:14-15` | Blob-versus-JSON test's supposed JSON loader uses the same blob under the module's enabling feature; minimum-count comparison also hides omissions. | Independent JSON oracle and equal expected cardinalities. |

### Core/game per-file coverage

Paths relative to `engine/src`; all files listed in a Full row were read entirely. Intentional single-threaded compatibility-counter assumptions and documented link-protocol limitations were not relabeled as newly proven bugs.

| Depth | Files |
|---|---|
| Full | `core/mod.rs`, `core/constants.rs`, `core/card_loader.rs`, `core/card_binary.rs`, `core/game_modifiers.rs`, `core/player.rs`, `core/pool.rs`, `core/stats_pipeline.rs`, `core/zones.rs`, `core/game_state/tracking.rs` |
| Full | `game/mod.rs`, `game/deck_builder.rs`, `game/deck_ordering.rs`, `game/deck_parser.rs`, `game/language.rs`, `game/link.rs`, `game/sav.rs`, `game/web_server.rs`, `main.rs`, `choice_renderer.rs`, `compat.rs`, `bin_common.rs` |
| Partial | `core/card.rs:1-750`, `core/types.rs:1-550`, `game/display.rs:589-2010` |
| Unreviewed by this reviewer | `core/game_state/mod.rs`, `core/game_state/abilities.rs`, `core/game_state/modifiers.rs`, `game/game_setup.rs`, `game/match_runner.rs`, `game/menu.rs`, `game/platform_ui.rs` |
| Generated inventory only | `core/cards_gen.rs`, `core/cards.bin` |

Ability reviewer separately inspected queue-related ranges in `core/game_state/abilities.rs`; this does not clear the rest of that file.

## Documentation accuracy and maintenance

Documentation-impact severity, not a claim of additional independently reproduced engine defects. Historical assertions, benchmarks, and external articles were not re-established. Entries preserve qualifications where instructions apply only to local checkout rather than deployment.

| ID | Severity | Location | Verified inconsistency / impact |
|---|---|---|---|
| D01 | Medium | `docs/ABILITY_PIPELINE.md:327-329` | Claims default-on-decode-error removed; `engine/src/ability/ability_store.rs:64-70` still returns `Ability::default()` on error. Overstates completed safeguards. |
| D02 | Medium | `docs/ABILITY_PIPELINE.md:121-134` | Describes removed serde/decode_like_json runtime fallback; current `engine/src/ability/vm.rs:241-278` directly decodes or errors. |
| D03 | Medium | `docs/memory_optimization_combined.md:20,205-209` | Cargo command is specified from repository root, which lacks Cargo.toml; manifest is in engine. The suspicion that parser.py is an invalid generator entry was disproved by its delegation to extractor main. |
| D04 | Medium | `docs/memory_optimization_combined.md:42-43,189` | Unqualified no-cache/zero-resident-ability claims contradict desktop OnceLock Arc caches in `ability_store.rs:21-29,45-55`. |
| D05 | Medium | `docs/ROADMAP.md:41,52` | Missing EnergyCondition handler listed as work although `engine/src/ability/cost.rs:964-981` implements it. |
| D06 | Medium | `docs/QR_DECK_SHARING.md:5-19,32` | Says plaintext QR output; `web_ui/card_browser.html:1204-1229` emits dictionary-index binary encoded as Base64. |
| D07 | Medium | `docs/QR_DECK_SHARING.md:60-63` | Recommends unsupported branch-based Pages folder /web_ui; actual workflow stages into docs and publishes via Actions. |
| D08 | Medium | `docs/3ds/PC_TRANSPORT_IMPLEMENTATION.md:63,68` | CLI examples use unsupported -u/-w; relay Args at `pc_relay/src/main.rs:44-52` defines long-only flags. |
| D09 | Medium | `docs/3ds/PC_TRANSPORT_IMPLEMENTATION.md:73-76` | Tells users to navigate to a WebSocket endpoint as if it were an HTML player application. No actual client procedure identified. |
| D10 | Medium | `docs/BOT_STRATEGY.md:175-178,639-640` | Internally contradicts tie-at-two-successes placement rule, producing incompatible strategy advice. |
| D11 | Low | `docs/BOT_STRATEGY.md:5,331,465` | Five-slot stage and rules v1.02 claims disagree with three-position constant and bundled rules v1.06. |
| D12 | Medium | `docs/WEB_MULTIPLAYER_ARCHITECTURE.md:204,216` | 100GB/month divided by assumed 270MB/visitor is approximately 370 visitors/month, not per day; capacity recommendation off roughly 30×. Assumed payload size unmeasured. |
| D13 | Low | `docs/WEB_MULTIPLAYER_ARCHITECTURE.md:22` | POST replay export contradicts own endpoint table and GET route at `engine/src/game/web_server.rs:3680`. |
| D14 | Low | `docs/ABILITY_PIPELINE.md:8` | Relative cards documentation link resolves into docs/cards rather than root cards. |
| D15 | Low | `engine/ISSUES_FOUND.md:27` | Link to absent REFACTOR_BACKLOG.md. |
| D16 | Low | `docs/index.html:5,9` | Checkout redirect target is absent until deployment staging; local serving fails. Not evidence deployed Pages is broken, as workflow copies target and replaces index. |
| D17 | Low | `docs/ABILITY_PIPELINE.md:531,559,582` | Handler lookup lists removed RepeatProcedure executor and wrong PositionChange/GainResource files. Actual dispatch/handlers in `effects/mod.rs:416-417` and `effects/misc.rs:2419,827`. |
| D18 | Medium | `docs/AUDITS.md:1318-1321,1428-1443` | Recommends adding already enabled casting lints; modifier-widening recommendation at 1399 contradicts explicit narrow-storage policy at 1051-1053. Unreconciled historical action items. |
| D19 | Low | `docs/TEST_AUDIT_PLAN.md:123-126,189-195` | Leaves identity investigation active despite its own later correction confirming Ren and the staged print. |
| D20 | Low | `docs/parity_plan.md:52,62,164` | Lists test_find_live_by_score missing despite implementation at `engine_c/src/test_game.c:102-113`. Behavior parity not executed. |
| D21 | Medium | `engine/PORTS.md:809,814,838` | Wii std/JSON architecture claim conflicts with current no_std/bytecode/compact features. |
| D22 | Low | `engine/PORTS.md:63-65,77,387-389,1015-1023` | Early absolute impossibility verdicts conflict with later working GBA/DC discussion and corrected ROM/RAM analysis. |
| D23 | Low | `docs/3ds/HANG_DEBUGGING.md:16,137-140` | Debugging paths use absent engine_3ds rather than platforms/3ds. Historical causes not revalidated. |
| D24 | Low | `docs/3ds/VISUAL_DESIGN.md:72-84` | RGB888 and rgba values mislabeled packed RGB565 hexadecimal. |
| D25 | Low | `docs/gba_deck_builder_plan.md:19,54,165` | Eight prefix series / 80-character alphabet differ from seven labels and longer current alphabet in `platforms/gba/src/deck_builder.rs:184-208`. |
| D26 | Low | `docs/ABILITY_FAMILIES.md:14-15,108` | Sampled generated anchor fragments retain colons removed by GFM heading slugging; index navigation fails under GitHub rendering. |
| D27 | Low | `docs/ABILITY_FAMILIES.md:124` | Sampled generated Japanese phrase is malformed. Encoding versus generation origin and extent unverified; do not infer widespread corruption. |

### Documentation per-file coverage

Fully read: `docs/ABILITY_PIPELINE.md` (598 lines), `docs/AUDITS.md` (1825), `docs/BOT_STRATEGY.md` (779), `docs/EFFECT_ARCHITECTURE.md` (169), `docs/ROADMAP.md` (56), `docs/TEST_AUDIT_PLAN.md` (302), `docs/WEB_MULTIPLAYER_ARCHITECTURE.md` (320), `docs/QR_DECK_SHARING.md` (104), `docs/parity_plan.md` (166), `docs/memory_optimization_combined.md` (255), `docs/gba_board_and_card_images.md` (336), `docs/gba_deck_builder_plan.md` (218), `docs/3ds/HANG_DEBUGGING.md` (140), `docs/3ds/freeze_checklist.md` (90), `docs/3ds/PC_TRANSPORT_IMPLEMENTATION.md` (115), `docs/3ds/VISUAL_DESIGN.md` (214), `docs/index.html` (11), `engine/PORTS.md` (1371), `engine/ISSUES_FOUND.md` (27), `engine/3DS_CROSS_COMPILE_GUIDE.md` (78), `engine/PORT_TO_3DS.md` (110).

Generated inventory read, not semantic coverage validation: `docs/ABILITY_MATRIX.md` (125 lines). Sample only: `docs/ABILITY_FAMILIES.md:1-240` of 1972. `docs/img/**` inventory was initially truncated; image contents/completeness not audited. A full document read does not independently validate every historical implementation assertion.

## Bots, training, harnesses, tests, analysis scripts

Static evidence only; no executable verification. Reviewer inspected `1ab04279` with concurrent later edits to strategy_v7/tests. The PPO failures are independent: correcting the first failure alone would not establish a working training/inference pipeline.

| ID | Severity | Evidence | Finding and impact | Direction only |
|---|---|---|---|---|
| BT01 | High | `training/train_ppo.py:217-227,325-333` | Config dataclass has unannotated class attributes, but main passes constructor keywords. Entry point fails before training. | Typed fields or explicit constructor. |
| BT02 | High | `engine/src/bin/ppo_collect.rs:61-62,203-211`; `training/train_ppo.py:164-172` | Collector writes one-byte trajectory/state lengths; Python reads four-byte fields. State dimension truncates and record alignment is lost. | Shared versioned fixed-width format and round-trip fixtures. |
| BT03 | High | `engine/src/bot/encoding.rs:87-110`; `engine/src/bot/neural.rs:184-207`; `training/train_ppo.py:34-36,246-256` | Dimension counts eight pooled zones but emits nine: declared 1844, actual 1972 floats. Rust drops tail features; Python shape assignment fails. | Single explicit encoding layout with length assertions. |
| BT04 | High | `engine/src/bot/encoding.rs:123-145`; `engine/src/bot/neural.rs:35,213-219,247-252` | Action encoding appends entire embedding table instead of selected action row, never reads action_type, emits wrong position width. Inference consumes an identical table prefix for every action. | Index correct row and assert exact encoding width/distinctness. |
| BT05 | High | `training/train_ppo.py:27,86-102`; `engine/src/bot/neural.rs:58-79` | Python writes 25 action rows while Rust reads 16 and ignores version validation. Later tensors shift by 144 floats while load may succeed. | Version/shape/length validation and shared cardinalities. |
| BT06 | High | `engine/src/bot/determinization.rs:158-215` | Fair reconstruction fills opponent deck with entire database remainder, only padding undersized decks and never trimming to observed count. Unrealistic draws/refresh and inflated rollout work. | Bounded belief sampling preserving zone counts. |
| BT07 | High | `engine/src/bot/observation.rs:8-31`; `engine/src/bot/determinization.rs:63-112` | Fresh player/state reconstruction omits public subphase, choices, orientations/modifiers and usage state. Search evaluates a different rules position. | Preserve public state; randomize only hidden information. |
| BT08 | High | `engine/src/bot/ismcts.rs:39-48,67-74,115-162` | Any eventual rollout win is treated as a proven immediate win; first lucky early-listed action terminates comparison. | Only root terminal wins justify immediate exit. |
| BT09 | High | `engine/src/bot/rollout.rs:9-11,128-145,175-194`; `engine/src/bot/strategy_v7.rs:172-184,398-400` | Optional portfolio rollout and clone/execute lookahead use actual hidden hands/deck order. Public-only scoring does not remove information revealed through simulated outcomes. | Determinize before simulation; hidden-state invariance checks. |
| BT10 | Medium | `engine/src/bot/strategy_v5.rs:31-42` | Binomial-tail loop stops on tiny initial mass even when later terms grow. n20,p0.9,k1 yields zero instead of nearly one. | Numerically stable tail calculation. |
| BT11 | Medium | `engine/src/bot/rollout.rs:35-48` | Winner valuation equates player index0 with first attacker; role and index can differ. | Resolve winner by actual attacker role. |
| BT12 | Medium | `engine/src/bot/ismcts.rs:181-188` | Selected action remapping compares only type/card, collapsing different positions, indices, and abilities to first match. | Preserve original index or full action identity. |
| BT13 | Medium | `engine/src/bin/bot_demo.rs:24-30`; `engine/src/bot/mod.rs:117-133`; `engine/src/bot/ismcts.rs:174-180` | Demo loads weights, but chosen search path ignores network, so it does not demonstrate trained policy. | Explicit inference integration or honest interface. |
| BT14 | Medium | `engine/src/bin/bot_data_gen.rs:53-60,98-109` | Games use p1/p2 but smart branch requires player1; supplied weights still produce random actions. | Actual player identity and intended policy dispatch. |
| BT15 | Medium | `training/train_ppo.py:75-83` | Padding mask matches legal Pass encoding, effectively eliminating passing when alternatives exist. | Explicit padding/legal-action mask. |
| BT16 | Medium | `engine/src/bin/ppo_collect.rs:155-162` | Samples 10% uniform/90% policy mixture but records policy-only probability; PPO behavior ratio is wrong. | Record actual mixture probability. |
| BT17 | Medium | `training/train_ppo.py:199-207,268-269` | GAE uses next action terminal flag for current bootstrapping; singleton unbiased std produces NaN advantages. | Correct transition terminal semantics and singleton normalization. |
| BT18 | Medium | `engine/src/bin/calibrate.rs:365-406,447-469` | Any success among six trials is reported as placement rate; true50% becomes98.4% probability of at least one. | Aggregate per-trial outcomes/fractions and counts. |
| BT19 | Medium | `engine/src/bin/calibrate.rs:80-106`; `engine/src/bot/strategy_v4.rs:100-145` | Copied calibration predictor uses wildcard expected flips rather than current v4 colored distribution. | Call shared versioned predictor. |
| BT20 | Medium | `engine/benches/performance.rs:100-153,166-170` | Completion benchmark accepts stuck/capped games and ignores execution errors; phase advance can bypass pending choices; microbenchmark includes randomized setup. | Fixed valid fixtures, separate setup, explicit completion/errors. |
| BT21 | Medium | `engine/src/bot/rollout.rs:292-314,325-328` | Thread-local plan cache omits game/player/board/opponent/modifier identity, allowing cross-game stale plan reuse. | Decision-scoped cache or complete key/lifecycle. |
| BT22 | Medium | `engine/analyze_checks.py:6`; `engine/analyze_curve.py:6`; `engine/analyze_perf.py:8`; `engine/analyze_stall.py:6,27`; `engine/show_game.py:10`; `engine/src/bin/bot_arena.rs:482` | Consumers expect ENTER while producer emits ENTER:<phase>; empty/zero reports or unpack-None failure result. | Versioned trace schema and missing-record errors. |
| BT23 | Medium | `engine/analyze_throws.py:21` | Invalid Python token sequence el    if prevents startup. | Include utility scripts in syntax validation. |
| BT24 | Medium | `engine/analyze_losses.py:93-100,118-123` | Per-turn failures inferred from final successes; later success masks earlier failure. folded_all predicate requires both nonempty values and an empty map, so never fires. | Adjacent live-boundary attribution and valid emptiness predicate. |
| BT25 | Medium | `engine/tests/test_modules/characterization/action_coverage_test.rs:32-47,61-120` | Counts almost every attempted activation error as action coverage; no valid trigger/effect postcondition, shallow compound scan, empty coverage can pass. | Prove target action executed under valid fixture and nonempty coverage. |
| BT26 | Medium | `engine/tests/test_modules/characterization/corpus_smoke_test.rs:21-81` | Ignored activation errors/bounded draining do not prove each ability executed; uniqueness map resets per player and omits under-cards, energy decks, resolution zone. | Execution outcome accounting and global conservation invariants. |
| BT27 | Medium | `engine/tests/test_modules/characterization/corpus_smoke_test.rs:103-118,137-152` | Global panic suppression applies while any smoke worker runs, suppressing unrelated parallel tests' diagnostics. | Thread-scoped suppression or avoid global hook change. |
| BT28 | Medium | `engine/tests/run_all.rs:71-88` | Missing/nonarray/empty unique_abilities field makes no-custom-actions check scan nothing and pass. | Assert schema and meaningful corpus cardinality. |
| BT29 | Low | `engine/tests/test_modules/integration/e2e_basic_game_test.rs:6-9,178-188,304-350` | Named turn3 test stops at >=2; accepts Pass alone as main-action coverage; repeated assertions add little evidence. Earlier phase assertions remain useful. | Exact intended progression and fixture-specific actions. |
| BT30 | Low | `engine/src/bot/neural.rs:52-55` | Short weight files panic through unchecked slicing despite Result API. | Validate lengths before mutating model. |
| BT31 | Low | `engine/src/bot/strategy_v6.rs:118-192`; `engine/src/bot/strategy_v7.rs:181-291` | Diagnostic strings built even when debugging off. | Gate construction. |
| BT32 | Low | `engine/src/bot/mod.rs:45-65`; `engine/src/bot/ismcts.rs:96,116` | Exposed rollout-depth/heuristic/progressive-widening settings do not control this search path. | Implement semantics or remove unsupported configuration. |
| BT33 | Low | `engine/analyze_logs.py:12-15,30` | Two-word result regex skips single-word DRAW headers. | Structured/complete result parsing. |
| BT34 | Low | `engine/pass_probe.py:15-19,32-39` | Early Main/Pass filters make later live-set branch unreachable. | Separate phase analysis. |
| BT35 | Low | `engine/count_batons.py:10`; `engine/extract_examples.py:6-8`; `engine/pass_probe.py:10` | Developer-specific temporary paths/fixed game IDs impair reuse and provenance. | Path/selection arguments and dataset metadata. |
| BT36 | Low | `engine/src/bin/harness.rs:139-151` | EOF returns zero bytes, not error, so input loop repeatedly prints invalid input instead of exiting. | Treat zero read as EOF. |

### Bot/training/test per-file coverage

Full reads under `engine/src/bot`: `conductor.rs`, `determinization.rs`, `encoding.rs`, `evaluation.rs`, `ismcts.rs`, `mod.rs`, `neural.rs`, `observation.rs`, `registry.rs`, `rollout.rs`, `strategy.rs`, `strategy_common.rs`, `strategy_v2.rs`, `strategy_v3.rs`, `strategy_v4.rs`, `strategy_v5.rs`, `strategy_v6.rs`, `strategy_v7.rs`, `weights.rs` (all19).

Full reads under `engine/src/bin`: `bot_arena.rs`, `bot_data_gen.rs`, `bot_demo.rs`, `calibrate.rs`, `collect_data.rs`, `describe_dump.rs`, `harness.rs`, `ppo_collect.rs`, `profile_target.rs`. Mechanical targeted scan only: `bad_game_report.rs`, `diag_stall.rs`, `hunt_loop.rs`, `play_game.rs`, `rabuka_3ds.rs`, `test_hang.rs`, `trace_game.rs`, `turn_replay.rs` (entry points/caps/execution/ignored results/unwraps/RNG).

Full reads: `engine/examples/ds3_smoke.rs`, `engine/examples/gen_flamegraph.rs`, `engine/examples/play_and_observe.rs`, `engine/benches/performance.rs`, `training/train_ppo.py`, `ai_design/nn_architecture.md`. Training binaries inventory only; `ai_design/rabuka_bot_design.md` unreviewed.

Full engine-root utility reads: `analyze_audit.py`, `analyze_checks.py`, `analyze_curve.py`, `analyze_deck.py`, `analyze_logs.py`, `analyze_losses.py`, `analyze_perf.py`, `analyze_stall.py`, `analyze_throws.py`, `count_batons.py`, `extract_examples.py`, `pass_probe.py`, `search_card.py`, `show_game.py`, `cargo`.

Full test reads only: `engine/tests/run_all.rs`, `engine/tests/test_modules/characterization/action_coverage_test.rs`, `engine/tests/test_modules/characterization/corpus_smoke_test.rs`, `engine/tests/test_modules/integration/e2e_basic_game_test.rs`, plus inline bot_arena/strategy_v7 tests. Other `engine/tests/**/*.rs` underwent a smell search capped at80 results, **not a file-by-file semantic review**. Separate search found no explicit references to the named neural/determinization/search APIs; this does not exclude indirect tests. Helpers, support, repetitive ability/rule bodies, generated inventories, and concurrent new tests remain unreviewed by this reviewer.

## Frontend, card pipeline, and tools

Static partial review; no files changed or commands executed by this reviewer. No card identity/printed behavior, generated binary contents, or images were manually validated. Reachability of the alternate WASM service was not established; normal network facade uses HTTP GameService.

| ID | Severity | Evidence | Finding / impact | Direction only |
|---|---|---|---|---|
| UICT01 | High | `tools/bake_deck_cards.py:101-118,141-148`; `cards/compile_cards.py:81-89`; `cards/build_lib.py:39-56` | Deck string table serialized before encode_card interns additional fields, creating references absent from the serialized table. Four workers share a mutable StringTable, making contents scheduling-dependent and contaminating compact per-deck data with unrelated strings. Specific current binaries not decoded. | Private per-blob table; finish interning/encoding before serialization. |
| UICT02 | High | `cards/ability_extraction/extract_card_abilities.py:578-590`; `cards/compile_abilities.py:515-516,592-598` | Successful extraction deletes abilities.bin after compilation despite generated GBA code embedding it. Regeneration leaves required artifact absent. | Preserve published required artifacts. |
| UICT03 | Medium | `cards/ability_extraction/extract_card_abilities.py:519-529,580-605` | Writes JSON before validation; deletes old binary before compile; compiler failure not propagated and decoder failure warning-only. Can finish successfully with inconsistent/incomplete artifacts. | Validate/stage complete set and propagate every failure. |
| UICT04 | Medium | `cards/ability_extraction/extract_card_abilities.py:538-563,575-576` | Late local import of sys makes earlier sys.exit branches reference unassigned local. Improvements comprehension defaults missing key for comparison but directly indexes it for result, permitting KeyError when rule disappears. | Module-scope import and consistent defaulted lookup. |
| UICT05 | Medium | `web_ui/js/services/RoomManager.js:126-164` | Room state persisted before join success; false success flag and caught errors still hide modal, announce join, fetch/connect SSE. | Commit UI/session only on success; preserve dialog/state on failure. |
| UICT06 | Medium | `web_ui/js/services/SSEClient.js:49-59`; `web_ui/js/services/GameService.js:65-88` | SSE errors mark disconnected only at CLOSED, not reconnecting CONNECTING; false connection state never restarts polling. Updates stall despite advertised fallback. | Resume polling during loss/reconnect; stop on open. |
| UICT07 | Low | `web_ui/js/services/SSEClient.js:10-27` | Retry increments backoff counter, then connect resets it immediately, keeping terminal retries near initial delay. | Reset only on successful open or deliberate new session. |
| UICT08 | Medium | `web_ui/js/services/WasmGameService.js:179-203,225-228` | Delta updater cannot initialize absent state, omits player-zone updates, and ignores null pending-choice clearing; fetch/optimistic rollback cannot restore authoritative board. Alternate-service reachability unverified. | Complete snapshot/bootstrap contract and presence-aware delta semantics. |
| UICT09 | Medium | `web_ui/js/services/WasmGameService.js:81-102,259-265` | Teardown clears pending request map without rejecting promises; timeout also depends on map membership, leaving awaits pending indefinitely. | Reject/cancel requests before clearing and clean timers. |
| UICT10 | Medium | `cards/generate_condition_decoder.py:260-269`; `cards/generate_effect_decoder.py:215-220,278-309,346` | Unsupported declared field types silently skipped/omitted, allowing generated optional-field semantics to disappear without generation failure. No currently emitted field proven affected. | Reject unsupported types except explicit ignored fields. |
| UICT11 | Medium | `cards/validate_schema.py:17,57-82,94-103` | Compiler path only printed; handler comparison uses incompatible identifiers and mismatch branch does nothing. Passing check does not prove advertised compiler/handler synchronization. | Actual mapping/dispatch validation with failures. |
| UICT12 | Low | `cards/build_lib.py:88-137`; `cards/compile_cards.py:180-190`; `cards/compile_abilities.py:627-643` | Both compilers replace one manifest; second run discards first compiler provenance. | Separate manifests or combined independently maintained entries. |
| UICT13 | Low | `web_ui/js/card_utils.js:68-80` | Recognized zero-point cards counted as unknown because recognition inferred from points rather than lookup. Downstream field use not reviewed. | Separate identity lookup from point value. |

Integration qualification for B12: the reviewer confirmed mapped/local image resolution and no automatic CDN fallback, but did not fully inspect mapping initialization. Coordinator inspected local mapping construction at state.js:381. Neither review establishes that every image in every Docker deployment fails. Pages deck-asset delivery remains unverified.

### Frontend/pipeline/tools file coverage

| Depth | Files |
|---|---|
| Full | `cards/build_lib.py`, `cards/compile_cards.py`, `cards/compile_abilities.py`, `cards/generate_condition_decoder.py`, `cards/generate_effect_decoder.py`, `cards/validate_schema.py`, `cards/audit_emitted_keys.py`, `cards/ability_extraction/extract_card_abilities.py`, `tools/bake_deck_cards.py` |
| Full | `web_ui/src/workers/gameWorker.js`, `web_ui/js/network.js`, `web_ui/js/card_utils.js`, `web_ui/js/constants.js`, `web_ui/js/services/GameService.js`, `web_ui/js/services/WasmGameService.js`, `web_ui/js/services/SSEClient.js`, `web_ui/js/services/RoomManager.js` |
| Partial | `web_ui/js/components/CardRenderer.js:1-285`; `cards/ability_schema.json:1-65`; image-related excerpts only from `web_ui/js/ui_tooltips.js` |
| Inventory / unreviewed | `web_ui/index.html`, `web_ui/tutorial.html`, `web_ui/ability_tree.html`, `web_ui/card_browser.html`, `web_ui/deck_converter.html` (documentation reviewer separately read QR-producer excerpt) |
| Inventory / unreviewed | All11 `web_ui/css` files; `web_ui/js/app_controller.js`, `compat.js`, `constants_dom.js`, `interaction_adapter.js`, `layout.js`, `logger.js`, `main.js`, `replay_system.js`, `state.js`, `ui_drag_drop.js`, `ui_modals.js`, `ui_rendering.js`, `view_state.js` (coordinator's state.js excerpts do not constitute full review) |
| Inventory / unreviewed | Components `ActionButtons.js`, `ActionListView.js`, `ActionMenu.js`, `AiDriver.js`, `BoardRenderer.js`, `ChoiceView.js`, `HeaderStats.js`, `Highlighter.js`, `LogRenderer.js`, `PerformanceRenderer.js`, `RpsView.js`, `ZoneViewer.js` |
| Inventory / unreviewed | All15 `web_ui/js/modals` files; services `DebugService.js`, `PlannerService.js`; utils `Attribution.js`, `DOMUtils.js`, `LogFilter.js`, `ModalManager.js`, `PerformanceMonitor.js`, `StringUtils.js`, `TextEnricher.js`; i18n index/names/translator/locales; `web_ui/js/tests/ui_logs.test.js`; `web_ui/public/wasm/test.html` |
| Inventory / unreviewed | `cards/add_new_cards.py`, `deck_compression.py`, `describe_fidelity_report.py`, `find_bad_tests.py`, `fix_images.py`, `pipeline_report.py`, `scrape_all.py`, `scrape_new_cards.py`, `scrape_qa.py`, `scrape_stats.py`, `test_inventory.py`, `ABILITY_DOCUMENTATION.md` |
| Inventory / unreviewed | Extraction `_d.py`, `analyze_phrases.py`, `card_overrides.py`, `gap_report.py`, `parser.py`, `parser_utils.py`, parser notes/plans/baseline, all six extraction test files, ability_docs_scripts analysis/inversion/README |
| Inventory / unreviewed | Tools `analyze_deck_ordering.py`, `analyze_decks.py`, `bake_card_art.py`, `bake_font_tiles.py`, `bake_texticon_tiles.py`, `compare_dpad.py`, `gen_cjk_font.py`, `gen_sjis_table.py`, `merge_yoster_ja.py`, `preview_card_art.py`, `reorder_for_gba.py`, `test_bpp.py`, `font/used_chars.py`, `bake/Cargo.toml`, `bake/src/main.rs` |
| Generated/data/vendor inventory only | Master cards/abilities/QA JSON, cards/build outputs, generated_constants/assets_registry/translations, qrcode.min.js, WASM generated glue, images, all13 listed deck files, fonts/character lists, mkpsxiso vendor binaries/docs, tool lockfile |

Glob inventory was initially capped; the reviewer does not claim a complete filesystem inventory. Most UI rendering, the large parser, scrapers, art/font tooling and test bodies remain substantive audit gaps. Incidental cache/log/base64 search matches are not review coverage.

C implementation results remain pending consolidation.
