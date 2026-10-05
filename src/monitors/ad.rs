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

            let fetch_url = "https://orbit-tui.vercel.app/api/ad";

            if let Ok(res) = ureq::get(fetch_url).call() {
                if let Ok(payload) = res.into_body().read_json::<AdPayload>() {
                    // Only show the ad if the message isn't empty/placeholder
                    if !payload.message.is_empty() && payload.message != "Ad Space Available" {
                        snap.ready = true;
                        snap.message = payload.message;
                        snap.subtext = payload.subtext;
                        snap.url = payload.url;
                    }
                }
            }

            *SNAP.lock().unwrap() = snap;
            std::thread::sleep(std::time::Duration::from_secs(3600));
        })
        .expect("spawn ad thread");
}
