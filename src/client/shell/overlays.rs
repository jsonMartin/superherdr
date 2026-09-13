use super::*;

mod settings_overlay;
mod worktree_overlays;

#[derive(Default)]
pub(crate) struct OverlayRender {
    pub(crate) primary: Rect,
    pub(crate) clear: Rect,
    pub(crate) cancel: Rect,
    pub(crate) snooze_popup: Rect,
    pub(crate) snooze_choice_rows: Vec<(Rect, usize)>,
    pub(crate) snooze_management_popup: Rect,
    pub(crate) snooze_management_rows: Vec<(Rect, usize)>,
    pub(crate) snooze_management_wake_all: Rect,
    pub(crate) snooze_management_wake_parent: Rect,
    pub(crate) snooze_management_reset: Rect,
    pub(crate) snooze_management_previous: Rect,
    pub(crate) snooze_management_next: Rect,
    pub(crate) navigator_popup: Rect,
    pub(crate) navigator_search: Rect,
    pub(crate) navigator_rows: Vec<(Rect, ClientNavigatorTarget)>,
    pub(crate) worktree_search: Rect,
    pub(crate) worktree_rows: Vec<(Rect, usize)>,
    pub(crate) help_popup: Rect,
    pub(crate) help_scrollbar: Rect,
    pub(crate) help_scroll_metrics: Option<crate::pane::ScrollMetrics>,
    pub(crate) help_max_scroll: usize,
    pub(crate) settings_popup: Rect,
    pub(crate) settings_tabs: Vec<(Rect, ClientSettingsSection)>,
    pub(crate) settings_choices: Vec<(Rect, usize)>,
    pub(crate) product_announcement_scrollbar: Rect,
    pub(crate) product_announcement_scroll_metrics: Option<crate::pane::ScrollMetrics>,
    pub(crate) product_announcement_max_scroll: usize,
    pub(crate) release_notes_scrollbar: Rect,
    pub(crate) release_notes_scroll_metrics: Option<crate::pane::ScrollMetrics>,
    pub(crate) release_notes_max_scroll: usize,
    pub(crate) cursor: Option<crate::protocol::CursorState>,
}

pub(crate) fn render_client_overlay(
    b: &mut Buffer,
    o: &ClientShellOverlay,
    s: &ClientShellSnapshot,
    endpoints: &[ClientShellEndpoint],
    active_endpoint_id: &ClientEndpointId,
    k: &LiveKeybindConfig,
    p: &Palette,
) -> Option<OverlayRender> {
    if !matches!(
        o,
        ClientShellOverlay::Navigator(_)
            | ClientShellOverlay::ContextMenu(_)
            | ClientShellOverlay::GlobalMenu(_)
    ) {
        for y in b.area.y..b.area.bottom() {
            for x in b.area.x..b.area.right() {
                let c = &mut b[(x, y)];
                c.set_style(c.style().add_modifier(Modifier::DIM));
            }
        }
    }
    match o {
        ClientShellOverlay::Onboarding => render_onboarding_overlay(b, p),
        ClientShellOverlay::ProductAnnouncement(v) => render_product_announcement_overlay(b, v, p),
        ClientShellOverlay::ReleaseNotes(v) => {
            render_release_notes_overlay(b, v, &s.update_install_command, p)
        }
        ClientShellOverlay::Rename(v) => render_rename_overlay(b, v, p),
        ClientShellOverlay::ConfirmClose(v) => render_confirm_close_overlay(b, v, p),
        ClientShellOverlay::ConfirmWakeSharedSnoozes(v) => {
            render_confirm_wake_shared_snoozes_overlay(b, v, p)
        }
        ClientShellOverlay::Snooze(v) => render_snooze_overlay(b, v, p),
        ClientShellOverlay::SnoozeManagement(v) => render_snooze_management_overlay(b, v, p),
        ClientShellOverlay::Help(v) => render_help_overlay(b, v, k, p),
        ClientShellOverlay::Navigator(v) => {
            render_navigator_overlay(b, v, endpoints, active_endpoint_id, p)
        }
        ClientShellOverlay::Settings(v) => {
            settings_overlay::render_settings_overlay(b, v, s.integration_updates_available, p)
        }
        ClientShellOverlay::WorktreeCreate(v) => {
            worktree_overlays::render_worktree_create_overlay(b, v, p)
        }
        ClientShellOverlay::WorktreeOpen(v) => {
            worktree_overlays::render_worktree_open_overlay(b, v, p)
        }
        ClientShellOverlay::WorktreeRemove(v) => {
            worktree_overlays::render_worktree_remove_overlay(b, v, p)
        }
        ClientShellOverlay::ContextMenu(_) | ClientShellOverlay::GlobalMenu(_) => None,
    }
}

/// Scope wording for a management row; inherited coverage spells out its parent project.
fn snooze_scope_text(record: &ClientSnoozeManagementRecord) -> String {
    match &record.target {
        ClientSnoozeManagementTarget::Inherited { .. } => format!(
            "Inherited from {}",
            record.project_label.as_deref().unwrap_or("its project")
        ),
        _ => record.scope.clone(),
    }
}

fn render_snooze_management_overlay(
    b: &mut Buffer,
    management: &ClientSnoozeManagementOverlay,
    p: &Palette,
) -> Option<OverlayRender> {
    let q = if b.area.width <= 28 || b.area.height <= 8 {
        b.area
    } else {
        popup(b.area, 92, 22)?
    };
    let i = panel(b, q, p.accent, p.panel_bg)?;
    if i.height < 10 {
        let style = Style::default().fg(p.text).bg(p.panel_bg);
        let (target, scope, project, wake, status) =
            management.records.get(management.selected).map_or_else(
                || {
                    (
                        String::new(),
                        String::new(),
                        String::new(),
                        String::new(),
                        String::new(),
                    )
                },
                |record| {
                    (
                        record.label.clone(),
                        snooze_scope_text(record),
                        record
                            .project_label
                            .clone()
                            .unwrap_or_else(|| record.label.clone()),
                        super::snooze_presets::wake_label_for_deadline(record.deadline_unix_ms),
                        if record.outside_focus {
                            if record.available {
                                "Outside Focus · Avail".to_owned()
                            } else {
                                "Outside Focus · Unavail".to_owned()
                            }
                        } else if record.available {
                            "Available".to_owned()
                        } else {
                            "Unavailable".to_owned()
                        },
                    )
                },
            );
        let status = if scope.is_empty() {
            status
        } else {
            format!("{scope} · {status}")
        };
        let navigation = management.host_count > 1 && i.width >= 16;
        let title_width = i.width.saturating_sub(if navigation { 5 } else { 0 });
        let heading = format!(" Snoozed · {}", management.endpoint_label);
        let target_line = management
            .restriction
            .as_deref()
            .or(management.notice.as_deref())
            .map_or_else(|| format!(" {target}"), |notice| format!(" ! {notice}"));
        let footer_y = i.y + i.height.saturating_sub(1);
        let lines = [
            heading,
            target_line,
            format!(" {project}"),
            format!(" {wake}"),
            format!(" {status}"),
            if i.width >= 21 {
                "↵Wake Reset Esc close".to_owned()
            } else {
                "↵Wake Reset Esc".to_owned()
            },
        ];
        for (offset, line) in lines.iter().enumerate().take(i.height as usize) {
            let line_style = if offset == 0 {
                Style::default()
                    .fg(p.yellow)
                    .bg(p.panel_bg)
                    .add_modifier(Modifier::BOLD)
            } else if offset == 5 {
                Style::default().fg(p.overlay0).bg(p.panel_bg)
            } else {
                style
            };
            let y = if offset == 5 {
                footer_y
            } else {
                i.y + offset as u16
            };
            put_text(
                b,
                i.x,
                y,
                if offset == 0 { title_width } else { i.width },
                line,
                line_style,
            );
        }
        let wake = Rect::new(i.x, footer_y, i.width.min(5), 1);
        let reset_x = i.x.saturating_add(6).min(i.right());
        let reset = Rect::new(
            reset_x,
            footer_y,
            i.right().saturating_sub(reset_x).min(5),
            1,
        );
        let close_x = i.x.saturating_add(12).min(i.right());
        let close = Rect::new(
            close_x,
            footer_y,
            i.right().saturating_sub(close_x).min(9),
            1,
        );
        let previous = if navigation {
            Rect::new(i.right().saturating_sub(5), i.y, 2, 1)
        } else {
            Rect::default()
        };
        let next = if navigation {
            Rect::new(i.right().saturating_sub(2), i.y, 2, 1)
        } else {
            Rect::default()
        };
        if navigation {
            let nav_style = Style::default()
                .fg(p.accent)
                .bg(p.panel_bg)
                .add_modifier(Modifier::UNDERLINED);
            put_text(b, previous.x, previous.y, previous.width, "‹", nav_style);
            put_text(b, next.x, next.y, next.width, "›", nav_style);
        }
        return Some(OverlayRender {
            primary: wake,
            cancel: close,
            snooze_management_popup: q,
            snooze_management_reset: reset,
            snooze_management_previous: previous,
            snooze_management_next: next,
            ..OverlayRender::default()
        });
    }
    let navigation = management.host_count > 1 && i.width >= 40;
    let host_label = format!("Host: {} ›", management.endpoint_label);
    let host_width = display_width(&host_label).min(i.width / 2);
    put_text(
        b,
        i.x,
        i.y,
        i.width
            .saturating_sub(if navigation { host_width + 1 } else { 0 }),
        " Snoozed",
        Style::default()
            .fg(p.text)
            .bg(p.panel_bg)
            .add_modifier(Modifier::BOLD),
    );
    let previous = Rect::default();
    let next = if navigation {
        Rect::new(i.right() - host_width, i.y, host_width, 1)
    } else {
        Rect::default()
    };
    if navigation {
        put_text(
            b,
            next.x,
            next.y,
            next.width,
            &host_label,
            Style::default()
                .fg(p.accent)
                .bg(p.panel_bg)
                .add_modifier(Modifier::UNDERLINED),
        );
    }
    let table = i.width >= 64;
    let name_width = if table {
        i.width.saturating_sub(38)
    } else {
        i.width
    };
    if !management.records.is_empty() {
        let style = Style::default()
            .fg(p.overlay0)
            .bg(p.panel_bg)
            .add_modifier(Modifier::BOLD);
        put_text(b, i.x, i.y + 2, name_width, " Spaces", style);
        if table {
            put_text(b, i.x + name_width, i.y + 2, 14, "Snoozed", style);
            put_text(b, i.x + name_width + 14, i.y + 2, 10, "Time left", style);
            put_text(b, i.x + name_width + 24, i.y + 2, 14, "Wakes", style);
        }
    }
    let mut rows = Vec::new();
    let list_height = i.height.saturating_sub(11).max(1) as usize;
    let start = management
        .scroll
        .min(management.records.len().saturating_sub(list_height));
    for (visible, (index, record)) in management
        .records
        .iter()
        .enumerate()
        .skip(start)
        .take(list_height)
        .enumerate()
    {
        let y = i.y.saturating_add(3 + visible as u16);
        let row = Rect::new(i.x, y, i.width, 1);
        let style = if index == management.selected {
            Style::default()
                .fg(contrast(p))
                .bg(p.accent)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(p.text).bg(p.panel_bg)
        };
        // Tree children (explicit or inherited) indent under their project parent; branch
        // connectors show whether another sibling child follows.
        let child = record.covered_by_project && record.workspace_id.is_some();
        let sibling_follows = management.records.get(index + 1).is_some_and(|next| {
            next.covered_by_project
                && next.workspace_id.is_some()
                && next.project_key == record.project_key
        });
        let indent = if child {
            if sibling_follows {
                "   ├─ "
            } else {
                "   └─ "
            }
        } else {
            " "
        };
        let inherited = matches!(
            record.target,
            ClientSnoozeManagementTarget::Inherited { .. }
        );
        let scope = if inherited {
            String::new()
        } else {
            format!(" · {}", record.scope)
        };
        let warning = if !record.available {
            " · Unavailable"
        } else if record.outside_focus {
            " · Outside Focus"
        } else {
            ""
        };
        let label = format!("{indent}{}{scope}{warning}", record.label);
        b.set_style(row, style);
        put_text(b, row.x, row.y, name_width.saturating_sub(1), &label, style);
        if table {
            let created = if inherited {
                "Via project".to_owned()
            } else {
                record
                    .created_unix_ms
                    .map(snooze_table_time)
                    .unwrap_or_else(|| "—".to_owned())
            };
            put_text(b, row.x + name_width, row.y, 13, &created, style);
            put_text(
                b,
                row.x + name_width + 14,
                row.y,
                9,
                &snooze_time_left(record.deadline_unix_ms, management.now_unix_ms),
                style,
            );
            put_text(
                b,
                row.x + name_width + 24,
                row.y,
                14,
                &snooze_table_time(record.deadline_unix_ms),
                style,
            );
        }
        rows.push((row, index));
    }
    if let Some(record) = management.records.get(management.selected) {
        let detail_y = i.y + i.height.saturating_sub(7);
        let details = [
            format!(
                " Snoozed: {} · Time left: {}",
                record
                    .created_unix_ms
                    .map(snooze_table_time)
                    .unwrap_or_else(|| "unknown".to_owned()),
                snooze_time_left(record.deadline_unix_ms, management.now_unix_ms)
            ),
            if matches!(
                record.target,
                ClientSnoozeManagementTarget::Inherited { .. }
            ) {
                format!(" Scope: {}", snooze_scope_text(record))
            } else {
                format!(
                    " Scope: {}{}",
                    record.scope,
                    record
                        .project_label
                        .as_deref()
                        .map_or(String::new(), |project| format!(" · project {project}"))
                )
            },
            format!(
                " Wake at: {}",
                super::snooze_presets::wake_label_for_deadline(record.deadline_unix_ms)
            ),
            format!(
                " {}{}",
                if record.available {
                    ""
                } else {
                    "Unavailable / reassociation needed"
                },
                if record.outside_focus {
                    " · Outside current Focus"
                } else {
                    ""
                }
            ),
        ];
        for (offset, detail) in details.iter().enumerate() {
            put_text(
                b,
                i.x,
                detail_y.saturating_add(offset as u16),
                i.width,
                detail,
                Style::default()
                    .fg(if record.outside_focus {
                        p.yellow
                    } else {
                        p.overlay0
                    })
                    .bg(p.panel_bg),
            );
        }
        let parent_label = if record.covered_by_project {
            record
                .project_label
                .as_deref()
                .or(Some(record.label.as_str()))
        } else {
            management.parent_action.as_ref().and_then(|parent| {
                parent
                    .project_label
                    .as_deref()
                    .or(Some(parent.label.as_str()))
            })
        };
        if let Some(parent_label) = parent_label {
            let parent = Rect::new(i.x, detail_y.saturating_add(4), i.width, 1);
            put_text(
                b,
                parent.x,
                parent.y,
                parent.width,
                &format!(" P  Wake parent project: {parent_label}"),
                Style::default()
                    .fg(p.yellow)
                    .bg(p.panel_bg)
                    .add_modifier(Modifier::UNDERLINED),
            );
        }
    }
    let notice_y = i.y + 1;
    if let Some(notice) = management
        .restriction
        .as_deref()
        .or(management.notice.as_deref())
    {
        put_text(
            b,
            i.x,
            notice_y,
            i.width,
            &format!(" ! {notice}"),
            Style::default().fg(p.yellow).bg(p.panel_bg),
        );
    }
    let buttons_y = i.bottom().saturating_sub(2);
    let button_row = Rect::new(i.x, buttons_y, i.width, 1);
    let widths = if i.width >= 64 {
        vec![12, 12, 12, 10]
    } else {
        vec![i.width.saturating_sub(3) / 4; 4]
    };
    let button_rects = row(button_row, &widths, 1, 0);
    // Standalone wake is disabled for inherited rows; the covering Wake project action (P)
    // is the supported path there.
    let wake_style = if management
        .records
        .get(management.selected)
        .is_some_and(|record| {
            matches!(
                record.target,
                ClientSnoozeManagementTarget::Inherited { .. }
            )
        }) {
        Style::default().fg(p.overlay0).bg(p.surface0)
    } else {
        Style::default()
            .fg(contrast(p))
            .bg(p.accent)
            .add_modifier(Modifier::BOLD)
    };
    let mut render = OverlayRender {
        snooze_management_popup: q,
        snooze_management_rows: rows,
        ..OverlayRender::default()
    };
    render.snooze_management_previous = previous;
    render.snooze_management_next = next;
    if management
        .records
        .get(management.selected)
        .is_some_and(|record| record.covered_by_project)
        || management.parent_action.is_some()
    {
        render.snooze_management_wake_parent =
            Rect::new(i.x, i.bottom().saturating_sub(3), i.width, 1);
    }
    if i.width >= 64 {
        let [wake, all, reset, cancel] = button_rects.as_slice() else {
            return Some(render);
        };
        button(b, *wake, " wake now ", wake_style);
        button(
            b,
            *all,
            " wake all ",
            Style::default()
                .fg(p.text)
                .bg(p.surface0)
                .add_modifier(Modifier::BOLD),
        );
        button(
            b,
            *reset,
            " Reset ",
            Style::default()
                .fg(p.text)
                .bg(p.surface0)
                .add_modifier(Modifier::BOLD),
        );
        button(
            b,
            *cancel,
            " close ",
            Style::default()
                .fg(p.text)
                .bg(p.surface0)
                .add_modifier(Modifier::BOLD),
        );
        render.primary = *wake;
        render.cancel = *cancel;
        render.snooze_management_wake_all = *all;
        render.snooze_management_reset = *reset;
    } else {
        let [wake, all, reset, cancel] = button_rects.as_slice() else {
            return Some(render);
        };
        button(b, *wake, " wake ", wake_style);
        button(
            b,
            *all,
            " all ",
            Style::default()
                .fg(p.text)
                .bg(p.surface0)
                .add_modifier(Modifier::BOLD),
        );
        button(
            b,
            *reset,
            " Reset ",
            Style::default()
                .fg(p.text)
                .bg(p.surface0)
                .add_modifier(Modifier::BOLD),
        );
        button(
            b,
            *cancel,
            " close ",
            Style::default()
                .fg(p.text)
                .bg(p.surface0)
                .add_modifier(Modifier::BOLD),
        );
        render.primary = *wake;
        render.cancel = *cancel;
        render.snooze_management_wake_all = *all;
        render.snooze_management_reset = *reset;
    }
    Some(render)
}

fn render_snooze_overlay(
    b: &mut Buffer,
    s: &ClientSnoozeOverlay,
    p: &Palette,
) -> Option<OverlayRender> {
    let q = popup(b.area, 70, 18)?;
    let i = panel(b, q, p.accent, p.panel_bg)?;
    if i.width < 28 || i.height < 14 {
        return None;
    }
    let title = if s.project_key.is_some() {
        " Snooze project"
    } else {
        " Snooze workspace"
    };
    put_text(
        b,
        i.x,
        i.y,
        i.width,
        title,
        Style::default()
            .fg(p.text)
            .bg(p.panel_bg)
            .add_modifier(Modifier::BOLD),
    );
    let scope = s.project_label.as_deref().map_or_else(
        || s.target_label.clone(),
        |label| format!("{} · project {label}", s.target_label),
    );
    put_text(
        b,
        i.x,
        i.y + 1,
        i.width,
        &format!(" {scope}"),
        Style::default().fg(p.overlay0).bg(p.panel_bg),
    );
    let Some(selected) = s.choices.get(s.selected) else {
        return None;
    };
    put_text(
        b,
        i.x,
        i.y + 2,
        i.width,
        &format!(" wake at {}", selected.wake_label),
        Style::default().fg(p.yellow).bg(p.panel_bg),
    );
    let mut choice_rows = Vec::new();
    for (index, choice) in s.choices.iter().enumerate() {
        let row = Rect::new(i.x, i.y + 4 + index as u16, i.width, 1);
        let style = if index == s.selected {
            Style::default()
                .fg(contrast(p))
                .bg(p.accent)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(p.text).bg(p.panel_bg)
        };
        b.set_style(row, style);
        put_text(
            b,
            row.x,
            row.y,
            row.width,
            &format!(" {}  {}", choice.label, choice.wake_label),
            style,
        );
        choice_rows.push((row, index));
    }
    let buttons = row(i, &[12, 12], 2, 11);
    let [confirm, cancel] = buttons.as_slice() else {
        return None;
    };
    button(
        b,
        *confirm,
        " snooze ",
        Style::default()
            .fg(contrast(p))
            .bg(p.accent)
            .add_modifier(Modifier::BOLD),
    );
    button(
        b,
        *cancel,
        " esc cancel ",
        Style::default()
            .fg(p.text)
            .bg(p.surface0)
            .add_modifier(Modifier::BOLD),
    );
    put_text(
        b,
        i.x,
        i.y + 12,
        i.width,
        " agents keep running · notifications unchanged",
        Style::default().fg(p.overlay0).bg(p.panel_bg),
    );
    put_text(
        b,
        i.x,
        i.bottom() - 1,
        i.width,
        " ↑↓/tab choose · enter snooze · esc cancel",
        Style::default().fg(p.overlay0).bg(p.panel_bg),
    );
    Some(OverlayRender {
        primary: *confirm,
        cancel: *cancel,
        snooze_popup: q,
        snooze_choice_rows: choice_rows,
        ..OverlayRender::default()
    })
}

pub(crate) fn render_global_menu(
    buffer: &mut Buffer,
    launcher: Rect,
    menu: &ClientGlobalMenuOverlay,
    snapshot: &ClientShellSnapshot,
    palette: &Palette,
) -> Option<Vec<(Rect, usize)>> {
    let items = super::super::global_menu::global_menu_items(snapshot);
    let screen = buffer.area;
    let width = items
        .iter()
        .map(|(label, action)| {
            display_width(label)
                + u16::from(super::super::global_menu::global_menu_item_has_badge(
                    snapshot, *action,
                )) * 2
        })
        .max()
        .unwrap_or(8)
        .saturating_add(4)
        .min(screen.width.max(1));
    let height = (items.len() as u16)
        .saturating_add(2)
        .min(screen.height.max(1));
    let x = launcher
        .right()
        .saturating_sub(width)
        .min(screen.right().saturating_sub(width));
    let y = launcher.y.saturating_sub(height).max(screen.y);
    let inner = panel(
        buffer,
        Rect::new(x, y, width, height),
        palette.accent,
        palette.panel_bg,
    )?;
    let mut rows = Vec::new();
    let first = menu
        .highlighted
        .saturating_sub(usize::from(inner.height.saturating_sub(1)));
    for (index, (label, action)) in items.iter().enumerate().skip(first) {
        let row_y = inner.y.saturating_add((index - first) as u16);
        if row_y >= inner.bottom() {
            break;
        }
        let row = Rect::new(inner.x, row_y, inner.width, 1);
        let highlighted = index == menu.highlighted;
        let style = if highlighted {
            Style::default()
                .fg(panel_contrast_fg(palette))
                .bg(palette.accent)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(palette.text).bg(palette.panel_bg)
        };
        buffer.set_style(row, style);
        let has_badge = super::super::global_menu::global_menu_item_has_badge(snapshot, *action);
        if has_badge {
            let badge_style = if highlighted {
                style
            } else {
                Style::default()
                    .fg(palette.accent)
                    .bg(palette.panel_bg)
                    .add_modifier(Modifier::BOLD)
            };
            put_text(buffer, row.x, row.y, row.width.min(2), " ●", badge_style);
            put_text(
                buffer,
                row.x.saturating_add(2),
                row.y,
                row.width.saturating_sub(2),
                &format!(" {label}"),
                style,
            );
        } else {
            put_text(buffer, row.x, row.y, row.width, &format!(" {label}"), style);
        }
        rows.push((row, index));
    }
    Some(rows)
}

pub(crate) fn render_context_menu(
    buffer: &mut Buffer,
    menu: &ClientContextMenuOverlay,
    palette: &Palette,
) -> Option<Vec<(Rect, usize)>> {
    let items = menu.items();
    let screen = buffer.area;
    let max_item_width = items
        .iter()
        .map(|item| display_width(item.label.as_str()))
        .max()
        .unwrap_or(0);
    let width = max_item_width
        .saturating_add(4)
        .max(14)
        .min(screen.width.max(1));
    let height = (items.len() as u16)
        .saturating_add(2)
        .min(screen.height.max(1));
    let x = menu
        .x
        .min(screen.x.saturating_add(screen.width.saturating_sub(width)));
    let y = menu.y.min(
        screen
            .y
            .saturating_add(screen.height.saturating_sub(height)),
    );
    let rect = Rect::new(x, y, width, height);
    let inner = panel(buffer, rect, palette.accent, palette.panel_bg)?;
    let mut rows = Vec::new();
    for (index, item) in items.iter().enumerate() {
        let row_y = inner.y.saturating_add(index as u16);
        if row_y >= inner.bottom() {
            break;
        }
        let row = Rect::new(inner.x, row_y, inner.width, 1);
        let highlighted = index == menu.highlighted;
        let style = if highlighted {
            Style::default()
                .fg(panel_contrast_fg(palette))
                .bg(palette.accent)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(palette.text).bg(palette.panel_bg)
        };
        buffer.set_style(row, style);
        put_text(buffer, row.x, row.y, row.width, &item.label, style);
        rows.push((row, index));
    }
    Some(rows)
}

fn panel(
    b: &mut Buffer,
    a: Rect,
    c: ratatui::style::Color,
    bg: ratatui::style::Color,
) -> Option<Rect> {
    if a.width < 2 || a.height < 2 {
        return None;
    }
    let background = Style::default().bg(bg).remove_modifier(Modifier::DIM);
    let border = Style::default().fg(c).bg(bg).remove_modifier(Modifier::DIM);
    for y in a.y..a.bottom() {
        for x in a.x..a.right() {
            b[(x, y)].set_symbol(" ").set_style(background);
        }
    }
    for x in a.x..a.right() {
        b[(x, a.y)]
            .set_symbol(if x == a.x {
                "┌"
            } else if x + 1 == a.right() {
                "┐"
            } else {
                "─"
            })
            .set_style(border);
        let y = a.bottom() - 1;
        b[(x, y)]
            .set_symbol(if x == a.x {
                "└"
            } else if x + 1 == a.right() {
                "┘"
            } else {
                "─"
            })
            .set_style(border);
    }
    for y in a.y + 1..a.bottom() - 1 {
        b[(a.x, y)].set_symbol("│").set_style(border);
        b[(a.right() - 1, y)].set_symbol("│").set_style(border);
    }
    Some(Rect::new(a.x + 1, a.y + 1, a.width - 2, a.height - 2))
}
fn popup(a: Rect, w: u16, h: u16) -> Option<Rect> {
    let w = w.min(a.width.saturating_sub(4));
    let h = h.min(a.height.saturating_sub(2));
    if w < 4 || h < 4 {
        return None;
    }
    Some(Rect::new(
        a.x + (a.width - w) / 2,
        a.y + (a.height - h) / 2,
        w,
        h,
    ))
}
fn button(b: &mut Buffer, r: Rect, t: &str, s: Style) {
    b.set_style(r, s);
    let w = display_width(t).min(r.width);
    put_text(b, r.x + (r.width - w) / 2, r.y, w, t, s)
}
fn row(i: Rect, ws: &[u16], gap: u16, off: u16) -> Vec<Rect> {
    let total = ws.iter().sum::<u16>() + gap * (ws.len().saturating_sub(1) as u16);
    let mut x = i.x + i.width.saturating_sub(total) / 2;
    ws.iter()
        .map(|w| {
            let r = Rect::new(
                x,
                i.y + off.min(i.height.saturating_sub(1)),
                (*w).min(i.width.saturating_sub(x - i.x)),
                1,
            );
            x += *w + gap;
            r
        })
        .collect()
}
fn contrast(p: &Palette) -> ratatui::style::Color {
    match p.panel_bg {
        ratatui::style::Color::Reset => p.surface_dim,
        c => c,
    }
}
fn render_release_notes_overlay(
    b: &mut Buffer,
    notes: &crate::app::state::ReleaseNotesState,
    install_command: &str,
    p: &Palette,
) -> Option<OverlayRender> {
    let outer = popup(
        b.area,
        crate::ui::RELEASE_NOTES_MODAL_SIZE.0,
        crate::ui::RELEASE_NOTES_MODAL_SIZE.1,
    )?;
    let inner = panel(b, outer, p.accent, p.panel_bg)?;
    if inner.height < 8 || inner.width < 20 {
        return Some(OverlayRender::default());
    }

    let stack = crate::ui::modal_stack_areas(inner, 2, 1, 0, 1);
    let title_area = Rect::new(
        stack.header.x.saturating_add(1),
        stack.header.y,
        stack.header.width.saturating_sub(2),
        1,
    );
    let subtitle_area = Rect::new(
        stack.header.x.saturating_add(1),
        stack.header.y.saturating_add(1),
        stack.header.width.saturating_sub(2),
        1,
    );
    let base = Style::default()
        .bg(p.panel_bg)
        .remove_modifier(Modifier::DIM);
    put_text(
        b,
        title_area.x,
        title_area.y,
        title_area.width,
        &format!("v{}", notes.version),
        base.fg(p.text).add_modifier(Modifier::BOLD),
    );
    put_text(
        b,
        subtitle_area.x,
        subtitle_area.y,
        subtitle_area.width,
        if notes.preview {
            "update ready"
        } else {
            "what's new in this release"
        },
        base.fg(p.overlay1),
    );
    let close = crate::ui::release_notes_close_button_rect(Rect::new(
        stack.header.x,
        stack.header.y,
        stack.header.width,
        1,
    ));
    button(
        b,
        close,
        " esc close ",
        Style::default()
            .fg(contrast(p))
            .bg(p.accent)
            .add_modifier(Modifier::BOLD)
            .remove_modifier(Modifier::DIM),
    );

    let body = stack.content;
    let lines = crate::ui::release_notes_display_lines(notes, install_command, p);
    let metrics = crate::ui::release_notes_scroll_metrics(notes, install_command, body, p);
    let max_scroll = metrics.max_offset_from_bottom;
    let scroll = usize::from(notes.scroll).min(max_scroll);
    let track = crate::ui::release_notes_scrollbar_rect(body, metrics);
    let text_area = track
        .map(|_| Rect::new(body.x, body.y, body.width.saturating_sub(1), body.height))
        .unwrap_or(body);
    let paragraph = ratatui::widgets::Paragraph::new(
        lines.into_iter().map(|(_, line)| line).collect::<Vec<_>>(),
    )
    .wrap(ratatui::widgets::Wrap { trim: false })
    .scroll((u16::try_from(scroll).unwrap_or(u16::MAX), 0));
    ratatui::widgets::Widget::render(paragraph, text_area, b);
    if let Some(track) = track {
        crate::ui::render_scrollbar_buffer(b, metrics, track, p.overlay0, p.overlay1, "▐");
    }

    if let Some(footer_area) = stack.footer {
        let footer_line = ratatui::text::Line::from(vec![
            ratatui::text::Span::styled(" scroll ", base.fg(p.overlay0)),
            ratatui::text::Span::styled("wheel ↑↓", base.fg(p.text)),
            ratatui::text::Span::styled("  ·  ", base.fg(p.overlay0)),
            ratatui::text::Span::styled("close", base.fg(p.overlay0)),
            ratatui::text::Span::styled(" esc / enter ", base.fg(p.text)),
        ]);
        ratatui::widgets::Widget::render(
            ratatui::widgets::Paragraph::new(footer_line),
            footer_area,
            b,
        );
    }

    Some(OverlayRender {
        primary: close,
        release_notes_scrollbar: track.unwrap_or_default(),
        release_notes_scroll_metrics: Some(metrics),
        release_notes_max_scroll: max_scroll,
        ..OverlayRender::default()
    })
}

fn render_product_announcement_overlay(
    b: &mut Buffer,
    announcement: &crate::app::state::ProductAnnouncementState,
    p: &Palette,
) -> Option<OverlayRender> {
    let outer = popup(
        b.area,
        crate::ui::PRODUCT_ANNOUNCEMENT_MODAL_SIZE.0,
        crate::ui::PRODUCT_ANNOUNCEMENT_MODAL_SIZE.1,
    )?;
    let inner = panel(b, outer, p.accent, p.panel_bg)?;
    if inner.height < 8 || inner.width < 20 {
        return Some(OverlayRender::default());
    }

    let stack = crate::ui::modal_stack_areas(inner, 2, 1, 0, 1);
    let title_area = Rect::new(
        stack.header.x.saturating_add(1),
        stack.header.y,
        stack.header.width.saturating_sub(2),
        1,
    );
    let subtitle_area = Rect::new(
        stack.header.x.saturating_add(1),
        stack.header.y.saturating_add(1),
        stack.header.width.saturating_sub(2),
        1,
    );
    let base = Style::default()
        .bg(p.panel_bg)
        .remove_modifier(Modifier::DIM);
    put_text(
        b,
        title_area.x,
        title_area.y,
        title_area.width,
        &announcement.title,
        base.fg(p.text).add_modifier(Modifier::BOLD),
    );
    let subtitle = if announcement.preview {
        "product announcement preview"
    } else {
        "product announcement"
    };
    put_text(
        b,
        subtitle_area.x,
        subtitle_area.y,
        subtitle_area.width,
        &format!("{subtitle} · v{}", announcement.version),
        base.fg(p.overlay1),
    );
    let close = crate::ui::release_notes_close_button_rect(Rect::new(
        stack.header.x,
        stack.header.y,
        stack.header.width,
        1,
    ));
    button(
        b,
        close,
        " esc close ",
        Style::default()
            .fg(contrast(p))
            .bg(p.accent)
            .add_modifier(Modifier::BOLD)
            .remove_modifier(Modifier::DIM),
    );

    let body = stack.content;
    let lines = crate::ui::product_announcement_display_lines(announcement, p);
    let metrics = crate::ui::product_announcement_scroll_metrics(announcement, body, p);
    let max_scroll = metrics.max_offset_from_bottom;
    let scroll = usize::from(announcement.scroll).min(max_scroll);
    let track = crate::ui::release_notes_scrollbar_rect(body, metrics);
    let text_area = track
        .map(|_| Rect::new(body.x, body.y, body.width.saturating_sub(1), body.height))
        .unwrap_or(body);
    let paragraph = ratatui::widgets::Paragraph::new(
        lines.into_iter().map(|(_, line)| line).collect::<Vec<_>>(),
    )
    .wrap(ratatui::widgets::Wrap { trim: false })
    .scroll((u16::try_from(scroll).unwrap_or(u16::MAX), 0));
    ratatui::widgets::Widget::render(paragraph, text_area, b);
    if let Some(track) = track {
        crate::ui::render_scrollbar_buffer(b, metrics, track, p.overlay0, p.overlay1, "▐");
    }

    if let Some(footer_area) = stack.footer {
        let footer_line = ratatui::text::Line::from(vec![
            ratatui::text::Span::styled(" scroll ", base.fg(p.overlay0)),
            ratatui::text::Span::styled("wheel ↑↓", base.fg(p.text)),
            ratatui::text::Span::styled("  ·  ", base.fg(p.overlay0)),
            ratatui::text::Span::styled("close", base.fg(p.overlay0)),
            ratatui::text::Span::styled(" esc / enter ", base.fg(p.text)),
        ]);
        ratatui::widgets::Widget::render(
            ratatui::widgets::Paragraph::new(footer_line),
            footer_area,
            b,
        );
    }

    Some(OverlayRender {
        primary: close,
        product_announcement_scrollbar: track.unwrap_or_default(),
        product_announcement_scroll_metrics: Some(metrics),
        product_announcement_max_scroll: max_scroll,
        ..OverlayRender::default()
    })
}

fn render_onboarding_overlay(b: &mut Buffer, p: &Palette) -> Option<OverlayRender> {
    let outer = popup(b.area, 64, 16)?;
    let inner = panel(b, outer, p.accent, p.panel_bg)?;
    if inner.height < 11 {
        return Some(OverlayRender::default());
    }
    let stack = crate::ui::modal_stack_areas(inner, 2, 0, 1, 1);
    let base = Style::default()
        .bg(p.panel_bg)
        .remove_modifier(Modifier::DIM);
    let title = base.fg(p.text).add_modifier(Modifier::BOLD);
    let muted = base.fg(p.overlay0);
    let text = base.fg(p.overlay1);
    let accent = base.fg(p.accent).add_modifier(Modifier::BOLD);

    put_text(
        b,
        stack.header.x,
        stack.header.y,
        stack.header.width,
        crate::ui::ONBOARDING_TITLE,
        title,
    );
    put_text(
        b,
        stack.header.x,
        stack.header.y.saturating_add(1),
        stack.header.width,
        crate::ui::ONBOARDING_SUBTITLE,
        muted,
    );

    let content = stack.content;
    for (offset, line) in crate::ui::ONBOARDING_DESCRIPTION.iter().enumerate() {
        put_text(
            b,
            content.x,
            content.y.saturating_add(offset as u16),
            content.width,
            line,
            text,
        );
    }

    let key_y = content.y.saturating_add(4);
    let mut key_x = content.x;
    for (value, style) in [
        ("  ", base),
        (crate::ui::ONBOARDING_PREFIX_LABEL, accent),
        (crate::ui::ONBOARDING_PREFIX_SUFFIX, text),
        (crate::ui::ONBOARDING_HELP_LABEL, accent),
        (crate::ui::ONBOARDING_HELP_SUFFIX, text),
    ] {
        let width = display_width(value);
        put_text(
            b,
            key_x,
            key_y,
            content.right().saturating_sub(key_x),
            value,
            style,
        );
        key_x = key_x.saturating_add(width);
    }
    put_text(
        b,
        content.x,
        content.y.saturating_add(5),
        content.width,
        crate::ui::ONBOARDING_NEXT,
        text,
    );

    let primary = crate::ui::onboarding_welcome_continue_rect(stack.actions.unwrap_or_default());
    button(
        b,
        primary,
        " ↵ continue ",
        Style::default()
            .fg(contrast(p))
            .bg(p.accent)
            .add_modifier(Modifier::BOLD)
            .remove_modifier(Modifier::DIM),
    );
    Some(OverlayRender {
        primary,
        ..OverlayRender::default()
    })
}

fn render_rename_overlay(
    b: &mut Buffer,
    v: &ClientRenameOverlay,
    p: &Palette,
) -> Option<OverlayRender> {
    let q = popup(b.area, 56, 7)?;
    let i = panel(b, q, p.accent, p.panel_bg)?;
    put_text(
        b,
        i.x,
        i.y,
        i.width,
        v.title,
        Style::default()
            .fg(p.text)
            .bg(p.panel_bg)
            .add_modifier(Modifier::BOLD),
    );
    let input = Rect::new(i.x, i.y + 2, i.width, 1);
    b.set_style(input, Style::default().fg(p.text).bg(p.surface0));
    put_text(
        b,
        input.x,
        input.y,
        input.width.saturating_sub(1),
        &format!(" {}", v.input),
        Style::default().fg(p.text).bg(p.surface0),
    );
    let rs = row(i, &[8, 10, 12], 2, 3);
    let [save, clear, cancel] = rs.as_slice() else {
        return None;
    };
    button(
        b,
        *save,
        " ↵ save ",
        Style::default()
            .fg(contrast(p))
            .bg(p.accent)
            .add_modifier(Modifier::BOLD),
    );
    let n = Style::default()
        .fg(p.text)
        .bg(p.surface0)
        .add_modifier(Modifier::BOLD);
    button(b, *clear, " ^c clear ", n);
    button(b, *cancel, " esc cancel ", n);
    Some(OverlayRender {
        primary: *save,
        clear: *clear,
        cancel: *cancel,
        navigator_popup: Rect::default(),
        navigator_search: Rect::default(),
        navigator_rows: Vec::new(),
        worktree_search: Rect::default(),
        worktree_rows: Vec::new(),
        cursor: Some(crate::protocol::CursorState {
            x: (input.x + 1 + display_width(&v.input)).min(input.right() - 1),
            y: input.y,
            visible: true,
            shape: 0,
        }),
        ..OverlayRender::default()
    })
}

fn render_navigator_overlay(
    b: &mut Buffer,
    n: &ClientNavigatorOverlay,
    endpoints: &[ClientShellEndpoint],
    active_endpoint_id: &ClientEndpointId,
    p: &Palette,
) -> Option<OverlayRender> {
    let a = b.area;
    let mx = (a.width / 16).max(2);
    let my = (a.height / 10).max(1);
    let q = Rect::new(
        a.x + mx,
        a.y + my,
        a.width.saturating_sub(mx * 2).max(4),
        a.height.saturating_sub(my * 2).max(4),
    );
    let i = panel(b, q, p.accent, p.panel_bg)?;
    let rows = super::aggregate_navigation::navigator_rows(endpoints, active_endpoint_id, n);
    let search = if n.search_focused {
        format!(" / {}", n.query)
    } else if let Some(f) = n.filter {
        format!(
            " / {}",
            match f {
                ClientNavigatorFilter::Blocked => "blocked",
                ClientNavigatorFilter::Working => "working",
                ClientNavigatorFilter::Idle => "idle",
                ClientNavigatorFilter::Done => "done",
            }
        )
    } else if n.query.is_empty() {
        " / search panes".to_owned()
    } else {
        format!(" / {}", n.query)
    };
    put_text(
        b,
        i.x,
        i.y,
        i.width,
        &search,
        Style::default()
            .fg(if n.search_focused { p.text } else { p.overlay0 })
            .bg(p.panel_bg),
    );
    put_right_text(
        b,
        i,
        i.y,
        &format!(
            "{} panes",
            rows.iter()
                .filter(|row| matches!(row.target, ClientNavigatorTarget::Pane { .. }))
                .count()
        ),
        Style::default().fg(p.overlay0).bg(p.panel_bg),
    );
    put_text(
        b,
        i.x,
        i.y + 1,
        i.width,
        &"─".repeat(i.width as usize),
        Style::default().fg(p.surface1).bg(p.panel_bg),
    );
    let body = Rect::new(i.x, i.y + 2, i.width, i.height.saturating_sub(4));
    let selected = super::aggregate_navigation::navigator_selected_index(&rows, n).unwrap_or(0);
    let max = rows.len().saturating_sub(body.height as usize);
    let scroll = n
        .scroll
        .max(selected.saturating_sub(body.height.saturating_sub(1) as usize))
        .min(selected)
        .min(max);
    let mut row_hits = Vec::new();
    for (vis, (ix, r)) in rows
        .iter()
        .enumerate()
        .skip(scroll)
        .take(body.height as usize)
        .enumerate()
    {
        let rect = Rect::new(body.x, body.y + vis as u16, body.width, 1);
        row_hits.push((rect, r.target.clone()));
        let st = if r.stale {
            Style::default()
                .fg(p.overlay0)
                .bg(if ix == selected {
                    p.surface0
                } else {
                    p.panel_bg
                })
                .add_modifier(Modifier::DIM)
        } else if ix == selected {
            Style::default()
                .fg(contrast(p))
                .bg(p.accent)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default()
                .fg(if r.current { p.text } else { p.subtext0 })
                .bg(p.panel_bg)
        };
        b.set_style(rect, st);
        let tree = match &r.target {
            ClientNavigatorTarget::Machine { .. } => "▾",
            ClientNavigatorTarget::Workspace {
                endpoint_id,
                workspace_id,
            } if n
                .expanded_workspaces
                .contains(&(endpoint_id.clone(), workspace_id.clone())) =>
            {
                if r.depth == 0 {
                    "▾"
                } else {
                    "  ▾"
                }
            }
            ClientNavigatorTarget::Workspace { .. } => {
                if r.depth == 0 {
                    "▸"
                } else {
                    "  ▸"
                }
            }
            ClientNavigatorTarget::Tab { .. } => {
                if r.depth == 1 {
                    "└──"
                } else {
                    "    └──"
                }
            }
            ClientNavigatorTarget::Pane { .. } => {
                if r.depth == 2 {
                    "   └──"
                } else {
                    "        └──"
                }
            }
        };
        let current = if r.current { "◆ " } else { "" };
        let status = r.status.map(status_dot).unwrap_or_default();
        let status_separator = if status.is_empty() { "" } else { " " };
        let label = format!(" {tree} {current}{status}{status_separator}{}", r.label);
        put_text(b, rect.x, rect.y, rect.width, &label, st);
        if let Some(status) = r.status {
            let prefix = format!(" {tree} {current}");
            let status_style = if r.stale || ix == selected {
                st
            } else {
                Style::default().fg(status_color(status, p)).bg(p.panel_bg)
            };
            put_text(
                b,
                rect.x.saturating_add(display_width(&prefix)),
                rect.y,
                display_width(status_dot(status)),
                status_dot(status),
                status_style,
            );
        }
        let machine_status = match &r.target {
            ClientNavigatorTarget::Machine { endpoint_id } if !endpoint_id.is_local() => endpoints
                .iter()
                .find(|endpoint| &endpoint.endpoint_id == endpoint_id)
                .map(|endpoint| endpoint.status),
            _ => None,
        };
        if let Some(status) = machine_status {
            let (glyph, state, color) = endpoint_status_presentation(status, p);
            let signal = if status == ClientEndpointStatus::Online {
                glyph.to_owned()
            } else {
                format!("{glyph} {state}")
            };
            let signal_style = if ix == selected {
                st
            } else {
                Style::default()
                    .fg(color)
                    .bg(p.panel_bg)
                    .add_modifier(if r.stale {
                        Modifier::DIM
                    } else {
                        Modifier::empty()
                    })
            };
            put_right_text(b, rect, rect.y, &signal, signal_style);
        } else if !r.meta.is_empty() {
            let label_width = display_width(&label).min(rect.width);
            let meta = Rect::new(
                rect.x.saturating_add(label_width).saturating_add(1),
                rect.y,
                rect.width.saturating_sub(label_width.saturating_add(1)),
                1,
            );
            put_right_text(b, meta, rect.y, &r.meta, st)
        }
    }
    let dy = i.bottom() - 2;
    if let Some(r) = rows.get(selected) {
        put_text(
            b,
            i.x,
            dy,
            i.width,
            &format!(" {} · {}", r.label, r.meta),
            Style::default().fg(p.overlay0).bg(p.panel_bg),
        )
    }
    put_text(
        b,
        i.x,
        i.bottom() - 1,
        i.width,
        if n.search_focused {
            " search type · move ↑↓/ctrl+n/p · open enter · back esc"
        } else {
            " move j/k · expand space · filter a/b/w/i/d · search / · open enter · close esc"
        },
        Style::default().fg(p.overlay0).bg(p.panel_bg),
    );
    Some(OverlayRender {
        primary: Rect::default(),
        clear: Rect::default(),
        cancel: Rect::default(),
        navigator_popup: q,
        navigator_search: Rect::new(i.x, i.y, i.width, 1),
        navigator_rows: row_hits,
        worktree_search: Rect::default(),
        worktree_rows: Vec::new(),
        cursor: n.search_focused.then(|| crate::protocol::CursorState {
            x: i.x + 3 + display_width(&n.query),
            y: i.y,
            visible: true,
            shape: 0,
        }),
        ..OverlayRender::default()
    })
}

fn help_lines(
    keybinds: &LiveKeybindConfig,
    query: &str,
    palette: &Palette,
) -> Vec<(usize, ratatui::text::Line<'static>)> {
    use ratatui::text::{Line, Span};

    let groups = crate::input::filter_keybind_help_groups(
        crate::input::keybind_help_groups(&keybinds.keybinds, keybinds.prefix),
        query,
    );
    let key_width = groups
        .iter()
        .flat_map(|(_, entries)| entries.iter().map(|(key, _)| key.chars().count()))
        .max()
        .unwrap_or(8);
    if groups.is_empty() {
        let message = " no matching keybinds";
        return vec![(
            message.chars().count(),
            Line::from(Span::styled(
                message,
                Style::default().fg(palette.overlay1).bg(palette.panel_bg),
            )),
        )];
    }

    let mut lines = Vec::new();
    for (group, entries) in groups {
        lines.push((
            group.len() + 1,
            Line::from(Span::styled(
                format!(" {group}"),
                Style::default()
                    .fg(palette.accent)
                    .bg(palette.panel_bg)
                    .add_modifier(Modifier::BOLD),
            )),
        ));
        for (key, label) in entries {
            let padded_key = format!(" {key:<key_width$} ");
            let width = padded_key.chars().count() + label.chars().count();
            lines.push((
                width,
                Line::from(vec![
                    Span::styled(
                        padded_key,
                        Style::default()
                            .fg(palette.mauve)
                            .bg(palette.panel_bg)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        label.into_owned(),
                        Style::default().fg(palette.text).bg(palette.panel_bg),
                    ),
                ]),
            ));
        }
        lines.push((0, Line::raw("")));
    }
    lines
}

fn render_help_overlay(
    b: &mut Buffer,
    h: &ClientHelpOverlay,
    k: &LiveKeybindConfig,
    p: &Palette,
) -> Option<OverlayRender> {
    use ratatui::widgets::{Paragraph, Widget, Wrap};

    let q = popup(b.area, 76, 22)?;
    let i = panel(b, q, p.accent, p.panel_bg)?;
    if i.width < 20 || i.height < 6 {
        return None;
    }
    put_text(
        b,
        i.x,
        i.y,
        i.width,
        "keybinds",
        Style::default()
            .fg(p.text)
            .bg(p.panel_bg)
            .add_modifier(Modifier::BOLD),
    );
    let close = Rect::new(i.right() - 13, i.y, 13, 1);
    button(
        b,
        close,
        if h.search_focused {
            " esc back "
        } else {
            " esc close "
        },
        Style::default()
            .fg(contrast(p))
            .bg(p.accent)
            .add_modifier(Modifier::BOLD),
    );
    let sy = i.y + 1;
    put_text(
        b,
        i.x,
        sy,
        i.width,
        &if h.search_focused {
            format!(" / {}", h.query)
        } else {
            " / press / to filter by command or shortcut".to_owned()
        },
        Style::default()
            .fg(if h.search_focused { p.text } else { p.overlay0 })
            .bg(p.panel_bg),
    );

    let body = Rect::new(i.x, i.y + 3, i.width, i.height.saturating_sub(5));
    let lines = help_lines(k, &h.query, p);
    let viewport_rows = usize::from(body.height.max(1));
    let wrapped_rows = |width: u16| {
        let width = usize::from(width.max(1));
        lines
            .iter()
            .map(|(line_width, _)| line_width.max(&1).div_ceil(width))
            .sum::<usize>()
    };
    let needs_scrollbar = wrapped_rows(body.width) > viewport_rows;
    let text_area = if needs_scrollbar {
        Rect::new(body.x, body.y, body.width.saturating_sub(1), body.height)
    } else {
        body
    };
    let total_rows = wrapped_rows(text_area.width);
    let max_scroll = total_rows.saturating_sub(viewport_rows);
    let scroll = h.scroll.min(max_scroll);
    let metrics = crate::pane::ScrollMetrics {
        offset_from_bottom: max_scroll.saturating_sub(scroll),
        max_offset_from_bottom: max_scroll,
        viewport_rows,
    };
    let scrollbar = needs_scrollbar.then_some(Rect::new(
        body.right().saturating_sub(1),
        body.y,
        1,
        body.height,
    ));
    Widget::render(
        Paragraph::new(lines.into_iter().map(|(_, line)| line).collect::<Vec<_>>())
            .wrap(Wrap { trim: false })
            .scroll((u16::try_from(scroll).unwrap_or(u16::MAX), 0)),
        text_area,
        b,
    );
    if let Some(track) = scrollbar {
        if let Some(thumb) = crate::ui::scrollbar_thumb(metrics, track) {
            for y in track.y..track.bottom() {
                b[(track.x, y)]
                    .set_symbol("▐")
                    .set_style(Style::default().fg(p.overlay0).bg(p.panel_bg));
            }
            for y in thumb.top..thumb.top.saturating_add(thumb.len) {
                b[(track.x, y)]
                    .set_symbol("▐")
                    .set_style(Style::default().fg(p.overlay1).bg(p.panel_bg));
            }
        }
    }

    put_text(
        b,
        i.x,
        i.bottom() - 1,
        i.width,
        if h.search_focused {
            " filter type/backspace · clear ctrl+u · scroll ↑↓/pgup/pgdn · back esc"
        } else {
            " search / · scroll j/k/↑↓/pgup/pgdn · close esc/enter"
        },
        Style::default().fg(p.overlay0).bg(p.panel_bg),
    );
    Some(OverlayRender {
        cancel: close,
        help_popup: q,
        help_scrollbar: scrollbar.unwrap_or_default(),
        help_scroll_metrics: Some(metrics),
        help_max_scroll: max_scroll,
        cursor: h.search_focused.then(|| crate::protocol::CursorState {
            x: (i.x + 3 + display_width(&h.query)).min(i.right() - 1),
            y: sy,
            visible: true,
            shape: 0,
        }),
        ..OverlayRender::default()
    })
}
fn render_confirm_close_overlay(
    b: &mut Buffer,
    c: &ClientConfirmCloseOverlay,
    p: &Palette,
) -> Option<OverlayRender> {
    let q = popup(b.area, 64, 6)?;
    let i = panel(b, q, p.red, p.panel_bg)?;
    put_text(
        b,
        i.x,
        i.y,
        i.width,
        &format!(" {}", c.title),
        Style::default()
            .fg(p.red)
            .bg(p.panel_bg)
            .add_modifier(Modifier::BOLD),
    );
    put_text(
        b,
        i.x,
        i.y + 1,
        i.width,
        &format!(" {}", c.detail),
        Style::default().fg(p.text).bg(p.panel_bg),
    );
    let rs = row(i, &[13, 12], 2, 3);
    let [ok, cancel] = rs.as_slice() else {
        return None;
    };
    button(
        b,
        *ok,
        " ↵ confirm ",
        Style::default()
            .fg(contrast(p))
            .bg(p.red)
            .add_modifier(Modifier::BOLD),
    );
    button(
        b,
        *cancel,
        " esc cancel ",
        Style::default()
            .fg(p.text)
            .bg(p.surface0)
            .add_modifier(Modifier::BOLD),
    );
    Some(OverlayRender {
        primary: *ok,
        clear: Rect::default(),
        cancel: *cancel,
        navigator_popup: Rect::default(),
        navigator_search: Rect::default(),
        navigator_rows: Vec::new(),
        worktree_search: Rect::default(),
        worktree_rows: Vec::new(),
        cursor: None,
        ..OverlayRender::default()
    })
}

fn render_confirm_wake_shared_snoozes_overlay(
    b: &mut Buffer,
    c: &ClientConfirmWakeSharedSnoozesOverlay,
    p: &Palette,
) -> Option<OverlayRender> {
    let q = popup(b.area, 64, 8)?;
    let i = panel(b, q, p.yellow, p.panel_bg)?;
    let (title, details) = match c.purpose {
        ClientWakeConfirmationPurpose::WakeSharedSnoozes => (
            " Wake shared snoozes?",
            vec![format!(" {} records on {}", c.count, c.endpoint_label)],
        ),
        ClientWakeConfirmationPurpose::ResetFocusSnooze => (
            " Reset Focus + Snooze?",
            vec![
                " Clear this window's Focus".to_owned(),
                format!(" plus all shared snoozes on {}", c.endpoint_label),
                format!(" ({} records)", c.count),
            ],
        ),
    };
    put_text(
        b,
        i.x,
        i.y,
        i.width,
        title,
        Style::default()
            .fg(p.yellow)
            .bg(p.panel_bg)
            .add_modifier(Modifier::BOLD),
    );
    for (offset, detail) in details.iter().enumerate() {
        put_text(
            b,
            i.x,
            i.y + 1 + offset as u16,
            i.width,
            detail,
            Style::default().fg(p.text).bg(p.panel_bg),
        );
    }
    let rs = row(i, &[13, 12], 2, 4);
    let [ok, cancel] = rs.as_slice() else {
        return None;
    };
    button(
        b,
        *ok,
        " ↵ confirm ",
        Style::default()
            .fg(contrast(p))
            .bg(p.yellow)
            .add_modifier(Modifier::BOLD),
    );
    button(
        b,
        *cancel,
        " esc cancel ",
        Style::default()
            .fg(p.text)
            .bg(p.surface0)
            .add_modifier(Modifier::BOLD),
    );
    Some(OverlayRender {
        primary: *ok,
        cancel: *cancel,
        ..OverlayRender::default()
    })
}

fn snooze_table_time(timestamp: i64) -> String {
    let label = super::snooze_presets::wake_label_for_deadline(timestamp);
    label.get(5..16).unwrap_or(&label).to_owned()
}

fn snooze_time_left(deadline: i64, now: i64) -> String {
    let millis = deadline.saturating_sub(now);
    if millis <= 0 {
        return "Due".to_owned();
    }
    let minutes = millis.saturating_add(59_999) / 60_000;
    if minutes >= 1_440 {
        format!("{}d {}h", minutes / 1_440, minutes % 1_440 / 60)
    } else if minutes >= 60 {
        format!("{}h {}m", minutes / 60, minutes % 60)
    } else {
        format!("{minutes}m")
    }
}

#[cfg(test)]
mod snooze_time_tests {
    use super::snooze_time_left;
    #[test]
    fn remaining_time_rounds_up_and_stops_at_deadline() {
        assert_eq!(snooze_time_left(0, 0), "Due");
        assert_eq!(snooze_time_left(0, 1), "Due");
        assert_eq!(snooze_time_left(1, 0), "1m");
        assert_eq!(snooze_time_left(3_600_000, 0), "1h 0m");
        assert_eq!(snooze_time_left(86_400_000, 0), "1d 0h");
    }
}
