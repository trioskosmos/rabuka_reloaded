"""Log in to decklog once, in a browser window this script opens.

This is the only step that needs a human. Cookies lifted from disk do not work because
the decklog session lives in Chrome's memory, so the login has to happen in a real
browser window. It uses a dedicated profile directory, so it is independent of your
normal Chrome and keeps working even if your main browser is closed.

Run it, log in when the window appears, and it exits on its own. After that,
harvest_all.py needs nothing further from you.
"""

import sys
from pathlib import Path

PROFILE = Path(__file__).resolve().parent / ".chrome-profile"
UA = ("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 "
      "(KHTML, like Gecko) Chrome/153.0.0.0 Safari/537.36")


def main():
    from playwright.sync_api import sync_playwright
    PROFILE.mkdir(parents=True, exist_ok=True)
    print(f"profile: {PROFILE}")
    print("a Chrome window will open on the decklog search page")
    print("log in there (any login method). this script exits when it detects the session.\n")

    with sync_playwright() as pw:
        ctx = pw.chromium.launch_persistent_context(
            user_data_dir=str(PROFILE),
            headless=False,
            args=["--disable-blink-features=AutomationControlled", "--no-sandbox"],
        )
        page = ctx.new_page()
        page.goto("https://decklog.bushiroad.com/search?c=11",
                  wait_until="domcontentloaded", timeout=60000)
        try:
            print("waiting as long as it takes - log in whenever you are ready")
            for i in range(900):
                page.wait_for_timeout(2000)
                try:
                    if "is_logined" in page.content():
                        print("\nsession detected - closing the window")
                        ctx.close()
                        return 0
                except Exception:
                    pass
                if i % 30 == 29:
                    print(f"  still waiting ({(i + 1) * 2 // 60} min)")
            print("still no login; leaving the window open anyway")
            return 1
        except KeyboardInterrupt:
            ctx.close()
            return 130


if __name__ == "__main__":
    sys.exit(main())