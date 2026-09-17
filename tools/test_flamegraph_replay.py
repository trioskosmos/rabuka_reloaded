import contextlib
import io
import json
import os
from pathlib import Path
import subprocess
import tempfile
import tomllib
import unittest
from unittest.mock import patch

import flamegraph_replay as replay


MANIFEST = '''
[package]
name = "fixture_engine"
version = "0.2.3"
edition = "2021"
build = "build.rs"
default-run = "old"
[lib]
doctest = false
[dependencies]
plain = "=1.2.3"
optional = { version = "2", optional = true, default-features = false, features = ["alloc"] }
[dev-dependencies]
test_helper = "3"
[build-dependencies]
generator = "4"
[features]
default = ["dep:optional"]
profiling = []
[target.'cfg(windows)'.dependencies]
win = { version = "5", features = ["a"] }
[target.'cfg(unix)'.build-dependencies]
other_generator = "1"
[profile.release]
debug = "line-tables-only"
lto = "thin"
[profile.dev.build-override]
opt-level = 1
[profile.release.package."plain"]
opt-level = 3
[[bin]]
name = "old"
path = "src/main.rs"
[[bench]]
name = "performance"
harness = false
[[example]]
name = "example"
[[test]]
name = "test"
'''


class ReplayTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name).resolve()
        self.repo = self.root / "repo"
        self.engine = self.repo / "engine"
        self.build_dir = self.root / "external"
        (self.engine / "src").mkdir(parents=True)
        (self.repo / "tools").mkdir()
        (self.engine / "src/lib.rs").write_text("pub fn fixture() {}", encoding="utf-8")
        (self.engine / "build.rs").write_text("fn main() { panic!(); }", encoding="utf-8")
        (self.engine / "Cargo.toml").write_text(MANIFEST, encoding="utf-8")
        (self.engine / "Cargo.lock").write_bytes(b"version = 3\n")
        self.harness = self.repo / "tools/flamegraph_replay.rs"
        self.harness.write_text("fn main() {}", encoding="utf-8")
        for name in ("cards/build/cards.bin", "engine/baked/decks/fixture.bin",
                     "web_ui/decks/fixture.txt", "cards/abilities.json"):
            path = self.repo / name
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(name.encode())

    def manifest(self):
        return replay.derive_manifest(self.engine, self.harness)

    def prepare(self, features=None):
        return replay.prepare_build_directory(self.build_dir, self.repo, self.engine,
                                              self.manifest(), features or [])

    def fake_run(self, command, **kwargs):
        if command[:2] == ["rustc", "--version"]:
            return subprocess.CompletedProcess(command, 0, b"rustc fixture\nhost: fixture-host\n")
        if command[:3] == ["git", "rev-parse", "HEAD"]:
            return subprocess.CompletedProcess(command, 0, b"fixture-head\n")
        if command[:2] == ["git", "status"]:
            return subprocess.CompletedProcess(command, 0, b" M fixture\n")
        if command[:2] == ["git", "diff"]:
            return subprocess.CompletedProcess(command, 0, b"fixture-diff")
        self.assertEqual(command[:2], ["cargo", "build"])
        self.assertEqual(kwargs["cwd"], self.engine)
        identity = json.loads((self.build_dir / replay.IDENTITY_NAME).read_text(encoding="utf-8"))
        self.assertEqual(json.loads(kwargs["env"][replay.IDENTITY_ENV]), identity)
        self.assertEqual(kwargs["env"].get("RUSTFLAGS"), os.environ.get("RUSTFLAGS"))
        self.assertTrue(kwargs["check"])
        executable = self.build_dir / "target/fixture-host/release/flamegraph_replay.exe"
        executable.parent.mkdir(parents=True, exist_ok=True)
        executable.write_bytes(b"fixture-executable")
        message = {"reason": "compiler-artifact", "target": {"name": "flamegraph_replay"},
                   "executable": str(executable)}
        return subprocess.CompletedProcess(command, 0, json.dumps(message) + "\n")

    def run_build(self, profiling=False, side_effect=None):
        with patch.object(replay.subprocess, "run", side_effect=side_effect or self.fake_run) as run:
            result = replay.build(self.build_dir, self.engine, profiling, self.repo)
        return result, run

    def test_manifest_mapping_preserves_resolution_and_profiles(self):
        original = tomllib.loads(MANIFEST)
        mapped = tomllib.loads(self.manifest())
        for key in ("dependencies", "dev-dependencies", "build-dependencies", "features", "target", "profile"):
            self.assertEqual(mapped[key], original[key])
        for key in ("name", "version", "edition"):
            self.assertEqual(mapped["package"][key], original["package"][key])
        self.assertEqual(mapped["lib"]["path"], str(self.engine / "src/lib.rs"))
        self.assertFalse(mapped["lib"]["doctest"])
        self.assertNotIn("default-run", mapped["package"])
        self.assertEqual(mapped["bin"], [{"name": "flamegraph_replay", "path": str(self.harness),
                                         "bench": False}])

    def test_build_script_and_target_discovery_disabled(self):
        mapped = tomllib.loads(self.manifest())
        for key in ("build", "autolib", "autobins", "autoexamples", "autotests", "autobenches"):
            self.assertIs(mapped["package"][key], False)
        for key in ("bench", "test", "example"):
            self.assertNotIn(key, mapped)
        self.assertNotIn("fixture_engine", mapped["dependencies"])
        self.assertEqual((self.engine / "build.rs").read_text(), "fn main() { panic!(); }")

    def test_actual_engine_manifest(self):
        root = replay.REPO_ROOT / "engine"
        original = tomllib.loads((root / "Cargo.toml").read_text(encoding="utf-8"))
        mapped = tomllib.loads(replay.derive_manifest(root, replay.REPO_ROOT / "tools/flamegraph_replay.rs"))
        for key in ("dependencies", "dev-dependencies", "features", "profile"):
            self.assertEqual(mapped[key], original[key])
        self.assertFalse(mapped["package"]["build"])
        self.assertEqual(len(mapped["bin"]), 1)

    def test_serializer_preserves_quoted_keys_unicode_and_nested_values(self):
        data = {"target": {'cfg(target_os = "windows")': {"dependencies": {
            "quoted.key": {"version": "1", "features": ["a", "\U0001f600", "\x7f"]}}}},
                "path": 'C:\\space dir\\quote"\n', "values": [True, False, 1, 1.5, {}]}
        self.assertEqual(tomllib.loads(replay.serialize_manifest(data)), data)
        with self.assertRaises(ValueError):
            replay.serialize_manifest({"unsupported": object()})

    def test_relative_dependencies_rejected_in_every_section(self):
        for header in ("dependencies", "dev-dependencies", "build-dependencies",
                       "target.'cfg(windows)'.dependencies", "target.'cfg(windows)'.dev-dependencies",
                       "target.'cfg(unix)'.build-dependencies"):
            with self.subTest(header=header):
                text = MANIFEST.replace(f"[{header}]", f'[{header}]\nrelative = {{ path = "../other" }}')
                if text == MANIFEST:
                    text += f'\n[{header}]\nrelative = {{ path = "../other" }}\n'
                (self.engine / "Cargo.toml").write_text(text, encoding="utf-8")
                with self.assertRaisesRegex(ValueError, "Relative path"):
                    self.manifest()

    def test_absolute_dependency_is_preserved(self):
        original = tomllib.loads(MANIFEST)
        original["dependencies"]["absolute"] = {"path": str(self.root / "dependency")}
        (self.engine / "Cargo.toml").write_text(replay.serialize_manifest(original), encoding="utf-8")
        self.assertEqual(tomllib.loads(self.manifest())["dependencies"], original["dependencies"])

    def test_unsupported_structures_fail_closed(self):
        changes = [
            lambda data: data.update(workspace={}),
            lambda data: data.update(patch={"crates-io": {}}),
            lambda data: data["package"].update(version={"workspace": True}),
            lambda data: data["package"].update(workspace=".."),
            lambda data: data["dependencies"].update(inherited={"workspace": True}),
            lambda data: data["target"].update(unknown={"rustflags": ["flag"]}),
            lambda data: data["lib"].update(path="other.rs"),
            lambda data: data["features"].update(bad="not-an-array"),
            lambda data: data.update(bin={"name": "not-an-array"}),
        ]
        for index, change in enumerate(changes):
            with self.subTest(index=index):
                data = tomllib.loads(MANIFEST)
                change(data)
                (self.engine / "Cargo.toml").write_text(replay.serialize_manifest(data), encoding="utf-8")
                with self.assertRaises(ValueError):
                    self.manifest()

    def test_lock_is_copied_once_and_retained(self):
        self.build_dir.mkdir()
        self.prepare()
        manifest = (self.build_dir / "Cargo.toml").read_bytes()
        stamp = (self.build_dir / "Cargo.lock").stat().st_mtime_ns
        (self.engine / "Cargo.lock").write_bytes(b"changed upstream lock")
        self.prepare()
        self.assertEqual((self.build_dir / "Cargo.lock").read_bytes(), b"version = 3\n")
        self.assertEqual((self.build_dir / "Cargo.lock").stat().st_mtime_ns, stamp)
        self.assertEqual((self.build_dir / "Cargo.toml").read_bytes(), manifest)

    def test_unknown_build_directory_is_not_clobbered(self):
        self.build_dir.mkdir()
        sentinel = self.build_dir / "Cargo.toml"
        sentinel.write_bytes(b"unknown manifest")
        with self.assertRaisesRegex(ValueError, "unowned"):
            self.prepare()
        self.assertEqual(sentinel.read_bytes(), b"unknown manifest")
        self.assertFalse((self.build_dir / replay.MARKER).exists())

    def test_changed_configuration_is_rejected(self):
        self.prepare()
        with self.assertRaisesRegex(ValueError, "configuration changed"):
            self.prepare(["profiling"])
        (self.engine / "Cargo.toml").write_text(MANIFEST.replace('plain = "=1.2.3"', 'plain = "=1.2.4"'))
        with self.assertRaisesRegex(ValueError, "configuration changed"):
            self.prepare()

    def test_tampered_manifest_and_lock_are_rejected(self):
        self.prepare()
        for filename in ("Cargo.toml", "Cargo.lock"):
            with self.subTest(filename=filename):
                path = self.build_dir / filename
                before = path.read_bytes()
                path.write_bytes(b"tampered")
                with self.assertRaises(ValueError):
                    self.prepare()
                self.assertEqual(path.read_bytes(), b"tampered")
                path.write_bytes(before)

    def test_repo_engine_and_ancestor_output_directories_are_rejected(self):
        for directory in (self.repo, self.engine, self.repo / "output", self.engine / "target", self.root):
            with self.subTest(directory=directory), self.assertRaisesRegex(ValueError, "external"):
                replay.prepare_build_directory(directory, self.repo, self.engine, self.manifest(), [])
        other_engine = self.root / "other-engine"
        with self.assertRaises(ValueError):
            replay.external_directory(other_engine / "output", self.repo, other_engine)

    def test_symlink_into_repo_is_rejected(self):
        link = self.root / "linked-repo"
        try:
            link.symlink_to(self.repo, target_is_directory=True)
        except OSError as error:
            self.skipTest(f"Symlink creation unavailable: {error}")
        if not link.is_symlink():
            self.skipTest("Filesystem does not provide native symlinks")
        with self.assertRaises(ValueError):
            replay.external_directory(link / "output", self.repo, self.engine)

    def test_source_digest_includes_paths_and_contents(self):
        before = replay.source_snapshot(self.engine)
        path = self.engine / "src/extra.rs"
        path.write_bytes(b"first")
        added = replay.source_snapshot(self.engine)
        path.rename(self.engine / "src/renamed.rs")
        renamed = replay.source_snapshot(self.engine)
        (self.engine / "src/renamed.rs").write_bytes(b"second")
        changed = replay.source_snapshot(self.engine)
        self.assertEqual(len({item["sha256"] for item in (before, added, renamed, changed)}), 4)

    def test_build_command_provenance_and_inherited_environment(self):
        with patch.dict(os.environ, {"RUSTFLAGS": "caller flags"}):
            (executable, provenance), run = self.run_build()
        data = json.loads(provenance.read_text())
        command = data["command"]
        self.assertEqual(command[:5], ["cargo", "build", "--release", "--offline", "--locked"])
        self.assertNotIn("--features", command)
        self.assertEqual(command[command.index("--manifest-path") + 1], str(self.build_dir / "Cargo.toml"))
        self.assertEqual(command[command.index("--target-dir") + 1], str(self.build_dir / "target"))
        self.assertEqual(data["engine_src"], replay.source_snapshot(self.engine))
        self.assertEqual(data["git"]["head"], "fixture-head")
        self.assertTrue(data["git"]["dirty"])
        self.assertEqual(data["git"]["dirty_diff_sha256"], replay.sha256(b"fixture-diff"))
        self.assertIn("rustc fixture", data["compiler_version"])
        self.assertEqual(data["environment"]["RUSTFLAGS"], "caller flags")
        self.assertEqual(data["features"], [])
        self.assertEqual(data["cargo_lock_sha256"], replay.sha256(b"version = 3\n"))
        self.assertEqual(data["manifest_sha256"], replay.sha256((self.build_dir / "Cargo.toml").read_bytes()))
        self.assertEqual(data["executable"], str(executable))
        self.assertIn("cards/build/cards.bin", data["input_assets"])
        self.assertIn("engine/baked/decks/fixture.bin", data["input_assets"])
        self.assertIn("web_ui/decks/fixture.txt", data["input_assets"])
        self.assertEqual(run.call_args.kwargs["cwd"], self.engine)

    def test_msys_python_rejected_before_writing(self):
        with patch.object(replay.sys, "platform", "msys"):
            with self.assertRaisesRegex(ValueError, "native Windows Python"):
                replay.build(self.build_dir, self.engine, repo_root=self.repo)
        self.assertFalse(self.build_dir.exists())

    def test_profiling_is_explicit(self):
        (_, provenance), _ = self.run_build(profiling=True)
        data = json.loads(provenance.read_text())
        self.assertEqual(data["command"][-2:], ["--features", "profiling"])
        self.assertEqual(data["features"], ["profiling"])

    def test_changed_source_after_success_or_failure_rejects_provenance(self):
        for failure in (False, True):
            with self.subTest(failure=failure):
                def mutate(command, **kwargs):
                    if command[:2] == ["cargo", "build"]:
                        with (self.engine / "src/lib.rs").open("a") as source:
                            source.write("\nchanged")
                        if failure:
                            raise subprocess.CalledProcessError(1, command)
                    return self.fake_run(command, **kwargs)

                with self.assertRaisesRegex(ValueError, "source digest changed"):
                    self.run_build(side_effect=mutate)
                self.assertFalse((self.build_dir / "provenance.json").exists())

    def test_failed_rebuild_removes_old_provenance(self):
        self.run_build()

        def fail(command, **kwargs):
            if command[:2] == ["cargo", "build"]:
                raise subprocess.CalledProcessError(1, command)
            return self.fake_run(command, **kwargs)

        with self.assertRaises(subprocess.CalledProcessError):
            self.run_build(side_effect=fail)
        self.assertFalse((self.build_dir / "provenance.json").exists())

    def test_asset_change_rejects_provenance(self):
        def mutate(command, **kwargs):
            if command[:2] == ["cargo", "build"]:
                (self.repo / "cards/build/cards.bin").write_bytes(b"changed")
            return self.fake_run(command, **kwargs)

        with self.assertRaisesRegex(ValueError, "inputs changed"):
            self.run_build(side_effect=mutate)
        self.assertFalse((self.build_dir / "provenance.json").exists())

    def test_build_identity_binds_inputs_and_overrides_caller_value(self):
        with patch.dict(os.environ, {replay.IDENTITY_ENV: "untrusted"}):
            (_, provenance), run = self.run_build(profiling=True)
            self.assertEqual(os.environ[replay.IDENTITY_ENV], "untrusted")
        data = json.loads(provenance.read_text())
        identity = json.loads((self.build_dir / replay.IDENTITY_NAME).read_text())
        self.assertEqual(identity, {
            "format": 1, "tool": "flamegraph_replay",
            "assets_sha256": replay.sha256(replay.canonical_json(data["input_assets"]).encode()),
            "cargo_lock_sha256": data["cargo_lock_sha256"],
            "features": {"default": ["dep:optional"], "requested": ["profiling"]},
            "engine_src_sha256": data["engine_src"]["sha256"],
            "harness_sha256": data["harness_sha256"], "manifest_sha256": data["manifest_sha256"],
        })
        self.assertEqual(data["build_identity"], identity)
        self.assertEqual(run.call_args.kwargs["env"][replay.IDENTITY_ENV], replay.canonical_json(identity))
        self.assertNotIn("projection", identity)

    def test_mingw_python_rejected_but_native_python_in_msys_shell_allowed(self):
        with patch.object(replay.sys, "platform", "win32"):
            with patch.object(replay.sysconfig, "get_platform", return_value="mingw_x86_64"):
                with self.assertRaisesRegex(ValueError, "native Windows Python"):
                    self.run_build()
            self.assertFalse(self.build_dir.exists())
            with patch.object(replay.sysconfig, "get_platform", return_value="win-amd64"):
                with patch.dict(os.environ, {"MSYSTEM": "MINGW64"}):
                    self.run_build()

    def test_redirected_identity_and_nested_target_output_are_rejected(self):
        self.prepare()
        outside = self.root / "sentinel"
        outside.write_bytes(b"unchanged")
        identity_path = self.build_dir / replay.IDENTITY_NAME
        try:
            os.link(outside, identity_path)
        except OSError as error:
            self.skipTest(f"Hard links unavailable: {error}")
        with self.assertRaisesRegex(ValueError, "hard-linked"):
            self.run_build()
        self.assertEqual(outside.read_bytes(), b"unchanged")
        identity_path.unlink()
        nested = self.build_dir / "target/release"
        nested.mkdir(parents=True)
        with patch.object(Path, "is_symlink", autospec=True, side_effect=lambda path: path == nested):
            with self.assertRaisesRegex(ValueError, "redirected"):
                self.run_build()

    def test_compiler_diagnostics_are_preserved_on_success_and_failure(self):
        diagnostic = json.dumps({"reason": "compiler-message", "message": {"rendered": "warning: fixture\n"}})
        for failure in (False, True):
            with self.subTest(failure=failure):
                def report(command, **kwargs):
                    result = self.fake_run(command, **kwargs)
                    if command[:2] == ["cargo", "build"]:
                        result.stdout = diagnostic + "\nraw diagnostic\n[]\n" + result.stdout
                        if failure:
                            raise subprocess.CalledProcessError(1, command, output=result.stdout)
                    return result

                stderr = io.StringIO()
                with contextlib.redirect_stderr(stderr):
                    if failure:
                        with self.assertRaises(subprocess.CalledProcessError):
                            self.run_build(side_effect=report)
                        self.assertFalse((self.build_dir / replay.IDENTITY_NAME).exists())
                        self.assertFalse((self.build_dir / "provenance.json").exists())
                    else:
                        self.run_build(side_effect=report)
                self.assertIn("warning: fixture\n", stderr.getvalue())
                self.assertIn("raw diagnostic", stderr.getvalue())

    def test_nested_source_and_identity_mutations_reject_provenance(self):
        for filename in ("src/internal/non_rust.data", replay.IDENTITY_NAME):
            with self.subTest(filename=filename):
                def mutate(command, **kwargs):
                    result = self.fake_run(command, **kwargs)
                    if command[:2] == ["cargo", "build"]:
                        path = (self.engine if filename.startswith("src/") else self.build_dir) / filename
                        path.parent.mkdir(parents=True, exist_ok=True)
                        path.write_bytes(b"changed")
                    return result

                with self.assertRaises(ValueError):
                    self.run_build(side_effect=mutate)
                self.assertFalse((self.build_dir / replay.IDENTITY_NAME).exists())
                self.assertFalse((self.build_dir / "provenance.json").exists())

    def test_cli_only_build_and_required_external_directory(self):
        for args in ([], ["build"], ["run"]):
            with self.subTest(args=args), contextlib.redirect_stderr(io.StringIO()), self.assertRaises(SystemExit):
                replay.main(args)
        with patch.object(replay, "build", return_value=(Path("executable"), Path("provenance"))) as build:
            stdout = io.StringIO()
            with contextlib.redirect_stdout(stdout):
                self.assertEqual(replay.main(["build", "--build-dir", str(self.build_dir)]), 0)
            build.assert_called_once_with(self.build_dir, replay.REPO_ROOT / "engine", False)
            self.assertEqual(stdout.getvalue(), "executable\nprovenance\n")


if __name__ == "__main__":
    unittest.main()
