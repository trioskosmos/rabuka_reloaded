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

unsigned char __atomic_load_1(unsigned char* ptr, int memorder) {
    (void)memorder;
    return *ptr;
}

void __atomic_store_1(unsigned char* ptr, unsigned char val, int memorder) {
    (void)memorder;
    *ptr = val;
}

unsigned char __atomic_exchange_1(unsigned char* ptr, unsigned char val, int memorder) {
    (void)memorder;
    unsigned char tmp = *ptr;
    *ptr = val;
    return tmp;
}

int __atomic_compare_exchange_1(unsigned char* ptr, unsigned char* expected, unsigned char desired, int weak, int success_memorder, int failure_memorder) {
    (void)weak;
    (void)success_memorder;
    (void)failure_memorder;
    unsigned char cur = *ptr;
    if (cur == *expected) {
        *ptr = desired;
        return 1;
    } else {
        *expected = cur;
        return 0;
    }
}

int __atomic_fetch_add_1(unsigned char* ptr, unsigned char val, int memorder) {
    (void)memorder;
    unsigned char tmp = *ptr;
    *ptr = tmp + val;
    return tmp;
}

int __atomic_fetch_sub_1(unsigned char* ptr, unsigned char val, int memorder) {
    (void)memorder;
    unsigned char tmp = *ptr;
    *ptr = tmp - val;
    return tmp;
}

static u16* _tile_map = NULL;

void nds_init(void) {
    videoSetMode(MODE_0_2D);
    videoSetModeSub(MODE_0_2D);
    vramSetBankC(VRAM_C_SUB_BG);
    
    irqSet(IRQ_VBLANK, NULL);
    irqEnable(IRQ_VBLANK);
    
    // consoleDemoInit returns the ACTIVE console (not the static default),
    // which has fontBgMap correctly set after bgInit.
    PrintConsole* con = consoleDemoInit();
    if (con) {
        _tile_map = con->fontBgMap;
    } else {
        // Fallback: sub BG0 after consoleInit(..., mapBase=22)
        _tile_map = bgGetMapPtr(4);
    }
    iprintf("\x1b[2J");
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

// Access to console tile map for direct rendering
u16* nds_get_tilemap(void) {
    return _tile_map;
}

u16* nds_get_tilegfx(void) {
    // The default console's fontBgGfx is also 0 — not currently used.
    return NULL;
}

// Write a row of 32 tile indices directly to the tile map (flicker-free)
void nds_write_tile_row(int row, const u16* tiles) {
    u16* map = nds_get_tilemap();
    if (!map) return;
    int base = row * 32;
    for (int i = 0; i < 32; i++) {
        map[base + i] = tiles[i];
    }
}

// Clear one tile map row
void nds_clear_tile_row(int row) {
    u16* map = nds_get_tilemap();
    if (!map) return;
    int base = row * 32;
    for (int i = 0; i < 32; i++) {
        map[base + i] = 0;
    }
}

void nds_scan_keys(void) {
    scanKeys();
}

int nds_key_held(void) {
    return keysHeld();
}

void nds_wait_vblank(void) {
    swiWaitForVBlank();
}

unsigned long long nds_get_tick(void) {
    return (unsigned long long)REG_TM0DATA;
}


