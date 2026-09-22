#!/bin/bash
# manual link test
cd /mnt/c/Users/trios/OneDrive/Documents/rabuka_reloaded/engine_c
OBJS=""
for o in platforms/cdi/output/src_*.o platforms/cdi/output/crt0.o platforms/cdi/output/cdi_lib.o platforms/cdi/output/cdi_main.o; do
  OBJS="$OBJS $o"
done
m68k-linux-gnu-gcc -nostdlib -Tplatforms/cdi/cdi.ld -Wl,--gc-sections \
  -Wl,--start-group $OBJS -lgcc -Wl,--end-group -o /tmp/test.elf
echo "RC=$?"
ls -l /tmp/test.elf
