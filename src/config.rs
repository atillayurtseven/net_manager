use crate::units::{Mode, Unit};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Clone)]
pub struct Config {
    /// None = tüm ağ arayüzleri
    pub iface: Option<String>,
    pub autostart: bool,
    /// Göstergede kullanılacak en küçük / en büyük birim
    #[serde(default = "default_min")]
    pub min_unit: Unit,
    #[serde(default = "default_max")]
    pub max_unit: Unit,
    /// Bayt (KB/s) mi bit (Mbps) mi
    #[serde(default)]
    pub unit_mode: Mode,
    /// Arayüz dili kodu; tanınmıyorsa varsayılana düşer
    #[serde(default = "default_lang")]
    pub lang: String,
}

fn default_min() -> Unit { Unit::KB }
fn default_max() -> Unit { Unit::GB }
fn default_lang() -> String { crate::i18n::DEFAULT.to_string() }

impl Default for Config {
    fn default() -> Self {
        Self {
            iface: None,
            autostart: false,
            min_unit: default_min(),
            max_unit: default_max(),
            unit_mode: Mode::default(),
            lang: default_lang(),
        }
    }
}

fn path() -> PathBuf {
    let mut p = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
    p.push("net_manager");
    let _ = std::fs::create_dir_all(&p);
    p.push("config.json");
    p
}

impl Config {
    pub fn load() -> Self {
        std::fs::read_to_string(path())
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    pub fn save(&self) {
        if let Ok(s) = serde_json::to_string_pretty(self) {
            let _ = std::fs::write(path(), s);
        }
    }
}
