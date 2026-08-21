#!/bin/bash
cd /root/dcbuild
/root/wabt-1.0.41/bin/wasm-interp hw.wasm --dummy-import-func <<'EOF'
(invoke "rabuka_wasm_match" (i32.const 24249))
(invoke "rabuka_wasm_heap_highwater")
EOF
