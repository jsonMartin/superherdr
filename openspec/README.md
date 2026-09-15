# Superherdr behavior baseline

These specifications describe the behavior Superherdr adds to or changes from Herdr. They are deliberately concise. Inherited Herdr behavior is described by [Herdr's documentation](https://herdr.dev/docs/) until a Superherdr change specifies it. New behavior belongs in an accepted change proposal under `changes/` until it is implemented and archived.

| Capability | Baseline | Evidence to consult |
| --- | --- | --- |
| Client-local Focus | [Focus](specs/superherdr-focus/spec.md) | `src/client/shell`; client Focus tests |
| Shared, persisted Snooze | [Snooze](specs/superherdr-snooze/spec.md) | Snooze server, API and persistence modules; `cargo nextest run snooze` |
| All / Top-level Agents view | [Agents view](specs/superherdr-agents-view/spec.md) | `src/client/shell`; top-level agent filter tests |
| Release artifacts, shell installer and Homebrew | [Distribution](specs/superherdr-distribution/spec.md) | `distribution/install.sh`; `scripts/test_unix_installer.py`; `jsonmartin/homebrew-tap` |
| Identity, shared Herdr state and upstream compatibility | [Compatibility](specs/superherdr-compatibility/spec.md) | `tests/drop_in_herdr.rs`; `src/protocol/wire.rs`; endpoint fixtures in `tests/fixtures/` |

Evidence pointers show where to check behavior; they do not claim those checks were run when this index was written.

Validate the baseline and active changes with `just spec-check`.
