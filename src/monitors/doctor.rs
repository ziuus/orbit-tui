use serde_json::json;
use std::sync::{LazyLock, Mutex};

#[derive(Clone, Debug)]
pub enum DoctorStatus {
    Idle,
    Working(String),
    Done(String),
    Error(String),
}

static STATE: LazyLock<Mutex<DoctorStatus>> = LazyLock::new(|| Mutex::new(DoctorStatus::Idle));

pub fn state() -> DoctorStatus {
    STATE.lock().unwrap().clone()
}

pub fn trigger_diagnosis() {
    let mut lock = STATE.lock().unwrap();
    if matches!(*lock, DoctorStatus::Working(_)) {
        return;
    }

    let api_key = match std::env::var("ORBIT_OPENAI_KEY") {
        Ok(k) if !k.is_empty() => k,
        _ => {
            *lock = DoctorStatus::Error("Missing ORBIT_OPENAI_KEY".to_string());
            return;
        }
    };

    *lock = DoctorStatus::Working("Gathering system context...".to_string());
    drop(lock);

    std::thread::Builder::new()
        .name("orbit-doctor".into())
        .spawn(move || {
            // Simulated delay for UI feel
            std::thread::sleep(std::time::Duration::from_millis(500));

            // 1. Gather context
            let procs = crate::monitors::processes::top_by_cpu(5);
            let mut top_procs = String::new();
            for p in procs.iter() {
                top_procs.push_str(&format!("{} (CPU: {:.1}%, MEM: {}MB)\n", p.name, p.cpu_pct, p.mem_kb / 1024));
            }

            let summary = crate::monitors::summary();
            let prompt = format!(
                "You are Orbit Doctor, an elite AI terminal assistant. \
                Analyze the following Linux/Mac system state and provide a brief diagnosis (under 100 words). \
                Include a 'Diagnosis:' and 'Action:' section.\
                \nCPU Usage: {:.1}%\
                \nMemory Usage: {:.1}%\
                \nTop Processes:\n{}",
                summary.cpu_pct, summary.mem_pct, top_procs
            );

            *STATE.lock().unwrap() = DoctorStatus::Working("Analyzing via OpenAI...".to_string());

            // 2. Call OpenAI
            let body = json!({
                "model": "gpt-4o-mini",
                "messages": [
                    {"role": "system", "content": "You are a concise, expert sysadmin AI."},
                    {"role": "user", "content": prompt}
                ],
                "max_tokens": 200,
            });

            // Note: ureq v3 uses `.header()` rather than `.set()`
            let auth_val = format!("Bearer {}", api_key);
            match ureq::post("https://api.openai.com/v1/chat/completions")
                .header("Authorization", &auth_val)
                .send_json(body)
            {
                Ok(res) => {
                    let status = res.status();
                    if let Ok(json) = res.into_body().read_json::<serde_json::Value>() {
                        if let Some(content) = json["choices"][0]["message"]["content"].as_str() {
                            *STATE.lock().unwrap() = DoctorStatus::Done(content.to_string());
                        } else if let Some(error_msg) = json["error"]["message"].as_str() {
                            *STATE.lock().unwrap() = DoctorStatus::Error(format!("OpenAI Error: {}", error_msg));
                        } else {
                            *STATE.lock().unwrap() = DoctorStatus::Error("Failed to parse OpenAI response.".to_string());
                        }
                    } else {
                        *STATE.lock().unwrap() = DoctorStatus::Error(format!("HTTP {}: Failed to read response", status));
                    }
                }
                Err(e) => {
                    let err_msg = e.to_string();
                    let friendly = if err_msg.contains("401") || err_msg.contains("Unauthorized") {
                        "Invalid API key. Check ORBIT_OPENAI_KEY.".to_string()
                    } else if err_msg.contains("429") || err_msg.contains("rate limit") {
                        "Rate limited. Wait a moment and try again.".to_string()
                    } else if err_msg.contains("timeout") || err_msg.contains("timed out") {
                        "Request timed out. Check your network connection.".to_string()
                    } else {
                        format!("API Error: {}", err_msg)
                    };
                    *STATE.lock().unwrap() = DoctorStatus::Error(friendly);
                }
            }
        })
        .expect("spawn doctor thread");
}
