"""Shared build utilities for card/ability compilation."""

import json
import struct
import hashlib
import zlib
from pathlib import Path
from typing import Any


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
    """Write generation_manifest.json for reproducibility."""
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

    (build_dir / "generation_manifest.json").write_text(
        json.dumps(manifest, indent=2) + "\n", encoding="utf-8"
    )


def generate_rust_include_bytes(blob_path: Path, const_name: str, relative_to: str = "build") -> str:
    """Generate Rust include_bytes! const declaration."""
    return f'pub const {const_name}: &[u8] = include_bytes!("../../../cards/{relative_to}/{blob_path.name}");'


def sha256_short(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()[:16]


def load_json(path: Path) -> Any:
    return json.loads(path.read_text(encoding="utf-8"))


def save_json(path: Path, data: Any) -> None:
    path.write_text(json.dumps(data, indent=2) + "\n", encoding="utf-8")


class BinaryEncoder:
    """Generic binary JSON encoder with interned strings."""

    TAG_NULL = 0x00
    TAG_FALSE = 0x01
    TAG_TRUE = 0x02
    TAG_INT = 0x03
    TAG_FLOAT = 0x04
    TAG_STR = 0x06
    TAG_ARRAY = 0x07
    TAG_OBJECT = 0x08
    TAG_OBJECT_VARIANT = 0x09

    def __init__(self, strings: StringTable):
        self.strings = strings
        self.data = bytearray()

    def encode(self, v: Any, in_effect_vec: bool = False, is_condition: bool = False) -> None:
        if v is None:
            self.data.append(self.TAG_NULL)
        elif isinstance(v, bool):
            self.data.append(self.TAG_TRUE if v else self.TAG_FALSE)
        elif isinstance(v, int):
            self.data.append(self.TAG_INT)
            self._encode_int(v)
        elif isinstance(v, float):
            self.data.append(self.TAG_FLOAT)
            self.data.extend(struct.pack("<d", v))
        elif isinstance(v, str):
            self.data.append(self.TAG_STR)
            self.data.extend(struct.pack("<H", self.strings.intern(v)))
        elif isinstance(v, list):
            self.data.append(self.TAG_ARRAY)
            write_len(self.data, len(v))
            for item in v:
                self.encode(item, in_effect_vec, is_condition)
        elif isinstance(v, dict):
            self._encode_dict(v, in_effect_vec, is_condition)
        else:
            self.data.append(self.TAG_NULL)

    def _encode_int(self, v: int) -> None:
        if v < 0:
            self.data.append(0xFF)
            self.data.extend(struct.pack("<I", v & 0xFFFFFFFF))
        elif v <= 0xFD:
            self.data.append(v & 0xFF)
        elif v <= 0xFFFF:
            self.data.append(0xFE)
            self.data.extend(struct.pack("<H", v))
        else:
            self.data.append(0xFF)
            self.data.extend(struct.pack("<I", v))

    def _encode_dict(
        self,
        v: dict,
        in_effect_vec: bool,
        is_condition: bool,
        variant_tag: int = None,
        skip_keys: set = None,
    ) -> None:
        skip_keys = skip_keys or set()
        filtered = {k: val for k, val in v.items() if k not in skip_keys}

        if variant_tag is not None:
            self.data.append(self.TAG_OBJECT_VARIANT)
            self.data.append(variant_tag)
        else:
            self.data.append(self.TAG_OBJECT)

        write_len(self.data, len(filtered))
        for k, val in filtered.items():
            self.data.extend(struct.pack("<H", self.strings.intern(str(k))))
            self.encode(val, in_effect_vec, is_condition)

    def bytes(self) -> bytes:
        return bytes(self.data)


def delta_encode_offsets(offsets: list[int]) -> list[int]:
    """Convert absolute offsets to u16 deltas."""
    deltas = [offsets[i + 1] - offsets[i] for i in range(len(offsets) - 1)]
    assert all(0 <= d <= 0xFFFF for d in deltas), "delta exceeds u16"
    return deltas


def write_string_blob(strings: list[str], out_path: Path) -> tuple[bytes, list[int]]:
    """Write concatenated string blob and return (blob_bytes, offsets)."""
    return write_blob_and_offsets(strings, out_path)