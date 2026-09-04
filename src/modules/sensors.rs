//! Sıcaklık sensörleri.
//!
//! Apple silikonda CPU sıcaklığı ve fan hızı yalnızca özel (private) API ile
//! okunabiliyor; App Store incelemesi bunu reddettiği için burada
//! kullanılmıyor. Okunabilen ne varsa gösterilir: `sysinfo`'nun bileşen
//! listesi ve batarya sıcaklığı (ioreg, sudo istemez).

use super::{fit, Module, Readout};
use crate::config::Config;
use crate::i18n::{t, tn};
use sysinfo::Components;

const ROWS: usize = 5;
/// Bağlı olmayan sensörler uçuk değerler döndürür (-9200 °C gibi); makul
/// aralık dışındaki okumalar atılır.
const SANE: std::ops::Range<f32> = -40.0..150.0;
/// Sıcaklık hızlı değişmez.
const PERIOD: u32 = 5;

pub struct Sensors {
    components: Components,
}

impl Default for Sensors {
    fn default() -> Self {
        Self {
            components: Components::new_with_refreshed_list(),
        }
    }
}

fn celsius(v: f32) -> String {
    format!("{v:.1} °C")
}

/// Batarya sıcaklığı ioreg'de yüzde bir derece biriminde durur (3070 = 30,7 °C).
#[cfg(target_os = "macos")]
fn battery_temp() -> Option<f32> {
    let out = std::process::Command::new("ioreg")
        .args(["-r", "-n", "AppleSmartBattery"])
        .output()
        .ok()?;
    let text = String::from_utf8_lossy(&out.stdout);
    let raw: f32 = text
        .lines()
        .find_map(|l| l.split_once("\"Temperature\" = "))?
        .1
        .trim()
        .parse()
        .ok()?;
    Some(raw / 100.0)
}

#[cfg(not(target_os = "macos"))]
fn battery_temp() -> Option<f32> {
    None
}

impl Module for Sensors {
    fn id(&self) -> &'static str {
        "sensors"
    }

    fn rows(&self) -> usize {
        ROWS
    }

    fn period(&self) -> u32 {
        PERIOD
    }

    fn read(&mut self, _cfg: &Config) -> Readout {
        self.components.refresh();

        let mut rows: Vec<String> = self
            .components
            .list()
            .iter()
            .filter(|c| SANE.contains(&c.temperature()))
            .map(|c| tn("sensors.component", &[c.label(), &celsius(c.temperature())]))
            .take(ROWS - 1)
            .collect();

        let batt = battery_temp();
        if let Some(v) = batt {
            rows.push(tn("sensors.battery_temp", &[&celsius(v)]));
        }

        // Hiçbir kaynak okunamadıysa nedenini yaz: boş menü kafa karıştırır.
        if rows.is_empty() {
            rows.push(t("sensors.unavailable"));
            rows.push(t("sensors.note"));
        }

        // Menü çubuğunda en sıcak okuma dursun.
        let hottest = self
            .components
            .list()
            .iter()
            .map(|c| c.temperature())
            .filter(|t| SANE.contains(t))
            .chain(batt)
            .fold(f32::MIN, f32::max);

        Readout::new(if hottest > f32::MIN {
                celsius(hottest)
            } else {
                t("module.sensors")
            }, fit(rows, ROWS))
    }
}
