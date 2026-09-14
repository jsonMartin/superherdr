# Superherdr 0.1.0 release-candidate notes

Superherdr 0.1.0 is the first Superherdr release candidate. It is not yet published. Superherdr is a project built on [Herdr](https://github.com/herdrdev/herdr), with its own release versioning.

## What 0.1.0 adds

- **Focus is client-local.** Each client picks the project or workspace to show; other clients keep their own Focus.
- **Snooze is shared.** A snoozed project or workspace stays hidden on every client until its wake time, without stopping its terminals. Snooze never stops processes, mutes notices, or acknowledges requests; runtime behavior is unaffected.
- **All / Top-level Agents toggle.** Top-level hides agents in linked worktree workspaces when the same endpoint has a non-linked parent with the same worktree key. Parent, standalone, and orphan agents stay visible. Notices and runtime behavior are unchanged.

## Artifacts, platform, and installation

- The initial artifact is a macOS Apple Silicon build only. It declares a 13.0 macOS minimum matching its dependency, but it has actually been exercised only on macOS 27; older-OS validation is pending.
- Binary self-update is disabled. The existing local Homebrew `superherdr` 0.9.0_1 remains the active installation while the 0.1.0 candidate is validated separately.
- A matching Superherdr remote installation for SSH needs a separately prepared build for the remote platform; upstream installers install Herdr, not Superherdr.
- Existing plugins retain the Herdr 0.9.0 compatibility level; Superherdr release numbers are separate.
- `HERDR_*` environment variables, socket names, and protocol identifiers retain their inherited names for compatibility.

The owned release repository is [jsonmartin/superherdr](https://github.com/jsonmartin/superherdr), and the shared Homebrew formula is staged in [jsonmartin/homebrew-tap](https://github.com/jsonmartin/homebrew-tap). The release uses `superherdr-v0.1.0` so inherited Herdr tags remain unchanged. Draft assets are not anonymously downloadable; the tap installation command must be verified after publication.
