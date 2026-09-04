//! Gömülü çeviri kataloğu.
//!
//! Kaynak dil İngilizce (`i18n/en.json`); yeni anahtarlar önce oraya yazılır.
//! Bir anahtar aktif dilde yoksa İngilizce karşılığına, o da yoksa anahtarın
//! kendisine düşer — böylece eksik çeviri boş metin üretmez.
//!
//! Dil eklemek: `i18n/<kod>.json` dosyasını oluştur ve `CATALOGS`'a bir satır ekle.

use std::collections::HashMap;
use std::sync::RwLock;

/// (kod, menüde görünecek ad, gömülü JSON)
const CATALOGS: [(&str, &str, &str); 2] = [
    ("en", "English", include_str!("../i18n/en.json")),
    ("tr", "Türkçe", include_str!("../i18n/tr.json")),
];

const SOURCE: &str = "en";

struct State {
    /// Gerçekte yüklü olan dil.
    active: String,
    map: HashMap<String, String>,
    fallback: HashMap<String, String>,
}

fn parse(code: &str) -> HashMap<String, String> {
    CATALOGS
        .iter()
        .find(|(c, _, _)| *c == code)
        .and_then(|(_, _, json)| serde_json::from_str(json).ok())
        .unwrap_or_default()
}

fn state() -> &'static RwLock<State> {
    static S: std::sync::OnceLock<RwLock<State>> = std::sync::OnceLock::new();
    S.get_or_init(|| {
        RwLock::new(State {
            active: SOURCE.to_string(),
            map: parse(SOURCE),
            fallback: parse(SOURCE),
        })
    })
}

/// Desteklenen diller: (kod, kendi dilindeki adı).
pub fn available() -> Vec<(&'static str, &'static str)> {
    CATALOGS.iter().map(|(c, name, _)| (*c, *name)).collect()
}

fn is_known(code: &str) -> bool {
    CATALOGS.iter().any(|(c, _, _)| *c == code)
}

/// İşletim sisteminin dilinden desteklenen bir kod türetir.
fn system_lang() -> String {
    #[cfg(target_os = "macos")]
    let raw = std::process::Command::new("defaults")
        .args(["read", "-g", "AppleLocale"])
        .output()
        .ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .filter(|s| !s.is_empty())
        .or_else(|| std::env::var("LANG").ok())
        .unwrap_or_default();

    #[cfg(not(target_os = "macos"))]
    let raw = std::env::var("LC_ALL")
        .or_else(|_| std::env::var("LANG"))
        .unwrap_or_default();

    // "tr_TR.UTF-8" / "tr-TR" -> "tr"
    let code = raw
        .split(['_', '-', '.'])
        .next()
        .unwrap_or("")
        .to_lowercase();

    if is_known(&code) {
        code
    } else {
        SOURCE.to_string()
    }
}

/// Aktif dili ayarlar. `None` = sistem dilini izle.
pub fn set_lang(chosen: Option<&str>) {
    let active = match chosen {
        Some(c) if is_known(c) => c.to_string(),
        _ => system_lang(),
    };
    let mut s = state().write().unwrap();
    s.map = parse(&active);
    s.active = active;
}

/// Yüklü olan dilin kodu.
pub fn active() -> String {
    state().read().unwrap().active.clone()
}

/// Anahtarın karşılığı; yoksa İngilizcesi, o da yoksa anahtarın kendisi.
pub fn t(key: &str) -> String {
    let s = state().read().unwrap();
    s.map
        .get(key)
        .or_else(|| s.fallback.get(key))
        .cloned()
        .unwrap_or_else(|| key.to_string())
}

/// Metindeki tek `{}` yer tutucusunu doldurur.
pub fn t1(key: &str, arg: &str) -> String {
    t(key).replacen("{}", arg, 1)
}
