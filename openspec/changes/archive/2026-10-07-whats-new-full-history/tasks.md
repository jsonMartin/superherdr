# Tasks

## 1. Changelog restructure

- [x] 1.1 Restructure `CHANGELOG.md`: released Superherdr sections first; add `### Inherited from Herdr 0.9.1`, `0.9.2`, and `0.9.3` subsections ported from the upstream release bodies, newest first
- [x] 1.2 Add `CHANGELOG.md` to the Cargo `include` list so packaged builds embed it

## 2. View wiring

- [x] 2.1 Embed the changelog (`include_str!`) and expose it to the client shell
- [x] 2.2 Render the What's New view from the embedded changelog, omitting `Unreleased` sections; make the menu entry always available and the action always open the history view
- [x] 2.3 Leave startup, product-announcement, and stored-slot plumbing unchanged

## 3. Tests and docs

- [x] 3.1 Renderer tests over real changelog content: released-before-inherited order, `###` styling, `Unreleased` omitted
- [x] 3.2 Shell test: open the history view with no stored notes, apply empty and stale-note snapshots, close, assert no endpoint request; cover the mobile activation path
- [x] 3.3 Regression test: version extraction of a section followed by an inherited section excludes the inherited body
- [x] 3.4 Document the required changelog-graft step in the upstream sync checklist (CONTRIBUTING.md upstream section)
- [x] 3.5 Update `README.md` and `docs/next/` where release notes are described; add the user-facing CHANGELOG entry
- [ ] 3.6 Validate with `bunx --bun @fission-ai/openspec@1.13.0 validate whats-new-full-history --strict`; complete tasks, archive the change in the same PR, and get `just spec-check` green
