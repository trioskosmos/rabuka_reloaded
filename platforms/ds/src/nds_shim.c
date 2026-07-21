#include <nds.h>
#include <nds/arm9/console.h>
#include <nds/arm9/video.h>
#include <nds/arm9/background.h>
#include <calico/system/irq.h>
#include <stdio.h>

#define REG_TM0DATA  (*(vu16*)0x04000100)
#define REG_TM0CNT   (*(vu16*)0x04000102)

#define TIMER_ENABLE     (1<<7)
#define TIMER_PRESCALER_64 (1<<0)

void __sync_synchronize_none(void) {
}

int* __errno(void) {
    static int _calico_errno = 0;
    return &_calico_errno;
}

int __atomic_fetch_add_4(int* ptr, int val, int memorder) {
    (void)memorder;
    int tmp = *ptr;
    *ptr = tmp + val;
    return tmp;
}

int __atomic_fetch_sub_4(int* ptr, int val, int memorder) {
    (void)memorder;
    int tmp = *ptr;
    *ptr = tmp - val;
    return tmp;
}

void __atomic_store_4(int* ptr, int val, int memorder) {
    (void)memorder;
    *ptr = val;
}

int __atomic_load_4(int* ptr, int memorder) {
    (void)memorder;
    return *ptr;
}

int __atomic_compare_exchange_4(int* ptr, int* expected, int desired, int weak, int success_memorder, int failure_memorder) {
    (void)weak;
    (void)success_memorder;
    (void)failure_memorder;
    int cur = *ptr;
    if (cur == *expected) {
        *ptr = desired;
        return 1;
    } else {
        *expected = cur;
        return 0;
    }
}

void nds_init(void) {
    videoSetMode(MODE_0_2D);
    videoSetModeSub(MODE_0_2D);
    vramSetBankC(VRAM_C_SUB_BG);
    
    irqSet(IRQ_VBLANK, NULL);
    irqEnable(IRQ_VBLANK);
    
    consoleDemoInit();
    iprintf("\x1b[2J");
    iprintf("Rabuka DS\n");
    iprintf("==========\n");
    iprintf("Boot OK!\n");
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
    PrintConsole* con = consoleGetDefault();
    consoleSetWindow(con, 0, row, 32, 1);
    iprintf("\x1b[%d;0H                                         ", row + 1);
    consoleSetWindow(con, 0, 0, 32, 24);
}

void nds_set_cursor(int row, int col) {
    iprintf("\x1b[%d;%dH", row + 1, col + 1);
}

void nds_scan_keys(void) {
    scanKeys();
}

int nds_key_held(void) {
    return keysHeld();
}

void nds_wait_vblank(void) {
    volatile u16* vcount = (volatile u16*)0x04000006;
    while (*vcount < 192) {}
    while (*vcount >= 192) {}
}

unsigned long long nds_get_tick(void) {
    return (unsigned long long)REG_TM0DATA;
}


