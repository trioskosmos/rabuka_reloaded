/* Minimal bare-metal C runtime for the Atari Jaguar (m68k 68000).
 * Provides the libc symbols engine_c_wip needs: a REAL free-list
 * allocator (not a bump arena — the engine decodes and frees
 * AbilityEffect/Condition trees every turn via libc malloc/free,
 * so a no-op free would exhaust RAM mid-match), string/stdlib
 * helpers, and a printf family that writes to the GPU text console.
 *
 * No host OS, no filesystem. Everything is freestanding.
 */
#include <stddef.h>
#include <stdarg.h>
#include <stdint.h>
#include <ctype.h>
#include <stdio.h>
#include <string.h>
#include <time.h>

/* ---- console output hook (implemented in console.c) ---- */
void __console_out(int c);

/* ===================================================================== */
/*                     ALLOCATOR (free list, coalescing)                */
/* ===================================================================== */
/* Heap budget: 2 MB DRAM total. GameState is ~1.04 MB of .bss, the
 * engine's audit scratch is ~35 KB, so the libc heap gets
 * 128 KB — enough for the decode/free churn of a full match
 * (g_card_off ~10 KB one-time, effect trees a few KB live at
 * once) with the rest left to the stack. */
#define HEAP_SIZE (128u * 1024u)

typedef struct blk {
    size_t size;            /* payload size (excluding header) */
    struct blk *next;       /* next block in address order */
    int free;               /* 1 = free, 0 = used */
} blk_t;

#define BLK_HDR (sizeof(blk_t))
#define MIN_SPLIT 32        /* don't split a remainder smaller than this */

static unsigned char g_heap[HEAP_SIZE];
static blk_t *g_head = NULL;

static void heap_init(void) {
    g_head = (blk_t *)g_heap;
    g_head->size = HEAP_SIZE - BLK_HDR;
    g_head->next = NULL;
    g_head->free = 1;
}

void *malloc(size_t n) {
    if (n == 0) return NULL;
    if (g_head == NULL) heap_init();
    /* 8-byte align the payload */
    size_t need = (n + 7u) & ~7u;
    for (blk_t *b = g_head; b; b = b->next) {
        if (b->free && b->size >= need) {
            if (b->size >= need + MIN_SPLIT + BLK_HDR) {
                /* split: remainder becomes a free block */
                blk_t *rest = (blk_t *)((unsigned char *)(b + 1) + need);
                rest->size = b->size - need - BLK_HDR;
                rest->next = b->next;
                rest->free = 1;
                b->size = need;
                b->next = rest;
            }
            b->free = 0;
            return (void *)(b + 1);
        }
    }
    return NULL;              /* out of memory */
}

void free(void *p) {
    if (!p) return;
    /* rb_load_streaming() frees the ROM-resident buffers its
       read callback returns (data.c). Those addresses live in
       cart ROM, far outside g_heap -- freeing one would read a
       garbage block header and corrupt the list. Ignore them. */
    if ((const unsigned char *)p < g_heap ||
        (const unsigned char *)p >= g_heap + HEAP_SIZE) return;
    blk_t *b = (blk_t *)p - 1;
    b->free = 1;
    /* coalesce with the next neighbour(s) */
    while (b->next && b->next->free) {
        b->size += BLK_HDR + b->next->size;
        b->next = b->next->next;
    }
}

void *calloc(size_t nmemb, size_t size) {
    size_t t = nmemb * size;
    void *p = malloc(t);
    if (p) memset(p, 0, t);
    return p;
}

void *realloc(void *old, size_t n) {
    if (!old) return malloc(n);
    if (n == 0) { free(old); return NULL; }
    blk_t *b = (blk_t *)old - 1;
    size_t need = (n + 7u) & ~7u;
    if (b->size >= need) return old;      /* fits in place */
    void *q = malloc(n);
    if (q) memcpy(q, old, b->size < n ? b->size : n);
    free(old);
    return q;
}

/* ===================================================================== */
/*                            MEMORY / STRING                            */
/* ===================================================================== */
void *memcpy(void *dst, const void *src, size_t n) {
    unsigned char *d = (unsigned char *)dst;
    const unsigned char *s = (const unsigned char *)src;
    while (n--) *d++ = *s++;
    return dst;
}
void *memmove(void *dst, const void *src, size_t n) {
    unsigned char *d = (unsigned char *)dst;
    const unsigned char *s = (const unsigned char *)src;
    if (d < s) { while (n--) *d++ = *s++; }
    else { d += n; s += n; while (n--) *--d = *--s; }
    return dst;
}
void *memset(void *dst, int c, size_t n) {
    unsigned char *d = (unsigned char *)dst;
    unsigned char v = (unsigned char)c;
    while (n--) *d++ = v;
    return dst;
}
int memcmp(const void *a, const void *b, size_t n) {
    const unsigned char *x = (const unsigned char *)a, *y = (const unsigned char *)b;
    while (n--) { int d = (int)*x++ - (int)*y++; if (d) return d; }
    return 0;
}
void *memchr(const void *s, int c, size_t n) {
    const unsigned char *p = (const unsigned char *)s;
    while (n--) { if (*p == (unsigned char)c) return (void *)(uintptr_t)p; p++; }
    return NULL;
}

size_t strlen(const char *s) {
    const char *p = s;
    while (*p) p++;
    return (size_t)(p - s);
}
int strcmp(const char *a, const char *b) {
    while (*a && *a == *b) { a++; b++; }
    return (int)(unsigned char)*a - (int)(unsigned char)*b;
}
int strncmp(const char *a, const char *b, size_t n) {
    while (n--) { if (!*a) return (int)(unsigned char)*a - (int)(unsigned char)*b;
                 if (*a != *b) return (int)(unsigned char)*a - (int)(unsigned char)*b; a++; b++; }
    return 0;
}
char *strcpy(char *dst, const char *src) {
    char *d = dst;
    while ((*d++ = *src++));
    return dst;
}
char *strncpy(char *dst, const char *src, size_t n) {
    char *d = dst;
    while (n--) { *d++ = *src; if (*src) src++; }
    return dst;
}
char *strcat(char *dst, const char *src) {
    char *d = dst + strlen(dst);
    while ((*d++ = *src++));
    return dst;
}
char *strncat(char *dst, const char *src, size_t n) {
    char *d = dst + strlen(dst);
    while (n-- && *src) *d++ = *src++;
    *d = 0;
    return dst;
}
char *strchr(const char *s, int c) {
    while (*s) { if (*(unsigned char *)s == (unsigned char)c) return (char *)(uintptr_t)s; s++; }
    if (c == 0) return (char *)(uintptr_t)s;
    return NULL;
}
char *strrchr(const char *s, int c) {
    const char *last = NULL;
    while (*s) { if (*(unsigned char *)s == (unsigned char)c) last = s; s++; }
    if (c == 0) return (char *)(uintptr_t)s;
    return (char *)(uintptr_t)last;
}
char *strstr(const char *hay, const char *needle) {
    if (!*needle) return (char *)(uintptr_t)hay;
    size_t nl = strlen(needle);
    while (*hay) {
        if (strncmp(hay, needle, nl) == 0) return (char *)(uintptr_t)hay;
        hay++;
    }
    return NULL;
}
size_t strspn(const char *s, const char *set) {
    size_t n = 0;
    while (*s && strchr(set, *(unsigned char *)s)) { n++; s++; }
    return n;
}
size_t strcspn(const char *s, const char *set) {
    size_t n = 0;
    while (*s && !strchr(set, *(unsigned char *)s)) { n++; s++; }
    return n;
}
char *strpbrk(const char *s, const char *set) {
    while (*s) { if (strchr(set, *s)) return (char *)(uintptr_t)s; s++; }
    return NULL;
}
static char *g_strtok_state = NULL;
char *strtok(char *s, const char *delim) {
    if (s) g_strtok_state = s;
    if (!g_strtok_state) return NULL;
    g_strtok_state += strspn(g_strtok_state, delim);
    if (!*g_strtok_state) { g_strtok_state = NULL; return NULL; }
    char *tok = g_strtok_state;
    char *e = strpbrk(tok, delim);
    if (e) { *e = 0; g_strtok_state = e + 1; }
    else g_strtok_state = NULL;
    return tok;
}
char *strdup(const char *s) {
    size_t n = strlen(s) + 1;
    char *p = (char *)malloc(n);
    if (p) memcpy(p, s, n);
    return p;
}

int strcasecmp(const char *a, const char *b) {
    while (*a && tolower((unsigned char)*a) == tolower((unsigned char)*b)) { a++; b++; }
    return tolower((unsigned char)*a) - tolower((unsigned char)*b);
}

int strncasecmp(const char *a, const char *b, size_t n) {
    while (n--) {
        int ca = tolower((unsigned char)*a), cb = tolower((unsigned char)*b);
        if (ca != cb) return ca - cb;
        if (!*a) return 0;
        a++; b++;
    }
    return 0;
}

/* ===================================================================== */
/*                                SORT                                   */
/* ===================================================================== */
/* qsort: in-place quicksort (median-of-three pivot, Lomuto
   partition, insertion sort under 16 entries, smaller side
   recursed first so the stack stays O(log n)). The engine
   sorts at most a few thousand entries (card pair index,
   mask lists, decks). */
static void qsort_swap(char *a, char *b, size_t w) {
    while (w--) { char t = *a; *a = *b; *b = t; a++; b++; }
}

static void qsort_insert(char *base, size_t n, size_t w,
                         int (*cmp)(const void *, const void *)) {
    for (size_t i = 1; i < n; i++) {
        size_t j = i;
        while (j > 0 && cmp(base + (j - 1) * w, base + j * w) > 0) {
            qsort_swap(base + (j - 1) * w, base + j * w, w);
            j--;
        }
    }
}

static size_t qsort_partition(char *base, size_t lo, size_t hi, size_t w,
                              int (*cmp)(const void *, const void *)) {
    size_t mid = lo + (hi - lo) / 2;
    if (cmp(base + lo * w, base + mid * w) > 0)
        qsort_swap(base + lo * w, base + mid * w, w);
    if (cmp(base + lo * w, base + (hi - 1) * w) > 0)
        qsort_swap(base + lo * w, base + (hi - 1) * w, w);
    if (cmp(base + mid * w, base + (hi - 1) * w) > 0)
        qsort_swap(base + mid * w, base + (hi - 1) * w, w);
    qsort_swap(base + mid * w, base + (hi - 1) * w, w);  /* pivot to hi-1 */
    char *pivot = base + (hi - 1) * w;
    size_t i = lo;
    for (size_t j = lo; j < hi - 1; j++) {
        if (cmp(base + j * w, pivot) <= 0) {
            qsort_swap(base + i * w, base + j * w, w);
            i++;
        }
    }
    qsort_swap(base + i * w, pivot, w);
    return i;
}

static void qsort_range(char *base, size_t lo, size_t hi, size_t w,
                        int (*cmp)(const void *, const void *)) {
    while (hi - lo > 16) {
        size_t p = qsort_partition(base, lo, hi, w, cmp);
        if (p - lo < hi - p - 1) {
            qsort_range(base, lo, p, w, cmp);
            lo = p + 1;
        } else {
            qsort_range(base, p + 1, hi, w, cmp);
            hi = p;
        }
    }
    qsort_insert(base + lo * w, hi - lo, w, cmp);
}

void qsort(void *base, size_t nmemb, size_t size,
           int (*compar)(const void *, const void *)) {
    if (!base || nmemb < 2 || !size || !compar) return;
    qsort_range((char *)base, 0, nmemb, size, compar);
}

/* ===================================================================== */
/*                              STDLIB MATH                              */
/* ===================================================================== */
int abs(int v) { return v < 0 ? -v : v; }
long labs(long v) { return v < 0 ? -v : v; }

static unsigned long g_rand = 1;
int rand(void) {
    g_rand = g_rand * 1103515245u + 12345u;
    return (int)((g_rand >> 16) & 0x7FFF);
}
void srand(unsigned int seed) { g_rand = seed ? seed : 1; }

long strtol(const char *s, char **end, int base) {
    while (isspace((unsigned char)*s)) s++;
    int neg = 0;
    if (*s == '+') s++; else if (*s == '-') { neg = 1; s++; }
    if (base == 0) {
        if (*s == '0' && (s[1] == 'x' || s[1] == 'X')) { base = 16; s += 2; }
        else base = 10;
    }
    long v = 0;
    while (1) {
        int d;
        char c = *s;
        if (c >= '0' && c <= '9') d = c - '0';
        else if (c >= 'a' && c <= 'f') d = c - 'a' + 10;
        else if (c >= 'A' && c <= 'F') d = c - 'A' + 10;
        else break;
        if (d >= base) break;
        v = v * base + d;
        s++;
    }
    if (end) *end = (char *)s;
    return neg ? -v : v;
}
long strtoul(const char *s, char **end, int base) { return (long)strtol(s, end, base); }
long long strtoll(const char *s, char **end, int base) { return (long long)strtol(s, end, base); }
int atoi(const char *s) { return (int)strtol(s, NULL, 10); }
long atol(const char *s) { return strtol(s, NULL, 10); }

void exit(int code) { (void)code; for (;;); }
void abort(void) { for (;;); }

/* ===================================================================== */
/*                          PRINTF FAMILY                                */
/* ===================================================================== */
#define OUTBUF 600

static void out_num(unsigned long long v, int base, int width, int pad, int upper) {
    char tmp[24];
    int i = 0;
    if (v == 0) tmp[i++] = '0';
    while (v) {
        int d = (int)(v % base);
        tmp[i++] = (char)(d < 10 ? ('0' + d) : (upper ? ('A' + d - 10) : ('a' + d - 10)));
        v /= base;
    }
    while (i < width) tmp[i++] = (char)(pad ? pad : ' ');
    while (i--) __console_out(tmp[i]);
}

int vsnprintf(char *buf, size_t n, const char *fmt, va_list ap) {
    size_t written = 0;
    const char *p = fmt;
    #define EMIT(ch) do { if (written + 1 < n && n > 0) buf[written] = (ch); if (written < n) written++; } while (0)
    while (*p) {
        if (*p != '%') { EMIT(*p++); continue; }
        p++;
        int pad = 0, width = 0, left = 0;
        while (*p == '-') { left = 1; p++; }
        while (*p == '0') { pad = '0'; p++; }
        while (*p >= '0' && *p <= '9') { width = width * 10 + (*p - '0'); p++; }
        int longness = 0;
        if (*p == 'l') { longness = 1; p++; if (*p == 'l') { longness = 2; p++; } }
        int spec = *p++;
        char cb[OUTBUF];
        size_t ci = 0;
        switch (spec) {
            case 'c': { int ch = va_arg(ap, int); cb[ci++] = (char)ch; break; }
            case 's': {
                const char *s = va_arg(ap, const char *);
                if (!s) s = "(null)";
                while (*s) cb[ci++] = *s++;
                break;
            }
            case 'd': case 'i': {
                long long v = (long long)(longness == 2 ? va_arg(ap, long long)
                                    : longness == 1 ? va_arg(ap, long)
                                    : va_arg(ap, int));
                if (v < 0) { cb[ci++] = '-'; v = -v; }
                char t[24]; int ti = 0;
                if (v == 0) t[ti++] = '0';
                while (v) { t[ti++] = (char)('0' + (v % 10)); v /= 10; }
                while (ti--) cb[ci++] = t[ti];
                break;
            }
            case 'u': {
                unsigned long long v = (unsigned long long)(longness == 2 ? va_arg(ap, unsigned long long)
                                          : longness == 1 ? va_arg(ap, unsigned long)
                                          : va_arg(ap, unsigned int));
                char t[24]; int ti = 0;
                if (v == 0) t[ti++] = '0';
                while (v) { t[ti++] = (char)('0' + (v % 10)); v /= 10; }
                while (ti--) cb[ci++] = t[ti];
                break;
            }
            case 'x': case 'X': {
                unsigned long long v = (unsigned long long)(longness == 2 ? va_arg(ap, unsigned long long)
                                          : longness == 1 ? va_arg(ap, unsigned long)
                                          : va_arg(ap, unsigned int));
                char t[24]; int ti = 0;
                if (v == 0) t[ti++] = '0';
                while (v) { int d = (int)(v % 16); t[ti++] = (char)(d < 10 ? ('0'+d) : (spec=='X'?('A'+d-10):('a'+d-10))); v /= 16; }
                while (ti--) cb[ci++] = t[ti];
                break;
            }
            case 'p': {
                unsigned long long v = (unsigned long long)(uintptr_t)va_arg(ap, void *);
                char t[24]; int ti = 0;
                if (v == 0) t[ti++] = '0';
                while (v) { int d = (int)(v % 16); t[ti++] = (char)(d < 10 ? ('0'+d) : ('a'+d-10)); v /= 16; }
                while (ti--) cb[ci++] = t[ti];
                break;
            }
            case '%': cb[ci++] = '%'; break;
            default: cb[ci++] = '?'; break;
        }
        if (left) { for (size_t k = 0; k < ci; k++) EMIT(cb[k]); }
        else { for (int k = (int)ci; k < width; k++) EMIT(pad ? pad : ' '); for (size_t k = 0; k < ci; k++) EMIT(cb[k]); }
    }
    #undef EMIT
    if (n > 0) buf[(written < n) ? written : n - 1] = 0;
    return (int)written;
}

int snprintf(char *buf, size_t n, const char *fmt, ...) {
    va_list ap; va_start(ap, fmt);
    int r = vsnprintf(buf, n, fmt, ap);
    va_end(ap);
    return r;
}
int sprintf(char *buf, const char *fmt, ...) {
    va_list ap; va_start(ap, fmt);
    int r = vsnprintf(buf, (size_t)-1, fmt, ap);
    va_end(ap);
    return r;
}
int printf(const char *fmt, ...) {
    char b[OUTBUF];
    va_list ap; va_start(ap, fmt);
    int r = vsnprintf(b, sizeof b, fmt, ap);
    va_end(ap);
    for (char *p = b; *p; p++) __console_out(*p);
    return r;
}
int fprintf(FILE *stream, const char *fmt, ...) {
    (void)stream;
    char b[OUTBUF];
    va_list ap; va_start(ap, fmt);
    int r = vsnprintf(b, sizeof b, fmt, ap);
    va_end(ap);
    for (char *p = b; *p; p++) __console_out(*p);
    return r;
}
int puts(const char *s) {
    for (; *s; s++) __console_out(*s);
    __console_out('\n');
    return 0;
}

int fputs(const char *s, FILE *stream) {
    (void)stream;
    for (; *s; s++) __console_out(*s);
    return 0;
}
int putchar(int c) {
    __console_out(c);
    return (unsigned char)c;
}
int fputc(int c, FILE *stream) {
    (void)stream;
    __console_out(c);
    return (unsigned char)c;
}
int fflush(FILE *stream) {
    (void)stream;
    return 0;
}

time_t time(time_t *t) { if (t) *t = 0; return 0; }
clock_t clock(void) { return 0; }
int clock_gettime(int clk_id, struct timespec *tp) {
    (void)clk_id;
    if (tp) { tp->tv_sec = 0; tp->tv_nsec = 0; }
    return 0;
}

/* stream stubs (unused; read_file() is host-only and not linked) */
FILE *stdin = (FILE *)0, *stdout = (FILE *)0, *stderr = (FILE *)0;
FILE *fopen(const char *path, const char *mode) { (void)path; (void)mode; return NULL; }
int   fclose(FILE *f) { (void)f; return -1; }
size_t fread(void *ptr, size_t size, size_t nmemb, FILE *f) { (void)ptr; (void)size; (void)nmemb; (void)f; return 0; }
int   fseek(FILE *f, long off, int whence) { (void)f; (void)off; (void)whence; return -1; }
long  ftell(FILE *f) { (void)f; return -1; }
