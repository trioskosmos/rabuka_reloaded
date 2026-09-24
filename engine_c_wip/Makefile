CC      ?= gcc
CFLAGS  ?= -std=c11 -O2 -Wall -Iinclude -Isrc -Isrc/core/generated
# Layout mirrors engine/src: ability/ (vm, condition, choice, queue, effects/),
# core/ (card, data, alloc, modifiers, stats_pipeline, generated/), turn/ (phase, live, triggers).
SRC     := src/ability/enums.c src/ability/types.c src/ability/vm.c src/ability/condition.c src/ability/choice.c src/ability/ability_queue.c src/ability/dynamic_count.c src/ability/util.c src/ability/cost.c src/ability/compound.c src/ability/resolver.c src/ability/effects/move.c src/ability/effects/move_missing.c src/ability/effects/look.c src/ability/effects/state.c src/ability/effects/ability.c src/ability/effects/misc.c src/ability/effects/draw.c src/ability/effects/score.c src/ability/log.c src/ability/debug.c src/ability/abilities_gen.c src/game/deck_parser.c src/core/card.c src/core/data.c src/core/alloc.c src/core/modifiers.c src/core/stats_pipeline.c src/core/game_state_abilities.c src/core/tracking.c src/core/zones.c src/bytecode_data.c src/core/generated/bytecode_blob.c src/core/generated/gen_data.c src/turn/phase.c src/turn/live.c src/turn/triggers.c src/engine.c src/main.c
OBJ     := $(SRC:.c=.o)
# library objects (everything except main, so tests can supply their own main)
OBJ_LIB := src/ability/enums.o src/ability/types.o src/ability/vm.o src/ability/condition.o src/ability/choice.o src/ability/ability_queue.o src/ability/dynamic_count.o src/ability/util.o src/ability/cost.o src/ability/compound.o src/ability/resolver.o src/ability/effects/move.o src/ability/effects/move_missing.o src/ability/effects/look.o src/ability/effects/state.o src/ability/effects/ability.o src/ability/effects/misc.o src/ability/effects/draw.o src/ability/effects/score.o src/ability/log.o src/ability/debug.o src/ability/abilities_gen.o src/game/deck_parser.o src/core/card.o src/core/data.o src/core/alloc.o src/core/modifiers.o src/core/stats_pipeline.o src/core/game_state_abilities.o src/core/tracking.o src/core/zones.o src/bytecode_data.o src/core/generated/bytecode_blob.o src/core/generated/gen_data.o src/turn/phase.o src/turn/live.o src/turn/triggers.o src/engine.o src/test_game.o
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
AUDIT   := tools/audit_actions.c

.PHONY: all web test audit regen clean mechanics live-rules phase-rules performance-rules surplus-heart opponent-live-flow live-movement restrictions condition-eval target-selection max-distinct-names baton-touch member-activation b8-live-timing cross-player draw-phase-fix zone-filter-parity
all: $(TARGET)

web: $(WEB_TARGET)

$(WEB_TARGET): $(OBJ_LIB) src/web_server.o
	$(CC) $(CFLAGS) -mwindows -Wl,--stack,16777216 -o $@ $(OBJ_LIB) src/web_server.o

regen:
	python3 tools/gen_from_rs.py ../cards/build/abilities_gen.rs
	python3 tools/gen_bytecode.py

$(TARGET): $(OBJ)
	$(CC) $(CFLAGS) -o $@ $(OBJ)

src/%.o: src/%.c $(wildcard include/*.h)
	$(CC) $(CFLAGS) -c -o $@ $<

test: $(TARGET)_generated $(TARGET)_replay
	./$(TARGET)_generated
	./$(TARGET)_replay

$(TARGET)_generated: $(OBJ_LIB) $(GENERATED)
	$(CC) $(CFLAGS) -o $@ $(OBJ_LIB) $(GENERATED)

$(TARGET)_replay: $(OBJ_LIB) tests/replay.c
	$(CC) $(CFLAGS) -o $@ $(OBJ_LIB) tests/replay.c

$(TARGET)_mechanics: $(OBJ_LIB) $(MECHANICS)
	$(CC) $(CFLAGS) -o $@ $(OBJ_LIB) $(MECHANICS)

$(TARGET)_live_rules: $(OBJ_LIB) $(LIVE_RULES)
	$(CC) $(CFLAGS) -o $@ $(OBJ_LIB) $(LIVE_RULES)

mechanics: $(TARGET)_mechanics
	./$(TARGET)_mechanics

live-rules: $(TARGET)_live_rules
	./$(TARGET)_live_rules

$(TARGET)_phase_rules: $(OBJ_LIB) $(PHASE_RULES)
	$(CC) $(CFLAGS) -o $@ $(OBJ_LIB) $(PHASE_RULES)

$(TARGET)_performance_rules: $(OBJ_LIB) $(PERFORMANCE_RULES)
	$(CC) $(CFLAGS) -o $@ $(OBJ_LIB) $(PERFORMANCE_RULES)

$(TARGET)_surplus_heart: $(OBJ_LIB) $(SURPLUS_HEART)
	$(CC) $(CFLAGS) -o $@ $(OBJ_LIB) $(SURPLUS_HEART)

phase-rules: $(TARGET)_phase_rules
	./$(TARGET)_phase_rules

performance-rules: $(TARGET)_performance_rules
	./$(TARGET)_performance_rules

surplus-heart: $(TARGET)_surplus_heart
	./$(TARGET)_surplus_heart

$(TARGET)_opponent_live_flow: $(OBJ_LIB) $(OPPONENT_LIVE_FLOW)
	$(CC) $(CFLAGS) -o $@ $(OBJ_LIB) $(OPPONENT_LIVE_FLOW)

$(TARGET)_live_movement: $(OBJ_LIB) $(LIVE_MOVEMENT)
	$(CC) $(CFLAGS) -o $@ $(OBJ_LIB) $(LIVE_MOVEMENT)

$(TARGET)_restrictions: $(OBJ_LIB) $(RESTRICTIONS)
	$(CC) $(CFLAGS) -o $@ $(OBJ_LIB) $(RESTRICTIONS)

opponent-live-flow: $(TARGET)_opponent_live_flow
	./$(TARGET)_opponent_live_flow

live-movement: $(TARGET)_live_movement
	./$(TARGET)_live_movement

restrictions: $(TARGET)_restrictions
	./$(TARGET)_restrictions

$(TARGET)_condition_eval: $(OBJ_LIB) $(CONDITION_EVAL)
	$(CC) $(CFLAGS) -o $@ $(OBJ_LIB) $(CONDITION_EVAL)

$(TARGET)_target_selection: $(OBJ_LIB) $(TARGET_SELECTION)
	$(CC) $(CFLAGS) -o $@ $(OBJ_LIB) $(TARGET_SELECTION)

$(TARGET)_max_distinct_names: $(OBJ_LIB) $(MAX_DISTINCT_NAMES)
	$(CC) $(CFLAGS) -o $@ $(OBJ_LIB) $(MAX_DISTINCT_NAMES)

condition-eval: $(TARGET)_condition_eval
	./$(TARGET)_condition_eval

target-selection: $(TARGET)_target_selection
	./$(TARGET)_target_selection

max-distinct-names: $(TARGET)_max_distinct_names
	./$(TARGET)_max_distinct_names

$(TARGET)_baton_touch: $(OBJ_LIB) $(BATON_TOUCH)
	$(CC) $(CFLAGS) -o $@ $(OBJ_LIB) $(BATON_TOUCH)

$(TARGET)_member_activation: $(OBJ_LIB) $(MEMBER_ACTIVATION)
	$(CC) $(CFLAGS) -o $@ $(OBJ_LIB) $(MEMBER_ACTIVATION)

$(TARGET)_b8_live_timing: $(OBJ_LIB) $(B8_LIVE_TIMING)
	$(CC) $(CFLAGS) -o $@ $(OBJ_LIB) $(B8_LIVE_TIMING)

baton-touch: $(TARGET)_baton_touch
	./$(TARGET)_baton_touch

member-activation: $(TARGET)_member_activation
	./$(TARGET)_member_activation

b8-live-timing: $(TARGET)_b8_live_timing
	./$(TARGET)_b8_live_timing

$(TARGET)_cross_player: $(OBJ_LIB) $(CROSS_PLAYER)
	$(CC) $(CFLAGS) -o $@ $(OBJ_LIB) $(CROSS_PLAYER)

$(TARGET)_draw_phase_fix: $(OBJ_LIB) $(DRAW_PHASE_FIX)
	$(CC) $(CFLAGS) -o $@ $(OBJ_LIB) $(DRAW_PHASE_FIX)

cross-player: $(TARGET)_cross_player
	./$(TARGET)_cross_player

draw-phase-fix: $(TARGET)_draw_phase_fix
	./$(TARGET)_draw_phase_fix

$(TARGET)_zone_filter_parity: $(OBJ_LIB) $(ZONE_FILTER_PARITY)
	$(CC) $(CFLAGS) -o $@ $(OBJ_LIB) $(ZONE_FILTER_PARITY)

zone-filter-parity: $(TARGET)_zone_filter_parity
	./$(TARGET)_zone_filter_parity

 $(TARGET)_debug: $(OBJ_LIB) src/debug_umi.o
	$(CC) $(CFLAGS) -o $@ $(OBJ_LIB) src/debug_umi.o


audit: $(TARGET)_audit
	./$(TARGET)_audit

$(TARGET)_audit: $(OBJ_LIB) $(AUDIT)
	$(CC) $(CFLAGS) -o $@ $(OBJ_LIB) $(AUDIT)

clean:
	rm -f $(OBJ) $(TARGET) $(TARGET)_test $(TARGET)_audit $(TARGET)_replay $(TARGET)_generated $(TARGET)_mechanics $(TARGET)_live_rules
