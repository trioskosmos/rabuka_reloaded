import argparse
import copy
import hashlib
import json
import math
import os
from pathlib import Path
import subprocess
import sys
import sysconfig
import tomllib


REPO_ROOT = Path(__file__).resolve().parent.parent
MARKER = ".flamegraph-replay.json"
IDENTITY_NAME = "replay-build-identity.json"
IDENTITY_ENV = "RABUKA_REPLAY_BUILD_IDENTITY"
DEPENDENCY_SECTIONS = ("dependencies", "dev-dependencies", "build-dependencies")


def sha256(data):
    return hashlib.sha256(data).hexdigest()


def canonical_json(value):
    return json.dumps(value, sort_keys=True, separators=(",", ":"))


def toml_value(value):
    if isinstance(value, str):
        return json.dumps(value, ensure_ascii=False).replace("\x7f", "\\u007f")
    if isinstance(value, bool):
        return "true" if value else "false"
    if isinstance(value, int):
        return str(value)
    if isinstance(value, float) and math.isfinite(value):
        return repr(value)
    if isinstance(value, list):
        return "[" + ", ".join(toml_value(item) for item in value) + "]"
    if isinstance(value, dict) and all(isinstance(key, str) for key in value):
        return "{ " + ", ".join(
            f"{toml_value(key)} = {toml_value(item)}" for key, item in value.items()
        ) + " }"
    raise ValueError(f"Unsupported TOML value: {type(value).__name__}")


def serialize_manifest(manifest):
    text = "\n".join(f"{toml_value(key)} = {toml_value(value)}" for key, value in manifest.items()) + "\n"
    if tomllib.loads(text) != manifest:
        raise ValueError("Manifest cannot be serialized without changes")
    return text


def validate_dependencies(dependencies):
    if not isinstance(dependencies, dict):
        raise ValueError("Dependency section must be a table")
    for name, value in dependencies.items():
        if isinstance(value, str):
            continue
        if not isinstance(value, dict):
            raise ValueError(f"Unsupported dependency: {name}")
        if "workspace" in value:
            raise ValueError(f"Workspace dependency is unsupported: {name}")
        if "path" in value:
            path = value["path"]
            if not isinstance(path, str) or not Path(path).is_absolute():
                raise ValueError(f"Relative path dependency is unsupported: {name}")


def derive_manifest(engine_root, harness):
    with (engine_root / "Cargo.toml").open("rb") as source:
        original = tomllib.load(source)
    allowed = {"package", "lib", "features", "target", "profile", "bin", "bench", "test", "example", *DEPENDENCY_SECTIONS}
    if original.keys() - allowed:
        raise ValueError(f"Unsupported manifest sections: {sorted(original.keys() - allowed)}")
    package = original.get("package")
    if not isinstance(package, dict):
        raise ValueError("Manifest requires a package table")
    package_allowed = {
        "name", "version", "edition", "rust-version", "authors", "description",
        "documentation", "homepage", "repository", "license", "keywords", "categories",
        "publish", "default-run", "build", "autolib", "autobins", "autoexamples",
        "autotests", "autobenches",
    }
    if package.keys() - package_allowed:
        raise ValueError(f"Unsupported package fields: {sorted(package.keys() - package_allowed)}")
    if any(isinstance(value, dict) for value in package.values()):
        raise ValueError("Inherited package fields are unsupported")
    for key in ("name", "version", "edition"):
        if not isinstance(package.get(key), str):
            raise ValueError(f"Package {key} must be a string")
    for section in DEPENDENCY_SECTIONS:
        validate_dependencies(original.get(section, {}))
    targets = original.get("target", {})
    if not isinstance(targets, dict):
        raise ValueError("Target section must be a table")
    for target in targets.values():
        if not isinstance(target, dict) or target.keys() - set(DEPENDENCY_SECTIONS):
            raise ValueError("Unsupported target structure")
        for dependencies in target.values():
            validate_dependencies(dependencies)
    for section in ("features", "profile", "lib"):
        if not isinstance(original.get(section, {}), dict):
            raise ValueError(f"{section} must be a table")
    if any(not isinstance(value, list) or not all(isinstance(item, str) for item in value)
           for value in original.get("features", {}).values()):
        raise ValueError("Features must contain arrays of strings")
    lib = original.get("lib", {})
    if lib.keys() - {"name", "path", "crate-type", "test", "doctest", "bench", "doc", "harness", "edition"}:
        raise ValueError("Unsupported library structure")
    if "path" in lib and (not isinstance(lib["path"], str) or
                          (engine_root / lib["path"]).resolve() != (engine_root / "src/lib.rs").resolve()):
        raise ValueError("Custom library path is unsupported")
    for section in ("bin", "bench", "test", "example"):
        if section in original and (not isinstance(original[section], list) or
                                    not all(isinstance(item, dict) for item in original[section])):
            raise ValueError(f"Unsupported {section} structure")
    mapped = copy.deepcopy(original)
    for section in ("bin", "bench", "test", "example"):
        mapped.pop(section, None)
    mapped["package"].update(build=False, autolib=False, autobins=False,
                              autoexamples=False, autotests=False, autobenches=False)
    mapped["package"].pop("default-run", None)
    mapped.setdefault("lib", {})["path"] = str((engine_root / "src/lib.rs").resolve())
    mapped["bin"] = [{"name": "flamegraph_replay", "path": str(harness.resolve()), "bench": False}]
    return serialize_manifest(mapped)


def external_directory(build_dir, repo_root, engine_root):
    build_dir = build_dir.resolve()
    for protected in (repo_root.resolve(), engine_root.resolve()):
        if build_dir.is_relative_to(protected) or protected.is_relative_to(build_dir):
            raise ValueError("Build directory must be external to the repository and engine, not an ancestor")
    return build_dir


def validate_build_outputs(build_dir):
    paths = [build_dir / name for name in
             (MARKER, "Cargo.toml", "Cargo.lock", "target", "provenance.json", IDENTITY_NAME, ".cargo")]
    target = build_dir / "target"
    if target.is_dir():
        paths.extend(target.rglob("*"))
    for path in paths:
        if path.is_symlink() or (path.exists() and path.resolve() != path):
            raise ValueError(f"Refusing redirected build output: {path}")
        if path.parent == build_dir and path.is_file() and path.stat().st_nlink > 1:
            raise ValueError(f"Refusing hard-linked build output: {path}")
    if (build_dir / ".cargo").exists():
        raise ValueError("Build-directory Cargo configuration is unsupported")


def prepare_build_directory(build_dir, repo_root, engine_root, manifest, features):
    build_dir = external_directory(build_dir, repo_root, engine_root)
    identity = {"format": 1, "repo_root": str(repo_root.resolve()),
                "engine_root": str(engine_root.resolve()), "manifest_sha256": sha256(manifest.encode()),
                "features": features}
    if build_dir.exists() and (not build_dir.is_dir() or any(build_dir.iterdir())):
        marker = build_dir / MARKER
        if marker.is_symlink() or not marker.is_file():
            raise ValueError("Refusing an unowned build directory")
        saved = json.loads(marker.read_text(encoding="utf-8"))
        if not isinstance(saved, dict) or {key: saved.get(key) for key in identity} != identity:
            raise ValueError("Build directory configuration changed; use a new external directory")
        validate_build_outputs(build_dir)
        if (build_dir / "Cargo.toml").read_bytes() != manifest.encode():
            raise ValueError("External manifest was changed")
        if sha256((build_dir / "Cargo.lock").read_bytes()) != saved.get("lock_sha256"):
            raise ValueError("Pinned external lockfile was changed")
        return build_dir
    lock = (engine_root / "Cargo.lock").read_bytes()
    build_dir.mkdir(parents=True, exist_ok=True)
    (build_dir / "Cargo.toml").write_bytes(manifest.encode())
    (build_dir / "Cargo.lock").write_bytes(lock)
    identity["lock_sha256"] = sha256(lock)
    (build_dir / MARKER).write_text(json.dumps(identity, indent=2) + "\n", encoding="utf-8")
    return build_dir


def file_hashes(root):
    return {path.relative_to(root).as_posix(): sha256(path.read_bytes())
            for path in sorted(root.rglob("*")) if path.is_file()}


def source_snapshot(engine_root):
    files = file_hashes(engine_root / "src")
    if "lib.rs" not in files:
        raise ValueError("Engine source must contain src/lib.rs")
    return {"sha256": sha256(json.dumps(files, sort_keys=True, separators=(",", ":")).encode()), "files": files}


def asset_hashes(engine_root):
    assets = {}
    for prefix, directory in (("cards/build", engine_root.parent / "cards/build"),
                              ("engine/baked", engine_root / "baked"),
                              ("web_ui/decks", engine_root.parent / "web_ui/decks")):
        assets.update({f"{prefix}/{name}": digest for name, digest in file_hashes(directory).items()})
    for name in ("cards.json", "abilities.json"):
        path = engine_root.parent / "cards" / name
        if path.is_file():
            assets[f"cards/{name}"] = sha256(path.read_bytes())
    return assets


def output(command, cwd):
    return subprocess.run(command, cwd=cwd, check=True, stdout=subprocess.PIPE, stderr=subprocess.PIPE).stdout


def cargo_messages(stdout):
    messages = []
    for line in (stdout or "").splitlines():
        try:
            message = json.loads(line)
        except json.JSONDecodeError:
            print(line, file=sys.stderr)
            continue
        if not isinstance(message, dict):
            print(line, file=sys.stderr)
            continue
        diagnostic = message.get("message")
        if message.get("reason") == "compiler-message" and isinstance(diagnostic, dict):
            rendered = diagnostic.get("rendered")
            if isinstance(rendered, str):
                print(rendered, end="" if rendered.endswith("\n") else "\n", file=sys.stderr)
        messages.append(message)
    return messages


def git_provenance(root):
    return {
        "head": output(["git", "rev-parse", "HEAD"], root).decode().strip(),
        "dirty": bool(output(["git", "status", "--porcelain", "--untracked-files=all"], root)),
        "dirty_diff_sha256": sha256(output(["git", "diff", "--binary", "--no-ext-diff", "--no-textconv", "HEAD", "--"], root)),
    }


def build(build_dir, engine_root, profiling=False, repo_root=REPO_ROOT):
    repo_root = repo_root.resolve()
    engine_root = engine_root.resolve()
    if (sys.platform.startswith(("msys", "cygwin")) or
            any(name in sysconfig.get_platform().lower() for name in ("mingw", "msys", "cygwin"))):
        raise ValueError("Use native Windows Python (py -3), not MSYS/Cygwin Python, with native Cargo")
    external_directory(build_dir, repo_root, engine_root)
    harness = repo_root / "tools/flamegraph_replay.rs"
    if not harness.is_file():
        raise ValueError(f"Rust harness not found: {harness}")
    engine_manifest_hash = sha256((engine_root / "Cargo.toml").read_bytes())
    manifest = derive_manifest(engine_root, harness)
    if profiling and "profiling" not in tomllib.loads(manifest).get("features", {}):
        raise ValueError("Engine does not define the profiling feature")
    features = ["profiling"] if profiling else []
    build_dir = prepare_build_directory(build_dir, repo_root, engine_root, manifest, features)
    command = ["cargo", "build", "--release", "--offline", "--locked", "--manifest-path",
               str(build_dir / "Cargo.toml"), "--bin", "flamegraph_replay", "--target-dir",
               str(build_dir / "target"), "--message-format=json-render-diagnostics"]
    if features:
        command.extend(["--features", "profiling"])
    before = source_snapshot(engine_root)
    assets = asset_hashes(engine_root)
    harness_hash = sha256(harness.read_bytes())
    lock_hash = sha256((build_dir / "Cargo.lock").read_bytes())
    provenance = {
        "command": command, "cwd": str(engine_root),
        "compiler_version": output(["rustc", "--version", "--verbose"], engine_root).decode().strip(),
        "git": git_provenance(repo_root), "engine_src": before,
        "input_assets": assets, "harness_sha256": harness_hash,
        "cargo_lock_sha256": lock_hash, "features": features,
        "default_features": tomllib.loads(manifest).get("features", {}).get("default", []),
        "manifest_sha256": sha256(manifest.encode()),
        "engine_manifest_sha256": engine_manifest_hash,
        "environment": {name: os.environ.get(name) for name in
                        ("RUSTFLAGS", "CARGO_ENCODED_RUSTFLAGS", "CARGO_BUILD_TARGET", "RUSTUP_TOOLCHAIN")},
        "engine_cargo_config": file_hashes(engine_root / ".cargo"),
    }
    identity = {
        "format": 1, "tool": "flamegraph_replay",
        "assets_sha256": sha256(canonical_json(assets).encode()),
        "cargo_lock_sha256": lock_hash,
        "features": {"default": provenance["default_features"], "requested": features},
        "engine_src_sha256": before["sha256"], "harness_sha256": harness_hash,
        "manifest_sha256": provenance["manifest_sha256"],
    }
    provenance["build_identity"] = identity
    identity_json = canonical_json(identity)
    environment = os.environ.copy()
    environment[IDENTITY_ENV] = identity_json
    provenance_path = build_dir / "provenance.json"
    identity_path = build_dir / IDENTITY_NAME
    validate_build_outputs(build_dir)
    for path in (provenance_path, identity_path):
        if path.exists():
            path.unlink()
    try:
        identity_path.write_text(identity_json + "\n", encoding="utf-8")
        try:
            try:
                result = subprocess.run(command, cwd=engine_root, check=True, stdout=subprocess.PIPE,
                                        text=True, encoding="utf-8", errors="replace", env=environment)
            except subprocess.CalledProcessError as error:
                cargo_messages(error.stdout)
                raise
            messages = cargo_messages(result.stdout)
        finally:
            if source_snapshot(engine_root) != before:
                raise ValueError("Engine source digest changed during build")
            if asset_hashes(engine_root) != assets or sha256(harness.read_bytes()) != harness_hash:
                raise ValueError("Build inputs changed during build")
            if sha256((build_dir / "Cargo.lock").read_bytes()) != lock_hash:
                raise ValueError("Pinned external lockfile changed during build")
            if (sha256((engine_root / "Cargo.toml").read_bytes()) != engine_manifest_hash or
                    (build_dir / "Cargo.toml").read_bytes() != manifest.encode() or
                    file_hashes(engine_root / ".cargo") != provenance["engine_cargo_config"] or
                    identity_path.read_text(encoding="utf-8") != identity_json + "\n"):
                raise ValueError("Build configuration changed during build")
        validate_build_outputs(build_dir)
        executable = None
        for message in messages:
            target = message.get("target")
            if (message.get("reason") == "compiler-artifact" and isinstance(target, dict) and
                    target.get("name") == "flamegraph_replay" and
                    isinstance(message.get("executable"), str) and message["executable"]):
                executable = Path(message["executable"]).resolve()
        if executable is None or not executable.is_file() or not executable.is_relative_to(build_dir / "target"):
            raise ValueError("Cargo did not report a harness executable in the external target directory")
        provenance["executable"] = str(executable)
        provenance["executable_sha256"] = sha256(executable.read_bytes())
        provenance_path.write_text(json.dumps(provenance, indent=2, sort_keys=True) + "\n", encoding="utf-8")
        return executable, provenance_path
    except BaseException:
        for path in (provenance_path, identity_path):
            if path.exists() or path.is_symlink():
                path.unlink()
        raise


def main(argv=None):
    parser = argparse.ArgumentParser()
    commands = parser.add_subparsers(dest="command", required=True)
    build_parser = commands.add_parser("build")
    build_parser.add_argument("--build-dir", required=True, type=Path)
    build_parser.add_argument("--engine-root", type=Path, default=REPO_ROOT / "engine")
    build_parser.add_argument("--profiling", action="store_true")
    args = parser.parse_args(argv)
    try:
        executable, provenance = build(args.build_dir, args.engine_root, args.profiling)
    except (OSError, ValueError, subprocess.CalledProcessError) as error:
        print(f"flamegraph_replay: {error}", file=sys.stderr)
        return 1
    print(executable)
    print(provenance)
    return 0


if __name__ == "__main__":
    sys.exit(main())
