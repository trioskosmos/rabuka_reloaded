import io

txt = io.open(
    r"C:\Users\trios\AppData\Local\Temp\opencode\missing.txt",
    encoding="utf-16",
    errors="replace",
).read()
lines = txt.splitlines()
# locate SECTION 6
i = next(k for k, ln in enumerate(lines) if "SECTION 6" in ln)
body = lines[i + 1 :]
out = io.open(
    r"C:\Users\trios\AppData\Local\Temp\opencode\missing6.txt", "w", encoding="utf-8"
)
count = body[0].strip() if body else "?"
out.write(count + "\n\n")
for ln in body[1:]:
    if ln.strip():
        out.write(ln.strip() + "\n")
out.close()
print(count)
