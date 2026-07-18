#include <nds.h>
#include <stdio.h>
#include <string.h>

#define REG_TM0DATA  (*(vu16*)0x04000100)
#define REG_TM0CNT   (*(vu16*)0x04000102)

#define TIMER_ENABLE     (1<<7)
#define TIMER_PRESCALER_64 (1<<0)

void nds_init(void) {
    consoleDemoInit();
    iprintf("\x1b[2J");
    iprintf("Rabuka DS\n");
    iprintf("==========\n");

    REG_TM0CNT = 0;
    REG_TM0DATA = 0;
    REG_TM0CNT = TIMER_ENABLE | TIMER_PRESCALER_64;
}

void nds_console_clear(void) {
    iprintf("\x1b[2J");
}

void nds_print(const char* text) {
    iprintf("%s", text);
}

void nds_println(const char* text) {
    iprintf("%s\n", text);
}

void nds_clear_line(int row) {
    iprintf("\x1b[%d;0H", row + 1);
    for (int i = 0; i < 32; i++) iprintf(" ");
    iprintf("\x1b[%d;0H", row + 1);
}

void nds_set_cursor(int row, int col) {
    iprintf("\x1b[%d;%dH", row + 1, col + 1);
}

void nds_scan_keys(void) {
    scanKeys();
}

int nds_key_down(void) {
    return keysDown();
}

int nds_key_held(void) {
    return keysHeld();
}

void nds_wait_vblank(void) {
    __asm__ volatile("swi 0x05");
}

unsigned long long nds_get_tick(void) {
    return (unsigned long long)REG_TM0DATA;
}
