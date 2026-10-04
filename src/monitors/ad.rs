use serde::{Deserialize, Serialize};
use std::sync::{LazyLock, Mutex};

#[derive(Clone, Default, Serialize, Deserialize, Debug)]
pub struct SponsorSnapshot {
    pub message: String,
    pub sponsor_name: String,
    pub url: String,
    pub ready: bool,
}

static SNAP: LazyLock<Mutex<SponsorSnapshot>> =
    LazyLock::new(|| Mutex::new(SponsorSnapshot::default()));

static DEMAND: super::Demand = super::Demand::new();

pub fn snapshot() -> SponsorSnapshot {
    DEMAND.touch();
    SNAP.lock().unwrap().clone()
}

pub fn start() {
    std::thread::Builder::new()
        .name("vanta-ad".into())
        .spawn(|| loop {
            std::thread::sleep(std::time::Duration::from_secs(2));
            if !DEMAND.due(std::time::Duration::from_secs(3600)) { // Don't hit API more than once per hour if idle
                continue;
            }
            let snap = SponsorSnapshot {
                ready: true,
                message: "GPT-6.1 Sol | Included in every plan.".to_string(),
                sponsor_name: "Freebuff".to_string(),
                url: "freebuff.com".to_string(),
            };

            // For now, we mock the fetch or attempt a silent dummy fetch.
            // In a real implementation, you'd fetch from a raw GitHub Gist or edge function:
            // if let Ok(res) = ureq::get("https://your-ad-server.com/sponsor.json").call() { ... }

            *SNAP.lock().unwrap() = snap;
            std::thread::sleep(std::time::Duration::from_secs(3600)); // Update hourly
        })
        .expect("spawn ad thread");
}
