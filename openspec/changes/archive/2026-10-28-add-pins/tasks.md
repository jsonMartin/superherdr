# Tasks

## 1. Server

- [x] 1.1 Add `PinState` and the `workspace.pin`, `pane.pin` and `pin.subscribe` methods to `src/api/schema.rs` and `src/api/schema/`, name them in `src/api/server.rs`, and list them in `OPTIONAL_CLIENT_SHELL_METHODS` in `src/server/client_commands.rs`. Verify: schema and method-name tests pass; `tests/fixtures/endpoint-method-shapes-v1.json` is unchanged.
- [x] 1.2 Add `src/server/pin_store.rs`: load, atomic save, bounds, malformed-file handling, mapping by workspace `lifetime_id` and public pane number. Verify: store unit tests pass, including restore id remapping.
- [x] 1.3 Wire pins into the headless server (`src/server/headless.rs`, `src/server/headless/endpoint_requests.rs`, `src/server/client_transport.rs`): handle the methods, checkpoint the session before every pin save, remap pins when a pane moves between workspaces (before pruning a removed source workspace), prune on workspace and pane close, broadcast `pin.state` to subscribers. Verify: headless tests for pin, unpin, idempotent pin, not-found, prune, pane move (source kept and source removed) then restore, split-pin-restore without waiting for the save debounce, and broadcast pass.

## 2. Client

- [x] 2.1 Decode `pin.state` in `src/client/endpoint/control.rs`, subscribe when `pin.subscribe` is advertised in `src/client/mod.rs`, and store per-endpoint pin state in the shell. Apply every `PinState` through one setter that rejects other boots and older revisions. Verify: decode, malformed-state and stale-revision tests pass.
- [x] 2.2 Apply pinned-first block ordering in the shared builders `sidebar::workspace_entries_with_filter` and `agent_sidebar::ordered_agent_pane_ids_with_filter`, and pinned-first agent ordering before every return of `aggregate_navigation::aggregate_agent_rows` (including the agent-view projection branch and custom sorts). Update every caller, and route the paths that build order independently through the same entries: collapsed endpoint rendering and its reveal offsets (`endpoint_sidebar.rs` `render_collapsed`, `collapsed_groups_for_endpoint`), collapsed workspace previews (`workspace_navigation.rs`), next/previous endpoint navigation (`endpoint_navigation.rs`), `state.rs::navigation_workspace_entries`, `mouse.rs`, `mobile.rs`, `actions.rs`, `input.rs`, `context_menu.rs`. Verify: ordering, block, escaped-worktree, navigation-order, collapsed-group, scroll and hiding-precedence tests pass in both sidebar modes.
- [x] 2.3 Render the status-colored pin glyph in workspace and agent rows (expanded, collapsed, mobile), following the `dots` and `symbols` rules. Verify: render tests pass.
- [x] 2.4 Add the pin and unpin entries to the workspace, agent row and pane context menus, hidden for servers without `pin.subscribe`. Verify: context menu tests pass, including an unsupported endpoint.

## 3. Docs and spec

- [x] 3.1 Document pins in `README.md` or `docs/next/` and add a `CHANGELOG.md` Unreleased entry.
- [x] 3.2 Archive the change and run `just spec-check`.
