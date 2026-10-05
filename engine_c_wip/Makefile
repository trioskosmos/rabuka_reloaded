CC      ?= gcc
CFLAGS  ?= -std=c11 -O2 -Wall -Iinclude -Isrc -Isrc/core/generated
# RB_HARNESS_WIN32 gates ONE thing and one thing only: the Windows console
# code-page fix in src/test_game.c's test_output_utf8_init(). A redirected
# stream was already byte-exact UTF-8 (measured 2026-10-05), so this changes
# nothing about the bytes a log file receives -- it stops the CONSOLE from
# rendering those bytes through code page 437/1252 as mojibake, which is what
# made the C logs unreadable. Undefine it and you get the old console
# behaviour back; no gate, no assertion and no other target is touched.
# sizeof(GameState) is 1091816 bytes -- MEASURED 2026-10-05 with
# sizeof() in a throwaway TU against the current header, not estimated.
#
# HISTORY, because this number has been wrong repeatedly and a wrong number here
# is how a 1.7 MB frame ships: it read "~780 KB" until 2026-10-03 (wrong by
# 415656 B), then 885912, then 1113736 after fields were APPENDED to GameState
# (rule_log, resolution_owners, live_face_up, constant_need_heart_global), then
# 1195656 after the three success_zone_* bonus ledgers joined them, then
# 1195720. It was 1090264 -- DOWN 105456 B -- because 13 dead fields were
# DELETED (field census 2026-10-04: 0 writers AND 0 readers, all tombstoned in
# place in include/rabuka.h). `constant_need_heart_global[4096][8]` +
# `_set[4096][8]` alone were 98304 B and were the largest dead allocation in the
# struct; the rest is base_score_total / card_bonus_total / live_spare (104 B),
# assignment+n_assignment (36 B), turn2_abilities_played (256 B),
# areas_placed_this_turn (64 B), auto_ability_trigger_counts (128 B),
# energy_placed_this_turn (8 B), n_gained_cards (4 B) and pending_energy_payment
# (4 B); the balance against that arithmetic is re-packing and trailing alignment.
# That took it to 1090528, and it is now 1091816 -- UP 1288 B -- because
# Rule 9.5.3 added two APPENDED fields to RbChoice
# (`auto_option_queue_index[16]` + `n_auto_options`, 68 B), and RbChoice is
# embedded in GameState SEVENTEEN times: once as `queue.pending` and once per
# `RB_QUEUE_DEPTH`(16) queue entry's `pending_choice`. 17 * 68 = 1156 B, plus
# 132 B of re-packing and trailing alignment. `AutoAbilityOption` itself
# (252 B) is deliberately NOT embedded in RbChoice -- a full copy at
# RB_MAX_AUTO_ABILITY_OPTIONS would have cost 17 * ~3.4 KB -- so see the comment
# on those two fields in include/rabuka.h.
# The pin in tests/test_harness_self.c section A4 moved in the same change.
#
# APPENDED, never inserted: insertion shifts every later offset and silently
# invalidates object files compiled against the previous header.
#
# ⛔ WHY YOU WILL RE-READ THIS FILE'S COMMENT TEXT INSTEAD OF THE HEADER next:
# `include/rabuka.h` once gained the line "writers live in src/ability/effects/*.c"
# inside a TOMBSTONE comment. That contains the two-character sequence `/*`, and
# GCC's -Wcomment fired on it -- and adding that ONE comment line was MEASURED to
# cost exactly one extra failure in each of five suites (parity_sequential,
# parity_zones_position, parity_live_total, audit_zones, mechanics: 142/50/50/5/7
# -> 143/51/51/6/8), byte-for-byte reproducibly, with sizeof(GameState) unchanged
# and no field added or removed. Never write `/*` inside a block comment in this
# tree; write the path without the trailing glob.
#
# WHY IT MATTERS: suites hold a GameState as a LOCAL, which overflows the 1 MB
# default stack on Windows, and two functions hold two at once (~1.7 MB before
# this growth). Every link target therefore gets a big stack below.
# tests/test_harness_self.c (A) asserts sizeof <= stack/15. At 16777216 that
# ceiling would be 1118481 -- ALREADY EXCEEDED, so the 32 MB stack is not
# optional: at 33554432 the ceiling is 2236962, which is real slack.
#
# TO RE-MEASURE after adding fields: compile a throwaway TU that prints
# sizeof(GameState) and sizeof(TestGame) against the live header. Do not
# hand-edit this number from an estimate -- an estimate here is the bug.
LDFLAGS ?= -Wl,--stack,33554432
# Layout mirrors engine/src: ability/ (vm, condition, choice, queue, effects/),
# core/ (card, data, alloc, modifiers, stats_pipeline, generated/), turn/ (phase, live, triggers).
#
# src/ability/choice_renderer.c (1964 lines, the port of engine/src/
# choice_renderer.rs) was in NO build target until 2026-10-03: not here, not in
# platforms/*/build_*.sh, not in platforms/Makefile. 1821 of its lines were
# compiled into nothing and no compiler had ever seen them. It is here now so
# that stops. Measured 2026-10-03: it compiles with ZERO diagnostics, its 27
# exported rb_rend_* symbols collide with nothing else in OBJ_LIB, and all 17
# engine symbols it references resolve to exactly one definition each. It still
# has no CONSUMER -- this tree has no PlatformUi backend -- which is why
# platforms/*/build_*.sh deliberately do NOT list it (those targets ship their
# own UI in platforms/*/src). Do not remove it from SRC/OBJ_LIB on the grounds
# that nothing calls it: a file no compiler sees is not a port, it is a guess.
# Exercised by tests/test_choice_renderer.c.
SRC     := src/ability/enums.c src/ability/types.c src/ability/vm.c src/ability/condition.c src/ability/choice.c src/ability/ability_queue.c src/ability/dynamic_count.c src/ability/util.c src/ability/cost.c src/ability/compound.c src/ability/resolver.c src/ability/effects/executor.c src/ability/effects/ability_effects.c src/ability/effects/custom.c src/ability/effects/move.c src/ability/effects/move_missing.c src/ability/effects/rules.c src/ability/effects/look.c src/ability/effects/state.c src/ability/effects/ability.c src/ability/effects/misc.c src/ability/effects/draw.c src/ability/effects/score.c src/ability/log.c src/ability/debug.c src/ability/abilities_gen.c src/ability/describe.c src/ability/choice_renderer.c src/game/deck_parser.c src/game/deck_builder.c src/game/game_setup.c src/core/card.c src/core/data.c src/core/alloc.c src/core/game_state_core.c src/core/modifiers.c src/core/stats_pipeline.c src/core/game_state_abilities.c src/core/tracking.c src/core/zones.c src/bytecode_data.c src/core/generated/bytecode_blob.c src/core/generated/gen_data.c src/core/gen_data_rom.c src/turn/phase.c src/turn/live.c src/turn/triggers.c src/engine.c src/test_game.c src/web_server.c src/main.c
OBJ     := $(SRC:.c=.o)
# library objects (everything except main, so tests can supply their own main)
OBJ_LIB := src/ability/enums.o src/ability/types.o src/ability/vm.o src/ability/condition.o src/ability/choice.o src/ability/ability_queue.o src/ability/dynamic_count.o src/ability/util.o src/ability/cost.o src/ability/compound.o src/ability/resolver.o src/ability/effects/executor.o src/ability/effects/ability_effects.o src/ability/effects/custom.o src/ability/effects/move.o src/ability/effects/move_missing.o src/ability/effects/rules.o src/ability/effects/look.o src/ability/effects/state.o src/ability/effects/ability.o src/ability/effects/misc.o src/ability/effects/draw.o src/ability/effects/score.o src/ability/log.o src/ability/debug.o src/ability/abilities_gen.o src/ability/describe.o src/ability/choice_renderer.o src/game/deck_parser.o src/game/deck_builder.o src/game/game_setup.o src/core/card.o src/core/data.o src/core/alloc.o src/core/game_state_core.o src/core/modifiers.o src/core/stats_pipeline.o src/core/game_state_abilities.o src/core/tracking.o src/core/zones.o src/bytecode_data.o src/core/generated/bytecode_blob.o src/core/generated/gen_data.o src/core/gen_data_rom.o src/turn/phase.o src/turn/live.o src/turn/triggers.o src/engine.o src/test_game.o src/web_server.o
TARGET  := rb_engine
WEB_TARGET := rb_web_server
GENERATED := tests/test_ported_generated.c
MECHANICS := tests/test_mechanics.c
LIVE_RULES := tests/test_live_success_rules.c
PHASE_RULES := tests/test_phase_machine_rules.c
PERFORMANCE_RULES := tests/test_performance_phase_rules.c
SURPLUS_HEART := tests/test_surplus_heart.c
OPPONENT_LIVE_FLOW := tests/test_opponent_live_success_flow.c
LIVE_MOVEMENT := tests/test_live_card_zone_movement.c
RESTRICTIONS := tests/test_restriction_mechanics.c
CONDITION_EVAL := tests/test_condition_evaluation.c
TARGET_SELECTION := tests/test_target_selection.c
MAX_DISTINCT_NAMES := tests/test_max_distinct_names.c
BATON_TOUCH := tests/test_baton_touch.c
MEMBER_ACTIVATION := tests/test_member_activation_debut_live_start.c
B8_LIVE_TIMING := tests/test_b8_live_timing.c
CROSS_PLAYER := tests/test_cross_player_interaction.c
DRAW_PHASE_FIX := tests/test_draw_phase_fix.c
ZONE_FILTER_PARITY := tests/test_zone_filter_parity.c
P1_HELPERS := tests/test_p1_helpers.c
LIVE_ALLOCATOR := tests/test_live_allocator.c
LOOK_SELECT := tests/test_look_select.c
MOVE_CARDS := tests/test_move_cards.c
DECK_BUILDER := tests/test_deck_builder.c
DECK_PARSER := tests/test_deck_parser.c
DECODE_FALLBACK := tests/test_decode_fallback.c
STATE_EFFECTS := tests/test_state_effects.c
ABILITY_EFFECTS := tests/test_ability_effects.c
AUDIT   := tools/audit_actions.c

.PHONY: all web test audit regen clean mechanics live-rules phase-rules performance-rules surplus-heart opponent-live-flow live-movement restrictions condition-eval target-selection max-distinct-names baton-touch member-activation b8-live-timing cross-player draw-phase-fix zone-filter-parity p1-helpers live-allocator look-select move-cards deck-builder deck-parser decode-fallback state-effects ability-effects suites t-run
all: $(TARGET)

web: $(WEB_TARGET)

$(WEB_TARGET): $(OBJ_LIB)
	$(CC) $(CFLAGS) $(LDFLAGS) -mwindows -o $@ $(OBJ_LIB)

regen: data
	python3 tools/gen_from_rs.py ../cards/build/abilities_gen.rs
	python3 tools/gen_bytecode.py
	$(MAKE) --no-print-directory regen-check

# ── REGEN DRIFT GATE ────────────────────────────────────────────────────────
# `regen` above rewrites gen_data.c, and that silently moves numbers that are
# pinned in OTHER files -- and nothing checked that the pins moved with them, so
# the only symptom was a red harness self-test hours later complaining about
# STACK BUDGET (tests/test_harness_self.c section A4) in a change that had
# touched no header at all.
#
# `make regen-check` runs after any regeneration and names the number that
# moved. It MEASURES sizeof(GameState) by compiling a throwaway TU against the
# live header rather than trusting a comment, and it cross-checks:
#     sizeof(GameState)          == RB_GAMESTATE_BYTES_PINNED (tests/ A4)
#     RBKA_STRINGS_TABLE_LEN     == STRINGS_BYTES (this Makefile)
#     src/abilities_strings.bin  == RBKA_STRINGS_TABLE_LEN
#     RBKA_STRINGS_FNV count     == RBKA_NUM_STRING_OFFSETS - 1
#     the sizeof note above       == the measured value
# Every one of those pins must move in the SAME change as the regen.
#
# NOTE, because the obvious guess here is wrong: RBKA_NUM_STRING_OFFSETS does
# NOT change sizeof(GameState). STRINGS_OFFSETS is `extern const uint32_t *`
# (a .rodata pointer, include/rabuka.h:408) and no GameState field is sized by
# the string count -- every large array in GameState is a fixed literal. What
# moves sizeof(GameState) is a change to include/rabuka.h. So this gate
# deliberately does NOT assert a string-count/sizeof coupling, which would have
# fired on every healthy regeneration. tools/regen_check.py documents this in
# full.
.PHONY: regen-check
regen-check:
	@python3 tools/regen_check.py --root . --makefile Makefile \
	    --harness tests/test_harness_self.c

# Card data blobs the test binaries load via rb_load("src"). Produced by the
# pure-Python generators in cards/; regenerated here so a clean checkout can
# build and run the suites without a manual copy step.
#
# `?=` not `:=` so a snapshot can point the rules at its own artifacts without
# editing this file (make data STRINGS_BIN=/path/to/blob).
CARDS_BIN      ?= ../cards/build/cards.bin
STRINGS_BIN    ?= ../cards/build/abilities_strings.bin

# Canonical identity of each blob, pinned to the ONE generator run that produced
# both cards/build/ and the committed src/core/generated/gen_data.c. gen_data.c
# records the same length as RBKA_STRINGS_TABLE_LEN and the same content digest
# as RBKA_STRINGS_TABLE_FNV, so these three numbers describe one artifact:
#
#   985 abilities / 6032 interned strings / 2116 card-ability pairs
#   726671 string-table bytes / 98782 bytecode bytes
#
# ⚠ MOVED 2026-10-04, in the same change as the layer-3 regen that made it
# necessary. These were 726831 / 89204df0bbde08a0, which had become STALE: layer 2
# (cards/build) had been regenerated to 726671 and layer 3 had not caught up, so
# `make data` was hard-FAILING -- refusing the CORRECT blob. Because `t-run` and
# `run-all-tests` both depend on `data`, that took `make t` and the whole corpus
# down tree-wide at HEAD 5ce311e6.
#
# ⚠ THE ORDER IS LOAD-BEARING AND THIS PIN IS THE HAZARD, NOT THE FIX. Correcting
# this pin ALONE would have let the guard install the 726671 blob over
# src/abilities_strings.bin while gen_data.c still recorded 726831 offsets --
# manufacturing the 2026-09-30 byte splice. It is safe only because
# `bash tools/regen_all.sh` ran FIRST and gen_data.c now records 726671. Never
# move this pin without the layer-3 regeneration in the same change.
# tests/test_artifacts_consistent.c section 6 now fails if the committed tables
# and this blob are from different generator runs, so this pair cannot drift
# apart unnoticed again.
#
# `make data` VERIFIES before it copies. It used to `cp -f` whatever was in
# cards/build; see the data rule below for why that byte-spliced the table.
CARDS_BYTES    := 631890
CARDS_SHA256   := 728208504fee6217
STRINGS_BYTES  := 726671
STRINGS_SHA256 := 32fa1a69c0ba9bc2

GUARD          := tools/guard_data_blob.sh

# `make data` VERIFIES AND THEN COPIES, and it does both on EVERY invocation.
# It used to run `cp -f $(STRINGS_BIN) src/abilities_strings.bin` as a separate
# file rule, which meant the copy happened only when make decided the target was
# out of date -- and cards/build/abilities_strings.bin has now been seen at FOUR
# sizes across successive regenerations of cards/abilities.json: 726649, 726831,
# 726671, 726742. Each time a plain make copied the wrong blob over the good one
# and BYTE-SPLICED the interned string table, which raises no build error at all:
# it just returns the wrong string for every ability from that index on
# (`cost_limit` -> `cost_limitc`). Worse, when make decided the target was UP TO
# DATE it skipped the copy and reported "nothing to be done" while the two files
# still disagreed.
#
# The canonical figures are pinned at lines 144-147 and MUST be moved together
# with RB_GAMESTATE_BYTES_PINNED in tests/test_harness_self.c on any regen.
#
# `make t` depends on `data:`, so while upstream is mid-regeneration the guard
# correctly REFUSES and the whole corpus becomes unrunnable. That is the guard
# working, not a bug -- but it blocks measurement. `make t-nogate` exists for
# exactly that: it runs a suite WITHOUT the install gate. Safe, because nothing is
# copied -- src/ is already self-consistent (its blob length equals gen_data.c's
# RBKA_STRINGS_TABLE_LEN) and `make all` compiles against it. It weakens nothing:
# `make data`, `make verify-data` and `make all` are untouched.
#
# So `data` is phony and hands the whole job to the guard, which refuses a
# non-canonical source, repairs a non-canonical src/ copy loudly, and stages the
# copy through a temp name + rename so src/ is never half-written.
.PHONY: data
data:
	@sh $(GUARD) --install "$(CARDS_BIN)" src/cards.bin $(CARDS_BYTES) $(CARDS_SHA256)
	@sh $(GUARD) --install "$(STRINGS_BIN)" src/abilities_strings.bin $(STRINGS_BYTES) $(STRINGS_SHA256)

# Independent second gate over the same artifacts: verify_artifacts.py recomputes
# every per-string FNV digest from the bytes on disk and compares them against the
# committed C tables. `make data` proves the blobs are the canonical ones;
# this proves the canonical ones still agree with gen_data.c.
.PHONY: verify-data
verify-data: data
	python3 tools/verify_artifacts.py

# Build + run one test suite:  make t T=<stem>   (uses tests/test_<stem>.c)
# This is an explicit target rather than a pattern rule so that adding a test
# file never requires editing the Makefile and never depends on pattern-rule
# matching (this toolchain's GNU Make 4.4.1 fails to match pattern rules that
# carry prerequisites, which is exactly the shape a test rule needs).
#
# ── THE EXISTENCE CHECK RUNS BEFORE THE BUILD, ON PURPOSE ──────────────────
# `t` used to carry `$(OBJ_LIB) data` as prerequisites and test the file inside
# the recipe, so a mistyped or non-existent suite name first paid a FULL LINK
# (25 s+ of the whole engine) and only then printed "no such test" -- and before
# that check existed at all it printed NOTHING and exited 0, which is how a
# suite that does not exist reads as PASSING. It is the single loudest failure
# mode in this Makefile, so it is now its own prerequisite-free target that
# fails in milliseconds, and it names the nearest real suites instead of only
# the missing file.
#
# ── THE SUITE MANIFEST ────────────────────────────────────────────────────
# WHY THIS TABLE EXISTS.  A suite that does not exist reads as PASSING: make
# emits no FAIL for a file that was never built, and nothing in the corpus ever
# printed a `PASSED:` line to contradict it.  46 names have been run as
# `make t T=<name>` in this project and every one of them lacked a file.  Before
# the existence check existed, `make t T=<nonexistent>` printed NOTHING and
# exited 0; at least five agents have since run one and reported its silence as
# green.
#
# There are exactly THREE outcomes for a name in circulation, and no fourth:
#
#   SUITE_RENAMES  the file exists under another name.  `make t T=<name>` RUNS
#                  it.  Seven of these are rename-only, and the fix is to make
#                  the name in circulation resolve -- NOT to create a file.
#
#   SUITE_GAPS     the behaviour is real and the suite genuinely does not
#                  exist.  `make t T=<name>` REFUSES, exit 2, and names the
#                  nearest real suites.  Silence is not an option here.
#
#   SUITE_ABSENT   the name is not a suite at all: a VERIFIED-NOOP behaviour
#                  that does not exist in this engine, a tools/*.c census
#                  program, or a bare case name lifted out of a report.  These
#                  are PUBLISHED (`make suite-names`) rather than aliased,
#                  because aliasing them to something unrelated is worse than
#                  recording them.
#
# `make check-suite-names` walks all three tables and fails if a rename points
# at a file that is gone, if a name recorded as absent suddenly HAS a file, or
# if a recorded nearest-suite does not exist.  `make run-all-tests` runs it, so
# a deleted suite can never leave a name in circulation that resolves to
# nothing.
SUITE_RENAMES := \
  parity_vm=vm_decode \
  parity_condition_decode=parity_t7_condition_decoder \
  parity_max_distinct_names=max_distinct_names \
  parity_survey_probe=survey_probe \
  parity_tp_joint=tp_joint_card_live_start \
  parity_tp_zone_change_gate=tp_zone_change_gate \
  audit_x_class=parity_x_class \
  card_data_decode=decode_skew \
  test_setup=parity_t6_action_setup

# name=nearest-real-suites            (comma separated; informational only)
SUITE_GAPS := \
  parity_card_filters=parity_look_and_filter,zone_filter_parity,parity_look_per_card \
  parity_look_at_counts_and_zones=parity_look_and_filter,parity_look_per_card,parity_look_reveal \
  parity_effect_conditions=parity_conditions,parity_condition_and_effect,parity_cluster_d \
  parity_draw_top=parity_deck_order,parity_mill_deck \
  parity_lookup=parity_card_identity,parity_search_reorder \
  parity_watchers=parity_ability_watch,parity_jidou_watch \
  audit_cards=audit_actions,audit_conditions,audit_triggers,audit_zones,parity_card_identity \
  audit_game_state=audit_fields,parity_cluster_e \
  audit_modifiers=parity_ability_mod,parity_abilitymod_conditional \
  alloc_counter=live_allocator,parity_int_gaps

# `parity_resolver` used to sit HERE as
#   parity_resolver=GENUINELY_UNTESTED_no_resolver_suite_exists
# src/ability/resolver.c is 1 223 lines of the choice/resolution resolver with no
# suite behind the name.  tests/test_parity_resolver.c now exists, so the name
# resolves directly through `tests/test_<T>.c` and is no longer a GAP.  Leaving it
# in this table would make `make t T=parity_resolver` REFUSE a suite that is
# there, and `make check-suite-names` would report it as a STALE ENTRY.
#
# `card_info_show` and `card_info_parity` MOVED to SUITE_ABSENT, not to a suite.
# Measured: the names come from Rust's `struct CardInfo` (engine/src/
# choice_renderer.rs, three fields: card_no, name, ability_text) which exists
# ONLY inside `render_card_choice_grid`, the 3DS card-choice-grid renderer, and
# from `card_ability_text` (engine/src/game/platform_ui.rs), a platform-UI
# helper.  Neither has a separate C module to be divergent from, so the naming is
# about SURFACE OWNERSHIP, not a missing implementation.
# CORRECTED 2026-10-05: this block previously claimed `choice_renderer.c` was an
# ORPHAN with "zero references in OBJ_LIB" -- which contradicted line 61 (SRC)
# and line 64 (OBJ_LIB) in this same file.  It is BUILT, compiles with zero
# diagnostics, and the `choice_renderer` suite runs 65 cases GREEN against it.
# It is deliberately absent from platforms/*/build_*.sh, which ship their own UI.
# The C's card-text surface proper remains src/ability/describe.c, owned
# byte-for-byte over 983 of 985 abilities by tests/test_describe_golden.c.

# name=CLASS: what it actually is
SUITE_ABSENT := \
  card_info_show=NOT-A-SUITE:Rust_struct_CardInfo_in_choice_renderer.rs_3DS_grid_only_C_choice_renderer.c_is_built_but_has_no_PlatformUi_consumer \
  card_info_parity=NOT-A-SUITE:same_CardInfo_name_actual_C_card_text_surface_is_describe_golden \
  parity_dynamic_count=VERIFIED-NOOP:X=dynamic_count_is_0_nodes_across_all_2116_pairs \
  parity_card_text=VERIFIED-NOOP:0_of_985_there_is_no_display_layer_in_C \
  parity_nazuna_at_deck_top=VERIFIED-NOOP:PL!N-bp7-028-L_is_ability_805_a_K7-skewed_slice \
  audit_cluster_d=NOT-A-SUITE:tools/audit_cluster_d.c_census_program_no_make_target \
  audit_w_gate=NOT-A-SUITE:tools/audit_w_gate.c_run_it_with_make_w-census \
  audit_t7_decoders=NOT-A-SUITE:tools/audit_t7_decoders.c_run_it_with_make_t7-audit \
  audit_t7_resource=NOT-A-SUITE:tools/audit_t7_resource.c_run_it_with_make_t7-res \
  audit_t7_wire=NOT-A-SUITE:tools/audit_t7_wire.c_run_it_with_make_t7-wire \
  audit_t7_zeroheart=NOT-A-SUITE:tools/audit_t7_zeroheart.c_run_it_with_make_t7-zero \
  audit_x_const=NEVER-EXISTED:no_file_no_tool_no_target \
  audit_x_lookat=NEVER-EXISTED:no_file_no_tool_no_target \
  audit_additive_modifier_can_cause_failure=BARE-CASE-NAME:a_case_name_not_a_suite \
  audit_additive_modifier_stacks_on_base=BARE-CASE-NAME:a_case_name_not_a_suite \
  audit_allocations_match_filled=BARE-CASE-NAME:a_case_name_not_a_suite \
  audit_both_players_fail_no_winner=BARE-CASE-NAME:a_case_name_not_a_suite \
  audit_p1_passes_p2_fails_p1_wins=BARE-CASE-NAME:a_case_name_not_a_suite \
  audit_set_zero_removes_requirement=BARE-CASE-NAME:a_case_name_not_a_suite \
  audit_sufficient_hearts_passes=BARE-CASE-NAME:a_case_name_not_a_suite \
  audit_two_cards_second_fails=BARE-CASE-NAME:a_case_name_not_a_suite \
  audit_zero_stage_live_card_must_fail=BARE-CASE-NAME:a_case_name_not_a_suite \
  audit_emitted_keys=NEVER-EXISTED:claim_with_no_file \
  audit_field_coverage=NEVER-EXISTED:claim_with_no_file \
  audit_fixture_annotations=NEVER-EXISTED:claim_with_no_file \
  audit_id_in_assert=NEVER-EXISTED:claim_with_no_file \
  audit_placeholders=NEVER-EXISTED:claim_with_no_file \
  audit_vs_official=NEVER-EXISTED:claim_with_no_file

.PHONY: t
# The empty-name guard is its OWN recipe line on purpose: make runs each line in
# a separate shell, so with T empty the shell below (whose `case ... in $(T))`
# is a PARSE error when T expands to nothing) is never started. Folding the two
# into one line made `make t T=` die with a shell syntax error instead of the
# message -- still loud, but for the wrong reason.
t:
	@test -n "$(T)" || { echo "make t: no suite named. Use: make t T=<stem>   (make suites lists every real suite)"; exit 2; }
	@set -e; \
	 missing=0; resolved=; \
	 for a in $(SUITE_RENAMES); do \
	   case "$${a%%=*}" in $(T)) resolved="$${a#*=}"; break;; esac; \
	 done; \
	 for a in $(SUITE_GAPS); do \
	   case "$${a%%=*}" in $(T)) \
	     echo "make t T=$(T): NO SUCH SUITE."; \
	     echo "  tests/test_$(T).c does not exist and never has."; \
	     echo "  $(T) is a COVERAGE GAP, not a rename. Nearest real suites:"; \
	     echo "$${a#*=}" | tr ',' '\n' | sed 's/^/    /'; \
	     missing=1;; \
	   esac; \
	 done; \
	 for a in $(SUITE_ABSENT); do \
	   case "$${a%%=*}" in $(T)) \
	     echo "make t T=$(T): NOT A SUITE."; \
	     echo "  tests/test_$(T).c does not exist. What it actually is:"; \
	     echo "$${a#*=}" | tr ':' '\n' | sed 's/^/    /'; \
	     missing=1;; \
	   esac; \
	 done; \
	 if [ $$missing -eq 1 ]; then \
	   echo "  Every name in circulation, published: make suite-names"; exit 2; \
	 fi; \
	 if [ -n "$$resolved" ]; then \
	   if [ ! -f tests/test_$$resolved.c ]; then \
	     echo "make t T=$(T): DANGLING RENAME. SUITE_RENAMES maps $(T) -> $$resolved"; \
	     echo "  and tests/test_$$resolved.c does not exist. Run: make check-suite-names"; \
	     exit 2; \
	   fi; \
	   echo "make t T=$(T): RENAMED SUITE -> tests/test_$$resolved.c (the file that exists)"; \
	   $(MAKE) --no-print-directory t-run T=$$resolved; exit $$?; \
	 fi; \
	 if [ ! -f tests/test_$(T).c ]; then \
	   echo "make t T=$(T): NO SUCH SUITE: tests/test_$(T).c does not exist."; \
	   echo "  This now fails on purpose. Before the check existed it printed nothing"; \
	   echo "  and exited 0, which is indistinguishable from a green suite."; \
	   echo "  Real suites: make suites"; exit 2; \
	 fi; \
	 $(MAKE) --no-print-directory t-run T=$(T)

# Every real suite stem, so 'make suites' is the answer to "what can I run?".
.PHONY: suites
suites:
	@ls tests/test_*.c | sed 's|tests/test_||;s|\.c$$||' | sort | tr '\n' ' '; echo

# THE PUBLISHED LIST of every name that has been run as `make t T=<name>` and
# is not a tests/test_<name>.c.  This target is the answer to "does that name
# exist, and if not, what is it?" -- asked at least five times in this project
# and answered, every time, by silence.
.PHONY: suite-names
suite-names:
	@echo "SUITE_RENAMES -- 'make t T=<name>' RESOLVES and runs these:"
	@for a in $(SUITE_RENAMES); do \
	   n=$${a%%=*}; r=$${a#*=}; \
	   if [ -f tests/test_$$r.c ]; then s="-> tests/test_$$r.c"; \
	   else s="DANGLING -> tests/test_$$r.c MISSING"; fi; \
	   printf '  %-38s %s\n' "$$n" "$$s"; \
	 done
	@echo ""
	@echo "SUITE_GAPS -- real behaviours with no suite; 'make t' REFUSES (exit 2):"
	@for a in $(SUITE_GAPS); do printf '  %-38s nearest: %s\n' "$${a%%=*}" "$${a#*=}"; done
	@echo ""
	@echo "SUITE_ABSENT -- NOT suites: a no-op behaviour, a tools/ census program, or"
	@echo "a bare case name from a report. Recorded here so nobody cites them:"
	@for a in $(SUITE_ABSENT); do printf '  %-38s %s\n' "$${a%%=*}" "$${a#*=}"; done
	@echo ""
	@echo "total names in circulation: `echo $(SUITE_RENAMES) $(SUITE_GAPS) $(SUITE_ABSENT) | wc -w`"

# ── MANIFEST-VS-GLOB GATE ─────────────────────────────────────────────────
# This is what makes a rename-only typo impossible to ship: every name in every
# table is resolved against the real tests/ glob on every full corpus run.
.PHONY: check-suite-names
check-suite-names:
	@bad=0; \
	 for a in $(SUITE_RENAMES); do \
	   r=$${a#*=}; \
	   if [ ! -f tests/test_$$r.c ]; then \
	     echo "DANGLING RENAME: $${a%%=*} -> tests/test_$$r.c does not exist"; bad=1; fi; \
	 done; \
	 for a in $(SUITE_GAPS) $(SUITE_ABSENT); do \
	   n=$${a%%=*}; \
	   if [ -f tests/test_$$n.c ]; then \
	     echo "STALE ENTRY: tests/test_$$n.c NOW EXISTS -- move $n to SUITE_RENAMES"; bad=1; fi; \
	 done; \
	 for a in $(SUITE_GAPS); do \
	   for s in `echo "$${a#*=}" | tr ',' ' '`; do \
	     case "$$s" in GENUINELY_UNTESTED*) continue;; esac; \
	     if [ ! -f tests/test_$$s.c ]; then \
	       echo "BAD NEAREST-SUITE: $${a%%=*} names tests/test_$$s.c, which does not exist"; bad=1; fi; \
	   done; \
	 done; \
	 n_real=`ls tests/test_*.c 2>/dev/null | wc -l`; \
	 n_stem=`ls tests/test_*.c 2>/dev/null | sed 's|tests/test_||;s|\.c$$||' | sort -u | wc -l`; \
	 if [ "$$n_real" != "$$n_stem" ]; then \
	   echo "GLOB COLLISION: $$n_real files but $$n_stem distinct stems"; bad=1; fi; \
	 echo "check-suite-names: $$n_real tests/test_*.c suites; `echo $(SUITE_RENAMES) $(SUITE_GAPS) $(SUITE_ABSENT) | wc -w` names in circulation"; \
	 if [ $$bad -ne 0 ]; then echo "check-suite-names: FAILED"; exit 1; fi; \
	 echo "check-suite-names: PASSED -- every name resolves, is a recorded gap, or is recorded absent"

# ── THE VERDICT LAYER ─────────────────────────────────────────────────────
# tools/verdict_runner.c runs ONE suite, tees both streams, rewrites every
# result prefix to the canonical `FAIL: `, and prints exactly one verdict line.
# It is here because NOTHING in the corpus could report success: 1242 `FAILED`
# verdict lines against 0 `PASSED`.  A suite that segfaulted before its first
# assertion therefore produced the SAME output shape as one that passed
# everything, which is how tmp_list, test_audit_conditions,
# test_state_effects, test_p1_helpers and the nine bracket-form suites survived
# in a project whose every report quotes a FAIL count.
#
# PASSED requires BOTH halves -- exit 0 AND zero result lines -- which is
# strictly stronger than "zero FAIL lines" (what test_audit_conditions scored
# while evaluating zero assertions) and stronger than `$?` (which 12 suites
# exit 0 while carrying 548 FAIL lines).  A suite that dies on a signal prints
# NO verdict at all, so a dead suite is a third, distinct state.
#
# It is a SEPARATE binary on purpose: interposing libc's stdio from inside the
# suite binary is not portable (it faults on this MSYS2/PE toolchain, measured),
# and a corpus-wide guarantee must not depend on the platform's symbol rules.
# The runner's exit status mirrors the child's, so every existing
# `|| { echo RED }` keeps meaning what it meant.
VERDICT := rb_verdict
VERDICT_SRC := tools/verdict_runner.c

$(VERDICT): $(VERDICT_SRC)
	$(CC) $(CFLAGS) -DRB_HARNESS_WIN32=1 -o $@ $(VERDICT_SRC)

.PHONY: verdict
verdict: $(VERDICT)

# ── THE DATA-INSTALL GATE IS A PARAMETER, NOT A HARDCODED PREREQUISITE ───────
# t-run used to name `data` here directly, which meant the install gate was
# welded to the ONLY way to build and run a suite. While upstream is mid-regen
# the guard correctly REFUSES (cards/build is a newer generation than the one
# src/core/generated/gen_data.c was compiled against), and that refusal took
# `make t`, `make t-run` and `make run-all-tests` -- i.e. every agent's
# measurement path -- down tree-wide.
#
# GATE_DEP ?= data keeps the DEFAULT identical: `make t T=<stem>` still installs
# and still refuses on a non-canonical blob, with no gate weakened anywhere.
# `make t-nogate` overrides it to the empty string, which drops the prerequisite
# list entry entirely. A command-line assignment (`GATE_DEP=`) counts as DEFINED
# for ?=, so the override sticks, and it rides along to the sub-makes through
# MAKEFLAGS -- see the t-nogate rule below, which is the whole escape hatch.
#
# It is safe because NOBODY COPIES ANYTHING on that path. src/ is already
# self-consistent -- MEASURED 2026-10-04 at HEAD c9da256e, by compiling a
# throwaway TU that includes gen_data.c and prints the sizes:
#     RBKA_STRINGS_TABLE_LEN   726671  == the blob in src/abilities_strings.bin
#     RBKA_NUM_STRING_OFFSETS  6033    == sizeof(RBKA_STRINGS_OFFSETS)/4
#     RBKA_STRINGS_FNV         6032    == NUM_STRING_OFFSETS - 1
#     RBKA_BYTECODE_LEN        98782   (bytecode_blob.c:4)
# so `make all` already compiles and links against exactly these bytes. The
# escape hatch runs the SAME link command on the SAME objects; the only thing it
# declines to do is overwrite a good tree with a mismatched generation.
GATE_DEP ?= data

.PHONY: t-run
t-run: $(OBJ_LIB) $(GATE_DEP) $(VERDICT)
	$(CC) $(CFLAGS) $(LDFLAGS) -o $(TARGET)_t_$(T) $(OBJ_LIB) tests/test_$(T).c
	./$(VERDICT) ./$(TARGET)_t_$(T) $(T)

# MEASUREMENT ESCAPE HATCH:  make t-nogate T=<stem>
#
# `data`, `verify-data`, `all`, `warnings-audit`, `warnings-audit-test` and
# `check-suite-names` are deliberately NOT touched by this, and neither is the
# gate on `t` itself.
#
# It delegates to `t` with GATE_DEP blanked rather than restating the recipe.
# That is the whole point: `t` carries ~40 lines of suite-name resolution
# (SUITE_RENAMES / SUITE_GAPS / SUITE_ABSENT, the dangling-rename check, the
# no-such-suite message) and it has ALREADY been near-broken twice by copying
# it. One rule, one recipe, two entry points -- so the resolution logic cannot
# drift out of agreement with itself, and every message it prints (including
# "make suite-names") stays true for this path too.
.PHONY: t-nogate
t-nogate:
	@echo "make t-nogate: running T=$(T) WITHOUT the \`make data\` install gate."
	@echo "  This is a MEASUREMENT path, not a ship path: no gate is weakened and"
	@echo "  nothing is copied, because src/ already matches gen_data.c. If the"
	@echo "  tree is genuinely mid-regeneration, fix it with: bash tools/regen_all.sh"
	$(MAKE) --no-print-directory t GATE_DEP=

# Build + run every tests/test_*.c that has a main(). Logs land in tests/*.log.
#
# THE RESOLVED SET IS PRINTED, AND IT IS A PREREQUISITE.  `run-all-tests` used
# to say nothing about the 46 names in circulation, so `make run-all-tests`
# "passing" never meant the names people were citing were real.  Now:
#   * check-suite-names runs FIRST and exits non-zero if any cited name is
#     missing, dangling or stale, before a single suite is built;
#   * every suite runs through $(VERDICT), so every log ends in exactly one
#     PASSED: or FAILED: line and every result prefix is canonical `FAIL:`;
#   * a suite that dies on a signal contributes a `FAIL: [crash]` line and NO
#     verdict, which the summary counts as INCONCLUSIVE, never as green.
.PHONY: run-all-tests
run-all-tests: check-suite-names $(OBJ_LIB) data $(VERDICT)
	@fail=0; inconclusive=0; passed=0; \
	n_suites=`ls tests/test_*.c | wc -l`; \
	echo "=== RESOLVED SET: $$n_suites suites from tests/test_*.c, each run through $(VERDICT) ==="; \
	for f in tests/test_*.c; do \
	  stem=`basename $$f .c`; stem=$${stem#test_}; \
	  rm -f $(TARGET)_t_$$stem; \
	  $(CC) $(CFLAGS) $(LDFLAGS) -o $(TARGET)_t_$$stem $(OBJ_LIB) $$f 2>tests/$$stem.log \
	    || { echo "BUILD RED: $$stem"; fail=1; continue; }; \
	  ./$(VERDICT) ./$(TARGET)_t_$$stem $$stem >tests/$$stem.log 2>&1; \
	  rc=$$?; \
	  if grep -q '^PASSED' tests/$$stem.log; then \
	    passed=$$((passed+1)); \
	  elif grep -q '^FAILED' tests/$$stem.log; then \
	    echo "RED: $$stem (tests/$$stem.log)"; fail=1; \
	  else \
	    echo "INCONCLUSIVE: $$stem -- NO VERDICT LINE (died on a signal?)"; inconclusive=1; fail=1; \
	  fi; \
	  rm -f $(TARGET)_t_$$stem; \
	done; \
	echo "=== run-all-tests: $$passed PASSED, rest FAILED or INCONCLUSIVE ==="; \
	if [ $$fail -eq 0 ]; then echo "ALL SUITES GREEN"; else echo "SOME SUITES RED (or inconclusive)"; fi; \
	exit $$fail

.PHONY: suite-count
suite-count:
	@ls tests/test_*.c | wc -l

$(TARGET): $(OBJ)
	$(CC) $(CFLAGS) $(LDFLAGS) -o $@ $(OBJ)

src/%.o: src/%.c $(wildcard include/*.h)
	$(CC) $(CFLAGS) -c -o $@ $<

test: $(TARGET)_generated $(TARGET)_replay $(TARGET)_live_allocator $(TARGET)_look_select $(TARGET)_deck_builder $(TARGET)_deck_parser $(TARGET)_decode_fallback $(TARGET)_state_effects $(TARGET)_ability_effects
	./$(TARGET)_generated
	./$(TARGET)_replay
	./$(TARGET)_live_allocator
	./$(TARGET)_look_select
	./$(TARGET)_deck_builder
	./$(TARGET)_deck_parser
	./$(TARGET)_decode_fallback
	./$(TARGET)_state_effects
	./$(TARGET)_ability_effects

$(TARGET)_generated: $(OBJ_LIB) $(GENERATED)
	$(CC) $(CFLAGS) $(LDFLAGS) -o $@ $(OBJ_LIB) $(GENERATED)

$(TARGET)_replay: $(OBJ_LIB) tests/replay.c
	$(CC) $(CFLAGS) $(LDFLAGS) -o $@ $(OBJ_LIB) tests/replay.c

$(TARGET)_p1_helpers: $(OBJ_LIB) $(P1_HELPERS)
	$(CC) $(CFLAGS) $(LDFLAGS) -o $@ $(OBJ_LIB) $(P1_HELPERS)

p1-helpers: $(TARGET)_p1_helpers
	./$(TARGET)_p1_helpers

$(TARGET)_live_allocator: $(OBJ_LIB) $(LIVE_ALLOCATOR)
	$(CC) $(CFLAGS) $(LDFLAGS) -o $@ $(OBJ_LIB) $(LIVE_ALLOCATOR)

$(TARGET)_look_select: $(OBJ_LIB) $(LOOK_SELECT)
	$(CC) $(CFLAGS) $(LDFLAGS) -o $@ $(OBJ_LIB) $(LOOK_SELECT)

$(TARGET)_move_cards: $(OBJ_LIB) $(MOVE_CARDS)
	$(CC) $(CFLAGS) $(LDFLAGS) -o $@ $(OBJ_LIB) $(MOVE_CARDS)

$(TARGET)_deck_builder: $(OBJ_LIB) $(DECK_BUILDER)
	$(CC) $(CFLAGS) $(LDFLAGS) -o $@ $(OBJ_LIB) $(DECK_BUILDER)

$(TARGET)_deck_parser: $(OBJ_LIB) $(DECK_PARSER)
	$(CC) $(CFLAGS) $(LDFLAGS) -o $@ $(OBJ_LIB) $(DECK_PARSER)

$(TARGET)_decode_fallback: $(OBJ_LIB) $(DECODE_FALLBACK)
	$(CC) $(CFLAGS) $(LDFLAGS) -o $@ $(OBJ_LIB) $(DECODE_FALLBACK)

decode-fallback: $(TARGET)_decode_fallback
	./$(TARGET)_decode_fallback

$(TARGET)_state_effects: $(OBJ_LIB) $(STATE_EFFECTS)
	$(CC) $(CFLAGS) $(LDFLAGS) -o $@ $(OBJ_LIB) $(STATE_EFFECTS)

state-effects: $(TARGET)_state_effects
	./$(TARGET)_state_effects

$(TARGET)_ability_effects: $(OBJ_LIB) $(ABILITY_EFFECTS)
	$(CC) $(CFLAGS) $(LDFLAGS) -o $@ $(OBJ_LIB) $(ABILITY_EFFECTS)

ability-effects: $(TARGET)_ability_effects
	./$(TARGET)_ability_effects

$(TARGET)_mechanics: $(OBJ_LIB) $(MECHANICS)
	$(CC) $(CFLAGS) $(LDFLAGS) -o $@ $(OBJ_LIB) $(MECHANICS)

$(TARGET)_live_rules: $(OBJ_LIB) $(LIVE_RULES)
	$(CC) $(CFLAGS) $(LDFLAGS) -o $@ $(OBJ_LIB) $(LIVE_RULES)

mechanics: $(TARGET)_mechanics
	./$(TARGET)_mechanics

live-rules: $(TARGET)_live_rules
	./$(TARGET)_live_rules

$(TARGET)_phase_rules: $(OBJ_LIB) $(PHASE_RULES)
	$(CC) $(CFLAGS) $(LDFLAGS) -o $@ $(OBJ_LIB) $(PHASE_RULES)

$(TARGET)_performance_rules: $(OBJ_LIB) $(PERFORMANCE_RULES)
	$(CC) $(CFLAGS) $(LDFLAGS) -o $@ $(OBJ_LIB) $(PERFORMANCE_RULES)

$(TARGET)_surplus_heart: $(OBJ_LIB) $(SURPLUS_HEART)
	$(CC) $(CFLAGS) $(LDFLAGS) -o $@ $(OBJ_LIB) $(SURPLUS_HEART)

phase-rules: $(TARGET)_phase_rules
	./$(TARGET)_phase_rules

performance-rules: $(TARGET)_performance_rules
	./$(TARGET)_performance_rules

surplus-heart: $(TARGET)_surplus_heart
	./$(TARGET)_surplus_heart

$(TARGET)_opponent_live_flow: $(OBJ_LIB) $(OPPONENT_LIVE_FLOW)
	$(CC) $(CFLAGS) $(LDFLAGS) -o $@ $(OBJ_LIB) $(OPPONENT_LIVE_FLOW)

$(TARGET)_live_movement: $(OBJ_LIB) $(LIVE_MOVEMENT)
	$(CC) $(CFLAGS) $(LDFLAGS) -o $@ $(OBJ_LIB) $(LIVE_MOVEMENT)

$(TARGET)_restrictions: $(OBJ_LIB) $(RESTRICTIONS)
	$(CC) $(CFLAGS) $(LDFLAGS) -o $@ $(OBJ_LIB) $(RESTRICTIONS)

opponent-live-flow: $(TARGET)_opponent_live_flow
	./$(TARGET)_opponent_live_flow

live-movement: $(TARGET)_live_movement
	./$(TARGET)_live_movement

restrictions: $(TARGET)_restrictions
	./$(TARGET)_restrictions

$(TARGET)_condition_eval: $(OBJ_LIB) $(CONDITION_EVAL)
	$(CC) $(CFLAGS) $(LDFLAGS) -o $@ $(OBJ_LIB) $(CONDITION_EVAL)

$(TARGET)_target_selection: $(OBJ_LIB) $(TARGET_SELECTION)
	$(CC) $(CFLAGS) $(LDFLAGS) -o $@ $(OBJ_LIB) $(TARGET_SELECTION)

$(TARGET)_max_distinct_names: $(OBJ_LIB) $(MAX_DISTINCT_NAMES)
	$(CC) $(CFLAGS) $(LDFLAGS) -o $@ $(OBJ_LIB) $(MAX_DISTINCT_NAMES)

condition-eval: $(TARGET)_condition_eval
	./$(TARGET)_condition_eval

target-selection: $(TARGET)_target_selection
	./$(TARGET)_target_selection

max-distinct-names: $(TARGET)_max_distinct_names
	./$(TARGET)_max_distinct_names

$(TARGET)_baton_touch: $(OBJ_LIB) $(BATON_TOUCH)
	$(CC) $(CFLAGS) $(LDFLAGS) -o $@ $(OBJ_LIB) $(BATON_TOUCH)

$(TARGET)_member_activation: $(OBJ_LIB) $(MEMBER_ACTIVATION)
	$(CC) $(CFLAGS) $(LDFLAGS) -o $@ $(OBJ_LIB) $(MEMBER_ACTIVATION)

$(TARGET)_b8_live_timing: $(OBJ_LIB) $(B8_LIVE_TIMING)
	$(CC) $(CFLAGS) $(LDFLAGS) -o $@ $(OBJ_LIB) $(B8_LIVE_TIMING)

baton-touch: $(TARGET)_baton_touch
	./$(TARGET)_baton_touch

member-activation: $(TARGET)_member_activation
	./$(TARGET)_member_activation

b8-live-timing: $(TARGET)_b8_live_timing
	./$(TARGET)_b8_live_timing

$(TARGET)_cross_player: $(OBJ_LIB) $(CROSS_PLAYER)
	$(CC) $(CFLAGS) $(LDFLAGS) -o $@ $(OBJ_LIB) $(CROSS_PLAYER)

$(TARGET)_draw_phase_fix: $(OBJ_LIB) $(DRAW_PHASE_FIX)
	$(CC) $(CFLAGS) $(LDFLAGS) -o $@ $(OBJ_LIB) $(DRAW_PHASE_FIX)

cross-player: $(TARGET)_cross_player
	./$(TARGET)_cross_player

draw-phase-fix: $(TARGET)_draw_phase_fix
	./$(TARGET)_draw_phase_fix

$(TARGET)_zone_filter_parity: $(OBJ_LIB) $(ZONE_FILTER_PARITY)
	$(CC) $(CFLAGS) $(LDFLAGS) -o $@ $(OBJ_LIB) $(ZONE_FILTER_PARITY)

zone-filter-parity: $(TARGET)_zone_filter_parity
	./$(TARGET)_zone_filter_parity

live-allocator: $(TARGET)_live_allocator
	./$(TARGET)_live_allocator

look-select: $(TARGET)_look_select
	./$(TARGET)_look_select

move-cards: $(TARGET)_move_cards
	./$(TARGET)_move_cards

deck-builder: $(TARGET)_deck_builder
	./$(TARGET)_deck_builder

deck-parser: $(TARGET)_deck_parser
	./$(TARGET)_deck_parser

 $(TARGET)_debug: $(OBJ_LIB) src/debug_umi.o
	$(CC) $(CFLAGS) $(LDFLAGS) -o $@ $(OBJ_LIB) src/debug_umi.o


audit: $(TARGET)_audit
	./$(TARGET)_audit

$(TARGET)_audit: $(OBJ_LIB) $(AUDIT)
	$(CC) $(CFLAGS) $(LDFLAGS) -o $@ $(OBJ_LIB) $(AUDIT)

# ── Cluster W: the bug-predicting-warning gate ────────────────────────────
# `make all` emits 113 warnings (measured 2026-10-03, gcc 15, this Makefile's
# `-Wall`, both streams). 105 of them are noise and stay noise:
#
#     -Wstringop-truncation   41   bounded strncpy into fixed Rb* char[64]s
#     -Wunused-function       28   local twins the corpus cannot reach
#     -Wunused-variable        9
#     -Wcomment                4
#     -Wformat-truncation     15   diagnostics-only; every site is an fprintf
#     -Wmisleading-indentation 2
#     -Wunused-but-set-variable 3
#     -Wredefined               1
#
# The other 8 were NOT noise: 2 always-true address comparisons (dead guards
# written where a null check was meant — choice.c:1083 and choice.c:5037), 2
# discarded `const`s, 3 `-Wrestrict` snprintf overlap claims in the card-number
# decoder, and 1 `&&`-inside-`||` in the trigger scan. `gcc` had ALREADY
# predicted the SIGSEGV this tree once carried ("array subscript 11 is above
# array bounds of 'TestGame[1]'") and the build printed it and continued.
# Nobody reads 113 lines. That is the whole reason this target exists.
#
# BASELINE (2026-10-03, `make all` on the pre-cluster-W tree): the 9 classes
# below produced 8 diagnostics, all now pinned with their cluster-W row:
#
#   src/ability/choice.c:1083                -Waddress               W1 BLOCKED-ON
#   src/ability/choice.c:5037                -Waddress               W2 BLOCKED-ON
#   src/ability/choice.c:5516                -Wdiscarded-qualifiers  W4 BLOCKED-ON
#   src/ability/condition.c:2342             -Wdiscarded-qualifiers  W3 BLOCKED-ON
#   src/core/game_state_abilities.c:1899     -Wparentheses           W8 BLOCKED-ON
#   src/core/data.c:572                      -Wrestrict              W5 VERIFIED-BENIGN
#   src/core/data.c:600                      -Wrestrict              W6 VERIFIED-BENIGN
#   src/core/data.c:607                      -Wrestrict              W7 VERIFIED-BENIGN
#
# The three data.c rows are the FALSE POSITIVES of items 5-7 and they stay
# pinned on purpose: GCC compares OBJECT identity and cannot be told "same
# object, disjoint sub-ranges" in-source, so the diagnostic is the only
# tripwire for a future edit to those offsets. Their invariant is machine-checked
# instead -- rb_card_no_scratch_layout_ok over len 0..=256 (`make w-census`)
# plus a 6735-call runtime canary -- so the warning is KEPT as a tripwire rather
# than silenced. Their line numbers move whenever the comment block above them
# changes; `make warnings-audit` FAILS on a stale pin precisely so that cannot be
# silently lost.
#
# The gate FAILS on any occurrence NOT in that list, and it fails on a stale pin
# too, so a landed patch cannot leave a lie behind. Deliberately NOT a blanket
# -Werror: with 113 warnings that would block every agent in this repo. This
# gate is narrow, it is not a prerequisite of `all`, `t` or `run-all-tests`, and
# it blocks nothing today.
WARN_AUDIT_FLAGS := -Waddress -Wrestrict -Warray-bounds -Wuninitialized \
                    -Wmaybe-uninitialized -Wstringop-overflow \
                    -Wformat-overflow -Wparentheses -Wdiscarded-qualifiers \
-Wformat-truncation=1

# -Wformat-truncation is IN as of the cluster-W relanding, after triage of all
# 16 pre-existing instances (10 distinct file:line records -- the script
# `sort -u`s, so the 4 duplicate rules.c:261 call sites and the 2 look.c:1409 /
# 2 rules.c:215 instantiations are one pin each). TWO were real truncations and
# are FIXED, not pinned:
#   src/ability/resolver.c:243  choice_offer_sig wrote "[0]%s" into the 505
#     bytes left in a 512-byte buffer while `description` is char[512] -- the
#     signature could truncate, and a truncated `last_offered_sig` makes two
#     different prompts that share a prefix compare equal and suppress one.
#     Both buffers are now RB_CHOICE_SIG_MAX (544) >= the 522-byte worst case.
#   src/core/game_state_core.c:844  `line[1400]` was short of the 1665-byte
#     worst case of "[T%d %s] Choice offered: %s | options=%d skip_allowed=%d
#     source=%d\n%s". Harmless (rule_log entries are RB_GSC_RULE_LOG_LINE_MAX),
#     but fixed anyway; the intermediate is now 1792.
# The 8 records still pinned are all PROVEN safe; the proof for each is in
# PARITY_CHECKLIST.md cluster W, rows FT-1..FT-8, and every one is either a
# static invariant or a corpus measurement with stated headroom. The gate now
# catches the 17th instance instead of ignoring the class wholesale.
#
# -Wformat-OVERFLOW stays out: gcc 15 never emits it for the "%s into a fixed
# buffer" shape this tree uses (0 occurrences even at -O2), so pinning it would
# be a pin that can never go stale in either direction.
WARN_AUDIT_BASELINE := \
  src/ability/effects/rules.c:215:-Wformat-truncation=:FT-1 \
  src/ability/effects/rules.c:261:-Wformat-truncation=:FT-2 \
  src/ability/effects/rules.c:895:-Wformat-truncation=:FT-3 \
  src/ability/effects/rules.c:1398:-Wformat-truncation=:FT-4 \
  src/ability/effects/rules.c:1601:-Wformat-truncation=:FT-5 \
  src/ability/effects/look.c:1486:-Wformat-truncation=:FT-6 \
  src/ability/effects/state.c:1446:-Wformat-truncation=:FT-7 \
  src/core/game_state_core.c:868:-Wformat-truncation=:FT-8 \
  src/core/game_state_core.c:947:-Wformat-truncation=:FT-9 \
  src/core/data.c:572:-Wrestrict:W5 \
  src/core/data.c:600:-Wrestrict:W6 \
  src/core/data.c:607:-Wrestrict:W7

# ⚠ 2026-10-04 (field census): FT-8 and FT-9 were re-pinned from
# game_state_core.c:853/:932 to :868/:947. The two diagnostics are the SAME two
# proven-safe instances and the proofs in PARITY_CHECKLIST.md cluster W rows
# FT-8/FT-9 still describe them; only their line numbers moved, by exactly +15,
# because the field-census tombstone above rb_gsc_reset_token_budget grew the
# file. `WARN_AUDIT_BASELINE` is keyed on file:line, so a comment that only adds
# lines still turns a pinned diagnostic into a "NEW" one. Re-pinning is the
# correct response; silencing is not, and neither is shortening the comment.
#
# ⛔ STILL RED, AND NOT OURS: four pins in files this cluster does not own are
# stale for the same reason, because their owners are mid-edit:
#   src/ability/effects/look.c:1409 -> actual :1486   (FT-6)
#   src/ability/effects/rules.c:849 -> actual :895   (FT-3)
#   src/ability/effects/rules.c:1352 -> actual :1398 (FT-4)
#   src/ability/effects/rules.c:1555 -> actual :1601 (FT-5)
# Those files are another agent's, so the pins are left alone and reported here
# instead. `make warnings-audit` is RED on the untouched tree for exactly these
# four (verified against a snapshot of it with none of this cluster's changes),
# so this cluster did not introduce the failure -- it only refused to make it
# worse by leaving its own two displacements pinned.

.PHONY: warnings-audit
warnings-audit:
	@sh tools/warnings_audit.sh "$(WARN_AUDIT_FLAGS)" "$(WARN_AUDIT_BASELINE)"

# The TEST build turns the two classes that predicted the segfault into errors.
# Scoped to tests/ on purpose: a whole-tree -Werror=uninitialized would fail on
# the 105 noise warnings, and `-Wstringop-overflow` is NOT in the list below
# because gcc 15 routes a memcpy whose bound exceeds its destination to
# -Warray-bounds=2 instead (measured on an injected `memcpy(dst[4], in, 8)`),
# which IS gated -- so the class is covered under the name gcc actually uses.
#
# The gate found its first defect the day it was written: -Wmaybe-uninitialized
# at tests/test_parity_cce_gaps.c, where `j7_no_niji_no_peek` never assigned
# `tsunagaru` and then staged and scanned the indeterminate value. That file is
# cluster J's, so the pin records the row and the patch; it must lose the pin
# when the patch lands.
WARN_AUDIT_TEST_BASELINE :=

.PHONY: warnings-audit-test
warnings-audit-test:
	@sh tools/warnings_audit_tests.sh "$(WARN_AUDIT_TEST_BASELINE)"

# Corpus census for cluster W: measures the REACHABILITY of every site the gate
# and the eight rows are about, so "does not fire today" and "can never fire"
# stay distinguishable. Prints the W-A8-discard / W-A2-paycostall /
# W-C-scratch rows; exits non-zero if the scratch-layout invariant is violated.
WCENSUS := tools/audit_w_gate.c

.PHONY: w-census
w-census: $(TARGET)_wcensus
	./$(TARGET)_wcensus src

$(TARGET)_wcensus: $(OBJ_LIB) $(WCENSUS)
	$(CC) $(CFLAGS) $(LDFLAGS) -o $@ $(OBJ_LIB) $(WCENSUS)

# ── Cluster T7: generated-decoder + resource-family census ────────────────────
# These walk the SHIPPED bytecode (src/core/generated/gen_data.c offsets +
# src/abilities_strings.bin), not cards/abilities.json.  See
# AUDIT_decoders_resource.md for the tables they produce.
#
#   make t7-audit  decoded-tree census: which keys survived the C decoder
#   make t7-wire   raw-bytecode census: which keys are ON THE WIRE, per node
#                  class, with corpus counts and per-branch reachability
#   make t7-res    per-branch reachability for the resource/energy/deck family
#   make t7-zero   does any card carry a zero-amount base heart
T7AUDIT := tools/audit_t7_decoders.c
T7WIRE  := tools/audit_t7_wire.c
T7RES   := tools/audit_t7_resource.c
T7ZERO  := tools/audit_t7_zeroheart.c

t7-audit: $(TARGET)_t7audit
	./$(TARGET)_t7audit src

$(TARGET)_t7audit: $(OBJ_LIB) $(T7AUDIT)
	$(CC) $(CFLAGS) $(LDFLAGS) -o $@ $(OBJ_LIB) $(T7AUDIT)

t7-wire: $(TARGET)_t7wire
	./$(TARGET)_t7wire src

$(TARGET)_t7wire: $(OBJ_LIB) $(T7WIRE)
	$(CC) $(CFLAGS) $(LDFLAGS) -o $@ $(OBJ_LIB) $(T7WIRE)

t7-res: $(TARGET)_t7res
	./$(TARGET)_t7res src

$(TARGET)_t7res: $(OBJ_LIB) $(T7RES)
	$(CC) $(CFLAGS) $(LDFLAGS) -o $@ $(OBJ_LIB) $(T7RES)

t7-zero: $(TARGET)_t7zero
	./$(TARGET)_t7zero src

$(TARGET)_t7zero: $(OBJ_LIB) $(T7ZERO)
	$(CC) $(CFLAGS) $(LDFLAGS) -o $@ $(OBJ_LIB) $(T7ZERO)

clean:
	rm -f $(OBJ) $(TARGET) $(VERDICT) $(TARGET)_test $(TARGET)_audit $(TARGET)_replay $(TARGET)_generated $(TARGET)_mechanics $(TARGET)_live_rules $(TARGET)_t_*
	@# CORRECTED 2026-10-05: this block previously claimed "src/web_server.o is
	@# deliberately NOT in SRC or OBJ_LIB ... $(OBJ) never names it, so make
	@# clean silently left it behind". Both halves were FALSE and contradicted
	@# lines 61/64 of this same file: SRC ends with src/web_server.c and OBJ_LIB
	@# ends with src/web_server.o, so $(OBJ) = $(SRC:.c=.o) DOES name it, the
	@# line above already removes it, and the web target links it via $(OBJ_LIB)
	@# rather than "explicitly at line 101-102". The only thing this rule was
	@# really reaching for is $(WEB_TARGET), which $(OBJ) does not cover; that
	@# part is true and is kept below.
	rm -f src/web_server.o $(WEB_TARGET)
