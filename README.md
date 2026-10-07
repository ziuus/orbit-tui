<div align="center">

# 🪐 Orbit

**The Extensible Terminal Workspace. Your machine, one pane.**

Orbit is a fast, highly-aesthetic terminal OS and developer dashboard written in Rust. It's the most complete keyboard-driven workspace — one pane, four modes, zero mouse.

[![npm version](https://img.shields.io/npm/v/@ziuus/orbit?style=for-the-badge&logo=npm&logoColor=white&color=CB3837)](https://www.npmjs.com/package/@ziuus/orbit)
[![npm weekly](https://img.shields.io/npm/dw/@ziuus/orbit?style=for-the-badge&logo=npm&logoColor=white&color=CB3837)](https://www.npmjs.com/package/@ziuus/orbit)
[![CI](https://img.shields.io/github/actions/workflow/status/ziuus/orbit-tui/ci.yml?style=for-the-badge&label=CI)](https://github.com/ziuus/orbit-tui/actions/workflows/ci.yml)
[![Integrations](https://img.shields.io/badge/Community_Extensions-27-052e16?style=for-the-badge&logo=github&logoColor=white)](https://github.com/ziuus/orbit-integrations)
[![Landing page](https://img.shields.io/badge/Landing_Page-030712?style=for-the-badge&logo=vercel&logoColor=white)](https://orbit-tui.vercel.app)

<br />

<p align="center">
  <a href="https://star-history.com/#ziuus/orbit-tui&Date">
   <picture>
     <source media="(prefers-color-scheme: dark)" srcset="https://api.star-history.com/svg?repos=ziuus/orbit-tui&type=Date&theme=dark" />
     <source media="(prefers-color-scheme: light)" srcset="https://api.star-history.com/svg?repos=ziuus/orbit-tui&type=Date" />
     <img alt="Star History Chart" src="https://api.star-history.com/svg?repos=ziuus/orbit-tui&type=Date" width="800" />
   </picture>
  </a>
</p>

<br />

<a href="https://orbit-tui.vercel.app">
  <img src="https://raw.githubusercontent.com/ziuus/orbit-tui/main/docs/orbit-demo.gif" alt="Orbit Live Demo" width="900" style="border-radius: 8px;" />
</a>

<p align="center">
  <em>Live terminal workspace in action. <a href="https://raw.githubusercontent.com/ziuus/orbit-tui/main/docs/orbit-demo.mp4">Watch 1080p 60FPS video</a> • <a href="https://orbit-tui.vercel.app">Visit Website</a></em>
</p>
</div>

---

## 🌌 The Terminal OS

Orbit is designed to sit on a secondary monitor and never be closed. It is not just a system monitor; it is a **complete developer workspace** powered by a WebAssembly plugin ecosystem.

It uses zero web tech, runs on roughly **0.5% of a core at idle**, and bypasses standard terminal cell boundaries by dynamically rendering block characters at a smooth 60 FPS.

## 🚀 Quick Start

Install Orbit via NPM (Recommended for prebuilt binaries) or Cargo.

```bash
# Via NPM (Recommended)
npm install -g @ziuus/orbit

# Via Cargo
cargo install --git https://github.com/ziuus/orbit-tui.git
```

> **Note:** If you are migrating from our previous iteration, simply rename your configuration folder: `mv ~/.config/vanta ~/.config/orbit`

## 🗂️ Core Workspaces

| Key | Page | What's on it |
|-----|------|--------------|
| `1` | **Dashboard** | System facts, distro logo, visualizer, top processes, interactive status, memory, network, calendar. |
| `2` | **Monitor** | Comprehensive system & process matrix. Filter, sort, and manage running tasks effortlessly. |
| `3` | **Aesthetic** | Ambient environments. Matrix rain, 3D starfield, spinning donut, pinned media, full-width visualizer. |
| `4` | **Workspace** | Built-in File Manager (with image previews), Agenda, Tasks, RSS feeds, and Obsidian vault viewer. |
| `?` | **Help** | Keybind reference overlay. |

### `2` — Monitor
A deep dive into your system's heartbeat. Every vital graph on top, every active process below. Sort it, filter it, tree it, kill it.

<img src="https://raw.githubusercontent.com/ziuus/orbit-tui/main/docs/monitor.png" alt="orbit monitor page" width="900" />

### `3` — Aesthetic
For when the build is running and you want a premium ambient display. Includes support for custom pinned images.

**Includes 7 immersive ambient scenes:**
- Audio Visualizer
- Matrix Rain
- Flip Clock
- Topographic Map
- True 3D Starfield (reacts to CPU spikes and music bass)
- Conway's Game of Life (auto-seeding)
- Terminal Snowfall (piles up and melts dynamically)

<img src="https://raw.githubusercontent.com/ziuus/orbit-tui/main/docs/aesthetic.png" alt="orbit aesthetic page" width="900" />

### `4` — Workspace
Turn your terminal into a productivity hub. Manage files (with image previews), read RSS news, track tasks, and view your Obsidian vault with markdown syntax highlighting. Press `e` on any focused panel to edit content in your `$EDITOR`.

## ⌨️ Keybinds

| Key | Action |
|-----|--------|
| `1` `2` `3` `4` | Switch workspace (persisted as startup page) |
| `Tab` / `Shift-Tab` | Cycle panel focus · `Esc` clears |
| `Enter` | Zoom the focused panel to the full page · `Esc` back |
| `T` | Next theme (persisted) |
| `v` | Visualizer style: bars / mirror / wave / peaks |
| `+` / `-` | Sample faster / slower |
| `Space` `n` `p` | Play/pause · next · previous (MPRIS, any page) |
| `<` `>` | Volume down / up |
| `?` | Help overlay |
| `q` | Quit |
| `~` / `F12` | Toggle Debug Logs overlay (`s` saves them to file) |
| `S` / `,` | Open Settings overlay |
| `Ctrl + ←/→` | Resize dashboard columns horizontally |
| `Ctrl + ↑/↓` | Resize dashboard components vertically |

**Processes (Monitor):** `↑↓ PgUp PgDn Home End` select · `/` filter · `s` sort field · `r` reverse · `t` tree · `←→` fold · `c` full command · `k` SIGTERM · `K` SIGKILL.

## 🎨 Themes & Customization

Cycle through built-in themes with `T`:
`dark` · `catppuccin` · `tokyo-night` · `nord` · `gruvbox` · `dracula` · `light` · `solarized-light`

### Custom Themes
You can load custom themes dynamically without recompiling Orbit:
1. Create a `themes` folder inside your config directory (e.g. `~/.config/orbit/themes/`).
2. Drop in a `.toml` file with your theme name (e.g., `cyberpunk.toml`).
3. Define your colors using hex codes (`bg`, `accent`, `surface`, etc.).
4. Press `T` in Orbit to cycle to it.

## 🔌 Extensions & Plugins

Orbit is fully extensible via WebAssembly (WASM). You can install community widgets, or build your own!

- **Install via Menu:** Run `orbit menu` to browse and install extensions interactively.
- **Local Dev:** Use `orbit link /path/to/my_widget.wasm` to test custom plugins instantly without publishing.

Explore and download community extensions at our official registry:
👉 **[github.com/ziuus/orbit-integrations](https://github.com/ziuus/orbit-integrations)**

## 📚 Documentation

- **[CONFIGURATION.md](docs/CONFIGURATION.md)**: Layouts, custom shell widgets, and performance profiles.
- **[EXTENSIONS.md](EXTENSIONS.md)**: Orbit Extension Developer Guide (WASM).
- **[AGENTS.md](AGENTS.md)**: AI Agent architecture guide for modifying Orbit.
- **[ORBIT_CONFIG_SKILL.md](ORBIT_CONFIG_SKILL.md)**: Prompt instructions for your AI to configure your dashboard.

## 🏗️ Architecture

All telemetry is collected on a single background sampler thread (`/proc`, `/sys`, sysinfo, D-Bus) into lock-guarded snapshots. The render loop only reads snapshots and never blocks on I/O.

| Layer | Technology |
|-------|------------|
| TUI | Ratatui 0.29 + Crossterm |
| Telemetry | `sysinfo`, `/proc`, `/sys/class/{hwmon,drm,power_supply}` |
| GPU | NVIDIA (`nvidia-smi`), AMD (sysfs), Intel (clock only) |
| Media | MPRIS over D-Bus (`dbus` crate) |
| Audio viz | `cava` raw output, idle wave fallback |

## 🤝 Contributing

Issues, new themes, and PRs are welcome! 

```bash
git clone https://github.com/ziuus/orbit-tui.git && cd orbit-tui
cargo run --release

# Ensure CI passes
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test
```

Read **[CONTRIBUTING.md](CONTRIBUTING.md)** first to understand our sampler-thread rule (*never do I/O inside a `render()`*) and our UI degradation contract.

## 📄 License

[MIT](LICENSE) — Do what you like, keep the notice.

<br />

<div align="center">
  built by <a href="https://github.com/ziuus">zius</a> · if it's useful, a ⭐ helps
</div>
