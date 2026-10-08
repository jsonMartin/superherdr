# superherdr-release-notes Specification

## Purpose
Describe how Superherdr presents its combined release history: the release-notes view renders the changelog embedded in the binary, released Superherdr sections first, inherited upstream sections below.

# superherdr-release-notes Specification

## Purpose
TBD - created by archiving change whats-new-full-history. Update Purpose after archive.

## Requirements

### Requirement: The What's New view shows the merged release history

The client SHALL offer a What's New view that renders the Superherdr `CHANGELOG.md` embedded in the binary, showing released Superherdr sections first (newest first, omitting `Unreleased` sections) followed by `Inherited from Herdr <version>` subsections, also newest first. The menu entry SHALL be available regardless of update-check or stored-notes state, and the action SHALL open this history view in all states.

#### Scenario: Installed build shows full history

- WHEN a user opens "what's new" from the global menu in an installed build outside a repository checkout
- THEN the view shows the newest released Superherdr section first, followed by inherited Herdr sections, scrollable, without filesystem or network access

#### Scenario: No release ever fetched

- WHEN the update checker has never saved stored notes, or the stored slot holds an old version
- THEN the menu entry is available and the history view opens with the merged changelog

#### Scenario: Unreleased entries are not shown

- WHEN the changelog contains an `Unreleased` section above released sections
- THEN the view omits it and starts at the newest released Superherdr section

#### Scenario: View survives client snapshots

- WHEN the history view is open and subsequent client snapshots carry no release notes, stored notes, or updates
- THEN the history view stays open and rendering, and closing it triggers no endpoint request

### Requirement: Startup and stored-slot behavior is unchanged

The client SHALL keep pending release notes available without auto-opening them at startup, SHALL keep the product-announcement popup system unchanged, and SHALL NOT auto-open the history view on launch. Stored-slot roles outside the What's New menu action (update preview plumbing, the fake-preview test helper) SHALL be unchanged.

#### Scenario: Pending notes never auto-open

- WHEN the app starts with pending release notes in the stored slot
- THEN nothing auto-opens; the notes stay available as today, and the history view is reachable only from the menu

### Requirement: Inherited sections stay extractable and styleable

Inherited sections SHALL use `###` headings (`### Inherited from Herdr <version>`), so version extraction that terminates on `## [` headings excludes them and the renderer styles them as sections.

#### Scenario: Version extraction excludes inherited sections

- WHEN a Superherdr version section is extracted from the changelog and an inherited section follows it
- THEN the extracted body excludes the inherited section
