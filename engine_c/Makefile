CC      ?= gcc
CFLAGS  ?= -std=c11 -O2 -Wall -Iinclude
SRC     := src/data.c src/vm.c src/cards.c src/engine.c src/main.c src/bytecode_blob.c src/gen_data.c
OBJ     := $(SRC:.c=.o)
# library objects (everything except main, so tests can supply their own main)
OBJ_LIB := src/data.o src/vm.o src/cards.c src/engine.o src/bytecode_blob.o src/gen_data.o
TARGET  := rb_engine
TEST    := tests/test_basic.c
AUDIT   := tools/audit_actions.c

.PHONY: all test audit clean
all: $(TARGET)

$(TARGET): $(OBJ)
	$(CC) $(CFLAGS) -o $@ $(OBJ)

src/%.o: src/%.c
	$(CC) $(CFLAGS) -c -o $@ $<

test: $(TARGET)_test
	./$(TARGET)_test

$(TARGET)_test: $(OBJ_LIB) $(TEST)
	$(CC) $(CFLAGS) -o $@ $(OBJ_LIB) $(TEST)

audit: $(TARGET)_audit
	./$(TARGET)_audit

$(TARGET)_audit: $(OBJ_LIB) $(AUDIT)
	$(CC) $(CFLAGS) -o $@ $(OBJ_LIB) $(AUDIT)

clean:
	rm -f $(OBJ) $(TARGET) $(TARGET)_test $(TARGET)_audit
