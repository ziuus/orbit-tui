use ratatui::{
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

use crate::app::App;
use crate::monitors::doctor::{state, DoctorStatus};
use crate::screens::{hit, Hit};

pub fn render(f: &mut Frame, area: Rect, app: &mut App) {
    let theme = &app.theme;

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.dim))
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
            .border_style(Style::default().fg(theme.dim))
            .title(" API Key Required ");

        let text = vec![
            Line::from(""),
            Line::from(vec![Span::styled(
                "Orbit Doctor",
                Style::default()
                    .fg(theme.accent)
                    .add_modifier(Modifier::BOLD),
            )])
            .alignment(Alignment::Center),
            Line::from(""),
            Line::from(vec![Span::styled(
                "requires an API key to diagnose system health.",
                Style::default().fg(theme.text),
            )])
            .alignment(Alignment::Center),
            Line::from(""),
            Line::from(vec![
                Span::styled("Export ", Style::default().fg(theme.text)),
                Span::styled(
                    "ORBIT_OPENAI_KEY",
                    Style::default()
                        .fg(theme.secondary)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    " in your shell to enable diagnostics.",
                    Style::default().fg(theme.text),
                ),
            ])
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
                    Line::from(vec![
                        Span::styled("Ready to analyze your system health, top processes, and resource utilization.", Style::default().fg(theme.text)),
                    ]),
                    Line::from(""),
                    Line::from(Span::styled("Press [Enter] to run Orbit AI Diagnostics", Style::default().fg(theme.green).add_modifier(Modifier::BOLD))),
                ];
                let p = Paragraph::new(text).alignment(Alignment::Center);
                hit(center, Hit::DoctorRun);
                f.render_widget(p, center);
            }
            DoctorStatus::Working(msg) => {
                let text = vec![
                    Line::from(""),
                    Line::from(vec![
                        Span::styled(
                            "⠼ ",
                            Style::default()
                                .fg(theme.yellow)
                                .add_modifier(Modifier::BOLD),
                        ),
                        Span::styled(msg, Style::default().fg(theme.secondary)),
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
                            .fg(theme.green)
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
                                .fg(theme.yellow)
                                .add_modifier(Modifier::BOLD),
                        )));
                    } else {
                        text.push(Line::from(Span::styled(
                            line,
                            Style::default().fg(theme.text),
                        )));
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
                        Style::default().fg(theme.red).add_modifier(Modifier::BOLD),
                    )),
                    Line::from(""),
                    Line::from(Span::styled(e, Style::default().fg(theme.text))),
                    Line::from(""),
                    Line::from(Span::styled(
                        "Press [Enter] or click to try again",
                        Style::default().fg(theme.dim),
                    )),
                ];
                let p = Paragraph::new(text)
                    .alignment(Alignment::Center)
                    .wrap(Wrap { trim: true });
                hit(center, Hit::DoctorRun);
                f.render_widget(p, center);
            }
        }
    }
}
