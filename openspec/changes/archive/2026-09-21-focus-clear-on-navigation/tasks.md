# Tasks

## 1. Automatic clear

- [x] 1.1 In `src/client/shell/state.rs`, add `apply_focus_scope_navigation_rule` and call it from `apply_active_snapshot` after the snapshot is installed: clear the scope when it matches no workspace on the endpoint, and clear it when the focused workspace transitions to a live workspace the scope rejects. Verify: `cargo nextest run focus_scope` passes.
- [x] 1.2 In `src/client/shell/endpoint_sidebar.rs`, hide machine rows for endpoints other than the focused one while a scope is active in the collapsed and expanded sidebars. Verify: `cargo nextest run focus_scope_hides_other_machines` passes.

## 2. Tests

- [x] 2.1 Cover the in-flight snapshot race, out-of-scope transitions, in-scope transitions, stale scopes, out-of-scope snoozed destinations, close refocus, and endpoint-qualified scopes in `src/client/shell/tests/focus_snooze.rs`. Verify: `cargo nextest run focus` passes.
- [x] 2.2 Cover machine hiding and restoration in `src/client/shell/tests/endpoints.rs`. Verify: `cargo nextest run focus_scope_hides_other_machines` passes.
