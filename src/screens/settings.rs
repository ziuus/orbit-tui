//! Settings overlay: grouped, searchable, keyboard and mouse driven.
//!
//! `/` filters by name, group or description; ↑↓ move; ←→ / enter / space
//! change the value; a click selects a row and a second click changes it.

use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Clear, Paragraph};
use ratatui::Frame;

use crate::app::App;
use crate::config::WidgetConfig;
use crate::screens::{hit, Hit};

fn centered(area: Rect, w: u16, h: u16) -> Rect {
    let w = w.min(area.width);
    let h = h.min(area.height);
    Rect::new(
        area.x + (area.width - w) / 2,
        area.y + (area.height - h) / 2,
        w,
        h,
    )
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SettingType {
    DashboardPreset,
    MonitorLayout,
    FocusLayout,
    ClockFont,
    ClockStyle,
    Theme,
    DesignStyle,
    HighContrast,
    NightHours,
    Transparent,
    GaugeStyle,
    GraphStyle,
    MeterStyle,
    MotionEnabled,
    MotionMode,
    MotionSpeed,
    Visualizer,
    PerformanceMode,
    RefreshRate,
    Fps,
    Clock24h,
    AmbientRotate,
    FocusMinutes,
    BreakMinutes,
    Mouse,
    Notify,
    QuietHours,
    BatteryAlertPct,
    DiskAlertPct,
    WidgetCpu,
    WidgetMemory,
    WidgetDisk,
    WidgetNetwork,
    WidgetGpu,
    WidgetClock,
    WidgetCalendar,
    WidgetMusicViz,
    WidgetProcesses,
    WidgetMedia,
    WidgetMatrix,
    WidgetVideo,
    WidgetPinnedMedia,
    ImageQuality,
    SystemLogo,
}

pub struct Item {
    pub kind: SettingType,
    pub group: &'static str,
    pub label: &'static str,
    pub about: &'static str,
}

const fn item(
    kind: SettingType,
    group: &'static str,
    label: &'static str,
    about: &'static str,
) -> Item {
    Item {
        kind,
        group,
        label,
        about,
    }
}

use SettingType as S;

pub const SETTINGS_ITEMS: &[Item] = &[
    item(
        S::Theme,
        "appearance",
        "Theme",
        "Colour palette. T cycles it from any page.",
    ),
    item(
        S::DesignStyle,
        "appearance",
        "Design style",
        "The shape of the whole UI: soft, minimal, brutalist, glass, retro or neon.",
    ),
    item(
        S::NightHours,
        "appearance",
        "Night dimming",
        "Dim every colour during these hours.",
    ),
    item(
        S::HighContrast,
        "appearance",
        "High contrast",
        "Maximize contrast for outdoor or sunlight readability.",
    ),
    item(
        S::Transparent,
        "appearance",
        "Transparent background",
        "Let the terminal's own background show through.",
    ),
    item(
        S::ImageQuality,
        "appearance",
        "Image engine",
        "Pixel-perfect images where the terminal supports them, or braille.",
    ),
    item(
        S::SystemLogo,
        "appearance",
        "System logo",
        "Your distro's logo or the Vanta robot in the system panel.",
    ),
    item(
        S::ClockFont,
        "clock",
        "Clock font",
        "Digit shapes for the big clocks.",
    ),
    item(
        S::ClockStyle,
        "clock",
        "Clock style",
        "How the digits are filled in.",
    ),
    item(
        S::Clock24h,
        "clock",
        "24-hour clock",
        "Show 14:30 instead of 2:30 pm.",
    ),
    item(
        S::GaugeStyle,
        "widgets",
        "Gauge style",
        "Look of the cpu/mem dials. g cycles it.",
    ),
    item(
        S::GraphStyle,
        "widgets",
        "Graph style",
        "History graphs in blocks or braille. G cycles it.",
    ),
    item(
        S::MeterStyle,
        "widgets",
        "Meter style",
        "Look of the usage bars. m cycles it.",
    ),
    item(
        S::Visualizer,
        "widgets",
        "Visualizer",
        "Audio visualizer style. v cycles it.",
    ),
    item(
        S::DashboardPreset,
        "layout",
        "Overview layout",
        "Which panels the Overview page shows, and where.",
    ),
    item(
        S::MonitorLayout,
        "layout",
        "Monitor layout",
        "classic, processes-first, side-by-side, or graphs-first.",
    ),
    item(
        S::FocusLayout,
        "layout",
        "Focus layout",
        "classic, writer (big notes), planner (day at a glance) or files.",
    ),
    item(
        S::AmbientRotate,
        "ambient",
        "Scene rotation",
        "How long each Ambient scene stays before the next.",
    ),
    item(
        S::MotionEnabled,
        "ambient",
        "Motion",
        "Animate 3D and moving scenes. o toggles it.",
    ),
    item(
        S::MotionMode,
        "ambient",
        "Motion mode",
        "How the 3D object moves.",
    ),
    item(
        S::MotionSpeed,
        "ambient",
        "Motion speed",
        "Animation speed multiplier.",
    ),
    item(
        S::FocusMinutes,
        "focus",
        "Focus length",
        "Minutes per focus session.",
    ),
    item(
        S::BreakMinutes,
        "focus",
        "Break length",
        "Minutes per short break.",
    ),
    item(
        S::PerformanceMode,
        "performance",
        "Performance profile",
        "Trade smoothness for CPU. Normal is best for 24/7.",
    ),
    item(
        S::RefreshRate,
        "performance",
        "Refresh rate",
        "Seconds between system samples. + / - adjust it.",
    ),
    item(
        S::Fps,
        "performance",
        "Frame rate cap",
        "Maximum redraws per second while something animates.",
    ),
    item(
        S::Mouse,
        "input",
        "Mouse",
        "Click, scroll and select. Off restores terminal text selection.",
    ),
    item(
        S::Notify,
        "alerts",
        "Desktop notifications",
        "Low battery, overheating, full disk and focus-timer alerts.",
    ),
    item(
        S::QuietHours,
        "alerts",
        "Quiet hours",
        "Silence desktop notifications during these hours.",
    ),
    item(
        S::BatteryAlertPct,
        "alerts",
        "Battery alert %",
        "Alert when remaining battery drops to this percentage.",
    ),
    item(
        S::DiskAlertPct,
        "alerts",
        "Disk alert %",
        "Alert when root disk fills to this percentage.",
    ),
    item(S::WidgetCpu, "panels", "CPU", "Show the CPU panel."),
    item(
        S::WidgetMemory,
        "panels",
        "Memory",
        "Show the memory panel.",
    ),
    item(S::WidgetDisk, "panels", "Disk", "Show the disk panels."),
    item(
        S::WidgetNetwork,
        "panels",
        "Network",
        "Show the network panel.",
    ),
    item(S::WidgetGpu, "panels", "GPU", "Show the GPU panel."),
    item(S::WidgetClock, "panels", "Clock", "Show the clock panel."),
    item(
        S::WidgetCalendar,
        "panels",
        "Calendar",
        "Show the calendar panel.",
    ),
    item(
        S::WidgetMedia,
        "panels",
        "Now playing",
        "Show the media panel.",
    ),
    item(
        S::WidgetMusicViz,
        "panels",
        "Visualizer",
        "Show the audio visualizer.",
    ),
    item(
        S::WidgetProcesses,
        "panels",
        "Processes",
        "Show the top-processes panel.",
    ),
    item(
        S::WidgetMatrix,
        "panels",
        "Matrix rain",
        "Include the Rain scene.",
    ),
    item(
        S::WidgetVideo,
        "panels",
        "3D object",
        "Include the Orbit scene.",
    ),
    item(
        S::WidgetPinnedMedia,
        "panels",
        "Pinned image",
        "Include the Gallery scene.",
    ),
];

/// The items matching the current search, in display order.
pub fn filtered(query: &str) -> Vec<&'static Item> {
    let q = query.trim().to_lowercase();
    SETTINGS_ITEMS
        .iter()
        .filter(|i| {
            q.is_empty()
                || i.label.to_lowercase().contains(&q)
                || i.group.contains(&q)
                || i.about.to_lowercase().contains(&q)
        })
        .collect()
}

enum Value {
    Switch(bool),
    Text(String),
}

fn widget_flag(w: &mut WidgetConfig, kind: SettingType) -> Option<&mut bool> {
    Some(match kind {
        S::WidgetCpu => &mut w.cpu,
        S::WidgetMemory => &mut w.memory,
        S::WidgetDisk => &mut w.disk,
        S::WidgetNetwork => &mut w.network,
        S::WidgetGpu => &mut w.gpu,
        S::WidgetClock => &mut w.clock,
        S::WidgetCalendar => &mut w.calendar,
        S::WidgetMusicViz => &mut w.music_viz,
        S::WidgetProcesses => &mut w.processes,
        S::WidgetMedia => &mut w.media,
        S::WidgetMatrix => &mut w.matrix,
        S::WidgetVideo => &mut w.video,
        S::WidgetPinnedMedia => &mut w.pinned_media,
        _ => return None,
    })
}

fn value(app: &App, kind: SettingType) -> Value {
    let ui = &app.config.ui;
    if let Some(on) = widget_flag(&mut app.config.widgets.clone(), kind) {
        return Value::Switch(*on);
    }
    Value::Text(match kind {
        S::DashboardPreset => app.config.dashboard.preset.clone(),
        S::MonitorLayout => ui.monitor_layout.clone(),
        S::FocusLayout => ui.focus_layout.clone(),
        S::Theme => ui.theme.clone(),
        S::DesignStyle => ui.style.label().to_string(),
        S::HighContrast => return Value::Switch(ui.high_contrast),
        S::NightHours => match ui.night_hours.trim() {
            "" => return Value::Switch(false),
            h => h.to_string(),
        },
        S::Transparent => {
            return Value::Switch(ui.transparent.unwrap_or_else(|| !app.theme.is_light()))
        }
        S::GaugeStyle => ui.gauge_style.clone(),
        S::GraphStyle => ui.graph_style.clone(),
        S::MeterStyle => ui.meter_style.clone(),
        S::MotionEnabled => return Value::Switch(ui.motion_enabled),
        S::MotionMode => ui.motion_mode.clone(),
        S::MotionSpeed => format!("{:.2}×", ui.motion_speed),
        S::Visualizer => ui.visualizer.clone(),
        S::ClockFont => ui.clock_font.clone(),
        S::ClockStyle => ui.clock_style.clone(),
        S::PerformanceMode => ui.performance_mode.label().to_lowercase(),
        S::RefreshRate => format!("{:.1}s", ui.refresh_rate),
        S::Fps => format!("{} fps", ui.fps),
        S::Clock24h => return Value::Switch(ui.clock_24h),
        S::AmbientRotate => match ui.ambient_rotate_secs {
            0 => "never".to_string(),
            s if s < 60 => format!("{}s", s),
            s => format!("{} min", s / 60),
        },
        S::FocusMinutes => format!("{} min", ui.focus_minutes),
        S::BreakMinutes => format!("{} min", ui.break_minutes),
        S::Mouse => return Value::Switch(ui.mouse),
        S::Notify => return Value::Switch(ui.notify),
        S::QuietHours => match ui.quiet_hours.trim() {
            "" => return Value::Switch(false),
            h => h.to_string(),
        },
        S::BatteryAlertPct => format!("{}%", ui.battery_alert_pct),
        S::DiskAlertPct => format!("{}%", ui.disk_alert_pct),
        S::ImageQuality => if app.panel_states.pixel_images {
            "braille"
        } else {
            "high-res"
        }
        .to_string(),
        S::SystemLogo => if app.panel_states.force_robot_logo {
            "vanta robot"
        } else {
            "distro logo"
        }
        .to_string(),
        _ => String::new(),
    })
}

/// A display row: a group heading, or the item at this filtered index.
enum Row {
    Group(&'static str),
    Item(usize),
}

fn rows(items: &[&'static Item]) -> Vec<Row> {
    let mut out = Vec::new();
    let mut last = "";
    for (i, it) in items.iter().enumerate() {
        if it.group != last {
            out.push(Row::Group(it.group));
            last = it.group;
        }
        out.push(Row::Item(i));
    }
    out
}

pub fn render(f: &mut Frame, area: Rect, app: &mut App) {
    let theme = app.theme.clone();
    let base = Style::default().bg(theme.surface).fg(theme.text);
    let items = filtered(&app.settings_search);
    app.settings_row = app.settings_row.min(items.len().saturating_sub(1));
    let rows = rows(&items);

    let w = area.width.saturating_sub(4).clamp(40, 68);
    // search + blank + rows + blank + description, plus the border.
    let want = rows.len() as u16 + 6;
    let box_area = centered(area, w, want.min(area.height.saturating_sub(2)).max(8));
    let inner_w = box_area.width.saturating_sub(2) as usize;
    let list_h = box_area.height.saturating_sub(6) as usize;

    // Keep the selection on screen, pulling its group heading in with it.
    let sel_row = rows
        .iter()
        .position(|r| matches!(r, Row::Item(i) if *i == app.settings_row))
        .unwrap_or(0);
    let want_top = sel_row.saturating_sub(1);
    if want_top < app.settings_scroll {
        app.settings_scroll = want_top;
    } else if sel_row >= app.settings_scroll + list_h {
        app.settings_scroll = sel_row + 1 - list_h;
    }
    app.settings_scroll = app.settings_scroll.min(rows.len().saturating_sub(list_h));

    let mut lines: Vec<Line> = Vec::new();
    // Search bar.
    let count = format!("{}/{} ", items.len(), SETTINGS_ITEMS.len());
    let (query, qstyle) = if app.settings_search_active || !app.settings_search.is_empty() {
        (
            format!(
                "{}{}",
                app.settings_search,
                if app.settings_search_active {
                    "▏"
                } else {
                    ""
                }
            ),
            base.fg(theme.text),
        )
    } else {
        ("type / to search".to_string(), base.fg(theme.dim))
    };
    let pad = inner_w.saturating_sub(3 + query.chars().count() + count.chars().count());
    lines.push(Line::from(vec![
        Span::styled(" ⌕ ", base.fg(theme.accent)),
        Span::styled(query, qstyle),
        Span::styled(" ".repeat(pad), base),
        Span::styled(count, base.fg(theme.dim)),
    ]));
    lines.push(Line::from(Span::styled(
        "─".repeat(inner_w),
        base.fg(theme.bg),
    )));

    // The box first, so the rows registered below sit on top of it.
    hit(box_area, Hit::Overlay);
    let list_y = box_area.y + 3;
    for (n, row) in rows
        .iter()
        .enumerate()
        .skip(app.settings_scroll)
        .take(list_h)
    {
        match row {
            Row::Group(g) => {
                let head = format!(" {} ", g.to_uppercase());
                let fill = inner_w.saturating_sub(head.chars().count() + 1);
                lines.push(Line::from(vec![
                    Span::styled(head, base.fg(theme.secondary).add_modifier(Modifier::BOLD)),
                    Span::styled("┈".repeat(fill), base.fg(theme.dim)),
                ]));
            }
            Row::Item(i) => {
                let it = items[*i];
                let sel = *i == app.settings_row;
                let row_style = if sel {
                    Style::default().bg(theme.accent).fg(theme.bg)
                } else {
                    base
                };
                let (val, vstyle) = match value(app, it.kind) {
                    Value::Switch(true) => (
                        "● on ".to_string(),
                        if sel { row_style } else { base.fg(theme.green) },
                    ),
                    Value::Switch(false) => (
                        "○ off".to_string(),
                        if sel { row_style } else { base.fg(theme.dim) },
                    ),
                    Value::Text(t) if sel => (format!("‹ {} ›", t), row_style),
                    Value::Text(t) => (t, base.fg(theme.accent)),
                };
                let label = format!("{}  {}", if sel { "▸" } else { " " }, it.label);
                let gap = inner_w.saturating_sub(label.chars().count() + val.chars().count() + 3);
                lines.push(Line::from(vec![
                    Span::styled(format!(" {}", label), row_style),
                    Span::styled(" ".repeat(gap), row_style),
                    Span::styled(val, vstyle),
                    Span::styled("  ", row_style),
                ]));
                hit(
                    Rect::new(
                        box_area.x + 1,
                        list_y + (n - app.settings_scroll) as u16,
                        box_area.width.saturating_sub(2),
                        1,
                    ),
                    Hit::SettingRow(*i),
                );
            }
        }
    }
    if items.is_empty() {
        lines.push(Line::from(Span::styled(
            "   no settings match",
            base.fg(theme.dim),
        )));
    }
    while lines.len() < list_h + 2 {
        lines.push(Line::from(Span::styled("", base)));
    }
    lines.push(Line::from(Span::styled("", base)));
    let about = items.get(app.settings_row).map_or("", |i| i.about);
    lines.push(Line::from(Span::styled(
        format!(
            " {}",
            crate::widgets::meter::ellipsize(about, inner_w.saturating_sub(2))
        ),
        base.fg(theme.dim).add_modifier(Modifier::ITALIC),
    )));

    f.render_widget(Clear, box_area);
    f.render_widget(
        Paragraph::new(lines).style(base).block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(theme.accent))
                .title(Span::styled(
                    " settings ",
                    Style::default().fg(theme.accent),
                ))
                .title_bottom(
                    Line::from(Span::styled(
                        " ↑↓ move · ←→ change · / search · esc close ",
                        Style::default().fg(theme.dim),
                    ))
                    .right_aligned(),
                )
                .style(base),
        ),
        box_area,
    );
}

pub fn handle_key(app: &mut App, key: crossterm::event::KeyCode) {
    use crossterm::event::KeyCode::*;
    let n = filtered(&app.settings_search).len();
    if app.settings_search_active {
        match key {
            Esc => {
                app.settings_search.clear();
                app.settings_search_active = false;
            }
            Enter => app.settings_search_active = false,
            Backspace => {
                app.settings_search.pop();
                app.settings_row = 0;
            }
            Up | Down => {}
            Char(c) => {
                app.settings_search.push(c);
                app.settings_row = 0;
            }
            _ => {}
        }
        if !matches!(key, Up | Down) {
            return;
        }
    }
    let step = |row: usize, by: isize| -> usize {
        if n == 0 {
            0
        } else {
            (row as isize + by).rem_euclid(n as isize) as usize
        }
    };
    match key {
        Up | Char('k') => app.settings_row = step(app.settings_row, -1),
        Down | Char('j') => app.settings_row = step(app.settings_row, 1),
        PageUp => app.settings_row = app.settings_row.saturating_sub(8),
        PageDown => app.settings_row = (app.settings_row + 8).min(n.saturating_sub(1)),
        Home => app.settings_row = 0,
        End => app.settings_row = n.saturating_sub(1),
        Char('/') => app.settings_search_active = true,
        Left | Char('h') => change_setting(app, false),
        Right | Char('l') | Enter | Char(' ') => change_setting(app, true),
        Esc if !app.settings_search.is_empty() => app.settings_search.clear(),
        Esc | Char('q') | Char('S') => app.show_settings = false,
        _ => {}
    }
}

/// Cycle `cur` through `opts` (by position when it's one of them, else to
/// the first option).
fn cycle<T: PartialEq + Clone>(opts: &[T], cur: &T, forward: bool) -> T {
    let n = opts.len();
    let pos = opts.iter().position(|x| x == cur);
    let i = match (pos, forward) {
        (Some(p), true) => (p + 1) % n,
        (Some(p), false) => (p + n - 1) % n,
        (None, _) => 0,
    };
    opts[i].clone()
}

pub fn change_setting(app: &mut App, forward: bool) {
    let Some(kind) = filtered(&app.settings_search)
        .get(app.settings_row)
        .map(|i| i.kind)
    else {
        return;
    };
    if let Some(flag) = widget_flag(&mut app.config.widgets, kind) {
        *flag = !*flag;
        app.config.save();
        return;
    }
    let ui = &mut app.config.ui;
    match kind {
        S::DashboardPreset => {
            let presets = [
                "mirador",
                "cockpit",
                "monitoring",
                "minimal",
                "aesthetic",
                "workspace",
            ]
            .map(String::from);
            let next = cycle(&presets, &app.config.dashboard.preset, forward);
            app.config.dashboard.apply_preset(&next);
        }
        S::Theme => app.cycle_theme(),
        S::MonitorLayout => {
            let opts = crate::screens::monitor::LAYOUTS.map(String::from);
            ui.monitor_layout = cycle(&opts, &ui.monitor_layout, forward);
        }
        S::FocusLayout => {
            let opts = crate::screens::workspace::LAYOUTS.map(String::from);
            ui.focus_layout = cycle(&opts, &ui.focus_layout, forward);
        }
        S::DesignStyle => return app.cycle_style(forward),
        S::HighContrast => {
            ui.high_contrast = !ui.high_contrast;
            app.refresh_theme();
        }
        S::NightHours => {
            let presets = crate::config::NIGHT_PRESETS.map(String::from);
            ui.night_hours = cycle(&presets, &ui.night_hours.trim().to_string(), forward);
            app.night = None;
        }
        S::Transparent => {
            let current = ui.transparent.unwrap_or_else(|| !app.theme.is_light());
            ui.transparent = Some(!current);
        }
        S::GaugeStyle => {
            crate::widgets::gauge::cycle_style();
            ui.gauge_style = crate::widgets::gauge::style_name().to_string();
        }
        S::GraphStyle => {
            crate::widgets::block_graph::cycle_style();
            ui.graph_style = crate::widgets::block_graph::style_name().to_string();
        }
        S::MeterStyle => {
            crate::widgets::meter::cycle_style();
            ui.meter_style = crate::widgets::meter::style_name().to_string();
        }
        S::MotionEnabled => ui.motion_enabled = !ui.motion_enabled,
        S::MotionMode => {
            let modes = ["spin", "tumble", "wobble", "swing"].map(String::from);
            ui.motion_mode = cycle(&modes, &ui.motion_mode, forward);
        }
        S::MotionSpeed => {
            let speeds = [0.25, 0.5, 0.75, 1.0, 1.25, 1.5, 2.0, 3.0];
            let pos = speeds
                .iter()
                .position(|&x| (x - ui.motion_speed).abs() < 0.1)
                .unwrap_or(3);
            ui.motion_speed = if forward {
                speeds[(pos + 1).min(speeds.len() - 1)]
            } else {
                speeds[pos.saturating_sub(1)]
            };
        }
        S::Visualizer => {
            crate::widgets::music_viz::cycle_style();
            ui.visualizer = crate::widgets::music_viz::style_name().to_string();
        }
        S::PerformanceMode => {
            use crate::config::PerformanceMode as P;
            let modes = [P::VeryLight, P::Light, P::Normal, P::High, P::VeryHigh];
            ui.performance_mode = cycle(&modes, &ui.performance_mode, forward);
            let mode = ui.performance_mode.clone();
            app.apply_performance_mode(&mode);
        }
        S::RefreshRate => app.adjust_refresh(forward),
        S::Fps => {
            ui.fps = if forward {
                ui.fps + 5
            } else {
                ui.fps.saturating_sub(5)
            }
            .clamp(5, 120)
        }
        S::ClockFont => {
            let fonts = ["minimal", "standard", "rounded", "digital"].map(String::from);
            ui.clock_font = cycle(&fonts, &ui.clock_font, forward);
        }
        S::ClockStyle => {
            let styles =
                ["braille", "minimal", "outline", "solid", "dotted", "hollow"].map(String::from);
            ui.clock_style = cycle(&styles, &ui.clock_style, forward);
        }
        S::Clock24h => ui.clock_24h = !ui.clock_24h,
        S::AmbientRotate => {
            ui.ambient_rotate_secs = cycle(
                &[60, 120, 300, 600, 1800, 0],
                &ui.ambient_rotate_secs,
                forward,
            )
        }
        S::FocusMinutes => {
            let opts: Vec<u64> = (15..=60).step_by(5).collect();
            ui.focus_minutes = cycle(&opts, &ui.focus_minutes, forward);
        }
        S::BreakMinutes => {
            ui.break_minutes = cycle(&[3, 5, 10, 15], &ui.break_minutes, forward);
        }
        S::Mouse => {
            ui.mouse = !ui.mouse;
            if ui.mouse {
                crate::mouse::enable();
            } else {
                crate::mouse::disable();
            }
        }
        S::Notify => ui.notify = !ui.notify,
        S::QuietHours => {
            let presets = crate::config::NIGHT_PRESETS.map(String::from);
            ui.quiet_hours = cycle(&presets, &ui.quiet_hours.trim().to_string(), forward);
        }
        S::BatteryAlertPct => {
            let opts: [u8; 5] = [10, 15, 20, 25, 30];
            ui.battery_alert_pct = cycle(&opts, &ui.battery_alert_pct, forward);
        }
        S::DiskAlertPct => {
            let opts: [u8; 4] = [80, 85, 90, 95];
            ui.disk_alert_pct = cycle(&opts, &ui.disk_alert_pct, forward);
        }
        S::ImageQuality => app.panel_states.pixel_images = !app.panel_states.pixel_images,
        S::SystemLogo => app.panel_states.force_robot_logo = !app.panel_states.force_robot_logo,
        _ => {}
    }
    app.config.save();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn search_matches_label_group_and_description() {
        assert_eq!(filtered("").len(), SETTINGS_ITEMS.len());
        assert!(filtered("mouse").iter().any(|i| i.kind == S::Mouse));
        assert!(filtered("PANELS").iter().all(|i| i.group == "panels"
            || i.label.to_lowercase().contains("panels")
            || i.about.to_lowercase().contains("panels")));
        assert!(filtered("dim").iter().any(|i| i.kind == S::NightHours));
        assert!(filtered("zzzz").is_empty());
    }

    #[test]
    fn groups_are_contiguous_so_each_heading_appears_once() {
        let items = filtered("");
        let heads: Vec<&str> = rows(&items)
            .iter()
            .filter_map(|r| match r {
                Row::Group(g) => Some(*g),
                Row::Item(_) => None,
            })
            .collect();
        let mut dedup = heads.clone();
        dedup.dedup();
        let mut sorted = heads.clone();
        sorted.sort();
        sorted.dedup();
        assert_eq!(
            heads.len(),
            sorted.len(),
            "a group heading repeats: {heads:?}"
        );
        assert_eq!(heads, dedup);
    }

    #[test]
    fn cycle_wraps_both_ways_and_resets_unknown_values() {
        assert_eq!(cycle(&[1, 2, 3], &3, true), 1);
        assert_eq!(cycle(&[1, 2, 3], &1, false), 3);
        assert_eq!(cycle(&[1, 2, 3], &9, true), 1);
    }
}
