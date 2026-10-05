import io, sys

p = "tests/test_prototype_closure.c"
s = io.open(p, "r", encoding="utf-8", newline="").read()

MARK = 'fprintf(stderr, "MARK %s\\n", "%s");'
marks = [
    ("test_header_declares_the_three_rb_gsc_prototypes();", "A"),
    ("test_recently_moved_cards_signature();", "B"),
    ("test_mulligan_owner_index_signature();", "C"),
    ("test_rules_clear_turn_state_declared_once_in_header();", "D"),
    ("test_rules_clear_turn_state_empties_the_turn_sets();", "E"),
]
for call, tag in marks:
    assert s.count(call) == 1, "no unique call: " + call
    s = s.replace(call, MARK % (tag, call[:-2]) + "\n    " + call, 1)

io.open(p, "w", encoding="utf-8", newline="").write(s)
print("instrumented 5 marks")