use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::Frame;

use crate::app::{App, PanelId};
use crate::monitors::{cpu, disk, gpu, memory, network, processes, system_info};
use crate::screens::{hit, panel, too_small, Hit};

const MIN: (u16, u16) = (80, 24);

/// Monitor layouts, cycled from the Esc menu (ui.monitor_layout).
pub const LAYOUTS: [&str; 4] = ["classic", "processes", "side", "graphs"];

/// Draw one Monitor panel into `area`.
fn draw(f: &mut Frame, app: &App, id: PanelId, area: Rect) {
    if area.width < 4 || area.height < 3 {
        return;
    }
    let theme = &app.theme;
    let focused = app.focused_panel == Some(id);
    hit(area, Hit::Panel(id));
    match id {
        PanelId::Cpu => {
            let inner = panel(f, area, "cpu", theme, focused);
            cpu::render(f, inner, theme, true);
        }
        PanelId::Memory => {
            let inner = panel(f, area, "memory", theme, focused);
            memory::render(f, inner, theme, true);
        }
        PanelId::Disk => {
            let inner = panel(f, area, "disk", theme, focused);
            disk::render(f, inner, theme, true);
        }
        PanelId::Network => {
            let inner = panel(f, area, "network", theme, focused);
            network::render(f, inner, theme, true);
        }
        PanelId::Gpu => {
            let inner = panel(f, area, "gpu", theme, focused);
            gpu::render(f, inner, theme, true);
        }
        PanelId::System => {
            let inner = panel(f, area, "system", theme, focused);
            system_info::render(f, inner, theme, &app.summary);
        }
        PanelId::Processes => {
            let ps = &app.panel_states;
            let title = format!(
                "processes · {}{}",
                processes::count(),
                if ps.process_tree_mode { " · tree" } else { "" }
            );
            let inner = panel(f, area, &title, theme, focused);
            processes::render(
                f,
                inner,
                theme,
                ps.process_scroll_offset,
                ps.process_sort_field,
                ps.process_sort_asc,
                &ps.process_search,
                ps.process_search_active,
                ps.process_tree_mode,
                &ps.process_collapsed,
                ps.process_selected_pid,
                ps.process_compact_cmd,
            );
        }
        _ => {}
    }
}

pub fn render(f: &mut Frame, area: Rect, app: &App) {
    if area.width < MIN.0 || area.height < MIN.1 {
        too_small(f, area, &app.theme, MIN);
        return;
    }
    let show_gpu = app.config.widgets.gpu;
    match app.config.ui.monitor_layout.as_str() {
        "processes" => {
            // btop's process view: a compact metrics strip, then the table.
            let [strip, table] =
                Layout::vertical([Constraint::Length(9), Constraint::Min(8)]).areas(area);
            let [c, m, n] = Layout::horizontal([
                Constraint::Percentage(46),
                Constraint::Percentage(27),
                Constraint::Percentage(27),
            ])
            .areas(strip);
            draw(f, app, PanelId::Cpu, c);
            draw(f, app, PanelId::Memory, m);
            draw(f, app, PanelId::Network, n);
            draw(f, app, PanelId::Processes, table);
        }
        "side" => {
            // Metrics stacked on the left, the table full height on the right.
            let [left, right] =
                Layout::horizontal([Constraint::Percentage(42), Constraint::Min(40)]).areas(area);
            let mut ids = vec![
                PanelId::Cpu,
                PanelId::Memory,
                PanelId::Network,
                PanelId::Disk,
            ];
            if show_gpu && left.height >= 40 {
                ids.push(PanelId::Gpu);
            }
            let mut cons = vec![Constraint::Fill(3)];
            cons.extend(std::iter::repeat_n(Constraint::Fill(2), ids.len() - 1));
            for (id, r) in ids
                .into_iter()
                .zip(Layout::vertical(cons).split(left).iter())
            {
                draw(f, app, id, *r);
            }
            draw(f, app, PanelId::Processes, right);
        }
        "graphs" => {
            // History first: a wide CPU graph, a row of the rest, then a
            // shorter table.
            let [top, mid, table] = Layout::vertical([
                Constraint::Percentage(38),
                Constraint::Percentage(27),
                Constraint::Min(8),
            ])
            .areas(area);
            draw(f, app, PanelId::Cpu, top);
            let mut ids = vec![PanelId::Memory, PanelId::Network, PanelId::Disk];
            if show_gpu {
                ids.push(PanelId::Gpu);
            }
            let n = ids.len() as u32;
            let cols = Layout::horizontal((0..n).map(|_| Constraint::Ratio(1, n))).split(mid);
            for (id, r) in ids.into_iter().zip(cols.iter()) {
                draw(f, app, id, *r);
            }
            draw(f, app, PanelId::Processes, table);
        }
        _ => classic(f, app, area, show_gpu),
    }
}

/// The default: a metrics band on top, the full process table below. The
/// only layout that honours the column/row resize keys.
fn classic(f: &mut Frame, app: &App, area: Rect, show_gpu: bool) {
    let adjusted = |c: Constraint, id: &str| -> Constraint {
        let adj = *app.panel_states.dash_vertical.get(id).unwrap_or(&0);
        let apply = |v: u16| -> u16 {
            if adj < 0 {
                v.saturating_sub(adj.unsigned_abs())
            } else {
                v.saturating_add(adj as u16)
            }
        };
        match c {
            Constraint::Percentage(p) => Constraint::Percentage(apply(p)),
            Constraint::Length(l) => Constraint::Length(apply(l)),
            Constraint::Min(m) => Constraint::Min(apply(m)),
            other => other,
        }
    };

    let adj_cpu = *app.panel_states.dash_vertical.get("cpu").unwrap_or(&0);
    let band_h = ((area.height * 2 / 5).clamp(14, 22) as i16 + adj_cpu).max(10) as u16;
    let [band, table] =
        Layout::vertical([Constraint::Length(band_h), Constraint::Min(8)]).areas(area);

    let r = app.panel_states.dash_ratios;
    let cols = Layout::horizontal([
        Constraint::Percentage(r[0]),
        Constraint::Percentage(r[1]),
        Constraint::Percentage(r[2]),
    ])
    .split(band);
    draw(f, app, PanelId::Cpu, cols[0]);

    let col1 = Layout::vertical([
        adjusted(Constraint::Percentage(50), "memory"),
        adjusted(Constraint::Percentage(50), "disk"),
    ])
    .split(cols[1]);
    draw(f, app, PanelId::Memory, col1[0]);
    draw(f, app, PanelId::Disk, col1[1]);

    let col2 = Layout::vertical([
        adjusted(
            if show_gpu {
                Constraint::Percentage(40)
            } else {
                Constraint::Min(4)
            },
            "network",
        ),
        if show_gpu {
            adjusted(Constraint::Percentage(40), "gpu")
        } else {
            Constraint::Length(0)
        },
        adjusted(Constraint::Length(5), "system"),
    ])
    .split(cols[2]);
    draw(f, app, PanelId::Network, col2[0]);
    if show_gpu {
        draw(f, app, PanelId::Gpu, col2[1]);
    }
    draw(f, app, PanelId::System, col2[2]);
    draw(f, app, PanelId::Processes, table);
}
