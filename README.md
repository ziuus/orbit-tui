<div align="center">

# orbit

**Your machine, one pane.**

A fast, aesthetic terminal system dashboard in Rust. The most complete keyboard-driven terminal dashboard — one pane, four modes, zero mouse.

[![npm](https://img.shields.io/npm/v/@ziuus/orbit?style=for-the-badge&logo=npm&logoColor=white&color=CB3837)](https://www.npmjs.com/package/@ziuus/orbit)
[![CI](https://img.shields.io/github/actions/workflow/status/ziuus/orbit/ci.yml?style=for-the-badge&label=CI)](https://github.com/ziuus/orbit/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/ziuus/orbit?style=for-the-badge&color=4A9E8E)](https://github.com/ziuus/orbit/releases)
[![License](https://img.shields.io/badge/License-MIT-blue?style=for-the-badge)](LICENSE)
[![Integrations](https://img.shields.io/badge/Community-Integrations-052e16?style=for-the-badge&logo=github&logoColor=white)](https://github.com/ziuus/orbit-integrations)
[![Landing page](https://img.shields.io/badge/Landing_Page-030712?style=for-the-badge&logo=vercel&logoColor=white)](https://orbit-website-omega.vercel.app)

<br />

<a href="https://orbit-website-omega.vercel.app">
  <video src="https://raw.githubusercontent.com/ziuus/orbit/main/docs/orbit-demo.mp4" autoplay loop muted playsinline width="900"></video>
</a>

<p align="center">
  <em>Live terminal dashboard in action. <a href="https://github.com/ziuus/orbit/releases/download/v0.10.38/orbit-demo.mp4">Watch 1080p 60FPS video</a> • <a href="https://orbit-website-omega.vercel.app">Visit Website</a></em>
</p>

</div>

---

Orbit collapses everything you care about into one terminal pane: CPU, memory, disk,
network, GPU (and radial thermals), processes, now-playing, GitHub contributions, multiple timezones, a calendar, and an audio
visualizer. Keyboard only. **~3 MB binary, ~2–5% CPU** at the default 30 fps.

```bash
npm install -g @ziuus/orbit && orbit
```

## Install

<table>
<tr><th align="left">Method</th><th align="left">Command</th><th align="left">Notes</th></tr>
<tr>
  <td><strong>npm</strong></td>
  <td><code>npm install -g @ziuus/orbit</code></td>
  <td>Fetches prebuilt binary for Linux, macOS (Apple Silicon &amp; Intel), or Windows. No Rust needed.</td>
</tr>
<tr>
  <td><strong>Prebuilt binary</strong></td>
  <td><a href="https://github.com/ziuus/orbit/releases/latest">Releases</a></td>
  <td>Download the <code>tar.gz</code>, drop <code>orbit</code> on your <code>PATH</code>.</td>
</tr>
<tr>
  <td><strong>cargo</strong></td>
  <td><code>cargo install --git https://github.com/ziuus/orbit</code></td>
  <td>Any architecture. Needs a Rust toolchain.</td>
</tr>
<tr>
  <td><strong>From source</strong></td>
  <td><code>git clone https://github.com/ziuus/orbit &amp;&amp; cd orbit &amp;&amp; cargo run --release</code></td>
  <td>For hacking on it.</td>
</tr>
</table>

**One-liner, no install:**

```bash
npx @ziuus/orbit@latest
```

### Requirements

Linux, macOS, and Windows.

Linux optionally requires `libdbus` for systemd and media player monitoring:

```bash
sudo pacman -S dbus              # Arch
sudo apt install libdbus-1-dev   # Debian / Ubuntu
sudo dnf install dbus-devel      # Fedora
```

On macOS and Windows, Orbit compiles natively and monitors standard system metrics out-of-the-box via `sysinfo`.

Optional tools that light up extra panels — orbit degrades quietly without them:

| Tool | Unlocks |
|------|---------|
| `cava` | Audio visualizer (falls back to an idle wave) |
| `nmcli` | Wi-Fi SSID + signal strength |
| `nvidia-smi` | NVIDIA GPU utilisation, VRAM, temp |
| `docker` | Running-container count |
| `checkupdates` | Pending Arch package updates |

## Managing orbit

```bash
orbit --version                 # what am I running
orbit --help                    # usage + keys

npm update -g @ziuus/orbit      # update  (npm)
npm uninstall -g @ziuus/orbit   # remove  (npm)

cargo install --git https://github.com/ziuus/orbit --force   # update  (cargo)
cargo uninstall orbit                                        # remove  (cargo)

rm -rf ~/.config/orbit          # drop config + persisted theme/page
```

Where things live:

| Path | What |
|------|------|
| `~/.config/orbit/config.toml` | Config, plus the theme / page / visualizer orbit persists for you |
| `$(npm root -g)/@ziuus/orbit/bin/` | The npm-installed binary |
| `~/.cargo/bin/orbit` | The cargo-installed binary |

Orbit respects `XDG_CONFIG_HOME` if you set it.

## Cross-Platform Support

Orbit was originally built as a Linux-first dashboard (it natively reads `/proc`, `/sys`, and `dbus` on Linux), but it is fully cross-platform and will dynamically fall back to the `sysinfo` crate on Windows and macOS.

**What works everywhere (Windows / macOS / Linux):**
- **Core System Stats**: CPU usage, Memory, Disk Space, Network I/O
- **Process Manager**: Shows running processes, CPU/Mem usage, process tree, and kill/terminate signals
- **File Manager**: Fully functional cross-platform (navigation, search, rename, open, trash)
- **Image Engine / UI**: The Ratatui UI, terminal graphics (Braille/Clear mode), themes, and custom layouts

**What is Linux-only (Will be hidden or disabled on Windows/macOS):**
- **Systemd Services**: The services tab only works on Linux
- **Media Player (MPRIS)**: The "Now Playing" widget uses Linux `dbus`
- **Open Network Connections**: Port binding info uses Linux-specific socket APIs
- **Fine-grained CPU stats**: `IOWait` percentage and specific CPU topology/temperature sensors rely directly on `/proc/stat` and `/sys`


## ☁️ Orbit Pro & Orbit AI Diagnostics

Orbit goes beyond a generic terminal dashboard by serving as a highly extensible developer control plane.

- **Orbit Core:** Completely free and open-source. All local monitoring, extensions, and standard dashboard features.
- **Orbit AI Diagnostics (Press `5`):** Your personal sysadmin AI natively inside the terminal. Orbit captures context (CPU, Mem, Top Processes) and sends it to OpenAI to diagnose system anomalies (e.g., OOM crashes, thermal throttling) and recommends immediate shell fixes.
  - *Bring Your Own Key (BYOK):* Export `ORBIT_OPENAI_KEY` to unlock this instantly.
  - *Orbit Pro:* Don't have an OpenAI key? Upgrade to **[Orbit Pro](https://orbit-tui.com/pro)** to get unlimited cloud diagnostics, layout syncing across devices, and premium plugins!

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

<img src="docs/monitor.png" alt="orbit monitor page" width="900" />

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

<img src="docs/aesthetic.png" alt="orbit aesthetic page" width="900" />

### `4` — Workspace

Turn your terminal into a productivity hub with a built-in file manager (with image previews), RSS news reader, tasks, agenda, and Obsidian vault viewer (with markdown syntax highlighting). Press `e` on any focused panel to edit the content in your `$EDITOR`.


### `?` — Help

Every key, on screen, on any page.

<img src="docs/help.png" alt="orbit help overlay" width="900" />

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

## Contributing

Issues and PRs welcome — bug reports, new themes, new panels, or a GPU vendor
that isn't covered yet.

```bash
git clone https://github.com/ziuus/orbit && cd orbit
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
