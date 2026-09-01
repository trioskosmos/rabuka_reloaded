CC      ?= gcc
CFLAGS  ?= -std=c11 -O2 -Wall -Iinclude -Isrc -Isrc/core/generated
# Layout mirrors engine/src: ability/ (vm, condition, choice, queue, effects/),
# core/ (card, data, alloc, modifiers, stats_pipeline, generated/), turn/ (phase, live, triggers).
SRC     := src/ability/enums.c src/ability/vm.c src/ability/condition.c src/ability/choice.c src/ability/ability_queue.c src/ability/dynamic_count.c src/ability/util.c src/ability/cost.c src/ability/compound.c src/ability/resolver.c src/ability/effects/move.c src/ability/effects/move_missing.c src/ability/effects/look.c src/ability/effects/state.c src/ability/effects/ability.c src/ability/effects/misc.c src/ability/effects/draw.c src/ability/effects/score.c src/ability/log.c src/ability/debug.c src/ability/abilities_gen.c src/core/card.c src/core/data.c src/core/alloc.c src/core/modifiers.c src/core/stats_pipeline.c src/core/game_state_abilities.c src/core/tracking.c src/core/zones.c src/bytecode_data.c src/core/generated/bytecode_blob.c src/core/generated/gen_data.c src/turn/phase.c src/turn/live.c src/turn/triggers.c src/engine.c src/main.c
OBJ     := $(SRC:.c=.o)
# library objects (everything except main, so tests can supply their own main)
OBJ_LIB := src/ability/enums.o src/ability/vm.o src/ability/condition.o src/ability/choice.o src/ability/ability_queue.o src/ability/dynamic_count.o src/ability/util.o src/ability/cost.o src/ability/compound.o src/ability/resolver.o src/ability/effects/move.o src/ability/effects/move_missing.o src/ability/effects/look.o src/ability/effects/state.o src/ability/effects/ability.o src/ability/effects/misc.o src/ability/effects/draw.o src/ability/effects/score.o src/ability/log.o src/ability/debug.o src/ability/abilities_gen.o src/core/card.o src/core/data.o src/core/alloc.o src/core/modifiers.o src/core/stats_pipeline.o src/core/game_state_abilities.o src/core/tracking.o src/core/zones.o src/bytecode_data.o src/core/generated/bytecode_blob.o src/core/generated/gen_data.o src/turn/phase.o src/turn/live.o src/turn/triggers.o src/engine.o src/test_game.o
TARGET  := rb_engine
GENERATED := tests/test_ported_generated.c
AUDIT   := tools/audit_actions.c

.PHONY: all test audit regen clean
all: $(TARGET)

regen:
	python3 tools/gen_from_rs.py ../cards/build/abilities_gen.rs
	python3 tools/gen_bytecode.py

$(TARGET): $(OBJ)
	$(CC) $(CFLAGS) -o $@ $(OBJ)

src/%.o: src/%.c $(wildcard include/*.h)
	$(CC) $(CFLAGS) -c -o $@ $<

test: $(TARGET)_generated
	./$(TARGET)_generated

$(TARGET)_generated: $(OBJ_LIB) $(GENERATED)
	$(CC) $(CFLAGS) -o $@ $(OBJ_LIB) $(GENERATED)

 $(TARGET)_debug: $(OBJ_LIB) src/debug_umi.o
	$(CC) $(CFLAGS) -o $@ $(OBJ_LIB) src/debug_umi.o


audit: $(TARGET)_audit
	./$(TARGET)_audit

$(TARGET)_audit: $(OBJ_LIB) $(AUDIT)
	$(CC) $(CFLAGS) -o $@ $(OBJ_LIB) $(AUDIT)

clean:
	rm -f $(OBJ) $(TARGET) $(TARGET)_test $(TARGET)_audit $(TARGET)_replay $(TARGET)_generated
