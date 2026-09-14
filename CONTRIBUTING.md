# Contributing to Superherdr

Superherdr is a fork of [Herdr](https://github.com/herdrdev/herdr), a terminal runtime for coding agents. Bug fixes, performance improvements, tests, and documentation are welcome. Read [AGENTS.md](AGENTS.md) before making changes; it describes the architecture rules every change must follow.

**Open a proposal before implementing a new feature or material change.** Straightforward fixes and improvements that preserve intended behavior can go directly to a pull request.

## Superherdr or Herdr?

Most of Superherdr's runtime comes from Herdr, and upstream releases are merged regularly. If a bug also exists in Herdr and is not caused by Superherdr's changes, consider reporting it [upstream](https://github.com/herdrdev/herdr) so both projects benefit. Superherdr-specific behavior (Focus, Snooze, the Top-level Agents view, installation, and compatibility rules) belongs here. See the [behavior baseline](openspec/README.md).

## Choose the contribution path

Search existing issues, pull requests, and [specifications](openspec/specs/) first. Link related work instead of opening duplicates.

| Change | Where to start | Evidence to include |
| --- | --- | --- |
| Documentation, tests, or a refactor that preserves behavior and architecture | A direct PR is welcome. | Explain the change and run the relevant checks. |
| Bug fix restoring intended behavior | A bug issue or direct PR; an issue is optional. | Reproduction, expected and actual behavior, and a focused regression test. |
| Performance improvement preserving behavior | A direct PR is welcome. | Reproducible before/after measurements, environment, and correctness checks. |
| New feature or material change | A proposal-only PR before implementation; keep it a draft while shaping the proposal. | Proposed behavior, alternatives, impact, and OpenSpec scenarios. |
| Unclear or disputed expected behavior | Clarify first; an issue is recommended. | The conflicting expectations and the relevant spec or examples. |

Material changes include changes to CLI commands or output, the socket API, endpoint or wire protocols, configuration keys or defaults, keybindings, saved state formats, installation and update behavior, supported platforms, network access, major dependencies, or architecture. A performance change that trades away correctness or responsiveness also needs a proposal. Impact decides the path; calling a change a fix does not exempt a redesign.

For example, fixing a snoozed workspace that fails to reappear at its wake time is a direct PR. Adding Snooze for individual panes, or changing what Snooze hides by default, needs a proposal.

## Propose before implementing

The OpenSpec proposal PR is the request for comments: reviewers discuss the proposed files, and agreed changes are made in those files before acceptance.

1. Create the change and write its artifacts (see [Record the change in OpenSpec](#record-the-change-in-openspec)). The PR contains only files under `openspec/changes/<change-name>/`.
2. Open the PR as a draft with an `RFC:` title while shaping the proposal. Mark it ready for review when you want a decision. No implementation code is needed.
3. A maintainer accepts the proposal by merging the proposal-only PR, or by commenting "Proposal accepted; continue implementation in this PR." A reaction, passing check, draft status, or silence is not acceptance.
4. Implement in a separate PR that links the accepted proposal. A maintainer may request changes, defer, or decline a proposal and will record the reason.

Acceptance approves the direction; it does not guarantee the implementation will merge. If feasibility needs code, ask for a bounded, disposable spike first; an approved spike is not approval to implement. If the accepted scope changes materially, update the proposal and get acceptance again before continuing.

An issue can help establish a problem before someone writes a proposal, but it is never required and does not replace review of the proposal files.

Write `proposal.md` and its supporting files with:

- **Problem and users:** what someone cannot do today, and why it belongs in Superherdr rather than upstream Herdr.
- **Proposed behavior:** the before/after experience, scope, and non-goals, with SHALL/MUST requirements and WHEN/THEN scenarios for each added or modified capability.
- **Alternatives and drawbacks:** including keeping the current behavior.
- **Impact:** affected UI, CLI, socket API, configuration, persistence, compatibility with Herdr clients and plugins, and how the change affects upstream merges.
- **Verification and open questions:** how success will be demonstrated, and the assumptions that could invalidate the approach.

### One PR for maintainer-led work

For the maintainer's own work, one PR is the normal path. Other contributors may use it when a maintainer explicitly agrees.

1. Put the proposal, delta specs, and tasks in the first commit, and open the PR as a draft.
2. Get explicit acceptance before writing implementation code. For maintainer-led work with an agent, the maintainer's explicit approval in the working session counts; record who approved, the accepted revision, and the scope in the PR description.
3. Add implementation and focused verification in later commits, and retitle the PR with a Conventional Commits title.
4. Archive the change and validate the baseline in the same PR before requesting final review.

## Keep specifications lightweight

Use OpenSpec's Lite style by default: short observable requirements, clear scope and non-goals, and a few testable scenarios for the main workflows. Add edge cases only when leaving them out would hide a user promise, a known failure, or a data, privacy, or correctness risk. Keep implementation details in code or design notes.

Use Full detail for interface and contract work: the socket API, CLI output that tools parse, endpoint and wire protocols, persisted formats, configuration schema, and compatibility guarantees. Full specs name every field or command they change, compatibility with existing Herdr and Superherdr clients, and failure behavior.

When an edge case appears, add a regression test if an existing requirement already covers it, or add a scenario if the intended outcome is missing. Specs record product promises; tests record specific regressions. The baseline describes implemented behavior, and documentation backfills may update it directly when behavior is unchanged.

## Record the change in OpenSpec

The baseline lives in `openspec/specs/`, and proposed changes live in `openspec/changes/<change-name>/` until implementation is complete. From the repository root:

```sh
bunx --bun @fission-ai/openspec@1.13.0 new change add-pane-snooze
```

This creates the change folder. Add `proposal.md`, delta specs under `specs/<capability>/spec.md`, and `tasks.md`; add `design.md` when technical decisions need explanation. Delta specs use `ADDED`, `MODIFIED`, or `REMOVED Requirements` sections with `#### Scenario:` headings. A modified requirement includes its full updated text and retained scenarios.

Validate the proposal:

```sh
bunx --bun @fission-ai/openspec@1.13.0 validate add-pane-snooze --strict
```

Before merging the implementation PR, mark tasks complete, archive the change, and validate the baseline:

```sh
bunx --bun @fission-ai/openspec@1.13.0 archive add-pane-snooze
just spec-check
```

Commit the updated baseline and archived change in the PR that completes the implementation, including when the proposal merged separately. Archiving is never a follow-up PR, and a proposal-only PR is never archived.

Direct fixes need no change folder. If a fix restores behavior the spec already describes, add a regression test. If the spec is missing or wrong about intended behavior, update it with the fix; changing intended behavior still needs a proposal.

## Submit a pull request

Describe the problem, resulting behavior, verification performed, and limitations, and link the proposal and its acceptance when one was required. Screenshots or a short recording help reviewers with UI changes; performance claims need reproducible measurements. Keep changes focused.

Use [Conventional Commits](https://www.conventionalcommits.org/) for commit subjects and PR titles, for example `fix(snooze): wake workspaces after restore`. CI checks PR titles. Do not add AI co-author trailers.

## Verification

Install the pinned Rust toolchain, Zig 0.15.2, [just](https://github.com/casey/just), [cargo-nextest](https://nexte.st/), and [Bun](https://bun.sh/). Then run from the repository root:

```sh
just test                 # unit, integration, and repository tests
just lint                 # rustfmt and clippy
just spec-check           # OpenSpec validation
just test-one <filter>    # one test filter, for a focused change
```

CI runs these checks on Linux and macOS. Windows and Nix files are inherited from Herdr and are not supported targets.

When testing a build from inside an existing Superherdr or Herdr session, clear inherited socket overrides so the debug build talks to its own `superherdr-dev` server:

```sh
env -u HERDR_SOCKET_PATH -u HERDR_CLIENT_SOCKET_PATH cargo run -- <command>
```

Agents must never stop, restart, or replace a running production server during testing.

## Contributor checklist

Use this checklist with the [pull request template](.github/pull_request_template.md). Mark an item not applicable with a reason rather than claiming work that was not done.

- [ ] Classify the change: direct fix or improvement, proposal only, single-PR proposal implementation, or implementation of a separately accepted proposal.
- [ ] For a new feature or material change, write and validate the proposal before implementation, and record its acceptance.
- [ ] Get renewed acceptance before implementing material changes to the accepted scope.
- [ ] Report focused verification and limitations: reproduction and regression evidence for bugs, measurements for performance claims, and visual evidence for UI changes.
- [ ] Update affected documentation and specs, or explain why none are needed.
- [ ] In the PR that completes a proposal, finish its tasks, archive the change, and run `just spec-check`.

## License

By contributing, you agree that your contributions are licensed under the [Apache License 2.0](LICENSE), the license of this project.
