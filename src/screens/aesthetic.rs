//! Ambient: borderless full-screen scenes meant to be glanced at all day.
//! Scenes rotate on a timer (`ui.ambient_rotate_secs`); ←/→ step through
//! them and `r` toggles rotation.

use std::time::Instant;

use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Clear, Paragraph, Wrap};
use ratatui::Frame;

use crate::app::App;
use crate::screens::too_small;
use crate::theme::Theme;
use crate::widgets::{clock, matrix, media, music_viz, pinned_media, upnext, video, weather};

const MIN: (u16, u16) = (60, 20);

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Scene {
    Custom(String),
    /// Giant clock over a calm audio horizon. Static when silent: the
    /// cheapest scene, and the default.
    Horizon,
    /// A big retro flip clock whose minute card folds over on the minute.
    Flip,
    /// Matrix rain with the clock floating in the middle.
    Rain,
    /// A drifting contour map; load speeds it up, music ripples it.
    Topo,
    /// The spinning donut beside the time, weather and track.
    Orbit,
    /// Now playing: art, track and a big visualizer. Only while music plays.
    Studio,
    /// The pinned image with a small clock. Only when an image is pinned.
    Gallery,
    Starfield,
    Life,
    Snow,
    Creative,
}

impl Scene {
    fn label(&self) -> &str {
        match self {
            Scene::Custom(ref id) => id.as_str(),
            Scene::Horizon => "horizon",
            Scene::Flip => "flip",
            Scene::Topo => "topography",
            Scene::Rain => "rain",
            Scene::Orbit => "orbit",
            Scene::Studio => "studio",
            Scene::Gallery => "gallery",
            Scene::Starfield => "starfield",
            Scene::Life => "life",
            Scene::Snow => "snow",
            Scene::Creative => "creative",
        }
    }
}

/// Rotation state. The scene is derived from wall time, so rendering stays
/// read-only: `index = base + elapsed / rotate_secs`.
#[derive(Clone, Debug)]
pub struct AmbientState {
    base: usize,
    anchor: Instant,
    pub auto: bool,
}

impl Default for AmbientState {
    fn default() -> Self {
        Self {
            base: 0,
            anchor: Instant::now(),
            auto: true,
        }
    }
}

impl AmbientState {
    fn index(&self, rotate_secs: u64, n: usize) -> usize {
        let steps = if self.auto && rotate_secs > 0 {
            (self.anchor.elapsed().as_secs() / rotate_secs) as usize
        } else {
            0
        };
        (self.base + steps) % n.max(1)
    }

    /// Move `delta` scenes from the one currently shown and restart the timer.
    pub fn step(&mut self, delta: isize, app_rotate_secs: u64, n: usize) {
        let n = n.max(1);
        let cur = self.index(app_rotate_secs, n) as isize;
        self.base = (cur + delta).rem_euclid(n as isize) as usize;
        self.anchor = Instant::now();
    }

    pub fn toggle_auto(&mut self, rotate_secs: u64, n: usize) {
        self.base = self.index(rotate_secs, n);
        self.anchor = Instant::now();
        self.auto = !self.auto;
    }
}

/// Scenes available right now, in rotation order.
pub fn scenes(app: &App) -> Vec<Scene> {
    let cfg = &app.config;
    let mut v = vec![Scene::Horizon, Scene::Flip];
    if cfg.widgets.matrix {
        v.push(Scene::Rain);
    }
    v.push(Scene::Topo);
    v.push(Scene::Starfield);
    v.push(Scene::Life);
    v.push(Scene::Snow);
    if cfg.widgets.video {
        v.push(Scene::Orbit);
    }
    if media::current_player().is_some() {
        v.push(Scene::Studio);
    }
    if cfg.widgets.pinned_media {
        v.push(Scene::Gallery);
    }
    v
}

pub fn render(f: &mut Frame, area: Rect, app: &App) {
    if area.width < MIN.0 || area.height < MIN.1 {
        too_small(f, area, &app.theme, MIN);
        return;
    }
    let theme = &app.theme;
    let list = scenes(app);
    let idx = app
        .ambient
        .index(app.config.ui.ambient_rotate_secs, list.len());
    let scene = list[idx].clone();

    // Hints overlay the bottom row only while visible, so the stage keeps
    // the full height and nothing jumps when they come and go.
    let stage = area;
    let footer = Rect::new(area.x, area.bottom() - 1, area.width, 1);
    let t = drift_step();
    match scene {
        Scene::Horizon => horizon(f, drift(stage, 3, 0, t), app, theme, t),
        Scene::Flip => flip(f, drift(stage, 3, 1, t), app, theme),
        Scene::Rain => rain(f, stage, app, theme, t),
        Scene::Topo => topo(f, stage, app, theme, t),
        Scene::Orbit => orbit(f, drift(stage, 3, 1, t), app, theme),
        Scene::Studio => studio(f, drift(stage, 3, 1, t), app, theme),
        Scene::Gallery => gallery(f, stage, app, theme, t),
        Scene::Starfield => starfield(f, stage, app, theme),
        Scene::Life => life(f, stage, app, theme),
        Scene::Snow => snow(f, stage, app, theme),
        Scene::Custom(ref id) => {
            let mut matched = false;
            for ext in &app.ext_manager.extensions {
                for mut comp in ext.components() {
                    if comp.id().eq_ignore_ascii_case(id)
                        || ext.metadata().id.eq_ignore_ascii_case(id)
                    {
                        comp.render(f, stage, theme);
                        matched = true;
                        break;
                    }
                }
                if matched {
                    break;
                }
            }
        }

        Scene::Creative => {
            let [left, right] = ratatui::layout::Layout::horizontal([
                ratatui::layout::Constraint::Percentage(65),
                ratatui::layout::Constraint::Percentage(35),
            ])
            .areas(stage);
            let [video_area, weather_area] = ratatui::layout::Layout::vertical([
                ratatui::layout::Constraint::Fill(1),
                ratatui::layout::Constraint::Length(3),
            ])
            .areas(left);
            let [clock_area, viz_area] = ratatui::layout::Layout::vertical([
                ratatui::layout::Constraint::Length(12),
                ratatui::layout::Constraint::Fill(1),
            ])
            .areas(right);

            // Render native components
            crate::widgets::weather::render(f, weather_area, theme);
            big_clock(f, clock_area, app, theme);
            if app.config.widgets.music_viz {
                crate::widgets::music_viz::render(f, viz_area, theme, app.frame);
            }

            // Lookup and render the video_widget
            for ext in &app.ext_manager.extensions {
                for mut comp in ext.components() {
                    if comp.id() == "video_widget" {
                        comp.render(f, video_area, theme);
                    }
                }
            }
        }
    }
    if app.panel_states.pinned_media_input_active {
        f.render_widget(Clear, footer);
        f.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled("image path: ", Style::default().fg(theme.dim)),
                Span::styled(
                    format!("{}_", app.panel_states.pinned_media_input),
                    Style::default().fg(theme.accent),
                ),
            ]))
            .alignment(Alignment::Center),
            footer,
        );
    } else if app.last_input.elapsed() < HINT_SECS {
        f.render_widget(Clear, footer);
        render_footer(f, footer, theme, &list, idx, app.ambient.auto);
    }
}

/// Scene hints disappear this long after the last key press.
const HINT_SECS: std::time::Duration = std::time::Duration::from_secs(10);
/// Content shifts one step this often, slowly enough to go unnoticed.
const DRIFT_SECS: u64 = 90;

fn drift_step() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs() / DRIFT_SECS)
}

/// Triangle wave over 0..=amp.
fn tri(t: u64, amp: u16) -> u16 {
    if amp == 0 {
        return 0;
    }
    let period = 2 * amp as u64;
    let k = t % period;
    (if k <= amp as u64 { k } else { period - k }) as u16
}

/// Burn-in protection: `area` shrunk by `ax`/`ay` on each side and nudged
/// around the freed margin, so static glyphs (the clock above all) never
/// sit on the same cells for hours on OLED/plasma screens. The vertical
/// axis moves slower so the path wanders instead of tracing one diagonal.
fn drift(area: Rect, ax: u16, ay: u16, t: u64) -> Rect {
    let ax = ax.min(area.width / 8);
    let ay = ay.min(area.height / 8);
    Rect::new(
        area.x + tri(t, 2 * ax),
        area.y + tri(t / 5, 2 * ay),
        area.width - 2 * ax,
        area.height - 2 * ay,
    )
}

fn big_clock(f: &mut Frame, area: Rect, app: &App, theme: &Theme) {
    let note = upnext::next_note();
    clock::render_with_note(
        f,
        area,
        theme,
        app.config.ui.clock_24h,
        &app.config.ui.clock_font,
        &app.config.ui.clock_style,
        &[],
        note.as_deref(),
    );
}

/// Centered single line of weather under the clock.
fn weather_line(f: &mut Frame, area: Rect, theme: &Theme) {
    if !crate::monitors::weather::snapshot().ready || area.height == 0 {
        return;
    }
    let w = area.width.min(48);
    weather::render(
        f,
        Rect::new(area.x + (area.width - w) / 2, area.y, w, 1),
        theme,
    );
}

/// The audio horizon stays pinned to the bottom edge; only the clock
/// block above it drifts vertically.
fn horizon(f: &mut Frame, area: Rect, app: &App, theme: &Theme, t: u64) {
    let viz_h = (area.height / 4).clamp(4, 10);
    let [sky, viz] = Layout::vertical([Constraint::Min(0), Constraint::Length(viz_h)]).areas(area);
    let sky = drift(sky, 0, 1, t);
    let [_, clock_area, wx, _] = Layout::vertical([
        Constraint::Fill(1),
        Constraint::Length((sky.height.saturating_sub(4)).min(18)),
        Constraint::Length(1),
        Constraint::Fill(1),
    ])
    .areas(sky);
    // Side margins keep the glyphs from running edge to edge.
    let margin = clock_area.width / 8;
    big_clock(
        f,
        Rect::new(
            clock_area.x + margin,
            clock_area.y,
            clock_area.width - 2 * margin,
            clock_area.height,
        ),
        app,
        theme,
    );
    weather_line(f, wx, theme);
    if app.config.widgets.music_viz {
        music_viz::render(f, viz, theme, app.frame);
    }
}

fn flip(f: &mut Frame, area: Rect, app: &App, theme: &Theme) {
    let transparent = app
        .config
        .ui
        .transparent
        .unwrap_or_else(|| !theme.is_light());
    let note = upnext::next_note();
    crate::widgets::flip_clock::render(
        f,
        area,
        theme,
        app.config.ui.clock_24h,
        transparent,
        note.as_deref(),
    );
}

/// Full-bleed map with a small legend that wanders along the bottom-left
/// corner (the map itself is always moving, so only the legend needs it).
fn topo(f: &mut Frame, area: Rect, app: &App, theme: &Theme, t: u64) {
    crate::widgets::topo::render(
        f,
        area,
        theme,
        app.summary.cpu_pct,
        app.config.ui.motion_enabled,
        app.night == Some(true),
    );
    let time = if app.config.ui.clock_24h {
        chrono::Local::now().format("%H:%M").to_string()
    } else {
        chrono::Local::now().format("%-I:%M %P").to_string()
    };
    let mut legend = format!(" {}  ·  cpu {:.0}% ", time, app.summary.cpu_pct);
    if crate::widgets::topo::reacting_to_music() {
        legend.push_str("·  ♪ ");
    }
    let w = (legend.chars().count() as u16).min(area.width);
    let (x, y) = (
        area.x + 1 + tri(t, 6).min(area.width.saturating_sub(w + 1)),
        area.bottom().saturating_sub(3 + tri(t / 5, 2)),
    );
    f.render_widget(Clear, Rect::new(x, y, w, 1));
    f.render_widget(
        Paragraph::new(Span::styled(legend, Style::default().fg(theme.dim))),
        Rect::new(x, y, w, 1),
    );
}

fn rain(f: &mut Frame, area: Rect, app: &App, theme: &Theme, t: u64) {
    matrix::render(f, area, theme);
    let w = (area.width * 3 / 5).clamp(40, 90).min(area.width);
    let h = 12.min(area.height);
    // The card floats around the middle half of the free space.
    let (fx, fy) = ((area.width - w) / 2, (area.height - h) / 2);
    let card = Rect::new(
        area.x + fx / 2 + tri(t, fx),
        area.y + fy / 2 + tri(t / 5, fy),
        w,
        h,
    );
    f.render_widget(Clear, card);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme.surface))
        .style(Style::default().bg(theme.bg));
    let inner = block.inner(card);
    f.render_widget(block, card);
    big_clock(f, inner, app, theme);
}

fn orbit(f: &mut Frame, area: Rect, app: &App, theme: &Theme) {
    let [left, right] =
        Layout::horizontal([Constraint::Percentage(55), Constraint::Percentage(45)]).areas(area);
    video::render_with_motion(
        f,
        left,
        theme,
        app.frame,
        app.config.ui.motion_enabled,
        app.config.ui.motion_speed,
        &app.config.ui.motion_mode,
    );
    let [_, clk, wx, _, track, _] = Layout::vertical([
        Constraint::Fill(1),
        Constraint::Length(10),
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(3),
        Constraint::Fill(1),
    ])
    .areas(right);
    big_clock(f, clk, app, theme);
    weather_line(f, wx, theme);
    if media::current_player().is_some() {
        media::render(f, track, theme);
    }
}

fn studio(f: &mut Frame, area: Rect, app: &App, theme: &Theme) {
    let [_, info, _, viz] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length((area.height / 3).clamp(4, 10)),
        Constraint::Length(1),
        Constraint::Min(4),
    ])
    .areas(area);
    let pad = area.width / 10;
    media::render(
        f,
        Rect::new(info.x + pad, info.y, info.width - 2 * pad, info.height),
        theme,
    );
    music_viz::render(f, viz, theme, app.frame);
}

fn make_mini_bar(pct: f32, total: usize) -> String {
    let filled = ((pct / 100.0) * total as f32)
        .round()
        .clamp(0.0, total as f32) as usize;
    let mut s = String::with_capacity(total + 2);
    s.push('[');
    for i in 0..total {
        if i < filled {
            s.push('■');
        } else {
            s.push('·');
        }
    }
    s.push(']');
    s
}

fn gallery(f: &mut Frame, area: Rect, app: &App, theme: &Theme, _t: u64) {
    use chrono::Timelike;
    crate::anim::request(6);

    let gallery_area = Rect::new(area.x, area.y, area.width, area.height.saturating_sub(1));
    if gallery_area.width < 50 || gallery_area.height < 12 {
        let [img_box, time_box] =
            Layout::vertical([Constraint::Fill(1), Constraint::Length(1)]).areas(gallery_area);
        pinned_media::render(
            f,
            img_box,
            theme,
            &app.config.ui.pinned_media_path,
            app.frame,
        );
        let time = if app.config.ui.clock_24h {
            chrono::Local::now().format(" %H:%M ").to_string()
        } else {
            chrono::Local::now().format(" %-I:%M %P ").to_string()
        };
        f.render_widget(
            Paragraph::new(Span::styled(
                time,
                Style::default().fg(theme.text).add_modifier(Modifier::BOLD),
            ))
            .alignment(Alignment::Center),
            time_box,
        );
        return;
    }

    // Header bar (2 lines)
    let [header_area, stage_area] =
        Layout::vertical([Constraint::Length(2), Constraint::Fill(1)]).areas(gallery_area);

    let [h_top, h_div] =
        Layout::vertical([Constraint::Length(1), Constraint::Length(1)]).areas(header_area);

    let time_str = if app.config.ui.clock_24h {
        chrono::Local::now().format("%H:%M:%S").to_string()
    } else {
        chrono::Local::now().format("%-I:%M:%S %P").to_string()
    };
    let date_str = chrono::Local::now().format("%A, %d %b %Y").to_string();

    let header_line = Line::from(vec![
        Span::styled(
            " ◈ VANTA ATELIER ",
            Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("· ", Style::default().fg(theme.dim)),
        Span::styled("EXHIBITION LOUNGE ", Style::default().fg(theme.text)),
        Span::styled("[GALLERY PERSPECTIVE]", Style::default().fg(theme.dim)),
    ]);
    f.render_widget(Paragraph::new(header_line), h_top);

    let right_info = Line::from(vec![
        Span::styled(date_str, Style::default().fg(theme.dim)),
        Span::styled(" · ", Style::default().fg(theme.dim)),
        Span::styled(
            time_str,
            Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("  ● LIVE ARCHIVE ", Style::default().fg(theme.green)),
    ]);
    f.render_widget(
        Paragraph::new(right_info).alignment(Alignment::Right),
        h_top,
    );

    let div_str: String = "─".repeat(h_div.width as usize);
    f.render_widget(
        Paragraph::new(Span::styled(div_str, Style::default().fg(theme.dim))),
        h_div,
    );

    // Main Stage: Left is Art Card, Right is Curated Exhibition Deck
    let (art_pct, deck_pct) = if stage_area.width >= 110 {
        (56, 44)
    } else {
        (52, 48)
    };

    let [left_col, right_col] = Layout::horizontal([
        Constraint::Percentage(art_pct),
        Constraint::Percentage(deck_pct),
    ])
    .areas(stage_area);

    let media_info = pinned_media::current_media_info();
    let is_fallback = media_info.as_ref().is_none_or(|m| m.is_fallback);
    let raw_name = media_info
        .as_ref()
        .map(|m| m.file_name.as_str())
        .unwrap_or("porsche_911_dusk.jpg");
    let file_lower = raw_name.to_lowercase();

    let (frame_title, curator_title, curator_sub, curator_quote, curator_medium) = if is_fallback {
        (
            "CLASSIC 911 // DUSK NOCTURNE".to_string(),
            "Porsche 911 Carrera · Stuttgart Nocturne".to_string(),
            "Silver metallic finish under wet neon reflection".to_string(),
            "“The street is a mirror of city lights, rain tracing the contours of timeless metal.”"
                .to_string(),
            "35mm Analog Chrome · ƒ/1.4 · 1/250s · ISO 100".to_string(),
        )
    } else if file_lower.contains("tartakow") {
        (
            "OUR LADY OF TARTAKOW // MARIAN ICON".to_string(),
            "Our Lady of Tartakow · Miraculous Grace".to_string(),
            "Sacred Marian Iconography · Polish-Ukrainian Tradition".to_string(),
            "“Under your protection we take refuge, Holy Mother of God.”".to_string(),
            "Gold Leaf & Tempera on Wood · 17th Century".to_string(),
        )
    } else if file_lower.contains("sorrow") {
        (
            "MATER DOLOROSA // MOTHER OF SORROWS".to_string(),
            "Mater Dolorosa · Sacred Reflection".to_string(),
            "Classical Devotional Sacred Iconography".to_string(),
            "“And a sword will pierce through your own soul also.”".to_string(),
            "Oil on Poplar Panel · Passion Tradition".to_string(),
        )
    } else if file_lower.contains("prayer") {
        (
            "VIRGIN IN PRAYER // SASSOFERRATO".to_string(),
            "The Virgin in Prayer · Giovanni Battista Salvi".to_string(),
            "High Baroque Masterwork · Rome c. 1640–1650".to_string(),
            "“My soul magnifies the Lord, and my spirit rejoices in God my Savior.”".to_string(),
            "Oil on Canvas · Sacred Marian Collection".to_string(),
        )
    } else if file_lower.contains("virgin")
        || file_lower.contains("mary")
        || file_lower.contains("madonna")
        || file_lower.contains("rosary")
        || file_lower.contains("sacred")
        || file_lower.contains("christ")
    {
        (
            "SACRED ICONOGRAPHY // MARIAN DEVOTION".to_string(),
            "The Virgin Mary · Reverence & Grace".to_string(),
            "Classical Devotional Sacred Art & Iconography".to_string(),
            "“Hail Mary, full of grace, the Lord is with thee.”".to_string(),
            "Tempera & Oil on Wood · Sacred Heritage".to_string(),
        )
    } else if file_lower.contains("porsche")
        || file_lower.contains("car")
        || file_lower.contains("ferrari")
        || file_lower.contains("auto")
    {
        (
            "AUTOMOTIVE DESIGN // PRECISION FORM".to_string(),
            "High-Performance Engineering & Form".to_string(),
            "Industrial Sculpting & Aerodynamic Architecture".to_string(),
            "“Design is not just what it looks like and feels like. Design is how it works.”"
                .to_string(),
            "High Resolution Photographic Study · Studio Lighting".to_string(),
        )
    } else {
        let stem = std::path::Path::new(raw_name)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or(raw_name);
        let trimmed = stem.trim_start_matches(|c: char| c.is_ascii_digit() || c == '_' || c == '-');
        let words: Vec<String> = trimmed
            .split(['_', '-', ' '])
            .filter(|w| !w.is_empty())
            .map(|w| {
                let mut c = w.chars();
                match c.next() {
                    None => String::new(),
                    Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
                }
            })
            .collect();
        let title_name = if words.is_empty() {
            "Masterwork Archive".to_string()
        } else {
            words.join(" ")
        };
        (
            format!("CURATED ARCHIVE // {}", title_name.to_uppercase()),
            format!("{} · Personal Collection", title_name),
            "Curated Masterwork Archive · Ambient Projection".to_string(),
            "“Art enables us to find ourselves and lose ourselves at the same time.”".to_string(),
            "Digital Master Archive · High-Fidelity Display".to_string(),
        )
    };

    let (frame_header_badge, frame_bottom) = if let Some(ref info) = media_info {
        let badge = if info.is_dir && info.total_images > 1 {
            format!(
                " [EXHIBIT {} / {}]",
                info.current_idx + 1,
                info.total_images
            )
        } else {
            String::new()
        };
        let bottom = if info.orig_width > 0 && info.orig_height > 0 {
            format!(
                " {}×{} px · {} KB · sRGB 24-bit ",
                info.orig_width, info.orig_height, info.file_size_kb
            )
        } else {
            " High-Fidelity Projection · sRGB 24-bit ".to_string()
        };
        (badge, bottom)
    } else {
        (
            String::new(),
            " High-Fidelity Projection · sRGB 24-bit ".to_string(),
        )
    };

    // Render Left Artwork Frame
    let art_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme.accent))
        .title(Line::from(vec![
            Span::styled(" ◈ ", Style::default().fg(theme.accent)),
            Span::styled(
                frame_title,
                Style::default().fg(theme.text).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                frame_header_badge,
                Style::default()
                    .fg(theme.secondary)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" ", Style::default()),
        ]))
        .title_bottom(Line::from(vec![Span::styled(
            frame_bottom,
            Style::default().fg(theme.dim),
        )]));
    let inner_art = art_block.inner(left_col);
    f.render_widget(art_block, left_col);

    pinned_media::render(
        f,
        inner_art,
        theme,
        &app.config.ui.pinned_media_path,
        app.frame,
    );

    // Render Right Column (3 stacked cards)
    let [card1_rect, card2_rect, card3_rect] = Layout::vertical([
        Constraint::Fill(1),
        Constraint::Fill(1),
        Constraint::Fill(1),
    ])
    .areas(right_col);

    // Card 1: Curator's Dossier
    let c1_badge = if let Some(ref info) = media_info {
        if info.is_dir && info.total_images > 1 {
            format!(
                " [EXHIBIT {} / {}] ",
                info.current_idx + 1,
                info.total_images
            )
        } else {
            " [MASTER ARCHIVE] ".to_string()
        }
    } else {
        " [MASTER ARCHIVE] ".to_string()
    };

    let c1_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme.surface))
        .title(Line::from(vec![
            Span::styled(" ◈ ", Style::default().fg(theme.secondary)),
            Span::styled(
                "CURATOR'S DOSSIER",
                Style::default().fg(theme.text).add_modifier(Modifier::BOLD),
            ),
            Span::styled(" ", Style::default()),
        ]))
        .title_bottom(Line::from(vec![Span::styled(
            c1_badge,
            Style::default().fg(theme.dim),
        )]));
    let c1_inner = c1_block.inner(card1_rect);
    f.render_widget(c1_block, card1_rect);

    let mut c1_lines = Vec::new();
    c1_lines.push(Line::from(vec![Span::styled(
        curator_title,
        Style::default()
            .fg(theme.accent)
            .add_modifier(Modifier::BOLD),
    )]));
    c1_lines.push(Line::from(vec![Span::styled(
        curator_sub,
        Style::default().fg(theme.text),
    )]));
    if c1_inner.height >= 7 {
        c1_lines.push(Line::from(vec![Span::styled(
            curator_quote,
            Style::default()
                .fg(theme.dim)
                .add_modifier(Modifier::ITALIC),
        )]));
    }

    if let Some(ref info) = media_info {
        if c1_inner.height >= 5 {
            let dim_str = if info.orig_width > 0 && info.orig_height > 0 {
                format!("{}×{} px", info.orig_width, info.orig_height)
            } else {
                "Vector/Scan".to_string()
            };
            c1_lines.push(Line::from(vec![
                Span::styled("CANVAS: ", Style::default().fg(theme.dim)),
                Span::styled(dim_str, Style::default().fg(theme.text)),
                Span::styled(" · ", Style::default().fg(theme.dim)),
                Span::styled(
                    format!("{} KB", info.file_size_kb),
                    Style::default().fg(theme.text),
                ),
                Span::styled(" · ", Style::default().fg(theme.dim)),
                Span::styled("sRGB 24b", Style::default().fg(theme.dim)),
            ]));
        }
        if c1_inner.height >= 8 {
            c1_lines.push(Line::from(vec![
                Span::styled("MEDIUM: ", Style::default().fg(theme.dim)),
                Span::styled(curator_medium, Style::default().fg(theme.text)),
            ]));
        }

        // Chromatic Palette
        if !info.palette.is_empty() && c1_inner.height >= 6 {
            let mut pal_spans = vec![Span::styled("PALETTE: ", Style::default().fg(theme.dim))];
            for c in &info.palette {
                pal_spans.push(Span::styled("■■ ", Style::default().fg(*c)));
            }
            c1_lines.push(Line::from(pal_spans));

            if c1_inner.height >= 10 {
                let mut hex_spans = vec![Span::styled("        ", Style::default())];
                for c in &info.palette {
                    if let Color::Rgb(r, g, b) = c {
                        hex_spans.push(Span::styled(
                            format!("#{:02X}{:02X}{:02X} ", r, g, b),
                            Style::default().fg(*c).add_modifier(Modifier::DIM),
                        ));
                    }
                }
                c1_lines.push(Line::from(hex_spans));
            }
        }
    }
    c1_lines.truncate(c1_inner.height as usize);
    f.render_widget(Paragraph::new(c1_lines).wrap(Wrap { trim: true }), c1_inner);

    // Card 2: Atmosphere & Acoustics
    let c2_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme.surface))
        .title(Line::from(vec![
            Span::styled(" ♫ ", Style::default().fg(theme.green)),
            Span::styled(
                "ATMOSPHERE & ACOUSTICS",
                Style::default().fg(theme.text).add_modifier(Modifier::BOLD),
            ),
            Span::styled(" ", Style::default()),
        ]));
    let c2_inner = c2_block.inner(card2_rect);
    f.render_widget(c2_block, card2_rect);

    let mut c2_lines = Vec::new();
    let current_track = media::current_track();

    if let Some(track) = current_track.filter(|t| !t.title.trim().is_empty()) {
        let is_playing = track.status == media::Status::Playing;
        let icon = if is_playing {
            "● PLAYING"
        } else {
            "❚❚ PAUSED"
        };
        c2_lines.push(Line::from(vec![
            Span::styled(
                "♫ TRACK: ",
                Style::default()
                    .fg(theme.green)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                track.title,
                Style::default().fg(theme.text).add_modifier(Modifier::BOLD),
            ),
        ]));
        c2_lines.push(Line::from(vec![
            Span::styled("  ARTIST: ", Style::default().fg(theme.dim)),
            Span::styled(track.artist, Style::default().fg(theme.accent)),
            Span::styled(" · ", Style::default().fg(theme.dim)),
            Span::styled(track.album, Style::default().fg(theme.dim)),
        ]));

        if c2_inner.height >= 5 {
            let pos_s = (track.position_us / 1_000_000).max(0);
            let len_s = (track.length_us / 1_000_000).max(0);
            let pct = if len_s > 0 {
                (pos_s as f32 / len_s as f32 * 100.0).clamp(0.0, 100.0)
            } else {
                0.0
            };
            let bar_len = (c2_inner.width.saturating_sub(22)).clamp(6, 24) as usize;
            let filled = ((pct / 100.0) * bar_len as f32).round() as usize;
            let mut bar_str = String::with_capacity(bar_len + 2);
            bar_str.push('[');
            for i in 0..bar_len {
                if i == filled {
                    bar_str.push('●');
                } else if i < filled {
                    bar_str.push('━');
                } else {
                    bar_str.push('─');
                }
            }
            bar_str.push(']');

            c2_lines.push(Line::from(vec![
                Span::styled("  POS: ", Style::default().fg(theme.dim)),
                Span::styled(bar_str, Style::default().fg(theme.green)),
                Span::styled(
                    format!(
                        " {:02}:{:02}/{:02}:{:02} ",
                        pos_s / 60,
                        pos_s % 60,
                        len_s / 60,
                        len_s % 60
                    ),
                    Style::default().fg(theme.text),
                ),
                Span::styled(
                    icon,
                    Style::default().fg(if is_playing {
                        theme.green
                    } else {
                        theme.yellow
                    }),
                ),
            ]));
        }
    } else {
        let hour = chrono::Local::now().hour();
        let (soundscape_title, soundscape_tuning, soundscape_room) = match hour {
            0..=5 => (
                "Night Nocturne · Quiet Solitude",
                "432 Hz Solfeggio Harmonic · Alpha Waves",
                "Cathedral Reverb · RT60 2.4s · Deep Silence",
            ),
            6..=11 => (
                "Dawn Awakening · Gentle Resonance",
                "528 Hz Transformation Frequency · 12 dB Depth",
                "Morning Chamber · Natural Acoustic Diffusion",
            ),
            12..=17 => (
                "Solar Meridian · Focused Clarity",
                "639 Hz Harmonic Balance · Low Ambient Hum",
                "Acoustic Studio · Controlled Reflection",
            ),
            18..=21 => (
                "Golden Hour · Melodic Contemplation",
                "432 Hz Warm Resonance · Analog Tube Drift",
                "Concert Hall Ambient · Warm Diffusion",
            ),
            _ => (
                "Dusk Reverie · Soft Shadows",
                "396 Hz Grounding Wave · Binaural Theta",
                "Twilight Lounge · Muted Reflections",
            ),
        };

        c2_lines.push(Line::from(vec![
            Span::styled(
                "♫ SOUNDSCAPE: ",
                Style::default()
                    .fg(theme.green)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                soundscape_title,
                Style::default().fg(theme.text).add_modifier(Modifier::BOLD),
            ),
        ]));
        c2_lines.push(Line::from(vec![
            Span::styled("  TUNING: ", Style::default().fg(theme.dim)),
            Span::styled(soundscape_tuning, Style::default().fg(theme.accent)),
        ]));
        if c2_inner.height >= 6 {
            c2_lines.push(Line::from(vec![
                Span::styled("  SPACE:  ", Style::default().fg(theme.dim)),
                Span::styled(soundscape_room, Style::default().fg(theme.text)),
            ]));
        }
    }

    let wx_snap = crate::monitors::weather::snapshot();
    if c2_inner.height >= 4 {
        if wx_snap.ready {
            let cond_desc = crate::monitors::weather::describe(wx_snap.condition_code);
            c2_lines.push(Line::from(vec![
                Span::styled("  CLIMATE: ", Style::default().fg(theme.dim)),
                Span::styled(
                    format!("{:.0}°C · {} ", wx_snap.temp_c, cond_desc),
                    Style::default().fg(theme.accent),
                ),
                Span::styled(
                    format!(
                        "({:.0}°C feels, {}% RH, {:.0} km/h)",
                        wx_snap.feels_c, wx_snap.humidity, wx_snap.wind_kmh
                    ),
                    Style::default().fg(theme.dim),
                ),
            ]));
        } else {
            c2_lines.push(Line::from(vec![
                Span::styled("  CLIMATE: ", Style::default().fg(theme.dim)),
                Span::styled(
                    "Ambient Chamber · 21.5°C · Passive Airflow",
                    Style::default().fg(theme.text),
                ),
            ]));
        }
    }

    if c2_inner.height >= 5 {
        let wave_chars = [' ', '▂', '▃', '▄', '▅', '▆', '▇', '█'];
        let wave_len = (c2_inner.width.saturating_sub(14)).clamp(8, 32) as usize;
        let wave_lo: String = (0..wave_len)
            .map(|i| {
                let phase = ((app.frame as f64 * 0.12) + (i as f64 * 0.38)).sin();
                let idx = ((phase + 1.0) * 3.5).clamp(0.0, 7.0) as usize;
                wave_chars[idx]
            })
            .collect();
        c2_lines.push(Line::from(vec![
            Span::styled("  SUB-BASS ", Style::default().fg(theme.dim)),
            Span::styled(wave_lo, Style::default().fg(theme.accent)),
        ]));

        if c2_inner.height >= 6 {
            let wave_hi: String = (0..wave_len)
                .map(|i| {
                    let phase = ((app.frame as f64 * 0.20) + (i as f64 * 0.55) + 1.4).cos();
                    let idx = ((phase + 1.0) * 3.5).clamp(0.0, 7.0) as usize;
                    wave_chars[idx]
                })
                .collect();
            c2_lines.push(Line::from(vec![
                Span::styled("  AIR-SHIM ", Style::default().fg(theme.dim)),
                Span::styled(wave_hi, Style::default().fg(theme.secondary)),
            ]));
        }
    }

    if c2_inner.height >= 7 {
        c2_lines.push(Line::from(vec![
            Span::styled("  ENGINE:  ", Style::default().fg(theme.dim)),
            Span::styled(
                "32-bit Float · 48 kHz · PipeWire Stream",
                Style::default().fg(theme.text),
            ),
        ]));
    }
    if c2_inner.height >= 8 {
        c2_lines.push(Line::from(vec![
            Span::styled("  SPATIAL: ", Style::default().fg(theme.dim)),
            Span::styled(
                "360° Binaural Stereo Field · Dynamic Diffusion",
                Style::default().fg(theme.dim),
            ),
        ]));
    }

    c2_lines.truncate(c2_inner.height as usize);
    f.render_widget(Paragraph::new(c2_lines), c2_inner);

    // Card 3: Zen Telemetry
    let c3_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme.surface))
        .title(Line::from(vec![
            Span::styled(" ⬡ ", Style::default().fg(theme.yellow)),
            Span::styled(
                "ZEN TELEMETRY",
                Style::default().fg(theme.text).add_modifier(Modifier::BOLD),
            ),
            Span::styled(" ", Style::default()),
        ]))
        .title_bottom(Line::from(vec![Span::styled(
            " [HOST VITALS] ",
            Style::default().fg(theme.dim),
        )]));
    let c3_inner = c3_block.inner(card3_rect);
    f.render_widget(c3_block, card3_rect);

    let cpu_val = app.summary.cpu_pct.clamp(0.0, 100.0);
    let mem_snap = crate::monitors::memory::snapshot();
    let mem_val = (mem_snap.pct() as f32).clamp(0.0, 100.0);
    let mem_used_gb = mem_snap.used as f64 / 1024.0 / 1024.0 / 1024.0;
    let mem_tot_gb = mem_snap.total as f64 / 1024.0 / 1024.0 / 1024.0;
    let disk_val = app.summary.disk_pct.unwrap_or(0.0) as f32;

    let cpu_bars = make_mini_bar(cpu_val, 8);
    let mem_bars = make_mini_bar(mem_val, 8);
    let disk_bars = make_mini_bar(disk_val, 8);

    let temp_str = if let Some(t) = app.summary.temp_c {
        format!("{:.0}°C", t)
    } else {
        "--°C".to_string()
    };

    let mut c3_lines = Vec::new();
    c3_lines.push(Line::from(vec![
        Span::styled("CPU  ", Style::default().fg(theme.dim)),
        Span::styled(cpu_bars, Style::default().fg(theme.accent)),
        Span::styled(
            format!(" {:>3.0}% · {} · Schedutil", cpu_val, temp_str),
            Style::default().fg(theme.text),
        ),
    ]));
    c3_lines.push(Line::from(vec![
        Span::styled("RAM  ", Style::default().fg(theme.dim)),
        Span::styled(mem_bars, Style::default().fg(theme.secondary)),
        Span::styled(
            format!(
                " {:>3.0}% · {:.1}/{:.1} GB (Swap: {:.0}%)",
                mem_val,
                mem_used_gb,
                mem_tot_gb,
                mem_snap.swap_pct()
            ),
            Style::default().fg(theme.text),
        ),
    ]));

    if c3_inner.height >= 4 {
        c3_lines.push(Line::from(vec![
            Span::styled("DISK ", Style::default().fg(theme.dim)),
            Span::styled(disk_bars, Style::default().fg(theme.yellow)),
            Span::styled(
                format!(" {:>3.0}% · NVMe Root · RW: Nominal", disk_val),
                Style::default().fg(theme.text),
            ),
        ]));
    }

    if c3_inner.height >= 5 {
        c3_lines.push(Line::from(vec![
            Span::styled("NET  ", Style::default().fg(theme.dim)),
            Span::styled(
                format!(
                    "↓ {:>5.1} KB/s   ↑ {:>5.1} KB/s · Eth0",
                    app.summary.rx_kbps, app.summary.tx_kbps
                ),
                Style::default().fg(theme.text),
            ),
        ]));
    }

    if c3_inner.height >= 5 {
        if let Some(gpu_val) = app.summary.gpu_pct {
            let gpu_bars = make_mini_bar(gpu_val as f32, 8);
            c3_lines.push(Line::from(vec![
                Span::styled("GPU  ", Style::default().fg(theme.dim)),
                Span::styled(gpu_bars, Style::default().fg(theme.green)),
                Span::styled(
                    format!(" {:>3.0}% · Dedicated 3D Core", gpu_val),
                    Style::default().fg(theme.text),
                ),
            ]));
        }
    }

    if c3_inner.height >= 6 {
        let proc_count = crate::monitors::processes::count();
        c3_lines.push(Line::from(vec![
            Span::styled("TASK ", Style::default().fg(theme.dim)),
            Span::styled(
                format!("{} Active System Threads · Sched: Normal", proc_count),
                Style::default().fg(theme.text),
            ),
        ]));
    }

    if c3_inner.height >= 7 {
        let bat_str = if let Some((pct, charging)) = app.summary.battery {
            format!("BAT: {}%{}", pct, if charging { " ⚡" } else { "" })
        } else {
            "PWR: AC ⚡".to_string()
        };
        c3_lines.push(Line::from(vec![
            Span::styled("SYS  ", Style::default().fg(theme.dim)),
            Span::styled(
                format!("UPTIME: {} · {}", app.summary.uptime, bat_str),
                Style::default().fg(theme.text),
            ),
        ]));
    }

    if c3_inner.height >= 8 {
        c3_lines.push(Line::from(vec![
            Span::styled("● ", Style::default().fg(theme.green)),
            Span::styled(
                "ALL SYSTEMS NOMINAL · ZEN EQUILIBRIUM",
                Style::default().fg(theme.dim).add_modifier(Modifier::BOLD),
            ),
        ]));
    }

    c3_lines.truncate(c3_inner.height as usize);
    f.render_widget(Paragraph::new(c3_lines), c3_inner);
}

/// Scene dots, e.g. "○ ● ○ ○  rain · ←/→ · r pause".
fn render_footer(f: &mut Frame, area: Rect, theme: &Theme, list: &[Scene], idx: usize, auto: bool) {
    let mut spans: Vec<Span> = list
        .iter()
        .enumerate()
        .map(|(i, _)| {
            if i == idx {
                Span::styled("● ", Style::default().fg(theme.accent))
            } else {
                Span::styled("○ ", Style::default().fg(theme.surface))
            }
        })
        .collect();
    spans.push(Span::styled(
        format!(
            " {} · ←/→ scenes · r {} · i image",
            list[idx].label(),
            if auto { "pause" } else { "rotate" }
        ),
        Style::default().fg(theme.dim),
    ));
    f.render_widget(
        Paragraph::new(Line::from(spans)).alignment(Alignment::Center),
        area,
    );
}

fn starfield(f: &mut Frame, area: Rect, _app: &App, theme: &Theme) {
    crate::widgets::starfield::render(f, area, theme);
}

fn life(f: &mut Frame, area: Rect, _app: &App, theme: &Theme) {
    crate::widgets::life::render(f, area, theme);
}

fn snow(f: &mut Frame, area: Rect, _app: &App, theme: &Theme) {
    crate::widgets::snow::render(f, area, theme);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stepping_wraps_and_pausing_freezes_the_scene() {
        let mut s = AmbientState::default();
        assert_eq!(s.index(300, 3), 0);
        s.step(-1, 300, 3);
        assert_eq!(s.index(300, 3), 2);
        s.step(1, 300, 3);
        assert_eq!(s.index(300, 3), 0);
        s.toggle_auto(300, 3);
        assert!(!s.auto);
        assert_eq!(s.index(1, 3), 0, "paused state ignores elapsed time");
    }

    #[test]
    fn drift_stays_inside_the_stage_and_visits_every_offset() {
        let stage = Rect::new(0, 1, 120, 30);
        let mut xs = std::collections::BTreeSet::new();
        let mut ys = std::collections::BTreeSet::new();
        for t in 0..200 {
            let r = drift(stage, 3, 1, t);
            assert!(r.x >= stage.x && r.right() <= stage.right());
            assert!(r.y >= stage.y && r.bottom() <= stage.bottom());
            assert_eq!((r.width, r.height), (114, 28));
            xs.insert(r.x);
            ys.insert(r.y);
        }
        assert_eq!(xs.len(), 7);
        assert_eq!(ys.len(), 3);
        // Tiny areas don't drift (and don't underflow).
        assert_eq!(drift(Rect::new(0, 0, 7, 7), 3, 1, 5), Rect::new(0, 0, 7, 7));
    }
}
