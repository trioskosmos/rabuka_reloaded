extern int main(void);

__attribute__((used, section(".text")))
int arch_main(void) {
    return main();
}
