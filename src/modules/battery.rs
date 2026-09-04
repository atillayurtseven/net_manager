//! Batarya durumu ve eşleşmiş Bluetooth cihazlarının pilleri.
//!
//! Veri `ioreg`'in AppleSmartBattery düğümünden gelir (sudo istemez).
//! Bluetooth pilleri `system_profiler` ile okunur; o çağrı görece pahalı
//! olduğu için seyrek yapılır.

use super::{dash, fit, Module, Readout};
use crate::config::Config;
use crate::i18n::{t, tn};

const ROWS: usize = 6;
/// Batarya yavaş değişir; her saniye yoklamanın anlamı yok.
const PERIOD: u32 = 10;
/// Bluetooth taraması en pahalı adım: birkaç dakikada bir yeter.
const BT_EVERY: u32 = 18;

#[derive(Default)]
pub struct Battery {
    devices: Vec<(String, String)>,
    ticks: u32,
}

/// `ioreg` çıktısındaki "AnahtarAdı" = değer çiftini okur.
fn field(text: &str, key: &str) -> Option<String> {
    let needle = format!("\"{key}\" = ");
    text.lines().find_map(|l| {
        l.split_once(needle.as_str())
            .map(|(_, v)| v.trim().to_string())
    })
}

fn num(text: &str, key: &str) -> Option<i64> {
    field(text, key)?.parse().ok()
}

#[cfg(target_os = "macos")]
fn read_ioreg() -> Option<String> {
    let out = std::process::Command::new("ioreg")
        .args(["-r", "-n", "AppleSmartBattery"])
        .output()
        .ok()?;
    Some(String::from_utf8_lossy(&out.stdout).to_string())
}

#[cfg(not(target_os = "macos"))]
fn read_ioreg() -> Option<String> {
    None
}

/// Eşleşmiş cihazların pil yüzdeleri: (cihaz adı, "%85").
#[cfg(target_os = "macos")]
fn read_bluetooth() -> Vec<(String, String)> {
    let Ok(out) = std::process::Command::new("system_profiler")
        .args(["SPBluetoothDataType", "-json"])
        .output()
    else {
        return Vec::new();
    };
    let Ok(v) = serde_json::from_slice::<serde_json::Value>(&out.stdout) else {
        return Vec::new();
    };

    let mut found = Vec::new();
    // Yapı sürümden sürüme değişiyor: pil alanı taşıyan her nesneyi gez.
    fn walk(v: &serde_json::Value, out: &mut Vec<(String, String)>) {
        match v {
            serde_json::Value::Object(map) => {
                for (name, item) in map {
                    if let Some(level) = item
                        .get("device_batteryLevelMain")
                        .or_else(|| item.get("device_batteryLevelSingle"))
                        .and_then(|l| l.as_str())
                    {
                        out.push((name.clone(), level.trim().to_string()));
                    }
                    walk(item, out);
                }
            }
            serde_json::Value::Array(items) => items.iter().for_each(|i| walk(i, out)),
            _ => {}
        }
    }
    walk(&v, &mut found);
    found.truncate(3);
    found
}

#[cfg(not(target_os = "macos"))]
fn read_bluetooth() -> Vec<(String, String)> {
    Vec::new()
}

impl Module for Battery {
    fn id(&self) -> &'static str {
        "battery"
    }

    fn rows(&self) -> usize {
        ROWS
    }

    fn period(&self) -> u32 {
        PERIOD
    }

    fn read(&mut self, _cfg: &Config) -> Readout {
        let Some(text) = read_ioreg() else {
            return Readout::new(t("module.battery"), fit(Vec::new(), ROWS));
        };

        let level = num(&text, "CurrentCapacity").unwrap_or(0);
        let charging = field(&text, "IsCharging").as_deref() == Some("Yes");
        let external = field(&text, "ExternalConnected").as_deref() == Some("Yes");
        let fully = field(&text, "FullyCharged").as_deref() == Some("Yes");

        let state = t(if charging {
            "battery.state_charging"
        } else if fully && external {
            "battery.state_charged"
        } else if external {
            "battery.state_ac"
        } else {
            "battery.state_discharging"
        });

        // Sağlık: kalan azami kapasitenin tasarım kapasitesine oranı.
        let health = match (
            num(&text, "AppleRawMaxCapacity").or_else(|| num(&text, "MaxCapacity")),
            num(&text, "DesignCapacity"),
        ) {
            (Some(max), Some(design)) if design > 0 => {
                format!("{:.0}%", max as f64 / design as f64 * 100.0)
            }
            _ => dash(),
        };

        // 65535 "hesaplanıyor" demek, dakika değil.
        let time_left = match num(&text, "TimeRemaining") {
            Some(m) if m > 0 && m < 65535 => format!("{}:{:02}", m / 60, m % 60),
            _ => dash(),
        };

        self.ticks += 1;
        if self.ticks % BT_EVERY == 1 {
            self.devices = read_bluetooth();
        }

        let mut rows = vec![
            tn("battery.level", &[&format!("{level}%")]),
            tn("battery.state", &[&state]),
            tn("battery.health", &[&health]),
            tn(
                "battery.cycles",
                &[&num(&text, "CycleCount")
                    .map(|c| c.to_string())
                    .unwrap_or_else(dash)],
            ),
            tn("battery.time", &[&time_left]),
        ];
        rows.extend(
            self.devices
                .iter()
                .map(|(name, lvl)| tn("battery.device", &[name, lvl])),
        );

        Readout::new(format!("{level}%"), fit(rows, ROWS))
    }
}
