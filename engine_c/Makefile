CC      ?= gcc
CFLAGS  ?= -std=c11 -O2 -Wall -Iinclude
SRC     := src/data.c src/vm.c src/cards.c src/alloc.c src/modifiers.c src/condition.c src/choice.c src/ability_queue.c src/triggers.c src/phase.c src/stats_pipeline.c src/live.c src/engine.c src/main.c src/bytecode_blob.c src/gen_data.c
OBJ     := $(SRC:.c=.o)
# library objects (everything except main, so tests can supply their own main)
OBJ_LIB := src/data.o src/vm.o src/cards.o src/alloc.o src/modifiers.o src/condition.o src/choice.o src/ability_queue.o src/triggers.o src/phase.o src/stats_pipeline.o src/live.o src/engine.o src/bytecode_blob.o src/gen_data.o
TARGET  := rb_engine
TEST    := tests/test_basic.c
REPLAY  := tests/replay.c
AUDIT   := tools/audit_actions.c

.PHONY: all test audit regen clean
all: $(TARGET)

regen:
	python3 tools/gen_from_rs.py ../cards/build/abilities_gen.rs
	python3 tools/gen_bytecode.py

$(TARGET): $(OBJ)
	$(CC) $(CFLAGS) -o $@ $(OBJ)

src/%.o: src/%.c
	$(CC) $(CFLAGS) -c -o $@ $<

test: $(TARGET)_test $(TARGET)_replay
	./$(TARGET)_test
	./$(TARGET)_replay

$(TARGET)_test: $(OBJ_LIB) $(TEST)
	$(CC) $(CFLAGS) -o $@ $(OBJ_LIB) $(TEST)

$(TARGET)_replay: $(OBJ_LIB) $(REPLAY)
	$(CC) $(CFLAGS) -o $@ $(OBJ_LIB) $(REPLAY)

audit: $(TARGET)_audit
	./$(TARGET)_audit

$(TARGET)_audit: $(OBJ_LIB) $(AUDIT)
	$(CC) $(CFLAGS) -o $@ $(OBJ_LIB) $(AUDIT)

clean:
	rm -f $(OBJ) $(TARGET) $(TARGET)_test $(TARGET)_audit
