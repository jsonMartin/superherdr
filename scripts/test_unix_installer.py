from __future__ import annotations

import hashlib
import io
import os
import shutil
import subprocess
import tarfile
import tempfile
import unittest
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[1]
INSTALLER = REPO_ROOT / "distribution" / "install.sh"
# GNU tar runs gzip as a child process for .tar.gz archives.
REQUIRED_COMMANDS = (
    "awk", "cat", "chmod", "cp", "grep", "gzip", "ln", "mkdir", "mktemp", "mv",
    "readlink", "rm", "tar",
)
VERSION = "0.9.1.2"
ARCHIVE_NAME = f"superherdr-{VERSION}-linux-x86_64.tar.gz"
OWNED_URL_BASE = (
    f"https://github.com/jsonMartin/superherdr/releases/download/superherdr-v{VERSION}"
)
NEW_BINARY = b'#!/bin/sh\necho "herdr 0.9.1 (superherdr 0.9.1.2)"\n'
PREVIOUS_SUPERHERDR = b'#!/bin/sh\necho "herdr 0.9.0 (superherdr 0.9.0.0)"\n'
LEGACY_BINARY = b'#!/bin/sh\necho "superherdr 0.1.0"\n'
UPSTREAM_HERDR = b'#!/bin/sh\necho "herdr 0.9.0"\n'


@unittest.skipUnless(os.name == "posix", "Unix installer requires a POSIX host")
class UnixInstallerTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temp_dir = tempfile.TemporaryDirectory(prefix="superherdr-installer-test-")
        self.root = Path(self.temp_dir.name)
        self.bin_dir = self.root / "bin"
        self.bin_dir.mkdir()
        self.install_dir = self.root / "install"
        self.xdg_data_dir = self.root / "xdg-data"
        self.license_dir = self.xdg_data_dir / "superherdr" / "licenses"
        self.curl_log = self.root / "curl-log"
        self.archive = self.root / ARCHIVE_NAME
        self._write_archive()
        self.expected_sha256 = hashlib.sha256(self.archive.read_bytes()).hexdigest()

        for command in REQUIRED_COMMANDS:
            path = shutil.which(command)
            if path is None:
                self.fail(f"test host is missing required command: {command}")
            (self.bin_dir / command).symlink_to(path)

        self._write_executable(
            self.bin_dir / "uname",
            """#!/bin/sh
case "$1" in
  -s) echo "${FAKE_UNAME_S:-Linux}" ;;
  -m) echo "${FAKE_UNAME_M:-x86_64}" ;;
  -o) echo "${FAKE_UNAME_O:-GNU/Linux}" ;;
  *) exit 1 ;;
esac
""",
        )
        self._write_executable(
            self.bin_dir / "curl",
            """#!/bin/sh
url=""
out=""
previous=""
for argument in "$@"; do
  case "$argument" in
    https://github.com/jsonMartin/superherdr/releases/download/*) url="$argument" ;;
  esac
  if [ "$previous" = "-o" ]; then
    out="$argument"
  fi
  previous="$argument"
done
if [ -z "$url" ]; then
  echo "fixture curl refused non-owned URL: $*" >> "$FAKE_CURL_LOG"
  exit 64
fi
printf '%s\\n' "$url" >> "$FAKE_CURL_LOG"
if [ -n "$out" ]; then
  cp "$FAKE_ARCHIVE" "$out"
else
  cat "$FAKE_SUMS"
fi
""",
        )
        # the installer accepts sha256sum, shasum, or openssl; use whichever
        # the host provides so macOS hosts without coreutils still run
        sha_tool = (
            shutil.which("sha256sum") or shutil.which("shasum") or shutil.which("openssl")
        )
        if sha_tool is None:
            self.fail("test host is missing sha256sum, shasum, and openssl")
        (self.bin_dir / Path(sha_tool).name).symlink_to(sha_tool)

    def tearDown(self) -> None:
        self.temp_dir.cleanup()

    def _write_archive(
        self,
        path: Path | None = None,
        *,
        include_binary: bool = True,
    ) -> None:
        with tarfile.open(path or self.archive, "w:gz") as tar:
            if include_binary:
                info = tarfile.TarInfo("herdr")
                info.size = len(NEW_BINARY)
                info.mode = 0o755
                tar.addfile(info, io.BytesIO(NEW_BINARY))
            license_text = b"fake-license\n"
            info = tarfile.TarInfo("LICENSE")
            info.size = len(license_text)
            tar.addfile(info, io.BytesIO(license_text))
            info = tarfile.TarInfo("licenses/libghostty-vt-LICENSE")
            info.size = len(license_text)
            tar.addfile(info, io.BytesIO(license_text))

    def _write_executable(self, path: Path, content: str | bytes) -> None:
        path.parent.mkdir(parents=True, exist_ok=True)
        if isinstance(content, str):
            path.write_text(content, encoding="utf-8")
        else:
            path.write_bytes(content)
        path.chmod(0o755)

    def _write_sums(self, checksum: str) -> Path:
        path = self.root / "SHA256SUMS"
        path.write_text(f"{checksum}  {ARCHIVE_NAME}\n", encoding="utf-8")
        return path

    def _run_installer(
        self,
        checksum: str,
        extra_env: dict[str, str] | None = None,
    ) -> subprocess.CompletedProcess[str]:
        sums = self._write_sums(checksum)
        env = {
            **os.environ,
            "PATH": str(self.bin_dir),
            "HOME": str(self.root / "home"),
            "FAKE_ARCHIVE": str(self.archive),
            "FAKE_SUMS": str(sums),
            "FAKE_CURL_LOG": str(self.curl_log),
            "XDG_DATA_HOME": str(self.xdg_data_dir),
            "SUPERHERDR_INSTALL_DIR": str(self.install_dir),
            **(extra_env or {}),
        }
        return subprocess.run(
            ["/bin/sh", str(INSTALLER)],
            env=env,
            # empty PATH entries scan the working directory
            cwd=self.root,
            capture_output=True,
            text=True,
            check=False,
        )

    def _install_previous_superherdr(self) -> None:
        self._write_executable(self.install_dir / "herdr", PREVIOUS_SUPERHERDR)

    def _install_legacy_pair(self) -> None:
        self._write_executable(self.install_dir / "superherdr", LEGACY_BINARY)
        (self.install_dir / "herdr").symlink_to("superherdr")

    def _assert_fetch_never_happened(self) -> None:
        self.assertFalse(self.curl_log.exists())

    def _assert_only_new_herdr_installed(self) -> None:
        herdr = self.install_dir / "herdr"
        self.assertFalse(herdr.is_symlink())
        self.assertEqual(herdr.read_bytes(), NEW_BINARY)
        self.assertTrue(os.access(herdr, os.X_OK))
        self.assertFalse((self.install_dir / "superherdr").exists())
        self.assertEqual(list(self.install_dir.glob(".superherdr-stage.*")), [])

    def test_fresh_install_creates_only_herdr(self) -> None:
        result = self._run_installer(self.expected_sha256.upper())

        self.assertEqual(result.returncode, 0, result.stderr)
        self._assert_only_new_herdr_installed()
        self.assertTrue((self.license_dir / "LICENSE").is_file())
        self.assertTrue((self.license_dir / "libghostty-vt-LICENSE").is_file())
        # the fixture curl must have talked only to the owned release base,
        # in order: SHA256SUMS first, then the archive
        self.assertEqual(
            self.curl_log.read_text(encoding="utf-8").splitlines(),
            [f"{OWNED_URL_BASE}/SHA256SUMS", f"{OWNED_URL_BASE}/{ARCHIVE_NAME}"],
        )

    def test_update_over_owned_herdr(self) -> None:
        self._install_previous_superherdr()

        # destination on PATH: the PATH scan must skip the install directory
        result = self._run_installer(
            self.expected_sha256,
            extra_env={"PATH": f"{self.install_dir}:{self.bin_dir}"},
        )

        self.assertEqual(result.returncode, 0, result.stderr)
        self._assert_only_new_herdr_installed()

    def test_upgrade_with_trailing_slash_install_dir_on_path(self) -> None:
        self._install_previous_superherdr()

        result = self._run_installer(
            self.expected_sha256,
            extra_env={
                "SUPERHERDR_INSTALL_DIR": f"{self.install_dir}/",
                "PATH": f"{self.install_dir}/:{self.bin_dir}",
            },
        )

        self.assertEqual(result.returncode, 0, result.stderr)
        self._assert_only_new_herdr_installed()

    def test_upgrade_from_legacy_pair_removes_superherdr(self) -> None:
        self._install_legacy_pair()

        result = self._run_installer(
            self.expected_sha256,
            extra_env={"PATH": f"{self.install_dir}:{self.bin_dir}"},
        )

        self.assertEqual(result.returncode, 0, result.stderr)
        self._assert_only_new_herdr_installed()

    def test_resumes_interrupted_legacy_upgrade(self) -> None:
        # herdr was already replaced, but the 0.1.0 superherdr was not removed
        self._install_previous_superherdr()
        self._write_executable(self.install_dir / "superherdr", LEGACY_BINARY)

        result = self._run_installer(self.expected_sha256)

        self.assertEqual(result.returncode, 0, result.stderr)
        self._assert_only_new_herdr_installed()

    def _snapshot_install_dir(self) -> dict[str, bytes | str]:
        return {
            p.name: os.readlink(p) if p.is_symlink() else p.read_bytes()
            for p in self.install_dir.iterdir()
        }

    def _assert_failure_preserves_both_layouts(
        self, expected_error: str, checksum: str, extra_env: dict[str, str] | None = None
    ) -> None:
        for install_previous in (self._install_previous_superherdr, self._install_legacy_pair):
            with self.subTest(layout=install_previous.__name__):
                shutil.rmtree(self.install_dir, ignore_errors=True)
                install_previous()
                before = self._snapshot_install_dir()

                result = self._run_installer(checksum, extra_env)

                self.assertNotEqual(result.returncode, 0)
                self.assertIn(expected_error, result.stderr)
                self.assertEqual(self._snapshot_install_dir(), before)

    def test_checksum_mismatch_preserves_both_previous_layouts(self) -> None:
        self._assert_failure_preserves_both_layouts("checksum did not match", "0" * 64)

    def test_failed_staged_rename_preserves_both_previous_layouts(self) -> None:
        real_mv = os.readlink(self.bin_dir / "mv")
        (self.bin_dir / "mv").unlink()
        self._write_executable(
            self.bin_dir / "mv",
            f"""#!/bin/sh
for argument in "$@"; do
  if [ "$argument" = "{self.install_dir}/herdr" ]; then
    echo "injected rename failure" >&2
    exit 1
  fi
done
exec "{real_mv}" "$@"
""",
        )

        # the snapshot also proves the staging directory was cleaned up
        self._assert_failure_preserves_both_layouts("injected rename failure", self.expected_sha256)

    def test_corrupt_archive_preserves_both_previous_layouts(self) -> None:
        corrupt = self.root / "corrupt.tar.gz"
        corrupt.write_bytes(b"\x1f\x8b not really a gzip stream")
        # publish the corrupt archive's own digest so the run reaches extraction
        self._assert_failure_preserves_both_layouts(
            "could not extract",
            hashlib.sha256(corrupt.read_bytes()).hexdigest(),
            {"FAKE_ARCHIVE": str(corrupt)},
        )

    def test_archive_missing_binary_preserves_both_previous_layouts(self) -> None:
        incomplete = self.root / "incomplete.tar.gz"
        self._write_archive(incomplete, include_binary=False)
        self._assert_failure_preserves_both_layouts(
            "archive does not contain herdr",
            hashlib.sha256(incomplete.read_bytes()).hexdigest(),
            {"FAKE_ARCHIVE": str(incomplete)},
        )

    def test_update_through_symlinked_install_dir_on_path(self) -> None:
        self._install_previous_superherdr()
        alias = self.root / "alias-bin"
        alias.symlink_to(self.install_dir)

        result = self._run_installer(
            self.expected_sha256,
            extra_env={"PATH": f"{alias}::{self.bin_dir}"},
        )

        self.assertEqual(result.returncode, 0, result.stderr)
        self._assert_only_new_herdr_installed()

    def test_herdr_in_current_directory_via_trailing_path_colon_is_refused(self) -> None:
        self._write_executable(self.root / "herdr", UPSTREAM_HERDR)

        result = self._run_installer(
            self.expected_sha256, extra_env={"PATH": f"{self.bin_dir}:"}
        )

        self.assertNotEqual(result.returncode, 0)
        self.assertIn("never takes over another installation", result.stderr)
        self._assert_fetch_never_happened()

    def test_foreign_superherdr_beside_owned_herdr_is_refused(self) -> None:
        self._install_previous_superherdr()
        self._write_executable(self.install_dir / "superherdr", UPSTREAM_HERDR)

        result = self._run_installer(self.expected_sha256)

        self.assertNotEqual(result.returncode, 0)
        self.assertIn("this installer did not create it", result.stderr)
        self.assertEqual((self.install_dir / "superherdr").read_bytes(), UPSTREAM_HERDR)
        self._assert_fetch_never_happened()

    def test_unsupported_target_is_refused_before_fetching(self) -> None:
        result = self._run_installer(
            self.expected_sha256,
            extra_env={"FAKE_UNAME_M": "aarch64"},
        )

        self.assertNotEqual(result.returncode, 0)
        self.assertIn("no Superherdr release build exists for Linux aarch64", result.stderr)
        self._assert_fetch_never_happened()

    def test_termux_is_refused_before_fetching(self) -> None:
        result = self._run_installer(
            self.expected_sha256,
            extra_env={"FAKE_UNAME_O": "Android"},
        )

        self.assertNotEqual(result.returncode, 0)
        self.assertIn("Android/Termux is not supported", result.stderr)
        self._assert_fetch_never_happened()

    def test_upstream_herdr_in_install_dir_is_refused(self) -> None:
        self._write_executable(self.install_dir / "herdr", UPSTREAM_HERDR)

        result = self._run_installer(self.expected_sha256)

        self.assertNotEqual(result.returncode, 0)
        self.assertIn("is not Superherdr", result.stderr)
        self.assertEqual((self.install_dir / "herdr").read_bytes(), UPSTREAM_HERDR)
        self._assert_fetch_never_happened()

    def test_non_executable_herdr_is_refused(self) -> None:
        self.install_dir.mkdir(parents=True)
        (self.install_dir / "herdr").write_bytes(NEW_BINARY)
        (self.install_dir / "herdr").chmod(0o644)

        result = self._run_installer(self.expected_sha256)

        self.assertNotEqual(result.returncode, 0)
        self.assertIn("not an executable file", result.stderr)
        self._assert_fetch_never_happened()

    def test_package_managed_symlink_in_install_dir_is_refused(self) -> None:
        foreign = self.root / "package" / "bin" / "herdr"
        self._write_executable(foreign, UPSTREAM_HERDR)
        self.install_dir.mkdir(parents=True)
        (self.install_dir / "herdr").symlink_to(foreign)

        result = self._run_installer(self.expected_sha256)

        self.assertNotEqual(result.returncode, 0)
        self.assertIn("refusing to overwrite", result.stderr)
        self.assertEqual(os.readlink(self.install_dir / "herdr"), str(foreign))
        self._assert_fetch_never_happened()

    def test_external_superherdr_alias_is_refused(self) -> None:
        # a herdr symlink whose target merely ends in /superherdr is not ours
        target = self.root / "package" / "bin" / "superherdr"
        self._write_executable(target, LEGACY_BINARY)
        self.install_dir.mkdir(parents=True)
        (self.install_dir / "herdr").symlink_to(target)

        result = self._run_installer(self.expected_sha256)

        self.assertNotEqual(result.returncode, 0)
        self.assertIn("refusing to overwrite", result.stderr)
        self.assertEqual(os.readlink(self.install_dir / "herdr"), str(target))
        self._assert_fetch_never_happened()

    def test_dangling_external_superherdr_alias_is_refused(self) -> None:
        target = self.root / "package" / "bin" / "superherdr"
        self.install_dir.mkdir(parents=True)
        (self.install_dir / "herdr").symlink_to(target)
        self.assertFalse(target.exists())

        result = self._run_installer(self.expected_sha256)

        self.assertNotEqual(result.returncode, 0)
        self.assertIn("refusing to overwrite", result.stderr)
        self._assert_fetch_never_happened()

    def test_unpaired_superherdr_is_refused(self) -> None:
        self._write_executable(self.install_dir / "superherdr", LEGACY_BINARY)

        result = self._run_installer(self.expected_sha256)

        self.assertNotEqual(result.returncode, 0)
        self.assertIn("this installer did not create it", result.stderr)
        self.assertEqual((self.install_dir / "superherdr").read_bytes(), LEGACY_BINARY)
        self.assertFalse((self.install_dir / "herdr").exists())
        self._assert_fetch_never_happened()

    def test_conflicting_install_elsewhere_on_path_is_refused(self) -> None:
        self._write_executable(self.bin_dir / "herdr", UPSTREAM_HERDR)

        result = self._run_installer(self.expected_sha256)

        self.assertNotEqual(result.returncode, 0)
        self.assertIn("never takes over another installation", result.stderr)
        self._assert_fetch_never_happened()

    def test_herdr_shadowed_later_on_path_is_refused(self) -> None:
        self._install_previous_superherdr()
        other = self.root / "usr-local-bin"
        self._write_executable(other / "herdr", UPSTREAM_HERDR)

        result = self._run_installer(
            self.expected_sha256,
            extra_env={"PATH": f"{self.install_dir}:{self.bin_dir}:{other}"},
        )

        self.assertNotEqual(result.returncode, 0)
        self.assertIn(str(other / "herdr"), result.stderr)
        self.assertEqual((self.install_dir / "herdr").read_bytes(), PREVIOUS_SUPERHERDR)
        self._assert_fetch_never_happened()


if __name__ == "__main__":
    unittest.main()
