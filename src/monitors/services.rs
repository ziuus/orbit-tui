use serde::{Deserialize, Serialize};
use std::sync::{LazyLock, RwLock};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceNode {
    pub name: String,
    pub load_state: String,
    pub active_state: String,
    pub sub_state: String,
    pub pid: u32,
    pub start_ts: u64,
}

static SNAPSHOT: LazyLock<RwLock<Vec<ServiceNode>>> = LazyLock::new(|| RwLock::new(Vec::new()));
static DEMAND: super::Demand = super::Demand::new();

pub fn snapshot() -> Vec<ServiceNode> {
    DEMAND.touch();
    SNAPSHOT.read().unwrap().clone()
}

#[cfg(not(target_os = "linux"))]
pub fn sample() {
    // Services monitoring relies on systemd via dbus, which is Linux only.
}

#[cfg(target_os = "linux")]
thread_local! {
    static CONN: std::cell::OnceCell<Option<dbus::blocking::Connection>> = const { std::cell::OnceCell::new() };
}

#[cfg(target_os = "linux")]
pub fn sample() {
    use std::time::Duration;
    if !DEMAND.due(Duration::from_secs(3)) {
        return;
    }
    CONN.with(|c| {
        if let Some(conn) = c.get_or_init(|| dbus::blocking::Connection::new_system().ok()) {
            sample_with(conn);
        }
    });
}

#[cfg(target_os = "linux")]
fn sample_with(conn: &dbus::blocking::Connection) {
    use dbus::arg::{RefArg, Variant};
    use std::time::Duration;
    
    let proxy = conn.with_proxy(
        "org.freedesktop.systemd1",
        "/org/freedesktop/systemd1",
        Duration::from_millis(500),
    );

    type UnitInfo = (
        String,
        String,
        String,
        String,
        String,
        String,
        dbus::Path<'static>,
        u32,
        String,
        dbus::Path<'static>,
    );

    let (units,): (Vec<UnitInfo>,) =
        match proxy.method_call("org.freedesktop.systemd1.Manager", "ListUnits", ()) {
            Ok(v) => v,
            Err(_) => return,
        };

    let mut services = Vec::new();

    for u in units {
        let (
            id,
            _desc,
            load_state,
            active_state,
            sub_state,
            _follow,
            path,
            _job_id,
            _job_type,
            _job_path,
        ) = u;
        if id.ends_with(".service") {
            let unit_proxy =
                conn.with_proxy("org.freedesktop.systemd1", path, Duration::from_millis(10));
            let mut pid = 0;
            let mut start_ts = 0;

            if active_state == "active" || active_state == "failed" {
                if let Ok((p,)) = unit_proxy.method_call::<(Variant<Box<dyn RefArg>>,), _, _, _>(
                    "org.freedesktop.DBus.Properties",
                    "Get",
                    ("org.freedesktop.systemd1.Service", "MainPID"),
                ) {
                    if let Some(n) = p.0.as_u64() {
                        pid = n as u32;
                    }
                }

                if let Ok((ts,)) = unit_proxy.method_call::<(Variant<Box<dyn RefArg>>,), _, _, _>(
                    "org.freedesktop.DBus.Properties",
                    "Get",
                    ("org.freedesktop.systemd1.Service", "ExecMainStartTimestamp"),
                ) {
                    if let Some(n) = ts.0.as_u64() {
                        start_ts = n;
                    }
                }
            }

            services.push(ServiceNode {
                name: id,
                load_state,
                active_state,
                sub_state,
                pid,
                start_ts,
            });
        }
    }

    // Sort by name for stable predictable ordering
    services.sort_unstable_by(|a, b| a.name.cmp(&b.name));

    *SNAPSHOT.write().unwrap() = services;
}
