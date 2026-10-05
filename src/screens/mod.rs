pub mod aesthetic;
pub mod dashboard;
pub mod doctor;
pub mod debug_logs;
pub mod help;
pub mod menu;
pub mod monitor;
pub mod notifications;
pub mod settings;
pub mod workspace;

use ratatui::layout::{Alignment, Rect};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Paragraph};
use ratatui::Frame;

use crate::theme::Theme;

/// Something the mouse can land on, recorded while a frame is drawn.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Hit {
    /// The n-th page tab in the title bar.
    Tab(usize),
    /// A panel's outer area.
    Panel(crate::app::PanelId),
    /// A list inside `panel` showing one item per row, the top row being
    /// item `first`.
    Rows {
        panel: crate::app::PanelId,
        first: usize,
    },
    /// A process-table column header.
    SortBy(crate::app::SortField),
    /// The note preview pane (wheel scrolls the note, not the list).
    NotePreview,
    /// The full-screen Ambient stage.
    Scene,
    /// The n-th visible settings row.
    SettingRow(usize),
    /// A modal overlay's box (clicks inside don't close it).
    Overlay,
    /// The n-th Esc-menu item.
    MenuRow(usize),
    /// The notification center trigger.
    Notifications,
}

static HITS: std::sync::Mutex<Vec<(Rect, Hit)>> = std::sync::Mutex::new(Vec::new());

/// Forget last frame's targets. Called at the start of every render.
pub fn clear_hits() {
    HITS.lock().unwrap_or_else(|e| e.into_inner()).clear();
}

/// Record that `area` is `hit` for this frame. Later calls sit on top.
pub fn hit(area: Rect, hit: Hit) {
    if area.width > 0 && area.height > 0 {
        HITS.lock()
            .unwrap_or_else(|e| e.into_inner())
            .push((area, hit));
    }
}

/// Everything under (x, y), topmost first, with each target's area.
pub fn hits_at(x: u16, y: u16) -> Vec<(Hit, Rect)> {
    let pos = ratatui::layout::Position { x, y };
    HITS.lock()
        .unwrap_or_else(|e| e.into_inner())
        .iter()
        .rev()
        .filter(|(r, _)| r.contains(pos))
        .map(|(r, h)| (*h, *r))
        .collect()
}

static DESIGN: std::sync::atomic::AtomicU8 = std::sync::atomic::AtomicU8::new(0);

/// Select the design style every panel is drawn in. Set once per frame.
pub fn set_design(style: crate::config::DesignStyle) {
    let i = crate::config::DesignStyle::ALL
        .iter()
        .position(|s| *s == style)
        .unwrap_or(0);
    DESIGN.store(i as u8, std::sync::atomic::Ordering::Relaxed);
}

pub fn design() -> crate::config::DesignStyle {
    let all = crate::config::DesignStyle::ALL;
    all[DESIGN.load(std::sync::atomic::Ordering::Relaxed) as usize % all.len()]
}

/// Panel chrome shared by every page. Its shape follows the design style;
/// its colours follow the theme. Returns the inner area.
pub fn panel_full(
    f: &mut Frame,
    area: Rect,
    title: &str,
    right_title: Option<&str>,
    footer: Option<&str>,
    theme: &Theme,
    focused: bool,
) -> Rect {
    use crate::config::DesignStyle as D;
    use crate::theme::blend;
    use ratatui::style::Modifier;

    let style = design();
    let (border, text) = match (style, focused) {
        (_, true) => (theme.accent, theme.accent),
        (D::Neon, false) => (blend(theme.accent, theme.bg, 0.35), theme.secondary),
        (D::Cyber, false) => (blend(theme.secondary, theme.bg, 0.45), theme.accent),
        (D::Glass, false) => (blend(theme.surface, theme.text, 0.28), theme.text),
        (D::Material, false) => (blend(theme.surface, theme.accent, 0.25), theme.text),
        (D::Brutalist, false) => (theme.dim, theme.text),
        (D::Retro, false) => (theme.dim, theme.text),
        _ => (theme.border(), theme.dim),
    };
    let fill = match style {
        D::Glass => Some(blend(theme.bg, theme.text, 0.075)),
        D::Material => Some(blend(theme.bg, theme.surface, 0.35)),
        _ => None,
    };

    // Title text in the style's voice.
    let tag = |t: &str, color| -> Span<'static> {
        match style {
            D::Brutalist => Span::styled(
                format!(" {} ", t.to_uppercase()),
                Style::default()
                    .fg(theme.bg)
                    .bg(color)
                    .add_modifier(Modifier::BOLD),
            ),
            D::Retro => Span::styled(
                format!("[ {} ]", t.to_uppercase()),
                Style::default().fg(color),
            ),
            D::Neon => Span::styled(
                format!("╸{}╺", t),
                Style::default().fg(color).add_modifier(Modifier::BOLD),
            ),
            D::Cyber => Span::styled(
                format!("◢ {} ◣", t.to_uppercase()),
                Style::default().fg(color).add_modifier(Modifier::BOLD),
            ),
            D::Material => Span::styled(
                format!(" ▰ {} ", t),
                Style::default().fg(color).add_modifier(Modifier::BOLD),
            ),
            D::Minimal => {
                Span::styled(format!("{} ", t.to_lowercase()), Style::default().fg(color))
            }
            _ => Span::styled(format!(" {} ", t), Style::default().fg(color)),
        }
    };

    let (borders, border_type) = match style {
        D::Soft | D::Glass | D::Neon => (Borders::ALL, BorderType::Rounded),
        D::Cyber => (Borders::ALL, BorderType::Rounded),
        D::Material => (Borders::ALL, BorderType::Plain),
        D::Brutalist => (Borders::ALL, BorderType::Thick),
        D::Retro => (Borders::ALL, BorderType::Double),
        D::Minimal => (Borders::TOP, BorderType::Plain),
    };
    let border_style = match style {
        D::Minimal if !focused => Style::default().fg(theme.border()),
        _ => Style::default().fg(border),
    };
    let mut block = Block::default()
        .borders(borders)
        .border_type(border_type)
        .border_style(border_style)
        .title_top(Line::from(tag(title, text)));
    if let Some(bg) = fill {
        block = block.style(Style::default().bg(bg));
    }

    if let Some(rt) = right_title {
        let left_len = title.chars().count() + 4;
        let avail = (area.width as usize).saturating_sub(left_len + 4);
        let rt = rt.trim();
        let shown = if rt.chars().count() + 4 <= avail {
            rt.to_string()
        } else if avail >= 8 {
            format!("{}…", rt.chars().take(avail - 5).collect::<String>())
        } else {
            String::new()
        };
        if !shown.is_empty() {
            let span = match style {
                D::Brutalist => Span::styled(
                    format!(" {} ", shown.to_uppercase()),
                    Style::default().fg(theme.bg).bg(theme.dim),
                ),
                D::Retro => Span::styled(format!("[ {} ]", shown), Style::default().fg(theme.dim)),
                D::Cyber => Span::styled(
                    format!("// {} //", shown),
                    Style::default().fg(theme.secondary),
                ),
                D::Material => {
                    Span::styled(format!("· {} ", shown), Style::default().fg(theme.dim))
                }
                _ => Span::styled(format!(" {} ", shown), Style::default().fg(theme.dim)),
            };
            block = block.title_top(Line::from(span).alignment(Alignment::Right));
        }
    }

    if let Some(ft) = footer.filter(|_| focused && borders.contains(Borders::BOTTOM)) {
        if area.width as usize >= ft.chars().count() + 4 {
            block = block.title_bottom(
                Line::from(Span::styled(
                    format!(" {} ", ft),
                    Style::default().fg(theme.accent),
                ))
                .alignment(Alignment::Center),
            );
        }
    }

    let mut inner = block.inner(area);
    // Minimal has no side borders; keep the content off the panel edges.
    if style == D::Minimal && inner.width > 2 {
        inner.x += 1;
        inner.width -= 2;
    }
    f.render_widget(block, area);
    inner
}

pub fn panel(f: &mut Frame, area: Rect, title: &str, theme: &Theme, focused: bool) -> Rect {
    panel_full(f, area, title, None, None, theme, focused)
}

/// Centred notice for when a page can't fit. `need` is the page area; the
/// message speaks in terminal size (page + title and status bars).
pub fn too_small(f: &mut Frame, area: Rect, theme: &Theme, need: (u16, u16)) {
    let term = f.area();
    let msg = format!(
        "terminal too small — need {}×{}, have {}×{}",
        need.0,
        need.1 + 2,
        term.width,
        term.height
    );
    let y = area.y + area.height / 2;
    f.render_widget(
        Paragraph::new(Line::from(Span::styled(
            msg,
            Style::default().fg(theme.dim),
        )))
        .alignment(Alignment::Center),
        Rect::new(area.x, y, area.width, 1),
    );
}

/// Render one panel filling `area` — used by zoom (Enter on a focused panel).
pub fn render_panel(f: &mut Frame, area: Rect, app: &mut crate::app::App, id: crate::app::PanelId) {
    use crate::app::PanelId as P;
    use crate::monitors::{cpu, disk, gpu, memory, network, system_info};
    use crate::widgets::{calendar, clock, gauge, matrix, media, music_viz, status, video};

    let theme = &app.theme;
    let title = if id == P::PinnedMedia && app.panel_states.pinned_media_input_active {
        format!(
            "media path: {}_ · zoomed · esc to return",
            app.panel_states.pinned_media_input
        )
    } else {
        format!("{} · zoomed · esc to return", id.label())
    };
    hit(area, Hit::Panel(id));
    let inner = panel(f, area, &title, theme, true);
    let sum = &app.summary;
    match id {
        P::System => system_info::render_neofetch(
            f,
            inner,
            theme,
            sum,
            (f.area().width, f.area().height),
            app.panel_states.force_robot_logo,
        ),
        P::Gauges => {
            let m = [
                (
                    "cpu",
                    sum.cpu_pct as f64,
                    format!("{:.0}%", sum.cpu_pct),
                    theme.usage(sum.cpu_pct as f64),
                ),
                (
                    "mem",
                    sum.mem_pct,
                    format!("{:.0}%", sum.mem_pct),
                    theme.usage(sum.mem_pct),
                ),
                (
                    "disk",
                    sum.disk_pct.unwrap_or(0.0),
                    format!("{:.0}%", sum.disk_pct.unwrap_or(0.0)),
                    theme.usage(sum.disk_pct.unwrap_or(0.0)),
                ),
            ];
            gauge::render(f, inner, theme, &m)
        }
        P::Cpu => cpu::render(f, inner, theme, true),
        P::Memory => memory::render(f, inner, theme, true),
        P::Disk => disk::render(f, inner, theme, true),
        P::Storage => disk::render_storage(f, inner, theme),
        P::Network => network::render(f, inner, theme, true),
        P::Gpu => gpu::render(f, inner, theme, true),
        P::Clock => clock::render(
            f,
            inner,
            theme,
            app.config.ui.clock_24h,
            &app.config.ui.clock_font,
            &app.config.ui.clock_style,
            &app.config.ui.timezones,
        ),
        P::Media => media::render(f, inner, app),
        P::Visualizer => music_viz::render(f, inner, theme, app.frame),
        P::Status => status::render(f, inner, theme, false, 0),
        P::Calendar => calendar::render(f, inner, theme, app.panel_states.calendar_month_offset),
        P::Matrix => matrix::render(f, inner, theme),
        P::Video => video::render(f, inner, theme, app.frame),
        P::Weather => crate::widgets::weather::render(f, inner, theme),
        P::UpNext => crate::widgets::upnext::render(f, inner, theme),
        P::Timer => crate::widgets::pomodoro::render(f, inner, theme, &app.config.ui, true),
        P::Agenda => crate::widgets::agenda::render(
            f,
            inner,
            theme,
            true,
            app.panel_states.agenda_selected,
            app.panel_states.agenda_input_active,
            &app.panel_states.agenda_input,
        ),
        P::Tasks => crate::widgets::tasks::render(
            f,
            inner,
            theme,
            true,
            app.panel_states.tasks_selected,
            app.panel_states.task_input_active,
            &app.panel_states.task_input,
        ),
        P::News => crate::widgets::news::render(f, inner, theme),
        P::PinnedMedia => {
            let path = app.config.ui.pinned_media_path.clone();
            let frame = app.frame;
            crate::widgets::pinned_media::render(f, inner, app, &path, frame);
        }
        P::WriterNotes => {}
        P::Files => {}
        P::Processes => {
            let ps = &app.panel_states;
            crate::monitors::processes::render(
                f,
                inner,
                theme,
                ps.process_scroll_offset,
                ps.process_sort_field,
                ps.process_sort_asc,
                &ps.process_search,
                ps.process_search_active,
                ps.process_tree_mode,
                &ps.process_collapsed,
                ps.process_selected_pid,
                ps.process_compact_cmd,
            )
        }
        P::Custom(idx) => {
            // The border with the zoom title is already drawn above (inner is
            // the area inside it).  Delegate pure content rendering.
            app.custom_widgets.render_widget_inner(f, inner, idx, theme);
        }
    }
}
pub mod setup_wizard;
