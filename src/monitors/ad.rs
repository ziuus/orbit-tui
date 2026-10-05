use serde::{Deserialize, Serialize};
use std::sync::{LazyLock, Mutex};

#[derive(Clone, Default, Serialize, Deserialize, Debug)]
pub struct SponsorSnapshot {
    pub message: String,
    pub subtext: String,
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

#[derive(Deserialize)]
struct AdPayload {
    message: String,
    subtext: String,
    url: String,
}

pub fn start() {
    std::thread::Builder::new()
        .name("orbit-ad".into())
        .spawn(|| loop {
            std::thread::sleep(std::time::Duration::from_secs(2));
            if !DEMAND.due(std::time::Duration::from_secs(3600)) {
                continue;
            }

            let mut snap = SponsorSnapshot {
                ready: false,
                message: String::new(),
                subtext: String::new(),
                url: String::new(),
            };

            let fetch_url = "https://raw.githubusercontent.com/ziuus/orbit-tui/main/sponsor.json";

            if let Ok(res) = ureq::get(fetch_url).call() {
                if let Ok(payload) = res.into_body().read_json::<AdPayload>() {
                    snap.ready = true;
                    snap.message = payload.message;
                    snap.subtext = payload.subtext;
                    snap.url = payload.url;
                }
            }

            if !snap.ready {
                snap.ready = true;
                snap.message = "Orbit Pro".to_string();
                snap.subtext = "Sync your layouts across all your devices.".to_string();
                snap.url = "orbit-tui.com/pro".to_string();
            }

            *SNAP.lock().unwrap() = snap;
            std::thread::sleep(std::time::Duration::from_secs(3600));
        })
        .expect("spawn ad thread");
}
