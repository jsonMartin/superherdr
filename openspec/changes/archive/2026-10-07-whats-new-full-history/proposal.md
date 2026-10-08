# Change: What's New shows the full merged release history

> Accepted by jsonmartin (maintainer) in the working session on 2026-10-07, revision 2 (post-critique). Scope: What's New full-history view from the embedded merged changelog; startup behavior untouched. Ships in Superherdr 0.9.3.1 on top of the Herdr 0.9.3 upstream sync (`upstream-sync/v0.9.3`).

## Why

### Problem and users

The global menu's "what's new" entry is fed by a single stored slot, `release-notes.json` in the config directory, written only when the update checker fetches a release. Superherdr's checker is disabled (`src/update.rs` returns immediately), so upgraded installs show whatever the slot froze at — Herdr 0.9.0's notes — while fresh installs, with no stored slot, show no entry at all. The local-changelog loader is reachable only through the fake-preview test helper, not menu loading, and resolves `CHANGELOG.md` relative to the current directory, which only works in a checkout. Installed-build users cannot see what any Superherdr release changed — neither the fork's own Focus/Snooze/Agents entries nor what inherited upstream releases brought.

This belongs in Superherdr, not upstream: it is about how the fork presents its own combined release history. Herdr's behavior does not change.

### Prerequisite

The Herdr 0.9.3 upstream sync (branch `upstream-sync/v0.9.3`, base 0.9.3). Inherited sections cover upstream releases up to the merged base; this change ships as Superherdr 0.9.3.1.

## What Changes

The "what's new" view renders Superherdr's `CHANGELOG.md` — Superherdr release sections first, then `Inherited from Herdr` sections — embedded in the binary at build time. The menu entry becomes always available and always opens this history view.

### Before and after

| | Before | After |
| --- | --- | --- |
| Menu "what's new" | Entry only when an update or stored notes exist; body frozen at whatever the checker last saved (0.9.0 on upgraded installs); fresh installs have no entry | Always available; always opens the embedded merged changelog, scrollable, Superherdr entries on top |
| Inherited upstream releases | Invisible | `Inherited from Herdr <version>` subsections under the Superherdr sections |
| Startup | Pending notes are kept available but never auto-open; popups come from the product-announcement system | Unchanged |
| Runtime data source | Config-directory JSON slot | Build-time `include_str!` of `CHANGELOG.md`; no network, no cwd dependence |

### Decisions

- One merged document instead of separate theirs/ours tabs: the split is expressed as `###` section headings, the heading level the existing renderer styles; no new view state.
- The menu action opens the history view in all states. Badge and label logic is presentation-only and out of scope; the stored slot's existing roles (update preview plumbing, fake-preview helper) are unchanged.
- `###` for inherited headings keeps `extract_version_section` behavior intact (it terminates on `## [`), with a regression test proving a version extraction excludes a following inherited section.
- The view omits `Unreleased` sections and starts at the newest released Superherdr section.
- `CHANGELOG.md` is Superherdr-owned (`.upstream-sync/ours`); each upstream sync must add the new inherited section as a documented manual porting step (sync report already lists upstream edits to ours files).

## Alternatives and drawbacks

- Keep current behavior: installed users stay unable to see release history, and a stale 0.9.0 body misleads.
- Fetch history at runtime from GitHub: adds a network dependency and failure modes for a static document.
- Separate theirs/ours tabs: duplicates view machinery for no extra information.
- Auto-open on startup after an upgrade: rejected; today's behavior deliberately never auto-opens release notes, and this change preserves that.

## Impact

- UI: What's New overlay content and menu availability; no other client surface.
- CLI, socket API, persistence: none; the `release-notes.json` format is unchanged.
- Compatibility with Herdr clients and plugins: none touched; presentation-only, client-local.
- Upstream merges: `CHANGELOG.md` stays ours-owned; the sync checklist gains a required porting step.

## Verification and open questions

- `just test`: renderer tests over real changelog content (section order, `###` styling, Unreleased omitted); shell test opens the history view with no stored notes, survives empty and stale-note snapshots, closes without endpoint requests, including the mobile activation path; regression test that version extraction excludes a following inherited section.
- Manual: in an installed build outside a checkout, the menu always shows "what's new", and it renders the current Superherdr release on top with inherited 0.9.1–0.9.3 sections below.
- Open questions: none blocking.
