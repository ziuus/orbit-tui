//! Terminal mouse reporting.
//!
//! crossterm's `EnableMouseCapture` also turns on any-motion tracking
//! (mode 1003), which makes the terminal report every pixel of pointer
//! movement and would wake the 2 fps idle loop constantly. Orbit only needs
//! presses, drags and the wheel, so it asks for exactly those: 1000 (press/
//! release + wheel), 1002 (motion only while a button is held) and 1006 (SGR
//! coordinates, so columns past 223 work).

use std::io::Write;
use std::sync::atomic::{AtomicBool, Ordering};

static ON: AtomicBool = AtomicBool::new(false);

fn write(seq: &str) {
    let mut out = std::io::stdout();
    let _ = out.write_all(seq.as_bytes());
    let _ = out.flush();
}

/// Start reporting clicks and the wheel.
pub fn enable() {
    write("\x1b[?1000h\x1b[?1002h\x1b[?1006h");
    ON.store(true, Ordering::Relaxed);
}

/// Stop reporting (restores the terminal's own text selection).
pub fn disable() {
    write("\x1b[?1006l\x1b[?1002l\x1b[?1000l");
    ON.store(false, Ordering::Relaxed);
}

/// Run `f` with raw mode and mouse reporting suspended, for handing the
/// terminal to an editor or another TUI, then restore both.
pub fn suspended<T>(f: impl FnOnce() -> T) -> T {
    let was_on = ON.load(Ordering::Relaxed);
    if was_on {
        disable();
    }
    let _ = crossterm::terminal::disable_raw_mode();
    let out = f();
    let _ = crossterm::terminal::enable_raw_mode();
    if was_on {
        enable();
    }
    out
}
