use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Clear, Paragraph, Wrap};
use ratatui::Frame;

use crate::app::App;
use crate::notify::{self, Urgency};
use crate::screens::{hit, Hit};
use crate::theme::blend;

pub fn render(f: &mut Frame, area: Rect, app: &mut App) {
    let theme = &app.theme;
    let list = notify::list();
    let count = list.len();

    // Dim the backdrop behind modal
    let buf = f.buffer_mut();
    for y in area.top()..area.bottom() {
        for x in area.left()..area.right() {
            if let Some(cell) = buf.cell_mut((x, y)) {
                let fg = cell.fg;
                if let Color::Rgb(..) = fg {
                    cell.set_fg(blend(fg, theme.bg, 0.70));
                }
                if let Color::Rgb(..) = cell.bg {
                    let bg = cell.bg;
                    cell.set_bg(blend(bg, theme.bg, 0.55));
                }
            }
        }
    }

    let w = 70u16.min(area.width.saturating_sub(4)).max(36);
    let h = 22u16.min(area.height.saturating_sub(4)).max(12);
    let box_area = Rect::new(
        area.x + area.width.saturating_sub(w) / 2,
        area.y + area.height.saturating_sub(h) / 2,
        w,
        h,
    );
    hit(box_area, Hit::Overlay);
    f.render_widget(Clear, box_area);

    let title = format!("NOTIFICATIONS ({})", count);
    let footer = "↑↓ select · d dismiss · c clear all · t test · esc close";
    let inner = crate::screens::panel_full(
        f,
        box_area,
        &title,
        Some(if count > 0 { "live feed" } else { "idle" }),
        Some(footer),
        theme,
        true,
    );

    if count == 0 {
        let empty_msg = vec![
            Line::from(""),
            Line::from(Span::styled(
                "No notifications recorded yet",
                Style::default().fg(theme.text).add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "System alerts (battery, temperature, memory, disk) and",
                Style::default().fg(theme.dim),
            )),
            Line::from(Span::styled(
                "timer events will automatically appear here in real time.",
                Style::default().fg(theme.dim),
            )),
            Line::from(""),
            Line::from(vec![
                Span::styled("Press ", Style::default().fg(theme.dim)),
                Span::styled(
                    "t",
                    Style::default()
                        .fg(theme.accent)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    " to emit a test notification right now.",
                    Style::default().fg(theme.dim),
                ),
            ]),
        ];
        f.render_widget(
            Paragraph::new(empty_msg)
                .alignment(Alignment::Center)
                .wrap(Wrap { trim: true }),
            inner,
        );
        return;
    }

    let sel = app.notif_selected.min(count.saturating_sub(1));
    app.notif_selected = sel;

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(6), Constraint::Length(5)])
        .split(inner);

    let list_area = chunks[0];
    let detail_area = chunks[1];

    let visible_rows = (list_area.height as usize).max(1);
    let scroll = if sel < app.notif_scroll {
        sel
    } else if sel >= app.notif_scroll + visible_rows {
        sel.saturating_sub(visible_rows - 1)
    } else {
        app.notif_scroll
    };
    app.notif_scroll = scroll;

    let mut lines = Vec::new();
    for (i, notif) in list.iter().enumerate().skip(scroll).take(visible_rows) {
        let is_sel = i == sel;
        let (badge_str, badge_fg) = match notif.urgency {
            Urgency::Critical => ("CRIT", theme.red),
            Urgency::Normal => ("WARN", theme.yellow),
        };
        let (bg, text_fg) = if is_sel {
            (theme.accent, theme.bg)
        } else {
            (theme.bg, theme.text)
        };

        let row_style = Style::default().bg(bg).fg(text_fg);
        let badge_style = if is_sel {
            Style::default()
                .bg(bg)
                .fg(theme.bg)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(badge_fg).add_modifier(Modifier::BOLD)
        };

        let mut spans = vec![
            Span::styled(if is_sel { " ▸ " } else { "   " }, row_style),
            Span::styled(format!("[{}] ", badge_str), badge_style),
            Span::styled(
                format!("{} ", notif.time),
                if is_sel {
                    row_style
                } else {
                    Style::default().fg(theme.dim)
                },
            ),
            Span::styled(
                format!(
                    "{:<20} ",
                    crate::widgets::meter::ellipsize(&notif.title, 20)
                ),
                row_style.add_modifier(if is_sel {
                    Modifier::BOLD
                } else {
                    Modifier::empty()
                }),
            ),
            Span::styled(
                crate::widgets::meter::ellipsize(
                    &notif.body,
                    list_area.width.saturating_sub(40) as usize,
                ),
                if is_sel {
                    row_style
                } else {
                    Style::default().fg(theme.dim)
                },
            ),
        ];

        let line_w: usize = spans.iter().map(|s| s.content.chars().count()).sum();
        if (list_area.width as usize) > line_w {
            spans.push(Span::styled(
                " ".repeat((list_area.width as usize) - line_w),
                row_style,
            ));
        }
        lines.push(Line::from(spans));
    }
    f.render_widget(Paragraph::new(lines), list_area);

    // Detail box for selected notification
    if let Some(selected) = list.get(sel) {
        let detail_block = Block::default()
            .borders(Borders::TOP)
            .border_type(BorderType::Plain)
            .border_style(Style::default().fg(theme.surface));
        let detail_inner = detail_block.inner(detail_area);
        f.render_widget(detail_block, detail_area);

        let d_lines = vec![
            Line::from(vec![
                Span::styled(
                    format!("● {} ", selected.title),
                    Style::default()
                        .fg(theme.accent)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    format!("· {} · alert #{}", selected.time, selected.id),
                    Style::default().fg(theme.dim),
                ),
            ]),
            Line::from(Span::styled(
                &selected.body,
                Style::default().fg(theme.text),
            )),
        ];
        f.render_widget(
            Paragraph::new(d_lines).wrap(Wrap { trim: true }),
            detail_inner,
        );
    }
}

pub fn handle_key(app: &mut App, key: crossterm::event::KeyCode) {
    use crossterm::event::KeyCode::*;
    let count = notify::list().len();
    match key {
        Up | Char('k') => {
            if app.notif_selected > 0 {
                app.notif_selected -= 1;
            }
        }
        Down | Char('j') => {
            if app.notif_selected + 1 < count {
                app.notif_selected += 1;
            }
        }
        Char('d') | Delete => {
            let list = notify::list();
            if let Some(selected) = list.get(app.notif_selected) {
                notify::dismiss(selected.id);
                if app.notif_selected >= count.saturating_sub(1) && app.notif_selected > 0 {
                    app.notif_selected -= 1;
                }
            }
        }
        Char('c') => {
            notify::clear();
            app.notif_selected = 0;
            app.notif_scroll = 0;
        }
        Char('t') => {
            notify::test_notification();
        }
        Esc | Char('q') => {
            app.show_notifications = false;
        }
        _ => {}
    }
}
