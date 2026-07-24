extern int rabuka_main(void);

void _arch_main(void) {
    rabuka_main();
}

int main(int argc, char* argv[]) {
    (void)argc; (void)argv;
    return 0;
}

void library_init(void) {}
void library_shutdown(void) {}
void export_init(void) {}
void* export_lookup(const char* name) { (void)name; return 0; }

const char* kos_get_banner(void) {
    return "Rabuka Reloaded for Dreamcast\n";
}

unsigned int _tdata_size = 0;
unsigned int _tbss_size = 0;
unsigned int _tdata_align = 1;
unsigned int _tbss_align = 1;

const unsigned char romdisk_data[1] = "";
const unsigned char* ___kos_romdisk = romdisk_data;
