# Focus and Snooze

Focus narrows one client's view to a project or workspace. Snooze hides a project or workspace on every client until its wake time. Neither stops terminals or processes.

## Focus

- Focus is client-local: each client chooses its own Focus, and other clients are unaffected.
- A project is an existing worktree group on one endpoint and session; ungrouped workspaces are focused individually. Agents follow the workspaces they belong to.
- Workspace context menus offer **Focus**. The expanded sidebar header shows `Focused: {name}` with a control to clear it, and the sidebar footer offers Focus without using an extra terminal row.
- Clearing Focus keeps existing snoozes.

## Snooze

- Snooze is shared and persisted by the server that owns the work, so every client sees the same hidden projects and workspaces.
- Snoozing a project covers all of its workspaces, including ones added later. Snoozing a workspace does not affect its siblings. A workspace snoozed on its own stays snoozed when its project wakes.
- Snooze never stops processes, mutes notifications, or acknowledges requests. When a client shows only snoozed work, terminal input is suppressed.
- Workspace context menus offer **Snooze workspace**; project roots also offer **Snooze project**. The picker shows the absolute wake time before you confirm, and Cancel changes nothing.
- At wake time the work reappears without taking focus. Waking keeps the client's Focus.
- The footer shows `Snoozed · N`. The Snooze management dialog lists snoozed projects with their explicit and inherited workspace rows, creation time, remaining time, and wake time. Inherited rows are not counted as separate snoozes. Host navigation appears when more than one host has snoozed work.

Snoozing individual agents, tabs, or panes is not supported.

## Known limits

- An open Snooze management dialog does not refresh when another client changes a snooze. Actions on stale rows are rejected safely; close and reopen the dialog to refresh it.
- Saved identities that cannot be matched stay visible so they can be reassociated, rather than hiding unrelated work.
- Clients without Superherdr's shared Snooze support, such as original Herdr clients, do not hide snoozed work.
