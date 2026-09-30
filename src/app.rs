use std::collections::HashSet;
use std::time::{Duration, Instant};

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::config::Config;
use crate::custom::CustomWidgetManager;
use crate::mode::DashboardMode;
use crate::monitors::{self, processes, Summary};
use crate::screens;
use crate::theme::Theme;
use crate::widgets::media::{self, Action};
use crate::widgets::music_viz;

/// Sort field for the processes panel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortField {
    Cpu,
    Mem,
    Pid,
    Name,
}

impl SortField {
    pub fn label(&self) -> &'static str {
        match self {
            SortField::Cpu => "cpu",
            SortField::Mem => "mem",
            SortField::Pid => "pid",
            SortField::Name => "name",
        }
    }
    pub fn next(&self) -> Self {
        match self {
            SortField::Cpu => SortField::Mem,
            SortField::Mem => SortField::Pid,
            SortField::Pid => SortField::Name,
            SortField::Name => SortField::Cpu,
        }
    }
}

/// Every focusable panel across all pages.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PanelId {
    System,
    Gauges,
    Cpu,
    Memory,
    Disk,
    Storage,
    Network,
    Gpu,
    Clock,
    Media,
    Visualizer,
    Processes,
    Status,
    Calendar,
    Matrix,
    Video,
    Weather,
    UpNext,
    Timer,
    Tasks,
    Agenda,
    News,
    WriterNotes,
    Files,
    PinnedMedia,
    /// A user-defined custom widget at the given index in `CustomWidgetManager`.
    Custom(usize),
}

impl PanelId {
    /// Tab order for a page, honouring widget toggles and custom dashboard layout.
    pub fn for_mode(mode: &DashboardMode, cfg: &Config) -> Vec<PanelId> {
        if *mode == DashboardMode::Dashboard {
            let mut list = Vec::new();
            for col in &cfg.dashboard.layout {
                for name in col {
                    if let Some(p) = Self::from_name(name, cfg) {
                        if !list.contains(&p) {
                            list.push(p);
                        }
                    }
                }
            }
            for (i, cw) in cfg.custom_widgets.iter().enumerate() {
                if cw.enabled && !list.contains(&PanelId::Custom(i)) {
                    list.push(PanelId::Custom(i));
                }
            }
            return list;
        }

        let w = &cfg.widgets;
        let list: Vec<(PanelId, bool)> = match mode {
            DashboardMode::Dashboard => unreachable!(),
            DashboardMode::Monitor => vec![
                (PanelId::Cpu, true),
                (PanelId::Memory, true),
                (PanelId::Disk, true),
                (PanelId::Network, true),
                (PanelId::Gpu, w.gpu),
                (PanelId::System, true),
                (PanelId::Processes, true),
            ],
            DashboardMode::Workspace => vec![
                (PanelId::Timer, true),
                (PanelId::Agenda, w.agenda),
                (PanelId::Tasks, w.tasks),
                (PanelId::News, w.news),
                (PanelId::WriterNotes, true),
                (PanelId::Files, true),
            ],
            // Ambient scenes are full-screen and borderless: nothing to focus.
            DashboardMode::Aesthetic => vec![],
            DashboardMode::Extension(ref ext_name) => {
                let mut list = Vec::new();
                if let Some(page) = cfg.pages.iter().find(|p| &p.name == ext_name) {
                    for col in &page.layout {
                        for name in col {
                            if let Some(p) = Self::from_name(name, cfg) {
                                if !list.contains(&p) {
                                    list.push(p);
                                }
                            }
                        }
                    }
                }
                return list;
            }
            DashboardMode::DebugLogs => vec![],
        };
        list.into_iter()
            .filter(|(_, on)| *on)
            .map(|(p, _)| p)
            .collect()
    }

    pub fn from_name(name: &str, cfg: &Config) -> Option<PanelId> {
        let w = &cfg.widgets;
        match name.to_lowercase().as_str() {
            "system" => Some(PanelId::System),
            "gauges" | "gauge" => Some(PanelId::Gauges),
            "cpu" => w.cpu.then_some(PanelId::Cpu),
            "storage" => w.disk.then_some(PanelId::Storage),
            "disk" => w.disk.then_some(PanelId::Disk),
            "clock" => w.clock.then_some(PanelId::Clock),
            "media" | "now_playing" | "now-playing" => w.media.then_some(PanelId::Media),
            "visualizer" | "viz" => w.music_viz.then_some(PanelId::Visualizer),
            "processes" | "procs" | "top_processes" | "top-processes" => {
                w.processes.then_some(PanelId::Processes)
            }
            "status" => Some(PanelId::Status),
            "weather" => w.weather.then_some(PanelId::Weather),
            "upnext" | "up_next" | "up-next" | "next" => Some(PanelId::UpNext),
            "timer" | "pomodoro" | "focus" => Some(PanelId::Timer),
            "memory" | "mem" => w.memory.then_some(PanelId::Memory),
            "network" | "net" => w.network.then_some(PanelId::Network),
            "calendar" | "cal" => w.calendar.then_some(PanelId::Calendar),
            "matrix" => w.matrix.then_some(PanelId::Matrix),
            "gpu" => w.gpu.then_some(PanelId::Gpu),
            "tasks" | "todo" => w.tasks.then_some(PanelId::Tasks),
            "agenda" => w.agenda.then_some(PanelId::Agenda),
            "news" => w.news.then_some(PanelId::News),
            "pinned_media" | "media_preview" | "media-preview" | "image" => {
                w.pinned_media.then_some(PanelId::PinnedMedia)
            }
            "video" | "donut" => w.video.then_some(PanelId::Video),
            "notes" | "writer" | "obsidian" => Some(PanelId::WriterNotes),
            "files" => Some(PanelId::Files),
            custom_id => {
                if let Some(pos) = cfg
                    .custom_widgets
                    .iter()
                    .position(|cw| cw.enabled && cw.id.eq_ignore_ascii_case(custom_id))
                {
                    Some(PanelId::Custom(pos))
                } else {
                    Some(PanelId::Custom(999))
                }
            }
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            PanelId::System => "system",
            PanelId::Gauges => "gauges",
            PanelId::Cpu => "cpu",
            PanelId::Memory => "memory",
            PanelId::Disk => "disk",
            PanelId::Storage => "storage",
            PanelId::Network => "network",
            PanelId::Gpu => "gpu",
            PanelId::Clock => "clock",
            PanelId::Media => "media",
            PanelId::Visualizer => "visualizer",
            PanelId::Processes => "processes",
            PanelId::Status => "status",
            PanelId::Calendar => "calendar",
            PanelId::Matrix => "matrix",
            PanelId::Video => "donut",
            PanelId::Weather => "weather",
            PanelId::UpNext => "up next",
            PanelId::Timer => "focus timer",
            PanelId::Custom(_) => "custom",
            PanelId::PinnedMedia => "media preview",
            &PanelId::Tasks => "tasks",
            &PanelId::Agenda => "agenda",
            &PanelId::News => "news",
            &PanelId::WriterNotes => "notes",
            &PanelId::Files => "files",
        }
    }
}

/// A condition worth interrupting a glance for.
pub struct Alert {
    /// Stable identity, for notifying once per occurrence.
    pub kind: &'static str,
    /// Short form for the title-bar pill.
    pub short: String,
    /// Sentence for the desktop notification.
    pub long: String,
    pub critical: bool,
}

/// Active alerts, most severe first.
pub fn alerts(s: &Summary) -> Vec<Alert> {
    let mut out = Vec::new();
    let mut push = |kind, short: String, long: String, critical| {
        out.push(Alert {
            kind,
            short,
            long,
            critical,
        })
    };
    if let Some((p, false)) = s.battery {
        if p <= 20 {
            let long = if p <= 10 {
                format!("Battery critical: {}%. Plug in now.", p)
            } else {
                format!("Battery low: {}%.", p)
            };
            push("bat", format!("bat {}%", p), long, p <= 10);
        }
    }
    if let Some(c) = s.temp_c.filter(|_| s.hot) {
        let crit = s.temp_critical();
        let long = if crit {
            format!("CPU at {:.0}°, at its thermal limit. It will throttle.", c)
        } else {
            format!("CPU running hot: {:.0}°.", c)
        };
        push("hot", format!("hot {:.0}°", c), long, crit);
    }
    if let Some(d) = s.disk_pct.filter(|d| *d >= 90.0) {
        push(
            "disk",
            format!("disk {:.0}%", d),
            format!("Root disk {:.0}% full.", d),
            d >= 97.0,
        );
    }
    if s.mem_pct >= 90.0 {
        push(
            "mem",
            format!("mem {:.0}%", s.mem_pct),
            format!("Memory {:.0}% used.", s.mem_pct),
            s.mem_pct >= 97.0,
        );
    }
    out.sort_by_key(|a| !a.critical);
    out
}

/// Per-panel interactive state.
#[derive(Debug, Clone)]
pub struct PanelStates {
    pub calendar_month_offset: i32,
    pub process_scroll_offset: usize,
    pub process_sort_field: SortField,
    pub process_sort_asc: bool,
    pub process_search: String,
    pub process_search_active: bool,
    pub process_tree_mode: bool,
    pub log_target: String,
    pub log_target_input_active: bool,
    pub process_compact_cmd: bool,
    pub writer_scroll: usize,
    /// First line of the selected note shown in the preview.
    pub note_scroll: u16,
    pub writer_selected: usize,
    pub files_selected: usize,
    pub files_scroll: usize,
    pub files_show_hidden: bool,
    pub pixel_images: bool,
    pub force_robot_logo: bool,
    pub files_rename_input_active: bool,
    pub files_rename_input: String,
    pub files_search_input_active: bool,
    pub files_search_input: String,
    pub files_mkdir_input_active: bool,
    pub files_mkdir_input: String,

    pub tasks_selected: usize,
    pub task_input_active: bool,
    pub task_input: String,
    pub agenda_selected: usize,
    pub agenda_input_active: bool,
    /// Index of the event being edited; None while adding a new one.
    pub agenda_edit: Option<usize>,
    pub agenda_input: String,
    pub pinned_media_input_active: bool,
    pub pinned_media_input: String,
    pub status_selected: usize,
    pub dash_ratios: [u16; 3],
    pub dash_vertical: std::collections::HashMap<String, i16>,
    pub work_ratio: u16,
    pub process_selected_pid: Option<u32>,
    pub process_collapsed: HashSet<u32>,
}

impl Default for PanelStates {
    fn default() -> Self {
        Self {
            calendar_month_offset: 0,
            process_scroll_offset: 0,
            process_sort_field: SortField::Cpu,
            process_sort_asc: false,
            process_search: String::new(),
            process_search_active: false,
            process_tree_mode: false,
            log_target: String::new(),
            log_target_input_active: false,
            process_compact_cmd: true,
            writer_scroll: 0,
            note_scroll: 0,
            writer_selected: 0,
            files_selected: 0,
            files_scroll: 0,
            files_show_hidden: false,
            pixel_images: true,
            force_robot_logo: false,
            files_rename_input_active: false,
            files_rename_input: String::new(),
            files_search_input_active: false,
            files_search_input: String::new(),
            files_mkdir_input_active: false,
            files_mkdir_input: String::new(),

            tasks_selected: 0,
            task_input_active: false,
            task_input: String::new(),
            agenda_selected: 0,
            agenda_input_active: false,
            agenda_edit: None,
            agenda_input: String::new(),
            pinned_media_input_active: false,
            pinned_media_input: String::new(),
            status_selected: 0,
            dash_ratios: [33, 34, 33],
            dash_vertical: std::collections::HashMap::new(),
            work_ratio: 25,
            process_selected_pid: None,
            process_collapsed: HashSet::new(),
        }
    }
}

pub struct App {
    pub running: bool,
    pub config: Config,
    pub ext_manager: crate::extension::ExtensionManager,
    pub theme: Theme,
    pub mode: DashboardMode,
    pub frame: u64,
    pub focused_panel: Option<PanelId>,
    pub panel_states: PanelStates,
    pub ambient: crate::screens::aesthetic::AmbientState,
    pub show_help: bool,
    /// The Esc menu.
    pub show_menu: bool,
    pub menu_row: usize,
    pub help_scroll: u16,
    pub show_settings: bool,
    pub show_setup_wizard: bool,
    pub setup_wizard_step: usize,
    pub setup_install_log: String,
    pub settings_row: usize,
    pub settings_scroll: usize,
    pub settings_search: String,
    pub settings_search_active: bool,
    /// A focused panel expanded to fill the page (Enter / Esc).
    pub zoomed: Option<PanelId>,
    pub summary: Summary,
    /// When the last key was pressed; ambient hides its hints once idle.
    pub last_input: Instant,
    /// Last left click (time, column, row), for double-click detection.
    last_click: Option<(Instant, u16, u16)>,
    alert_latch: crate::notify::AlertLatch,
    /// Whether `theme` is currently the night-dimmed variant; `None` forces
    /// it to be rebuilt on the next frame (after a theme or setting change).
    pub night: Option<bool>,
    /// Short transient message shown in the status bar (theme changed, killed pid …).
    toast: Option<(String, Instant)>,
    /// First press of k/K arms this; the second press within the window fires.
    pending_signal: Option<(u32, &'static str, Instant)>,
    sampler_interval: std::sync::Arc<std::sync::atomic::AtomicU64>,
    /// Manager for all user-defined custom widgets.
    pub custom_widgets: CustomWidgetManager,
    /// Image protocol picker for ratatui-image rendering.
    pub image_picker: Option<ratatui_image::picker::Picker>,
    pub image_protocols:
        std::collections::HashMap<String, ratatui_image::protocol::StatefulProtocol>,
    /// Sender to the background resize-encode worker thread.
    pub image_resize_tx: Option<std::sync::mpsc::Sender<ratatui_image::thread::ResizeRequest>>,
    /// Receiver for completed resize responses from the worker.
    pub image_resize_rx: Option<
        std::sync::mpsc::Receiver<
            Result<ratatui_image::thread::ResizeResponse, ratatui_image::errors::Errors>,
        >,
    >,
    /// Non-blocking ThreadProtocol image states keyed by path.
    pub thread_protocols: std::collections::HashMap<String, ratatui_image::thread::ThreadProtocol>,
}

impl App {
    pub fn apply_performance_mode(&mut self, mode: &crate::config::PerformanceMode) {
        use crate::config::PerformanceMode::*;
        let (fps, idle_fps, refresh) = match mode {
            VeryLight => (15, 1, 3.0),
            Light => (30, 1, 2.0),
            Normal => (60, 2, 0.5),
            High => (120, 5, 0.25),
            VeryHigh => (144, 10, 0.1),
        };
        self.config.ui.fps = fps;
        self.config.ui.refresh_rate = refresh;
        crate::anim::set_idle_fps(idle_fps);
        self.sampler_interval.store(
            (refresh * 1000.0) as u64,
            std::sync::atomic::Ordering::Relaxed,
        );
    }

    pub fn new(config: Config) -> Self {
        let theme = Theme::from_name(&config.ui.theme);
        let mode = DashboardMode::from_str(&config.ui.startup_mode);
        let sampler_interval = monitors::start(Duration::from_secs_f64(config.ui.refresh_rate));
        music_viz::set_style(&config.ui.visualizer);
        crate::widgets::gauge::set_style(&config.ui.gauge_style);
        crate::widgets::meter::set_style(&config.ui.meter_style);
        crate::widgets::block_graph::set_style(&config.ui.graph_style);
        let custom_widgets = CustomWidgetManager::start_all(&config.custom_widgets);
        crate::anim::set_idle_fps(match config.ui.performance_mode {
            crate::config::PerformanceMode::VeryLight | crate::config::PerformanceMode::Light => 1,
            crate::config::PerformanceMode::Normal => 2,
            crate::config::PerformanceMode::High => 5,
            crate::config::PerformanceMode::VeryHigh => 10,
        });
        let mut app = Self {
            running: true,
            ext_manager: crate::extension::ExtensionManager::new(),
            theme,
            config,
            mode: mode.clone(),
            frame: 0,
            focused_panel: None,
            panel_states: PanelStates::default(),
            ambient: Default::default(),
            show_help: false,
            show_menu: false,
            menu_row: 0,
            help_scroll: 0,
            show_settings: false,
            show_setup_wizard: false,
            setup_wizard_step: 0,
            setup_install_log: String::new(),
            settings_row: 0,
            settings_scroll: 0,
            settings_search: String::new(),
            settings_search_active: false,
            zoomed: None,
            summary: Summary::default(),
            last_input: Instant::now(),
            last_click: None,
            alert_latch: Default::default(),
            night: None,
            toast: None,
            pending_signal: None,
            sampler_interval,
            custom_widgets,
            image_picker: Some(detect_image_picker()),
            image_protocols: std::collections::HashMap::new(),
            image_resize_tx: None,
            image_resize_rx: None,
            thread_protocols: std::collections::HashMap::new(),
        };

        // Spin up the persistent background thread for non-blocking image resize+encode.
        {
            let (tx_worker, rx_worker) =
                std::sync::mpsc::channel::<ratatui_image::thread::ResizeRequest>();
            let (tx_done, rx_done) = std::sync::mpsc::channel();
            std::thread::Builder::new()
                .name("vanta-image-encoder".into())
                .spawn(move || {
                    while let Ok(req) = rx_worker.recv() {
                        let _ = tx_done.send(req.resize_encode());
                    }
                })
                .ok();
            app.image_resize_tx = Some(tx_worker);
            app.image_resize_rx = Some(rx_done);
        }
        if mode.clone() == DashboardMode::Monitor {
            app.focused_panel = Some(PanelId::Processes);
        }
        app
    }

    fn toast(&mut self, msg: impl Into<String>) {
        self.toast = Some((msg.into(), Instant::now()));
    }

    pub fn set_mode(&mut self, mode: DashboardMode) {
        if self.mode == mode {
            return;
        }
        self.mode = mode.clone();
        self.zoomed = None;
        self.focused_panel = (mode.clone() == DashboardMode::Monitor).then_some(PanelId::Processes);
        self.config.ui.startup_mode = mode.as_str().to_string();
        self.config.save();
    }

    pub fn available_modes(&self) -> Vec<DashboardMode> {
        let mut modes = vec![
            DashboardMode::Dashboard,
            DashboardMode::Monitor,
            DashboardMode::Aesthetic,
            DashboardMode::Workspace,
        ];
        for ext in &self.ext_manager.extensions {
            for page in ext.pages() {
                modes.push(DashboardMode::Extension(page.title().to_string()));
            }
        }
        for page in &self.config.pages {
            let has_valid_widgets = page.layout.iter().flatten().any(|id| {
                PanelId::from_name(id, &self.config).is_some()
                    || self
                        .config
                        .custom_widgets
                        .iter()
                        .any(|c| c.id.eq_ignore_ascii_case(id))
                    || self.ext_manager.extensions.iter().any(|e| {
                        e.components()
                            .iter()
                            .any(|c| c.id().eq_ignore_ascii_case(id))
                            || e.metadata().id.eq_ignore_ascii_case(id)
                    })
            });
            if has_valid_widgets {
                modes.push(DashboardMode::Extension(page.name.clone()));
            }
        }
        modes
    }

    pub fn cycle_theme_back(&mut self) {
        self.set_theme(Theme::prev_name(&self.config.ui.theme));
    }

    pub fn cycle_theme(&mut self) {
        self.set_theme(Theme::next_name(&self.config.ui.theme));
    }

    pub fn cycle_style(&mut self, forward: bool) {
        use crate::config::DesignStyle;
        let all = DesignStyle::ALL;
        let i = all
            .iter()
            .position(|s| *s == self.config.ui.style)
            .unwrap_or(0);
        let n = all.len();
        self.config.ui.style = all[if forward {
            (i + 1) % n
        } else {
            (i + n - 1) % n
        }];
        self.config.save();
        self.toast(format!("style · {}", self.config.ui.style.label()));
    }

    /// Name of the current page's layout, for the menu.
    pub fn layout_name(&self) -> String {
        match self.mode {
            DashboardMode::Dashboard => self.config.dashboard.preset.clone(),
            DashboardMode::Monitor => self.config.ui.monitor_layout.clone(),
            DashboardMode::Workspace => self.config.ui.focus_layout.clone(),
            _ => "scenes".to_string(),
        }
    }

    /// Step the current page to its next/previous layout.
    pub fn cycle_layout(&mut self, forward: bool) {
        const OVERVIEW: [&str; 5] = ["cockpit", "monitoring", "minimal", "aesthetic", "workspace"];
        let (list, cur): (&[&str], String) = match self.mode {
            DashboardMode::Dashboard => (&OVERVIEW, self.config.dashboard.preset.clone()),
            DashboardMode::Monitor => (
                &screens::monitor::LAYOUTS,
                self.config.ui.monitor_layout.clone(),
            ),
            DashboardMode::Workspace => (
                &screens::workspace::LAYOUTS,
                self.config.ui.focus_layout.clone(),
            ),
            _ => return,
        };
        let n = list.len();
        let i = list.iter().position(|p| *p == cur).unwrap_or(0);
        let next = list[if forward {
            (i + 1) % n
        } else {
            (i + n - 1) % n
        }]
        .to_string();
        match self.mode {
            DashboardMode::Dashboard => self.config.dashboard.apply_preset(&next),
            DashboardMode::Monitor => self.config.ui.monitor_layout = next.clone(),
            _ => self.config.ui.focus_layout = next.clone(),
        }
        self.config.save();
        self.toast(format!("layout · {}", next));
    }

    fn set_theme(&mut self, next: String) {
        self.config.ui.theme = next.to_string();
        self.theme = Theme::from_name(&next);
        self.night = None;
        self.config.save();
        self.toast(format!("theme · {}", next));
    }

    fn cycle_focus(&mut self, forward: bool) {
        let panels = PanelId::for_mode(&self.mode, &self.config);
        if panels.is_empty() {
            return;
        }
        let idx = self
            .focused_panel
            .and_then(|p| panels.iter().position(|&x| x == p));
        let next = match idx {
            Some(i) if forward => (i + 1) % panels.len(),
            Some(i) => (i + panels.len() - 1) % panels.len(),
            None if forward => 0,
            None => panels.len() - 1,
        };
        self.focused_panel = Some(panels[next]);
        if self.zoomed.is_some() {
            self.zoomed = self.focused_panel;
        }
    }

    pub fn adjust_refresh(&mut self, faster: bool) {
        let cur = self.config.ui.refresh_rate;
        let next = if faster { cur / 2.0 } else { cur * 2.0 }.clamp(0.1, 10.0);
        self.config.ui.refresh_rate = next;
        self.sampler_interval
            .store((next * 1000.0) as u64, std::sync::atomic::Ordering::Relaxed);
        self.config.save();
        self.toast(format!("refresh · {:.2}s", next));
    }

    // ── Input ─────────────────────────────────────────────────

    /// Any inline text field is taking keystrokes.
    fn typing(&self) -> bool {
        let ps = &self.panel_states;
        ps.agenda_input_active
            || ps.task_input_active
            || ps.pinned_media_input_active
            || ps.log_target_input_active
            || ps.files_rename_input_active
            || ps.files_search_input_active
            || ps.files_mkdir_input_active
            || ps.process_search_active
    }

    fn press(&mut self, code: KeyCode) {
        self.handle_key(KeyEvent::new(code, KeyModifiers::NONE));
    }

    /// Mouse: click a tab to switch page, a panel to focus it (double-click
    /// zooms), a row to select it (double-click opens), a column header to
    /// sort; the wheel scrolls whatever list is under the pointer. Returns
    /// whether anything changed, so pointer noise doesn't force redraws.
    pub fn handle_mouse(&mut self, ev: crossterm::event::MouseEvent) -> bool {
        use crate::screens::Hit;
        use crossterm::event::{MouseButton, MouseEventKind as K};
        if self.show_setup_wizard || self.typing() {
            return false;
        }
        let wheel = match ev.kind {
            K::ScrollDown => Some(true),
            K::ScrollUp => Some(false),
            _ => None,
        };
        let click = matches!(ev.kind, K::Down(MouseButton::Left));
        if wheel.is_none() && !click {
            return false;
        }
        self.last_input = Instant::now();

        // Overlays swallow the mouse: the wheel scrolls, a click closes.
        if self.show_help {
            match wheel {
                Some(true) => self.help_scroll += 3,
                Some(false) => self.help_scroll = self.help_scroll.saturating_sub(3),
                None => {
                    self.show_help = false;
                    self.help_scroll = 0;
                }
            }
            return true;
        }
        if self.show_menu {
            let n = crate::screens::menu::ITEMS.len();
            if let Some(down) = wheel {
                self.menu_row = if down {
                    (self.menu_row + 1) % n
                } else {
                    (self.menu_row + n - 1) % n
                };
                return true;
            }
            match crate::screens::hits_at(ev.column, ev.row)
                .first()
                .map(|(h, _)| *h)
            {
                Some(crate::screens::Hit::MenuRow(i)) => {
                    self.menu_row = i;
                    crate::screens::menu::activate(self);
                }
                Some(crate::screens::Hit::Overlay) => {}
                _ => self.show_menu = false,
            }
            return true;
        }
        if self.show_settings {
            if let Some(down) = wheel {
                self.press(if down { KeyCode::Down } else { KeyCode::Up });
                return true;
            }
            let hits = crate::screens::hits_at(ev.column, ev.row);
            match hits.first().map(|(h, _)| *h) {
                // First click selects, a click on the selected row changes it.
                Some(crate::screens::Hit::SettingRow(i)) => {
                    if self.settings_row == i {
                        crate::screens::settings::change_setting(self, true);
                    } else {
                        self.settings_row = i;
                    }
                }
                Some(crate::screens::Hit::Overlay) => {}
                _ => self.show_settings = false,
            }
            return true;
        }

        let double = click
            && self.last_click.is_some_and(|(t, x, y)| {
                t.elapsed() < Duration::from_millis(400) && x == ev.column && y == ev.row
            });
        if click {
            self.last_click = (!double).then(|| (Instant::now(), ev.column, ev.row));
        }

        let hits = crate::screens::hits_at(ev.column, ev.row);
        if let Some(down) = wheel {
            for (h, _) in hits {
                match h {
                    Hit::NotePreview => {
                        let ns = &mut self.panel_states.note_scroll;
                        *ns = if down {
                            ns.saturating_add(3)
                        } else {
                            ns.saturating_sub(3)
                        };
                        return true;
                    }
                    Hit::Scene => {
                        self.press(if down { KeyCode::Right } else { KeyCode::Left });
                        return true;
                    }
                    Hit::Rows { panel, .. } | Hit::Panel(panel) => {
                        let key = match (panel, down) {
                            (PanelId::Calendar, true) => KeyCode::Right,
                            (PanelId::Calendar, false) => KeyCode::Left,
                            (
                                PanelId::Processes
                                | PanelId::Files
                                | PanelId::WriterNotes
                                | PanelId::Tasks
                                | PanelId::Agenda,
                                true,
                            ) => KeyCode::Down,
                            (
                                PanelId::Processes
                                | PanelId::Files
                                | PanelId::WriterNotes
                                | PanelId::Tasks
                                | PanelId::Agenda,
                                false,
                            ) => KeyCode::Up,
                            _ => return false,
                        };
                        self.focused_panel = Some(panel);
                        self.press(key);
                        return true;
                    }
                    Hit::Tab(_)
                    | Hit::SortBy(_)
                    | Hit::SettingRow(_)
                    | Hit::Overlay
                    | Hit::MenuRow(_) => {}
                }
            }
            return false;
        }

        for (h, rect) in hits {
            match h {
                Hit::Tab(i) => {
                    if let Some(m) = self.available_modes().get(i).cloned() {
                        self.set_mode(m);
                    }
                    return true;
                }
                Hit::SortBy(field) => {
                    let ps = &mut self.panel_states;
                    if ps.process_sort_field == field {
                        ps.process_sort_asc = !ps.process_sort_asc;
                    } else {
                        ps.process_sort_field = field;
                        // Numbers read best biggest-first, names A→Z.
                        ps.process_sort_asc = matches!(field, SortField::Name | SortField::Pid);
                    }
                    self.focused_panel = Some(PanelId::Processes);
                    self.sync_selected_pid();
                    return true;
                }
                Hit::Rows { panel, first } => {
                    let idx = first + (ev.row - rect.y) as usize;
                    self.focused_panel = Some(panel);
                    let ps = &mut self.panel_states;
                    match panel {
                        PanelId::Processes => {
                            ps.process_scroll_offset = idx;
                            self.sync_selected_pid();
                        }
                        PanelId::Files => {
                            ps.files_selected = idx;
                            if double {
                                self.press(KeyCode::Enter);
                            }
                        }
                        PanelId::WriterNotes => {
                            if ps.writer_selected != idx {
                                ps.note_scroll = 0;
                            }
                            ps.writer_selected = idx;
                            if double {
                                self.trigger_focused_action();
                            }
                        }
                        PanelId::Tasks => ps.tasks_selected = idx,
                        _ => {}
                    }
                    return true;
                }
                Hit::Scene => {
                    // Left third steps back, anywhere else forward.
                    let back = ev.column < rect.x + rect.width / 3;
                    self.press(if back { KeyCode::Left } else { KeyCode::Right });
                    return true;
                }
                Hit::NotePreview | Hit::SettingRow(_) | Hit::Overlay | Hit::MenuRow(_) => {}
                Hit::Panel(panel) => {
                    if double {
                        self.focused_panel = Some(panel);
                        self.zoomed = if self.zoomed.is_some() {
                            None
                        } else {
                            Some(panel)
                        };
                    } else if self.focused_panel != Some(panel) {
                        self.focused_panel = Some(panel);
                    }
                    return true;
                }
            }
        }
        false
    }

    pub fn handle_key(&mut self, key: KeyEvent) {
        self.last_input = Instant::now();
        if matches!(self.mode, DashboardMode::Extension(_)) {
            for ext in &self.ext_manager.extensions {
                for mut comp in ext.components() {
                    if comp.handle_key(key) {
                        return;
                    }
                }
            }
        }

        let ps = &mut self.panel_states;

        // Text entry captures everything first.
        if ps.agenda_input_active {
            match key.code {
                KeyCode::Esc => {
                    ps.agenda_input_active = false;
                    ps.agenda_input.clear();
                    ps.agenda_edit = None;
                }
                KeyCode::Enter if ps.agenda_edit.is_some() => {
                    if let Some(i) = ps.agenda_edit.take() {
                        crate::monitors::agenda::edit_event(i, &ps.agenda_input);
                    }
                    ps.agenda_input_active = false;
                    ps.agenda_input.clear();
                }
                KeyCode::Enter => {
                    if !ps.agenda_input.trim().is_empty() {
                        crate::monitors::agenda::add_event(&ps.agenda_input);
                    }
                    // Do not close input_active so user can add multiple items in a row
                    ps.agenda_input.clear();
                }
                KeyCode::Backspace => {
                    ps.agenda_input.pop();
                }
                KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                    ps.agenda_input.push(c);
                }
                _ => {}
            }
            return;
        }

        if ps.pinned_media_input_active {
            match key.code {
                KeyCode::Esc => {
                    ps.pinned_media_input_active = false;
                    ps.pinned_media_input.clear();
                }
                KeyCode::Enter => {
                    if !ps.pinned_media_input.trim().is_empty() {
                        self.config.ui.pinned_media_path = ps.pinned_media_input.trim().to_string();
                        self.config.save();
                    }
                    ps.pinned_media_input_active = false;
                    ps.pinned_media_input.clear();
                }
                KeyCode::Backspace => {
                    ps.pinned_media_input.pop();
                }
                KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                    ps.pinned_media_input.push(c);
                }
                _ => {}
            }
            return;
        }
        if ps.log_target_input_active {
            match key.code {
                KeyCode::Esc | KeyCode::Enter => {
                    ps.log_target_input_active = false;
                }
                KeyCode::Backspace => {
                    ps.log_target.pop();
                }
                KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                    ps.log_target.push(c);
                }
                _ => {}
            }
            return;
        }

        if ps.task_input_active {
            match key.code {
                KeyCode::Esc => {
                    ps.task_input_active = false;
                    ps.task_input.clear();
                }
                KeyCode::Enter => {
                    if !ps.task_input.trim().is_empty() {
                        crate::monitors::tasks::add_task(&ps.task_input);
                    }
                    // Do not close input_active so user can add multiple items in a row
                    ps.task_input.clear();
                }
                KeyCode::Backspace => {
                    ps.task_input.pop();
                }
                KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                    ps.task_input.push(c);
                }
                _ => {}
            }
            return;
        }

        if ps.files_rename_input_active {
            match key.code {
                KeyCode::Esc => {
                    ps.files_rename_input_active = false;
                    ps.files_rename_input.clear();
                }
                KeyCode::Enter => {
                    if !ps.files_rename_input.trim().is_empty() {
                        let snap = crate::monitors::files::snapshot();
                        if let Some(item) = snap.items.get(ps.files_selected) {
                            let mut new_path = item.path.clone();
                            new_path.set_file_name(&ps.files_rename_input);
                            let _ = std::fs::rename(&item.path, &new_path);
                            let current = snap.current_dir.clone();
                            std::thread::spawn(move || {
                                std::thread::sleep(std::time::Duration::from_millis(100));
                                crate::monitors::files::chdir(&current);
                            });
                        }
                    }
                    ps.files_rename_input_active = false;
                    ps.files_rename_input.clear();
                }
                KeyCode::Backspace => {
                    ps.files_rename_input.pop();
                }
                KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                    ps.files_rename_input.push(c);
                }
                _ => {}
            }
            return;
        }

        if ps.files_search_input_active {
            match key.code {
                KeyCode::Esc | KeyCode::Enter => {
                    ps.files_search_input_active = false;
                    if key.code == KeyCode::Esc {
                        ps.files_search_input.clear();
                    }
                }
                KeyCode::Backspace => {
                    ps.files_search_input.pop();
                }
                KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                    ps.files_search_input.push(c);
                }
                _ => {}
            }
            return;
        }

        if ps.process_search_active {
            match key.code {
                KeyCode::Esc => {
                    ps.process_search_active = false;
                    ps.process_search.clear();
                    ps.process_scroll_offset = 0;
                }
                KeyCode::Enter => ps.process_search_active = false,
                KeyCode::Backspace => {
                    ps.process_search.pop();
                    ps.process_scroll_offset = 0;
                }
                KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                    ps.process_search.push(c);
                    ps.process_scroll_offset = 0;
                }
                KeyCode::Up | KeyCode::Down | KeyCode::PageUp | KeyCode::PageDown => {
                    self.process_nav(key.code);
                    return;
                }
                _ => {}
            }
            self.sync_selected_pid();
            return;
        }

        if self.show_setup_wizard {
            crate::screens::setup_wizard::handle_key(self, key.code);
            return;
        }

        if self.show_settings {
            crate::screens::settings::handle_key(self, key.code);
            return;
        }

        if self.show_menu {
            crate::screens::menu::handle_key(self, key.code);
            return;
        }

        if self.show_help {
            match key.code {
                KeyCode::Char('e') | KeyCode::Char('E')
                    if self.focused_panel != Some(PanelId::PinnedMedia) =>
                {
                    self.trigger_focused_action()
                }
                KeyCode::Char('q') | KeyCode::Char('Q') => self.running = false,
                KeyCode::Char('T') => self.cycle_theme(),
                KeyCode::Up | KeyCode::Char('k') => {
                    self.help_scroll = self.help_scroll.saturating_sub(1)
                }
                KeyCode::Down | KeyCode::Char('j') => self.help_scroll += 1,
                KeyCode::PageUp => self.help_scroll = self.help_scroll.saturating_sub(10),
                KeyCode::PageDown => self.help_scroll += 10,
                _ => {
                    self.show_help = false;
                    self.help_scroll = 0;
                }
            }
            return;
        }

        if self.mode == DashboardMode::DebugLogs {
            match key.code {
                KeyCode::Char('t') => {
                    self.panel_states.log_target_input_active = true;
                    self.panel_states.log_target.clear();
                }
                KeyCode::Char('c') => {
                    let mut logs = crate::logger::LOGS.write().unwrap();
                    logs.clear();
                }
                KeyCode::Char('s') => {
                    crate::screens::debug_logs::save_logs(self);
                }
                KeyCode::F(12) | KeyCode::Char('~') => {
                    self.set_mode(DashboardMode::Dashboard);
                }
                _ => {}
            }
            return;
        }

        // Ambient scenes: ←/→ (h/l) step, r pauses/resumes rotation.
        if self.mode == DashboardMode::Aesthetic && self.zoomed.is_none() {
            let n = crate::screens::aesthetic::scenes(self).len();
            let secs = self.config.ui.ambient_rotate_secs;
            match key.code {
                KeyCode::Left | KeyCode::Char('h') => return self.ambient.step(-1, secs, n),
                KeyCode::Right | KeyCode::Char('l') => return self.ambient.step(1, secs, n),
                KeyCode::Char('i') => {
                    self.panel_states.pinned_media_input = self.config.ui.pinned_media_path.clone();
                    self.panel_states.pinned_media_input_active = true;
                    return;
                }
                KeyCode::Char('r') => {
                    self.ambient.toggle_auto(secs, n);
                    let state = if self.ambient.auto { "on" } else { "paused" };
                    return self.toast(format!("scene rotation · {}", state));
                }
                _ => {}
            }
        }

        // Focus timer controls while its panel is focused.
        if self.focused_panel == Some(PanelId::Timer) && key.modifiers.is_empty() {
            use crate::widgets::pomodoro;
            match key.code {
                KeyCode::Char(' ') | KeyCode::Enter => return pomodoro::toggle(&self.config.ui),
                KeyCode::Char('r') => return pomodoro::reset(&self.config.ui),
                KeyCode::Char('s') => return pomodoro::skip(&self.config.ui),
                _ => {}
            }
        }

        match key.code {
            KeyCode::Left | KeyCode::Char('h') | KeyCode::Char('H')
                if key.modifiers.intersects(
                    KeyModifiers::CONTROL | KeyModifiers::ALT | KeyModifiers::SHIFT,
                ) =>
            {
                self.resize_focused(-2)
            }
            KeyCode::Right | KeyCode::Char('l') | KeyCode::Char('L')
                if key.modifiers.intersects(
                    KeyModifiers::CONTROL | KeyModifiers::ALT | KeyModifiers::SHIFT,
                ) =>
            {
                self.resize_focused(2)
            }
            KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('K')
                if key.modifiers.intersects(
                    KeyModifiers::CONTROL | KeyModifiers::ALT | KeyModifiers::SHIFT,
                ) =>
            {
                self.resize_focused_vertical(2)
            }
            KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('J')
                if key.modifiers.intersects(
                    KeyModifiers::CONTROL | KeyModifiers::ALT | KeyModifiers::SHIFT,
                ) =>
            {
                self.resize_focused_vertical(-2)
            }
            KeyCode::Char('e') | KeyCode::Char('E')
                if self.focused_panel != Some(PanelId::PinnedMedia) =>
            {
                self.trigger_focused_action()
            }
            KeyCode::Char('q') | KeyCode::Char('Q') => self.running = false,
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.running = false
            }
            KeyCode::Char('?') | KeyCode::F(1) => self.show_help = true,
            KeyCode::Char('S') | KeyCode::Char(',') => self.show_settings = true,
            KeyCode::F(12) | KeyCode::Char('~') => {
                if self.mode == DashboardMode::DebugLogs {
                    self.set_mode(DashboardMode::Dashboard);
                } else {
                    if let Some(panel) = self.focused_panel {
                        self.panel_states.log_target = format!("{:?}", panel).to_lowercase();
                    } else {
                        self.panel_states.log_target.clear();
                    }
                    self.set_mode(DashboardMode::DebugLogs);
                }
            }
            KeyCode::Char(c) if c.is_ascii_digit() => {
                let n = c.to_digit(10).unwrap() as usize;
                let modes = self.available_modes();
                if n > 0 && n <= modes.len() {
                    self.set_mode(modes[n - 1].clone());
                }
            }
            KeyCode::Char('T') => self.cycle_theme(),
            KeyCode::Char('v') | KeyCode::Char('V') => {
                music_viz::cycle_style();
                self.config.ui.visualizer = music_viz::style_name().to_string();
                self.config.save();
                self.toast(format!("visualizer · {}", music_viz::style_name()));
            }
            KeyCode::Char('g') => {
                crate::widgets::gauge::cycle_style();
                self.config.ui.gauge_style = crate::widgets::gauge::style_name().to_string();
                self.config.save();
                self.toast(format!("gauges · {}", crate::widgets::gauge::style_name()));
            }
            KeyCode::Char('G') => {
                crate::widgets::block_graph::cycle_style();
                self.config.ui.graph_style = crate::widgets::block_graph::style_name().to_string();
                self.config.save();
                self.toast(format!(
                    "graphs · {}",
                    crate::widgets::block_graph::style_name()
                ));
            }
            KeyCode::Char('m') => {
                crate::widgets::meter::cycle_style();
                self.config.ui.meter_style = crate::widgets::meter::style_name().to_string();
                self.config.save();
                self.toast(format!("meters · {}", crate::widgets::meter::style_name()));
            }
            KeyCode::Char('o') => {
                self.config.ui.motion_enabled = !self.config.ui.motion_enabled;
                self.config.save();
                let status = if self.config.ui.motion_enabled {
                    "resumed"
                } else {
                    "paused"
                };
                self.toast(format!("motion · {}", status));
            }
            KeyCode::Char('I') => {
                self.panel_states.pixel_images = !self.panel_states.pixel_images;
                let status = if self.panel_states.pixel_images {
                    "Pixelated (Half-block)"
                } else {
                    "Clear (High-res Protocol)"
                };
                self.toast(format!("images · {}", status));
            }
            KeyCode::Char('B') => {
                self.panel_states.force_robot_logo = !self.panel_states.force_robot_logo;
                let status = if self.panel_states.force_robot_logo {
                    "Vanta Robot"
                } else {
                    "OS Logo"
                };
                self.toast(format!("logo · {}", status));
            }
            KeyCode::Char('O') => {
                let modes = ["spin", "tumble", "wobble", "swing"];
                let pos = modes
                    .iter()
                    .position(|&x| x == self.config.ui.motion_mode)
                    .unwrap_or(0);
                let next = modes[(pos + 1) % modes.len()];
                self.config.ui.motion_mode = next.to_string();
                self.config.save();
                self.toast(format!("motion mode · {}", next));
            }
            KeyCode::Char('[') => {
                let speeds = [0.25, 0.5, 0.75, 1.0, 1.25, 1.5, 2.0, 3.0];
                let current = self.config.ui.motion_speed;
                let next = speeds
                    .iter()
                    .copied()
                    .rev()
                    .find(|&s| s < current - 0.05)
                    .unwrap_or(0.25);
                self.config.ui.motion_speed = next;
                self.config.save();
                self.toast(format!("motion speed · {:.2}x", next));
            }
            KeyCode::Char(']') => {
                let speeds = [0.25, 0.5, 0.75, 1.0, 1.25, 1.5, 2.0, 3.0];
                let current = self.config.ui.motion_speed;
                let next = speeds
                    .iter()
                    .copied()
                    .find(|&s| s > current + 0.05)
                    .unwrap_or(3.0);
                self.config.ui.motion_speed = next;
                self.config.save();
                self.toast(format!("motion speed · {:.2}x", next));
            }
            KeyCode::Char('+') | KeyCode::Char('=') => self.adjust_refresh(true),
            KeyCode::Char('-') | KeyCode::Char('_') => self.adjust_refresh(false),
            KeyCode::Tab => self.cycle_focus(true),
            KeyCode::BackTab => self.cycle_focus(false),
            KeyCode::Enter => {
                match self.focused_panel {
                    Some(PanelId::WriterNotes) | Some(PanelId::Tasks) | Some(PanelId::Agenda) => {
                        self.trigger_focused_action();
                        return;
                    }
                    _ => {}
                }
                match (self.zoomed, self.focused_panel) {
                    (Some(_), _) => self.zoomed = None,
                    (None, Some(p)) => self.zoomed = Some(p),
                    (None, None) => {}
                }
            }
            // Esc backs out one level: unzoom, then unfocus, then the menu.
            KeyCode::Esc => {
                if self.zoomed.is_some() {
                    self.zoomed = None;
                } else if self.focused_panel.is_some() {
                    self.focused_panel = None;
                } else {
                    self.show_menu = true;
                    self.menu_row = 0;
                }
            }

            // Media transport is global: it's the whole point of a dashboard.
            KeyCode::Char(' ') if self.focused_panel != Some(PanelId::Tasks) => {
                media::control(Action::PlayPause);
            }
            KeyCode::Char('n') | KeyCode::Char('N') => media::control(Action::Next),
            KeyCode::Char('p') | KeyCode::Char('P') => media::control(Action::Previous),
            KeyCode::Char('>') | KeyCode::Char('.') => media::control(Action::VolumeUp),
            KeyCode::Char('<') => media::control(Action::VolumeDown),

            _ => self.handle_panel_key(key.code),
        }
    }

    fn handle_panel_key(&mut self, key: KeyCode) {
        // Arrow keys with nothing focused grab the page's primary panel.
        if self.focused_panel.is_none()
            && matches!(
                key,
                KeyCode::Up
                    | KeyCode::Down
                    | KeyCode::Left
                    | KeyCode::Right
                    | KeyCode::PageUp
                    | KeyCode::PageDown
                    | KeyCode::Home
                    | KeyCode::End
            )
        {
            let panels = PanelId::for_mode(&self.mode, &self.config);
            self.focused_panel = if panels.contains(&PanelId::Processes) {
                Some(PanelId::Processes)
            } else {
                panels.first().copied()
            };
        }
        // Process hotkeys work from anywhere on the Monitor page.
        let process_hotkey = matches!(
            key,
            KeyCode::Char('/')
                | KeyCode::Char('s')
                | KeyCode::Char('r')
                | KeyCode::Char('t')
                | KeyCode::Char('c')
                | KeyCode::Char('k')
                | KeyCode::Char('K')
        );
        match self.focused_panel {
            Some(PanelId::PinnedMedia) => match key {
                KeyCode::Char('e') | KeyCode::Char('i') | KeyCode::Char('/') => {
                    self.panel_states.pinned_media_input_active = true;
                    self.panel_states.pinned_media_input.clear();
                }
                _ => {}
            },
            Some(PanelId::Tasks) => match key {
                KeyCode::Up | KeyCode::Char('k') => {
                    self.panel_states.tasks_selected =
                        self.panel_states.tasks_selected.saturating_sub(1);
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    let count = crate::monitors::tasks::snapshot().tasks.len();
                    if count > 0 {
                        self.panel_states.tasks_selected =
                            (self.panel_states.tasks_selected + 1).min(count - 1);
                    }
                }
                KeyCode::Char(' ') | KeyCode::Char('x') => {
                    crate::monitors::tasks::toggle_task(self.panel_states.tasks_selected);
                }
                KeyCode::Char('a') | KeyCode::Char('n') => {
                    self.panel_states.task_input_active = true;
                    self.panel_states.task_input.clear();
                }
                KeyCode::Char('d') | KeyCode::Delete => {
                    let count = crate::monitors::tasks::snapshot().tasks.len();
                    if count > 0 {
                        crate::monitors::tasks::delete_task(self.panel_states.tasks_selected);
                        if self.panel_states.tasks_selected >= count - 1
                            && self.panel_states.tasks_selected > 0
                        {
                            self.panel_states.tasks_selected -= 1;
                        }
                    }
                }
                KeyCode::Enter | KeyCode::Char('e') | KeyCode::Char('E') => {
                    self.trigger_focused_action();
                }
                _ => {}
            },
            Some(PanelId::Agenda) => match key {
                KeyCode::Up | KeyCode::Char('k') => {
                    self.panel_states.agenda_selected =
                        self.panel_states.agenda_selected.saturating_sub(1);
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    let count = crate::monitors::agenda::snapshot().events.len();
                    if count > 0 {
                        self.panel_states.agenda_selected =
                            (self.panel_states.agenda_selected + 1).min(count - 1);
                    }
                }
                KeyCode::Char('a') | KeyCode::Char('n') => {
                    self.panel_states.agenda_input_active = true;
                    self.panel_states.agenda_input.clear();
                    self.panel_states.agenda_edit = None;
                }
                KeyCode::Char('r') | KeyCode::F(2) => {
                    let snap = crate::monitors::agenda::snapshot();
                    let i = self.panel_states.agenda_selected;
                    if let Some(ev) = snap.events.get(i) {
                        self.panel_states.agenda_input =
                            crate::monitors::agenda::edit_input_for(ev);
                        self.panel_states.agenda_input_active = true;
                        self.panel_states.agenda_edit = Some(i);
                    }
                }
                KeyCode::Char('d') | KeyCode::Delete => {
                    let count = crate::monitors::agenda::snapshot().events.len();
                    if count > 0 {
                        crate::monitors::agenda::delete_event(self.panel_states.agenda_selected);
                        if self.panel_states.agenda_selected >= count - 1
                            && self.panel_states.agenda_selected > 0
                        {
                            self.panel_states.agenda_selected -= 1;
                        }
                    }
                }
                KeyCode::Enter | KeyCode::Char('e') | KeyCode::Char('E') => {
                    self.trigger_focused_action();
                }
                _ => {}
            },
            Some(PanelId::WriterNotes) => match key {
                KeyCode::Up | KeyCode::Char('k') => {
                    self.panel_states.writer_selected =
                        self.panel_states.writer_selected.saturating_sub(1);
                    self.panel_states.note_scroll = 0;
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    let count = crate::monitors::obsidian::snapshot().notes.len();
                    if count > 0 {
                        self.panel_states.writer_selected =
                            (self.panel_states.writer_selected + 1).min(count - 1);
                    }
                    self.panel_states.note_scroll = 0;
                }
                // The preview is clamped to the note's length when drawn.
                KeyCode::PageDown => {
                    self.panel_states.note_scroll = self.panel_states.note_scroll.saturating_add(10)
                }
                KeyCode::PageUp => {
                    self.panel_states.note_scroll = self.panel_states.note_scroll.saturating_sub(10)
                }
                KeyCode::Home => self.panel_states.note_scroll = 0,
                KeyCode::Enter | KeyCode::Char('e') | KeyCode::Char('E') => {
                    self.trigger_focused_action();
                }
                _ => {}
            },
            Some(PanelId::Files) => {
                if self.panel_states.files_rename_input_active {
                    match key {
                        KeyCode::Enter => {
                            self.panel_states.files_rename_input_active = false;
                            let snap = crate::monitors::files::snapshot();
                            let items = self.get_filtered_files(&snap);
                            if let Some(item) = items.get(self.panel_states.files_selected) {
                                let new_path = item
                                    .path
                                    .parent()
                                    .unwrap()
                                    .join(&self.panel_states.files_rename_input);
                                std::fs::rename(&item.path, &new_path).ok();
                                crate::monitors::files::chdir(&snap.current_dir);
                            }
                        }
                        KeyCode::Esc => self.panel_states.files_rename_input_active = false,
                        KeyCode::Backspace => {
                            self.panel_states.files_rename_input.pop();
                        }
                        KeyCode::Char(c) => self.panel_states.files_rename_input.push(c),
                        _ => {}
                    }
                    return;
                }
                if self.panel_states.files_search_input_active {
                    match key {
                        KeyCode::Enter | KeyCode::Esc => {
                            self.panel_states.files_search_input_active = false
                        }
                        KeyCode::Backspace => {
                            self.panel_states.files_search_input.pop();
                            self.panel_states.files_selected = 0;
                        }
                        KeyCode::Char(c) => {
                            self.panel_states.files_search_input.push(c);
                            self.panel_states.files_selected = 0;
                        }
                        _ => {}
                    }
                    return;
                }
                if self.panel_states.files_mkdir_input_active {
                    match key {
                        KeyCode::Enter => {
                            let name = self.panel_states.files_mkdir_input.trim().to_string();
                            self.panel_states.files_mkdir_input_active = false;
                            if !name.is_empty() {
                                let snap = crate::monitors::files::snapshot();
                                let new_dir = snap.current_dir.join(&name);
                                if std::fs::create_dir_all(&new_dir).is_ok() {
                                    crate::monitors::files::chdir(&snap.current_dir);
                                    self.toast(format!("created: {}", name));
                                }
                            }
                            self.panel_states.files_mkdir_input.clear();
                        }
                        KeyCode::Esc => {
                            self.panel_states.files_mkdir_input_active = false;
                            self.panel_states.files_mkdir_input.clear();
                        }
                        KeyCode::Backspace => {
                            self.panel_states.files_mkdir_input.pop();
                        }
                        KeyCode::Char(c) => self.panel_states.files_mkdir_input.push(c),
                        _ => {}
                    }
                    return;
                }
                match key {
                    KeyCode::Char('.') => {
                        self.panel_states.files_show_hidden = !self.panel_states.files_show_hidden;
                        self.panel_states.files_selected = 0;
                    }
                    KeyCode::Char('/') => {
                        self.panel_states.files_search_input_active = true;
                    }
                    KeyCode::Char('m') | KeyCode::F(7) => {
                        self.panel_states.files_mkdir_input_active = true;
                        self.panel_states.files_mkdir_input.clear();
                    }

                    KeyCode::Char('N') | KeyCode::F(2) => {
                        self.panel_states.files_rename_input_active = true;
                        self.panel_states.files_rename_input.clear();
                        let snap = crate::monitors::files::snapshot();
                        let items = self.get_filtered_files(&snap);
                        if let Some(item) = items.get(self.panel_states.files_selected) {
                            self.panel_states.files_rename_input = item
                                .path
                                .file_name()
                                .unwrap_or_default()
                                .to_string_lossy()
                                .to_string();
                        }
                    }
                    KeyCode::Char('y') | KeyCode::Char('c') => {
                        let snap = crate::monitors::files::snapshot();
                        let items = self.get_filtered_files(&snap);
                        if let Some(item) = items.get(self.panel_states.files_selected) {
                            let path = item.path.to_string_lossy().to_string();
                            std::thread::spawn(move || {
                                let _ = std::process::Command::new("sh")
                                .arg("-c")
                                .arg(format!("echo -n '{}' | xclip -sel clip || echo -n '{}' | wl-copy || echo -n '{}' | pbcopy || echo -n '{}' | clip.exe", path, path, path, path))
                                .output();
                            });
                            self.toast(format!(
                                "copied path: {}",
                                item.path.file_name().unwrap_or_default().to_string_lossy()
                            ));
                        }
                    }
                    KeyCode::Char('o') => {
                        let snap = crate::monitors::files::snapshot();
                        let items = self.get_filtered_files(&snap);
                        if let Some(item) = items.get(self.panel_states.files_selected) {
                            let path = item.path.clone();
                            std::thread::spawn(move || {
                                #[cfg(target_os = "macos")]
                                let _ = std::process::Command::new("open").arg(&path).spawn();
                                #[cfg(target_os = "windows")]
                                let _ = std::process::Command::new("cmd")
                                    .args(["/c", "start", "", path.to_str().unwrap()])
                                    .spawn();
                                #[cfg(target_os = "linux")]
                                let _ = std::process::Command::new("xdg-open").arg(&path).spawn();
                            });
                        }
                    }
                    KeyCode::Up | KeyCode::Char('k') => {
                        self.panel_states.files_selected =
                            self.panel_states.files_selected.saturating_sub(1);
                        let snap = crate::monitors::files::snapshot();
                        let items = self.get_filtered_files(&snap);
                        if let Some(item) = items.get(self.panel_states.files_selected) {
                            crate::monitors::files::update_preview(&item.path);
                        }
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        self.panel_states.files_selected =
                            self.panel_states.files_selected.saturating_add(1);
                        let snap = crate::monitors::files::snapshot();
                        let items = self.get_filtered_files(&snap);
                        if let Some(item) = items.get(self.panel_states.files_selected) {
                            crate::monitors::files::update_preview(&item.path);
                        }
                    }
                    KeyCode::Enter | KeyCode::Right | KeyCode::Char('l') => {
                        let snap = crate::monitors::files::snapshot();
                        let items = self.get_filtered_files(&snap);
                        if let Some(item) = items.get(self.panel_states.files_selected) {
                            if item.is_dir {
                                crate::monitors::files::chdir(&item.path);
                                self.panel_states.files_selected = 0;
                            }
                        }
                    }
                    KeyCode::Delete | KeyCode::Char('x') => {
                        let snap = crate::monitors::files::snapshot();
                        let items = self.get_filtered_files(&snap);
                        if let Some(item) = items.get(self.panel_states.files_selected) {
                            crate::monitors::fs_tasks::start_trash(
                                format!(
                                    "trash-{}",
                                    std::time::SystemTime::now()
                                        .duration_since(std::time::UNIX_EPOCH)
                                        .unwrap()
                                        .as_nanos()
                                ),
                                item.path.to_string_lossy().to_string(),
                            );
                            let current = snap.current_dir.clone();
                            std::thread::spawn(move || {
                                std::thread::sleep(std::time::Duration::from_millis(500));
                                crate::monitors::files::chdir(&current);
                            });
                        }
                    }
                    KeyCode::Char('r') => {
                        let snap = crate::monitors::files::snapshot();
                        crate::monitors::files::chdir(&snap.current_dir);
                    }
                    KeyCode::Backspace | KeyCode::Left | KeyCode::Char('h') => {
                        let snap = crate::monitors::files::snapshot();
                        if let Some(parent) = snap.current_dir.parent() {
                            crate::monitors::files::chdir(parent);
                            self.panel_states.files_selected = 0;
                        }
                    }
                    _ => {}
                }
            }
            _ => {}
        }

        if self.mode.clone() == DashboardMode::Monitor && process_hotkey {
            self.focused_panel = Some(PanelId::Processes);
        }

        match self.focused_panel {
            Some(PanelId::Calendar) => {
                let ps = &mut self.panel_states;
                match key {
                    KeyCode::Left | KeyCode::Char('h') => ps.calendar_month_offset -= 1,
                    KeyCode::Right | KeyCode::Char('l') => ps.calendar_month_offset += 1,
                    KeyCode::Up | KeyCode::Char('k') => ps.calendar_month_offset -= 12,
                    KeyCode::Down | KeyCode::Char('j') => ps.calendar_month_offset += 12,
                    KeyCode::Home | KeyCode::Char('t') => ps.calendar_month_offset = 0,
                    _ => {}
                }
            }
            Some(PanelId::Processes) => self.process_key(key),
            _ => {}
        }
    }

    pub fn get_filtered_files(
        &self,
        snap: &crate::monitors::files::FilesSnapshot,
    ) -> Vec<crate::monitors::files::FileItem> {
        snap.items
            .iter()
            .filter(|item| {
                if !self.panel_states.files_show_hidden && item.name.starts_with('.') {
                    return false;
                }
                if !self.panel_states.files_search_input.is_empty()
                    && !item
                        .name
                        .to_lowercase()
                        .contains(&self.panel_states.files_search_input.to_lowercase())
                {
                    return false;
                }
                true
            })
            .cloned()
            .collect()
    }

    fn trigger_focused_action(&mut self) {
        if self.focused_panel == Some(PanelId::Status) {
            let row_ids = crate::widgets::status::active_row_ids();
            let max_idx = row_ids.len().saturating_sub(1);
            let sel = self.panel_states.status_selected.min(max_idx);
            if let Some(row_id) = row_ids.get(sel) {
                let cmd = match *row_id {
                    "wifi" => "nmtui",
                    "procs" | "load" | "memory" => "htop",
                    "docker" => "lazydocker",
                    _ => return, // No action
                };

                let _ = crate::mouse::suspended(|| std::process::Command::new(cmd).status());
                let _ = std::process::Command::new("clear").status();
                return;
            }
        }

        let path = match self.focused_panel {
            Some(PanelId::Tasks) => {
                crate::monitors::tasks::ensure_todo_file();
                Some(crate::monitors::tasks::get_todo_file())
            }
            Some(PanelId::Agenda) => {
                crate::monitors::agenda::ensure_agenda_file();
                Some(crate::monitors::agenda::get_agenda_file())
            }
            Some(PanelId::WriterNotes) => {
                let snap = crate::monitors::obsidian::snapshot();
                if snap.notes.is_empty() {
                    let vault_path = crate::monitors::obsidian::detect_vault_path(
                        &crate::config::Config::load().ui.obsidian_vault,
                    );
                    let _ = std::fs::create_dir_all(&vault_path);
                    Some(vault_path.join("Note.md"))
                } else {
                    let max_idx = snap.notes.len().saturating_sub(1);
                    let sel = self.panel_states.writer_selected.min(max_idx);
                    Some(snap.notes[sel].path.clone())
                }
            }
            Some(PanelId::Files) => {
                let snap = crate::monitors::files::snapshot();
                if snap.items.is_empty() {
                    None
                } else {
                    let max_idx = snap.items.len().saturating_sub(1);
                    let sel = self.panel_states.files_selected.min(max_idx);
                    if !snap.items[sel].is_dir {
                        Some(snap.items[sel].path.clone())
                    } else {
                        None
                    }
                }
            }
            _ => None,
        };

        if let Some(p) = path {
            let editor = get_preferred_editor();
            let _ = crate::mouse::suspended(|| std::process::Command::new(editor).arg(p).status());
            let _ = std::process::Command::new("clear").status();

            // Immediate rescan of all workspace monitors
            crate::monitors::obsidian::rescan();
            crate::monitors::tasks::rescan();
            crate::monitors::agenda::rescan();
            let snap_files = crate::monitors::files::snapshot();
            crate::monitors::files::chdir(&snap_files.current_dir);
        }
    }

    fn resize_focused_vertical(&mut self, delta: i16) {
        if let Some(id) = self.focused_panel {
            let key = format!("{:?}", id).to_lowercase();
            let new_val = {
                let entry = self
                    .panel_states
                    .dash_vertical
                    .entry(key.clone())
                    .or_insert(0);
                *entry = (*entry + delta).clamp(-30, 30);
                *entry
            };
            self.toast(format!("Resized {} vertically (adj: {})", key, new_val));
        }
    }

    fn resize_focused(&mut self, delta: i16) {
        let id = if let Some(i) = self.focused_panel {
            i
        } else {
            return;
        };

        if self.mode == DashboardMode::Workspace {
            let left = matches!(id, PanelId::Agenda | PanelId::Tasks | PanelId::News);
            if left {
                self.panel_states.work_ratio =
                    (self.panel_states.work_ratio as i16 + delta).clamp(10, 90) as u16;
            } else {
                self.panel_states.work_ratio =
                    (self.panel_states.work_ratio as i16 - delta).clamp(10, 90) as u16;
            }
            return;
        }

        let col = match self.mode {
            DashboardMode::Dashboard => {
                let layout = &self.config.dashboard.layout;
                let id_str = format!("{:?}", id).to_lowercase();
                let mut found_col = None;
                for (c_idx, c_arr) in layout.iter().enumerate() {
                    for item in c_arr {
                        if item == &id_str || (item == "cve_feed" && id_str == "cve") {
                            found_col = Some(c_idx);
                            break;
                        }
                    }
                }
                found_col
            }
            DashboardMode::Monitor => match id {
                PanelId::Cpu => Some(0),
                PanelId::Memory | PanelId::Disk => Some(1),
                PanelId::Network | PanelId::Gpu | PanelId::System => Some(2),
                _ => None,
            },
            DashboardMode::Extension(ref ext_id) => {
                let mut found_col = None;
                let pages = &self.config.pages;
                {
                    if let Some(page) = pages.iter().find(|p| p.name == *ext_id) {
                        let id_str = format!("{:?}", id).to_lowercase();
                        for (c_idx, c_arr) in page.layout.iter().enumerate() {
                            for item in c_arr {
                                if item == &id_str || (item == "cve_feed" && id_str == "cve") {
                                    found_col = Some(c_idx);
                                    break;
                                }
                            }
                        }
                    }
                }
                found_col
            }
            _ => None,
        };

        if let Some(c) = col {
            if c == 0 {
                self.panel_states.dash_ratios[0] =
                    (self.panel_states.dash_ratios[0] as i16 + delta).clamp(10, 80) as u16;
                let rem = 100u16.saturating_sub(self.panel_states.dash_ratios[0]);
                if self.panel_states.dash_ratios[2] > 0 {
                    self.panel_states.dash_ratios[1] = rem
                        .saturating_sub(self.panel_states.dash_ratios[2])
                        .clamp(10, 80);
                } else {
                    self.panel_states.dash_ratios[1] = rem;
                }
            } else if c == 1 {
                self.panel_states.dash_ratios[1] =
                    (self.panel_states.dash_ratios[1] as i16 + delta).clamp(10, 80) as u16;
                let rem = 100u16.saturating_sub(self.panel_states.dash_ratios[1]);
                if self.panel_states.dash_ratios[2] > 0 {
                    self.panel_states.dash_ratios[2] = rem
                        .saturating_sub(self.panel_states.dash_ratios[0])
                        .clamp(10, 80);
                } else {
                    self.panel_states.dash_ratios[0] = rem;
                }
            } else if c == 2 {
                self.panel_states.dash_ratios[2] =
                    (self.panel_states.dash_ratios[2] as i16 - delta).clamp(10, 80) as u16;
                let rem = 100u16.saturating_sub(self.panel_states.dash_ratios[2]);
                self.panel_states.dash_ratios[1] = rem
                    .saturating_sub(self.panel_states.dash_ratios[0])
                    .clamp(10, 80);
            }
        }
    }

    fn process_nav(&mut self, key: KeyCode) {
        let ps = &mut self.panel_states;
        let page = 10;
        match key {
            KeyCode::Up => ps.process_scroll_offset = ps.process_scroll_offset.saturating_sub(1),
            KeyCode::Down => ps.process_scroll_offset = ps.process_scroll_offset.saturating_add(1),
            KeyCode::PageUp => {
                ps.process_scroll_offset = ps.process_scroll_offset.saturating_sub(page)
            }
            KeyCode::PageDown => {
                ps.process_scroll_offset = ps.process_scroll_offset.saturating_add(page)
            }
            KeyCode::Home => ps.process_scroll_offset = 0,
            KeyCode::End => ps.process_scroll_offset = processes::count().saturating_sub(1),
            _ => {}
        }
        self.sync_selected_pid();
    }

    fn process_key(&mut self, key: KeyCode) {
        match key {
            KeyCode::Up
            | KeyCode::Down
            | KeyCode::PageUp
            | KeyCode::PageDown
            | KeyCode::Home
            | KeyCode::End => self.process_nav(key),
            KeyCode::Char('t') => {
                let ps = &mut self.panel_states;
                ps.process_tree_mode = !ps.process_tree_mode;
                ps.process_scroll_offset = 0;
                self.sync_selected_pid();
            }
            KeyCode::Char('c') => {
                self.panel_states.process_compact_cmd = !self.panel_states.process_compact_cmd
            }
            KeyCode::Right if self.panel_states.process_tree_mode => self.set_collapsed(false),
            KeyCode::Left if self.panel_states.process_tree_mode => self.set_collapsed(true),
            KeyCode::Char('s') => {
                let ps = &mut self.panel_states;
                ps.process_sort_field = ps.process_sort_field.next();
                ps.process_scroll_offset = 0;
                self.sync_selected_pid();
            }
            KeyCode::Char('r') => {
                let ps = &mut self.panel_states;
                ps.process_sort_asc = !ps.process_sort_asc;
                ps.process_scroll_offset = 0;
                self.sync_selected_pid();
            }
            KeyCode::Char('/') => {
                let ps = &mut self.panel_states;
                ps.process_search_active = true;
                ps.process_search.clear();
                ps.process_scroll_offset = 0;
            }
            KeyCode::Char('k') => self.signal_selected("TERM"),
            KeyCode::Char('K') => self.signal_selected("KILL"),
            KeyCode::Char('x') | KeyCode::Char('X') => {
                self.pending_signal = None;
                self.toast("cancelled");
            }
            _ => {}
        }
    }

    fn selected_pid(&self) -> Option<u32> {
        let ps = &self.panel_states;
        processes::get_pid_at(
            ps.process_scroll_offset,
            ps.process_sort_field,
            ps.process_sort_asc,
            &ps.process_search,
            ps.process_tree_mode,
            &ps.process_collapsed,
        )
    }

    fn set_collapsed(&mut self, collapse: bool) {
        if let Some(pid) = self.selected_pid() {
            if collapse {
                self.panel_states.process_collapsed.insert(pid);
            } else {
                self.panel_states.process_collapsed.remove(&pid);
            }
            self.sync_selected_pid();
        }
    }

    fn signal_selected(&mut self, sig: &'static str) {
        let Some(pid) = self.selected_pid() else {
            return;
        };
        if pid == std::process::id() {
            self.toast("not killing myself — press q to quit");
            return;
        }
        // Two-step: the first press arms, the second (same pid + signal,
        // within 3s) fires. A mis-typed k on the wrong row is otherwise fatal.
        let armed = self
            .pending_signal
            .is_some_and(|(p, s, t)| p == pid && s == sig && t.elapsed() < Duration::from_secs(3));
        if !armed {
            let name = processes::name_of(pid).unwrap_or_default();
            self.pending_signal = Some((pid, sig, Instant::now()));
            self.toast(format!(
                "SIG{} → {} {}?  press {} again to confirm, x to cancel",
                sig,
                pid,
                name,
                if sig == "KILL" { "K" } else { "k" }
            ));
            return;
        }
        self.pending_signal = None;
        let ok = std::process::Command::new("kill")
            .args([&format!("-{}", sig), &pid.to_string()])
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
        self.toast(if ok {
            format!("sent SIG{} to {}", sig, pid)
        } else {
            format!("failed to signal {} (permission?)", pid)
        });
    }

    pub fn sync_selected_pid(&mut self) {
        self.panel_states.process_selected_pid = self.selected_pid();
    }

    // ── Render ────────────────────────────────────────────────

    pub fn render(&mut self, f: &mut Frame) {
        self.frame = self.frame.wrapping_add(1);
        screens::clear_hits();
        screens::set_design(self.config.ui.style);
        self.summary = monitors::summary();
        if self
            .toast
            .as_ref()
            .is_some_and(|(_, t)| t.elapsed() > Duration::from_secs(3))
        {
            self.toast = None;
        }

        // Drain any completed image encodes from the background thread.
        if let Some(rx) = &self.image_resize_rx {
            while let Ok(result) = rx.try_recv() {
                if let Ok(response) = result {
                    // ThreadProtocol::update_resized_protocol takes ownership.
                    // Find the first protocol and feed it — it checks the ID internally.
                    let mut iter = self.thread_protocols.values_mut();
                    if let Some(proto) = iter.next() {
                        if proto.update_resized_protocol(response) {
                            crate::anim::request(2);
                        }
                    }
                }
            }
        }

        // Advance custom widget history buffers before any rendering.
        self.custom_widgets.tick();
        crate::widgets::pomodoro::tick(&self.config.ui);
        self.announce_alerts();
        self.apply_night();

        let area = f.area();

        let is_transparent = self
            .config
            .ui
            .transparent
            .unwrap_or_else(|| !self.theme.is_light());
        if !is_transparent {
            f.render_widget(
                ratatui::widgets::Block::default().style(Style::default().bg(self.theme.bg)),
                area,
            );
        }
        // Ambient scenes carry their own hint line, so the key bar only
        // appears there for toasts and text input.
        let quiet = self.mode == DashboardMode::Aesthetic
            && self.zoomed.is_none()
            && self.toast.is_none()
            && !self.panel_states.pinned_media_input_active;
        let [title_bar, main, status_bar] = Layout::vertical([
            Constraint::Length(1),
            Constraint::Min(0),
            Constraint::Length(u16::from(!quiet)),
        ])
        .areas(area);

        self.render_title(f, title_bar);

        match (self.zoomed, self.mode.clone()) {
            (Some(p), _) => screens::render_panel(f, main, self, p),
            (None, DashboardMode::Dashboard) => screens::dashboard::render(f, main, self),
            (None, DashboardMode::Monitor) => screens::monitor::render(f, main, self),
            (None, DashboardMode::Aesthetic) => screens::aesthetic::render(f, main, self),
            (None, DashboardMode::Workspace) => screens::workspace::render(f, main, self),
            (None, DashboardMode::DebugLogs) => screens::debug_logs::render(f, main, self),
            (None, DashboardMode::Extension(name)) => {
                let mut rendered = false;
                for ext in &self.ext_manager.extensions {
                    for mut page in ext.pages() {
                        if page.title() == name {
                            page.render(f, main, &self.theme);
                            rendered = true;
                            break;
                        }
                    }
                }
                if !rendered {
                    // It might be a custom page from config
                    if let Some(cfg_page) = self.config.pages.iter().find(|p| p.name == name) {
                        screens::dashboard::render_layout(f, main, self, &cfg_page.layout.clone());
                        rendered = true;
                    }
                }

                if !rendered {
                    screens::dashboard::render(f, main, self);
                }
            }
        }

        self.render_status(f, status_bar);

        if self.show_help {
            screens::help::render(f, area, &self.theme, &self.config, &mut self.help_scroll);
        }
        if self.show_setup_wizard {
            screens::setup_wizard::render(f, area, self);
        } else if self.show_settings {
            screens::settings::render(f, area, self);
        }
        if self.show_menu {
            screens::menu::render(f, area, self);
        }
    }

    /// Desktop-notify alerts as they appear or turn critical. Checked each
    /// frame but only acts on changes, so it costs a comparison.
    fn announce_alerts(&mut self) {
        let active = alerts(&self.summary);
        let keys: Vec<(&'static str, bool)> = active.iter().map(|a| (a.kind, a.critical)).collect();
        let fire = self.alert_latch.update(&keys);
        if !self.config.ui.notify {
            return;
        }
        for (kind, _) in fire {
            if let Some(a) = active.iter().find(|a| a.kind == kind) {
                use crate::notify::{send, Urgency};
                let urgency = if a.critical {
                    Urgency::Critical
                } else {
                    Urgency::Normal
                };
                send("vanta", &a.long, urgency);
            }
        }
    }

    /// Swap in the dimmed palette during `ui.night_hours`. The check is a
    /// clock read per frame; the theme is only rebuilt when the state flips.
    fn apply_night(&mut self) {
        use chrono::Timelike;
        let now = chrono::Local::now();
        let night = self.config.ui.night_at(now.hour() * 60 + now.minute());
        if self.night != Some(night) {
            let base = Theme::from_name(&self.config.ui.theme);
            self.theme = if night { base.dimmed() } else { base };
            self.night = Some(night);
        }
    }

    fn render_title(&self, f: &mut Frame, area: Rect) {
        let t = &self.theme;
        let s = &self.summary;
        let base = Style::default().bg(t.bg);
        let dim = base.fg(t.dim);
        let sep = Span::styled("   ", dim);

        let mut worst = 0u8; // 0 ok, 1 warn, 2 crit
        let mut level = |pct: f64| {
            let l = if pct >= 90.0 {
                2
            } else if pct >= 75.0 {
                1
            } else {
                0
            };
            worst = worst.max(l);
            match l {
                2 => t.red,
                1 => t.yellow,
                _ => t.accent,
            }
        };

        let mut metrics: Vec<(&str, String, ratatui::style::Color)> = vec![
            (
                "cpu",
                format!("{:>3.0}%", s.cpu_pct),
                level(s.cpu_pct as f64),
            ),
            ("mem", format!("{:>3.0}%", s.mem_pct), level(s.mem_pct)),
        ];
        if let Some(g) = s.gpu_pct {
            metrics.push(("gpu", format!("{:>3.0}%", g), level(g)));
        }
        if let Some(d) = s.disk_pct {
            metrics.push(("disk", format!("{:>3.0}%", d), level(d)));
        }
        if let Some(c) = s.temp_c {
            // Relative to the chip's own limit: plenty of laptops idle at
            // 80°+, which a fixed 75° warning would flag all day.
            let col = if s.temp_critical() {
                worst = 2;
                t.red
            } else if s.hot {
                worst = worst.max(1);
                t.yellow
            } else {
                t.accent
            };
            metrics.push(("temp", format!("{:.0}°", c), col));
        }
        metrics.push((
            "net",
            format!(
                "↓{} ↑{}",
                crate::widgets::meter::fmt_kbps_fixed(s.rx_kbps),
                crate::widgets::meter::fmt_kbps_fixed(s.tx_kbps)
            ),
            t.secondary,
        ));
        if let Some((p, charging)) = s.battery {
            let col = if charging || p > 20 {
                t.accent
            } else if p > 10 {
                worst = worst.max(1);
                t.yellow
            } else {
                worst = 2;
                t.red
            };
            metrics.push((
                "bat",
                format!("{}%{}", p, if charging { "⚡" } else { "" }),
                col,
            ));
        }
        metrics.push(("up", s.uptime.clone(), t.text));

        let dot = match worst {
            2 => t.red,
            1 => t.yellow,
            _ => t.green,
        };
        let mut left: Vec<Span> = vec![
            Span::styled(" vanta", base.fg(t.accent)),
            Span::styled(" ● ", base.fg(dot)),
        ];
        if self.night == Some(true) {
            left.push(Span::styled("☾  ", dim));
        }
        // The most severe active alert gets a coloured pill so it's visible
        // from across the room; extra alerts are summarised as "+N".
        let alerts = alerts(s);
        if let Some(a) = alerts.first() {
            let bg = if a.critical { t.red } else { t.yellow };
            left.push(Span::styled(
                format!(" ⚠ {} ", a.short),
                Style::default()
                    .fg(t.bg)
                    .bg(bg)
                    .add_modifier(ratatui::style::Modifier::BOLD),
            ));
            let more = if alerts.len() > 1 {
                format!(" +{}  ", alerts.len() - 1)
            } else {
                "  ".to_string()
            };
            left.push(Span::styled(more, dim));
        }
        // A running focus timer follows you across pages.
        if let Some((text, color)) = crate::widgets::pomodoro::badge(t) {
            left.push(Span::styled(
                format!(" {} ", text),
                Style::default()
                    .fg(t.bg)
                    .bg(color)
                    .add_modifier(ratatui::style::Modifier::BOLD),
            ));
            left.push(Span::styled("  ", dim));
        }
        let fixed_spans = left.len();
        for (i, (k, v, c)) in metrics.iter().enumerate() {
            if i > 0 {
                left.push(sep.clone());
            }
            left.push(Span::styled(format!("{} ", k), dim));
            left.push(Span::styled(v.clone(), base.fg(*c)));
        }

        let mut right: Vec<Span> = Vec::new();
        // (offset into `right`, width) of each page tab, for mouse hits.
        let mut tabs: Vec<(usize, usize)> = Vec::new();

        let modes = self.available_modes();

        for (i, m) in modes.into_iter().enumerate() {
            let style = if m == self.mode {
                Style::default().fg(t.bg).bg(t.accent)
            } else {
                dim
            };
            // Hotkey is simply (i+1) instead of hardcoded!
            let hotkey = format!("{}", i + 1);
            let label = format!(" {} {} ", hotkey, m.label());
            let at: usize = right.iter().map(|s| s.content.chars().count()).sum();
            tabs.push((at, label.chars().count()));
            right.push(Span::styled(label, style));
            right.push(Span::styled(" ", base));
        }
        right.push(Span::styled("? help ", dim));

        let width = |v: &[Span]| -> usize { v.iter().map(|s| s.content.chars().count()).sum() };
        let (rw, avail) = (width(&right), area.width as usize);
        // Narrow terminal: shed metrics from the right (least important last)
        // until the page nav fits. Each metric is 3 spans (sep, key, value).
        while width(&left) + rw >= avail && left.len() > fixed_spans + 3 {
            left.truncate(left.len() - 3);
        }
        let lw = width(&left);
        let mut spans = left;
        if lw + rw < avail {
            spans.push(Span::styled(" ".repeat(avail - lw - rw), base));
            spans.extend(right);
            let x0 = area.x + (avail - rw) as u16;
            for (i, (at, w)) in tabs.into_iter().enumerate() {
                screens::hit(
                    Rect::new(x0 + at as u16, area.y, w as u16, 1),
                    screens::Hit::Tab(i),
                );
            }
        } else if lw < avail {
            spans.push(Span::styled(" ".repeat(avail - lw), base));
        }
        f.render_widget(Paragraph::new(Line::from(spans)).style(base), area);
    }

    fn render_status(&self, f: &mut Frame, area: Rect) {
        let t = &self.theme;
        let base = Style::default().bg(t.surface);
        let key = base.fg(t.accent);
        let txt = base.fg(t.dim);

        let mut spans: Vec<Span> = vec![Span::styled(" ", base)];
        let mut hint = |k: &str, d: &str| {
            spans.push(Span::styled(k.to_string(), key));
            spans.push(Span::styled(format!(" {}   ", d), txt));
        };

        if let Some((msg, _)) = &self.toast {
            spans.push(Span::styled(format!("{}   ", msg), base.fg(t.text)));
        } else if self.panel_states.process_search_active {
            hint("type", "to filter");
            hint("enter", "keep filter");
            hint("esc", "clear");
        } else if self.panel_states.task_input_active
            || self.panel_states.agenda_input_active
            || self.panel_states.pinned_media_input_active
        {
            hint("type", "to insert");
            hint("enter", "save (keep open)");
            hint("esc", "done");
        } else {
            match self.focused_panel {
                Some(PanelId::Tasks) | Some(PanelId::Agenda) => {
                    hint("a", "add");
                    if self.focused_panel == Some(PanelId::Agenda) {
                        hint("r", "edit");
                    }
                    hint("d", "delete");
                    hint("e", "edit file");
                    if self.focused_panel == Some(PanelId::Tasks) {
                        hint("space", "toggle");
                    }
                    hint(
                        "enter",
                        if self.zoomed.is_some() {
                            "unzoom"
                        } else {
                            "zoom"
                        },
                    );
                }
                Some(PanelId::PinnedMedia) => {
                    hint("e/i", "edit path");
                    hint(
                        "enter",
                        if self.zoomed.is_some() {
                            "unzoom"
                        } else {
                            "zoom"
                        },
                    );
                    hint("tab", "focus");
                }
                Some(PanelId::Processes) => {
                    hint("↑↓", "select");
                    hint("/", "search");
                    hint("s", "sort");
                    hint("r", "reverse");
                    hint("t", "tree");
                    if self.panel_states.process_tree_mode {
                        hint("←→", "fold");
                    }
                    hint("c", "cmd");
                    hint("k", "term");
                    hint("K", "kill");
                }
                Some(PanelId::Files) => {
                    hint("↑↓", "move");
                    hint("←→", "dir");
                    hint("/", "search");
                    hint(".", "hidden");
                    hint("N", "rename");
                    hint("m", "mkdir");
                    hint("y", "copy path");
                    hint("o", "open");
                    hint("x", "delete");
                }
                Some(PanelId::WriterNotes) => {
                    hint("↑↓", "note");
                    hint("PgUp/PgDn", "scroll");
                    hint("enter", "edit");
                    hint("tab", "focus");
                }
                Some(PanelId::Timer) => {
                    hint("space", "start/pause");
                    hint("r", "reset");
                    hint("s", "skip");
                    hint("tab", "focus");
                }
                Some(PanelId::Calendar) => {
                    hint("←→", "month");
                    hint("↑↓", "year");
                    hint("home", "today");
                }
                _ => {
                    hint("tab", "focus");
                    if self.focused_panel.is_some() {
                        hint(
                            "enter",
                            if self.zoomed.is_some() {
                                "unzoom"
                            } else {
                                "zoom"
                            },
                        );
                    }
                    match self.mode {
                        DashboardMode::Aesthetic => {
                            hint("←→", "scene");
                            hint("r", "rotation");
                            hint("o", "motion");
                        }
                        _ => {
                            hint("space", "play/pause");
                            hint("n/p", "track");
                        }
                    }
                    hint("T", "theme");
                    hint("S", "settings");
                }
            }
            hint("?", "keys");
            hint("q", "quit");
        }

        let mut right: Vec<Span> = Vec::new();
        if let Some(p) = self.focused_panel {
            right.push(Span::styled(format!("[{}] ", p.label()), base.fg(t.accent)));
        }
        if self.mode.clone() == DashboardMode::Monitor {
            let ps = &self.panel_states;
            right.push(Span::styled(
                format!(
                    "sort {}{} ",
                    ps.process_sort_field.label(),
                    if ps.process_sort_asc { "▴" } else { "▾" }
                ),
                txt,
            ));
        }
        right.push(Span::styled(
            format!(
                "{} · {} · {:.1}s · v{} ",
                self.config.ui.theme,
                self.config.ui.performance_mode.label().to_lowercase(),
                self.config.ui.refresh_rate,
                env!("CARGO_PKG_VERSION")
            ),
            txt,
        ));

        let width = |v: &[Span]| -> usize { v.iter().map(|s| s.content.chars().count()).sum() };
        let (lw, rw, avail) = (width(&spans), width(&right), area.width as usize);
        if lw + rw < avail {
            spans.push(Span::styled(" ".repeat(avail - lw - rw), base));
            spans.extend(right);
        }
        f.render_widget(Paragraph::new(Line::from(spans)).style(base), area);
    }
}

/// Graphics protocol a terminal is known to support, from its environment.
fn image_protocol_for(env: impl Fn(&str) -> String) -> ratatui_image::picker::ProtocolType {
    use ratatui_image::picker::ProtocolType::*;
    let (term, prog) = (env("TERM"), env("TERM_PROGRAM").to_lowercase());
    if !env("TMUX").is_empty() || term.starts_with("screen") {
        // Multiplexers need passthrough for pixels; blocks always work.
        Halfblocks
    } else if !env("KITTY_WINDOW_ID").is_empty()
        || term.contains("kitty")
        || term.contains("ghostty")
        || prog == "ghostty"
        || prog == "wezterm"
    {
        Kitty
    } else if prog == "iterm.app" {
        Iterm2
    } else if term.starts_with("foot")
        || term.contains("mlterm")
        || !env("KONSOLE_VERSION").is_empty()
        || !env("WT_SESSION").is_empty()
    {
        Sixel
    } else {
        Halfblocks
    }
}

/// Image picker built without querying the terminal. ratatui-image's
/// `from_query_stdio` leaves a thread blocked on stdin whenever the terminal
/// doesn't answer within its timeout (tmux, slow SSH, terminals that ignore
/// the query), and that thread swallows the user's first keypress. The cell
/// size comes from TIOCGWINSZ instead, and the protocol from the environment.
fn detect_image_picker() -> ratatui_image::picker::Picker {
    use ratatui_image::picker::{Picker, ProtocolType};
    let proto = image_protocol_for(|k| std::env::var(k).unwrap_or_default());
    let font = crossterm::terminal::window_size()
        .ok()
        .filter(|w| w.width > 0 && w.height > 0 && w.columns > 0 && w.rows > 0)
        .map(|w| (w.width / w.columns, w.height / w.rows));
    match font {
        Some(font) if proto != ProtocolType::Halfblocks => {
            // Deprecated only in favour of the stdin query we're avoiding.
            #[allow(deprecated)]
            let mut p = Picker::from_fontsize(font);
            p.set_protocol_type(proto);
            p
        }
        _ => Picker::halfblocks(),
    }
}

fn get_preferred_editor() -> String {
    if let Ok(ed) = std::env::var("EDITOR") {
        if !ed.trim().is_empty() {
            return ed;
        }
    }
    if let Ok(vis) = std::env::var("VISUAL") {
        if !vis.trim().is_empty() {
            return vis;
        }
    }
    for candidate in &["nvim", "vim", "micro", "nano"] {
        if let Ok(output) = std::process::Command::new("which").arg(candidate).output() {
            if output.status.success() {
                return candidate.to_string();
            }
        }
    }
    "nano".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui_image::picker::ProtocolType;

    fn proto(vars: &[(&str, &str)]) -> ProtocolType {
        image_protocol_for(|k| {
            vars.iter()
                .find(|(n, _)| *n == k)
                .map(|(_, v)| v.to_string())
                .unwrap_or_default()
        })
    }

    #[test]
    fn image_protocol_comes_from_the_environment() {
        assert_eq!(proto(&[("TERM", "foot")]), ProtocolType::Sixel);
        assert_eq!(proto(&[("TERM", "xterm-kitty")]), ProtocolType::Kitty);
        assert_eq!(proto(&[("TERM_PROGRAM", "WezTerm")]), ProtocolType::Kitty);
        assert_eq!(
            proto(&[("TERM_PROGRAM", "iTerm.app")]),
            ProtocolType::Iterm2
        );
        assert_eq!(
            proto(&[("TERM", "xterm-256color")]),
            ProtocolType::Halfblocks
        );
        // Inside tmux, blocks win even if the outer terminal could do pixels.
        assert_eq!(
            proto(&[("TERM", "foot"), ("TMUX", "/tmp/tmux-1000/default,1,0")]),
            ProtocolType::Halfblocks
        );
    }
}
