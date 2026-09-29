use std::fs;
use std::path::Path;
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{LazyLock, Mutex};
use std::time::{Duration, Instant, SystemTime};

use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::monitors::cpu;
use crate::theme::Theme;

/// Facts that need a subprocess or a directory walk. Refreshed on a TTL so the
/// ~125fps render loop never pays for them.
#[derive(Clone, Default)]
struct Facts {
    packages: Option<usize>,
    updates: Option<usize>,
    wifi: Option<(String, u8)>,
    ip: Option<String>,
    docker: Option<(usize, usize)>, // running, total
}

struct Cached {
    facts: Facts,
    stamp: Option<Instant>,
    updates_stamp: Option<Instant>,
}

static CACHE: LazyLock<Mutex<Cached>> = LazyLock::new(|| {
    Mutex::new(Cached {
        facts: Facts::default(),
        stamp: None,
        updates_stamp: None,
    })
});

static UPDATES_RUNNING: AtomicBool = AtomicBool::new(false);

struct PkgCache {
    count: usize,
    db_mtime: Option<SystemTime>,
    db_path: &'static str,
}

static PKG_CACHE: Mutex<Option<PkgCache>> = Mutex::new(None);

/// How long collected facts stay fresh.
const TTL: Duration = Duration::from_secs(30);
const UPDATES_TTL: Duration = Duration::from_secs(15 * 60);

fn run(cmd: &str, args: &[&str]) -> Option<String> {
    let out = Command::new(cmd).args(args).output().ok()?;
    if !out.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&out.stdout).into_owned())
}

/// Query package count using database mtime caching so we avoid
/// spawning subprocesses every 30 seconds unless a package changed.
fn count_packages() -> Option<usize> {
    let mut cache = PKG_CACHE.lock().unwrap();
    if let Some(c) = cache.as_mut() {
        if let Ok(meta) = fs::metadata(c.db_path) {
            if let Ok(mtime) = meta.modified() {
                if c.db_mtime == Some(mtime) {
                    return Some(c.count);
                }
                if let Some(n) = query_packages() {
                    c.count = n;
                    c.db_mtime = Some(mtime);
                    return Some(n);
                }
            }
        }
    }

    let candidates: [(&'static str, &'static str); 4] = [
        ("/var/lib/pacman/local", "pacman"),
        ("/var/lib/dpkg/status", "dpkg"),
        ("/var/lib/rpm", "rpm"),
        ("/lib/apk/db/installed", "apk"),
    ];

    for (db_path, _) in candidates {
        if let Ok(meta) = fs::metadata(db_path) {
            let mtime = meta.modified().ok();
            if let Some(n) = query_packages() {
                *cache = Some(PkgCache {
                    count: n,
                    db_mtime: mtime,
                    db_path,
                });
                return Some(n);
            }
        }
    }

    query_packages()
}

fn query_packages() -> Option<usize> {
    for (cmd, args) in [
        ("pacman", &["-Qq"][..]),
        ("dpkg-query", &["-f", ".\n", "-W"][..]),
        ("rpm", &["-qa"][..]),
        ("apk", &["info"][..]),
    ] {
        if let Some(out) = run(cmd, args) {
            return Some(out.lines().filter(|l| !l.trim().is_empty()).count());
        }
    }
    None
}

fn count_updates() -> Option<usize> {
    // checkupdates (arch) exits 2 when there is nothing to do, which `run`
    // already filters out as a non-success status.
    run("checkupdates", &[]).map(|o| o.lines().filter(|l| !l.trim().is_empty()).count())
}

/// Reads current wireless network and signal without triggering a NetworkManager
/// radio rescan (which causes frame drops and ping spikes).
fn read_wifi() -> Option<(String, u8)> {
    // Fast path 1: parse /proc/net/wireless for signal quality
    let signal_from_proc = if let Ok(proc_wireless) = fs::read_to_string("/proc/net/wireless") {
        proc_wireless.lines().skip(2).find_map(|l| {
            let mut parts = l.split_whitespace();
            let iface = parts.next()?.trim_end_matches(':');
            let _status = parts.next()?;
            let link_str = parts.next()?.trim_end_matches('.');
            let link: f64 = link_str.parse().ok()?;
            let pct = ((link / 70.0) * 100.0).round().clamp(0.0, 100.0) as u8;
            Some((iface.to_string(), pct))
        })
    } else {
        None
    };

    // Fast path 2: read SSID of current connection without wifi scanning
    if let Some(ssid) = run("iwgetid", &["-r"])
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
    {
        let sig = signal_from_proc.map(|(_, s)| s).unwrap_or(75);
        return Some((ssid, sig));
    }

    if let Some((iface, sig)) = &signal_from_proc {
        if let Some(out) = run("iw", &["dev", iface, "link"]) {
            for line in out.lines() {
                if let Some(ssid) = line.trim().strip_prefix("SSID: ") {
                    let s = ssid.trim();
                    if !s.is_empty() {
                        return Some((s.to_string(), *sig));
                    }
                }
            }
        }
        if let Some(out) = run(
            "nmcli",
            &["-t", "-f", "GENERAL.CONNECTION", "dev", "show", iface],
        ) {
            if let Some(conn) = out
                .lines()
                .find_map(|l| l.strip_prefix("GENERAL.CONNECTION:"))
            {
                let conn = conn.trim();
                if !conn.is_empty() && conn != "--" {
                    return Some((conn.to_string(), *sig));
                }
            }
        }
    }

    // Fallback: standard nmcli device wifi list
    let out = run("nmcli", &["-t", "-f", "active,ssid,signal", "dev", "wifi"])?;
    for line in out.lines() {
        let mut parts = line.split(':');
        if parts.next()? != "yes" {
            continue;
        }
        let ssid = parts.next()?.to_string();
        let signal = parts.next()?.parse().unwrap_or(0);
        return Some((ssid, signal));
    }
    None
}

/// Primary IPv4 address, prioritizing the default gateway route.
fn read_ip() -> Option<String> {
    let default_iface = if let Ok(route) = fs::read_to_string("/proc/net/route") {
        route.lines().skip(1).find_map(|l| {
            let mut parts = l.split_whitespace();
            let iface = parts.next()?;
            let dest = parts.next()?;
            if dest == "00000000" {
                Some(iface.to_string())
            } else {
                None
            }
        })
    } else {
        None
    };

    let args = if let Some(iface) = &default_iface {
        vec!["-o", "-4", "addr", "show", iface.as_str()]
    } else {
        vec!["-o", "-4", "addr", "show"]
    };

    let out = run("ip", &args)?;
    for line in out.lines() {
        let mut f = line.split_whitespace();
        let _idx = f.next()?;
        let iface = f.next()?;
        if iface == "lo" || iface.starts_with("docker") || iface.starts_with("waydroid") {
            continue;
        }
        let addr = f.nth(1)?;
        return Some(format!("{} {}", iface, addr.split('/').next()?));
    }
    None
}

fn read_docker() -> Option<(usize, usize)> {
    let socket_exists = Path::new("/var/run/docker.sock").exists()
        || std::env::var("XDG_RUNTIME_DIR")
            .map(|r| Path::new(&r).join("docker.sock").exists())
            .unwrap_or(false);
    if !socket_exists {
        return None;
    }

    let out = run("docker", &["ps", "-a", "--format", "{{.State}}"])?;
    let mut running = 0usize;
    let mut total = 0usize;
    for line in out.lines() {
        let s = line.trim();
        if !s.is_empty() {
            total += 1;
            if s.eq_ignore_ascii_case("running") {
                running += 1;
            }
        }
    }
    Some((running, total))
}

/// Refresh slow facts if stale. Runs on the dedicated vanta-facts background thread.
pub fn sample() {
    let (stale, updates_stale) = {
        let c = CACHE.lock().unwrap();
        (
            c.stamp.is_none_or(|t| t.elapsed() > TTL),
            c.updates_stamp.is_none_or(|t| t.elapsed() > UPDATES_TTL),
        )
    };

    if updates_stale
        && UPDATES_RUNNING
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_ok()
    {
        std::thread::Builder::new()
            .name("vanta-checkupdates".into())
            .spawn(move || {
                let n = count_updates();
                if let Ok(mut c) = CACHE.lock() {
                    c.facts.updates = n;
                    c.updates_stamp = Some(Instant::now());
                }
                UPDATES_RUNNING.store(false, Ordering::SeqCst);
            })
            .ok();
    }

    if !stale {
        return;
    }

    let packages = count_packages();
    let wifi = read_wifi();
    let ip = read_ip();
    let docker = read_docker();

    let mut c = CACHE.lock().unwrap();
    c.facts.packages = packages;
    c.facts.wifi = wifi;
    c.facts.ip = ip;
    c.facts.docker = docker;
    c.stamp = Some(Instant::now());
}

fn facts() -> Facts {
    CACHE.lock().unwrap().facts.clone()
}

fn signal_bars(pct: u8) -> &'static str {
    match pct {
        0..=20 => "▂   ",
        21..=40 => "▂▄  ",
        41..=60 => "▂▄▆ ",
        _ => "▂▄▆█",
    }
}

/// System/user status: network, packages, containers, load, session.
/// Clean cyber badges, aligned columns, and non-blocking background collection.
pub fn render(f: &mut Frame, area: Rect, theme: &Theme, is_focused: bool, selected: usize) {
    if area.height == 0 || area.width < 16 {
        return;
    }
    let fx = facts();

    let build_line = |idx: usize, k: &str, v_spans: Vec<Span<'static>>| {
        let is_sel = is_focused && idx == selected;
        let badge_style = if is_sel {
            Style::default().fg(theme.bg).bg(theme.accent)
        } else {
            Style::default().fg(theme.accent)
        };
        let label_style = if is_sel {
            Style::default().fg(theme.bg).bg(theme.accent)
        } else if is_focused {
            Style::default().fg(theme.text)
        } else {
            Style::default().fg(theme.dim)
        };

        let mut spans = vec![
            Span::styled("◈ ", badge_style),
            Span::styled(format!("{:<6} ", k), label_style),
            Span::styled("· ", Style::default().fg(theme.dim)),
        ];
        spans.extend(v_spans);
        Line::from(spans)
    };

    let mut lines: Vec<Line> = Vec::new();
    let mut i = 0;

    if let Some((ssid, sig)) = &fx.wifi {
        let w_name = if area.width >= 36 { 14 } else { 10 };
        let ellip = crate::widgets::meter::ellipsize(ssid, w_name);
        // Pad SSID to fixed width so signal bars always start at the same column
        let padded = format!("{:<w$}", ellip, w = w_name);
        let mut v = vec![
            Span::styled(padded, Style::default().fg(theme.text)),
            Span::styled(" · ", Style::default().fg(theme.dim)),
            Span::styled(
                signal_bars(*sig),
                Style::default().fg(if *sig < 40 {
                    theme.yellow
                } else {
                    theme.accent
                }),
            ),
            Span::styled(format!(" {:>3}%", sig), Style::default().fg(theme.dim)),
        ];
        if area.width < 28 {
            v.truncate(1);
        }
        lines.push(build_line(i, "WIFI", v));
        i += 1;
    }

    if let Some(ip) = &fx.ip {
        let mut parts = ip.split_whitespace();
        let iface = parts.next().unwrap_or("net");
        let addr = parts.next().unwrap_or(ip.as_str());
        // Pad iface to fixed 5 chars so IP address always starts at the same column
        let v = vec![
            Span::styled(format!("{:<5}", iface), Style::default().fg(theme.dim)),
            Span::styled(" · ", Style::default().fg(theme.dim)),
            Span::styled(addr.to_string(), Style::default().fg(theme.text)),
        ];
        lines.push(build_line(i, "IP", v));
        i += 1;
    }

    if let Some(n) = fx.packages {
        let upd = fx.updates.unwrap_or(0);
        let mut v = vec![Span::styled(
            format!("{} pkgs", n),
            Style::default().fg(theme.text),
        )];
        if upd > 0 {
            v.push(Span::styled(" · ", Style::default().fg(theme.dim)));
            v.push(Span::styled(
                format!("{} updates", upd),
                Style::default().fg(theme.yellow),
            ));
        } else if fx.updates.is_some() && area.width >= 28 {
            v.push(Span::styled(" · ", Style::default().fg(theme.dim)));
            v.push(Span::styled("up to date", Style::default().fg(theme.dim)));
        }
        lines.push(build_line(i, "PKGS", v));
        i += 1;
    }

    if let Some((run_n, all_n)) = fx.docker {
        let v = vec![
            Span::styled(
                format!("{}/{}", run_n, all_n),
                Style::default().fg(theme.text),
            ),
            Span::styled(" running", Style::default().fg(theme.dim)),
        ];
        lines.push(build_line(i, "DOCKER", v));
        i += 1;
    }

    let cpu = cpu::snapshot();
    {
        let cores = cpu.cores.len().max(1) as f64;
        let (one, five, fifteen) = cpu.load;
        let col = if one > cores {
            theme.red
        } else if one > cores * 0.7 {
            theme.yellow
        } else {
            theme.accent
        };
        let mut v = vec![Span::styled(
            format!("{:.2} {:.2} {:.2}", one, five, fifteen),
            Style::default().fg(col),
        )];
        if area.width >= 32 {
            v.push(Span::styled(
                format!(" · {}t", cores as usize),
                Style::default().fg(theme.dim),
            ));
        }
        lines.push(build_line(i, "LOAD", v));
        i += 1;

        lines.push(build_line(
            i,
            "PROCS",
            vec![
                Span::styled(
                    crate::monitors::processes::count().to_string(),
                    Style::default().fg(theme.text),
                ),
                Span::styled(" active", Style::default().fg(theme.dim)),
            ],
        ));
        i += 1;
    }

    if let Some(b) = crate::monitors::system_info::read_battery_detail() {
        let col = if b.charging || b.pct > 20 {
            theme.accent
        } else if b.pct > 10 {
            theme.yellow
        } else {
            theme.red
        };
        let mut v = vec![Span::styled(
            format!("{}%{}", b.pct, if b.charging { " ⚡" } else { "" }),
            Style::default().fg(col),
        )];
        if let Some(w) = b.watts {
            if area.width >= 34 {
                v.push(Span::styled(
                    format!(" · {:.1}W", w),
                    Style::default().fg(theme.dim),
                ));
            }
        }
        if let Some(s) = b.eta_secs.filter(|s| *s > 0 && *s < 48 * 3600) {
            let eta = format!(" · {}h{:02}m", s / 3600, (s % 3600) / 60);
            v.push(Span::styled(eta, Style::default().fg(theme.dim)));
        }
        lines.push(build_line(i, "BAT", v));
        i += 1;
    }

    if let Some(max) = cpu.max_temp() {
        let limit = if area.width > 35 { 6 } else { 3 };
        let mut t_str = cpu
            .temps
            .iter()
            .take(limit)
            .map(|t| format!("{:.0}°", t))
            .collect::<Vec<_>>()
            .join(" ");
        if cpu.temps.len() > limit {
            t_str.push_str(" …");
        }
        lines.push(build_line(
            i,
            "TEMPS",
            vec![Span::styled(t_str, Style::default().fg(theme.temp(max)))],
        ));
    }

    if lines.is_empty() {
        return;
    }

    f.render_widget(Paragraph::new(lines), area);
}

pub fn active_row_ids() -> Vec<&'static str> {
    let fx = facts();
    let mut ids = Vec::new();
    if fx.wifi.is_some() {
        ids.push("wifi");
    }
    if fx.ip.is_some() {
        ids.push("ip");
    }
    if fx.packages.is_some() {
        ids.push("pkgs");
    }
    if fx.docker.is_some() {
        ids.push("docker");
    }
    ids.push("load");
    ids.push("procs");
    if crate::monitors::system_info::read_battery_detail().is_some() {
        ids.push("bat");
    }
    if crate::monitors::cpu::snapshot().max_temp().is_some() {
        ids.push("temps");
    }
    ids
}
