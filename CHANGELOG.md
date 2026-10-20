# Changelog

Superherdr's release history. Superherdr is a fork of [Herdr](https://github.com/herdrdev/herdr); changes inherited from upstream are described in Herdr's releases and ported into the inherited sections at the end of this file.

## [0.9.5.2] - Unreleased

### Fixed
- The installer attached to a release installs that release by default; 0.9.5.1's installer installed 0.9.1.2.
- What's New shows the running release's notes; 0.9.3.1 and 0.9.5.1 left out their own.

## [0.9.5.1] - 2026-10-15

Based on Herdr 0.9.5. Upstream's 0.9.4 and 0.9.5 release bodies are ported into the inherited sections at the end of this file.

### Added
- `herdr plugin update` updates plugins installed from GitHub, all at once or by name. Running plugin panes keep working on their old files, and locally linked plugins are left alone. (#4245)

### Changed
- Focus, Snooze, and the Top-level Agents view continue to work across the upstream input and sidebar overhaul, including the worktree-group rendering fix that keeps repository parents out of worktree children, merged with Superherdr's Focus and Snooze visibility rules.
- On Linux and macOS, closing the shell that started the server no longer stops the server. Scripts that stopped Herdr with `SIGHUP` should use `herdr server stop` or `SIGTERM`. (#4920, #4965)
- Closing a workspace's last pane now asks for confirmation. Disable it with `ui.confirm_close = false`. (#5143)
- With mouse capture on, plugins that handle a link get first pick when you click it. Turn capture off to use your terminal's own link shortcuts, like Command-click on macOS. (#4874, #5064)
- The Codex integration no longer installs turn hooks, and Codex status comes from screen detection again. Run `herdr integration install codex` to update. (#5246, #5278)

### Fixed
- Prefix shortcuts work again when the terminal sends the next key as plain text. This affected Tern, Chinese, Japanese and Korean input methods on macOS, and Ghostty `text:` keybinds that send the prefix and a key together. Workspace navigation had the same problem. (#5255)
- Claude Code questions show as blocked in named sessions. (#4573, #5192)
- Restarts and shutdowns preserve agent sessions through pre-shutdown saves, and session saves are written to disk before replacing the old file, so an interrupted save cannot corrupt them. (#4414, #5150)
- Ctrl+Z suspend followed by `fg` keeps the agent name and the saved conversation. (#1647)
- Ctrl shortcuts work on non-Latin keyboard layouts such as Russian, Greek, and Hebrew. (#1079, #4961)
- Session restore stays responsive with slow or missing Git data, worktree groups survive unavailable checkouts, and nested duplicate workspaces no longer appear. (#4926, #4736, #4945)
- Out-of-range workspace numbers return `workspace_not_found` instead of crashing. (#5096)
- The server survives process and thread exhaustion instead of killing all panes. (#3324)
- Forwarded SSH agents are no longer polled every second, which broke SSH and Git auth over time. (#4931, #4962)
- Sidebar width, collapsed groups, and agent sorting survive reconnects. (#4364, #5019, #4536)
- Typing appears faster, notably in PowerShell, with lower CPU for busy agents and idle shells and less memory for hidden panes. (#5149, #4506, #5083)
- macOS `pane run` and `agent start` reject over-long commands during shell startup instead of silently truncating them. (#4950, #4988)

## [0.9.3.1] - 2026-10-08

Based on Herdr 0.9.3. Upstream's 0.9.2 and 0.9.3 release bodies are ported into the inherited sections at the end of this file.

### Added
- The global menu's "what's new" entry is now always available and opens the full merged release history: released Superherdr sections first, then the inherited Herdr sections, rendered from this changelog with no network or filesystem access. Startup, product announcements, and update-preview behavior are unchanged.

### Changed
- Focus, Snooze, and the Top-level Agents view continue to work across the upstream changes, including the new aggregate-navigation indexing and plugin `min_herdr_version` validation against the real Herdr version (the fork's pinned `HERDR_PLUGIN_COMPATIBILITY_VERSION` constant is gone, matching upstream).
- Upstream's release, preview, website, and Windows cross-compile tooling remains excluded from the fork as before.

## [0.9.1.2] - 2026-09-21

### Changed
- Focus now clears automatically when navigation leaves the focused context: creating a workspace with the keyboard shortcut, focusing work through the CLI or another client, switching machines, or closing the focused workspace all restore the full sidebar, and a scope whose target closed or whose server rebooted clears on the next snapshot. Snooze-driven hiding still keeps Focus.
- While Focus is active, machines other than the focused one hide from the sidebar; their agents were already hidden. Clearing Focus brings them back.

## [0.9.1.1] - 2026-09-18

Based on Herdr 0.9.1. Inherited upstream changes are described in the inherited sections below.

- Focus, Snooze, and the Top-level Agents view continue to work across the new upstream agent-view projection, per-machine workspace navigation, and collapsed-group changes.
- The vendored libghostty-vt build now requires Zig 0.16.0 (inherited from upstream; CI builds with it).
- Upstream's release, preview, and Windows cross-compile tooling is excluded from the fork as before.

## [0.9.0.1] - 2026-09-15

First Superherdr release, based on Herdr 0.9.0. Superherdr versions are the Herdr base version plus a Superherdr revision.

### Added
- Focus shows the project or workspace selected in each client; other clients keep their own Focus.
- Snooze hides a project or workspace on every client until its wake time, without stopping its terminals.
- The All / Top-level Agents toggle hides linked-worktree agents when their parent exists on the same endpoint.
- Release binaries for macOS Apple Silicon and Linux x86_64, a Homebrew formula (`jsonmartin/tap/superherdr`), and a checksum-verifying shell installer.

### Changed
- Superherdr installs and runs as the `herdr` command and uses Herdr's configuration, state, plugins, and sessions, so it is a drop-in replacement. Separate setups are opt-in with `HERDR_CONFIG_PATH`, `XDG_CONFIG_HOME`/`XDG_STATE_HOME`, or `--session`.
- `herdr --version` prints the Herdr base and the Superherdr version, for example `herdr 0.9.0 (superherdr 0.9.0.1)`.
- Binary self-update is disabled.
- New worktrees default to `~/.herdr/worktrees`, as in Herdr.
- Connecting to a remote host finds and installs `herdr` in the same places Herdr does. Superherdr does not download release binaries for remote hosts; set `HERDR_REMOTE_BINARY` or install herdr there.

### Inherited from Herdr 0.9.5


This is a hotfix release for v0.9.4. See the v0.9.4 notes for the full feature release: https://github.com/herdrdev/herdr/releases/tag/v0.9.4

Fixed:
- Prefix shortcuts work again when the terminal sends the next key as plain text. This affected Tern, Chinese, Japanese and Korean input methods on macOS, and Ghostty `text:` keybinds that send the prefix and a key together. Workspace navigation had the same problem. (#5255)
- Codex sessions no longer start with a "clamping Interrupt hook timeout" warning. Herdr no longer installs Codex turn hooks, and Codex status comes from screen detection again. Run `herdr integration install codex` to update. (#5246, #5278)
- Claude Code questions show as blocked in named sessions. (#4573, #5192)
- On Windows, plugin panes start even when a folder on `PATH` has the same name as the plugin's program. (#5267, #5269, thanks @JJLiebig)
- On Windows, Claude Code session tracking works when the user folder name contains non-ASCII characters, such as Korean. (#5253, #5270, thanks @JJLiebig)
- Adding a Windows machine no longer prints raw PowerShell progress output. (#4639, #5282, thanks @JJLiebig)

### Inherited from Herdr 0.9.4


Added:
- `herdr plugin update` updates plugins installed from GitHub, all at once or by name. Running plugin panes keep working on their old files, and new runs pick up the update without a config reload. Locally linked plugins are left alone. (#4245, thanks @JJLiebig)
- Windows notifications stay in Notification Center. Clicking one jumps to its pane and brings the terminal window forward. (#4773, #4803, thanks @JJLiebig)

Changed:
- An elevated Windows server now only accepts administrator clients by default. To let normal clients from the same account in, set `[server] allow_unelevated_clients = true` or start the server with `--allow-unelevated-clients`. Permission errors now say how to fix them, and sessions you can't reach show as unavailable instead of stopped. (#4717, #4769, #4777, #4779, thanks @JJLiebig)
- On Linux and macOS, closing the shell that started Herdr no longer stops the server. Scripts that stopped Herdr with `SIGHUP` should use `herdr server stop` or `SIGTERM`. The server log now says why the server stopped. (#4920, #4965)
- With mouse capture on, plugins that handle a link get first pick when you click it. Turn capture off to use your terminal's own link shortcuts, like Command-click on macOS. (#4874, #5064, thanks @JJLiebig)
- Antigravity and Grok count as idle once they are back at their prompt, even if their own background tasks or subagents are still running. (#3530, #4735, #4959)
- Ctrl+Shift+letter reaches plain shells as the Ctrl key, like in Alacritty and WezTerm. Super, Hyper and Meta shortcuts no longer type escape-code junk into apps that can't read them. (#4356, #5017)

Fixed:
- Restarting a Mac or shutting down Windows keeps your agents' sessions. Herdr saves before the system starts closing apps, so agents come back where they were. (#4414, #5013, #5057, #5092, thanks @JJLiebig)
- Session saves are written to disk before they replace the old file, so a crash or power cut can't leave an unreadable save. (#5150, #5152, thanks @JJLiebig)
- The server keeps running when the machine runs out of processes or threads, instead of taking every pane down with it. (#3324, #4958)
- An out-of-range workspace number returns `workspace_not_found` instead of crashing the server. (#5096, #5171)
- Suspending an agent with Ctrl+Z and bringing it back with `fg` keeps its name and saved conversation. (#1647, #4964)
- Restoring a session stays responsive when a checkout's Git data is slow or missing. Worktree groups survive checkouts that are temporarily unavailable, and opening the same repository twice no longer nests one workspace inside the other. (#4926, #4736, #4945, #4770, #4772, thanks @minatoaquaMK2 and @JJLiebig)
- Closing a workspace's only pane asks first, like closing its last tab already does. Turn this off with `ui.confirm_close = false`. (#5143, #5173)
- Ctrl shortcuts like Ctrl+W work on non-Latin keyboard layouts such as Russian, Greek or Hebrew. Latin layouts and AltGr characters keep their normal meaning. (#1079, #4961)
- Accent keys and AltGr shortcuts work on Windows. (#4688, #5021, thanks @JJLiebig)
- Shift+Enter and other enhanced keys keep working after the outer terminal resets, for example after VS Code's Reload Window. (#4985, #5088)
- Quick Escape taps work on macOS with WezTerm's Kitty keyboard mode. (#1266, #5089)
- Kitty keyboard apps get key releases and F1, F2 and F4 correctly, and Backspace works when the terminal sends Ctrl+H for it. (#4184, #4403, #3244, #5017)
- Keyboard and terminal color replies no longer leak stray characters into pane input, and a half-finished color reply no longer freezes typing. (#4856, #4025, #5052, #5017, #5087)
- Fish no longer repeats prompt fragments while you resize a pane. The same problem with zsh and Powerlevel10k is still open, waiting on a Ghostty fix. (#3672, #4977)
- Apps that ask for pixel mouse coordinates get them, and clicks land in the right place on terminals that only report cells. (#4750, #3675, #5017, #4971)
- Turning off Kitty graphics makes panes stop advertising image support, so apps fall back to text. (#4978, #4979)
- Typing shows up faster, especially in PowerShell. (#4884, #5149, thanks @JJLiebig)
- Big sessions use less CPU on busy agents and idle shells, hidden panes use less memory, and Windows panes need fewer threads. (#4506, #5083, #4845, #4847, #4849, #4850, #4873, #5085, #5091, thanks @JJLiebig)
- New workspaces, tabs and splits start their shell at the right size instead of borrowing another pane's size. (#4419, #4873)
- Forwarded SSH agents are no longer contacted every second, which could eventually break SSH and Git authentication. (#4931, #4962)
- Sidebar width, collapsed groups and agent sorting on remote machines survive reconnects. Switching workspaces quickly while changing machines no longer gets canceled. (#4364, #5019, #4536, #5061, thanks @minatoaquaMK2 and @JJLiebig)
- The sidebar collapse button stays clickable when the agent list has a scrollbar. (#4692, #4734, thanks @minatoaquaMK2)
- The welcome screen shows your configured prefix. (#4745, #4747, thanks @JJLiebig)
- Ctrl+[ leaves workspace navigation. (#1431, #2340, thanks @qaz74107410)
- Keybindings that use `+` survive config reloads, and `/` opens Go To search on layouts where it needs Shift. (#4960, #4963, #4983, #5090, thanks @minatoaquaMK2)
- Pasting from WezTerm on Windows into remote panes no longer types encoded key data. Copying no longer adds duplicate clipboard history entries, and closing a pane no longer hangs on a pending Enter. (#4725, #4730, #4709, #4872, thanks @JJLiebig)
- Windows shells started through Scoop or 32-bit wrappers keep their working directory in new tabs and after restarts. (#4472, #4749, #5062, thanks @JJLiebig)
- Windows ARM64 machines can be used as remote hosts through the x64 build, on a best-effort basis. (#4752)
- Codex reports finished turns through its integration when started with `--no-daemon`, its startup update screen shows as blocked, and Codex and Pi status is recognized in more cases. (#4507, #4811, #4756, #5174, thanks @JJLiebig)
- Antigravity permission, question and workspace-trust dialogs show as blocked, and its thinking spinner shows as working. (#3871, #3419, #3530, #4959)
- Hermes started through its Python installer is recognized, and installing or removing its integration keeps your YAML plugin list intact. (#4910, #4909, #4911, #4984, thanks @minatoaquaMK2)
- On macOS, `pane run` and `agent start` refuse commands too long for a shell that is still starting, instead of silently cutting them off. (#4950, #4988, thanks @minatoaquaMK2)

Removed:
- The experimental `HERDR_WINDOWS_INPUT_BACKEND=crossterm` setting is gone. Windows always uses its native console input. (#5017)

### Inherited from Herdr 0.9.3

This is a hotfix release for v0.9.2. See the v0.9.2 notes for the full feature release: https://github.com/herdrdev/herdr/releases/tag/v0.9.2

Fixed:
- Terminal shortcuts that send Escape followed by a key work again in panes. On macOS, Option+Left/Right and Option+Backspace from Ghostty's defaults or iTerm2's Natural Text Editing preset move and delete by word again, instead of typing `b` and `f` or deleting one character. Escape-based Shift+Enter bindings insert a newline in Claude Code instead of submitting. Clicking a pane still doesn't send a stray Escape. (#4751)
- Alt+[ followed quickly by another key no longer merges into a different key. (#4751)

### Inherited from Herdr 0.9.2

Breaking changes:
- The Herdr-specific pane graphics API is gone. `pane.graphics.info`, `set`, `clear`, and `stream` now return `unknown_method`. Apps show images by writing standard Kitty graphics to their terminal, which Herdr renders natively. (#4561)

Added:
- Agents can report their own resume command. Herdr then reopens their exact session after a server restart, with no built-in integration needed. Self-reported agents are also cleared once their pane is back at an idle shell. The new Add Herdr support to your agent guide covers state, resume, and release for agent authors. (#4687)
- Use more than one prefix key: `prefix = ["ctrl+space", "ctrl+s"]`. Every entry enters the same prefix mode, and `prefix+?` lists them all. (#4653, thanks @JJLiebig)
- Go To shows every agent and terminal as its own row, grouped by workspace, with its status and path. Left and Right jump between workspaces. (#4384)
- Bind `keys.clear_pane` to clear the focused pane's screen and scrollback while keeping the current prompt line. It is unbound by default and leaves full-screen apps such as Vim alone. (#4383)
- `herdr machine status` checks saved machines without prompting. `herdr machine reconnect` finishes SSH authentication, including MFA, in your terminal. Open clients recheck failed machines every 30 seconds, so no restart is needed after fixing a connection. (#3763)
- Interactive `machine add` finds the Herdr sessions already running on the host and lets you pick one. `--label` is optional; the machine is named after its SSH host. (#4204, #4678, thanks @JJLiebig and @dhh)
- `terminal session control` accepts `terminal.mouse` events, so bridge clients can click buttons in apps that enable mouse reporting. (#4685)
- Restored agents start one at a time, 100 ms apart by default, instead of all at once. Change the spacing with `[session] startup_per_agent_delay_ms`. (#4102, thanks @JJLiebig)

Changed:
- Images render faster and more reliably. Local Ghostty receives image data through temporary files instead of terminal output. Popups and notifications crop images around themselves instead of hiding them. Images that scroll out of view stay loaded, so scrolling back no longer resends them. (#4561, #4652, #4686)
- SSH connections request compression. On slow links, remote panes catch up after scrolling in a fraction of the time. (#4340)
- Scrolling output on saved machines sends only the rows that changed, cutting bandwidth for build logs and streaming agents. Older clients and servers keep working with the previous format. (#4711)
- New panes set `TERM_PROGRAM=herdr` and `TERM_PROGRAM_VERSION`. They no longer inherit terminal session IDs, such as iTerm2's, or Claude Code session markers from the terminal that started the server. (#4104)
- Windows panes default to PowerShell 7 (`pwsh`) when it is installed, falling back to Windows PowerShell. An explicit `default_shell` still wins. (#4297, thanks @JJLiebig)
- Closing the last tab of a workspace now asks for confirmation. (#4379, #4409, thanks @minatoaquaMK2)
- Event subscriptions deliver bursts in full. A reader that falls too far behind now gets an `events_lost` error instead of silently skipping events. (#4178, #4225, thanks @minatoaquaMK2)
- On Unix, sending `SIGWINCH` to a Herdr client re-reads the host terminal's colors, so theme switchers that change colors without a light/dark notification can refresh panes. (#4349, thanks @dhh)

Fixed:
- Saved layouts survive host shutdown and failed restores. On Linux with logind, Herdr saves before the system shuts down. Every platform keeps up to 48 layout snapshots in `session-snapshots/` for manual recovery. (#4320)
- Clicking a pane no longer sends a stray Escape that interrupts a working agent. (#3480)
- An agent's first task now counts as done, even when it started with a prompt. Startup, restored sessions, and Pi's `/new` no longer fire false done notifications. Hook-reported status survives live handoff. (#3338, #3990, #3916)
- The server uses far less CPU with many populated panes. The navigator, pane splits, and named targets stay fast in large sessions, and bursts of external events no longer stall rendering. (#4506, #4546, #4426, #4669, #4670, #4671, #4672, thanks @JJLiebig and @minatoaquaMK2)
- Apps that use synchronized output no longer show torn frames while you type. The cursor no longer flickers during status redraws or jumps around an idle Codex on Windows. (#2968, #4303, thanks @JJLiebig)
- The API socket keeps accepting connections after a transient error, so CLI commands and live handoff no longer fail while the server keeps running. (#4601)
- Slow Git operations during worktree lookups no longer freeze typing. (#4492, thanks @JJLiebig)
- `worktree open` no longer takes over the repository's own workspace, so removing the worktree no longer closes it. (#4293)
- Forwarded SSH agents keep working in remote panes after a reconnect. Repeated `--machine` commands open far fewer SSH connections. (#1931, #4252)
- OpenCode V1 panes stay working or blocked while their subagents run. An existing registration in `tui.json` is respected instead of regenerating `tui.jsonc`. (#1362, #4240)
- Codex is no longer reported idle during active output, its mention popups are recognized, and remapped interrupt keys no longer break detection. (#4507, #4196, thanks @JJLiebig and @unmanbearpig)
- Kiro approval prompts are reported as blocked. Grok idle panes settle with custom or disabled OSC titles. (#4203, #4372, #4333, thanks @vinayshah1998)
- Droid's scrollback clears are honored, so its welcome header and old transcript no longer repeat. (#4432, thanks @factory-ain3sh)
- Copy mode stays active while output continues and a movement key is held. (#3812, #4281, thanks @marcomayer)
- Ctrl+Shift+letter keeps Shift in panes that have not enabled enhanced keyboard input, so Ctrl+Shift+C no longer arrives as Ctrl+C. (#4581, #4597, thanks @factory-ain3sh)
- Mouse reports split across reads no longer leak into the shell after a focus switch over SSH. (#4630)
- Windows: multi-line pastes into Claude Code no longer submit at the first newline, and Shift+Enter is preserved. Mouse capture survives focus changes and resizes, Ctrl+Win no longer opens prefix mode with a `ctrl+space` prefix, and remote clipboard image paste works again. (#4251, #4284, #4470, #4314, thanks @JJLiebig)
- Windows: npm-installed OMP and agents launched through hardened runtimes are detected. `machine add` against a Windows host no longer fails as "not ready for saved machines." (#4313, #4579, #4309, thanks @JJLiebig)
- The navigate-mode workspace highlight is visible with `theme = "terminal"`. Session navigator search matches words independently. The sidebar reveals agents selected by cycling or shortcuts. (#4300, #4408, #4273, #4535, #4355, thanks @minatoaquaMK2)
- `session attach` without a terminal explains the problem instead of leaving a new session running. (#4393)
- Socket error responses keep the original request ID, including subscription setup errors. (#4344)
- `plugin install` accepts options before the repository. (#4446)
- Windows panes pick up the host terminal's light or dark colors from the start, so apps no longer default to dark mode inside a light terminal. (#1530, #4369)
- After `herdr update`, Herdr lists which running servers are still on the old version, with the exact commands to restart each one, and reminds you to update your saved SSH machines.

### Inherited from Herdr 0.9.1

Added:
- Control agents, panes, workspaces, and worktrees on saved SSH machines with `herdr --machine <label-or-id>`. Commands use the saved machine's session without needing an open Herdr window. Update Herdr on both machines to use CLI forwarding; failed remote commands never fall back to Local. (#3918)
- Connect to Windows SSH hosts from Linux, macOS, or Windows. Interactive setup can install or update the complete Windows package after confirmation; background reconnects never install updates. (#3651, #3701, #3661, #3687, thanks @JJLiebig)
- Edit names, filters, and search text at the cursor instead of only at the end. Herdr inputs now support character and word movement, Home/End, deletion, and familiar Ctrl+A/E/K/U/W/Y shortcuts, including Unicode text. (#1803, #3698, thanks @markjaquith)
- Added Letta Code detection and native conversation restore, including default conversations. Its experimental integration is installed through `herdr integration install letta`, not the Settings integration list or JSON integration API. (#3106, #3107, thanks @just-cameron)
- Hold Ctrl over a link to highlight it before opening it. Wrapped URLs and OSC 8 links remain clickable even when their beginning or end is outside the viewport. (#1282)
- Sidebar token rules can now hide matching tokens and their separators with `hide = true`; rows disappear when no tokens remain. (#3925)

Changed:
- Workspace keyboard navigation now spans connected machines in sidebar order, with a visible highlight before Enter switches machines. Collapsed machine and worktree groups remain navigable without accidentally sending actions to the wrong machine. (#3754)
- Experimental live handoff can now transfer more than 64 panes when the sending server includes this update. An older running server still has its old limit for the first upgrade; this does not change update installation order or make live handoff non-experimental. (#3393, #3411, thanks @kataokatsuki)
- On Linux and macOS, terminal observers that stop accepting output are disconnected after 30 seconds without write progress. Their pane and other clients keep running; an interrupted stream may end without a final close record. (#3612)
- Windows terminal cleanup now explicitly resets mouse-reporting modes. The remaining standalone Git Bash detach report in #3748 is still under investigation; do not treat this release as a confirmed fix for that report. (#4055, thanks @JJLiebig)

Fixed:
- Remote typing, switching, and popup interaction no longer resend the entire pane screen for small changes. Busy SSH sessions use less bandwidth, and idle attached clients avoid unnecessary redraw work. (#3769, #3745, #3822)
- A stalled SSH machine no longer traps the client away from Local. Clicking a local workspace or agent cancels the unfinished remote switch, and a recovered remote workspace refreshes without an away-and-back selection. Stale screens from before a reconnect are not reused. (#3903, #3842)
- Idle SSH connections use less CPU without losing final output. Repeated connection failures back off instead of reconnecting rapidly, and supported idle bridges clean up without stopping remote panes. (#3728, #4083)
- Large pastes no longer disconnect the client with an output-queue error. (#3833)
- SSH setup now shows authentication instructions and useful errors, including Tailscale login URLs and host-key failures, rather than appearing to hang or only reporting a lost connection. `machine add` also accepts options before the SSH target. (#3606, #3731, #3883)
- Machine and worktree groups collapse correctly without changing the selected workspace. The sidebar respects workspace row gaps, stays stable during resizing, and reveals newly focused workspaces. (#3778, #3738, #3817)
- The agent list keeps its scroll position when switching machines. Current-workspace and current-tab filters distinguish identical IDs on different machines. (#3937, #4101, #3732, thanks @minatoaquaMK2)
- Background machine activation no longer resizes another client's focused pane. Window titles follow each client's own view rather than another client's selection. (#3744, #4091, #4130, thanks @JJLiebig)
- `agent focus` and `pane move --focus` now move attached clients to the requested pane. Moving without focus no longer leaves a client following a deleted tab or later unrelated focus changes. (#3760, #4153, #4171, thanks @minatoaquaMK2)
- Creating a worktree focuses its new workspace again. Partially failed worktree removals can be retried, and worktrees with submodules offer an explicit forced-removal choice rather than failing without a recovery action. (#3766, #3314, #1797)
- Temporary worktree-action error banners expire instead of remaining on screen indefinitely. (#1827, #4063, thanks @JJLiebig)
- The last tab remains fully visible when the tab strip overflows, and the session navigator once again draws connected tree branches. (#4151, #3843)
- Muted sidebar and tab labels remain readable on dark themes, including Windows Terminal. (#2692, #4062, thanks @JJLiebig)
- Mouse drag and copy-mode selections stay active while output continues, even when part of the selection is outside the viewport. Copying uses the current text in the selected range. Selection repainting also avoids building up a backlog of mouse movements. (#3841, #3895)
- Double-click selections remain visible with automatic copy disabled. Holding the second click and dragging extends by whole words, and token selection stops at CJK punctuation. Host selection colors remain visible on transparent backgrounds. (#3847, #1836, #3706, #3798)
- Scrolling no longer combines mismatched terminal snapshots or leaves stale wrapped rows in fullscreen applications such as Neovim. Alternate-screen history reads preserve content across changing headers and scrollbars. (#3900, #3329, #3979)
- Local Kitty graphics keep their direct transport after the first image. Popups and notifications hide only overlapping image placements; uncovered images remain visible. (#3785, #4081)
- Pixel mouse coordinates work independently of image rendering and use the terminal's integer cell size, preventing clicks from drifting across the screen. (#3295, #4140, thanks @JJLiebig)
- Function keys F1–F12 are recognized in Kitty keyboard reports, and F1–F4 also work with parameterized reports from terminals such as foot. (#1809, #2378, #4067, thanks @ashokDevs and @JJLiebig)
- WezTerm control-key reports now preserve Enter, Backspace, Tab, and Escape. Cmd/Super-modified input reaches pane applications with its modifier intact. Modified Enter stays compatible with panes that have not requested enhanced key encoding. (#3589, #3710, #4103, #4110, thanks @minatoaquaMK2)
- Mouse reports split across delayed reads no longer leak their trailing bytes into pane input. Clients reassert mouse reporting after terminal reconnects. (#3911, #4079, thanks @JJLiebig)
- Saved sessions that cannot be loaded are preserved before replacement, rather than silently overwritten. Recovery copies are kept in `session-backups/`; failed preservation leaves the original untouched. (#4125)
- Deleting a session now requires its exact name, including case, on case-insensitive filesystems. Names beginning with a hyphen can be passed after `--`. (#3819, #4233, #3220, thanks @minatoaquaMK2)
- Newly started macOS servers retain access to user lookup and DNS across logout. Existing affected servers need a full session restart; live handoff cannot repair their inherited service context. (#4100)
- Linux process discovery stays responsive around stuck WSL agents and avoids repeatedly scanning an ever-growing process tree. (#2179, #3621, #3674, thanks @dark2momo and @caner-akca)
- Linux desktop notifications identify Herdr as their source. (#3638)
- `pane split` without a target now splits the calling pane when invoked inside Herdr, rather than another client's focused pane. Explicit targets keep their existing behavior. (#4123)
- Live agent names survive temporary process-detection uncertainty instead of leaving an active agent unreachable by name. (#3225, #3574, thanks @caner-akca)
- Windows Codex prompts reliably submit after pasted text instead of losing Enter, including long prompts. (#3187, #3961, thanks @JJLiebig)
- OpenCode V2 reports lifecycle state from the pane displaying the conversation, so shared servers do not mix up panes. Completed and interrupted turns return to idle, while pending permissions, forms, and failed executions remain blocked. V1 remains supported; restart OpenCode after installing the updated integration. (#3757, thanks @markjaquith)
- Codex stays working with static titles, animations disabled, and queued follow-ups. Composer sparkles and old confirmation text no longer make an idle pane appear blocked. (#4092, #4099, #3988, thanks @minatoaquaMK2)
- Claude Code's Unicode activity spinner and Pi's working border are recognized correctly. Cline launcher detection and idle-state reporting are more reliable. (#3949, #3629, #2396)
- Pi records native session paths on Windows. Nested OMP processes no longer replace the parent pane's resumable session, and Windows Kimi npm shims are recognized. (#3726, #2593, #3994, #3317, #4053, thanks @JJLiebig)
- Grok updates its restore reference after `/new`, and its imported Claude-compatible hooks no longer overwrite that reference. Reinstall the Claude integration to adopt the narrower hook matching. (#2681, #4018)
- Integration configuration updates preserve existing files when writes fail. Hermes installation also preserves valid YAML when enabled plugins use an inline list. (#3970, #3839, #4056, thanks @JJLiebig)
- Plugin registry writes preserve symlinks. Plugins keep using the running server's compatible binary after a client-only update, and manual pane navigation emits focus events again. (#3922, #3824, #4077)
- Config files containing a UTF-8 BOM load and save correctly. The default config places `ui.accent` in the right table. (#3678, #3962, #2697, thanks @JJLiebig)
- Windows local and SSH input preserves Alt combinations, Ctrl+S, and enhanced keyboard text instead of dropping keys or inserting garbage. Remote dead-key input no longer adds an extra base character. (#3702, #3910, #3932, #3948, #4133, #4148, thanks @JJLiebig)
- Windows mouse capture preserves SGR coordinates, including past column 95, and reattaching restores mouse reporting. Legacy SSH mouse reports no longer produce phantom characters or block subsequent input. (#4155, #3735, #4080, thanks @JJLiebig)
- Git Bash SSH connections honor host aliases from the user's SSH config. (#3947, #4054, thanks @JJLiebig)
- Windows login-mode panes honor the configured shell and keep PowerShell working-directory reports current. Pane launch paths use consistent directory casing. (#1445, #4060, #4065, #4199, thanks @JJLiebig)
