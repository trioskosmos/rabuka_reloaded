#include "rabuka.h"
#include "test_game.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

static int failures;
static int checks;

#define CHECK(cond, msg) do { \
    checks++; \
    if (!(cond)) { \
        fprintf(stderr, "FAIL %s:%d: %s\n", __FILE__, __LINE__, (msg)); \
        failures++; \
    } else { printf("ok: %s\n", (msg)); } \
} while (0)

#define CHECK_EQ(actual, expected, msg) do { \
    long a_ = (long)(actual), e_ = (long)(expected); \
    checks++; \
    if (a_ != e_) { \
        fprintf(stderr, "FAIL %s:%d: %s (got %ld expected %ld)\n", __FILE__, __LINE__, (msg), a_, e_); \
        failures++; \
    } else { printf("ok: %s\n", (msg)); } \
} while (0)

#define CHECK_STR(actual, expected, msg) do { \
    const char *a_ = (actual), *e_ = (expected); \
    checks++; \
    if (!a_ || !e_ || strcmp(a_, e_) != 0) { \
        fprintf(stderr, "FAIL %s:%d: %s (got %s expected %s)\n", __FILE__, __LINE__, (msg), a_ ? a_ : "(null)", e_ ? e_ : "(null)"); \
        failures++; \
    } else { printf("ok: %s\n", (msg)); } \
} while (0)


int main(void)
{
    printf("\n%s: %d checks, %d failures\n", "ability_golden_test", checks, failures);
    return failures ? 1 : 0;
}

/* NOT PORTED (whole function skipped):
 *   abilities_shape_matches: let obj = val.as_object().expect("abilities.json is object")
 */
