#include "rabuka.h"
#include <stdlib.h>
#include <string.h>

#ifdef RB_NO_MALLOC
/* Bare-metal bump arena: 512 KB static pool, no free(). Swap in per-platform. */
#define ARENA_SIZE (512u * 1024u)
static unsigned char arena[ARENA_SIZE];
static size_t arena_off = 0;

void *rb_malloc(size_t n) {
    n = (n + 7u) & ~7u; /* 8-byte align */
    if (arena_off + n > ARENA_SIZE) return NULL;
    void *p = arena + arena_off;
    arena_off += n;
    return p;
}
void rb_free(void *p) { (void)p; /* no-op */ }
void rb_alloc_reset(void) { arena_off = 0; }
#else
void *rb_malloc(size_t n) { return malloc(n); }
void rb_free(void *p) { free(p); }
void rb_alloc_reset(void) {}
#endif

char *rb_strdup2(const char *s) {
    if (!s) return NULL;
    size_t n = strlen(s) + 1;
    char *p = (char *)rb_malloc(n);
    if (p) memcpy(p, s, n);
    return p;
}
