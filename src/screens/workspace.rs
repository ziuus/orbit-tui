use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use ratatui::Frame;

use crate::app::{App, PanelId};
use crate::config::WidgetConfig;
use crate::monitors::files;
use crate::monitors::obsidian;
use crate::screens::{panel, panel_full};

pub fn render(f: &mut Frame, area: Rect, app: &mut App) {
    let theme_val = app.theme.clone();
    let theme = &theme_val;
    let cfg = app.config.widgets.clone();
    let focused_panel = app.focused_panel;
    let focus = |id: PanelId| focused_panel == Some(id);

    let layout = app.config.ui.focus_layout.clone();
    let [timer_area, agenda_area, tasks_area, news_area, notes_area, files_area] =
        focus_areas(&layout, area, app.panel_states.work_ratio, &cfg);
    let rows = [agenda_area, tasks_area, news_area];

    let timer_focused = focus(PanelId::Timer);
    let show_news = cfg.news && news_area.height > 2;
    crate::screens::hit(timer_area, crate::screens::Hit::Panel(PanelId::Timer));
    let inner = panel(f, timer_area, "focus timer", theme, timer_focused);
    crate::widgets::pomodoro::render(f, inner, theme, &app.config.ui, timer_focused);

    if cfg.agenda && agenda_area.height > 2 {
        let snap = crate::monitors::agenda::snapshot();
        let count = snap.events.len();
        let agenda_rt = if count == 0 {
            " no events ".to_string()
        } else {
            format!(" {} upcoming ", count)
        };
        let hint = if app.panel_states.agenda_input_active {
            "Enter submit · Esc cancel"
        } else {
            "a add · r edit · d del · e file · ↑↓"
        };
        crate::screens::hit(rows[0], crate::screens::Hit::Panel(PanelId::Agenda));
        let inner = panel_full(
            f,
            rows[0],
            "agenda",
            Some(&agenda_rt),
            Some(hint),
            theme,
            focus(PanelId::Agenda),
        );
        crate::widgets::agenda::render(
            f,
            inner,
            theme,
            focus(PanelId::Agenda),
            app.panel_states.agenda_selected,
            app.panel_states.agenda_input_active,
            &app.panel_states.agenda_input,
        );
    }

    if cfg.tasks && tasks_area.height > 2 {
        let snap = crate::monitors::tasks::snapshot();
        let open = snap.tasks.iter().filter(|t| !t.completed).count();
        let tasks_rt = format!(" {} open ", open);
        let hint = if app.panel_states.task_input_active {
            "Enter submit · Esc cancel"
        } else {
            "a add · Space toggle · d del · e edit"
        };
        crate::screens::hit(rows[1], crate::screens::Hit::Panel(PanelId::Tasks));
        let inner = panel_full(
            f,
            rows[1],
            "tasks",
            Some(&tasks_rt),
            Some(hint),
            theme,
            focus(PanelId::Tasks),
        );
        crate::widgets::tasks::render(
            f,
            inner,
            theme,
            focus(PanelId::Tasks),
            app.panel_states.tasks_selected,
            app.panel_states.task_input_active,
            &app.panel_states.task_input,
        );
    }

    if show_news {
        let snap = crate::monitors::news::snapshot();
        let source = if snap.channel_title.is_empty() {
            " fetching ".to_string()
        } else {
            format!(" {} ", snap.channel_title)
        };
        crate::screens::hit(rows[2], crate::screens::Hit::Panel(PanelId::News));
        let inner = panel_full(
            f,
            rows[2],
            "news",
            Some(&source),
            None,
            theme,
            focus(PanelId::News),
        );
        crate::widgets::news::render(f, inner, theme);
    }

    if notes_area.height > 2 {
        render_notes(f, notes_area, app);
    }
    if files_area.height < 3 {
        return;
    }

    // Bottom Right: Files
    let files_hint = if app.panel_states.files_show_hidden {
        "h hide · / search · m dir · N rename · x del · y copy · o open"
    } else {
        ". hidden · / search · m dir · N rename · x del · y copy · o open"
    };

    let files_rt = {
        let snap = files::snapshot();
        let total = snap.items.len();
        format!(" {} items ", total)
    };
    let files_focused = focus(PanelId::Files);
    crate::screens::hit(files_area, crate::screens::Hit::Panel(PanelId::Files));
    let files_inner = panel_full(
        f,
        files_area,
        "files",
        Some(&files_rt),
        Some(files_hint),
        theme,
        files_focused,
    );

    crate::widgets::files::render(f, files_inner, app);
}

pub fn render_notes(f: &mut Frame, area: Rect, app: &mut App) {
    let snap = obsidian::snapshot();
    let theme = &app.theme;
    let is_notes_focused = app.focused_panel == Some(PanelId::WriterNotes);
    let notes_title = if snap.vault_name.is_empty() {
        "notes".to_string()
    } else {
        format!("notes ({})", snap.vault_name)
    };
    crate::screens::hit(area, crate::screens::Hit::Panel(PanelId::WriterNotes));
    let notes_inner = panel_full(
        f,
        area,
        &notes_title,
        None,
        Some("Enter/e edit · ↑↓ select · PgUp/PgDn scroll"),
        theme,
        is_notes_focused,
    );
    let note_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(33), Constraint::Percentage(67)])
        .spacing(1)
        .split(notes_inner);

    let list_area = note_chunks[0];
    let content_area = note_chunks[1];

    let border_color = if is_notes_focused {
        theme.accent
    } else {
        theme.surface
    };

    let list_inner_area = Block::default()
        .borders(Borders::RIGHT)
        .border_style(Style::default().fg(border_color))
        .inner(list_area);

    f.render_widget(
        Block::default()
            .borders(Borders::RIGHT)
            .border_style(Style::default().fg(border_color)),
        list_area,
    );

    let mut list_lines = Vec::new();
    let num_notes = snap.notes.len();
    if num_notes == 0 {
        list_lines.push(Line::from(vec![Span::styled(
            " No notes found in vault. Press e to create.",
            Style::default().fg(theme.dim),
        )]));
    } else {
        let max_idx = num_notes.saturating_sub(1);
        app.panel_states.writer_selected = app.panel_states.writer_selected.min(max_idx);
        let selected = app.panel_states.writer_selected;

        let visible_items = list_inner_area.height as usize;
        let mut scroll = app.panel_states.writer_scroll;
        if selected < scroll {
            scroll = selected;
        } else if selected >= scroll + visible_items && visible_items > 0 {
            scroll = selected.saturating_sub(visible_items - 1);
        }
        app.panel_states.writer_scroll = scroll;
        crate::screens::hit(
            list_inner_area,
            crate::screens::Hit::Rows {
                panel: PanelId::WriterNotes,
                first: scroll,
            },
        );

        let title_w = (list_inner_area.width as usize).saturating_sub(3);
        for (i, note) in snap
            .notes
            .iter()
            .enumerate()
            .skip(scroll)
            .take(visible_items)
        {
            let title = crate::widgets::meter::ellipsize(&note.title, title_w);
            if i == selected {
                list_lines.push(Line::from(vec![
                    Span::styled(" > ", Style::default().fg(theme.accent)),
                    Span::styled(title, Style::default().fg(theme.text)),
                ]));
            } else {
                list_lines.push(Line::from(vec![
                    Span::raw("   "),
                    Span::styled(title, Style::default().fg(theme.dim)),
                ]));
            }
        }
    }

    f.render_widget(Paragraph::new(list_lines), list_inner_area);

    let mut content_lines = Vec::new();
    if num_notes > 0 {
        let selected = app.panel_states.writer_selected;
        let note = &snap.notes[selected];
        let words = note.content.split_whitespace().count();
        let age = note
            .modified
            .elapsed()
            .map(|d| ago(d.as_secs()))
            .unwrap_or_default();
        content_lines.push(Line::from(vec![Span::styled(
            note.title.clone(),
            Style::default()
                .fg(theme.accent)
                .add_modifier(ratatui::style::Modifier::BOLD),
        )]));
        content_lines.push(Line::from(Span::styled(
            format!("{} words · edited {}", words, age),
            Style::default().fg(theme.dim),
        )));
        content_lines.push(Line::from(""));
        content_lines.extend(crate::widgets::markdown::render(&note.content, theme));
    }

    // Keep the title/meta header pinned; scroll only the body. Clamped so
    // the last line of the note can reach the top but never past it.
    let header = content_lines.len().min(3);
    let body = content_lines.split_off(header);
    let max_scroll = body.len().saturating_sub(1) as u16;
    let scroll = app.panel_states.note_scroll.min(max_scroll);
    app.panel_states.note_scroll = scroll;
    crate::screens::hit(content_area, crate::screens::Hit::NotePreview);
    let [head_area, body_area] =
        Layout::vertical([Constraint::Length(header as u16), Constraint::Min(0)])
            .areas(content_area);
    f.render_widget(Paragraph::new(content_lines), head_area);
    f.render_widget(
        Paragraph::new(body)
            .wrap(Wrap { trim: false })
            .scroll((scroll, 0)),
        body_area,
    );
}

/// "just now", "5m ago", "3h ago", "2d ago", "6w ago".
fn ago(secs: u64) -> String {
    match secs {
        0..60 => "just now".to_string(),
        60..3600 => format!("{}m ago", secs / 60),
        3600..86_400 => format!("{}h ago", secs / 3600),
        86_400..1_209_600 => format!("{}d ago", secs / 86_400),
        _ => format!("{}w ago", secs / 604_800),
    }
}

/// Focus layouts, cycled from the Esc menu (ui.focus_layout).
pub const LAYOUTS: [&str; 4] = ["classic", "writer", "planner", "files"];

/// Areas for [timer, agenda, tasks, news, notes, files]; a zero-height
/// area means that panel isn't part of the layout.
fn focus_areas(layout: &str, area: Rect, work_ratio: u16, cfg: &WidgetConfig) -> [Rect; 6] {
    let none = Rect::new(area.x, area.y, 0, 0);
    // The left column holds the timer's big digits and task text, so it
    // never drops below what those need; the right side keeps 40 too.
    let half = area.width / 2;
    let left_w = (area.width as u32 * work_ratio as u32 / 100) as u16;
    let left_w = left_w.clamp(40.min(half), area.width.saturating_sub(40).max(half));
    let [left, right] =
        Layout::horizontal([Constraint::Length(left_w), Constraint::Min(0)]).areas(area);
    // 10 rows is the minimum that fits the 5-row big digits.
    let timer_h = |h: u16| match h {
        36.. => 11,
        30.. => 10,
        _ => 7,
    };
    let column = |col: Rect, agenda: bool| -> [Rect; 4] {
        // News only when there's room to spare: a feed is the opposite of focus.
        let news = cfg.news && col.height >= 44;
        Layout::vertical([
            Constraint::Length(timer_h(col.height)),
            Constraint::Length(if agenda && cfg.agenda { 9 } else { 0 }),
            Constraint::Min(if cfg.tasks { 6 } else { 0 }),
            Constraint::Length(if news { 10 } else { 0 }),
        ])
        .areas(col)
    };
    match layout {
        // A tall note preview; the timer and tasks beside it.
        "writer" => {
            let [t, a, k, n] = column(left, true);
            [t, a, k, n, right, none]
        }
        // Plan the day: timer, agenda and tasks side by side over the notes
        // and files.
        "planner" => {
            let [top, bottom] =
                Layout::vertical([Constraint::Percentage(48), Constraint::Min(8)]).areas(area);
            let [t, a, k] = Layout::horizontal([
                Constraint::Percentage(34),
                Constraint::Percentage(33),
                Constraint::Percentage(33),
            ])
            .areas(top);
            let [notes, files] =
                Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)])
                    .areas(bottom);
            [t, a, k, none, notes, files]
        }
        // The file manager full height.
        "files" => {
            let [t, a, k, n] = column(left, true);
            [t, a, k, n, none, right]
        }
        _ => {
            let [t, a, k, n] = column(left, true);
            let [notes, files] =
                Layout::vertical([Constraint::Percentage(50), Constraint::Percentage(50)])
                    .areas(right);
            [t, a, k, n, notes, files]
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn ago_picks_a_readable_unit() {
        assert_eq!(super::ago(5), "just now");
        assert_eq!(super::ago(300), "5m ago");
        assert_eq!(super::ago(7200), "2h ago");
        assert_eq!(super::ago(3 * 86_400), "3d ago");
        assert_eq!(super::ago(30 * 86_400), "4w ago");
    }
}
