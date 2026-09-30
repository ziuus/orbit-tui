//! The Esc menu: a big gradient logo over a dimmed page, with the handful
//! of things you reach for most. ↑↓ move, ←→ change inline values, enter
//! picks, esc closes. Mouse: click an item, click outside to close.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Clear, Paragraph};
use ratatui::Frame;

use crate::app::App;
use crate::screens::{hit, Hit};
use crate::theme::{blend, Theme};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Item {
    Resume,
    Theme,
    Style,
    Layout,
    Options,
    Help,
    Quit,
}

pub const ITEMS: [Item; 7] = [
    Item::Resume,
    Item::Theme,
    Item::Style,
    Item::Layout,
    Item::Options,
    Item::Help,
    Item::Quit,
];

impl Item {
    fn label(self) -> &'static str {
        match self {
            Item::Resume => "resume",
            Item::Theme => "theme",
            Item::Style => "style",
            Item::Layout => "layout",
            Item::Options => "options",
            Item::Help => "help",
            Item::Quit => "quit",
        }
    }
    /// Items whose value ←/→ cycles in place.
    fn cycles(self) -> bool {
        matches!(self, Item::Theme | Item::Style | Item::Layout)
    }
}

/// 5×5 pixel letters for the logo; each pixel is drawn two cells wide.
const LOGO: [[&str; 5]; 5] = [
    ["#...#", "#...#", "#...#", ".#.#.", "..#.."],
    [".###.", "#...#", "#####", "#...#", "#...#"],
    ["#...#", "##..#", "#.#.#", "#..##", "#...#"],
    ["#####", "..#..", "..#..", "..#..", "..#.."],
    [".###.", "#...#", "#####", "#...#", "#...#"],
];
const LOGO_W: u16 = 5 * 10 + 4 * 2;
const LOGO_H: u16 = 5;

/// Fade everything already drawn toward the background, so the menu reads
/// as a layer above the page instead of a box pasted on top.
fn dim_backdrop(buf: &mut Buffer, area: Rect, theme: &Theme) {
    for y in area.top()..area.bottom() {
        for x in area.left()..area.right() {
            if let Some(cell) = buf.cell_mut((x, y)) {
                let fg = cell.fg;
                if let Color::Rgb(..) = fg {
                    cell.set_fg(blend(fg, theme.bg, 0.72));
                }
                if let Color::Rgb(..) = cell.bg {
                    let bg = cell.bg;
                    cell.set_bg(blend(bg, theme.bg, 0.6));
                }
            }
        }
    }
}

fn draw_logo(buf: &mut Buffer, x0: u16, y0: u16, theme: &Theme) {
    for (li, letter) in LOGO.iter().enumerate() {
        for (py, row) in letter.iter().enumerate() {
            for (px, ch) in row.chars().enumerate() {
                if ch != '#' {
                    continue;
                }
                let x = x0 + li as u16 * 12 + px as u16 * 2;
                let y = y0 + py as u16;
                // Shadow one cell down-right, then the lit pixel on top.
                for (dx, dy, color) in [
                    (1u16, 1u16, theme.surface),
                    (0, 0, {
                        let t = (x - x0) as f32 / LOGO_W as f32;
                        let top = blend(theme.accent, theme.secondary, t);
                        blend(top, theme.bg, py as f32 * 0.07)
                    }),
                ] {
                    for w in 0..2 {
                        if let Some(cell) = buf.cell_mut((x + dx + w, y + dy)) {
                            cell.set_char('█').set_style(Style::default().fg(color));
                        }
                    }
                }
            }
        }
    }
}

fn value(app: &App, item: Item) -> Option<String> {
    Some(match item {
        Item::Theme => app.config.ui.theme.clone(),
        Item::Style => app.config.ui.style.label().to_string(),
        Item::Layout => app.layout_name(),
        _ => return None,
    })
}

pub fn render(f: &mut Frame, area: Rect, app: &App) {
    let theme = &app.theme;
    dim_backdrop(f.buffer_mut(), area, theme);

    let big = area.width >= LOGO_W + 6 && area.height >= LOGO_H + ITEMS.len() as u16 + 9;
    let w = if big { LOGO_W + 6 } else { 40.min(area.width) };
    let h = ITEMS.len() as u16 + if big { LOGO_H + 8 } else { 6 };
    let box_area = Rect::new(
        area.x + area.width.saturating_sub(w) / 2,
        area.y + area.height.saturating_sub(h) / 2,
        w.min(area.width),
        h.min(area.height),
    );
    hit(box_area, Hit::Overlay);
    f.render_widget(Clear, box_area);
    let base = Style::default().bg(theme.bg);
    f.render_widget(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(blend(theme.accent, theme.bg, 0.35)))
            .title_bottom(
                Line::from(Span::styled(
                    " ↑↓ move · ←→ change · enter select · esc close ",
                    Style::default().fg(theme.dim),
                ))
                .centered(),
            )
            .style(base),
        box_area,
    );

    let mut y = box_area.y + 1;
    if big {
        let x0 = box_area.x + (box_area.width - LOGO_W) / 2;
        draw_logo(f.buffer_mut(), x0, y + 1, theme);
        y += LOGO_H + 2;
        f.render_widget(
            Paragraph::new(Line::from(Span::styled(
                format!("v{}", env!("CARGO_PKG_VERSION")),
                Style::default().fg(theme.dim),
            )))
            .right_aligned(),
            Rect::new(x0, y, LOGO_W, 1),
        );
        y += 2;
    } else {
        f.render_widget(
            Paragraph::new(Line::from(Span::styled(
                "V A N T A",
                Style::default()
                    .fg(theme.accent)
                    .add_modifier(Modifier::BOLD),
            )))
            .centered(),
            Rect::new(box_area.x, y + 1, box_area.width, 1),
        );
        y += 3;
    }

    let row_w = 30u16.min(box_area.width.saturating_sub(4));
    let rx = box_area.x + (box_area.width - row_w) / 2;
    for (i, item) in ITEMS.iter().enumerate() {
        let sel = i == app.menu_row;
        let r = Rect::new(rx, y + i as u16, row_w, 1);
        hit(r, Hit::MenuRow(i));
        let (fg, bg) = if sel {
            (theme.bg, theme.accent)
        } else {
            (theme.text, theme.bg)
        };
        let style = Style::default().fg(fg).bg(bg);
        let label = if sel {
            item.label().to_uppercase()
        } else {
            item.label().to_string()
        };
        let mut spans = vec![Span::styled(
            format!(" {} {}", if sel { "▸" } else { " " }, label),
            style.add_modifier(if sel {
                Modifier::BOLD
            } else {
                Modifier::empty()
            }),
        )];
        let right = match value(app, *item) {
            Some(v) if sel => format!("‹ {} › ", v),
            Some(v) => format!("{} ", v),
            None => String::new(),
        };
        let used: usize = spans[0].content.chars().count() + right.chars().count();
        spans.push(Span::styled(
            " ".repeat((row_w as usize).saturating_sub(used)),
            style,
        ));
        spans.push(Span::styled(
            right,
            if sel {
                style
            } else {
                Style::default().fg(theme.dim).bg(bg)
            },
        ));
        f.render_widget(Paragraph::new(Line::from(spans)), r);
    }
}

pub fn handle_key(app: &mut App, key: crossterm::event::KeyCode) {
    use crossterm::event::KeyCode::*;
    let n = ITEMS.len();
    match key {
        Up | Char('k') => app.menu_row = (app.menu_row + n - 1) % n,
        Down | Char('j') => app.menu_row = (app.menu_row + 1) % n,
        Left | Char('h') => change(app, false),
        Right | Char('l') => change(app, true),
        Enter | Char(' ') => activate(app),
        Esc | Char('q') => app.show_menu = false,
        _ => {}
    }
}

fn change(app: &mut App, forward: bool) {
    match ITEMS[app.menu_row] {
        Item::Theme => {
            if forward {
                app.cycle_theme();
            } else {
                app.cycle_theme_back();
            }
        }
        Item::Style => app.cycle_style(forward),
        Item::Layout => app.cycle_layout(forward),
        _ => {}
    }
}

pub fn activate(app: &mut App) {
    let item = ITEMS[app.menu_row];
    if item.cycles() {
        return change(app, true);
    }
    app.show_menu = false;
    match item {
        Item::Options => app.show_settings = true,
        Item::Help => app.show_help = true,
        Item::Quit => app.running = false,
        _ => {}
    }
}
