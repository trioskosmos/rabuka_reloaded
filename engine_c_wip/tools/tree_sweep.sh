#!/bin/bash
# Prove two trees differ ONLY in the named files. SHA-256 over every file.
A="$1"; B="$2"
cd "$A" || exit 1
find . -type f ! -name '*.o' ! -name 'rb_engine*' ! -path './out_*' ! -name '*.log' | sort > /tmp/la.txt
cd "$B" || exit 1
find . -type f ! -name '*.o' ! -name 'rb_engine*' ! -path './out_*' ! -name '*.log' | sort > /tmp/lb.txt
diff /tmp/la.txt /tmp/lb.txt > /tmp/ldiff.txt || true
if [ -s /tmp/ldiff.txt ]; then
  echo "FILE-LIST DIFFERS:"; cat /tmp/ldiff.txt
else
  echo "file lists identical ($(wc -l < /tmp/la.txt) files)"
fi
same=0; differ=0
while read -r f; do
  ha=$(cd "$A" && sha256sum "$f" | sed -n 's/^[^0-9a-fA-F]*\([0-9a-fA-F]\{64\}\).*/\1/p')
  hb=$(cd "$B" && sha256sum "$f" | sed -n 's/^[^0-9a-fA-F]*\([0-9a-fA-F]\{64\}\).*/\1/p')
  if [ "$ha" = "$hb" ]; then same=$((same+1)); else differ=$((differ+1)); echo "DIFFER: $f"; fi
done < /tmp/la.txt
echo "identical=$same differ=$differ"