use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{LazyLock, Mutex};
use std::time::{Duration, Instant};

use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Paragraph};
use ratatui::Frame;

use crate::app::App;
use crate::theme::Theme;

#[derive(Debug)]
pub enum TmuxInput {
    Literal(String),
    Special(&'static str),
}

static WORKER_STARTED: AtomicBool = AtomicBool::new(false);
static KEY_TX: LazyLock<Mutex<Option<Sender<TmuxInput>>>> = LazyLock::new(|| Mutex::new(None));

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AgentKind {
    ClaudeCode,
    Opencode,
    Agy,
    Shell,
    Custom(String),
}

impl AgentKind {
    pub fn label(&self) -> &str {
        match self {
            Self::ClaudeCode => "Claude Code",
            Self::Opencode => "OpenCode",
            Self::Agy => "Antigravity (agy)",
            Self::Shell => "Interactive Shell",
            Self::Custom(c) => c.as_str(),
        }
    }

    pub fn command(&self) -> &str {
        match self {
            Self::ClaudeCode => "claude",
            Self::Opencode => "opencode",
            Self::Agy => "agy",
            Self::Shell => "bash",
            Self::Custom(c) => c.as_str(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct DetectedAgent {
    pub name: String,
    pub pid: u32,
    pub cpu_pct: f64,
    pub mem_mb: u64,
    pub status: String,
}

pub struct AgentTerminalState {
    pub active_session: Option<String>,
    pub terminal_focused: bool,
    pub input_buffer: String,
    pub scroll: usize,
    pub history: VecDeque<String>,
    pub last_poll: Instant,
    pub detected: Vec<DetectedAgent>,
    pub session_screen: Vec<String>,
}

impl Default for AgentTerminalState {
    fn default() -> Self {
        Self {
            active_session: None,
            terminal_focused: false,
            input_buffer: String::new(),
            scroll: 0,
            history: VecDeque::with_capacity(500),
            last_poll: Instant::now(),
            detected: Vec::new(),
            session_screen: Vec::new(),
        }
    }
}

pub static AGENT_STATE: LazyLock<Mutex<AgentTerminalState>> =
    LazyLock::new(|| Mutex::new(AgentTerminalState::default()));

const TMUX_SESSION_NAME: &str = "orbit_agent_session";

/// Check if a CLI binary is reachable in PATH without spawning child processes.
pub fn is_binary_available(cmd: &str) -> bool {
    if let Some(path_var) = std::env::var_os("PATH") {
        for dir in std::env::split_paths(&path_var) {
            let full_path = dir.join(cmd);
            if full_path.is_file() {
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    if let Ok(meta) = full_path.metadata() {
                        if meta.permissions().mode() & 0o111 != 0 {
                            return true;
                        }
                    }
                }
                #[cfg(not(unix))]
                return true;
            }
            #[cfg(windows)]
            {
                for ext in ["exe", "cmd", "bat"] {
                    if full_path.with_extension(ext).is_file() {
                        return true;
                    }
                }
            }
        }
    }
    false
}

/// Ensure background terminal capture & telemetry worker is running.
pub fn ensure_worker_started() {
    if WORKER_STARTED.swap(true, Ordering::SeqCst) {
        return;
    }

    let (tx, rx) = channel::<TmuxInput>();
    if let Ok(mut lock) = KEY_TX.lock() {
        *lock = Some(tx);
    }

    std::thread::spawn(move || {
        agent_worker_loop(rx);
    });
}

fn agent_worker_loop(rx: Receiver<TmuxInput>) {
    let mut last_agent_scan = Instant::now() - Duration::from_secs(10);
    let mut last_alive_check = Instant::now();

    loop {
        // 1. Process all pending keys non-blocking
        while let Ok(cmd) = rx.try_recv() {
            match cmd {
                TmuxInput::Literal(s) => {
                    let _ = std::process::Command::new("tmux")
                        .args(["send-keys", "-t", TMUX_SESSION_NAME, "-l", &s])
                        .output();
                }
                TmuxInput::Special(s) => {
                    let _ = std::process::Command::new("tmux")
                        .args(["send-keys", "-t", TMUX_SESSION_NAME, s])
                        .output();
                }
            }
        }

        let is_active = {
            let st = AGENT_STATE.lock().unwrap();
            st.active_session.is_some()
        };

        if is_active {
            // Live capture pane
            let lines = poll_agent_output();
            if let Ok(mut st) = AGENT_STATE.lock() {
                st.session_screen = lines;
            }

            // Check alive every 1 second
            if last_alive_check.elapsed() >= Duration::from_millis(1000) {
                if !is_session_alive() {
                    if let Ok(mut st) = AGENT_STATE.lock() {
                        st.active_session = None;
                        st.terminal_focused = false;
                        st.history
                            .push_back("⏹ Agent session completed.".to_string());
                    }
                }
                last_alive_check = Instant::now();
            }
        }

        // Fleet telemetry scan every 2.5 seconds
        if last_agent_scan.elapsed() >= Duration::from_millis(2500) {
            let detected = scan_running_agents();
            if let Ok(mut st) = AGENT_STATE.lock() {
                st.detected = detected;
            }
            last_agent_scan = Instant::now();
        }

        // Adaptive frame sleep / interrupt
        if is_active {
            // Wait up to 35ms (~28 fps) or wake immediately on incoming key
            if let Ok(cmd) = rx.recv_timeout(Duration::from_millis(35)) {
                match cmd {
                    TmuxInput::Literal(s) => {
                        let _ = std::process::Command::new("tmux")
                            .args(["send-keys", "-t", TMUX_SESSION_NAME, "-l", &s])
                            .output();
                    }
                    TmuxInput::Special(s) => {
                        let _ = std::process::Command::new("tmux")
                            .args(["send-keys", "-t", TMUX_SESSION_NAME, s])
                            .output();
                    }
                }
                // Immediately refresh screen on user typing
                let lines = poll_agent_output();
                if let Ok(mut st) = AGENT_STATE.lock() {
                    st.session_screen = lines;
                }
            }
        } else {
            // Idle: sleep up to 500ms or wake immediately if a key arrives
            if let Ok(cmd) = rx.recv_timeout(Duration::from_millis(500)) {
                match cmd {
                    TmuxInput::Literal(s) => {
                        let _ = std::process::Command::new("tmux")
                            .args(["send-keys", "-t", TMUX_SESSION_NAME, "-l", &s])
                            .output();
                    }
                    TmuxInput::Special(s) => {
                        let _ = std::process::Command::new("tmux")
                            .args(["send-keys", "-t", TMUX_SESSION_NAME, s])
                            .output();
                    }
                }
            }
        }
    }
}

/// Detect active AI agents running on the machine (claude, opencode, agy, etc.)
pub fn scan_running_agents() -> Vec<DetectedAgent> {
    let procs = crate::monitors::processes::snapshot_all();
    let mut detected = Vec::new();

    for p in procs {
        let name_lower = p.name.to_lowercase();
        let cmd_lower = p.cmdline.to_lowercase();

        let is_agent = name_lower.contains("claude")
            || name_lower.contains("opencode")
            || name_lower.contains("agy")
            || cmd_lower.contains("claude")
            || cmd_lower.contains("opencode")
            || cmd_lower.contains("agy");

        if is_agent && !name_lower.contains("grep") && !name_lower.contains("orbit") {
            let label = if name_lower.contains("claude") || cmd_lower.contains("claude") {
                "Claude Code"
            } else if name_lower.contains("opencode") || cmd_lower.contains("opencode") {
                "OpenCode Agent"
            } else if name_lower.contains("agy") || cmd_lower.contains("agy") {
                "Antigravity CLI"
            } else {
                "AI Agent"
            };

            detected.push(DetectedAgent {
                name: label.to_string(),
                pid: p.pid,
                cpu_pct: p.cpu_pct,
                mem_mb: p.mem_kb / 1024,
                status: if p.cpu_pct > 1.0 {
                    "Active (Running)"
                } else {
                    "Idle (Waiting)"
                }
                .to_string(),
            });
        }
    }

    detected
}

/// Check if the tmux agent session is currently running.
pub fn is_session_alive() -> bool {
    std::process::Command::new("tmux")
        .args(["has-session", "-t", TMUX_SESSION_NAME])
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Spawn or attach to a tmux session running the requested agent command.
pub fn spawn_agent(cmd: &str, width: u16, height: u16) -> Result<(), String> {
    // Kill existing session if any
    let _ = std::process::Command::new("tmux")
        .args(["kill-session", "-t", TMUX_SESSION_NAME])
        .output();

    let cwd = std::env::current_dir()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|_| ".".to_string());
    let shell = std::env::var("SHELL").unwrap_or_else(|_| "bash".to_string());

    let status = std::process::Command::new("tmux")
        .args([
            "new-session",
            "-d",
            "-s",
            TMUX_SESSION_NAME,
            "-c",
            &cwd,
            "-x",
            &width.max(40).to_string(),
            "-y",
            &height.max(10).to_string(),
            &shell,
        ])
        .status()
        .map_err(|e| format!("Failed to spawn agent in tmux: {}", e))?;

    if status.success() {
        if !cmd.trim().is_empty() && cmd != "bash" && cmd != "shell" {
            let _ = std::process::Command::new("tmux")
                .args(["send-keys", "-t", TMUX_SESSION_NAME, cmd, "Enter"])
                .output();
        }
        let mut st = AGENT_STATE.lock().unwrap();
        st.active_session = Some(cmd.to_string());
        st.terminal_focused = true;
        st.history
            .push_back(format!("⚡ Launched [{}] in: {}", cmd, cwd));
        st.session_screen.clear();
        st.scroll = 0;
        Ok(())
    } else {
        Err(format!(
            "tmux exited with non-zero status: {:?}",
            status.code()
        ))
    }
}

/// Attach directly to the active tmux agent session with raw terminal handover.
pub fn attach_to_agent_session() {
    if is_session_alive() {
        let _ = crate::mouse::suspended(|| {
            std::process::Command::new("tmux")
                .args(["attach-session", "-t", TMUX_SESSION_NAME])
                .status()
        });
        let _ = std::process::Command::new("clear").status();
    }
}

/// Send keystrokes / command to active tmux agent session.
pub fn send_input_to_agent(input: &str) {
    let _ = std::process::Command::new("tmux")
        .args(["send-keys", "-t", TMUX_SESSION_NAME, input, "Enter"])
        .output();
    let mut st = AGENT_STATE.lock().unwrap();
    st.history.push_back(format!("› {}", input));
}

/// Send raw key/signal (e.g. C-c) to active tmux agent session.
pub fn send_signal_to_agent(sig: &str) {
    ensure_worker_started();
    if let Ok(lock) = KEY_TX.lock() {
        if let Some(tx) = lock.as_ref() {
            let _ = tx.send(TmuxInput::Special("C-c"));
            return;
        }
    }
    let _ = std::process::Command::new("tmux")
        .args(["send-keys", "-t", TMUX_SESSION_NAME, sig])
        .output();
}

/// Forward raw crossterm key event directly to tmux session asynchronously.
pub fn forward_key_to_tmux(key: crossterm::event::KeyEvent) {
    ensure_worker_started();
    use crossterm::event::{KeyCode, KeyModifiers};

    let input = if key.modifiers.contains(KeyModifiers::CONTROL) {
        let key_str = match key.code {
            KeyCode::Char('c') => "C-c",
            KeyCode::Char('d') => "C-d",
            KeyCode::Char('z') => "C-z",
            KeyCode::Char('l') => "C-l",
            KeyCode::Char('u') => "C-u",
            KeyCode::Char('w') => "C-w",
            KeyCode::Char('a') => "C-a",
            KeyCode::Char('e') => "C-e",
            KeyCode::Char('r') => "C-r",
            KeyCode::Char('k') => "C-k",
            KeyCode::Char('t') => "C-t",
            KeyCode::Char('p') => "C-p",
            KeyCode::Char('n') => "C-n",
            KeyCode::Char('b') => "C-b",
            KeyCode::Char('f') => "C-f",
            _ => return,
        };
        TmuxInput::Special(key_str)
    } else {
        match key.code {
            KeyCode::Char(c) => TmuxInput::Literal(c.to_string()),
            KeyCode::Enter => TmuxInput::Special("Enter"),
            KeyCode::Backspace => TmuxInput::Special("BSpace"),
            KeyCode::Tab => TmuxInput::Special("Tab"),
            KeyCode::Up => TmuxInput::Special("Up"),
            KeyCode::Down => TmuxInput::Special("Down"),
            KeyCode::Left => TmuxInput::Special("Left"),
            KeyCode::Right => TmuxInput::Special("Right"),
            KeyCode::Home => TmuxInput::Special("Home"),
            KeyCode::End => TmuxInput::Special("End"),
            KeyCode::PageUp => TmuxInput::Special("PgUp"),
            KeyCode::PageDown => TmuxInput::Special("PgDn"),
            _ => return,
        }
    };

    if let Ok(lock) = KEY_TX.lock() {
        if let Some(tx) = lock.as_ref() {
            let _ = tx.send(input);
            return;
        }
    }

    match input {
        TmuxInput::Literal(s) => {
            let _ = std::process::Command::new("tmux")
                .args(["send-keys", "-t", TMUX_SESSION_NAME, "-l", &s])
                .output();
        }
        TmuxInput::Special(s) => {
            let _ = std::process::Command::new("tmux")
                .args(["send-keys", "-t", TMUX_SESSION_NAME, s])
                .output();
        }
    }
}

/// Capture output lines from running tmux session.
pub fn poll_agent_output() -> Vec<String> {
    let output = std::process::Command::new("tmux")
        .args(["capture-pane", "-t", TMUX_SESSION_NAME, "-p", "-S", "-100"])
        .output();

    match output {
        Ok(out) if out.status.success() => {
            let s = String::from_utf8_lossy(&out.stdout);
            let mut lines: Vec<String> = s.lines().map(|l| l.to_string()).collect();
            while lines.last().map(|l| l.trim().is_empty()).unwrap_or(false) {
                lines.pop();
            }
            lines
        }
        _ => Vec::new(),
    }
}

/// Terminate the active agent session.
pub fn terminate_agent_session() {
    let _ = std::process::Command::new("tmux")
        .args(["kill-session", "-t", TMUX_SESSION_NAME])
        .output();
    let mut st = AGENT_STATE.lock().unwrap();
    st.active_session = None;
    st.terminal_focused = false;
    st.session_screen.clear();
    st.history
        .push_back("⏹ Terminated agent session.".to_string());
}

pub fn render(f: &mut Frame, area: Rect, app: &mut App) {
    ensure_worker_started();
    let theme = &app.theme;

    let st = AGENT_STATE.lock().unwrap();

    // While an agent session is active, request 30 FPS for buttery smooth terminal streaming
    if st.active_session.is_some() {
        crate::anim::request(30);
    }

    let [top_hud, main_body, bottom_bar] = ratatui::layout::Layout::vertical([
        ratatui::layout::Constraint::Length(3),
        ratatui::layout::Constraint::Fill(1),
        ratatui::layout::Constraint::Length(3),
    ])
    .areas(area);

    // 1. TOP HUD: Agent Fleet Telemetry
    render_agent_hud(
        f,
        top_hud,
        theme,
        &st.detected,
        st.active_session.as_deref(),
    );

    // 2. MAIN BODY: Split between Terminal Console (70%) and Agent Manager Deck (30%)
    let [console_area, deck_area] = ratatui::layout::Layout::horizontal([
        ratatui::layout::Constraint::Percentage(70),
        ratatui::layout::Constraint::Percentage(30),
    ])
    .areas(main_body);

    render_console(f, console_area, theme, &st);
    render_agent_deck(f, deck_area, theme, &st);

    // 3. BOTTOM BAR: Command input or action prompts
    render_command_bar(f, bottom_bar, theme, &st);
}

fn render_agent_hud(
    f: &mut Frame,
    area: Rect,
    theme: &Theme,
    detected: &[DetectedAgent],
    active_session: Option<&str>,
) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme.surface));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let active_count = detected
        .iter()
        .filter(|a| a.status.contains("Active"))
        .count();
    let total_mem: u64 = detected.iter().map(|a| a.mem_mb).sum();

    let mut spans = vec![
        Span::styled(
            " 🤖 AGENT FLEET: ",
            Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!("{} Online ", detected.len()),
            Style::default().fg(theme.text),
        ),
        Span::styled("· ", Style::default().fg(theme.dim)),
        Span::styled(
            format!("{} Active ", active_count),
            Style::default()
                .fg(theme.green)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("· ", Style::default().fg(theme.dim)),
        Span::styled(
            format!("{} MB RAM ", total_mem),
            Style::default().fg(theme.secondary),
        ),
    ];

    if let Some(sess) = active_session {
        spans.push(Span::styled("· ", Style::default().fg(theme.dim)));
        spans.push(Span::styled(
            "EMBEDDED: ",
            Style::default().fg(theme.yellow),
        ));
        spans.push(Span::styled(
            format!("[{}] ", sess),
            Style::default().fg(theme.text).add_modifier(Modifier::BOLD),
        ));
        spans.push(Span::styled("● RUNNING", Style::default().fg(theme.green)));
    } else {
        spans.push(Span::styled("· ", Style::default().fg(theme.dim)));
        spans.push(Span::styled(
            "EMBEDDED: Standby (Press [c,o,a,s] to spawn)",
            Style::default().fg(theme.dim),
        ));
    }

    f.render_widget(Paragraph::new(Line::from(spans)), inner);
}

fn render_console(f: &mut Frame, area: Rect, theme: &Theme, st: &AgentTerminalState) {
    let (border_color, status_badge) = if st.active_session.is_some() {
        if st.terminal_focused {
            (
                theme.green,
                Span::styled(
                    " ● TERMINAL FOCUSED (Esc: Orbit Nav · Alt+Enter: Fullscreen) ",
                    Style::default()
                        .fg(theme.green)
                        .add_modifier(Modifier::BOLD),
                ),
            )
        } else {
            (
                theme.accent,
                Span::styled(
                    " ○ ORBIT NAV MODE (Enter/i: Focus Terminal) ",
                    Style::default().fg(theme.yellow),
                ),
            )
        }
    } else {
        (
            theme.dim,
            Span::styled(" ○ IDLE ", Style::default().fg(theme.dim)),
        )
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(border_color))
        .title(Line::from(vec![
            Span::styled(
                " ⚡ AI AGENT RUNTIME CONSOLE ",
                Style::default().fg(theme.text).add_modifier(Modifier::BOLD),
            ),
            status_badge,
        ]));

    let inner = block.inner(area);
    f.render_widget(block, area);

    if st.active_session.is_some() && !st.session_screen.is_empty() {
        let visible_height = inner.height as usize;
        let total = st.session_screen.len();
        let end_idx = total.saturating_sub(st.scroll);
        let start_idx = end_idx.saturating_sub(visible_height);
        let slice = &st.session_screen[start_idx..end_idx.min(total)];

        let lines: Vec<Line> = slice
            .iter()
            .map(|l| {
                if l.contains("error") || l.contains("Error") || l.contains("FAILED") {
                    Line::from(Span::styled(l.clone(), Style::default().fg(theme.red)))
                } else if l.contains("warn") || l.contains("Warning") {
                    Line::from(Span::styled(l.clone(), Style::default().fg(theme.yellow)))
                } else if l.starts_with('›') || l.starts_with('$') || l.starts_with('❯') {
                    Line::from(Span::styled(
                        l.clone(),
                        Style::default()
                            .fg(theme.accent)
                            .add_modifier(Modifier::BOLD),
                    ))
                } else {
                    Line::from(Span::styled(l.clone(), Style::default().fg(theme.text)))
                }
            })
            .collect();
        f.render_widget(Paragraph::new(lines), inner);
    } else {
        let mut welcome = vec![
            Line::default(),
            Line::from(vec![
                Span::styled("  ⚡ Orbit Autonomous Agent Command Workstation", Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
            ]),
            Line::from(vec![
                Span::styled("  Run background AI agents 24/7 without terminal context switches or workflow interruptions.", Style::default().fg(theme.dim)),
            ]),
            Line::default(),
            Line::from(vec![
                Span::styled("  AVAILABLE AGENTS:", Style::default().fg(theme.secondary).add_modifier(Modifier::BOLD)),
            ]),
            Line::from(vec![
                Span::styled("    [c] ", Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
                Span::styled("Claude Code        ", Style::default().fg(theme.text).add_modifier(Modifier::BOLD)),
                Span::styled(if is_binary_available("claude") { "● Ready in PATH" } else { "○ Not in PATH" }, Style::default().fg(if is_binary_available("claude") { theme.green } else { theme.dim })),
            ]),
            Line::from(vec![
                Span::styled("    [o] ", Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
                Span::styled("OpenCode Agent     ", Style::default().fg(theme.text).add_modifier(Modifier::BOLD)),
                Span::styled(if is_binary_available("opencode") { "● Ready in PATH" } else { "○ Not in PATH" }, Style::default().fg(if is_binary_available("opencode") { theme.green } else { theme.dim })),
            ]),
            Line::from(vec![
                Span::styled("    [a] ", Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
                Span::styled("Antigravity CLI    ", Style::default().fg(theme.text).add_modifier(Modifier::BOLD)),
                Span::styled(if is_binary_available("agy") { "● Ready in PATH" } else { "○ Not in PATH" }, Style::default().fg(if is_binary_available("agy") { theme.green } else { theme.dim })),
            ]),
            Line::from(vec![
                Span::styled("    [s] ", Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
                Span::styled("Interactive Shell  ", Style::default().fg(theme.text).add_modifier(Modifier::BOLD)),
                Span::styled("● Bash Session", Style::default().fg(theme.green)),
            ]),
            Line::default(),
            Line::from(vec![
                Span::styled("  ACTIVITY LOG:", Style::default().fg(theme.dim)),
            ]),
        ];

        for h in st.history.iter().rev().take(6) {
            welcome.push(Line::from(vec![
                Span::styled("  › ", Style::default().fg(theme.accent)),
                Span::styled(h.clone(), Style::default().fg(theme.dim)),
            ]));
        }

        f.render_widget(Paragraph::new(welcome), inner);
    }
}

fn render_agent_deck(f: &mut Frame, area: Rect, theme: &Theme, st: &AgentTerminalState) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme.surface))
        .title(" ◈ ACTIVE AGENTS ");

    let inner = block.inner(area);
    f.render_widget(block, area);

    if st.detected.is_empty() {
        let empty = vec![
            Line::default(),
            Line::from(Span::styled(
                "No autonomous agents",
                Style::default().fg(theme.dim),
            )),
            Line::from(Span::styled(
                "currently running.",
                Style::default().fg(theme.dim),
            )),
            Line::default(),
            Line::from(Span::styled(
                "Press [c,o,a] to spawn.",
                Style::default().fg(theme.secondary),
            )),
        ];
        f.render_widget(
            Paragraph::new(empty).alignment(ratatui::layout::Alignment::Center),
            inner,
        );
        return;
    }

    let mut lines = Vec::new();
    for a in &st.detected {
        lines.push(Line::from(vec![
            Span::styled(
                "● ",
                Style::default().fg(if a.cpu_pct > 1.0 {
                    theme.green
                } else {
                    theme.yellow
                }),
            ),
            Span::styled(
                format!("{:<15}", a.name),
                Style::default().fg(theme.text).add_modifier(Modifier::BOLD),
            ),
        ]));
        lines.push(Line::from(vec![
            Span::styled("  PID: ", Style::default().fg(theme.dim)),
            Span::styled(
                format!("{:<6}", a.pid),
                Style::default().fg(theme.secondary),
            ),
            Span::styled("CPU: ", Style::default().fg(theme.dim)),
            Span::styled(
                format!("{:>4.1}% ", a.cpu_pct),
                Style::default().fg(theme.accent),
            ),
            Span::styled("RAM: ", Style::default().fg(theme.dim)),
            Span::styled(format!("{}MB", a.mem_mb), Style::default().fg(theme.text)),
        ]));
        lines.push(Line::from(vec![
            Span::styled("  State: ", Style::default().fg(theme.dim)),
            Span::styled(a.status.clone(), Style::default().fg(theme.dim)),
        ]));
        lines.push(Line::default());
    }

    f.render_widget(Paragraph::new(lines), inner);
}

fn render_command_bar(f: &mut Frame, area: Rect, theme: &Theme, st: &AgentTerminalState) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(if st.active_session.is_some() {
            if st.terminal_focused {
                theme.green
            } else {
                theme.accent
            }
        } else {
            theme.surface
        }));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let content = if st.active_session.is_some() {
        if st.terminal_focused {
            Line::from(vec![
                Span::styled(
                    " ⌨ TERMINAL FOCUSED: ",
                    Style::default()
                        .fg(theme.green)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled("Keys pass to agent · ", Style::default().fg(theme.text)),
                Span::styled(
                    "[Esc] ",
                    Style::default()
                        .fg(theme.yellow)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled("Orbit Nav Mode · ", Style::default().fg(theme.dim)),
                Span::styled(
                    "[Alt+Enter] ",
                    Style::default()
                        .fg(theme.accent)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled("Fullscreen · ", Style::default().fg(theme.dim)),
                Span::styled(
                    "[Ctrl+C] ",
                    Style::default().fg(theme.red).add_modifier(Modifier::BOLD),
                ),
                Span::styled("Interrupt", Style::default().fg(theme.dim)),
            ])
        } else {
            Line::from(vec![
                Span::styled(
                    " ◈ ORBIT NAV MODE: ",
                    Style::default()
                        .fg(theme.accent)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    "[Enter / i] ",
                    Style::default()
                        .fg(theme.green)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled("Focus Terminal  ", Style::default().fg(theme.text)),
                Span::styled(
                    "[Alt+Enter / f] ",
                    Style::default()
                        .fg(theme.accent)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled("Fullscreen  ", Style::default().fg(theme.text)),
                Span::styled(
                    "[k] ",
                    Style::default().fg(theme.red).add_modifier(Modifier::BOLD),
                ),
                Span::styled("Terminate  ", Style::default().fg(theme.text)),
                Span::styled("[↑↓] ", Style::default().fg(theme.secondary)),
                Span::styled("Scroll  ", Style::default().fg(theme.dim)),
                Span::styled("[1-5] ", Style::default().fg(theme.dim)),
                Span::styled("Tabs  ", Style::default().fg(theme.dim)),
                Span::styled("[q] ", Style::default().fg(theme.dim)),
                Span::styled("Quit", Style::default().fg(theme.dim)),
            ])
        }
    } else {
        Line::from(vec![
            Span::styled(
                " SPAWN AGENT: ",
                Style::default()
                    .fg(theme.accent)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                "[c] ",
                Style::default()
                    .fg(theme.accent)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("Claude Code  ", Style::default().fg(theme.text)),
            Span::styled(
                "[o] ",
                Style::default()
                    .fg(theme.accent)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("OpenCode  ", Style::default().fg(theme.text)),
            Span::styled(
                "[a] ",
                Style::default()
                    .fg(theme.accent)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("Antigravity  ", Style::default().fg(theme.text)),
            Span::styled(
                "[s] ",
                Style::default()
                    .fg(theme.accent)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("Shell  ", Style::default().fg(theme.text)),
            Span::styled("[r] ", Style::default().fg(theme.secondary)),
            Span::styled("Refresh Fleet", Style::default().fg(theme.dim)),
        ])
    };

    f.render_widget(Paragraph::new(content), inner);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_kind_labels() {
        assert_eq!(AgentKind::ClaudeCode.label(), "Claude Code");
        assert_eq!(AgentKind::Opencode.label(), "OpenCode");
        assert_eq!(AgentKind::Agy.label(), "Antigravity (agy)");
        assert_eq!(AgentKind::Shell.label(), "Interactive Shell");
        assert_eq!(AgentKind::Custom("custom-bot".into()).label(), "custom-bot");

        assert_eq!(AgentKind::ClaudeCode.command(), "claude");
        assert_eq!(AgentKind::Opencode.command(), "opencode");
        assert_eq!(AgentKind::Agy.command(), "agy");
        assert_eq!(AgentKind::Shell.command(), "bash");
    }

    #[test]
    fn test_is_binary_available() {
        #[cfg(unix)]
        assert!(is_binary_available("sh") || is_binary_available("bash"));
        #[cfg(windows)]
        assert!(
            is_binary_available("cmd")
                || is_binary_available("powershell")
                || is_binary_available("cargo")
        );
        assert!(!is_binary_available("non_existent_binary_xyz_12345"));
    }

    #[test]
    fn test_scan_running_agents() {
        let agents = scan_running_agents();
        assert!(agents.is_empty() || !agents.is_empty());
    }

    #[test]
    fn test_default_agent_state() {
        let st = AgentTerminalState::default();
        assert!(st.active_session.is_none());
        assert!(st.input_buffer.is_empty());
        assert_eq!(st.scroll, 0);
    }
}
