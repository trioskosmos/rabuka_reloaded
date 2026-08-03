#include <nds.h>
#include <nds/arm9/console.h>
#include <nds/arm9/video.h>
#include <nds/arm9/background.h>
#include <stdio.h>
#include <stdarg.h>
#include <sys/time.h>

#ifndef REG_TM0DATA
#define REG_TM0DATA  (*(vu16*)0x04000100)
#define REG_TM0CNT   (*(vu16*)0x04000102)
#define TIMER_ENABLE       (1<<7)
#define TIMER_PRESCALER_64 (1<<0)
#endif
#ifndef REG_BG_COLOR
#define REG_BG_COLOR (*(vu16*)0x05000000) // main engine backdrop = BG_PALETTE[0]
#endif

void nds_init(void) {
    videoSetMode(MODE_0_2D);
    videoSetModeSub(MODE_0_2D);
    vramSetBankC(VRAM_C_SUB_BG);
    irqSet(IRQ_VBLANK, 0);
    irqEnable(IRQ_VBLANK);
    consoleDemoInit();
    printf("\x1b[2J");
    // Free-running timer 0 for RNG seeding / timing.
    REG_TM0DATA = 0;
    REG_TM0CNT = TIMER_ENABLE | TIMER_PRESCALER_64;
}

unsigned long long nds_get_tick(void) {
    return (unsigned long long)REG_TM0DATA;
}

// ── NO$GBA debugger TTY (shown by melonDS / NO$GBA) ──────────────
// Write a pointer to a null-terminated string; the emulator prints it to its
// debug console. Registers live in the emulator-debug region (0x04FFFAxx).
static volatile void* NO_CASH_STRING_OUT = (void*)0x04FFFA14;

void nds_nocash_log(const char* msg) {
    *((volatile const char**)NO_CASH_STRING_OUT) = msg;
}

// Solid-color the main (bottom) engine backdrop for stage LEDs we can read
// from a screenshot without needing to read text.
void nds_set_backdrop_color(unsigned short color) {
    REG_BG_COLOR = color;
}

void nds_printf(const char* fmt, ...) {
    va_list args;
    va_start(args, fmt);
    vprintf(fmt, args);
    va_end(args);
}

void nds_print(const char* text) {
    fputs(text, stdout);
}

void nds_print_len(const char* text, int len) {
    fwrite(text, 1, len, stdout);
}

void nds_set_cursor(int x, int y) {
    printf("\x1b[%d;%dH", y + 1, x + 1);
}

void nds_clear_line(int row) {
    printf("\x1b[%d;1H\x1b[2K", row + 1);
}

u16* nds_get_map(void) {
    return consoleGetDefault()->fontBgMap;
}

void nds_write_tile_row(int row, const u16* tiles) {
    u16* map = consoleGetDefault()->fontBgMap;
    if (!map) return;
    int base = row * 32;
    for (int i = 0; i < 32; i++) {
        map[base + i] = tiles[i];
    }
}

void nds_clear(void) {
    printf("\x1b[2J");
}

void nds_wait_vblank(void) {
    swiWaitForVBlank();
}

void nds_scan_keys(void) {
    scanKeys();
}

int nds_keys_held(void) {
    return keysHeld();
}

// The DS ARM9 (arm946e-s) has no exclusive-access atomics; LLVM emits calls to
// these GCC __sync_* builtins. Single-core: plain non-atomic implementations are
// correct here.
unsigned int __sync_fetch_and_add_4(volatile void* ptr, unsigned int val) {
    unsigned int* p = (unsigned int*)ptr;
    unsigned int tmp = *p;
    *p = tmp + val;
    return tmp;
}
unsigned int __sync_fetch_and_sub_4(volatile void* ptr, unsigned int val) {
    unsigned int* p = (unsigned int*)ptr;
    unsigned int tmp = *p;
    *p = tmp - val;
    return tmp;
}
unsigned int __sync_val_compare_and_swap_4(volatile void* ptr, unsigned int expected, unsigned int desired) {
    unsigned int* p = (unsigned int*)ptr;
    unsigned int cur = *p;
    if (cur == expected) {
        *p = desired;
    }
    return cur;
}
unsigned int __sync_lock_test_and_set_4(volatile void* ptr, unsigned int val) {
    unsigned int* p = (unsigned int*)ptr;
    unsigned int tmp = *p;
    *p = val;
    return tmp;
}
void __sync_synchronize(void) {}

// The DS has no real wall clock. picolibc's time() calls gettimeofday(); stub it.
int gettimeofday(struct timeval* tv, void* tz) {
    (void)tz;
    if (tv) {
        tv->tv_sec = 0;
        tv->tv_usec = 0;
    }
    return 0;
}
