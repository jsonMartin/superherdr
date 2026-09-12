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
