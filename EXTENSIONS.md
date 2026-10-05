# Orbit Extension Developer Guide

Orbit is built to be highly extensible. Any developer can build custom widgets, data fetchers, and UI components using **WebAssembly (WASM)** and the **Extism** plugin system. 

This guide explains how to create your own Orbit plugin out of the box, compile it, and publish it to the community.

## 1. Extension Architecture

Orbit uses micro-extensions. Every individual widget (e.g., a Crypto tracker, a World Clock, a File Browser) is compiled as its own independent `.wasm` plugin.

Because the plugins run in an Extism sandbox:
- **They are secure:** They cannot randomly access your filesystem unless permitted by the host.
- **They can be written in many languages:** While we use Rust for first-party plugins, Extism supports Go, Zig, C, JS, and more.
- **They fetch data via the Host:** WASM plugins cannot perform native blocking I/O directly. They use `extism:host/env::http_request` for network requests, or query the Orbit host for system data via a Key-Value mailbox (`orbit_query`).

## 2. Setting Up a New Plugin (Rust)

Currently, the most supported way to build a Orbit plugin is using Rust.

1. Create a new Rust library crate:
   ```bash
   cargo new my_orbit_widget --lib
   cd my_orbit_widget
   ```

2. Update your `Cargo.toml` to include the Extism PDK (Plugin Development Kit) and Orbit's required types (if any):
   ```toml
   [package]
   name = "my_orbit_widget"
   version = "0.1.0"
   edition = "2021"

   [lib]
   crate-type = ["cdylib"]

   [dependencies]
   extism-pdk = "1.0"
   serde = { version = "1.0", features = ["derive"] }
   serde_json = "1.0"
   ```

## 3. Plugin Structure

Every Orbit widget needs to export a `render` function that Orbit's rendering engine will call every frame.

Here is a minimal example of a Orbit widget (`src/lib.rs`):

```rust
use extism_pdk::*;
use serde_json::json;

// This is called when the plugin is first loaded
#[plugin_fn]
pub fn init() -> FnResult<()> {
    // You can set up initial state, fetch APIs, etc.
    Ok(())
}

// This is called every frame to draw the widget
#[plugin_fn]
pub fn render() -> FnResult<String> {
    // Return a JSON representation of a Ratatui Paragraph/Block
    // Orbit's host engine will parse this JSON and render it in the terminal.
    
    let ui_json = json!({
        "type": "paragraph",
        "text": "Hello from my custom WASM widget!",
        "style": { "fg": "green" },
        "block": {
            "title": "My Widget",
            "borders": "ALL"
        }
    });

    Ok(ui_json.to_string())
}
```

## 4. Compiling to WebAssembly

To compile your plugin, you need the `wasm32-unknown-unknown` target installed.

```bash
# Install the WASM target (only needed once)
rustup target add wasm32-unknown-unknown

# Build the plugin in release mode
cargo build --target wasm32-unknown-unknown --release
```

This will produce a `.wasm` file at `target/wasm32-unknown-unknown/release/my_orbit_widget.wasm`.

## 5. Local Testing

You don't need to publish your plugin to test it. You can instantly link it to your local Orbit installation.

1. **Link the plugin:**
   Orbit has a built-in CLI command to symlink your compiled WASM file into the extensions folder (`~/.config/orbit/extensions/`).
   ```bash
   orbit link ./target/wasm32-unknown-unknown/release/my_orbit_widget.wasm
   ```

2. **Add it to your Orbit configuration:**
   Open your `~/.config/orbit/config.toml` (or run `orbit config`) and add your widget to a layout page:

   ```toml
   [extensions]
   # Ensure extensions are enabled
   enable = true

   [[pages]]
   name = "My Custom Page"
   layout = [
       ["my_orbit_widget", "system_info"],
       ["cpu_chart", "memory_chart"]
   ]
   ```

3. **Run Orbit:**
   Start Orbit, and you should see your custom widget running!

## 6. Publishing to the Community

Once your plugin is polished and ready for the world, you can publish it to the official Orbit community registry.

1. Fork the [orbit-integrations](https://github.com/ziuus/vanta-integrations) repository.
2. Copy your plugin's source code folder into the `components/` directory.
3. Add your crate to the workspace `Cargo.toml`.
4. Create a Pull Request against the main repository.

Once your PR is merged, the GitHub Actions CI pipeline will automatically compile your widget, calculate its SHA256 hash, and publish it to the official `registry.json`. 

Users worldwide will then be able to install your plugin just by adding it to their `config.toml`!
