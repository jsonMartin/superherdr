## ADDED Requirements

### Requirement: Shared, persisted pins
A user SHALL be able to pin and unpin a workspace and an agent. The server that owns the item SHALL store the pin, broadcast it to every subscribed client and persist it across server restarts. A pin SHALL belong to one server; an item with the same name or worktree key on another server SHALL NOT be pinned by it. Closing a workspace SHALL remove its pin and the pins of its agents; closing a pane SHALL remove its agent pin. A pinned agent pane whose agent exits SHALL keep its pin while the pane exists. A pinned pane moved to another workspace SHALL keep its pin under its new pane id, including when the move removes the source workspace.

#### Scenario: Pin on one client
- **WHEN** a user pins a workspace on one client
- **THEN** every client attached to that server SHALL show it pinned

#### Scenario: Server restart
- **WHEN** the server restarts and restores its session
- **THEN** the restored workspaces and agent panes that were pinned SHALL still be pinned

#### Scenario: Pinned pane moved
- **WHEN** a pinned agent pane is moved into another workspace and the server later restarts
- **THEN** the agent SHALL be pinned in the destination workspace before and after the restart

#### Scenario: Workspace closed
- **WHEN** a pinned workspace is closed
- **THEN** its pin and the pins of agents in it SHALL be removed and SHALL NOT reappear on a later workspace

### Requirement: Pinned items first
Within each machine in the sidebar, ordering SHALL work on blocks. A block is a parent workspace with its linked worktree children as the sidebar already groups them, a standalone workspace, or an orphan worktree. A block is pinned when its top workspace is pinned; its unpinned children stay attached inside it. A pinned linked worktree whose parent is unpinned SHALL leave its group, even a collapsed one, and form its own pinned block rendered unindented. Pinned blocks SHALL come first in their existing relative order, followed by unpinned blocks in their existing order. In the Agents list, pinned agents SHALL come first: in `spaces` sort within each machine's block, and in `priority` sort and in agent views with their own sort as a leading block ordered by machine and then by the existing sort. Keyboard next/previous navigation, numbered shortcuts, click targets, reveal scrolling, collapsed-sidebar rendering and previews, and the mobile switcher SHALL use the same order as the rendered list. A pin SHALL NOT make visible an item the list would otherwise hide (Focus, Snooze, Top-level mode, collapsed groups except an escaped pinned worktree, agent-view filters).

#### Scenario: Pinned workspace on a busy machine
- **WHEN** a machine has ten workspaces and the user pins the eighth
- **THEN** the eighth SHALL be listed first under that machine and next-workspace navigation from it SHALL go to the first unpinned workspace

#### Scenario: Pinned parent with unpinned child
- **WHEN** a machine lists blocks A (parent with unpinned child A1), B and C (parent), and the user pins C and then A
- **THEN** the sidebar SHALL list A, A1, C, B

#### Scenario: Pinned worktree under an unpinned parent
- **WHEN** a machine lists parent P with children P1 and P2, and the user pins P2
- **THEN** the sidebar SHALL list P2 unindented, then P with P1

#### Scenario: Priority sort
- **WHEN** the Agents list is sorted by priority and a pinned agent is idle while others are working
- **THEN** the pinned agent SHALL still be listed before the unpinned agents

#### Scenario: Pinned but snoozed
- **WHEN** a pinned workspace is snoozed
- **THEN** it SHALL be hidden until it wakes and SHALL then reappear pinned

### Requirement: Pin glyph colored by status
A pinned workspace or agent row SHALL show a single-cell pin glyph styled with the color its status indicator would use (working, blocked, done, idle or unknown). In the `dots` indicator style the pin glyph SHALL replace the status dot. In the `symbols` indicator style the status symbol SHALL remain and the pin glyph SHALL be shown before it, so status stays distinguishable without color. Rows on an offline or stale machine SHALL keep the existing dimmed styling.

#### Scenario: Pinned agent becomes blocked
- **WHEN** a pinned agent changes from working to blocked
- **THEN** its pin glyph SHALL change to the blocked color

### Requirement: Pin controls
Workspace context menus SHALL offer "Pin workspace" or "Unpin workspace". Agent row context menus, and pane context menus when the pane hosts an agent, SHALL offer "Pin agent" or "Unpin agent". A client SHALL hide these actions for a server that does not advertise `pin.subscribe`. A failed pin request SHALL leave the displayed pins unchanged and SHALL show the existing client-local notice.

#### Scenario: Herdr server
- **WHEN** a Superherdr client is attached to a Herdr server
- **THEN** the pin actions SHALL be hidden and no pins SHALL be shown for that server

### Requirement: Pin socket API
The server SHALL advertise `workspace.pin`, `pane.pin` and `pin.subscribe` as optional client shell methods. `workspace.pin` SHALL take `workspace_id` (string, public id) and `pinned` (boolean). `pane.pin` SHALL take `pane_id` (string, public id) and `pinned` (boolean). `pin.subscribe` SHALL take no parameters. Each SHALL respond with the current `PinState`: `boot_id` (string), `revision` (unsigned integer, increased on every change), `workspace_ids` (array of public workspace ids) and `pane_ids` (array of public pane ids). Setting a pin to its current value SHALL succeed without increasing `revision`. An unknown workspace or pane id SHALL fail with a not-found error and change nothing. After every change the server SHALL send the `pin.state` endpoint control message carrying the `PinState` as JSON to every subscriber. A client SHALL treat a malformed `pin.state` as an optional-feature error for that server only, and SHALL ignore a state whose `boot_id` differs from the server's current boot. A client SHALL apply a `PinState` from a subscription response, an action response or a broadcast only when its `revision` is not older than the one it holds for that boot. No existing method, codec, fixture or `PROTOCOL_VERSION` SHALL change.

#### Scenario: Pin request for a closed workspace
- **WHEN** a client sends `workspace.pin` for a workspace another client just closed
- **THEN** the server SHALL return a not-found error and SHALL NOT change any pin

#### Scenario: Older client
- **WHEN** a client that does not know `pin.subscribe` connects
- **THEN** the connection SHALL succeed and that client SHALL simply not show pins

### Requirement: Pin persistence
When session persistence is enabled, the server SHALL store pins in `pins.json` in the session data directory beside `snooze.json`, as `{"version": 1, "workspaces": [<workspace lifetime_id>], "agents": [{"workspace_lifetime_id": <string>, "pane_number": <integer>}]}`. It SHALL write the file atomically, SHALL bound it to 256 workspace entries, 256 agent entries and 64 KiB, and SHALL checkpoint the session before every pin save, so the workspace `lifetime_id` and public pane number it references are on disk. On restore it SHALL map stored entries to live workspaces by `lifetime_id` and to panes by public pane number, and SHALL drop entries that match nothing. A missing file SHALL mean no pins. An unreadable or malformed file SHALL start with no pins, SHALL be logged, and SHALL NOT be overwritten until the next successful pin change. A failed save SHALL keep the in-memory pins and log a warning.

#### Scenario: Malformed pins.json
- **WHEN** the server starts and `pins.json` is not valid JSON
- **THEN** the server SHALL start with no pins and leave the file untouched until a pin changes
