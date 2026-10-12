use crossterm::event::KeyCode;
use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Clear, Paragraph};
use ratatui::Frame;

use crate::app::App;

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

pub fn render(f: &mut Frame, area: Rect, app: &App) {
    let theme = &app.theme;
    let popup_area = centered(area, 78, 22);
    f.render_widget(Clear, popup_area);

    let block = Block::default()
        .title(" ★ Notice: Vanta has moved to Orbit ★ ")
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(
            Style::default()
                .fg(Color::Rgb(130, 200, 255))
                .add_modifier(Modifier::BOLD),
        )
        .style(Style::default().bg(theme.bg));

    let inner = block.inner(popup_area);
    f.render_widget(block, popup_area);

    let mut text = Vec::new();
    text.push(Line::from(""));
    text.push(Line::from(vec![
        Span::styled(
            "Vanta is now officially Orbit!",
            Style::default()
                .fg(Color::Rgb(150, 255, 150))
                .add_modifier(Modifier::BOLD),
        ),
    ]));
    text.push(Line::from(""));
    text.push(Line::from(
        "We've moved the dashboard and ecosystem to Orbit — featuring faster",
    ));
    text.push(Line::from(
        "Ratatui rendering, Extism WASM sandboxed plugins, and community themes.",
    ));
    text.push(Line::from(""));
    text.push(Line::from(Span::styled(
        "HOW TO CLEAN UP OLD VANTA & STAY UPDATED:",
        Style::default()
            .fg(Color::Rgb(255, 215, 0))
            .add_modifier(Modifier::BOLD),
    )));
    text.push(Line::from(""));
    text.push(Line::from(vec![
        Span::styled("1. Remove old npm package:  ", Style::default().fg(theme.dim)),
        Span::styled("npm uninstall -g @ziuus/vanta", Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
    ]));
    text.push(Line::from(vec![
        Span::styled("2. Install Orbit globally:   ", Style::default().fg(theme.dim)),
        Span::styled("npm install -g @ziuus/orbit", Style::default().fg(Color::Rgb(150, 255, 150)).add_modifier(Modifier::BOLD)),
    ]));
    text.push(Line::from(vec![
        Span::styled("   Or build via Cargo:       ", Style::default().fg(theme.dim)),
        Span::styled("cargo install --git https://github.com/ziuus/orbit-tui", Style::default().fg(theme.dim)),
    ]));
    text.push(Line::from(""));
    text.push(Line::from(vec![
        Span::styled("3. Remove old vanta configs: ", Style::default().fg(theme.dim)),
        Span::styled("rm -rf ~/.config/vanta", Style::default().fg(Color::Rgb(255, 150, 150))),
    ]));
    text.push(Line::from(""));
    text.push(Line::from(vec![
        Span::styled("LINKS & BUTTONS: ", Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
        Span::styled("[G] Open GitHub   ", Style::default().fg(Color::Rgb(130, 200, 255)).add_modifier(Modifier::BOLD)),
        Span::styled("[O] Open npm package   ", Style::default().fg(Color::Rgb(150, 255, 150)).add_modifier(Modifier::BOLD)),
        Span::styled("[Esc/Enter] Dismiss", Style::default().fg(theme.dim)),
    ]));

    let p = Paragraph::new(text).alignment(Alignment::Left);
    f.render_widget(p, inner);
}

pub fn handle_key(app: &mut App, code: KeyCode) {
    match code {
        KeyCode::Char('g') | KeyCode::Char('G') => {
            let _ = open::that("https://github.com/ziuus/orbit-tui");
            app.toast("Opened https://github.com/ziuus/orbit-tui in browser");
        }
        KeyCode::Char('o') | KeyCode::Char('O') => {
            let _ = open::that("https://www.npmjs.com/package/@ziuus/orbit");
            app.toast("Opened npm package in browser");
        }
        KeyCode::Enter | KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char(' ') => {
            app.show_vanta_migration_popup = false;
        }
        _ => {}
    }
}
