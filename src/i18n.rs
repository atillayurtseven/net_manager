//! Gömülü çeviri kataloğu.
//!
//! Kaynak ve varsayılan dil İngilizce (`i18n/en.json`); yeni anahtarlar önce
//! oraya yazılır. Bir anahtar aktif dilde yoksa İngilizce karşılığına, o da
//! yoksa anahtarın kendisine düşer — böylece eksik çeviri boş metin üretmez.
//!
//! Dil eklemek: `i18n/<kod>.json` dosyasını oluştur ve `CATALOGS`'a bir satır ekle.

use std::collections::HashMap;
use std::sync::RwLock;

/// (kod, seçim listesinde görünecek ad, gömülü JSON)
const CATALOGS: [(&str, &str, &str); 2] = [
    ("en", "English", include_str!("../i18n/en.json")),
    ("tr", "Türkçe", include_str!("../i18n/tr.json")),
];

/// Kaynak dil: hem varsayılan hem eksik anahtarların düştüğü yer.
pub const DEFAULT: &str = "en";

struct State {
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
            active: DEFAULT.to_string(),
            map: parse(DEFAULT),
            fallback: parse(DEFAULT),
        })
    })
}

/// Desteklenen diller: (kod, kendi dilindeki adı).
pub fn available() -> Vec<(&'static str, &'static str)> {
    CATALOGS.iter().map(|(c, name, _)| (*c, *name)).collect()
}

pub fn is_known(code: &str) -> bool {
    CATALOGS.iter().any(|(c, _, _)| *c == code)
}

/// Aktif dili ayarlar; tanınmayan kod varsayılana düşer.
pub fn set_lang(code: &str) {
    let active = if is_known(code) { code } else { DEFAULT };
    let mut s = state().write().unwrap();
    s.map = parse(active);
    s.active = active.to_string();
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

/// Metindeki `{}` yer tutucularını sırayla doldurur.
pub fn tn(key: &str, args: &[&str]) -> String {
    args.iter()
        .fold(t(key), |acc, a| acc.replacen("{}", a, 1))
}

/// Tek argümanlı kısayol.
pub fn t1(key: &str, arg: &str) -> String {
    tn(key, &[arg])
}
