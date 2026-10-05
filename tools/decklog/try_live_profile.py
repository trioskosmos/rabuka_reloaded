"""Check whether the decklog session is now readable from a copy of the real profile.

Chrome keeps session cookies in memory, but a persistent cookie written hours ago would
be in the on-disk database. If a fresh copy of the profile is already logged in, the
harvest can run against that copy and no login is needed at all.

Run with --use-copy to point the harvester at the verified copy.
"""

import json
import shutil
import sys
import time
from pathlib import Path

SRC = Path.home() / "AppData/Local/Google/Chrome/User Data"
COPY = Path.home() / "AppData/Local/Temp/kilo/chrome-live"
UA = ("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 "
      "(KHTML, like Gecko) Chrome/153.0.0.0 Safari/537.36")


def copy_profile():
    if COPY.exists():
        shutil.rmtree(COPY, ignore_errors=True)
    (COPY / "Default" / "Network").mkdir(parents=True, exist_ok=True)
    files = [
        (SRC / "Local State", COPY / "Local State"),
        (SRC / "Default" / "Preferences", COPY / "Default" / "Preferences"),
        (SRC / "Default" / "Secure Preferences", COPY / "Default" / "Secure Preferences"),
        (SRC / "Default" / "Network" / "Cookies", COPY / "Default" / "Network" / "Cookies"),
    ]
    for extra in ("Cookies-wal", "Cookies-shm", "Cookies-journal"):
        files.append((SRC / "Default" / "Network" / extra,
                      COPY / "Default" / "Network" / extra))
    ok, failed = 0, []
    for s, d in files:
        try:
            if s.exists():
                shutil.copy2(s, d)
                ok += 1
        except OSError as exc:
            failed.append(f"{s.name}: {exc}")
    # profiles other than Default, in case that's the one in use
    for prof in SRC.glob("Profile *"):
        try:
            shutil.copytree(prof, COPY / prof.name, ignore=shutil.ignore_patterns(
                "Cache", "Code Cache", "GPUCache", "Service Worker", "blob_storage",
                "IndexedDB", "Local Storage", "Session Storage"))
            ok += 1
        except OSError as exc:
            failed.append(f"{prof.name}: {exc}")
    print(f"copied {ok} items" + (f", {len(failed)} failed: {failed[:3]}" if failed else ""))
    return ok


def check():
    from playwright.sync_api import sync_playwright
    for channel in ("chrome", "chromium"):
        try:
            with sync_playwright() as pw:
                ctx = pw.chromium.launch_persistent_context(
                    user_data_dir=str(COPY), headless=True, channel=channel,
                    args=["--disable-blink-features=AutomationControlled", "--no-sandbox"])
                page = ctx.new_page()
                page.goto("https://decklog.bushiroad.com/search?c=11",
                          wait_until="domcontentloaded", timeout=60000)
                page.wait_for_timeout(3000)
                body = page.content()
                logged_in = "is_logined" in body
                bc = ctx.cookies()
                ours = [c for c in bc if "bushiroad" in (c.get("domain") or "")]
                print(f"{channel:9} is_logined={logged_in}  cookies={len(bc)} "
                      f"bushiroad={len(ours)}")
                for c in ours:
                    print(f"           {c['name']:26} len={len(c.get('value',''))}")
                ctx.close()
                if logged_in:
                    return True
        except Exception as exc:
            print(f"{channel:9} ERROR {str(exc)[:120]}")
    return False


if __name__ == "__main__":
    copy_profile()
    time.sleep(1)
    if check():
        print(f"\nVERIFIED LOGGED IN: harvest can use {COPY}")
        sys.exit(0)
    print("\nstill memory-only. a login is required once.")