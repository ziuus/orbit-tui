use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::screens::doctor::{scan_running_agents, AGENT_STATE};
use crate::theme::Theme;

pub fn render(f: &mut Frame, area: Rect, theme: &Theme, _is_focused: bool) {
    if area.height < 2 || area.width < 10 {
        return;
    }

    let (detected, active_session, latest_output) = {
        let st = AGENT_STATE.lock().unwrap();
        let det = if !st.detected.is_empty() {
            st.detected.clone()
        } else {
            scan_running_agents()
        };
        let sess = st.active_session.clone();
        let last = st.session_screen.last().cloned();
        (det, sess, last)
    };

    let mut lines = Vec::new();
    let max_lines = area.height as usize;

    if detected.is_empty() && active_session.is_none() {
        lines.push(Line::from(vec![
            Span::styled("● ", Style::default().fg(theme.dim)),
            Span::styled(
                "Fleet Standby ",
                Style::default().fg(theme.text).add_modifier(Modifier::BOLD),
            ),
            Span::styled("· 0 active agents", Style::default().fg(theme.dim)),
        ]));
        if max_lines >= 3 {
            lines.push(Line::from(vec![Span::styled(
                "  Press [5] (Orbit AI) to spawn Claude / OpenCode",
                Style::default().fg(theme.secondary),
            )]));
        }
    } else {
        if let Some(ref sess) = active_session {
            lines.push(Line::from(vec![
                Span::styled(
                    "⚡ ",
                    Style::default()
                        .fg(theme.green)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    "Console: ",
                    Style::default()
                        .fg(theme.accent)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    sess.clone(),
                    Style::default().fg(theme.text).add_modifier(Modifier::BOLD),
                ),
                Span::styled(" (running)", Style::default().fg(theme.green)),
            ]));
            if max_lines >= 4 {
                if let Some(ref out) = latest_output {
                    let w = (area.width as usize).saturating_sub(6);
                    let trimmed = if out.len() > w {
                        &out[..w]
                    } else {
                        out.as_str()
                    };
                    lines.push(Line::from(vec![
                        Span::styled("  › ", Style::default().fg(theme.dim)),
                        Span::styled(trimmed, Style::default().fg(theme.dim)),
                    ]));
                }
            }
        }

        for a in &detected {
            if lines.len() >= max_lines {
                break;
            }
            let is_busy = a.cpu_pct > 1.0;
            let dot_color = if is_busy { theme.green } else { theme.yellow };
            lines.push(Line::from(vec![
                Span::styled("● ", Style::default().fg(dot_color)),
                Span::styled(
                    format!("{:<15} ", a.name),
                    Style::default().fg(theme.text).add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    format!("{:>4.1}% ", a.cpu_pct),
                    Style::default().fg(theme.accent),
                ),
                Span::styled(
                    format!("{}MB ", a.mem_mb),
                    Style::default().fg(theme.secondary),
                ),
                Span::styled(
                    if is_busy { "Active" } else { "Idle" },
                    Style::default().fg(if is_busy { theme.green } else { theme.dim }),
                ),
            ]));
        }
    }

    f.render_widget(Paragraph::new(lines), area);
}
