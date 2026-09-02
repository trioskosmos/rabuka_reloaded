import subprocess, os
os.chdir('engine/tests')
out = subprocess.run(['grep','-rln','miracle_wave_q182','.'],capture_output=True,text=True)
print(out.stdout)
print(out.stderr)