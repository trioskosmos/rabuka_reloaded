#include <nds.h>
#include <nds/arm9/console.h>
#include <nds/arm9/video.h>
#include <nds/arm9/background.h>
#include <calico/system/irq.h>
#include <stdio.h>
#include <stdlib.h>

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

unsigned int __atomic_fetch_add_4(volatile void* ptr, unsigned int val, int memorder) {
    (void)memorder;
    unsigned int* p = (unsigned int*)ptr;
    unsigned int tmp = *p;
    *p = tmp + val;
    return tmp;
}

unsigned int __atomic_fetch_sub_4(volatile void* ptr, unsigned int val, int memorder) {
    (void)memorder;
    unsigned int* p = (unsigned int*)ptr;
    unsigned int tmp = *p;
    *p = tmp - val;
    return tmp;
}

void __atomic_store_4(volatile void* ptr, unsigned int val, int memorder) {
    (void)memorder;
    *(unsigned int*)ptr = val;
}

unsigned int __atomic_load_4(const volatile void* ptr, int memorder) {
    (void)memorder;
    return *(const unsigned int*)ptr;
}

_Bool __atomic_compare_exchange_4(volatile void* ptr, void* expected, unsigned int desired, _Bool weak, int success_memorder, int failure_memorder) {
    (void)weak;
    (void)success_memorder;
    (void)failure_memorder;
    unsigned int cur = *(unsigned int*)ptr;
    unsigned int* exp = (unsigned int*)expected;
    if (cur == *exp) {
        *(unsigned int*)ptr = desired;
        return 1;
    } else {
        *exp = cur;
        return 0;
    }
}

unsigned char __atomic_load_1(const volatile void* ptr, int memorder) {
    (void)memorder;
    return *(const unsigned char*)ptr;
}

void __atomic_store_1(volatile void* ptr, unsigned char val, int memorder) {
    (void)memorder;
    *(unsigned char*)ptr = val;
}

unsigned char __atomic_exchange_1(volatile void* ptr, unsigned char val, int memorder) {
    (void)memorder;
    unsigned char* p = (unsigned char*)ptr;
    unsigned char tmp = *p;
    *p = val;
    return tmp;
}

_Bool __atomic_compare_exchange_1(volatile void* ptr, void* expected, unsigned char desired, _Bool weak, int success_memorder, int failure_memorder) {
    (void)weak;
    (void)success_memorder;
    (void)failure_memorder;
    unsigned char cur = *(unsigned char*)ptr;
    unsigned char* exp = (unsigned char*)expected;
    if (cur == *exp) {
        *(unsigned char*)ptr = desired;
        return 1;
    } else {
        *exp = cur;
        return 0;
    }
}

unsigned char __atomic_fetch_add_1(volatile void* ptr, unsigned char val, int memorder) {
    (void)memorder;
    unsigned char* p = (unsigned char*)ptr;
    unsigned char tmp = *p;
    *p = tmp + val;
    return tmp;
}

unsigned char __atomic_fetch_sub_1(volatile void* ptr, unsigned char val, int memorder) {
    (void)memorder;
    unsigned char* p = (unsigned char*)ptr;
    unsigned char tmp = *p;
    *p = tmp - val;
    return tmp;
}

static u16* _tile_map = NULL;
PrintConsole _top_console;

// Forward declarations for top-screen functions used in nds_init
void nds_top_clear(void);
void nds_top_print(const char* text);
void nds_top_println(const char* text);

void nds_init(void) {
    videoSetMode(MODE_0_2D);
    videoSetModeSub(MODE_0_2D);
    vramSetBankC(VRAM_C_SUB_BG);

    irqSet(IRQ_VBLANK, NULL);
    irqEnable(IRQ_VBLANK);

    PrintConsole* con = consoleDemoInit();
    if (con) {
        _tile_map = con->fontBgMap;
    } else {
        _tile_map = bgGetMapPtr(4);
    }

    consoleInit(&_top_console, 1, BgType_Text4bpp, BgSize_T_256x256, 0, 2, false, true);

    iprintf("\x1b[2J");
    nds_top_clear();

    REG_TM0CNT = 0;
    REG_TM0DATA = 0;
    REG_TM0CNT = TIMER_ENABLE | TIMER_PRESCALER_64;
}

void nds_top_clear(void) {
    PrintConsole* old = consoleGetDefault();
    consoleSelect(&_top_console);
    iprintf("\x1b[2J");
    consoleSelect(old);
}

void nds_top_print(const char* text) {
    PrintConsole* old = consoleGetDefault();
    consoleSelect(&_top_console);
    iprintf("%s", text);
    consoleSelect(old);
}

void nds_top_println(const char* text) {
    PrintConsole* old = consoleGetDefault();
    consoleSelect(&_top_console);
    iprintf("%s\n", text);
    consoleSelect(old);
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

void nds_dbg_direct(int row, const char* text) {
    u16* map = _tile_map;
    if (!map) return;
    int base = row * 32;
    int i = 0;
    while (text[i] && i < 32) {
        map[base + i] = 0xF000 | (unsigned char)text[i];
        i++;
    }
    while (i < 32) {
        map[base + i] = 0xF000 | 0x20;
        i++;
    }
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

u16* nds_get_tilemap(void) {
    return _tile_map;
}

u16* nds_get_tilegfx(void) {
    return NULL;
}

void nds_write_tile_row(int row, const u16* tiles) {
    u16* map = nds_get_tilemap();
    if (!map) return;
    int base = row * 32;
    for (int i = 0; i < 32; i++) {
        map[base + i] = tiles[i];
    }
}

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

extern char __heap_start_ntr;
static char* _brk = &__heap_start_ntr;

void* _sbrk(int incr) {
    char* prev = _brk;
    if ((unsigned int)(_brk + incr) > 0x02400000) {
        return (void*)-1;
    }
    _brk += incr;
    return prev;
}

void* ds_malloc(unsigned int size) { return malloc(size); }
void ds_free(void* ptr) { free(ptr); }
void* ds_realloc(void* ptr, unsigned int size) { return realloc(ptr, size); }
