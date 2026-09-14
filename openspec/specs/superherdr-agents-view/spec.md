# superherdr-agents-view Specification

## Purpose
Let a client switch its Agents list between all agents and top-level agents, hiding agents in linked worktree workspaces whose parent is present. The mode is client-local presentation state.

## Requirements

### Requirement: Top-level Agents filter
In Top-level mode, the Agents list SHALL hide agents in a linked worktree workspace when the same endpoint has a non-linked parent workspace with the same worktree key. It SHALL keep parent agents, standalone workspaces, workspaces without grouping metadata, and orphan worktrees whose parent is absent. Identical worktree keys on different endpoints SHALL NOT form a group. A collapsed, focused-out or snoozed parent SHALL still count as present.

#### Scenario: Linked worktree with parent present
- **WHEN** Top-level mode is active and a linked worktree's parent workspace exists on the same endpoint
- **THEN** the worktree's agents SHALL be hidden from the Agents list

#### Scenario: Orphan worktree
- **WHEN** Top-level mode is active and a linked worktree's parent workspace is absent
- **THEN** the worktree's agents SHALL remain visible

### Requirement: Mode scope
The mode SHALL apply to the Agents list, its click targets, numbered agent shortcuts and next/previous agent navigation. It SHALL NOT change workspaces, tabs, panes, notifications, approvals or processes. Each client SHALL start in All, and changing the mode SHALL NOT persist or affect another client. The sidebar agents control, the global menu and the mobile switcher SHALL offer the toggle.

#### Scenario: Toggle on one client
- **WHEN** a user switches one client to Top-level
- **THEN** another client SHALL keep showing all agents, and restarting the client SHALL start in All
