import re
body = """static void gen_x(void){
    TestGame tg; test_game_new(&tg);
    // action result consumed: let keke1 = test_id(&tg, "PL!SP-bp4-013-N");
    test_add_to_hand(&tg, keke1);
    test_drain_auto_choices(&tg);
}
"""
real = re.sub(r'//.*', '', body)
print('REAL:', repr(real))
print('keke1 in real:', 'keke1' in real)