from __future__ import annotations

import os
import subprocess
import tempfile
import unittest
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[1]
SCRIPT = REPO_ROOT / "scripts" / "sync_upstream.sh"


def git(cwd: Path, *args: str) -> str:
    return subprocess.run(
        ["git", *args], cwd=cwd, check=True, capture_output=True, text=True
    ).stdout.strip()


def write(root: Path, rel: str, text: str) -> None:
    path = root / rel
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(text, encoding="utf-8")


class SyncUpstreamTests(unittest.TestCase):
    """Simulate an upstream Herdr release against a small fork."""

    def setUp(self) -> None:
        self.temp = tempfile.TemporaryDirectory(prefix="superherdr-sync-test-")
        root = Path(self.temp.name)
        self.upstream = root / "upstream"
        self.fork = root / "fork"
        self.upstream.mkdir()
        git(self.upstream, "init", "-q", "-b", "master")
        self._identity(self.upstream)
        write(self.upstream, "SPONSORS.md", "sponsors v1\n")
        write(self.upstream, "README.md", "upstream readme v1\n")
        write(self.upstream, "src/lib.rs", "fn a() {}\n")
        write(self.upstream, "src/shared.rs", "line one\n")
        git(self.upstream, "add", "-A")
        git(self.upstream, "commit", "-q", "-m", "base")
        git(self.upstream, "tag", "v1.0.0")

        git(root, "clone", "-q", str(self.upstream), str(self.fork))
        self._identity(self.fork)
        git(self.fork, "rm", "-q", "SPONSORS.md")
        write(self.fork, "README.md", "superherdr readme\n")
        write(self.fork, "src/shared.rs", "fork line\n")
        write(self.fork, ".upstream-sync/exclude", "# comment\nSPONSORS.md\ndocs/versions/\n")
        write(self.fork, ".upstream-sync/ours", "README.md\n")
        write(self.fork, "scripts/sync_upstream.sh", SCRIPT.read_text(encoding="utf-8"))
        (self.fork / "scripts/sync_upstream.sh").chmod(0o755)
        git(self.fork, "add", "-A")
        git(self.fork, "commit", "-q", "-m", "fork")
        git(self.fork, "tag", "-d", "v1.0.0")

    def tearDown(self) -> None:
        self.temp.cleanup()

    def _identity(self, repo: Path) -> None:
        git(repo, "config", "user.name", "Test")
        git(repo, "config", "user.email", "test@example.com")
        git(repo, "config", "commit.gpgsign", "false")

    def _release(self, tag: str, files: dict[str, str]) -> None:
        for rel, text in files.items():
            write(self.upstream, rel, text)
        git(self.upstream, "add", "-A")
        git(self.upstream, "commit", "-q", "-m", tag)
        git(self.upstream, "tag", tag)

    def _sync(self, *args: str) -> subprocess.CompletedProcess[str]:
        env = {**os.environ, "SUPERHERDR_UPSTREAM_URL": str(self.upstream)}
        return subprocess.run(
            ["scripts/sync_upstream.sh", *args],
            cwd=self.fork,
            env=env,
            capture_output=True,
            text=True,
            check=False,
        )

    def test_clean_release_drops_exclusions_and_keeps_owned_files(self) -> None:
        self._release(
            "v1.1.0",
            {
                "SPONSORS.md": "sponsors v2\n",
                "docs/versions/1.1.0/index.md": "new snapshot\n",
                "README.md": "upstream readme v2\n",
                "src/lib.rs": "fn a() {}\nfn b() {}\n",
            },
        )

        result = self._sync("v1.1.0")

        self.assertEqual(result.returncode, 0, result.stderr + result.stdout)
        tracked = git(self.fork, "ls-files").splitlines()
        self.assertNotIn("SPONSORS.md", tracked)
        self.assertNotIn("docs/versions/1.1.0/index.md", tracked)
        self.assertEqual((self.fork / "README.md").read_text(), "superherdr readme\n")
        self.assertIn("fn b()", (self.fork / "src/lib.rs").read_text())
        self.assertIn("README.md", result.stdout)
        self.assertEqual(git(self.fork, "log", "-1", "--format=%s"), "chore(upstream): merge herdr v1.1.0")
        # upstream tags never land in refs/tags
        self.assertEqual(git(self.fork, "tag", "--list"), "")
        self.assertEqual(self._sync("v1.1.0").stdout.strip(), "Herdr v1.1.0 is already merged")

    def test_genuine_conflict_stops_for_manual_resolution(self) -> None:
        self._release("v1.2.0", {"src/shared.rs": "upstream line\n", "SPONSORS.md": "v3\n"})

        result = self._sync("v1.2.0", "--report", "report.md")

        self.assertEqual(result.returncode, 2, result.stderr + result.stdout)
        self.assertIn("src/shared.rs", (self.fork / "report.md").read_text())
        self.assertTrue((self.fork / ".git" / "MERGE_HEAD").exists())
        self.assertNotIn("SPONSORS.md", git(self.fork, "ls-files").splitlines())

    def test_untracked_owned_path_is_skipped_not_reported_as_kept(self) -> None:
        write(self.fork, ".upstream-sync/ours", "README.md\nMISSING.md\n")
        git(self.fork, "commit", "-q", "-am", "own a missing path")
        self._release("v1.3.0", {"MISSING.md": "upstream file\n"})

        result = self._sync("v1.3.0")

        self.assertEqual(result.returncode, 0, result.stderr + result.stdout)
        self.assertIn("not tracked, skipping: MISSING.md", result.stderr)
        self.assertIn("### New upstream files", result.stdout)
        self.assertNotIn("port by hand)\n\n- `MISSING.md`", result.stdout)

    def test_check_rejects_reintroduced_exclusion(self) -> None:
        self.assertEqual(self._sync("--check").returncode, 0)
        write(self.fork, "SPONSORS.md", "back again\n")
        git(self.fork, "add", "SPONSORS.md")

        result = self._sync("--check")

        self.assertEqual(result.returncode, 1)
        self.assertIn("SPONSORS.md", result.stderr)

    def test_latest_picks_highest_semver_release(self) -> None:
        for tag in ("v1.9.0", "v1.10.0", "v2.0.0-rc.1"):
            git(self.upstream, "tag", tag)

        self.assertEqual(self._sync("--latest").stdout.strip(), "v1.10.0")


if __name__ == "__main__":
    unittest.main()
