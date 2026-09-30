//! Desktop notifications, with the terminal bell as the fallback.
//!
//! Each notification runs `notify-send` on a short-lived thread that waits
//! for it, so a dashboard that runs for weeks never accumulates zombie
//! children.

use std::collections::HashMap;

/// How loudly to notify. Maps to notify-send's urgency levels.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Urgency {
    Normal,
    Critical,
}

pub fn send(title: &str, body: &str, urgency: Urgency) {
    let (title, body) = (title.to_string(), body.to_string());
    let level = match urgency {
        Urgency::Normal => "normal",
        Urgency::Critical => "critical",
    };
    let _ = std::thread::Builder::new()
        .name("vanta-notify".into())
        .spawn(move || {
            let ok = std::process::Command::new("notify-send")
                .args(["-a", "vanta", "-u", level, &title, &body])
                .status()
                .is_ok_and(|s| s.success());
            if !ok {
                use std::io::Write;
                let _ = std::io::stdout().write_all(b"\x07");
                let _ = std::io::stdout().flush();
            }
        });
}

/// Remembers which alerts were already announced so each is notified once
/// when it appears, again only if it escalates to critical, and re-armed
/// after it clears.
#[derive(Default)]
pub struct AlertLatch {
    shown: HashMap<&'static str, bool>,
}

impl AlertLatch {
    /// Given the alerts active now as (kind, critical), return the ones to
    /// announce and update the latch.
    pub fn update(&mut self, active: &[(&'static str, bool)]) -> Vec<(&'static str, bool)> {
        let mut fire = Vec::new();
        for &(kind, crit) in active {
            match self.shown.get(kind) {
                Some(&was_crit) if was_crit || !crit => {}
                _ => fire.push((kind, crit)),
            }
            let was = self.shown.get(kind).copied().unwrap_or(false);
            self.shown.insert(kind, crit || was);
        }
        self.shown.retain(|k, _| active.iter().any(|(a, _)| a == k));
        fire
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn alerts_fire_once_escalate_once_and_rearm_after_clearing() {
        let mut l = AlertLatch::default();
        assert_eq!(l.update(&[("bat", false)]), vec![("bat", false)]);
        assert!(l.update(&[("bat", false)]).is_empty(), "no repeat");
        assert_eq!(
            l.update(&[("bat", true)]),
            vec![("bat", true)],
            "escalation"
        );
        assert!(l.update(&[("bat", true)]).is_empty());
        assert!(
            l.update(&[("bat", false)]).is_empty(),
            "de-escalating is quiet"
        );
        assert!(l.update(&[]).is_empty());
        assert_eq!(
            l.update(&[("bat", false)]),
            vec![("bat", false)],
            "re-armed"
        );
        // Independent kinds.
        assert_eq!(
            l.update(&[("bat", false), ("disk", true)]),
            vec![("disk", true)]
        );
    }
}
