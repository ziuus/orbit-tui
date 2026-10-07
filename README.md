<div align="center">

# orbit

**Your machine, one pane.**

A fast, aesthetic terminal system dashboard in Rust. The most complete keyboard-driven terminal dashboard — one pane, four modes, zero mouse.

[![npm version](https://img.shields.io/npm/v/@ziuus/orbit?style=for-the-badge&logo=npm&logoColor=white&color=CB3837)](https://www.npmjs.com/package/@ziuus/orbit)
[![npm weekly](https://img.shields.io/npm/dw/@ziuus/orbit?style=for-the-badge&logo=npm&logoColor=white&color=CB3837)](https://www.npmjs.com/package/@ziuus/orbit)
[![CI](https://img.shields.io/github/actions/workflow/status/ziuus/orbit-tui/ci.yml?style=for-the-badge&label=CI)](https://github.com/ziuus/orbit-tui/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/ziuus/orbit-tui?style=for-the-badge&color=4A9E8E)](https://github.com/ziuus/orbit-tui/releases)
[![License](https://img.shields.io/badge/License-MIT-blue?style=for-the-badge)](LICENSE)
[![Integrations](https://img.shields.io/badge/Community_Extensions-27-052e16?style=for-the-badge&logo=github&logoColor=white)](https://github.com/ziuus/orbit-integrations)
[![Landing page](https://img.shields.io/badge/Landing_Page-030712?style=for-the-badge&logo=vercel&logoColor=white)](https://orbit-tui.vercel.app)

<br />

<a href="https://orbit-tui.vercel.app">
  <img src="https://raw.githubusercontent.com/ziuus/orbit-tui/main/docs/vanta-demo.gif" alt="Orbit Live Demo" width="900" style="border-radius: 8px;" />
</a>

<p align="center">
  <em>Live terminal dashboard in action. <a href="https://raw.githubusercontent.com/ziuus/orbit-tui/main/docs/vanta-demo.mp4">Watch 1080p 60FPS video</a> • <a href="https://orbit-tui.vercel.app">Visit Website</a></em>
</p>

<p align="center">
  <a href="https://star-history.com/#ziuus/orbit-tui&Date">
   <picture>
     <source media="(prefers-color-scheme: dark)" srcset="https://api.star-history.com/svg?repos=ziuus/orbit-tui&type=Date&theme=dark" />
     <source media="(prefers-color-scheme: light)" srcset="https://api.star-history.com/svg?repos=ziuus/orbit-tui&type=Date" />
     <img alt="Star History Chart" src="https://api.star-history.com/svg?repos=ziuus/orbit-tui&type=Date" width="800" />
   </picture>
  </a>
</p>

## Overview

Orbit is designed to sit on a secondary monitor and never be closed. It uses
zero web tech, runs on roughly 0.5% of a core at idle, and bypasses standard
terminal cell boundaries by dynamically rendering block characters at 60 FPS.

## Installation

You can install Orbit in two ways: via NPM (recommended) or Cargo.

### 1. NPM (Recommended - Prebuilt Binaries)

```bash
npm install -g @ziuus/orbit
```

### 2. Cargo (Build from Source)

```bash
cargo install --git https://github.com/ziuus/orbit-tui.git
```

## Migration from Vanta

If you used the previous version (Vanta), your configuration is fully compatible.

```bash
# 1. Uninstall the old version
npm uninstall -g @ziuus/vanta

# 2. Migrate your configuration directory
mv ~/.config/vanta ~/.config/orbit

# 3. Install the new Orbit CLI
npm install -g @ziuus/orbit
```
*(If you installed via cargo, replace the npm commands with `cargo uninstall vanta` and `cargo install --git https://github.com/ziuus/orbit-tui.git`).*


## Pages

| Key | Page | What's on it |
|-----|------|--------------|
| `1` | **Dashboard** | System facts + distro logo, semicircle gauges, CPU graph & cores, storage, big clock, now playing, visualizer, top processes, interactive status, memory, network, calendar |
| `2` | **Monitor** | btop-style: CPU / memory / disk I/O / network / GPU graphs on top, full process table below (sort, filter, tree, kill) |
| `3` | **Aesthetic** | Huge clock, calendar, matrix rain, spinning donut, pinned media (photos), full-width visualizer |
| `4` | **Workspace** | Built-in File Manager (with image previews), Agenda, Tasks, RSS feeds, and Obsidian vault viewer |
| `?` | **Help** | Keybind reference overlay |

### `2` — Monitor

Every graph on top, every process below. Sort it, filter it, tree it, kill it.

<img src="https://raw.githubusercontent.com/ziuus/orbit-tui/main/docs/monitor.png" alt="orbit monitor page" width="900" />

### `3` — Aesthetic

For when the build is running and you want something to look at. Includes support for displaying a custom pinned image.

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

Turn your terminal into a productivity hub with a built-in file manager (with image previews), RSS news reader, tasks, agenda, and Obsidian vault viewer (with markdown syntax highlighting). Press `e` on any focused panel to edit the content in your `$EDITOR`.


### `?` — Help

Every key, on screen, on any page.

<img src="https://raw.githubusercontent.com/ziuus/orbit-tui/main/docs/help.png" alt="orbit help overlay" width="900" />

## Keys

| Key | Action |
|-----|--------|
| `1` `2` `3` `4` | Switch page (persisted as startup page) |
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
| `Ctrl + ←/→` | Resize dashboard columns horizontally (when focused) |
| `Ctrl + ↑/↓` | Resize dashboard components vertically (when focused) |

**Processes (Monitor):** `↑↓ PgUp PgDn Home End` select · `/` filter (`Enter` keeps, `Esc` clears) · `s` sort field · `r` reverse · `t` tree · `←→` fold · `c` full command · `k` SIGTERM · `K` SIGKILL (press twice to confirm, `x` cancels)

**Workspace:** `e` opens the active note, agenda, task list, or file in your `$EDITOR`.

**Calendar (focused):** `←→` month · `↑↓` year · `Home` today

## Themes

`dark` · `catppuccin` · `tokyo-night` · `nord` · `gruvbox` · `dracula` · `light` · `solarized-light`

Cycle with `T` — your choice is written back to the config file.

### Custom Themes

You can easily load custom themes dynamically without recompiling Orbit.

1. Create a `themes` folder inside your config directory (e.g. `~/.config/orbit/themes/`).
2. Drop in a `.toml` file with your theme name (e.g., `cyberpunk.toml`).
3. Define your colors using hex codes:
   ```toml
   bg = "#000000"
   accent = "#00ff00"
   secondary = "#ff00ff"
   surface = "#111111"
   text = "#ffffff"
   dim = "#555555"
   green = "#00ff00"
   yellow = "#ffff00"
   red = "#ff0000"
   ```
4. Press `T` in Orbit to cycle to it, or set it explicitly in your `config.toml` (`theme = "cyberpunk"`).

## 🔌 Extensions & Plugins

- **Configuration:** Run `orbit config` to open the interactive settings editor.
- **First-time Setup:** The same setup menu runs automatically when starting Orbit for the first time.


Orbit is fully extensible via WebAssembly (WASM). You can install community widgets, or build your own!

- **Install via Menu:** Run `orbit menu` to browse and install extensions interactively.
- **Local Dev:** Use `orbit link /path/to/my_widget.wasm` to instantly test your custom plugin locally without publishing it.

Read more about building plugins at the [orbit-integrations](https://github.com/ziuus/orbit-integrations) repository.

## Configuration & Customization

Orbit is highly configurable via `~/.config/orbit/config.toml`. You can:
- Change the layout, refresh rates, and themes
- Add your own **Custom Widgets** using shell commands or files
- Install community-built **WASM Extensions** to add entirely new UI panels without recompiling

For a complete guide to all configuration options, extensions, and custom widgets, please read:
👉 **[docs/CONFIGURATION.md](docs/CONFIGURATION.md)**

## Architecture

All telemetry is collected on a single background sampler thread (`/proc`, `/sys`,
sysinfo, D-Bus) into lock-guarded snapshots; the render loop only reads snapshots
and never blocks on I/O.

| Layer | Technology |
|-------|------------|
| TUI | Ratatui 0.29 + Crossterm |
| Telemetry | `sysinfo`, `/proc`, `/sys/class/{hwmon,drm,power_supply}` |
| GPU | NVIDIA (`nvidia-smi`), AMD (sysfs), Intel (clock only) |
| Media | MPRIS over D-Bus (`dbus` crate) |
| Audio viz | `cava` raw output, idle wave fallback |

## 🧩 Creating Plugins & Extensions

Want to build your own custom widgets (like a Crypto tracker or a File Space viewer)? Orbit supports a powerful, secure WebAssembly (WASM) plugin system via Extism.

Check out the **[Orbit Extension Developer Guide (EXTENSIONS.md)](EXTENSIONS.md)** to learn how to:
- Structure a new Rust WASM plugin out of the box.
- Compile and test your plugin locally (`orbit link`).
- Publish your plugin to the community registry (`orbit-integrations`).

## 🤖 AI Agent Integration

Orbit is explicitly designed to be modified and configured by AI agents (like Cursor, Claude, or Copilot). We provide dedicated skill/prompt files that you can feed directly into your AI:

- **For End Users (Configuration):** Provide the **[ORBIT_CONFIG_SKILL.md](ORBIT_CONFIG_SKILL.md)** file to your AI. It teaches the agent how to build gorgeous 2D dashboard grids, customize themes, set up performance profiles, and write your `config.toml`.
- **For Developers (WASM Plugins):** Provide the **[AGENTS.md](AGENTS.md)** file to your AI. It acts as a complete architecture and API guide, teaching the agent how to write Extism WASM widgets, hook into the host Key-Value mailbox, and compile plugins for Orbit.

## 📣 Sponsors

Orbit is MIT licensed and maintained by [zius](https://github.com/ziuus) and
[naborajs](https://github.com/naborajs). It is free, has no paid tier gating the core
dashboard, and always will.

**Orbit ships with a sponsor slot.** A two-line, non-intrusive banner anchored to the
bottom of the TUI — shown on every page, rendered only when a sponsor is configured.
There are no ad blockers in the CLI, so it is never filtered, but it is also plainly
visible to you in the source and in this README. I do not track who looks at it.

If your product is used by terminal users, operators, or SREs, the slot is available.
Reach out at **sponsor@orbit-tui.com** — full media kit, pricing, and past sponsors in
**[SPONSORS.md](SPONSORS.md)**.

Currently sponsoring: **Vercel** (in-UI slot) · **Railway** (README)

You can also support the project directly through [GitHub Sponsors](https://github.com/sponsors/ziuus).

## Contributing

Issues and PRs welcome — bug reports, new themes, new panels, or a GPU vendor
that isn't covered yet.

```bash
git clone https://github.com/ziuus/orbit-tui.git && cd orbit-tui
cargo run --release

# CI runs exactly this — make it pass first
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test
```

Read **[CONTRIBUTING.md](CONTRIBUTING.md)** first — it covers how the code is laid
out, the sampler-thread rule (*never do I/O inside a `render()`*), the panel
degradation contract, and how to add a theme.

Good first contributions:

- A new theme in `src/theme.rs` (add a constructor + its name to `THEME_NAMES`)
- Per-interface network stats
- NVMe / extra `hwmon` temperature sources
- Mouse click-to-focus


## License

[MIT](LICENSE) — do what you like, keep the notice.

<div align="center">
<br />
built by <a href="https://github.com/ziuus">zius</a> · if it's useful, a ⭐ helps
</div>
