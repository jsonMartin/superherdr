use super::*;
use ratatui::{
    text::Line,
    widgets::{Paragraph, Widget},
};

fn workspace_selection_background(palette: &Palette) -> ratatui::style::Color {
    if palette.selection_bg == ratatui::style::Color::Reset {
        palette.active_row_bg
    } else {
        palette.selection_bg
    }
}

pub(in crate::client::shell) fn workspace_active_background(
    palette: &Palette,
    navigating: bool,
) -> ratatui::style::Color {
    // The fallback cursor shares the active-row color; only fill the cursor while navigating.
    if navigating && palette.selection_bg == ratatui::style::Color::Reset {
        palette.sidebar_bg
    } else {
        palette.active_row_bg
    }
}

pub(in crate::client::shell) fn collapsed_sidebar_sections(
    area: Rect,
) -> (Rect, Option<u16>, Rect) {
    let content = Rect::new(area.x, area.y, area.width.saturating_sub(1), area.height);
    if content.is_empty() {
        return (Rect::default(), None, Rect::default());
    }
    // The bottom row hosts the » expand toggle; workspace and agent rows must stay above
    // it at every height, including the height < 7 all-workspace fallback.
    let body_height = content.height.saturating_sub(1);
    if body_height == 0 {
        return (Rect::default(), None, Rect::default());
    }
    if body_height < 7 {
        return (
            Rect::new(content.x, content.y, content.width, body_height),
            None,
            Rect::default(),
        );
    }
    let workspace_height = body_height.div_ceil(2);
    let divider_y = content.y + workspace_height;
    let detail_height = body_height.saturating_sub(workspace_height + 1);
    (
        Rect::new(content.x, content.y, content.width, workspace_height),
        Some(divider_y),
        Rect::new(content.x, divider_y + 1, content.width, detail_height),
    )
}

pub(crate) fn render_collapsed_sidebar(
    buffer: &mut Buffer,
    area: Rect,
    snapshot: &ClientShellSnapshot,
    config: &ClientShellConfig,
    selected_workspace_id: Option<&str>,
    focus_scope: Option<&ClientFocusScope>,
    snooze_state: Option<&crate::api::schema::WorkspaceSnoozeState>,
    pinned_workspaces: &HashSet<&str>,
    pinned_panes: &HashSet<&str>,
    hits: &mut ShellHitMap,
) {
    let palette = &config.palette;
    let selection_background = workspace_selection_background(palette);
    let active_background = workspace_active_background(palette, selected_workspace_id.is_some());
    render_sidebar_background(buffer, area, palette);
    let (workspace_area, divider_y, detail_area) = collapsed_sidebar_sections(area);
    let visible_workspaces = super::focus_snooze::visible_workspace_ids(
        focus_scope,
        snooze_state,
        &ClientEndpointId::Local,
        Some(snapshot.boot_id.as_str()),
        &snapshot.workspaces,
    );
    let entries = pinned_flat_entries(snapshot, pinned_workspaces, |workspace| {
        visible_workspaces.contains(&workspace.workspace_id)
    });
    for (index, entry) in entries
        .iter()
        .take(workspace_area.height as usize)
        .enumerate()
    {
        let Some(workspace) = snapshot.workspaces.get(entry.index) else {
            continue;
        };
        let rect = Rect::new(
            workspace_area.x,
            workspace_area.y + index as u16,
            workspace_area.width,
            1,
        );
        let selected = selected_workspace_id == Some(workspace.workspace_id.as_str());
        if selected {
            buffer.set_style(rect, Style::default().bg(selection_background));
        } else if workspace.focused {
            buffer.set_style(rect, Style::default().bg(active_background));
        }
        let number_style = if selected {
            Style::default()
                .fg(palette.overlay1)
                .bg(selection_background)
        } else if workspace.focused {
            Style::default().fg(palette.text).bg(active_background)
        } else {
            Style::default().fg(palette.overlay0)
        };
        put_text(
            buffer,
            rect.x,
            rect.y,
            rect.width.min(2),
            &format!("{:<2}", index + 1),
            number_style,
        );
        let status = workspace.agent_status;
        let status_style = Style::default().fg(status_color(status, palette));
        let (icon, pin_prefix) = status_indicator(
            pinned_workspaces.contains(workspace.workspace_id.as_str()),
            status,
            config.status_indicators,
            status_style,
        );
        let mut status_x = rect.x.saturating_add(2);
        if pin_prefix.is_some() {
            let width = rect.right().saturating_sub(status_x);
            put_text(
                buffer,
                status_x,
                rect.y,
                width,
                crate::ui::PIN_GLYPH,
                status_style,
            );
            status_x = status_x.saturating_add(display_width(crate::ui::PIN_GLYPH).min(width));
        }
        put_text(
            buffer,
            status_x,
            rect.y,
            rect.right().saturating_sub(status_x),
            icon.0,
            icon.1,
        );
        hits.workspaces.push(WorkspaceHit {
            rect,
            endpoint_id: ClientEndpointId::Local,
            workspace_id: workspace.workspace_id.clone(),
            indented: false,
            group_toggle: None,
        });
    }

    if let Some(divider_y) = divider_y {
        put_text(
            buffer,
            workspace_area.x,
            divider_y,
            workspace_area.width,
            &"─".repeat(workspace_area.width as usize),
            Style::default().fg(palette.surface_dim),
        );
    }

    let agent_workspaces = super::agent_scope::visible_agent_workspace_ids(
        snapshot,
        focus_scope,
        snooze_state,
        &ClientEndpointId::Local,
        config.top_level_agents,
    );
    // The » toggle row is already excluded by collapsed_sidebar_sections.
    let detail_content = detail_area;
    for (index, pane_id) in super::ordered_agent_pane_ids_with_filter(
        snapshot,
        config.agent_panel_sort,
        pinned_panes,
        |agent| agent_workspaces.contains(&agent.workspace_id),
    )
    .into_iter()
    .take(detail_content.height as usize)
    .enumerate()
    {
        let Some(agent) = snapshot
            .agents
            .iter()
            .find(|agent| agent.pane_id == pane_id)
        else {
            continue;
        };
        let rect = Rect::new(
            detail_content.x,
            detail_content.y + index as u16,
            detail_content.width,
            1,
        );
        if agent.focused {
            buffer.set_style(rect, Style::default().bg(palette.active_row_bg));
        }
        put_text(
            buffer,
            rect.x,
            rect.y,
            rect.width.min(2),
            &format!("{:<2}", index + 1),
            Style::default().fg(if agent.focused {
                palette.text
            } else {
                palette.overlay0
            }),
        );
        let status_style = Style::default().fg(status_color(agent.agent_status, palette));
        let (icon, pin_prefix) = status_indicator(
            pinned_panes.contains(agent.pane_id.as_str()),
            agent.agent_status,
            config.status_indicators,
            status_style,
        );
        let mut status_x = rect.x.saturating_add(2);
        if pin_prefix.is_some() {
            let width = rect.right().saturating_sub(status_x);
            put_text(
                buffer,
                status_x,
                rect.y,
                width,
                crate::ui::PIN_GLYPH,
                status_style,
            );
            status_x = status_x.saturating_add(display_width(crate::ui::PIN_GLYPH).min(width));
        }
        put_text(
            buffer,
            status_x,
            rect.y,
            rect.right().saturating_sub(status_x),
            icon.0,
            icon.1,
        );
        hits.agents.push((rect, pane_id));
    }
    hits.sidebar_toggle = if area.is_empty() || workspace_area.width == 0 {
        Rect::default()
    } else {
        Rect::new(
            workspace_area.x + workspace_area.width / 2,
            area.bottom().saturating_sub(1),
            1,
            1,
        )
    };
    put_text(
        buffer,
        hits.sidebar_toggle.x,
        hits.sidebar_toggle.y,
        hits.sidebar_toggle.width,
        "»",
        if super::super::global_menu::global_menu_attention(snapshot) {
            Style::default()
                .fg(palette.accent)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(palette.overlay0)
        },
    );
}

pub(crate) fn render_sidebar(
    buffer: &mut Buffer,
    area: Rect,
    snapshot: &ClientShellSnapshot,
    config: &ClientShellConfig,
    state: &mut ShellRenderState<'_>,
    hits: &mut ShellHitMap,
) {
    let palette = &config.palette;
    render_sidebar_background(buffer, area, palette);
    hits.sidebar_divider = if area.is_empty() {
        Rect::default()
    } else {
        Rect::new(area.right().saturating_sub(1), area.y, 1, area.height)
    };
    let (workspace_area, mut detail_area) =
        crate::ui::expanded_sidebar_sections(area, state.sidebar_section_split);
    // Reserve the bottom row for the focus/snooze footer so no clickable agent row
    // sits underneath it.
    detail_area.height = detail_area.height.saturating_sub(1);
    hits.sidebar_section_divider =
        crate::ui::sidebar_section_divider_rect(area, state.sidebar_section_split);
    super::recovery_bar::render_focus_header(
        buffer,
        workspace_area,
        " spaces",
        config,
        state,
        hits,
    );

    let visible_workspaces = super::focus_snooze::visible_workspace_ids(
        state.focus_scope,
        state.snooze_state,
        state.active_endpoint_id,
        Some(snapshot.boot_id.as_str()),
        &snapshot.workspaces,
    );
    let (pinned_workspaces, pinned_panes) =
        active_pin_sets(state.endpoints, state.active_endpoint_id);
    let entries = workspace_entries_with_filter(
        snapshot,
        state.collapsed_groups,
        &pinned_workspaces,
        |workspace| visible_workspaces.contains(&workspace.workspace_id),
    );
    let body = Rect::new(
        workspace_area.x,
        workspace_area.y.saturating_add(WORKSPACE_HEADER_ROWS),
        workspace_area.width,
        workspace_area
            .height
            .saturating_sub(WORKSPACE_HEADER_ROWS + 1),
    );
    hits.workspace_body = body;
    let row_heights = entries
        .iter()
        .map(|entry| {
            snapshot
                .workspaces
                .get(entry.index)
                .map(|workspace| {
                    workspace_rows(
                        workspace,
                        displayed_workspace_status(snapshot, workspace, state.collapsed_groups),
                        entry.indented,
                        &config.spaces,
                    )
                    .len()
                    .max(1)
                    .min(u16::MAX as usize) as u16
                })
                .unwrap_or(1)
        })
        .collect::<Vec<_>>();
    let gaps = entries
        .iter()
        .enumerate()
        .map(|(index, _)| {
            entries
                .get(index + 1)
                .map_or(0, |next| u16::from(!next.indented) * config.spaces.row_gap)
        })
        .collect::<Vec<_>>();
    let mut metrics = super::scroll::list_scroll_metrics(
        &row_heights,
        &gaps,
        body.height,
        *state.workspace_scroll,
    );
    if !body.is_empty() && std::mem::take(state.reveal_focused_workspace) {
        if let Some(target) = entries
            .iter()
            .position(|entry| snapshot.workspaces[entry.index].focused)
        {
            *state.workspace_scroll = super::scroll::list_scroll_start_to_reveal(
                &row_heights,
                &gaps,
                body.height,
                *state.workspace_scroll,
                target,
            );
            metrics = super::scroll::list_scroll_metrics(
                &row_heights,
                &gaps,
                body.height,
                *state.workspace_scroll,
            );
        }
    }
    hits.workspace_max_scroll = metrics.max_offset_from_bottom;
    hits.workspace_scroll_metrics = Some(metrics);
    *state.workspace_scroll = metrics
        .max_offset_from_bottom
        .saturating_sub(metrics.offset_from_bottom);
    let show_scrollbar = metrics.max_offset_from_bottom > 0 && body.width > 1;
    let content_width = body.width.saturating_sub(u16::from(show_scrollbar));
    let mut y = body.y;
    for (entry_position, entry) in entries.iter().enumerate().skip(*state.workspace_scroll) {
        let Some(workspace) = snapshot.workspaces.get(entry.index) else {
            continue;
        };
        let status = displayed_workspace_status(snapshot, workspace, state.collapsed_groups);
        let rows = workspace_rows(workspace, status, entry.indented, &config.spaces);
        let row_height = (rows.len().max(1).min(u16::MAX as usize) as u16).min(body.height);
        if y.saturating_add(row_height) > body.bottom() {
            break;
        }
        let rect = Rect::new(body.x, y, content_width, row_height);
        let selected = state.selected_workspace_id.is_some_and(|target| {
            target.matches(state.active_endpoint_id, &workspace.workspace_id)
        });
        let dragged = state.dragged_workspace_id == Some(workspace.workspace_id.as_str());
        if selected {
            buffer.set_style(rect, Style::default().bg(palette.selection_bg));
        } else if dragged {
            buffer.set_style(rect, Style::default().bg(palette.surface1));
        } else if workspace.focused {
            buffer.set_style(rect, Style::default().bg(palette.active_row_bg));
        }
        render_workspace_rows(
            buffer,
            rect,
            status,
            config.status_indicators,
            pinned_workspaces.contains(workspace.workspace_id.as_str()),
            entry,
            rows,
            workspace.focused,
            selected,
            state.selected_workspace_id.is_some(),
            dragged,
            palette,
        );
        let group_toggle = render_parent_group_toggle(
            buffer,
            rect,
            snapshot,
            entry.index,
            state.collapsed_groups,
            palette,
        );
        hits.workspaces.push(WorkspaceHit {
            rect,
            endpoint_id: ClientEndpointId::Local,
            workspace_id: workspace.workspace_id.clone(),
            indented: entry.indented,
            group_toggle,
        });
        let gap = entries
            .get(entry_position + 1)
            .map_or(0, |next| u16::from(!next.indented) * config.spaces.row_gap);
        y = y.saturating_add(row_height + gap);
    }

    if show_scrollbar {
        let track = Rect::new(body.right().saturating_sub(1), body.y, 1, body.height);
        hits.workspace_scrollbar = track;
        super::scroll::render_list_scrollbar(buffer, track, metrics, palette);
    }

    if let Some(row) = state.workspace_drop_indicator_row.filter(|row| {
        *row >= workspace_area.y.saturating_add(1)
            && *row < workspace_area.bottom().saturating_sub(1)
    }) {
        put_text(
            buffer,
            body.x,
            row,
            body.width,
            &"─".repeat(body.width as usize),
            Style::default().fg(palette.accent),
        );
    }

    let footer_y = workspace_area.bottom().saturating_sub(1);
    if config.mouse_capture {
        hits.new_workspace = Rect::new(
            workspace_area.x,
            footer_y,
            5.min(workspace_area.width),
            u16::from(workspace_area.height > 0),
        );
        put_text(
            buffer,
            workspace_area.x,
            footer_y,
            workspace_area.width,
            " new",
            Style::default().fg(palette.overlay0),
        );
        let attention = super::super::global_menu::global_menu_attention(snapshot);
        let launcher_width = if attention { 8 } else { 6 }.min(workspace_area.width);
        hits.global_launcher = Rect::new(
            workspace_area.right().saturating_sub(launcher_width),
            footer_y,
            launcher_width,
            1,
        );
        if attention {
            let start_x = workspace_area.right().saturating_sub(6);
            put_text(
                buffer,
                start_x,
                footer_y,
                2,
                "● ",
                Style::default()
                    .fg(palette.accent)
                    .add_modifier(Modifier::BOLD),
            );
            put_text(
                buffer,
                start_x.saturating_add(2),
                footer_y,
                4,
                "menu",
                Style::default().fg(palette.overlay0),
            );
        } else {
            put_right_text(
                buffer,
                workspace_area,
                footer_y,
                "menu",
                Style::default().fg(palette.overlay0),
            );
        }
    }

    let agent_workspaces = super::agent_scope::visible_agent_workspace_ids(
        snapshot,
        state.focus_scope,
        state.snooze_state,
        state.active_endpoint_id,
        config.top_level_agents,
    );
    super::render_agent_panel_with_filter(
        buffer,
        detail_area,
        snapshot,
        config,
        state.agent_scroll,
        hits,
        &pinned_panes,
        |agent| agent_workspaces.contains(&agent.workspace_id),
    );

    let footer = super::recovery_bar::SidebarFooter::for_endpoints(
        state.focus_scope.is_some(),
        state.endpoints,
    );
    super::recovery_bar::render_sidebar_footer(buffer, area, config, &footer, hits);

    hits.sidebar_toggle = Rect::new(
        area.right().saturating_sub(2),
        area.bottom().saturating_sub(1),
        u16::from(area.width > 1),
        u16::from(area.height > 0),
    );
    put_text(
        buffer,
        hits.sidebar_toggle.x,
        hits.sidebar_toggle.y,
        hits.sidebar_toggle.width,
        "«",
        Style::default().fg(palette.overlay0),
    );
}

/// Sidebar ordering works on blocks: a parent workspace with its linked
/// worktree children, a standalone workspace, or an orphan worktree. A block is
/// pinned when its top workspace is pinned; a pinned linked worktree under an
/// unpinned parent escapes its group — even a collapsed one — and forms its own
/// unindented pinned block. Pinned blocks come first in their existing relative
/// order, followed by unpinned blocks in their existing order.
pub(crate) fn workspace_entries_with_filter(
    snapshot: &ClientShellSnapshot,
    collapsed_groups: &HashSet<String>,
    pinned: &HashSet<&str>,
    filter: impl Fn(&crate::protocol::ClientShellWorkspace) -> bool,
) -> Vec<WorkspaceEntry> {
    let mut members = HashMap::<&str, Vec<usize>>::new();
    for (index, workspace) in snapshot.workspaces.iter().enumerate() {
        if let Some(worktree) = &workspace.worktree {
            members.entry(&worktree.key).or_default().push(index);
        }
    }
    let grouped = members
        .iter()
        .filter(|(_, indices)| {
            indices.iter().any(|index| {
                snapshot.workspaces[*index]
                    .worktree
                    .as_ref()
                    .is_some_and(|worktree| worktree.is_linked_worktree)
            }) && indices.iter().any(|index| {
                snapshot.workspaces[*index]
                    .worktree
                    .as_ref()
                    .is_some_and(|worktree| !worktree.is_linked_worktree)
            })
        })
        .map(|(key, _)| *key)
        .collect::<HashSet<_>>();
    let mut emitted = HashSet::<&str>::new();
    // Pinned blocks stay ahead of unpinned blocks, each in encounter order.
    // Appending into two vectors avoids a block list and a second partition pass.
    let mut pinned_entries = Vec::new();
    let mut unpinned_entries = Vec::new();
    fn push_block(
        pinned_entries: &mut Vec<WorkspaceEntry>,
        unpinned_entries: &mut Vec<WorkspaceEntry>,
        pinned_block: bool,
        entries: Vec<WorkspaceEntry>,
    ) {
        if entries.is_empty() {
            return;
        }
        if pinned_block {
            pinned_entries.extend(entries);
        } else {
            unpinned_entries.extend(entries);
        }
    }
    for (index, workspace) in snapshot.workspaces.iter().enumerate() {
        let Some(worktree) = workspace
            .worktree
            .as_ref()
            .filter(|worktree| grouped.contains(worktree.key.as_str()))
        else {
            if filter(workspace) {
                let pinned_block = pinned.contains(workspace.workspace_id.as_str());
                push_block(
                    &mut pinned_entries,
                    &mut unpinned_entries,
                    pinned_block,
                    vec![WorkspaceEntry {
                        index,
                        indented: false,
                        last_child: false,
                    }],
                );
            }
            continue;
        };
        if !emitted.insert(&worktree.key) {
            continue;
        }
        let Some(group_members) = members.get(worktree.key.as_str()) else {
            continue;
        };
        let parents = group_members
            .iter()
            .copied()
            .filter(|member| {
                snapshot.workspaces[*member]
                    .worktree
                    .as_ref()
                    .is_some_and(|worktree| !worktree.is_linked_worktree)
            })
            .collect::<Vec<_>>();
        let visible_parents = parents
            .iter()
            .copied()
            .filter(|parent| filter(&snapshot.workspaces[*parent]))
            .collect::<Vec<_>>();
        // The block is pinned when its top workspace is pinned.
        let block_pinned = visible_parents.first().is_some_and(|parent| {
            pinned.contains(snapshot.workspaces[*parent].workspace_id.as_str())
        });
        // A pinned linked worktree under an unpinned parent leaves the group
        // and renders unindented in the pinned section.
        let escapes = |member: &usize| {
            !block_pinned
                && pinned.contains(snapshot.workspaces[*member].workspace_id.as_str())
                && filter(&snapshot.workspaces[*member])
        };
        let mut escaped_children = Vec::new();
        for member in group_members.iter().copied() {
            let is_linked = snapshot.workspaces[member]
                .worktree
                .as_ref()
                .is_some_and(|worktree| worktree.is_linked_worktree);
            if is_linked && escapes(&member) {
                escaped_children.push(WorkspaceEntry {
                    index: member,
                    indented: false,
                    last_child: false,
                });
            }
        }
        for parent in visible_parents.iter().copied() {
            push_block(
                &mut pinned_entries,
                &mut unpinned_entries,
                block_pinned,
                vec![WorkspaceEntry {
                    index: parent,
                    indented: false,
                    last_child: false,
                }],
            );
        }
        if visible_parents.is_empty() {
            // A hidden repository parent must not erase its visible linked worktrees.
            let children = group_members
                .iter()
                .copied()
                .filter(|member| {
                    filter(&snapshot.workspaces[*member])
                        && snapshot.workspaces[*member]
                            .worktree
                            .as_ref()
                            .is_some_and(|worktree| worktree.is_linked_worktree)
                })
                .filter(|member| !escapes(member))
                .collect::<Vec<_>>();
            for child in children {
                push_block(
                    &mut pinned_entries,
                    &mut unpinned_entries,
                    false,
                    vec![WorkspaceEntry {
                        index: child,
                        indented: false,
                        last_child: false,
                    }],
                );
            }
        } else if collapsed_groups.contains(&worktree.key) {
            if let Some(active) = group_members.iter().copied().find(|member| {
                let workspace = &snapshot.workspaces[*member];
                workspace.focused
                    && filter(workspace)
                    && workspace
                        .worktree
                        .as_ref()
                        .is_some_and(|worktree| worktree.is_linked_worktree)
                    && !escapes(member)
            }) {
                push_block(
                    &mut pinned_entries,
                    &mut unpinned_entries,
                    block_pinned,
                    vec![WorkspaceEntry {
                        index: active,
                        indented: true,
                        last_child: true,
                    }],
                );
            }
        } else {
            let children = group_members
                .iter()
                .copied()
                .filter(|member| {
                    filter(&snapshot.workspaces[*member])
                        && snapshot.workspaces[*member]
                            .worktree
                            .as_ref()
                            .is_some_and(|worktree| worktree.is_linked_worktree)
                })
                .filter(|member| !escapes(member))
                .collect::<Vec<_>>();
            let mut block = Vec::new();
            for (child_index, child) in children.iter().enumerate() {
                block.push(WorkspaceEntry {
                    index: *child,
                    indented: true,
                    last_child: child_index + 1 == children.len(),
                });
            }
            push_block(
                &mut pinned_entries,
                &mut unpinned_entries,
                block_pinned,
                block,
            );
        }
        if !escaped_children.is_empty() {
            push_block(
                &mut pinned_entries,
                &mut unpinned_entries,
                true,
                escaped_children,
            );
        }
    }
    pinned_entries.append(&mut unpinned_entries);
    pinned_entries
}

pub(crate) fn workspace_entries(
    snapshot: &ClientShellSnapshot,
    collapsed_groups: &HashSet<String>,
    pinned: &HashSet<&str>,
) -> Vec<WorkspaceEntry> {
    workspace_entries_with_filter(snapshot, collapsed_groups, pinned, |_| true)
}

/// Compact strips keep snapshot order until a visible workspace is pinned.
/// Existing navigation tests require that ungrouped snapshot order, including
/// collapsed-group children. A visible pin switches the strip onto the same
/// block order as the expanded sidebar, with children still shown.
pub(crate) fn pinned_flat_entries(
    snapshot: &ClientShellSnapshot,
    pinned: &HashSet<&str>,
    filter: impl Fn(&crate::protocol::ClientShellWorkspace) -> bool,
) -> Vec<WorkspaceEntry> {
    let any_pinned = snapshot
        .workspaces
        .iter()
        .any(|workspace| filter(workspace) && pinned.contains(workspace.workspace_id.as_str()));
    if any_pinned {
        workspace_entries_with_filter(snapshot, &HashSet::new(), pinned, filter)
    } else {
        snapshot
            .workspaces
            .iter()
            .enumerate()
            .filter(|(_, workspace)| filter(workspace))
            .map(|(index, _)| WorkspaceEntry {
                index,
                indented: false,
                last_child: false,
            })
            .collect()
    }
}

/// The order a sidebar actually draws for one endpoint.
///
/// Compact strips use snapshot order until a visible workspace is pinned, then
/// the pinned block order with children kept visible. Expanded sidebars use
/// the real collapsed-group set. Navigation across machines uses this same
/// builder so next/previous cannot walk a row the sidebar hid.
pub(crate) fn rendered_workspace_entries(
    snapshot: &ClientShellSnapshot,
    collapsed_groups: &HashSet<String>,
    pinned: &HashSet<&str>,
    sidebar_collapsed: bool,
    filter: impl Fn(&crate::protocol::ClientShellWorkspace) -> bool,
) -> Vec<WorkspaceEntry> {
    if sidebar_collapsed {
        pinned_flat_entries(snapshot, pinned, filter)
    } else {
        workspace_entries_with_filter(snapshot, collapsed_groups, pinned, filter)
    }
}

pub(super) fn active_pin_sets<'a>(
    endpoints: &'a [ClientShellEndpoint],
    active_endpoint_id: &ClientEndpointId,
) -> (HashSet<&'a str>, HashSet<&'a str>) {
    let pin = endpoints
        .iter()
        .find(|endpoint| &endpoint.endpoint_id == active_endpoint_id)
        .and_then(|endpoint| endpoint.pin_state.as_ref());
    let workspaces = pin
        .map(|pin| pin.workspace_ids.iter().map(String::as_str).collect())
        .unwrap_or_default();
    let panes = pin
        .map(|pin| pin.pane_ids.iter().map(String::as_str).collect())
        .unwrap_or_default();
    (workspaces, panes)
}

pub(super) fn parent_group_key(snapshot: &ClientShellSnapshot, index: usize) -> Option<String> {
    let workspace = snapshot.workspaces.get(index)?;
    let worktree = workspace.worktree.as_ref()?;
    if worktree.is_linked_worktree {
        return None;
    }
    snapshot
        .workspaces
        .iter()
        .any(|candidate| {
            candidate.worktree.as_ref().is_some_and(|candidate| {
                candidate.key == worktree.key && candidate.is_linked_worktree
            })
        })
        .then(|| worktree.key.clone())
}

pub(in crate::client::shell) fn workspace_close_is_group(
    snapshot: &ClientShellSnapshot,
    workspace: &ClientShellWorkspace,
) -> bool {
    let Some(worktree) = workspace
        .worktree
        .as_ref()
        .filter(|worktree| !worktree.is_linked_worktree)
    else {
        return false;
    };
    let mut has_child = false;
    for member in &snapshot.workspaces {
        if member.workspace_id == workspace.workspace_id {
            continue;
        }
        if let Some(candidate) = member
            .worktree
            .as_ref()
            .filter(|candidate| candidate.key == worktree.key)
        {
            if !candidate.is_linked_worktree {
                return false;
            }
            has_child = true;
        }
    }
    has_child
}

pub(in crate::client::shell) fn render_parent_group_toggle(
    buffer: &mut Buffer,
    workspace_rect: Rect,
    snapshot: &ClientShellSnapshot,
    workspace_index: usize,
    collapsed_groups: &HashSet<String>,
    palette: &Palette,
) -> Option<(Rect, String)> {
    let key = parent_group_key(snapshot, workspace_index)?;
    let toggle = Rect::new(
        workspace_rect.right().saturating_sub(1),
        workspace_rect.y,
        1,
        1,
    );
    put_text(
        buffer,
        toggle.x,
        toggle.y,
        toggle.width,
        if collapsed_groups.contains(&key) {
            "▸"
        } else {
            "▾"
        },
        Style::default().fg(palette.accent),
    );
    Some((toggle, key))
}

pub(in crate::client::shell) fn displayed_workspace_status(
    snapshot: &ClientShellSnapshot,
    workspace: &ClientShellWorkspace,
    collapsed_groups: &HashSet<String>,
) -> crate::api::schema::AgentStatus {
    let Some(worktree) = workspace
        .worktree
        .as_ref()
        .filter(|worktree| !worktree.is_linked_worktree)
    else {
        return workspace.agent_status;
    };
    if !collapsed_groups.contains(&worktree.key) {
        return workspace.agent_status;
    }
    snapshot
        .workspaces
        .iter()
        .filter(|candidate| {
            candidate.worktree.as_ref().is_some_and(|member| {
                member.key == worktree.key
                    && (member.is_linked_worktree
                        || candidate.workspace_id == workspace.workspace_id)
            })
        })
        .map(|candidate| candidate.agent_status)
        .max_by_key(|status| status_priority(*status))
        .unwrap_or(workspace.agent_status)
}

pub(in crate::client::shell) fn workspace_rows(
    workspace: &ClientShellWorkspace,
    status: crate::api::schema::AgentStatus,
    indented: bool,
    config: &SpacesSidebarConfig,
) -> Vec<Vec<crate::ui::ResolvedToken>> {
    let label = if indented && !workspace.custom_label {
        workspace
            .branch
            .as_deref()
            .and_then(|branch| branch.strip_prefix("worktree/").or(Some(branch)))
            .unwrap_or(&workspace.label)
    } else {
        &workspace.label
    };
    let token_values = workspace.tokens.iter().cloned().collect::<HashMap<_, _>>();
    crate::ui::sidebar_space_rows(
        config,
        crate::ui::SpaceTokenContext {
            workspace: label,
            branch: workspace.branch.as_deref(),
            state_text: status_text(status),
            ahead_behind: workspace.git_ahead_behind,
            tokens: &token_values,
            suppress_git_details: indented,
        },
    )
}

// `pinned` is one paint flag on top of the existing row inputs. A struct would
// only exist to satisfy the argument limit.
#[allow(clippy::too_many_arguments)]
pub(in crate::client::shell) fn render_workspace_rows(
    buffer: &mut Buffer,
    area: Rect,
    status: crate::api::schema::AgentStatus,
    indicators: crate::config::StatusIndicatorStyle,
    pinned: bool,
    entry: &WorkspaceEntry,
    rows: Vec<Vec<crate::ui::ResolvedToken>>,
    focused: bool,
    selected: bool,
    navigating: bool,
    dragged: bool,
    palette: &Palette,
) {
    for (row_index, row) in rows.iter().enumerate() {
        let y = area.y + row_index as u16;
        if y >= area.bottom() {
            break;
        }
        let mut x = area.x;
        if entry.indented {
            let prefix = if row_index == 0 {
                if entry.last_child {
                    "   └─ "
                } else {
                    "   ├─ "
                }
            } else if entry.last_child {
                "        "
            } else {
                "   │    "
            };
            x = put_segment(
                buffer,
                x,
                y,
                area.right(),
                prefix,
                Style::default().fg(palette.overlay0),
            );
        } else if row_index == 0 {
            x = x.saturating_add(1);
        } else {
            x = x.saturating_add(3);
        }
        let highlighted = focused || dragged;
        let workspace_style = Style::default()
            .fg(if highlighted {
                palette.text
            } else {
                palette.subtext0
            })
            .add_modifier(if highlighted {
                Modifier::BOLD
            } else {
                Modifier::empty()
            });
        let secondary_style = Style::default().fg(if focused {
            palette.mauve
        } else {
            palette.overlay0
        });
        let status_style = Style::default().fg(status_color(status, palette));
        let (icon, pin_prefix) = status_indicator(pinned, status, indicators, status_style);
        let spans = crate::ui::resolved_token_spans(
            row,
            icon,
            pin_prefix,
            status_style,
            workspace_style,
            secondary_style,
            Style::default().fg(palette.overlay1),
            palette,
            area.right().saturating_sub(2).saturating_sub(x) as usize,
        );
        Paragraph::new(Line::from(spans)).render(
            Rect::new(x, y, area.right().saturating_sub(2).saturating_sub(x), 1),
            buffer,
        );
    }

    let background = if selected {
        Some(workspace_selection_background(palette))
    } else if dragged {
        Some(palette.surface1)
    } else if focused {
        Some(workspace_active_background(palette, navigating))
    } else {
        None
    };
    if let Some(background) = background {
        for y in area.y..area.bottom() {
            for x in area.x..area.right() {
                buffer[(x, y)].set_bg(background);
            }
        }
    }
}
