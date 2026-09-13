# Focus and Snooze: first working local version

This fork extends upstream v0.9.0 (`b99002ac99b09e00b4ca692436cb15a6b0d676f1`). Core Focus/shared Snooze landed in `84f228b85f1c717469cfdac8338adaec467f647a`; the fixed-height sidebar footer landed in `f695380227f2c7f6c46475f812eaf0417e04ec0f`. Commit `585fe3d2e2fe21ca0595f50edaac8c147e331971` finishes the accepted menus, Focus header, and Snooze management table. It is a local working checkpoint, not a public release.

## Behavior

- Focus is client-local. A project is an endpoint/session-qualified existing worktree group; ungrouped workspaces focus individually. Related agents follow workspace membership. Clearing Focus preserves Snooze.
- Project/workspace Snooze is shared and persisted by the owning server. A project covers its workspaces, including new members; workspace Snooze does not cover siblings. Explicit child snoozes survive waking the parent. Waking preserves Focus.
- Snooze hides work without stopping its processes, muting notifications, or acknowledging requests. Empty presentation suppresses terminal input. Timer wake does not steal focus; a client latched in the empty view requires deliberate selection.
- Workspace context menus offer Snooze workspace and Focus; non-linked project roots also offer Snooze project. The picker previews an absolute wake time before confirmation. Cancel changes nothing.
- Expanded headers show `Focused: {name}` with a clear control. The permanent footer offers Focus and `Snoozed · N` beside the collapse control without consuming an additional terminal row. Collapsed sidebars show only the expand control.
- Snooze management displays project trees with explicit and inherited workspace rows, creation time, remaining time, and wake time. Only nonempty sections appear. Inherited rows do not inflate the explicit count or pretend to have their own wake record. Host navigation appears only for multiple known Snooze hosts.
- Individual agent/tab/pane Snooze, conversation History/Reopen, and top-level Agents filtering are not implemented in this version.

## Closeout evidence — September 13, 2026

The user successfully trialed the feature and successive UI refinements in this chat: `01a08ea3-0ea4-7061-9f4e-615233e35476`. Earlier real two-client smoke verified non-active-row Focus targeting, client independence, shared hiding, empty-input suppression, expiry, heartbeat continuation, and canonical identity retention. Those are historical checks, not a fresh replay at closeout.

Fresh closeout checks on the candidate code:

| Command | Result |
| --- | --- |
| `cargo test --locked client:: -- --test-threads=1` | 524 passed, 0 failed, 1 ignored |
| `cargo test --locked snooze -- --test-threads=1` | 111 passed, 0 failed, 1 ignored |
| `cargo build --locked` | Passed; existing dead-code warnings remain |
| `python3 -m unittest scripts.test_ui_hot_path_architecture` | 6 passed |
| `rustfmt --edition 2021 --config skip_children=true --check <modified Rust files>` | Passed |
| `git diff --check` | Passed |

The Rust filters overlap. The ignored environment-dependent DST test previously passed under `TZ=America/Los_Angeles`; it was not rerun at closeout. Only modified files were formatted. `just` was unavailable, so the existing direct Cargo fallback was used with four build jobs, Xcode's macOS SDK, and the already-established private repaired Zig wrapper. No toolchain or production installation changed. Full `just check`, Windows/Linux runtime checks, fresh SSH interaction, release-mode scaling, and release-smoke comparisons were not performed for this local checkpoint.

Earlier GLM review covered the core trial. A fresh Gemini 3.8 Flash High source review covered the final UI diff and returned no blocking findings; the coordinator checked its key claims and ran the checks above. Reviewer language claiming exhaustive correctness, zero overhead, or already-staged files is not adopted as evidence. This closeout makes no public-release certification claim.

## Known limits

An open management dialog is not rebuilt on every cross-client Snooze broadcast. Stale actions reject safely; close and reopen the dialog to refresh its records. Its countdown refresh does not solve that record-refresh limitation. Unavailable/uncertain saved identities remain visible for reassociation rather than hiding unrelated work. Stock clients without the optional shared Snooze feature do not adopt this fork's presentation filtering.

## Local trial and rollback

The current isolated checkout is `/Users/json/Projects/herdr-helper/focus-snooze-trial`. Its existing local launcher is `.local/prd/focus-snooze/open-trial.command`; it uses private session/config/socket paths, clears inherited socket overrides, and disables update checks for the trial. This ignored, machine-specific launcher is not a distributable installer. The existing visible client/server was not restarted during closeout.

Return to the stock client by closing only the trial client window; that does not close running work. Do not stop or overwrite a production server or remove its configuration. Keep the private trial state until it is no longer needed. Product naming, standalone installation/state paths, updater isolation, and GitHub publication belong to the next Superherdr rename task.

## Maintenance boundary

Keep client projection/menu/overlay changes in `src/client/shell`, shared Snooze/API/persistence in their existing server modules, and frozen endpoint codecs unchanged. Do not import the separate Wrangler runtime or terminal parser. If upstream supplies equivalent behavior, compare identity, client-local Focus, shared Snooze and recovery semantics before dropping the fork changes.
