#include <stdio.h>
#include "rabuka.h"
#include "test_game.h"
static TestGame tg;
int main(int argc,char**argv){
  printf("SIZEOF_GAMESTATE %zu\n", sizeof(GameState));
  printf("SIZEOF_TESTGAME  %zu\n", sizeof(TestGame));
  return 0;
}
