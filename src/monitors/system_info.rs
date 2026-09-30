use std::fs;
use std::sync::LazyLock;

use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::monitors::{gpu, Summary};
use crate::theme::Theme;
use crate::widgets::meter;

/// Facts that never change for the lifetime of the process.
pub struct Facts {
    pub os: String,
    pub os_id: String,
    pub host: String,
    pub kernel: String,
    pub shell: String,
    pub cpu: String,
    pub threads: usize,
    pub term: String,
}

pub static FACTS: LazyLock<Facts> = LazyLock::new(|| {
    let mut sys = sysinfo::System::new();
    sys.refresh_cpu_usage();
    let os = sysinfo::System::long_os_version().unwrap_or_else(|| "Unknown".into());
    let os_id = sysinfo::System::distribution_id().to_lowercase();
    let host = sysinfo::System::host_name().unwrap_or_else(|| "?".into());
    let kernel = sysinfo::System::kernel_version().unwrap_or_default();

    let cpu = sys
        .cpus()
        .first()
        .map(|c| short_cpu(c.brand()))
        .unwrap_or_default();

    Facts {
        os,
        os_id,
        host,
        kernel,
        shell: std::env::var("SHELL")
            .ok()
            .and_then(|s| s.rsplit('/').next().map(str::to_string))
            .unwrap_or_default(),
        cpu,
        threads: std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(1),
        term: std::env::var("TERM_PROGRAM")
            .or_else(|_| std::env::var("TERM"))
            .unwrap_or_default(),
    }
});

fn short_cpu(model: &str) -> String {
    let mut s = model
        .replace("(R)", "")
        .replace("(TM)", "")
        .replace(" CPU", "")
        .replace("Intel Core ", "")
        .replace("AMD ", "")
        .replace(" Processor", "")
        .replace("-Core", "c");
    if let Some(idx) = s.find(" @ ") {
        s.truncate(idx);
    }
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Battery {
    pub pct: u8,
    pub charging: bool,
    /// Draw (or charge) in watts, when the driver reports it.
    pub watts: Option<f64>,
    /// Estimated seconds to empty (discharging) or full (charging).
    pub eta_secs: Option<u64>,
}

/// Power/energy detail for the first battery; separate from the summary tuple
/// because only the status panel wants it.
pub fn read_battery_detail() -> Option<Battery> {
    let dir = fs::read_dir("/sys/class/power_supply").ok()?;
    let bat = dir
        .flatten()
        .find(|e| e.file_name().to_string_lossy().starts_with("BAT"))?
        .path();
    let num =
        |f: &str| -> Option<f64> { fs::read_to_string(bat.join(f)).ok()?.trim().parse().ok() };
    let pct = num("capacity")? as u8;
    let status = fs::read_to_string(bat.join("status")).unwrap_or_default();
    let charging = matches!(status.trim(), "Charging" | "Full");
    // µW directly, or µA × µV.
    let watts = num("power_now")
        .or_else(|| Some(num("current_now")? * num("voltage_now")? / 1e6))
        .map(|uw| uw / 1e6)
        .filter(|w| *w > 0.05);
    let (now, full) = match (num("energy_now"), num("energy_full")) {
        (Some(n), Some(f)) => (Some(n / 1e6), Some(f / 1e6)),
        _ => match (num("charge_now"), num("charge_full"), num("voltage_now")) {
            (Some(n), Some(f), Some(v)) => (Some(n * v / 1e12), Some(f * v / 1e12)),
            _ => (None, None),
        },
    };
    let eta_secs = match (watts, now, full, status.trim()) {
        (Some(w), Some(n), _, "Discharging") => Some((n / w * 3600.0) as u64),
        (Some(w), Some(n), Some(f), "Charging") => Some(((f - n).max(0.0) / w * 3600.0) as u64),
        _ => None,
    };
    Some(Battery {
        pct,
        charging,
        watts,
        eta_secs,
    })
}

/// First BAT* capacity under /sys/class/power_supply, or None on AC-only machines.
pub fn read_battery() -> Option<(u8, bool)> {
    let dir = fs::read_dir("/sys/class/power_supply").ok()?;
    for entry in dir.flatten() {
        if !entry.file_name().to_string_lossy().starts_with("BAT") {
            continue;
        }
        let pct = fs::read_to_string(entry.path().join("capacity"))
            .ok()?
            .trim()
            .parse::<u8>()
            .ok()?;
        let charging = fs::read_to_string(entry.path().join("status"))
            .map(|s| matches!(s.trim(), "Charging" | "Full"))
            .unwrap_or(false);
        return Some((pct, charging));
    }
    None
}

pub fn fmt_uptime(secs: u64) -> String {
    let d = secs / 86400;
    let h = (secs % 86400) / 3600;
    let m = (secs % 3600) / 60;
    if d > 0 {
        format!("{}d {}h", d, h)
    } else if h > 0 {
        format!("{}h {}m", h, m)
    } else {
        format!("{}m", m)
    }
}

// ── Distro logo ────────────────────────────────────────────────

/// Plain-string logo for distros (and fallback robot). Returns `Vec<String>`
/// so the caller can apply its own gradient colouring.
fn logo_lines(id: &str, height: u16, width: u16, force_robot: bool) -> Vec<String> {
    let known = [
        "arch",
        "archarm",
        "endeavouros",
        "manjaro",
        "cachyos",
        "ubuntu",
        "pop",
        "linuxmint",
        "fedora",
        "nobara",
        "debian",
        "raspbian",
        "nixos",
        "gentoo",
        "opensuse",
        "opensuse-tumbleweed",
        "opensuse-leap",
        "windows",
    ];
    if force_robot || !known.contains(&id) {
        // For the robot we delegate to robot_colored_lines which handles its
        // own rendering; return empty here so the caller skips plain rendering.
        let _ = (height, width);
        return vec![];
    }

    block_logo(id, height, width)
        .into_iter()
        .map(String::from)
        .collect()
}

// ── Spectacular animated Vanta robot ───────────────────────────

/// Returns a fully multi-coloured `Vec<Line>` for the Vanta robot mascot.
/// Each row has individually styled spans: glowing eyes, pulsing core,
/// waving arms, circuitry, scanlines — all animated across 8 phases.
pub fn robot_colored_lines<'a>(
    height: u16,
    width: u16,
    theme: &crate::theme::Theme,
) -> Vec<ratatui::text::Line<'a>> {
    use ratatui::style::{Color, Modifier, Style};
    use ratatui::text::{Line, Span};

    crate::anim::request(4);
    let millis = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    let step = (millis / 250) % 8; // 8 animation phases at 4fps

    // ── color palette ──────────────────────────────────────────
    let acc = theme.accent; // primary accent (usually blue/violet)
    let sec = theme.secondary; // secondary accent
    let txt = theme.text; // normal text / chrome
    let dim = theme.dim; // subtle chrome
    let sur = theme.surface; // panel surface
    let red = theme.red;
    let grn = theme.green;
    let ylw = theme.yellow;

    // Derived glow colours: a lighter tint of accent for eye "lit" state
    let glow = crate::theme::blend(acc, Color::Rgb(255, 255, 255), 0.35);
    let eye_off = crate::theme::blend(dim, sur, 0.4);
    let core_hot = crate::theme::blend(acc, Color::Rgb(255, 255, 255), 0.5);
    let chrome = crate::theme::blend(txt, sur, 0.3);
    let panel_hi = crate::theme::blend(sec, txt, 0.25);

    // Helper: styled span
    let s = |text: &'static str, col: Color| -> Span<'a> {
        Span::styled(text, Style::default().fg(col))
    };
    let sb = |text: &'static str, col: Color| -> Span<'a> {
        Span::styled(text, Style::default().fg(col).add_modifier(Modifier::BOLD))
    };

    // ── animation slots ────────────────────────────────────────
    // Eyes: 8 distinct states cycling through personality
    let (el, er, eye_col) = match step {
        0 => ("◉", "◉", glow),     // fully lit
        1 => ("●", "●", acc),      // solid
        2 => ("◈", "◈", sec),      // scanning diamond
        3 => ("⊙", "⊙", ylw),      // wide-eyed
        4 => ("◉", "◉", glow),     // lit again
        5 => ("─", "─", eye_off),  // sleep blink
        6 => ("◉", "◉", core_hot), // overclocking
        7 => ("⊛", "⊛", grn),      // green scan
        _ => ("◉", "◉", glow),
    };

    // Antenna tip
    let ant_col = if step.is_multiple_of(2) {
        red
    } else {
        crate::theme::blend(red, sur, 0.6)
    };

    // Mouth expression
    let (mouth, mouth_col) = match step {
        0 | 4 => ("▬▬▬▬▬", acc), // flat confident
        1 | 5 => ("╌╌╌╌╌", dim), // resting
        2 | 6 => ("◜   ◝", sec), // smiling
        3 | 7 => ("▔▔▔▔▔", ylw), // grin
        _ => ("▬▬▬▬▬", acc),
    };

    // Power core pulse
    let (core_char, core_col) = match step % 4 {
        0 => ("◈", core_hot),
        1 => ("◉", acc),
        2 => ("⊛", sec),
        3 => ("◎", crate::theme::blend(acc, sec, 0.5)),
        _ => ("◈", core_hot),
    };

    // Left arm pose (4 states)
    let (la1, la2, ra1, ra2) = match step % 4 {
        0 => ("╔═╗", "║ ║", "╔═╗", "║ ║"), // arms down
        1 => ("╱  ", " ╱ ", "  ╲", " ╲ "), // wave up
        2 => ("═══", "║ ║", "═══", "║ ║"), // arms out
        3 => ("╲  ", " ╲ ", "  ╱", " ╱ "), // wave down
        _ => ("╔═╗", "║ ║", "╔═╗", "║ ║"),
    };

    // Chest vent blink
    let vent_col = if step.is_multiple_of(3) { sec } else { dim };
    // Circuit flicker
    let ckt_col = if step.is_multiple_of(2) {
        crate::theme::blend(acc, sur, 0.5)
    } else {
        sur
    };

    // ── compact mode (height < 20 or width < 32) ───────────────
    if height < 20 || width < 32 {
        // Compact 10-line version — rich, clean, fits small panels
        let (small_eyes, small_eye_col) = match step % 4 {
            0 => ("◉  ◉", glow),
            1 => ("●  ●", acc),
            2 => ("◈  ◈", sec),
            3 => ("⊙  ⊙", ylw),
            _ => ("◉  ◉", glow),
        };
        let small_mouth = match step % 4 {
            0 | 3 => "▬▬▬▬",
            1 => "╌╌╌╌",
            2 => "◜  ◝",
            _ => "▬▬▬▬",
        };
        // Arms: 3-char wide each, animated
        let (arm_l, arm_r): (&'static str, &'static str) = match step % 4 {
            0 => ("╔═╗", "╔═╗"), // down
            1 => ("╱  ", "  ╲"), // wave up
            2 => ("═══", "═══"), // out
            3 => ("╲  ", "  ╱"), // wave down
            _ => ("╔═╗", "╔═╗"),
        };
        return vec![
            // antenna
            Line::from(vec![s("    ", dim), sb("◆", ant_col), s("     ", dim)]),
            // pole
            Line::from(vec![s("    │     ", chrome)]),
            // head top
            Line::from(vec![sb("  ╔══╧══╗  ", chrome)]),
            // eyes
            Line::from(vec![
                s("  ║ ", chrome),
                sb(small_eyes, small_eye_col),
                s(" ║  ", chrome),
            ]),
            // mouth
            Line::from(vec![
                s("  ║  ", chrome),
                s(small_mouth, mouth_col),
                s("  ║  ", chrome),
            ]),
            // jaw
            Line::from(vec![sb("  ╚═╤═══╤═╝  ", chrome)]),
            // shoulders + body
            Line::from(vec![
                sb(arm_l, panel_hi),
                s("╔═╧═╗", chrome),
                sb(arm_r, panel_hi),
            ]),
            // core
            Line::from(vec![
                s("   ", dim),
                s("║ ", chrome),
                sb(core_char, core_col),
                s(" ║   ", chrome),
            ]),
            // body bottom
            Line::from(vec![sb("   ╚═════╝   ", chrome)]),
            // hip bar (replaces legs in compact mode)
            Line::from(vec![
                s("   ", dim),
                sb(
                    "▰▰▰▰▰",
                    if step.is_multiple_of(2) {
                        grn
                    } else {
                        crate::theme::blend(grn, sur, 0.5)
                    },
                ),
                s("   ", dim),
            ]),
        ];
    }

    // ── Full spectacular 24-line robot ─────────────────────────
    // Width needed: ~28 chars. This is the showpiece.
    vec![
        // Row 0: antenna
        Line::from(vec![
            s("        ", dim),
            sb("◆", ant_col),
            s("              ", dim),
        ]),
        // Row 1: antenna pole + halo ring
        Line::from(vec![
            s("       ", dim),
            s("╿", chrome),
            s("              ", dim),
        ]),
        // Row 2: head top with ear bolts
        Line::from(vec![
            s("  ◈  ", ckt_col),
            sb("╔══════╧══════╗", chrome),
            s("  ◈  ", ckt_col),
        ]),
        // Row 3: ear connectors + visor top
        Line::from(vec![
            sb("══╡", panel_hi),
            s("║ ╔═════════╗ ║", chrome),
            sb("╞══", panel_hi),
        ]),
        // Row 4: eye row — the star of the show
        Line::from(vec![
            sb("══╡", panel_hi),
            s("║ ║ ", chrome),
            sb(el, eye_col),
            s("   ", dim),
            sb(er, eye_col),
            s(" ║ ║", chrome),
            sb("╞══", panel_hi),
        ]),
        // Row 5: scan line under eyes
        Line::from(vec![
            sb("  ║", chrome),
            s("║ ╚", dim),
            sb("═════════", ckt_col),
            s("╝ ║", dim),
            sb("║  ", chrome),
        ]),
        // Row 6: nose band / cheekline
        Line::from(vec![
            s("  ║", chrome),
            sb("║   ─────────   ║", dim),
            s("║  ", chrome),
        ]),
        // Row 7: mouth
        Line::from(vec![
            s("  ║", chrome),
            s("║  ╔", chrome),
            sb(mouth, mouth_col),
            s("╗  ║", chrome),
            s("║  ", chrome),
        ]),
        // Row 8: chin / jaw
        Line::from(vec![
            s("  ╚", chrome),
            sb("═╗ ╚═════════╝ ╔═", chrome),
            s("╝  ", chrome),
        ]),
        // Row 9: neck block
        Line::from(vec![
            s("    ", dim),
            sb("║ ╔═══════╗ ║", chrome),
            s("    ", dim),
        ]),
        // Row 10: shoulder brace
        Line::from(vec![
            s("╔══", dim),
            sb("╩══╩═══════╩══╩", chrome),
            s("══╗", dim),
        ]),
        // Row 11: upper body — arms + chest top
        Line::from(vec![
            sb(la1, panel_hi),
            s("║ ", chrome),
            sb("┌─────────┐", sur),
            s(" ║", chrome),
            sb(ra1, panel_hi),
        ]),
        // Row 12: arm mid + chest with vent slits
        Line::from(vec![
            sb(la2, panel_hi),
            s("║ ", chrome),
            s("│ ", chrome),
            sb("≡ ≡ ≡ ≡ ≡", vent_col),
            s(" │", chrome),
            s(" ║", chrome),
            sb(ra2, panel_hi),
        ]),
        // Row 13: arm mid + core row
        Line::from(vec![
            sb(la1, panel_hi),
            s("║ ", chrome),
            s("│  ⚙  ", chrome),
            sb(core_char, core_col),
            s("  ⚙  │", chrome),
            s(" ║", chrome),
            sb(ra1, panel_hi),
        ]),
        // Row 14: arm low + circuit detail
        Line::from(vec![
            sb(la2, panel_hi),
            s("║ ", chrome),
            s("│ ", chrome),
            sb("╌╌╌╌╌╌╌╌╌", ckt_col),
            s(" │", chrome),
            s(" ║", chrome),
            sb(ra2, panel_hi),
        ]),
        // Row 15: chest bottom + gear dots
        Line::from(vec![
            s("   ", dim),
            s("║ ", chrome),
            s("│ ", chrome),
            sb("◦ ◦ ◦ ◦ ◦", dim),
            s(" │", chrome),
            s(" ║", chrome),
            s("   ", dim),
        ]),
        // Row 16: waist connector
        Line::from(vec![
            s("   ", dim),
            sb("╚══╝ └─────────┘ ╚══╝", chrome),
            s("   ", dim),
        ]),
        // Row 17: hip bar
        Line::from(vec![
            s("      ", dim),
            sb("╔═══════════╗", chrome),
            s("      ", dim),
        ]),
        // Row 18: hip detail
        Line::from(vec![
            s("      ", dim),
            s("║ ", chrome),
            sb("▰▰▰▰▰▰▰▰▰", grn),
            s(" ║", chrome),
            s("      ", dim),
        ]),
        // Row 19: upper legs
        Line::from(vec![
            s("     ", dim),
            sb("╔╝   ╔══╗   ╚╗", chrome),
            s("     ", dim),
        ]),
        // Row 20: leg shafts
        Line::from(vec![
            s("     ", dim),
            sb("║", chrome),
            s("   ", dim),
            sb("║  ║", chrome),
            s("   ", dim),
            sb("║", chrome),
            s("     ", dim),
        ]),
        // Row 21: knee joint
        Line::from(vec![
            s("    ", dim),
            sb("╔╩╗  ╔╩══╩╗  ╔╩╗", chrome),
            s("    ", dim),
        ]),
        // Row 22: lower legs
        Line::from(vec![
            s("    ", dim),
            sb("║ ║  ║    ║  ║ ║", chrome),
            s("    ", dim),
        ]),
        // Row 23: feet
        Line::from(vec![
            s("   ", dim),
            sb("╚═══╝  ╚════╝  ╚═══╝", chrome),
            s("   ", dim),
        ]),
    ]
}

/// Distro art. Arch uses fastfetch-style ASCII art (compact in dashboard, full
/// in zoom). Other distros use clean block art.
fn block_logo(id: &str, height: u16, width: u16) -> Vec<&'static str> {
    match id {
        "arch" | "archarm" | "endeavouros" | "manjaro" | "cachyos" => {
            if height >= 19 && width >= 65 {
                vec![
                    "                  -`                 ",
                    "                 .o+`                ",
                    "                `ooo/                ",
                    "               `+oooo:               ",
                    "              `+oooooo:              ",
                    "              -+oooooo+:             ",
                    "            `/:-:++oooo+:            ",
                    "           `/++++/+++++++:           ",
                    "          `/++++++++++++++:          ",
                    "         `/+++ooooooooooooo/`        ",
                    "        ./ooosssso++osssssso+`       ",
                    "       .oossssso-````/ossssss+`      ",
                    "      -osssssso.      :ssssssso.     ",
                    "     :osssssss/        osssso+++.    ",
                    "    /ossssssss/        +ssssooo/-    ",
                    "  `/ossssso+/:-        -:/+osssso+-  ",
                    " `+sso+:-`                 `.-/+oso: ",
                    "`++:.                           `-/+/",
                    ".`                                 `/",
                ]
            } else {
                vec![
                    "          .o+`         ",
                    "         `ooo/         ",
                    "        `+oooo:        ",
                    "       -+oooooo+:      ",
                    "     `/:-:++oooo+:     ",
                    "    `/++++++++++++:    ",
                    "   ./ooosss++osssso+`  ",
                    "  .oossss-````/sssss+` ",
                    " -ossss/        /ssssso",
                    "`+sso+:-`     `.-/+oso:",
                ]
            }
        }
        "ubuntu" | "pop" | "linuxmint" => vec![
            "   ▄▄▄▄▄   ",
            " ◢█▀   ▀█◣ ",
            "▐█   ▄   █▌",
            "▐█  ▀ ▀  █▌",
            " ◥█▄   ▄█◤ ",
            "   ▀▀▀▀▀   ",
        ],
        "fedora" | "nobara" => vec![
            "  ▄▄▄▄▄▄▄  ",
            " ◢█▀   ▀█◣ ",
            "▐█  ▄▄▄█▌  ",
            "▐█  █▌ ▀   ",
            " ◥█▄█▌     ",
            "   ▀▀▀     ",
        ],
        "debian" | "raspbian" => vec![
            "   ▄▄▄▄▄   ",
            "  ◢█▀ ▀█◣  ",
            " ▐█  ▄  █▌ ",
            " ▐█ ▐█▌    ",
            "  ◥█▄▄█◤   ",
            "     ▀▀    ",
        ],
        "nixos" => vec![
            "  ◥█◣ ◢█◤  ",
            "▀▀▀◥█◣█◤▀▀▀",
            "   ◢█◤◥█◣  ",
            "  ◢█◤ ◥█◣  ",
            "▄▄◢█◤ ◥█◣▄▄",
            "  ◥█◣ ◢█◤  ",
        ],
        "gentoo" => vec![
            "  ▄▄▄▄▄▄   ",
            " ◢█▀  ▀█◣  ",
            "▐█  ▄▄  █▌ ",
            " ◥▀▀  ▄█◤  ",
            "   ◢██◤    ",
            "  ▀▀       ",
        ],
        "opensuse" | "opensuse-tumbleweed" | "opensuse-leap" => vec![
            "  ▄▄▄▄▄▄▄  ",
            " ◢█▀▀▀▀▀█◣ ",
            "▐█  ▄▄  █▌ ",
            "▐█ ▐██▌ ▄█▌",
            " ◥█▄▄▄▄▄█◤ ",
            "   ▀▀▀▀▀   ",
        ],
        "windows" => vec![
            " ▄▄▄▄ ▄▄▄▄ ",
            " ████ ████ ",
            " ▀▀▀▀ ▀▀▀▀ ",
            " ▄▄▄▄ ▄▄▄▄ ",
            " ████ ████ ",
            " ▀▀▀▀ ▀▀▀▀ ",
        ],
        _ => vec![],
    }
}

/// Render the neofetch-style key/value facts into `area`.
fn render_facts(
    f: &mut Frame,
    area: Rect,
    theme: &Theme,
    sum: &Summary,
    term: (u16, u16),
    facts: &Facts,
) {
    let mem = crate::monitors::memory::snapshot();
    let gpu_name = gpu::name();
    let bat = match sum.battery {
        Some((p, true)) => format!("{}% ⚡", p),
        Some((p, false)) => format!("{}%", p),
        None => "AC".to_string(),
    };

    let user = std::env::var("USER")
        .or_else(|_| std::env::var("USERNAME"))
        .unwrap_or_else(|_| "user".into());

    let clean_os = if let Some(stripped) = facts
        .os
        .strip_prefix("Linux (")
        .and_then(|s| s.strip_suffix(')'))
    {
        stripped.to_string()
    } else {
        facts.os.clone()
    };

    let mut kv: Vec<(&str, String)> = vec![
        ("OS", clean_os),
        ("HOST", facts.host.clone()),
        ("KERNEL", facts.kernel.clone()),
        ("UPTIME", sum.uptime.clone()),
        ("CPU", format!("{} ({}t)", facts.cpu, facts.threads)),
    ];
    if !gpu_name.is_empty() {
        kv.push(("GPU", gpu_name));
    }
    let ram_pct = if mem.total > 0 {
        (mem.used as f64 / mem.total as f64) * 100.0
    } else {
        0.0
    };
    kv.push((
        "RAM",
        format!(
            "{} / {} · {:.0}%",
            meter::fmt_bytes(mem.used),
            meter::fmt_bytes(mem.total),
            ram_pct
        ),
    ));
    kv.push(("SHELL", facts.shell.clone()));
    kv.push(("TERM", format!("{} {}×{}", facts.term, term.0, term.1)));
    kv.push(("POWER", bat));

    let pad: u16 = 1;
    let show_header = area.height >= 8 && area.width >= 20;
    let header_rows = if show_header { 2 } else { 0 };
    let max_rows = area.height.saturating_sub(pad + header_rows) as usize;
    let kv_count = kv.len().min(max_rows);
    let kv: Vec<_> = kv.into_iter().take(kv_count).collect();
    let max_v = (area.width as usize).saturating_sub(12);

    let mut rows: Vec<Line> = (0..pad).map(|_| Line::from("")).collect();

    if show_header {
        rows.push(Line::from(vec![
            Span::styled("◈ ", Style::default().fg(theme.dim)),
            Span::styled(
                user,
                Style::default()
                    .fg(theme.accent)
                    .add_modifier(ratatui::style::Modifier::BOLD),
            ),
            Span::styled("@", Style::default().fg(theme.dim)),
            Span::styled(
                facts.host.clone(),
                Style::default()
                    .fg(theme.accent)
                    .add_modifier(ratatui::style::Modifier::BOLD),
            ),
        ]));
        let div_len = (facts.host.len() + 10).min(area.width as usize).min(28);
        rows.push(Line::from(vec![
            Span::raw("  "),
            Span::styled("─".repeat(div_len), Style::default().fg(theme.surface)),
        ]));
    }

    for (k, v) in kv {
        rows.push(Line::from(vec![
            Span::styled("◈ ", Style::default().fg(theme.accent)),
            Span::styled(
                format!("{:<6} ", k),
                Style::default()
                    .fg(theme.secondary)
                    .add_modifier(ratatui::style::Modifier::BOLD),
            ),
            Span::styled("· ", Style::default().fg(theme.dim)),
            Span::styled(meter::ellipsize(&v, max_v), Style::default().fg(theme.text)),
        ]));
    }

    if (area.height as usize) >= rows.len() + 2 && area.width >= 22 {
        rows.push(Line::from(""));
        rows.push(Line::from(vec![
            Span::styled("● ", Style::default().fg(theme.red)),
            Span::styled("● ", Style::default().fg(theme.green)),
            Span::styled("● ", Style::default().fg(theme.yellow)),
            Span::styled("● ", Style::default().fg(theme.accent)),
            Span::styled("● ", Style::default().fg(theme.secondary)),
            Span::styled("● ", Style::default().fg(theme.text)),
            Span::styled("● ", Style::default().fg(theme.dim)),
            Span::styled("● ", Style::default().fg(theme.surface)),
        ]));
    }

    f.render_widget(Paragraph::new(rows), area);
}

/// Dashboard SYSTEM panel: distro logo left, neofetch-style facts right.
pub fn render_neofetch(
    f: &mut Frame,
    area: Rect,
    theme: &Theme,
    sum: &Summary,
    term: (u16, u16),
    force_robot: bool,
) {
    if area.height < 3 || area.width < 20 {
        return;
    }
    let facts = &*FACTS;
    // VANTA_LOGO forces a distro logo, for previewing art on any machine.
    let os_id = std::env::var("VANTA_LOGO").unwrap_or_else(|_| facts.os_id.clone());

    // ── Robot path: rich multi-color rendered lines ─────────────
    let known_distros = [
        "arch",
        "archarm",
        "endeavouros",
        "manjaro",
        "cachyos",
        "ubuntu",
        "pop",
        "linuxmint",
        "fedora",
        "nobara",
        "debian",
        "raspbian",
        "nixos",
        "gentoo",
        "opensuse",
        "opensuse-tumbleweed",
        "opensuse-leap",
        "windows",
    ];
    let use_robot = force_robot || !known_distros.contains(&os_id.as_str());

    if use_robot {
        let robot_lines = robot_colored_lines(area.height, area.width, theme);
        // Robot art width: ~28 visible chars in full mode, 16 in compact
        let robot_w: u16 = if area.height >= 20 && area.width >= 32 {
            29
        } else {
            18
        };
        let logo_w = if area.width >= robot_w + 20 {
            robot_w
        } else {
            0
        };
        if logo_w > 0 {
            let pad: u16 = if area.height > robot_lines.len() as u16 + 2 {
                area.height.saturating_sub(robot_lines.len() as u16) / 2
            } else {
                0
            };
            let mut lines: Vec<Line> = (0..pad).map(|_| Line::from("")).collect();
            lines.extend(robot_lines);
            f.render_widget(
                Paragraph::new(lines),
                Rect::new(area.x, area.y, logo_w, area.height),
            );
        }
        // Facts panel
        let kv_area = Rect::new(
            area.x + logo_w,
            area.y,
            area.width.saturating_sub(logo_w),
            area.height,
        );
        render_facts(f, kv_area, theme, sum, term, facts);
        return;
    }

    // ── Distro logo path: simple gradient string lines ───────────
    let logo = logo_lines(&os_id, area.height, area.width, force_robot);
    let max_len = logo.iter().map(|l| l.chars().count()).max().unwrap_or(0);
    // +2 for the leading indent and a column of air before the facts.
    let logo_w: u16 = if area.width >= (max_len + 20) as u16 {
        max_len as u16 + 2
    } else {
        0
    };

    let pad = if area.height > logo.len() as u16 + 4 {
        2
    } else {
        area.height.saturating_sub(logo.len() as u16) / 2
    };

    if logo_w > 0 {
        let mut lines: Vec<Line> = (0..pad).map(|_| Line::from("")).collect();
        let n = logo.len().max(1) as f32;
        lines.extend(logo.iter().enumerate().map(|(i, l)| {
            // Accent at the crown fading to secondary at the base: a flat fill
            // makes the art read as one undifferentiated blob.
            let col = crate::theme::blend(theme.accent, theme.secondary, i as f32 / n);
            Line::from(Span::styled(l.clone(), Style::default().fg(col)))
        }));
        f.render_widget(
            Paragraph::new(lines),
            Rect::new(area.x + 1, area.y, max_len as u16, area.height),
        );
    }

    let kv_area = Rect::new(
        area.x + logo_w,
        area.y,
        area.width.saturating_sub(logo_w),
        area.height,
    );
    render_facts(f, kv_area, theme, sum, term, facts);
}

/// Monitor page: compact two-column facts.
pub fn render(f: &mut Frame, area: Rect, theme: &Theme, sum: &Summary) {
    if area.height < 1 {
        return;
    }
    let facts = &*FACTS;
    let k = Style::default().fg(theme.dim);
    let v = Style::default().fg(theme.text);
    let bat = match sum.battery {
        Some((p, true)) => format!("{}% ⚡", p),
        Some((p, false)) => format!("{}%", p),
        None => "AC".to_string(),
    };
    let left = [
        (
            "os",
            facts.os.split_whitespace().next().unwrap_or("Linux").into(),
        ),
        ("host", facts.host.clone()),
        ("kernel", facts.kernel.clone()),
    ];
    let right = [
        ("up", sum.uptime.clone()),
        ("bat", bat),
        ("procs", crate::monitors::processes::count().to_string()),
    ];
    // Values are ellipsized to their column so nothing clips mid-word.
    let lines = |items: &[(&str, String)], width: u16| -> Vec<Line<'static>> {
        let kw = items.iter().map(|(k, _)| k.len()).max().unwrap_or(0);
        // One trailing cell keeps the left column off the right one.
        let vw = (width as usize).saturating_sub(kw + 3);
        items
            .iter()
            .map(|(key, val)| {
                Line::from(vec![
                    Span::styled(format!("{:>kw$}  ", key), k),
                    Span::styled(meter::ellipsize(val, vw), v),
                ])
            })
            .collect()
    };
    let cols = Layout::horizontal([Constraint::Ratio(3, 5), Constraint::Ratio(2, 5)]).split(area);
    f.render_widget(Paragraph::new(lines(&left, cols[0].width)), cols[0]);
    f.render_widget(Paragraph::new(lines(&right, cols[1].width)), cols[1]);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_arch_logos_uniform_lengths() {
        let compact = logo_lines("arch", 10, 60, false);
        assert_eq!(compact.len(), 10);
        for line in &compact {
            assert_eq!(line.chars().count(), 23);
        }

        let full = logo_lines("arch", 25, 80, false);
        assert_eq!(full.len(), 19);
        for line in &full {
            assert_eq!(line.chars().count(), 37);
        }
    }
}
