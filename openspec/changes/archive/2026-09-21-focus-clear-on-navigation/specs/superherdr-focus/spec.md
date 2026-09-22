## ADDED Requirements

### Requirement: Focus clears on navigation away
The client SHALL clear Focus automatically when the server-focused workspace changes to a live workspace outside the focused scope, and SHALL clear a scope that no longer matches any workspace on its endpoint. Snooze-driven hiding SHALL NOT clear Focus.

#### Scenario: New workspace while focused
- **WHEN** a user creates a workspace with the keyboard shortcut while a project is focused
- **THEN** the client SHALL show the new workspace and clear Focus so the sidebar lists all work again

#### Scenario: Focus change outside the scope
- **WHEN** focus moves to a workspace outside the scope through any navigation path, including the CLI, another client, a close-refocus after deleting the focused workspace, or switching machines
- **THEN** the client SHALL clear Focus so the sidebar matches the presented workspace

#### Scenario: Stale scope
- **WHEN** the focused workspace closes or the endpoint's server reboots while Focus is active
- **THEN** the client SHALL clear Focus on the next snapshot

#### Scenario: Snooze keeps Focus
- **WHEN** the focused workspace becomes snoozed while Focus is active
- **THEN** the client SHALL keep Focus and move the presentation to a visible workspace inside the scope

### Requirement: Focus hides other machines
While Focus is active on one endpoint, the sidebar SHALL hide machines other than the focused one; clearing Focus SHALL restore them.

#### Scenario: Switch machines while focused
- **WHEN** a user navigates to another machine while Focus is active
- **THEN** the client SHALL clear Focus and show the full machine list with all sessions
