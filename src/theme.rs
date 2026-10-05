#![allow(dead_code)]
use ratatui::style::Color;
use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::LazyLock;

pub const BUILTIN_THEMES: [&str; 30] = [
    "dark",
    "catppuccin",
    "tokyo-night",
    "tokyo-night-storm",
    "nord",
    "nordic-frost",
    "gruvbox",
    "dracula",
    "cyberpunk",
    "rose-pine",
    "everforest",
    "kanagawa",
    "one-dark",
    "monokai",
    "synthwave",
    "outrun-sunset",
    "oxocarbon",
    "github-dark",
    "ayu-mirage",
    "phosphor",
    "matrix-green",
    "amber",
    "zenburn",
    "solarized-dark",
    "light",
    "solarized-light",
    "papercolor-light",
    "catppuccin-latte",
    "catppuccin-frappe",
    "gruvbox-light",
];

/// Palettes defined as data: name, then bg, accent, secondary, surface,
/// text, dim, green, yellow, red.
#[rustfmt::skip]
const PALETTES: &[(&str, [u32; 9])] = &[
    ("cyberpunk",         [0x08080a, 0xfcee0a, 0x00f0ff, 0x18181f, 0xf0f0f5, 0x717182, 0x00ff9f, 0xfcee0a, 0xff003c]),
    ("solarized-dark",   [0x002b36, 0x268bd2, 0x2aa198, 0x073642, 0x93a1a1, 0x586e75, 0x859900, 0xb58900, 0xdc322f]),
    ("tokyo-night-storm", [0x24283b, 0x7aa2f7, 0xbb9af7, 0x292e42, 0xc0caf5, 0x565f89, 0x9ece6a, 0xe0af68, 0xf7768e]),
    ("catppuccin-frappe", [0x303446, 0x8caaee, 0xf4b8e4, 0x414559, 0xc6d0f5, 0x737994, 0xa6d189, 0xe5c890, 0xe78284]),
    ("nordic-frost",      [0x0f141c, 0x88c0d0, 0x81a1c1, 0x1e2736, 0xe5e9f0, 0x4c566a, 0xa3be8c, 0xebcb8b, 0xbf616a]),
    ("matrix-green",      [0x020a04, 0x00ff41, 0x008f11, 0x0b1c0e, 0xa6ffb8, 0x1f5c2b, 0x00ff41, 0x96f53d, 0xff3b30]),
    ("outrun-sunset",     [0x140a24, 0xff007f, 0x00f5d4, 0x221338, 0xf5e6ff, 0x7f6596, 0x00f5d4, 0xffbe0b, 0xff0055]),
    ("zenburn",           [0x3f3f3f, 0xdca3a3, 0x93e0e3, 0x4f4f4f, 0xdcdccc, 0x7f9f7f, 0x7f9f7f, 0xdfaf8f, 0xcc9393]),
    ("papercolor-light",  [0xeeeeee, 0x0087af, 0xaf005f, 0xe4e4e4, 0x444444, 0x878787, 0x5f8700, 0xd75f00, 0xd70000]),
    ("rose-pine",        [0x191724, 0xebbcba, 0xc4a7e7, 0x26233a, 0xe0def4, 0x6e6a86, 0x9ccfd8, 0xf6c177, 0xeb6f92]),
    ("everforest",       [0x2d353b, 0xa7c080, 0x7fbbb3, 0x3d484d, 0xd3c6aa, 0x859289, 0xa7c080, 0xdbbc7f, 0xe67e80]),
    ("kanagawa",         [0x1f1f28, 0x7e9cd8, 0x957fb8, 0x2a2a37, 0xdcd7ba, 0x727169, 0x98bb6c, 0xe6c384, 0xe46876]),
    ("one-dark",         [0x282c34, 0x61afef, 0xc678dd, 0x3e4451, 0xabb2bf, 0x5c6370, 0x98c379, 0xe5c07b, 0xe06c75]),
    ("monokai",          [0x272822, 0xa6e22e, 0x66d9ef, 0x3e3d32, 0xf8f8f2, 0x75715e, 0xa6e22e, 0xe6db74, 0xf92672]),
    ("synthwave",        [0x241b2f, 0xff7edb, 0x36f9f6, 0x34294f, 0xf4eeff, 0x848bbd, 0x72f1b8, 0xfede5d, 0xfe4450]),
    ("oxocarbon",        [0x161616, 0x78a9ff, 0xbe95ff, 0x262626, 0xf2f4f8, 0x6f6f6f, 0x42be65, 0xffe97b, 0xee5396]),
    ("github-dark",      [0x0d1117, 0x58a6ff, 0xbc8cff, 0x21262d, 0xc9d1d9, 0x6e7681, 0x3fb950, 0xd29922, 0xf85149]),
    ("ayu-mirage",       [0x1f2430, 0xffcc66, 0x5ccfe6, 0x2d3442, 0xcccac2, 0x707a8c, 0xd5ff80, 0xffd173, 0xf28779]),
    ("phosphor",         [0x050a05, 0x33ff66, 0x1fbf4d, 0x0f1f12, 0xb8ffc8, 0x2f6b3c, 0x33ff66, 0xd6ff5c, 0xff5c5c]),
    ("amber",            [0x0c0802, 0xffb000, 0xff8c1a, 0x1f1606, 0xffd98a, 0x7a5a1e, 0xc8d65a, 0xffb000, 0xff5a36]),
    ("catppuccin-latte", [0xeff1f5, 0x1e66f5, 0x8839ef, 0xdce0e8, 0x4c4f69, 0x8c8fa1, 0x40a02b, 0xdf8e1d, 0xd20f39]),
    ("gruvbox-light",    [0xfbf1c7, 0x076678, 0x8f3f71, 0xebdbb2, 0x3c3836, 0x928374, 0x79740e, 0xb57614, 0x9d0006]),
];

fn hex(c: u32) -> Color {
    Color::Rgb((c >> 16) as u8, (c >> 8) as u8, c as u8)
}

fn palette(name: &str) -> Option<Theme> {
    let (_, c) = PALETTES.iter().find(|(n, _)| *n == name)?;
    Some(Theme {
        bg: hex(c[0]),
        accent: hex(c[1]),
        secondary: hex(c[2]),
        surface: hex(c[3]),
        text: hex(c[4]),
        dim: hex(c[5]),
        green: hex(c[6]),
        yellow: hex(c[7]),
        red: hex(c[8]),
    })
}

#[derive(Deserialize)]
struct CustomTheme {
    bg: String,
    accent: String,
    secondary: String,
    surface: String,
    text: String,
    dim: String,
    green: String,
    yellow: String,
    red: String,
}

fn parse_hex(hex: &str) -> Option<Color> {
    let hex = hex.trim_start_matches('#');
    if hex.len() == 6 {
        let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
        let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
        let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
        Some(rgb(r, g, b))
    } else {
        None
    }
}

impl CustomTheme {
    fn to_theme(&self) -> Option<Theme> {
        Some(Theme {
            bg: parse_hex(&self.bg)?,
            accent: parse_hex(&self.accent)?,
            secondary: parse_hex(&self.secondary)?,
            surface: parse_hex(&self.surface)?,
            text: parse_hex(&self.text)?,
            dim: parse_hex(&self.dim)?,
            green: parse_hex(&self.green)?,
            yellow: parse_hex(&self.yellow)?,
            red: parse_hex(&self.red)?,
        })
    }
}

static CUSTOM_THEMES: LazyLock<HashMap<String, Theme>> = LazyLock::new(|| {
    let mut map = HashMap::new();
    let mut dir = if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME") {
        PathBuf::from(xdg)
    } else if let Ok(home) = std::env::var("HOME") {
        let mut p = PathBuf::from(home);
        p.push(".config");
        p
    } else {
        PathBuf::from(".config")
    };
    dir.push("orbit");
    dir.push("themes");

    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("toml") {
                if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                    if let Ok(content) = fs::read_to_string(&path) {
                        if let Ok(ct) = toml::from_str::<CustomTheme>(&content) {
                            if let Some(theme) = ct.to_theme() {
                                map.insert(stem.to_string(), theme);
                            }
                        }
                    }
                }
            }
        }
    }
    map
});

pub fn theme_names() -> Vec<String> {
    let mut names: Vec<String> = BUILTIN_THEMES.iter().map(|s| s.to_string()).collect();
    let mut custom: Vec<String> = CUSTOM_THEMES.keys().cloned().collect();
    custom.sort();
    names.extend(custom);
    names
}

#[derive(Clone)]
pub struct Theme {
    pub bg: Color,
    pub accent: Color,
    pub secondary: Color,
    pub surface: Color,
    pub text: Color,
    pub dim: Color,
    pub green: Color,
    pub yellow: Color,
    pub red: Color,
}

const fn rgb(r: u8, g: u8, b: u8) -> Color {
    Color::Rgb(r, g, b)
}

/// Linear blend from `a` to `b`, `t` in 0..1.
///
/// Every palette here is truecolor, so this is exact in practice. An indexed
/// or named colour has no meaningful midpoint, so it snaps at the halfway
/// point rather than inventing one — a wrong-but-stable colour beats a
/// gradient that flickers between two palette entries.
pub fn blend(a: Color, b: Color, t: f32) -> Color {
    let t = t.clamp(0.0, 1.0);
    match (a, b) {
        (Color::Rgb(ar, ag, ab), Color::Rgb(br, bg, bb)) => {
            let mix = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round() as u8;
            Color::Rgb(mix(ar, br), mix(ag, bg), mix(ab, bb))
        }
        _ if t < 0.5 => a,
        _ => b,
    }
}

impl Theme {
    pub fn from_name(name: &str) -> Self {
        if let Some(custom) = CUSTOM_THEMES.get(name) {
            return custom.clone();
        }
        if let Some(t) = palette(name) {
            return t;
        }
        match name {
            "light" => Self::light(),
            "dracula" => Self::dracula(),
            "solarized-light" => Self::solarized_light(),
            "catppuccin" | "catppuccin-mocha" => Self::catppuccin(),
            "tokyo-night" => Self::tokyo_night(),
            "nord" => Self::nord(),
            "gruvbox" => Self::gruvbox(),
            _ => Self::dark(),
        }
    }

    /// Night variant: every colour pulled toward the background so a screen
    /// left on in a dark room glows less. `bg` itself is unchanged.
    pub fn dimmed(&self) -> Self {
        const FG: f32 = 0.45;
        const QUIET: f32 = 0.25;
        let d = |c: Color, t: f32| blend(c, self.bg, t);
        Self {
            bg: self.bg,
            accent: d(self.accent, FG),
            secondary: d(self.secondary, FG),
            surface: d(self.surface, QUIET),
            text: d(self.text, FG),
            dim: d(self.dim, QUIET),
            green: d(self.green, FG),
            yellow: d(self.yellow, FG),
            red: d(self.red, FG),
        }
    }

    /// High-contrast / outdoor variant: pushes background to pure black or
    /// white and ensures maximum foreground readability.
    pub fn high_contrast(&self) -> Self {
        if self.is_light() {
            Self {
                bg: rgb(255, 255, 255),
                surface: rgb(235, 235, 235),
                text: rgb(0, 0, 0),
                dim: rgb(70, 70, 70),
                accent: rgb(0, 75, 180),
                secondary: rgb(110, 30, 160),
                green: rgb(0, 130, 40),
                yellow: rgb(180, 100, 0),
                red: rgb(200, 15, 15),
            }
        } else {
            Self {
                bg: rgb(0, 0, 0),
                surface: rgb(20, 20, 24),
                text: rgb(255, 255, 255),
                dim: rgb(165, 165, 180),
                accent: rgb(80, 220, 255),
                secondary: rgb(230, 140, 255),
                green: rgb(40, 255, 120),
                yellow: rgb(255, 230, 40),
                red: rgb(255, 65, 65),
            }
        }
    }

    /// Name that follows `current` in the cycle order.
    pub fn next_name(current: &str) -> String {
        let names = theme_names();
        let idx = names.iter().position(|n| n == current).unwrap_or(0);
        names[(idx + 1) % names.len()].clone()
    }

    /// Name before `current` in the cycle order.
    pub fn prev_name(current: &str) -> String {
        let names = theme_names();
        let idx = names.iter().position(|n| n == current).unwrap_or(0);
        names[(idx + names.len() - 1) % names.len()].clone()
    }

    pub fn dark() -> Self {
        Self {
            bg: rgb(10, 10, 15),
            accent: rgb(120, 220, 150),
            secondary: rgb(100, 120, 200),
            surface: rgb(22, 22, 32),
            text: rgb(220, 220, 230),
            dim: rgb(80, 80, 95),
            green: rgb(80, 200, 120),
            yellow: rgb(220, 200, 60),
            red: rgb(220, 80, 80),
        }
    }

    pub fn catppuccin() -> Self {
        Self {
            bg: rgb(30, 30, 46),
            accent: rgb(203, 166, 247),
            secondary: rgb(137, 180, 250),
            surface: rgb(49, 50, 68),
            text: rgb(205, 214, 244),
            dim: rgb(108, 112, 134),
            green: rgb(166, 227, 161),
            yellow: rgb(249, 226, 175),
            red: rgb(243, 139, 168),
        }
    }

    pub fn tokyo_night() -> Self {
        Self {
            bg: rgb(26, 27, 38),
            accent: rgb(122, 162, 247),
            secondary: rgb(187, 154, 247),
            surface: rgb(41, 46, 66),
            text: rgb(192, 202, 245),
            dim: rgb(86, 95, 137),
            green: rgb(158, 206, 106),
            yellow: rgb(224, 175, 104),
            red: rgb(247, 118, 142),
        }
    }

    pub fn nord() -> Self {
        Self {
            bg: rgb(46, 52, 64),
            accent: rgb(136, 192, 208),
            secondary: rgb(129, 161, 193),
            surface: rgb(59, 66, 82),
            text: rgb(236, 239, 244),
            dim: rgb(97, 110, 136),
            green: rgb(163, 190, 140),
            yellow: rgb(235, 203, 139),
            red: rgb(191, 97, 106),
        }
    }

    pub fn gruvbox() -> Self {
        Self {
            bg: rgb(40, 40, 40),
            accent: rgb(250, 189, 47),
            secondary: rgb(131, 165, 152),
            surface: rgb(60, 56, 54),
            text: rgb(235, 219, 178),
            dim: rgb(124, 111, 100),
            green: rgb(184, 187, 38),
            yellow: rgb(254, 128, 25),
            red: rgb(251, 73, 52),
        }
    }

    /// Colour for resting panel borders: the surface colour, lifted toward
    /// `dim` when a palette's surface is too close to its background to
    /// read as a line.
    pub fn border(&self) -> Color {
        let lum = |c: Color| match c {
            Color::Rgb(r, g, b) => (r as i32 * 3 + g as i32 * 6 + b as i32) / 10,
            _ => 0,
        };
        if (lum(self.surface) - lum(self.bg)).abs() < 22 {
            blend(self.surface, self.dim, 0.45)
        } else {
            self.surface
        }
    }

    pub fn is_light(&self) -> bool {
        // A simple heuristic for truecolor: if background RGB average > 128, it's light.
        // Wait, we know the exact light themes! But a heuristic handles custom themes too.
        let Color::Rgb(r, g, b) = self.bg else {
            return false;
        };
        (r as u16 + g as u16 + b as u16) / 3 > 128
    }

    pub fn dracula() -> Self {
        Self {
            bg: rgb(40, 42, 54),
            accent: rgb(189, 147, 249),
            secondary: rgb(139, 233, 253),
            surface: rgb(68, 71, 90),
            text: rgb(248, 248, 242),
            dim: rgb(98, 114, 164),
            green: rgb(80, 250, 123),
            yellow: rgb(255, 203, 107),
            red: rgb(255, 121, 198),
        }
    }

    pub fn light() -> Self {
        Self {
            bg: rgb(245, 245, 240),
            accent: rgb(0, 100, 200),
            secondary: rgb(60, 80, 180),
            surface: rgb(228, 228, 222),
            text: rgb(20, 20, 30),
            dim: rgb(140, 140, 145),
            green: rgb(40, 160, 80),
            yellow: rgb(180, 140, 20),
            red: rgb(200, 50, 50),
        }
    }

    pub fn solarized_light() -> Self {
        Self {
            bg: rgb(253, 246, 227),
            accent: rgb(42, 161, 152),
            secondary: rgb(108, 113, 196),
            surface: rgb(238, 232, 213),
            text: rgb(88, 110, 117),
            dim: rgb(147, 161, 161),
            green: rgb(133, 153, 0),
            yellow: rgb(181, 137, 0),
            red: rgb(220, 50, 47),
        }
    }

    /// Continuous accent → yellow → red ramp, `t` in 0..1. The gradient
    /// counterpart to `usage()`: a number badge wants three unambiguous states,
    /// a swept dial or bar wants no visible seam between them.
    pub fn usage_ramp(&self, t: f64) -> Color {
        let t = t.clamp(0.0, 1.0) as f32;
        if t < 0.5 {
            blend(self.accent, self.yellow, t * 2.0)
        } else {
            blend(self.yellow, self.red, (t - 0.5) * 2.0)
        }
    }

    /// Smooth green → yellow → red gradient by load percentage.
    /// Shared by every "usage %" readout so colours are intensely load-reactive.
    pub fn usage(&self, pct: f64) -> Color {
        self.usage_ramp(pct / 100.0)
    }

    /// CPU temperature colour, relative to the chip's critical limit.
    pub fn temp(&self, c: f64) -> Color {
        let crit = *crate::monitors::cpu::TEMP_CRIT;
        if c >= crit - 5.0 {
            self.red
        } else if c >= crit - 15.0 {
            self.yellow
        } else {
            self.accent
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blend_hits_both_ends_exactly() {
        let a = rgb(0, 0, 0);
        let b = rgb(200, 100, 50);
        assert_eq!(blend(a, b, 0.0), a);
        assert_eq!(blend(a, b, 1.0), b);
        assert_eq!(blend(a, b, 0.5), rgb(100, 50, 25));
    }

    #[test]
    fn blend_clamps_out_of_range_t() {
        let a = rgb(10, 10, 10);
        let b = rgb(20, 20, 20);
        assert_eq!(blend(a, b, -5.0), a);
        assert_eq!(blend(a, b, 5.0), b);
    }

    #[test]
    fn blend_snaps_for_non_rgb() {
        // No meaningful midpoint between palette entries — must not panic or
        // invent one.
        assert_eq!(blend(Color::Red, Color::Blue, 0.2), Color::Red);
        assert_eq!(blend(Color::Red, Color::Blue, 0.8), Color::Blue);
    }

    #[test]
    fn usage_ramp_is_continuous_across_the_midpoint() {
        let t = Theme::dark();
        // The seam between the two halves is where a naive two-segment ramp
        // jumps; both sides must land on yellow.
        assert_eq!(t.usage_ramp(0.5), t.yellow);
        assert_eq!(t.usage_ramp(0.0), t.accent);
        assert_eq!(t.usage_ramp(1.0), t.red);
    }

    #[test]
    fn every_builtin_name_resolves_to_its_own_palette() {
        for name in BUILTIN_THEMES {
            let t = Theme::from_name(name);
            if name != "dark" {
                assert!(
                    t.bg != Theme::dark().bg || t.accent != Theme::dark().accent,
                    "{name} fell back to dark"
                );
            }
            // Text must stand out from the background in every palette.
            let lum = |c: Color| match c {
                Color::Rgb(r, g, b) => (r as i32 * 3 + g as i32 * 6 + b as i32) / 10,
                _ => 0,
            };
            assert!(
                (lum(t.text) - lum(t.bg)).abs() > 90,
                "{name}: text too close to bg"
            );
        }
        assert!(Theme::from_name("catppuccin-latte").is_light());
        assert!(!Theme::from_name("phosphor").is_light());
    }
}
