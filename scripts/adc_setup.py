import subprocess, os, sys, threading, time

gcloud = os.path.expanduser("~/google-cloud-sdk/bin/gcloud.cmd")

proc = subprocess.Popen(
    [gcloud, "auth", "application-default", "login", "--no-launch-browser"],
    stdout=subprocess.PIPE,
    stderr=subprocess.STDOUT,
    stdin=subprocess.PIPE,
    text=True,
    bufsize=1,
)

lines = []
done = False


def reader():
    global done
    while True:
        line = proc.stdout.readline()
        if not line:
            break
        lines.append(line)
        if "enter the verification code" in line.lower():
            done = True
            break


t = threading.Thread(target=reader, daemon=True)
t.start()

time.sleep(2)
while not done:
    time.sleep(0.5)

for line in lines:
    print(line, end="", flush=True)

if done:
    code = input("Paste verification code: ")
    proc.stdin.write(code + "\n")
    proc.stdin.flush()
    for line in proc.stdout:
        print(line, end="", flush=True)
