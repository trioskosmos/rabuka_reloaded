#!/bin/bash
# Capture a Virtual Jaguar frame with longer boot wait + full log.
ROM=${1:?rom}
OUT=${2:-/tmp/frame.png}
WAIT=${3:-12}

rm -f /tmp/rabuka.rom "$OUT" /tmp/vj.out /tmp/vj.log
cp "$ROM" /tmp/rabuka.rom

Xvfb :99 -screen 0 640x480x24 >/tmp/xvfb.log 2>&1 &
XVFB_PID=$!
sleep 2
export DISPLAY=:99

virtualjaguar --alpine --ntsc --no-bios --log /tmp/rabuka.rom >/tmp/vj.out 2>&1 &
VJ_PID=$!
sleep "$WAIT"

import -window root "$OUT" 2>/dev/null && echo "shot -> $OUT"
kill $VJ_PID $XVFB_PID 2>/dev/null
sleep 1
echo "=== vj.out ==="; tail -15 /tmp/vj.out 2>/dev/null
echo "=== vj.log ==="; tail -15 /tmp/vj.log 2>/dev/null
