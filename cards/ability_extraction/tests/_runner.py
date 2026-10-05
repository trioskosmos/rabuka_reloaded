"""
Shared test runner for the ability_extraction parser tests.

There is no pytest here and there must not be: these tests gate the generated
corpus, and a runner that silently collects nothing is worse than no runner.
So discovery is explicit and a file that yields zero tests is a hard failure.

Usage, at the bottom of a test module:

    from _runner import run_module
    if __name__ == "__main__":
        run_module(globals())

Collects module-level ``test_*`` callables and ``Test*`` classes' ``test_*``
methods. A test is never skipped silently: discovery is by prefix, so a test
function that exists is a test function that runs.
"""

import sys
import traceback


def _make_console_lossless():
    """Stop a non-ASCII test message from killing the report.

    This runner's whole job is to print an actionable diagnostic. Printing a
    traceback straight to stdout on a cp932/cp936 console raises
    UnicodeEncodeError the moment the message contains anything non-ASCII — an
    em-dash, a `』` quoting mark, a card name — and because the print happens
    inside the failure-reporting loop, that aborts the loop and every
    REMAINING failure's diagnostic is lost too. The symptom is a bare
    UnicodeEncodeError instead of the parser bug you were hunting, which is the
    worst possible trade: a red run that tells you nothing.

    `errors="replace"` keeps the report going and marks the unencodable
    characters, rather than dropping them silently. Streams without
    `reconfigure` (pytest capture, StringIO, a closed pipe) are left alone.
    """
    for stream in (sys.stdout, sys.stderr):
        reconfigure = getattr(stream, "reconfigure", None)
        if reconfigure is None:
            continue
        try:
            reconfigure(errors="replace")
        except (ValueError, OSError):
            pass


def _collect(namespace):
    """(display_name, zero_arg_callable) for every test in `namespace`."""
    found = []
    for name, obj in sorted(namespace.items()):
        if name.startswith("test_") and callable(obj):
            found.append((name, obj))
        elif name.startswith("Test") and isinstance(obj, type):
            for attr in sorted(vars(obj)):
                if attr.startswith("test_"):
                    method = getattr(obj, attr)
                    if callable(method):
                        found.append((f"{name}::{attr}", method))
    return found


def _invoke(fn):
    """Call a collected test, binding a `self` for class methods if needed."""
    argcount = fn.__code__.co_argcount
    if argcount >= 1 and fn.__code__.co_varnames[0] == "self":
        return fn(None)
    return fn()


def run_module(namespace, title=None):
    """Run every test in `namespace`; print a report; return an exit code."""
    _make_console_lossless()
    tests = _collect(namespace)
    if not tests:
        print(f"FAIL: {title or namespace.get('__name__')}: collected 0 tests")
        return 1

    passed = 0
    failures = []
    for name, fn in tests:
        try:
            _invoke(fn)
            passed += 1
        except Exception:
            failures.append((name, traceback.format_exc()))

    for name, tb in failures:
        print(f"\n  FAIL  {name}\n" + "".join(f"        {ln}" for ln in tb.splitlines(True)))

    print(f"\n{passed} passed, {len(failures)} failed  ({len(tests)} collected)")
    return 1 if failures else 0
