import io, hashlib
P = "tests/test_tp_activation_offer.c"
s = io.open(P, encoding="utf-8", newline="").read()

old = '''    CHECK(activations <= 5,
          "RUST (:75-79): activations <= 5 -- the activation ran %d times, bounded "
          "by affordable energy (9/2 = 4)", activations);'''

new = '''    printf("info: offered activation count = %d (Rust asserts <= 5)\\n", activations);
    CHECK(activations <= 5,
          "RUST (:75-79): activations <= 5 -- bounded by affordable energy "
          "(9/2 = 4); the count on the preceding info line is what is bounded");'''

assert old in s, "anchor missing"
out = s.replace(old, new, 1)
with io.open(P, "r+", encoding="utf-8", newline="") as f:
    f.seek(0)
    f.write(out)
    f.truncate()
print("sha", hashlib.sha256(out.encode("utf-8")).hexdigest())