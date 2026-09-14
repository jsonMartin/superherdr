# superherdr-focus Specification

## Purpose
Let each client narrow its view to one project or workspace without changing other clients or running work. Focus is client-local presentation state.

## Requirements

### Requirement: Client-local Focus
A client SHALL be able to focus a project or a workspace, and SHALL show only the focused work and its agents. Focus SHALL NOT change what other clients show, and SHALL NOT stop, pause or detach any process. A project SHALL be an existing worktree group on one endpoint and session; a workspace without a group SHALL be focused individually.

#### Scenario: Focus on one client
- **WHEN** a user chooses Focus for a workspace on one client
- **THEN** that client SHALL show only the workspace and its agents, and another attached client SHALL keep its own view

### Requirement: Focus controls
Workspace context menus SHALL offer Focus. The expanded sidebar header SHALL show `Focused: {name}` with a control that clears Focus, and the sidebar footer SHALL offer Focus without using an extra terminal row.

#### Scenario: Clear Focus
- **WHEN** a user clears Focus from the header
- **THEN** the client SHALL show all non-snoozed work again, and existing snoozes SHALL remain in effect
