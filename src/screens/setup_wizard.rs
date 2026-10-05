use crossterm::event::KeyCode;
use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Modifier, Style};
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

fn is_cava_installed() -> bool {
    std::process::Command::new("cava")
        .arg("-v")
        .output()
        .is_ok()
}

pub fn render(f: &mut Frame, area: Rect, app: &App) {
    let theme = &app.theme;
    let popup_area = centered(area, 75, 20);
    f.render_widget(Clear, popup_area);

    let block = Block::default()
        .title(" Orbit First-Time Setup ")
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(
            Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD),
        )
        .style(Style::default().bg(theme.bg));

    let inner = block.inner(popup_area);
    f.render_widget(block, popup_area);

    let mut text = Vec::new();

    match app.setup_wizard_step {
        0 => {
            text.push(Line::from(""));
            text.push(Line::from(Span::styled(
                "Welcome to Orbit!",
                Style::default()
                    .fg(theme.green)
                    .add_modifier(Modifier::BOLD),
            )));
            text.push(Line::from(""));
            text.push(Line::from(
                "Orbit is a high-performance, aesthetic dashboard.",
            ));
            text.push(Line::from("Let's quickly get your system configured."));
            text.push(Line::from(""));
            text.push(Line::from(Span::styled(
                "Press [Enter] to continue",
                Style::default().fg(theme.dim),
            )));
        }
        1 => {
            text.push(Line::from(""));
            text.push(Line::from(Span::styled(
                "Step 1: Dependencies (Audio Visualizer)",
                Style::default()
                    .fg(theme.accent)
                    .add_modifier(Modifier::BOLD),
            )));
            text.push(Line::from(""));
            text.push(Line::from(
                "Orbit uses 'cava' for its live audio visualizer.",
            ));

            if is_cava_installed() {
                text.push(Line::from(""));
                text.push(Line::from(Span::styled(
                    "✓ Cava is already installed! You're good to go.",
                    Style::default().fg(theme.green),
                )));
            } else {
                text.push(Line::from(""));
                #[cfg(target_os = "macos")]
                {
                    text.push(Line::from(Span::styled(
                        "! Cava is missing.",
                        Style::default().fg(theme.yellow),
                    )));
                    text.push(Line::from(
                        "To enable live audio, exit and run:  brew install cava",
                    ));
                }
                #[cfg(target_os = "linux")]
                {
                    text.push(Line::from(Span::styled(
                        "! Cava is missing.",
                        Style::default().fg(theme.yellow),
                    )));
                    text.push(Line::from(
                        "To enable live audio, exit and run:  sudo apt install cava",
                    ));
                    text.push(Line::from("(Or pacman -S cava / dnf install cava)"));
                }
                #[cfg(target_os = "windows")]
                {
                    text.push(Line::from(Span::styled(
                        "! Cava is not natively supported on Windows.",
                        Style::default().fg(theme.dim),
                    )));
                    text.push(Line::from(
                        "Orbit will automatically use a simulated fallback animation.",
                    ));
                }
            }
            text.push(Line::from(""));
            text.push(Line::from(""));
            text.push(Line::from(Span::styled(
                "Press [Enter] to continue",
                Style::default().fg(theme.dim),
            )));
        }
        2 => {
            text.push(Line::from(""));
            text.push(Line::from(Span::styled(
                "Step 2: Graphics Engine",
                Style::default()
                    .fg(theme.accent)
                    .add_modifier(Modifier::BOLD),
            )));
            text.push(Line::from(""));
            text.push(Line::from(
                "Orbit renders meters, graphs, and images in Braille by default.",
            ));
            text.push(Line::from("Choose your image rendering mode:"));
            text.push(Line::from(""));
            text.push(Line::from(Span::styled(
                format!(
                    "  [1] Braille (Default - compatible with all terminals) {}",
                    if app.panel_states.pixel_images {
                        "(Selected)"
                    } else {
                        ""
                    }
                ),
                if app.panel_states.pixel_images {
                    Style::default().fg(theme.green)
                } else {
                    Style::default()
                },
            )));
            text.push(Line::from(Span::styled(
                format!(
                    "  [2] High-Res Engine (Requires Kitty/Sixel support) {}",
                    if !app.panel_states.pixel_images {
                        "(Selected)"
                    } else {
                        ""
                    }
                ),
                if !app.panel_states.pixel_images {
                    Style::default().fg(theme.green)
                } else {
                    Style::default()
                },
            )));
            text.push(Line::from(""));
            text.push(Line::from(Span::styled(
                "Press [Enter] to continue",
                Style::default().fg(theme.dim),
            )));
        }
        3 => {
            text.push(Line::from(""));
            text.push(Line::from(Span::styled(
                "Setup Complete!",
                Style::default()
                    .fg(theme.green)
                    .add_modifier(Modifier::BOLD),
            )));
            text.push(Line::from(""));
            text.push(Line::from("Your configuration has been saved."));
            text.push(Line::from(
                "You can change these later by pressing 'S' or running 'orbit config'.",
            ));
            text.push(Line::from(""));
            text.push(Line::from(Span::styled(
                "Press [Enter] to launch Orbit",
                Style::default().fg(theme.dim),
            )));
        }
        _ => {}
    }

    let p = Paragraph::new(text).alignment(Alignment::Center);
    f.render_widget(p, inner);
}

pub fn handle_key(app: &mut App, code: KeyCode) {
    match app.setup_wizard_step {
        0 => {
            if code == KeyCode::Enter {
                app.setup_wizard_step = 1;
            }
        }
        1 => {
            if code == KeyCode::Enter {
                app.setup_wizard_step = 2;
            }
        }
        2 => {
            if code == KeyCode::Char('1') {
                app.panel_states.pixel_images = true;
            } else if code == KeyCode::Char('2') {
                app.panel_states.pixel_images = false;
            } else if code == KeyCode::Enter {
                app.setup_wizard_step = 3;
            }
        }
        3 => {
            if code == KeyCode::Enter {
                app.config.save();
                app.show_setup_wizard = false;
            }
        }
        _ => {
            app.show_setup_wizard = false;
        }
    }
}
