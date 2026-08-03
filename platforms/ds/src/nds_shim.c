#include <nds.h>
#include <nds/arm9/console.h>
#include <nds/arm9/video.h>
#include <nds/arm9/background.h>
#include <stdio.h>
#include <stdarg.h>
#include <sys/time.h>

void nds_init(void) {
    videoSetMode(MODE_0_2D);
    videoSetModeSub(MODE_0_2D);
    vramSetBankC(VRAM_C_SUB_BG);
    irqSet(IRQ_VBLANK, 0);
    irqEnable(IRQ_VBLANK);
    consoleDemoInit();
    printf("\x1b[2J");
}

void nds_printf(const char* fmt, ...) {
    va_list args;
    va_start(args, fmt);
    vprintf(fmt, args);
    va_end(args);
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

// The DS has no real wall clock. picolibc's time() calls gettimeofday(); stub it.
int gettimeofday(struct timeval* tv, void* tz) {
    (void)tz;
    if (tv) {
        tv->tv_sec = 0;
        tv->tv_usec = 0;
    }
    return 0;
}
