#include <nds.h>
#include <stdio.h>

int main(void) {
    videoSetMode(MODE_0_2D);
    videoSetModeSub(MODE_0_2D);
    vramSetBankC(VRAM_C_SUB_BG);
    consoleDemoInit();

    iprintf("Rabuka DS Test\n");
    iprintf("==============\n");
    iprintf("If you see this,\n");
    iprintf("display works!\n");

    while(1) {
        swiWaitForVBlank();
    }

    return 0;
}
