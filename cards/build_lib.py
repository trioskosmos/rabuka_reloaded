"""Shared build utilities for card/ability compilation."""

import json
import struct
import hashlib
import zlib
from pathlib import Path


def write_len(out: bytearray, n: int) -> None:
    """Write a container length as u8 with 0xFE escape for large values."""
    if n < 0xFE:
        out.append(n)
    else:
        out.append(0xFE)
        out.extend(struct.pack("<H", n))


def read_len(bc: bytes, pos: int) -> tuple[int, int]:
    """Read a container length written by write_len. Returns (n, new_pos)."""
    if pos >= len(bc):
        return 0, pos
    b = bc[pos]
    if b < 0xFE:
        return b, pos + 1
    if pos + 3 > len(bc):
        return 0, len(bc)
    return (bc[pos + 1] | (bc[pos + 2] << 8)), pos + 3


class StringTable:
    """String interning with u16 indices (index 0 = empty string)."""

    def __init__(self):
        self._strings = [""]
        self._index = {"": 0}

    def intern(self, s: str) -> int:
        if not s:
            return 0
        if s not in self._index:
            if len(self._strings) >= 0x10000:
                return 0xFFFF
            self._index[s] = len(self._strings)
            self._strings.append(s)
        return self._index[s]

    def __iter__(self):
        return iter(self._strings)

    def __len__(self):
        return len(self._strings)

    def get_strings(self) -> list[str]:
        return self._strings


def encode_strtab(strings: list[str]) -> bytes:
    """Encode string table as u16 length + UTF-8 bytes per entry."""
    out = bytearray()
    for s in strings:
        encoded = s.encode("utf-8")
        out.extend(struct.pack("<H", len(encoded)))
        out.extend(encoded)
    return bytes(out)


def write_blob_and_offsets(strings: list[str], out_path: Path) -> tuple[bytes, list[int]]:
    """Write concatenated string blob and return (blob_bytes, offsets)."""
    blob_parts = []
    offsets = [0]
    for s in strings:
        encoded = s.encode("utf-8")
        blob_parts.append(encoded)
        offsets.append(offsets[-1] + len(encoded))
    blob = b"".join(blob_parts)
    out_path.write_bytes(blob)
    return blob, offsets


def compress_with_header(data: bytes, magic: bytes, version: int = 1) -> bytes:
    """Compress data with magic+version header."""
    header = magic + version.to_bytes(4, "little")
    return zlib.compress(header + data, level=9)


# Top-level manifest keys that no single writer owns, so a rebuild must carry
# them forward instead of dropping them. See write_generation_manifest().
PRESERVED_MANIFEST_KEYS = ("c_artifacts",)


def write_generation_manifest(
    build_dir: Path,
    schema: str,
    compiler: str,
    input_source: str,
    input_hash: str,
    input_count: int,
    output_bytes: int,
    compressed_bytes: int,
    bin_name: str,
    extra: dict = None,
) -> None:
    """Write generation_manifest.json for reproducibility.

    PRESERVES KEYS THIS FUNCTION DOES NOT OWN (2026-10-04).

    Two writers share this one path -- compile_abilities.py and compile_cards.py
    -- and each used to build the dict from scratch and write it over whatever
    was there. That silently deleted the top-level `c_artifacts` block, the
    only cross-engine provenance record in the tree: the digest of the COMMITTED
    C tables (gen_data.c / bytecode_blob.c / src/abilities_strings.bin). Its own
    comment warned that the next compile_abilities.py run would delete it, and it
    did, which left the C's generation unverifiable against the Rust one.

    A rebuild therefore now merges: keys named in PRESERVED_MANIFEST_KEYS are
    read from the existing manifest and carried forward. `output` is still
    replaced wholesale, because it describes the run that just happened.

    This only preserves the RECORD. It cannot tell whether the record is still
    true, and preserving a stale record silently is its own hazard -- so the
    consumer fails loudly on disagreement instead:
    engine_c_wip/tests/test_artifacts_consistent.c compares every c_artifacts
    field against the values compiled into gen_data.c and against the canonical
    blob in cards/build/. Absent block, stale block, and disagreeing block are
    all FAILs there, never "cannot cross-check, assume fine".
    """
    git_hash = "unknown"
    try:
        import subprocess
        result = subprocess.run(
            ["git", "rev-parse", "--short", "HEAD"],
            capture_output=True,
            text=True,
            cwd=str(build_dir.parent.parent),
            timeout=5,
        )
        if result.returncode == 0:
            git_hash = result.stdout.strip()
    except Exception:
        pass

    manifest = {
        "schema": schema,
        "compiler": compiler,
        "engine_commit": git_hash,
        "input": {
            "source": input_source,
            "sha256": input_hash,
            "count": input_count,
        },
        "output": {
            "bytes": output_bytes,
            "compressed_bytes": compressed_bytes,
            "sha256": hashlib.sha256(open(build_dir / f"{bin_name}.bin", "rb").read()).hexdigest()[:16],
            "compressed_sha256": hashlib.sha256(open(build_dir / f"{bin_name}.bin.z", "rb").read()).hexdigest()[:16],
        },
    }
    if extra:
        manifest["output"].update(extra)

    # ── carry forward the keys this function does not own ──────────────────
    # See the docstring. A missing or unparseable existing manifest is not an
    # error here: this is a rebuild, and the record simply starts fresh. The
    # harness is what turns "no record" into a FAIL, so staying quiet here is
    # correct -- raising would make a first-ever build impossible.
    man_path = build_dir / "generation_manifest.json"
    if man_path.exists():
        try:
            prev = json.loads(man_path.read_text(encoding="utf-8"))
        except (ValueError, OSError):
            prev = {}
        if isinstance(prev, dict):
            for k in PRESERVED_MANIFEST_KEYS:
                if k in prev and k not in manifest:
                    manifest[k] = prev[k]
                    print(f"  manifest: preserved '{k}' from the previous manifest")

    man_path.write_text(
        json.dumps(manifest, indent=2) + "\n", encoding="utf-8"
    )


def sha256_short(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()[:16]


def delta_encode_offsets(offsets: list[int]) -> list[int]:
    """Convert absolute offsets to u16 deltas."""
    deltas = [offsets[i + 1] - offsets[i] for i in range(len(offsets) - 1)]
    assert all(0 <= d <= 0xFFFF for d in deltas), "delta exceeds u16"
    return deltas


def write_string_blob(strings: list[str], out_path: Path) -> tuple[bytes, list[int]]:
    """Write concatenated string blob and return (blob_bytes, offsets)."""
    return write_blob_and_offsets(strings, out_path)