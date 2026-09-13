use super::*;

pub(super) fn snooze_visible(state: &ClientShellState) -> bool {
    state.endpoints.iter().any(|endpoint| {
        endpoint.snooze_state.as_ref().is_some_and(|snooze| {
            !snooze.records.is_empty()
                || !snooze.project_records.is_empty()
                || snooze.persistence.as_ref().is_some_and(|persistence| {
                    !persistence.records.is_empty() || persistence.notice.is_some()
                })
        })
    })
}

pub(super) fn is_visible(state: &ClientShellState) -> bool {
    state.focus_scope.is_some() || snooze_visible(state)
}

pub(super) fn snooze_count(state: &ClientShellState) -> usize {
    snooze_count_for(state, &state.active_endpoint_id)
}

pub(super) fn snooze_count_all(state: &ClientShellState) -> usize {
    state
        .endpoints
        .iter()
        .filter_map(|endpoint| endpoint.snooze_state.as_ref())
        .map(snooze_count_state)
        .sum()
}

pub(super) fn snooze_count_for(state: &ClientShellState, endpoint_id: &ClientEndpointId) -> usize {
    state
        .endpoints
        .iter()
        .find(|endpoint| &endpoint.endpoint_id == endpoint_id)
        .and_then(|endpoint| endpoint.snooze_state.as_ref())
        .map_or(0, snooze_count_state)
}

fn snooze_count_state(snooze: &crate::api::schema::WorkspaceSnoozeState) -> usize {
    snooze.persistence.as_ref().map_or_else(
        || {
            snooze
                .records
                .len()
                .saturating_add(snooze.project_records.len())
        },
        |persistence| persistence.records.len(),
    )
}

fn snooze_warning(state: &ClientShellState) -> bool {
    state.endpoints.iter().any(|endpoint| {
        endpoint
            .snooze_state
            .as_ref()
            .and_then(|snooze| snooze.persistence.as_ref())
            .is_some_and(|persistence| persistence.notice.is_some())
    })
}

fn focus_label(state: &ClientShellState) -> String {
    let Some(scope) = state.focus_scope.as_ref() else {
        return String::new();
    };
    let Some(snapshot) = state.snapshot.as_deref() else {
        return "unavailable".to_owned();
    };
    snapshot
        .workspaces
        .iter()
        .find(|workspace| {
            scope.matches_workspace(
                &state.active_endpoint_id,
                Some(snapshot.boot_id.as_str()),
                workspace,
            )
        })
        .map(|workspace| match scope {
            ClientFocusScope::Worktree { .. } => workspace.worktree.as_ref().map_or_else(
                || workspace.label.clone(),
                |worktree| worktree.label.clone(),
            ),
            ClientFocusScope::StandaloneWorkspace { .. } => workspace.label.clone(),
        })
        .unwrap_or_else(|| "unavailable".to_owned())
}

fn segment(prefix: &str, label: &str, suffix: &str, compact: &str, width: u16) -> String {
    let fixed = usize::from(super::render::display_width(prefix))
        .saturating_add(usize::from(super::render::display_width(suffix)));
    let width = usize::from(width);
    if width < fixed.saturating_add(usize::from(super::render::display_width(label))) {
        return crate::ui::truncate_end(compact, width);
    }
    let label_width = width.saturating_sub(fixed);
    format!(
        "{prefix}{}{suffix}",
        crate::ui::truncate_end(label, label_width)
    )
}

pub(super) fn render(
    buffer: &mut ratatui::buffer::Buffer,
    state: &ClientShellState,
    area: ratatui::layout::Rect,
) -> (ratatui::layout::Rect, ratatui::layout::Rect) {
    if area.is_empty() || !is_visible(state) {
        return (
            ratatui::layout::Rect::default(),
            ratatui::layout::Rect::default(),
        );
    }
    let both = state.focus_scope.is_some() && snooze_visible(state);
    let (focus_area, snooze_area) = if both {
        let left_width = area.width / 2;
        (
            ratatui::layout::Rect::new(area.x, area.y, left_width, area.height),
            ratatui::layout::Rect::new(
                area.x + left_width,
                area.y,
                area.width - left_width,
                area.height,
            ),
        )
    } else if state.focus_scope.is_some() {
        (area, ratatui::layout::Rect::default())
    } else {
        (ratatui::layout::Rect::default(), area)
    };
    let base = ratatui::style::Style::default()
        .fg(state.config.palette.text)
        .bg(state.config.palette.panel_bg);
    buffer.set_style(area, base);
    if !focus_area.is_empty() {
        let text = segment(
            " Focus: ",
            &focus_label(state),
            " · Clear",
            "Clear focus",
            focus_area.width,
        );
        super::render::put_text(
            buffer,
            focus_area.x,
            focus_area.y,
            focus_area.width,
            &text,
            base,
        );
    }
    if !snooze_area.is_empty() {
        let warning = snooze_warning(state);
        let count = snooze_count_all(state).to_string();
        let text = segment(
            if warning {
                " Snooze warning: "
            } else {
                " Snoozed: "
            },
            &count,
            " · Show",
            &format!("{}{count} · Show", if warning { "! " } else { "" }),
            snooze_area.width,
        );
        super::render::put_text(
            buffer,
            snooze_area.x,
            snooze_area.y,
            snooze_area.width,
            &text,
            base,
        );
    }
    if state.config.mouse_capture {
        (focus_area, snooze_area)
    } else {
        (
            ratatui::layout::Rect::default(),
            ratatui::layout::Rect::default(),
        )
    }
}

/// Focus/snooze controls hosted on the sidebar bottom row, next to the «/» toggle. They
/// replace the full-width recovery bar on layouts that show a sidebar so toggling Focus or
/// snoozes never changes the pane surface size. The focus toggle is permanent chrome —
/// 🎯 while a focus is active, 🌐 when all projects are shown — so the row is always
/// reserved from the agent panel projections.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct SidebarFooter {
    pub(super) focus_active: bool,
    pub(super) snooze_visible: bool,
    pub(super) snooze_count: usize,
    pub(super) snooze_warning: bool,
}

impl SidebarFooter {
    pub(super) fn for_active_snooze(
        focus_active: bool,
        snooze_state: Option<&crate::api::schema::WorkspaceSnoozeState>,
    ) -> Self {
        Self {
            focus_active,
            snooze_visible: snooze_state.is_some_and(|snooze| {
                !snooze.records.is_empty()
                    || !snooze.project_records.is_empty()
                    || snooze
                        .persistence
                        .as_ref()
                        .is_some_and(|persistence| {
                            !persistence.records.is_empty() || persistence.notice.is_some()
                        })
            }),
            snooze_count: snooze_state.map_or(0, snooze_count_state),
            snooze_warning: snooze_state
                .and_then(|snooze| snooze.persistence.as_ref())
                .is_some_and(|persistence| persistence.notice.is_some()),
        }
    }

    pub(super) fn for_endpoints(
        focus_active: bool,
        endpoints: &[ClientShellEndpoint],
    ) -> Self {
        let mut footer = Self::for_active_snooze(focus_active, None);
        for endpoint in endpoints {
            let part = Self::for_active_snooze(false, endpoint.snooze_state.as_ref());
            footer.snooze_visible |= part.snooze_visible;
            footer.snooze_count = footer.snooze_count.saturating_add(part.snooze_count);
            footer.snooze_warning |= part.snooze_warning;
        }
        footer
    }
}

fn compact_snooze_symbol(footer: &SidebarFooter) -> String {
    if footer.snooze_warning {
        "!".to_owned()
    } else if footer.snooze_count > 9 {
        "+".to_owned()
    } else {
        std::char::from_digit(footer.snooze_count.min(9) as u32, 10)
            .map(String::from)
            .unwrap_or_else(|| "+".to_owned())
    }
}

/// Draws the footer onto the sidebar's bottom row and registers its hit rects. `compact`
/// sidebars (width 4) get one-cell symbols; the «/» toggle cell is never overlapped.
pub(super) fn render_sidebar_footer(
    buffer: &mut Buffer,
    area: Rect,
    config: &ClientShellConfig,
    footer: &SidebarFooter,
    compact: bool,
    hits: &mut ShellHitMap,
) {
    if area.is_empty() || area.width < 2 || area.height == 0 {
        return;
    }
    let y = area.bottom().saturating_sub(1);
    let palette = &config.palette;
    let dim = Style::default().fg(palette.overlay0);
    let accent = Style::default()
        .fg(palette.accent)
        .add_modifier(Modifier::BOLD);
    if compact {
        let content_width = area.width.saturating_sub(1);
        let toggle_x = area.x + content_width / 2;
        super::render::put_text(
            buffer,
            area.x,
            y,
            1,
            if footer.focus_active { "◉" } else { "○" },
            if footer.focus_active { accent } else { dim },
        );
        if config.mouse_capture {
            hits.feature_clear_focus = Rect::new(area.x, y, 1, 1);
        }
        let snooze_x = toggle_x + 1;
        if snooze_x < area.x + content_width && (footer.snooze_visible || footer.snooze_warning) {
            let symbol = compact_snooze_symbol(footer);
            super::render::put_text(
                buffer,
                snooze_x,
                y,
                1,
                &symbol,
                if footer.snooze_warning {
                    Style::default().fg(palette.red)
                } else {
                    dim
                },
            );
            if config.mouse_capture {
                hits.feature_show_snoozed = Rect::new(snooze_x, y, 1, 1);
            }
        }
        return;
    }
    // Expanded footer: keep clear of the «/» toggle cell and the divider column.
    let limit = area.right().saturating_sub(2);
    let focus_text = if footer.focus_active { " 🎯" } else { " 🌐" };
    let focus_width = super::render::display_width(focus_text);
    if focus_width <= limit.saturating_sub(area.x) {
        super::render::put_text(
            buffer,
            area.x,
            y,
            focus_width,
            focus_text,
            if footer.focus_active { accent } else { dim },
        );
        if config.mouse_capture {
            hits.feature_clear_focus = Rect::new(area.x, y, focus_width, 1);
        }
    }
    if !footer.snooze_visible {
        return;
    }
    let snooze_x = area.x.saturating_add(focus_width);
    let available = limit.saturating_sub(snooze_x);
    // Shorten the label instead of dropping it so snooze management stays reachable on
    // narrow sidebars; the warning marker outranks the count.
    let count = footer.snooze_count.to_string();
    let candidates: Vec<String> = if footer.snooze_warning {
        vec![
            format!(" Snoozed({count}) ⚠"),
            " Snoozed ⚠".to_owned(),
            " ⚠".to_owned(),
        ]
    } else {
        vec![format!(" Snoozed({count})"), " Snoozed".to_owned()]
    };
    let snooze_text = candidates
        .into_iter()
        .find(|candidate| super::render::display_width(candidate) <= available);
    if let Some(snooze_text) = snooze_text {
        let snooze_width = super::render::display_width(&snooze_text);
        super::render::put_text(
            buffer,
            snooze_x,
            y,
            snooze_width,
            &snooze_text,
            if footer.snooze_warning {
                Style::default().fg(palette.red)
            } else {
                dim
            },
        );
        if config.mouse_capture {
            hits.feature_show_snoozed = Rect::new(snooze_x, y, snooze_width, 1);
        }
    }
}
