#include <gccore.h>
#include <ogc/gx.h>
#include <stdlib.h>

extern void rabuka_main(void);
extern void display_init(void);

int main(int argc, char **argv) {
    PAD_Init();
    display_init();
    rabuka_main();
    return 0;
}
