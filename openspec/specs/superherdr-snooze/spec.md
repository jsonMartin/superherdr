# superherdr-snooze Specification

## Purpose
Hide a project or workspace on every client until a chosen wake time, without affecting running work. Snooze is shared state owned and persisted by the server.

## Requirements

### Requirement: Shared, persisted Snooze
Snoozing a project or workspace SHALL hide it on every client attached to the owning server until its wake time. The server SHALL persist snoozes across restarts. A project snooze SHALL cover all of the project's workspaces, including workspaces added later; a workspace snooze SHALL NOT cover its siblings.

#### Scenario: Snooze on one client
- **WHEN** a user snoozes a project on one client
- **THEN** every client attached to that server SHALL hide the project's workspaces until the wake time

#### Scenario: Wake a project with an explicit child snooze
- **WHEN** a project wakes while one of its workspaces has its own later snooze
- **THEN** that workspace SHALL stay hidden until its own wake time

### Requirement: Snooze never affects running work
Snooze SHALL NOT stop processes, mute notifications or acknowledge agent requests. When a client shows only snoozed work, it SHALL NOT forward terminal input to hidden panes.

#### Scenario: Agent runs while snoozed
- **WHEN** an agent in a snoozed workspace produces output or requests approval
- **THEN** the agent SHALL keep running and its notifications SHALL still be delivered

### Requirement: Snooze controls and wake
Workspace context menus SHALL offer Snooze workspace, and project roots SHALL also offer Snooze project. The picker SHALL show the absolute wake time before confirmation, and cancelling SHALL change nothing. At wake time the work SHALL reappear without taking focus, and the client's Focus SHALL be kept. The sidebar footer SHALL show the number of snoozes, and a management dialog SHALL list explicit and inherited snoozes with creation, remaining and wake times; inherited rows SHALL NOT count as separate snoozes.

#### Scenario: Timed wake
- **WHEN** a snooze reaches its wake time while a user works elsewhere
- **THEN** the work SHALL reappear in the sidebar and the user's current selection SHALL not change

#### Scenario: Stale management action
- **WHEN** another client changes a snooze while the management dialog is open and the user acts on the stale row
- **THEN** the action SHALL be rejected without changing other snoozes
