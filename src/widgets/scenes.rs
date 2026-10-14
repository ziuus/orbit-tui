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

/// Synthwave: a glowing striped sun behind multi-layered parallax mountain ridges,
/// an audio-reactive neon grid rolling toward the viewer, and atmospheric horizon glow.
pub fn synthwave(buf: &mut Buffer, area: Rect, theme: &Theme, motion: bool, night: bool) {
    let t = tick(motion);
    request(motion, night, 15);
    let (w, h) = (area.width as f32, area.height as f32 * 2.0);
    let horizon = h * 0.58;
    let (sun_x, sun_y, sun_r) = (w * 0.5, horizon - h * 0.04, (h * 0.38).min(w * 0.18));
    let glow = blend(theme.secondary, theme.red, 0.4);
    let grid = theme.accent;
    let ground = blend(theme.bg, Color::Rgb(0, 0, 0), 0.6);
    let music = crate::widgets::music_viz::energy();

    paint(buf, area, |x, y| {
        let (fx, fy) = (x as f32 + 0.5, y as f32 + 0.5);
        if fy < horizon {
            let dx = (fx - sun_x) * 0.5;
            let dy = fy - sun_y;
            let d = (dx * dx + dy * dy).sqrt();

            // Layered mountain ridges: background distant range + foreground jagged peak
            let ridge_far = horizon
                - h * (0.09 + 0.04 * (fx * 0.03 + 2.1).sin() + 0.025 * (fx * 0.08 + 1.2).sin());
            let ridge_near = horizon
                - h * (0.045
                    + 0.035 * (fx * 0.06 + 0.8).sin()
                    + 0.02 * (fx * 0.16 + 0.4).sin()
                    + 0.01 * (fx * 0.38).sin())
                .max(0.0);

            if fy > ridge_near {
                return lerp(ground, theme.secondary, 0.22 + (fy - ridge_near) / h);
            }
            if fy > ridge_far {
                return lerp(
                    ground,
                    blend(theme.bg, theme.secondary, 0.35),
                    0.15 + (fy - ridge_far) / h,
                );
            }

            if d < sun_r {
                let v = (fy - (sun_y - sun_r)) / (2.0 * sun_r);
                let gap = ((v - 0.40) / 0.60).clamp(0.0, 1.0) * 0.65;
                let phase = (v * 7.5 + t * 0.28).rem_euclid(1.0);
                if gap == 0.0 || phase > gap {
                    let sun_core = lerp(theme.yellow, blend(theme.red, theme.secondary, 0.5), v);
                    return lerp(sun_core, theme.text, (0.3 - v).max(0.0));
                }
            }
            let halo = (1.0 - (d - sun_r) / (sun_r * 1.5))
                .clamp(0.0, 1.0)
                .powf(1.6)
                * 0.45;
            let sky_col = sky(theme, x, y, horizon, t, glow);
            return lerp(sky_col, theme.red, halo);
        }

        // Perspective ground with audio-reactive neon grid
        let depth = |py: f32| ((py - horizon) / (h - horizon)).max(0.003);
        let speed = 1.3 + (music * 1.8);
        let u = |py: f32| 1.8 / depth(py) + t * speed;
        let v = |px: f32, py: f32| (px - w * 0.5) / (w * 0.5) / depth(py) * 5.0;
        let d = depth(fy);
        let row = u(fy).floor() != u(fy + 1.0).floor();
        let spacing = w * 0.5 * d / 5.0;
        let col = spacing > 2.8 && v(fx, fy).floor() != v(fx + 1.0, fy).floor();
        let base = lerp(ground, glow, (1.0 - d).powf(2.5) * 0.5);

        if row || col {
            let grid_pulse = lerp(grid, theme.yellow, music * 0.7);
            lerp(base, grid_pulse, 0.3 + d.powf(0.4) * 0.7)
        } else {
            base
        }
    });
}

/// Aurora: rippling curtains of multi-frequency northern lights over a starry sky,
/// reactive to system and audio energy.
pub fn aurora(buf: &mut Buffer, area: Rect, theme: &Theme, motion: bool, night: bool) {
    let t = tick(motion);
    request(motion, night, 14);
    let (w, h) = (area.width as f32, area.height as f32 * 2.0);
    let low = theme.green;
    let high = blend(theme.secondary, theme.accent, 0.45);
    let dark = blend(theme.bg, Color::Rgb(0, 0, 0), 0.65);
    let music = crate::widgets::music_viz::energy();

    let curtains = [
        (0.52, 1.1, 0.11 + music * 0.08, 0.0),
        (0.62, 0.8, -0.07 - music * 0.06, 2.1),
        (0.40, 0.65, 0.06 + music * 0.05, 4.2),
    ];
    paint(buf, area, |x, y| {
        let (fx, fy) = (x as f32, y as f32);
        let nx = fx / w;
        let tree =
            h * (0.86 - 0.05 * (0.5 + 0.5 * (fx * 0.9).sin()) * hash(x as u32 / 2, 7).max(0.4));
        if fy > tree {
            return dark;
        }
        let mut c = sky(theme, x, y, h, t, blend(theme.bg, low, 0.15));
        for &(base, amp, speed, phase) in &curtains {
            let edge = h * base
                + h * 0.09 * amp * (nx * TAU * 1.3 + t * speed * 3.0 + phase).sin()
                + h * 0.045 * amp * (nx * TAU * 3.1 - t * speed * 5.0 + phase * 1.7).sin();
            let above = edge - fy;
            if above < -1.5 {
                continue;
            }
            let fall = (1.0 - above / (h * 0.42)).clamp(0.0, 1.0);
            let lip = if above < 0.0 { 1.0 + above / 1.5 } else { 1.0 };
            let rays = 0.55 + 0.45 * (fx * 0.85 + (t * 1.0 + phase).sin() * 3.2).sin();
            let a = (fall.powf(2.0) * lip * rays * amp).clamp(0.0, 1.0);
            let hue = lerp(low, high, (above / (h * 0.28)).clamp(0.0, 1.0));
            c = lerp(c, hue, a * 0.88);
        }
        c
    });
}

/// Lava lamp: luminous metaballs drifting on smooth paths, merging and
/// splitting with high-heat core coloration and refraction rings.
pub fn lava(buf: &mut Buffer, area: Rect, theme: &Theme, motion: bool, night: bool) {
    let t = tick(motion);
    request(motion, night, 14);
    let (w, h) = (area.width as f32, area.height as f32 * 2.0);
    let r = h.min(w * 0.5) * 0.11;
    let blobs: Vec<(f32, f32, f32)> = (0..7)
        .map(|i| {
            let k = i as f32;
            let x = w * (0.5 + 0.32 * (t * (0.07 + k * 0.013) + k * 1.9).sin());
            let y = h * (0.5 + 0.40 * (t * (0.05 + k * 0.011) + k * 2.7).sin());
            (x, y, r * (0.80 + 0.12 * (k * 1.7).sin().abs()))
        })
        .collect();
    let glass = blend(theme.bg, Color::Rgb(2, 2, 8), 0.4);
    let bg_top = blend(glass, theme.secondary, 0.12);
    let bg_bot = blend(glass, theme.red, 0.22);
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
            let heat = ((field - 1.0) / 2.2).clamp(0.0, 1.0);
            let core = lerp(theme.red, theme.yellow, heat);
            if heat > 0.7 {
                lerp(core, theme.text, (heat - 0.7) / 0.3)
            } else {
                core
            }
        } else {
            let rim = (field - 0.40).max(0.0) / 0.60;
            lerp(back, theme.secondary, rim.powf(1.8) * 0.5)
        }
    });
}

/// Cyberpunk: a rain-drenched megacity skyline with neon billboards, glowing multi-story
/// skyscraper windows, volumetric holographic beacons, and wet asphalt reflections.
pub fn cyberpunk(buf: &mut Buffer, area: Rect, theme: &Theme, motion: bool, night: bool) {
    let t = tick(motion);
    request(motion, night, 15);
    let (w, h) = (area.width as f32, area.height as f32 * 2.0);
    let horizon = h * 0.70;
    let sky_bg = blend(theme.bg, Color::Rgb(6, 7, 18), 0.7);
    let fog = blend(theme.secondary, theme.red, 0.45);
    let ground = blend(theme.bg, Color::Rgb(0, 0, 0), 0.75);

    paint(buf, area, |x, y| {
        let (fx, fy) = (x as f32, y as f32);
        if fy < horizon {
            let norm_y = fy / horizon;
            let mut c = lerp(sky_bg, fog, norm_y.powf(2.2) * 0.42);

            // Layer 1: Background distant monolith towers (width = 9)
            let bg_col_idx = (fx / 9.0).floor() as u32;
            let bg_col_h = horizon * (0.50 + 0.42 * hash(bg_col_idx, 203));
            let bg_top = horizon - bg_col_h;
            if fy > bg_top {
                c = lerp(sky_bg, theme.surface, 0.35);
            }

            // Layer 2: Foreground skyscrapers with windows and neon accents (width = 6)
            let col_idx = (fx / 6.0).floor() as u32;
            let col_h = horizon * (0.30 + 0.55 * hash(col_idx, 101));
            let b_top = horizon - col_h;

            if fy > b_top {
                let b_edge = (fx % 6.0) < 0.7;
                if b_edge {
                    return lerp(c, theme.dim, 0.4);
                }
                let win_x = (fx % 6.0) > 1.4 && (fx % 6.0) < 5.2;
                let win_y = (fy % 3.0) > 0.8 && (fy % 3.0) < 2.5;
                let win_hash = hash(col_idx, (fy / 3.0).floor() as u32);
                let lit = win_hash > 0.40;
                if win_x && win_y && lit {
                    let win_col = if win_hash > 0.85 {
                        theme.accent
                    } else if win_hash > 0.65 {
                        theme.yellow
                    } else if win_hash > 0.50 {
                        theme.secondary
                    } else {
                        theme.green
                    };
                    let flicker = 0.82 + 0.18 * (t * 2.5 + win_hash * 47.0).sin();
                    return lerp(theme.bg, win_col, flicker);
                }
                // Tower structural body
                return lerp(ground, theme.surface, 0.55);
            }

            // Volumetric holographic spotlight sweeping across the sky
            let beacon_x = w * (0.5 + 0.3 * (t * 0.4).sin());
            let beam_dist = (fx - beacon_x).abs();
            if beam_dist < 12.0 {
                let beam_intensity = (1.0 - beam_dist / 12.0) * (1.0 - norm_y);
                c = lerp(c, theme.accent, beam_intensity * 0.25);
            }

            // Neon billboard sign in the midground
            let neon_x = w * 0.70;
            let neon_y = horizon * 0.42;
            let dx = fx - neon_x;
            let dy = fy - neon_y;
            let dist_sq = dx * dx + dy * dy;
            if dist_sq < 320.0 {
                let glow = (1.0 - (dist_sq / 320.0).sqrt()).powf(1.6);
                let pulse = 0.85 + 0.15 * (t * 4.0).sin();
                c = lerp(c, theme.red, glow * 0.85 * pulse);
            }
            c
        } else {
            // Wet asphalt reflection with ripple distortion
            let depth = (fy - horizon) / (h - horizon);
            let ripple = (fx * 0.18 + t * 4.5).sin() * 0.025 * depth;
            let reflected_y =
                (horizon - (fy - horizon) * 0.82 + ripple * h).clamp(0.0, horizon - 1.0);
            let ref_col_idx = (fx / 6.0).floor() as u32;
            let ref_h = horizon * (0.30 + 0.55 * hash(ref_col_idx, 101));
            let mut ref_col = ground;
            if reflected_y > (horizon - ref_h) {
                ref_col = lerp(theme.surface, theme.accent, 0.35);
            }
            let sheen = (1.0 - depth).powf(1.6) * 0.72;
            lerp(ground, ref_col, sheen)
        }
    });
}

/// Black hole: a spinning relativistic accretion disk with gravitational lensing
/// and Doppler beaming around a dark event horizon.
pub fn blackhole(buf: &mut Buffer, area: Rect, theme: &Theme, motion: bool, night: bool) {
    let t = tick(motion);
    request(motion, night, 14);
    let (w, h) = (area.width as f32, area.height as f32 * 2.0);
    let cx = w * 0.5;
    let cy = h * 0.5;
    let r_event = (h * 0.20).min(w * 0.10);
    let r_disk = r_event * 2.8;

    paint(buf, area, |x, y| {
        let fx = x as f32 + 0.5;
        let fy = y as f32 + 0.5;

        let dx = (fx - cx) * 0.55;
        let dy = fy - cy;
        let dist = (dx * dx + dy * dy).sqrt();

        let warp = if dist > r_event {
            (r_event / dist).powf(2.0) * 0.4
        } else {
            0.0
        };

        if dist < r_event {
            let photon_ring = (dist / r_event).powf(6.0);
            return lerp(Color::Rgb(0, 0, 0), theme.accent, photon_ring * 0.3);
        }

        let disk_y = dy * 2.2 + dx * 0.35;
        let disk_r = (dx * dx + disk_y * disk_y).sqrt();

        let in_disk = disk_r > (r_event * 1.05) && disk_r < r_disk;
        if in_disk {
            let v = (disk_r - r_event * 1.05) / (r_disk - r_event * 1.05);
            let doppler = (-dx / r_disk).clamp(-0.8, 0.8) + 0.8;
            let angle = (disk_y).atan2(dx);
            let swirl = 0.5 + 0.5 * (angle * 4.0 - t * 2.2 + (1.0 - v) * 5.0).sin();
            let intensity = (1.0 - v).powf(1.4) * (0.6 + 0.4 * swirl) * doppler;

            let disk_col = if intensity > 0.8 {
                lerp(theme.yellow, theme.text, (intensity - 0.8) / 0.5)
            } else if intensity > 0.4 {
                lerp(theme.red, theme.yellow, (intensity - 0.4) / 0.4)
            } else {
                lerp(blend(theme.bg, theme.red, 0.3), theme.red, intensity / 0.4)
            };
            return disk_col;
        }

        let lx = (fx + warp * dx * 10.0) as u16;
        let ly = (fy + warp * dy * 10.0) as u16;
        let bg_c = sky(theme, lx, ly, h, t, blend(theme.bg, theme.secondary, 0.08));

        let einstein_dist = (dist - r_event * 1.5).abs();
        let glow = (1.0 - einstein_dist / (r_event * 0.8))
            .clamp(0.0, 1.0)
            .powf(2.5);
        lerp(bg_c, theme.accent, glow * 0.35)
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
            for scene in [synthwave, aurora, lava, cyberpunk, blackhole] {
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
