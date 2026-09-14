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
REQUIRED_COMMANDS = (
    "awk", "cat", "chmod", "cp", "grep", "ln", "mkdir", "mktemp", "mv",
    "readlink", "rm", "tar",
)
ARCHIVE_NAME = "superherdr-0.1.0-linux-x86_64.tar.gz"
OWNED_URL_BASE = (
    "https://github.com/jsonMartin/superherdr/releases/download/superherdr-v0.1.0"
)
PREVIOUS_BINARY = b"previous-superherdr\n"


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
        self.payload = b"#!/bin/sh\necho fake-superherdr\n"
        self._write_archive()
        self.expected_sha256 = hashlib.sha256(self.archive.read_bytes()).hexdigest()

        for command in REQUIRED_COMMANDS:
            path = shutil.which(command)
            if path is None:
                self.fail(f"test host is missing required command: {command}")
            (self.bin_dir / command).symlink_to(path)

        self._write_executable(
            "uname",
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
            "curl",
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
                info = tarfile.TarInfo("superherdr")
                info.size = len(self.payload)
                info.mode = 0o755
                tar.addfile(info, io.BytesIO(self.payload))
            license_text = b"fake-license\n"
            info = tarfile.TarInfo("LICENSE")
            info.size = len(license_text)
            tar.addfile(info, io.BytesIO(license_text))
            info = tarfile.TarInfo("licenses/libghostty-vt-LICENSE")
            info.size = len(license_text)
            tar.addfile(info, io.BytesIO(license_text))

    def _write_executable(self, name: str, content: str) -> None:
        path = self.bin_dir / name
        path.write_text(content, encoding="utf-8")
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
            capture_output=True,
            text=True,
            check=False,
        )

    def _install_previous_release(self) -> None:
        self.install_dir.mkdir(parents=True)
        (self.install_dir / "superherdr").write_bytes(PREVIOUS_BINARY)
        (self.install_dir / "superherdr").chmod(0o755)
        (self.install_dir / "herdr").symlink_to("superherdr")

    def _assert_fetch_never_happened(self) -> None:
        self.assertFalse(self.curl_log.exists())

    def test_successful_archive_installation_in_temporary_home(self) -> None:
        result = self._run_installer(self.expected_sha256.upper())

        self.assertEqual(result.returncode, 0, result.stderr)
        installed = self.install_dir / "superherdr"
        self.assertEqual(installed.read_bytes(), self.payload)
        self.assertTrue(os.access(installed, os.X_OK))
        alias = self.install_dir / "herdr"
        self.assertTrue(alias.is_symlink())
        self.assertEqual(os.readlink(alias), "superherdr")
        self.assertTrue((self.license_dir / "LICENSE").is_file())
        self.assertTrue((self.license_dir / "libghostty-vt-LICENSE").is_file())
        # the fixture curl must have talked only to the owned release base,
        # in order: SHA256SUMS first, then the archive
        self.assertEqual(
            self.curl_log.read_text(encoding="utf-8").splitlines(),
            [f"{OWNED_URL_BASE}/SHA256SUMS", f"{OWNED_URL_BASE}/{ARCHIVE_NAME}"],
        )

    def test_update_over_previous_direct_install(self) -> None:
        self._install_previous_release()

        # destination on PATH so the ownership check sees the recognized
        # direct install through command -v as well
        result = self._run_installer(
            self.expected_sha256,
            extra_env={"PATH": f"{self.install_dir}:{self.bin_dir}"},
        )

        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual((self.install_dir / "superherdr").read_bytes(), self.payload)
        self.assertTrue((self.install_dir / "herdr").is_symlink())
        self.assertEqual(os.readlink(self.install_dir / "herdr"), "superherdr")

    def test_upgrade_with_trailing_slash_install_dir_on_path(self) -> None:
        self._install_previous_release()

        result = self._run_installer(
            self.expected_sha256,
            extra_env={
                "SUPERHERDR_INSTALL_DIR": f"{self.install_dir}/",
                "PATH": f"{self.install_dir}:{self.bin_dir}",
            },
        )

        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual((self.install_dir / "superherdr").read_bytes(), self.payload)

    def test_checksum_mismatch_leaves_existing_binary_intact(self) -> None:
        self._install_previous_release()

        result = self._run_installer("0" * 64)

        self.assertNotEqual(result.returncode, 0)
        self.assertIn("checksum did not match", result.stderr)
        self.assertEqual(
            (self.install_dir / "superherdr").read_bytes(), PREVIOUS_BINARY
        )
        self.assertEqual(os.readlink(self.install_dir / "herdr"), "superherdr")

    def test_failed_staged_rename_preserves_previous_binary(self) -> None:
        self._install_previous_release()
        real_mv = os.readlink(self.bin_dir / "mv")
        (self.bin_dir / "mv").unlink()
        self._write_executable(
            "mv",
            f"""#!/bin/sh
for argument in "$@"; do
  if [ "$argument" = "{self.install_dir}/superherdr" ]; then
    echo "injected rename failure" >&2
    exit 1
  fi
done
exec "{real_mv}" "$@"
""",
        )

        result = self._run_installer(self.expected_sha256)

        self.assertNotEqual(result.returncode, 0)
        self.assertIn("injected rename failure", result.stderr)
        self.assertEqual(
            (self.install_dir / "superherdr").read_bytes(), PREVIOUS_BINARY
        )
        self.assertEqual(os.readlink(self.install_dir / "herdr"), "superherdr")
        # the task-owned staging directory is cleaned up even on failure
        self.assertEqual(list(self.install_dir.glob(".superherdr-stage.*")), [])

    def test_corrupt_archive_preserves_previous_binary(self) -> None:
        self._install_previous_release()
        corrupt = self.root / "corrupt.tar.gz"
        corrupt.write_bytes(b"\x1f\x8b not really a gzip stream")
        # publish the corrupt archive's own digest so the run reaches extraction
        corrupt_sha256 = hashlib.sha256(corrupt.read_bytes()).hexdigest()

        result = self._run_installer(
            corrupt_sha256, extra_env={"FAKE_ARCHIVE": str(corrupt)}
        )

        self.assertNotEqual(result.returncode, 0)
        self.assertIn("could not extract", result.stderr)
        self.assertEqual(
            (self.install_dir / "superherdr").read_bytes(), PREVIOUS_BINARY
        )
        self.assertEqual(os.readlink(self.install_dir / "herdr"), "superherdr")

    def test_archive_missing_binary_preserves_previous_binary(self) -> None:
        self._install_previous_release()
        incomplete = self.root / "incomplete.tar.gz"
        self._write_archive(incomplete, include_binary=False)
        incomplete_sha256 = hashlib.sha256(incomplete.read_bytes()).hexdigest()

        result = self._run_installer(
            incomplete_sha256, extra_env={"FAKE_ARCHIVE": str(incomplete)}
        )

        self.assertNotEqual(result.returncode, 0)
        self.assertIn("archive does not contain superherdr", result.stderr)
        self.assertEqual(
            (self.install_dir / "superherdr").read_bytes(), PREVIOUS_BINARY
        )
        self.assertEqual(os.readlink(self.install_dir / "herdr"), "superherdr")

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

    def test_package_managed_symlink_in_install_dir_is_refused(self) -> None:
        self.install_dir.mkdir(parents=True)
        foreign = self.root / "package" / "bin" / "herdr"
        foreign.parent.mkdir(parents=True)
        foreign.write_bytes(b"package-managed\n")
        (self.install_dir / "herdr").symlink_to(foreign)

        result = self._run_installer(self.expected_sha256)

        self.assertNotEqual(result.returncode, 0)
        self.assertIn("refusing to overwrite", result.stderr)
        self.assertEqual(foreign.read_bytes(), b"package-managed\n")
        self.assertEqual(os.readlink(self.install_dir / "herdr"), str(foreign))
        self._assert_fetch_never_happened()

    def test_external_superherdr_alias_is_refused(self) -> None:
        # an alias whose target merely ends in /superherdr is not ours
        self.install_dir.mkdir(parents=True)
        target = self.root / "package" / "bin" / "superherdr"
        target.parent.mkdir(parents=True)
        target.write_bytes(b"package-managed-superherdr\n")
        (self.install_dir / "herdr").symlink_to(target)

        result = self._run_installer(self.expected_sha256)

        self.assertNotEqual(result.returncode, 0)
        self.assertIn("refusing to overwrite", result.stderr)
        self.assertEqual(target.read_bytes(), b"package-managed-superherdr\n")
        self.assertEqual(os.readlink(self.install_dir / "herdr"), str(target))
        self._assert_fetch_never_happened()

    def test_dangling_external_superherdr_alias_is_refused(self) -> None:
        self.install_dir.mkdir(parents=True)
        target = self.root / "package" / "bin" / "superherdr"
        target.parent.mkdir(parents=True)
        (self.install_dir / "herdr").symlink_to(target)
        self.assertFalse(target.exists())

        result = self._run_installer(self.expected_sha256)

        self.assertNotEqual(result.returncode, 0)
        self.assertIn("refusing to overwrite", result.stderr)
        self.assertEqual(os.readlink(self.install_dir / "herdr"), str(target))
        self._assert_fetch_never_happened()

    def test_unknown_regular_binary_without_owned_alias_is_refused(self) -> None:
        self.install_dir.mkdir(parents=True)
        (self.install_dir / "superherdr").write_bytes(b"mystery-superherdr\n")
        (self.install_dir / "superherdr").chmod(0o755)

        result = self._run_installer(self.expected_sha256)

        self.assertNotEqual(result.returncode, 0)
        self.assertIn("this installer did not create it", result.stderr)
        self.assertEqual(
            (self.install_dir / "superherdr").read_bytes(), b"mystery-superherdr\n"
        )
        self.assertFalse((self.install_dir / "herdr").exists())
        self._assert_fetch_never_happened()

    def test_conflicting_install_outside_install_dir_is_refused(self) -> None:
        (self.bin_dir / "superherdr").write_bytes(b"#!/bin/sh\necho homebrew-superherdr\n")
        (self.bin_dir / "superherdr").chmod(0o755)

        result = self._run_installer(self.expected_sha256)

        self.assertNotEqual(result.returncode, 0)
        self.assertIn("never takes over another installation", result.stderr)
        self._assert_fetch_never_happened()


if __name__ == "__main__":
    unittest.main()
