//! Full-screen painted Ambient scenes: synthwave sunset, aurora, lava lamp.
//!
//! Each scene paints a pixel canvas at double vertical resolution (one
//! terminal cell = two square-ish pixels, drawn as `▀` with fg = top pixel
//! and bg = bottom pixel). Colours come from the theme, so theme changes and
//! night dimming apply. Time is accumulated only while motion is enabled, so
//! the `o` key freezes a scene in place.

use std::f32::consts::TAU;
use std::sync::Mutex;
use std::time::Instant;

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};

use crate::theme::{blend, Theme};

/// Per-scene clock that only runs while motion is on.
struct Clock {
    t: f32,
    last: Option<Instant>,
}

static CLOCK: Mutex<Clock> = Mutex::new(Clock { t: 0.0, last: None });

fn tick(motion: bool) -> f32 {
    let mut c = CLOCK.lock().unwrap_or_else(|e| e.into_inner());
    let now = Instant::now();
    let dt = c
        .last
        .replace(now)
        .map_or(0.0, |l| now.duration_since(l).as_secs_f32().min(0.5));
    if motion {
        c.t += dt;
    }
    c.t
}

/// Cheap deterministic hash → 0..1, for star fields.
fn hash(x: u32, y: u32) -> f32 {
    let mut h = x.wrapping_mul(374_761_393) ^ y.wrapping_mul(668_265_263);
    h = (h ^ (h >> 13)).wrapping_mul(1_274_126_177);
    (h ^ (h >> 16)) as f32 / u32::MAX as f32
}

fn lerp(a: Color, b: Color, t: f32) -> Color {
    blend(a, b, t.clamp(0.0, 1.0))
}

/// Paint `px(x, y)` over `area` at 1×2 pixels per cell.
fn paint(buf: &mut Buffer, area: Rect, mut px: impl FnMut(u16, u16) -> Color) {
    for cy in 0..area.height {
        for cx in 0..area.width {
            let top = px(cx, cy * 2);
            let bottom = px(cx, cy * 2 + 1);
            if let Some(cell) = buf.cell_mut((area.x + cx, area.y + cy)) {
                cell.set_char('▀')
                    .set_style(Style::default().fg(top).bg(bottom));
            }
        }
    }
}

fn request(motion: bool, night: bool, fps: u32) {
    if motion {
        crate::anim::request(if night { fps.min(6) } else { fps });
    }
}

/// Starry night gradient shared by the sky scenes.
fn sky(theme: &Theme, x: u16, y: u16, h: f32, t: f32, glow: Color) -> Color {
    let fy = y as f32 / h;
    let base = lerp(
        blend(theme.bg, Color::Rgb(0, 0, 0), 0.35),
        glow,
        fy.powf(1.8) * 0.55,
    );
    let s = hash(x as u32, y as u32);
    if s > 0.985 && fy < 0.75 {
        // Twinkle: each star pulses on its own phase.
        let tw = 0.55 + 0.45 * (t * 1.7 + s * 97.0).sin();
        lerp(base, theme.text, tw * (1.0 - fy))
    } else {
        base
    }
}

/// Synthwave: a striped sun behind a mountain ridge, and a neon grid
/// rolling toward the viewer.
pub fn synthwave(buf: &mut Buffer, area: Rect, theme: &Theme, motion: bool, night: bool) {
    let t = tick(motion);
    request(motion, night, 15);
    let (w, h) = (area.width as f32, area.height as f32 * 2.0);
    let horizon = h * 0.58;
    let (sun_x, sun_y, sun_r) = (w * 0.5, horizon - h * 0.02, (h * 0.34).min(w * 0.16));
    let glow = blend(theme.secondary, theme.red, 0.35);
    let grid = theme.accent;
    let ground = blend(theme.bg, Color::Rgb(0, 0, 0), 0.5);
    paint(buf, area, |x, y| {
        let (fx, fy) = (x as f32 + 0.5, y as f32 + 0.5);
        if fy < horizon {
            // Sun: circle (cells are ~1:2, pixels here are ~1:1 so no aspect fix
            // beyond halving x distance).
            let dx = (fx - sun_x) * 0.5;
            let dy = fy - sun_y;
            let d = (dx * dx + dy * dy).sqrt();
            // Mountain ridge silhouette in front of the sun.
            let ridge = horizon
                - h * (0.05
                    + 0.035 * (fx * 0.045 + 1.3).sin()
                    + 0.02 * (fx * 0.13 + 0.4).sin()
                    + 0.01 * (fx * 0.31).sin())
                .max(0.0);
            if fy > ridge {
                return lerp(ground, theme.secondary, 0.18 + (fy - ridge) / h);
            }
            if d < sun_r {
                // v: 0 at the top of the sun, 1 at the bottom.
                let v = (fy - (sun_y - sun_r)) / (2.0 * sun_r);
                // Classic stripes on the lower half: gaps widen toward the
                // bottom and scroll slowly downward.
                let gap = ((v - 0.45) / 0.55).clamp(0.0, 1.0) * 0.6;
                let phase = (v * 7.0 + t * 0.25).rem_euclid(1.0);
                if gap == 0.0 || phase > gap {
                    return lerp(theme.yellow, blend(theme.red, theme.secondary, 0.4), v);
                }
            }
            let halo = (1.0 - (d - sun_r) / (sun_r * 1.2)).clamp(0.0, 1.0) * 0.35;
            return lerp(sky(theme, x, y, horizon, t, glow), theme.red, halo);
        }
        // Ground: perspective grid. A line is lit where it crosses this
        // pixel (the integer part changes between neighbours), so lines stay
        // one pixel wide at every depth instead of aliasing into dots.
        let depth = |py: f32| ((py - horizon) / (h - horizon)).max(0.004);
        let u = |py: f32| 1.6 / depth(py) + t * 1.3;
        let v = |px: f32, py: f32| (px - w * 0.5) / (w * 0.5) / depth(py) * 5.0;
        let d = depth(fy);
        let row = u(fy).floor() != u(fy + 1.0).floor();
        // Near the horizon the columns converge; skip them once they would
        // be closer than ~3 pixels apart.
        let spacing = w * 0.5 * d / 5.0;
        let col = spacing > 3.0 && v(fx, fy).floor() != v(fx + 1.0, fy).floor();
        let base = lerp(ground, glow, (1.0 - d).powf(3.0) * 0.45);
        if row || col {
            lerp(base, grid, 0.25 + d.powf(0.5) * 0.75)
        } else {
            base
        }
    });
}

/// Aurora: rippling curtains of light over a starry sky and a treeline.
pub fn aurora(buf: &mut Buffer, area: Rect, theme: &Theme, motion: bool, night: bool) {
    let t = tick(motion);
    request(motion, night, 12);
    let (w, h) = (area.width as f32, area.height as f32 * 2.0);
    let low = theme.green;
    let high = blend(theme.secondary, theme.accent, 0.4);
    let dark = blend(theme.bg, Color::Rgb(0, 0, 0), 0.55);
    // Three curtains at different heights and speeds.
    let curtains = [
        (0.52, 1.0, 0.11, 0.0),
        (0.62, 0.7, -0.07, 2.1),
        (0.42, 0.55, 0.05, 4.2),
    ];
    paint(buf, area, |x, y| {
        let (fx, fy) = (x as f32, y as f32);
        let nx = fx / w;
        // Treeline: jagged spikes along the bottom.
        let tree =
            h * (0.86 - 0.05 * (0.5 + 0.5 * (fx * 0.9).sin()) * hash(x as u32 / 2, 7).max(0.4));
        if fy > tree {
            return dark;
        }
        let mut c = sky(theme, x, y, h, t, blend(theme.bg, low, 0.12));
        for &(base, amp, speed, phase) in &curtains {
            let edge = h * base
                + h * 0.08 * amp * (nx * TAU * 1.3 + t * speed * 3.0 + phase).sin()
                + h * 0.04 * amp * (nx * TAU * 3.1 - t * speed * 5.0 + phase * 1.7).sin();
            // Light hangs above the lower edge and fades upward.
            let above = edge - fy;
            if above < -1.5 {
                continue;
            }
            let fall = (1.0 - above / (h * 0.40)).clamp(0.0, 1.0);
            let lip = if above < 0.0 { 1.0 + above / 1.5 } else { 1.0 };
            // Vertical ray striations that shimmer sideways.
            let rays = 0.55 + 0.45 * (fx * 0.8 + (t * 0.9 + phase).sin() * 3.0).sin();
            let a = (fall.powf(2.2) * lip * rays * amp).clamp(0.0, 1.0);
            let hue = lerp(low, high, (above / (h * 0.25)).clamp(0.0, 1.0));
            c = lerp(c, hue, a * 0.85);
        }
        c
    });
}

/// Lava lamp: metaballs drifting on slow Lissajous paths, merging and
/// splitting.
pub fn lava(buf: &mut Buffer, area: Rect, theme: &Theme, motion: bool, night: bool) {
    let t = tick(motion);
    request(motion, night, 12);
    let (w, h) = (area.width as f32, area.height as f32 * 2.0);
    let r = h.min(w * 0.5) * 0.10;
    let blobs: Vec<(f32, f32, f32)> = (0..7)
        .map(|i| {
            let k = i as f32;
            let x = w * (0.5 + 0.32 * (t * (0.07 + k * 0.013) + k * 1.9).sin());
            let y = h * (0.5 + 0.40 * (t * (0.05 + k * 0.011) + k * 2.7).sin());
            (x, y, r * (0.75 + 0.1 * (k * 1.7).sin().abs()))
        })
        .collect();
    let glass = blend(theme.bg, Color::Rgb(0, 0, 0), 0.3);
    let bg_top = blend(glass, theme.secondary, 0.08);
    let bg_bot = blend(glass, theme.red, 0.18);
    paint(buf, area, |x, y| {
        let (fx, fy) = (x as f32 * 0.5, y as f32);
        let field: f32 = blobs
            .iter()
            .map(|&(bx, by, br)| {
                let (dx, dy) = (fx - bx * 0.5, fy - by);
                br * br / (dx * dx + dy * dy + 1.0)
            })
            .sum();
        let back = lerp(bg_top, bg_bot, fy / h);
        if field > 1.0 {
            // Hot core → cooler rim.
            let heat = ((field - 1.0) / 2.5).clamp(0.0, 1.0);
            lerp(theme.red, theme.yellow, heat)
        } else {
            // Soft glow around each blob.
            lerp(back, theme.red, (field - 0.45).max(0.0) / 0.55 * 0.45)
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scenes_fill_the_area_without_panicking_at_odd_sizes() {
        let t = Theme::dark();
        for (w, h) in [(1, 1), (3, 2), (60, 20), (200, 50)] {
            let area = Rect::new(0, 0, w, h);
            for scene in [synthwave, aurora, lava] {
                let mut buf = Buffer::empty(area);
                scene(&mut buf, area, &t, true, false);
                assert!((0..w).all(|x| (0..h).all(|y| buf[(x, y)].symbol() == "▀")));
            }
        }
    }

    #[test]
    fn hash_is_in_unit_range_and_varies() {
        let v: Vec<f32> = (0..100).map(|i| hash(i, i * 3)).collect();
        assert!(v.iter().all(|x| (0.0..=1.0).contains(x)));
        assert!(v.windows(2).any(|p| (p[0] - p[1]).abs() > 0.1));
    }
}
