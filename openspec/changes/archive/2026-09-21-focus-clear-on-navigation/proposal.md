# Change: Focus clears automatically when navigation leaves the focused context

## Why

Focus filtered the sidebar for a context the user had already left. Enabling Focus on a space and then creating a new space with the keyboard shortcut left the new space on screen while the sidebar stayed filtered to the old one; CLI or cross-client focus changes and machine switches behaved the same way. A scope whose target workspace closed, or whose server rebooted, kept filtering forever. Focus is client-local presentation state, so this is fixed entirely in the client.

## What Changes

- The client clears Focus automatically when the focused workspace changes to a live workspace outside the scope, or when the scope no longer matches any workspace on its endpoint (target closed, server reboot).
- While Focus is active, machines other than the focused one hide from the sidebar; their agents were already hidden. Clearing Focus restores them.
- Explicit Focus clears are unchanged, Snooze-driven hiding is unchanged, and no server or protocol behavior changes.

### Before and after

| | Before | After |
| --- | --- | --- |
| New workspace via keybind while scoped | Screen shows the new workspace; sidebar stays filtered to the old scope | Focus clears; the sidebar matches the screen |
| Focus moves outside the scope (CLI, other client, close refocus) | Sidebar stays filtered | Focus clears |
| Endpoint switch while scoped | Other machines show headers with no sessions; the scope persists | Other machines are hidden while scoped; switching clears Focus and restores the full list |
| Focused workspace closed or server reboot | Scope survives, filtering nothing or the wrong thing | Scope clears on the next snapshot |
