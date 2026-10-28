# Change: Pin workspaces and agents to the top of their machine

## Why

With several machines and many workspaces and agents, the few a user is actively watching drift down the sidebar as work is created, reordered or sorted by priority. Users cannot keep them in a fixed, glanceable place. Pins are user organization of shared session work, like Snooze, so they belong to the server and are seen by every client of that server. This is Superherdr behavior built on Superherdr's Snooze patterns, not an upstream Herdr fix.

## What Changes

- A user can pin or unpin a workspace from its context menu and an agent from its agent row or pane context menu.
- In the sidebar, each machine lists its pinned workspaces first, then the rest. The Agents list shows pinned agents first. Keyboard navigation, numbered shortcuts, the collapsed sidebar and the mobile switcher follow the same order.
- A pinned row shows a pin glyph colored by the item's current status (working, blocked, done, idle, unknown).
- The server owns, broadcasts and persists pins in a new `pins.json` beside `snooze.json`. Pins survive server restarts.
- New optional socket API methods `workspace.pin`, `pane.pin` and `pin.subscribe`, and a `pin.state` endpoint control message. Clients hide the pin actions for servers that do not advertise them.

### Before and after

| | Before | After |
| --- | --- | --- |
| Watching three workspaces among twenty | They move as others are created or reordered | Pinned, they stay at the top of their machine |
| Agents list sorted by priority | A watched idle agent sinks below busy ones | A pinned agent stays in the leading block |
| Second client or server restart | n/a | Same pins on every client and after restart |
| Herdr server or remote | n/a | No pin actions; everything else unchanged |

## Non-goals

CLI commands, keybindings, manual ordering among pinned items, pinning tabs or projects, per-client pins, and notes on workspaces or agents.

## Alternatives and drawbacks

- Keep current behavior: users rely on manual reordering, which does not help the priority-sorted Agents list.
- Client-local pins: no protocol work, but pins would not follow the user across clients, and workspace and pane ids are per server boot, so local pins go stale after a restart.
- A `pinned` field on the shell snapshot: rejected because `ClientShellSnapshot` is part of the frozen generation-1 endpoint codec.
- Drawback: another Superherdr-only state file and optional API surface to keep compatible.

## Impact

- UI: workspace and agent context menus, sidebar machines list (expanded and collapsed), Agents list, mobile switcher, status glyph rendering.
- Socket API: three new optional methods and one control message (Full spec below). No existing method, codec or fixture changes. `PROTOCOL_VERSION` unchanged.
- Persistence: new `pins.json`; `session.json` format unchanged. Every pin change checkpoints the session first, as Snooze does, so the identities a pin references are on disk.
- Compatibility: Herdr clients ignore pins; Herdr servers and remotes lack the methods, so Superherdr clients hide pin actions for them. Switching back to Herdr leaves `pins.json` in place.
- Upstream merges: new code lives in new modules (`src/server/pin_store.rs`, client pin state); inherited files get small call-site edits for ordering and menus.

## Verification and open questions

- Unit tests for the store (round trip, restore id remapping, malformed file, size bound), API methods and broadcast, ordering builders, navigation order and hiding precedence, and unsupported endpoints.
- `just check` and a manual smoke with a debug build: pin, reorder, restart the dev server, confirm pins persist.
- Assumption: public pane numbers survive restore (covered by `restore_preserves_public_id_mapping_after_pane_id_remap`). If that fails for a layout, the agent pin is dropped rather than moved to another pane.
