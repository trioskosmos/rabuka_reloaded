"""Run one suite binary and capture stdout/stderr as RAW BYTES.

PowerShell `>` writes UTF-16LE and every downstream count then reads zero, so
this is the only sanctioned way to capture suite output in this tree.

usage: python tools/scratch/h_run.py <exe> <stem> [outdir]
"""
import os
import subprocess
import sys

exe = sys.argv[1]
stem = sys.argv[2]
outdir = sys.argv[3] if len(sys.argv) > 3 else '.'
os.makedirs(outdir, exist_ok=True)
op = os.path.join(outdir, stem + '.out')
ep = os.path.join(outdir, stem + '.err')
with open(op, 'wb') as o, open(ep, 'wb') as e:
    rc = subprocess.call([exe], stdout=o, stderr=e)
print('exit=%d out=%d err=%d bytes' % (
    rc, os.path.getsize(op), os.path.getsize(ep)))
