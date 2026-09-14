# Superherdr

Superherdr is a terminal runtime for coding agents, forked from [Herdr](https://github.com/herdrdev/herdr). It adds Focus, shared Snooze, and the Top-level Agents view, and ships its own binaries, shell installer, and Homebrew formula. Build and run `superherdr`. Keep `HERDR_*` integration names and frozen protocol identifiers compatible with Herdr.

These instructions apply to every person and agent working in this repository. Read [CONTRIBUTING.md](CONTRIBUTING.md) for the contribution process, and the [behavior baseline](openspec/README.md) for Superherdr-specific behavior.

## Specifications first

Superherdr uses OpenSpec. New features and material behavior, interface, or architecture changes need an accepted proposal under `openspec/changes/` before implementation; see CONTRIBUTING.md for the paths, acceptance rules, and the single-PR path for maintainer-led work. Use Lite specs by default and Full specs for the socket API, CLI output that tools parse, endpoint and wire protocols, persisted formats, configuration schema, and compatibility guarantees. Archive a change in the PR that completes its implementation, and run `just spec-check`.

Do not reopen accepted scope while implementing. A contradiction, invalid assumption, or missing requirement in an accepted proposal is a specification gap: stop that work, report the evidence, and update the proposal for renewed acceptance.

## Principles

- **State is separated from runtime.** `AppState` is pure data, testable without PTYs or async. `PaneState` is separate from `PaneRuntime`. Workspace logic doesn't need real terminals.
- **Render is pure.** `compute_view()` handles geometry and mutations. `render()` takes `&AppState` and only draws. Never mutate state during render.
- **No god objects.** If a module is doing too many things, split it. `app/` is already split into state, actions, and input. Keep it that way.
- **Platform code is isolated.** OS-specific behavior lives in the matching `src/platform/<os>.rs` file, with only shared traits, types, wrappers, and testable contracts in `src/platform/mod.rs`. Core modules don't have `#[cfg(target_os)]`.
- **Detection is decoupled.** The detector reads a screen snapshot, never touches the parser or viewport state.
- **UI patterns should be reused.** Superherdr is a mouse-first TUI. New dialogs, settings, and flows should follow the existing UI language and interaction patterns instead of inventing one-off screens. Reuse existing modal and screen structure, affordances, and close actions.

## Multiplicative performance paths

Treat work reachable from view computation, rendering, background-pane resizing, PTY parsing, detection, and client frame fanout as multiplicative. Before adding work, identify its frequency and cardinality: per byte, event, or render × panes, tabs, or workspaces × attached clients.

Inside pane-scaled render and layout loops:

- Use narrow terminal-state accessors. Do not collect aggregate input state, format terminal snapshots, inspect process trees, perform filesystem I/O, or allocate when one scalar fact is enough.
- Keep terminal-core lock duration minimal.
- Preserve hidden-source and retained-render early exits. Hidden panes still parse output, but their output must not trigger presentation work merely to keep terminal or detection state current.
- When a change adds or widens work in one of these loops, profile fixed geometry with 1 and at least 15 populated panes and report the scaling delta. Use `just bench-render-scale` to exercise both background-workspace and active-pane cardinality when applicable.

Prefer deterministic operation or architecture tests to wall-clock CI limits. Benchmarks such as `just bench-release-smoke` are supporting evidence, not substitutes for behavioral coverage.

## Runtime and client boundary

The server owns the runtime protocol; the TUI is one client. New work should not deepen server/TUI coupling.

Before adding state, API fields, events, commands, or socket messages, classify the feature:

- Shared runtime or session fact: belongs in server state and should be exposed through the JSON API and event path when practical. Example: Snooze.
- TUI presentation state: belongs only in the client layer. Examples: Focus and the Top-level Agents view.

Do not add shared behavior that only works through the private TUI client socket. Use neutral server and API names, not UI-surface names like sidebar, row, card, or widget. Workspace, tab, and pane remain shared session organization, but avoid making them mandatory identity for unrelated runtime features.

## Stable client endpoint contract

The client endpoint generation is independent from the private same-install protocol. Generation 1 is the compatibility floor for Local, SSH, and Cloud connections, shared with Herdr, and must remain available unless retired for a security reason.

- Named core codecs are immutable. Do not add, remove, reorder, or reinterpret fields or enum variants reachable from a published codec. Introduce a new codec name and keep the old codec as a fallback instead.
- Keep baseline JSON handshake and snapshot fields required. New JSON fields must be optional or have field-specific defaults; new enum values need an `Unknown` fallback where older clients can safely ignore them.
- Add server features through advertised API methods and optional snapshot data when possible. A missing optional feature must disable only that action, not reject the connection. Herdr clients and servers must keep connecting.
- Do not change the meaning or load-bearing parameter shape of an advertised endpoint method. If an old server could ignore a new field and incorrectly report success, add a new method name or a separately advertised capability.
- Missing methods, rejections, timeouts, and unavailable servers are client-local outcomes. They must not disconnect other compatible servers, and typing in a pane must not dismiss their notices.
- Frozen endpoint fixtures, bincode digests, wire-tag tests, and `tests/fixtures/endpoint-method-shapes-v1.json` are compatibility contracts. Never update a generation-1 expectation to bless a wire change; create and negotiate a new codec or method.
- Existing-value digests cannot detect an appended enum variant. Review every enum reachable from a frozen codec as append-closed even when tests pass.

## Upstream Herdr

Superherdr merges upstream Herdr releases with `just sync-upstream <tag>`, and a daily workflow opens a sync pull request when Herdr publishes a new release tag. The script applies `.upstream-sync/exclude` (paths removed from the fork on purpose, which must not come back) and `.upstream-sync/ours` (files Superherdr rewrote, whose upstream changes are shown for manual porting). CI runs `scripts/sync_upstream.sh --check` to reject reintroduced exclusions. Resolve genuine conflicts case by case. Merge sync pull requests with a merge commit, never squash or rebase, so the upstream history stays an ancestor and later syncs do not re-conflict. Never push upstream tags to this repository.

Every upstream release is expected to conflict in `Cargo.toml` and `Cargo.lock`, because both sides change the package name and version. Keep Superherdr's `name` and `version`, take upstream's dependency changes, then run `cargo update -p superherdr --offline` to regenerate the lock entry.

- Prefer keeping inherited code, even when Superherdr does not use it, so upstream merges stay simple. Guard it rather than deleting it.
- Keep Superherdr-specific changes in clearly separated modules or small, well-named edits to inherited files.
- Superherdr uses Herdr's agent-detection catalog at `herdr.dev` while its detection engine and rules match Herdr's. A change that makes them diverge must move the catalog to Superherdr-hosted infrastructure in the same change (see the compatibility spec).
- Inherited files under `distribution/` other than `install.sh` and `agent-detection/` are kept only because inherited update code embeds or tests them. Superherdr does not publish them.

## Testing

Use `just` recipes instead of invoking cargo or scripts directly.

```bash
just test               # cargo nextest + repository tests
just check              # lint + tests + OpenSpec validation
just test-one <filter>  # a single nextest filter
```

Run `just check` before opening a pull request, or explain exactly why a narrower check is enough. Do not bypass failing checks, weaken tests, or change configuration to hide a failure.

Unit tests live next to the code (`#[cfg(test)] mod tests`). New `AppState` or `Workspace` behavior should be testable with `AppState::test_new()` and `Workspace::test_new()` without PTYs.

For broad refactors or release-risk regressions, classify the risk before editing. Treat changes as refactor-risk when they touch two or more core surfaces, persisted state, protocol or API IDs, workspace/tab/pane identity, restore or handoff, agent detection authority, or UI/input state projection. Before moving code, identify the protected behavior and add or name characterization tests. Identity and state refactors should use `AppState::assert_invariants_for_test()` or `Workspace::assert_invariants_for_test()` with adversarial state from `AppState::test_with_adversarial_identity_state()` or `Workspace::test_adversarial_identity_state()`.

When testing a build from inside an existing Superherdr or Herdr session, clear inherited socket overrides so the debug binary talks to its own `superherdr-dev` server instead of a production server:

```bash
env -u HERDR_SOCKET_PATH -u HERDR_CLIENT_SOCKET_PATH cargo run -- <command>
```

Never stop, restart, or replace a running production server or its state while testing.

## Agent detection

Detection rules are evidence-based. When changing `src/detect/manifests/`, first capture the relevant bottom-buffer state with `superherdr agent read <pane> --source detection --format text`, and `--format ansi` when styling or the alternate screen matters. Decide which visible controls are invariant and which are alternatives, and encode them as explicit AND/OR gates. Do not match whole-pane incidental text, and do not use the user-visible viewport for agent status, because users can scroll it.

Inspect matching with `superherdr agent explain <pane> --json`, test a rule through a local override in `~/.config/superherdr/agent-detection/<agent>.toml`, and apply it with `superherdr server reload-agent-manifests`. Never overwrite or remove an existing override without the owner's agreement, and remove the temporary override afterwards so the bundled manifest stays the source of truth.

Detection fixes that apply equally to Herdr belong upstream. Keep `distribution/agent-detection/` aligned with the bundled manifests; `scripts/agent_detection_manifest_check.py` enforces this. Do not add large agent-specific full-screen fixture suites for routine tuning; keep Rust tests focused on manifest parsing, rule semantics, source precedence, reload, and update behavior.

## Vendored libghostty-vt

`vendor/libghostty-vt.vendor.json` records the vendored upstream source commit.

Local patches on top of the vendored source must be tracked in `vendor/libghostty-vt.patches.md` and stored under `vendor/patches/libghostty-vt/`. Each entry states why the patch exists, the related issue, upstream PR or discussion, vendored base commit, touched files, verification, and the exact removal condition.

When updating libghostty-vt, check every active patch. If the new upstream commit contains the fix, remove the patch and its index entry and rerun the listed verification; otherwise reapply the patch. `just test` verifies that patch files are indexed and reverse-apply cleanly.

## Documentation

Superherdr-specific user documentation lives in `README.md` and `docs/next/`. Inherited behavior is documented by [Herdr's documentation](https://herdr.dev/docs/); do not copy it here. Update documentation in the same pull request as a user-facing change. `skills/superherdr/SKILL.md` describes the CLI for agents; keep it accurate when commands, IDs, or agent lifecycle semantics change.

Add user-facing entries to `CHANGELOG.md` under an Unreleased heading when a change is user-visible. Do not add entries for documentation-only, CI, or repository-maintenance changes.

Put local planning notes under `.local/`, which is ignored.

## Commits and releases

Use lowercase [Conventional Commits](https://www.conventionalcommits.org/) with a scope where useful, no emojis, and no AI co-author trailers. CI validates commit subjects and PR titles. Agents commit or push only when asked.

Releases are published as `superherdr-v<version>` on `jsonMartin/superherdr` with archives for `macos-aarch64` and `linux-x86_64`, `install.sh`, and `SHA256SUMS`, and the Homebrew formula in `jsonmartin/homebrew-tap` is updated to match. Only the maintainer publishes releases.

## Code conventions

- Rust: no `unwrap()` in production code. Use `tracing` for logging. Use `#[allow]` only with a comment explaining why.
- Platform-specific code must be compile-gated. Put OS APIs and substantial OS behavior in `src/platform/`; elsewhere use `#[cfg(windows)]`, `#[cfg(unix)]`, or target-specific `#[cfg(...)]` on imports, fields, functions, impls, and match arms. Use `cfg!(...)` only for pure cross-platform policy constants whose branches compile on every target.
- Don't add dependencies without a reason. Check whether existing dependencies cover the need first.
- Integration asset versions (`HERDR_INTEGRATION_VERSION` markers and matching `*_INTEGRATION_VERSION` constants) are migration versions relative to the latest released tag, not per-commit counters. Bump once per release when an asset changes.
- When changing the server/client wire protocol incompatibly, bump `src/protocol/wire.rs::PROTOCOL_VERSION` if the current source protocol has already been published in a Superherdr release, and only once before the next release. Update hardcoded protocol expectations and fixtures in tests.
