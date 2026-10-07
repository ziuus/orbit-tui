use ratatui::{
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

use crate::app::App;
use crate::monitors::doctor::{state, DoctorStatus};

pub fn render(f: &mut Frame, area: Rect, app: &mut App) {
    let _theme = &app.theme;

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray))
        .title(" Orbit AI Diagnostics ")
        .title_alignment(Alignment::Center);

    let inner = block.inner(area);
    f.render_widget(block, area);

    // Layout:
    let [_, center_y, _] = Layout::vertical([
        Constraint::Percentage(20),
        Constraint::Min(10),
        Constraint::Percentage(20),
    ])
    .areas(inner);

    let [_, center, _] = Layout::horizontal([
        Constraint::Percentage(15),
        Constraint::Percentage(70),
        Constraint::Percentage(15),
    ])
    .areas(center_y);

    let has_key = std::env::var("ORBIT_OPENAI_KEY").is_ok();
    let current_state = state();

    if !has_key {
        let lock_box = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray))
            .title(" API Key Required ");

        let text = vec![
            Line::from(""),
            Line::from(vec![
                Span::styled(
                    "Orbit Doctor ",
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw("requires an API key to diagnose system health."),
            ])
            .alignment(Alignment::Center),
            Line::from(""),
            Line::from("Export ORBIT_OPENAI_KEY in your shell to enable diagnostics.")
                .alignment(Alignment::Center),
            Line::from(""),
        ];

        let p = Paragraph::new(text)
            .block(lock_box)
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: true });

        // Smaller box for the lock
        let [_, small_center_y, _] = Layout::vertical([
            Constraint::Length(4),
            Constraint::Length(8),
            Constraint::Min(0),
        ])
        .areas(center);

        let [_, small_center_x, _] = Layout::horizontal([
            Constraint::Percentage(20),
            Constraint::Percentage(60),
            Constraint::Percentage(20),
        ])
        .areas(small_center_y);

        f.render_widget(p, small_center_x);
    } else {
        match current_state {
            DoctorStatus::Idle => {
                let text = vec![
                    Line::from(""),
                    Line::from("Ready to analyze your system health, top processes, and resource utilization."),
                    Line::from(""),
                    Line::from(Span::styled("Press [Enter] to run Orbit AI Diagnostics", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD))),
                ];
                let p = Paragraph::new(text).alignment(Alignment::Center);
                f.render_widget(p, center);
            }
            DoctorStatus::Working(msg) => {
                let text = vec![
                    Line::from(""),
                    Line::from(vec![
                        Span::styled(
                            "⠼ ",
                            Style::default()
                                .fg(Color::Yellow)
                                .add_modifier(Modifier::BOLD),
                        ),
                        Span::styled(msg, Style::default().fg(Color::Cyan)),
                    ]),
                ];
                let p = Paragraph::new(text).alignment(Alignment::Center);
                f.render_widget(p, center);
            }
            DoctorStatus::Done(diagnosis) => {
                let mut text = vec![
                    Line::from(Span::styled(
                        "Diagnostic Report:",
                        Style::default()
                            .fg(Color::Green)
                            .add_modifier(Modifier::BOLD),
                    )),
                    Line::from(""),
                ];

                // Split by newline so it formats nicely
                for line in diagnosis.lines() {
                    if line.starts_with("Diagnosis:") || line.starts_with("Action:") {
                        text.push(Line::from(Span::styled(
                            line,
                            Style::default()
                                .fg(Color::Yellow)
                                .add_modifier(Modifier::BOLD),
                        )));
                    } else {
                        text.push(Line::from(line));
                    }
                }

                let p = Paragraph::new(text).wrap(Wrap { trim: true });
                f.render_widget(p, center);
            }
            DoctorStatus::Error(e) => {
                let text = vec![
                    Line::from(""),
                    Line::from(Span::styled(
                        "Analysis Failed",
                        Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
                    )),
                    Line::from(""),
                    Line::from(Span::styled(e, Style::default().fg(Color::White))),
                    Line::from(""),
                    Line::from(Span::styled(
                        "Press [Enter] to try again",
                        Style::default().fg(Color::DarkGray),
                    )),
                ];
                let p = Paragraph::new(text)
                    .alignment(Alignment::Center)
                    .wrap(Wrap { trim: true });
                f.render_widget(p, center);
            }
        }
    }
}
