# Pins

Right-click a workspace and choose **Pin workspace**, or right-click an agent row or its pane and choose **Pin agent**. Choose **Unpin** from the same menu to undo it.

A pinned workspace moves to the top of its machine in the sidebar. A parent workspace keeps its worktree children with it; a pinned worktree under an unpinned parent moves out of the group on its own, even when the group is collapsed. Pinned agents lead the Agents list: within each machine when sorted by spaces, and as a leading block ordered by machine when sorted by priority or by an agent view. Keyboard navigation, numbered shortcuts, the collapsed sidebar and the mobile switcher follow the same order.

A pinned row shows `⚲` in its status color: working, blocked, done, idle or unknown. With the `dots` indicator style the pin replaces the dot; with `symbols` the status symbol stays and the pin sits before it.

Pins belong to the server, like Snooze: every client attached to that server sees them, and they survive server restarts in `pins.json` beside `snooze.json`. Closing a workspace or pane removes its pin; a pane moved to another workspace keeps it. A pin never shows something Focus, Snooze, Top-level or an agent view hides.

Herdr servers and remotes don't support pins, so the menu entries are hidden for them. Herdr clients connected to a Superherdr server show their usual order.
