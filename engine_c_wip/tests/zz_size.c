#include "rabuka.h"
#include "test_game.h"
#include <stdio.h>
int main(void) {
    printf("GameState=%lu TestGame=%lu\n",
           (unsigned long)sizeof(GameState), (unsigned long)sizeof(TestGame));
    return 0;
}