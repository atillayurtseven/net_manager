//! Menü çubuğu modülleri.
//!
//! Her modül kendi menü çubuğu öğesini alır: solda kısa bir gösterge metni,
//! tıklanınca sabit sayıda bilgi satırı. Satır sayısı sabittir çünkü menü
//! öğeleri bir kez kurulup sonra yalnızca metinleri güncellenir.

pub mod battery;
pub mod clock;
pub mod cpu;
pub mod disk;
pub mod memory;
pub mod sensors;

use crate::config::Config;

/// Bir modülün tek turda ürettiği çıktı.
pub struct Readout {
    /// Menü çubuğunda görünen kısa metin.
    pub title: String,
    /// Açılır menüdeki bilgi satırları; uzunluğu her zaman `rows()` kadar.
    pub rows: Vec<String>,
    /// Eşik uyarısına konu olan doluluk yüzdesi (varsa).
    pub level: Option<f64>,
    /// Uyarı metnine girecek ad (disk adı gibi).
    pub label: Option<String>,
}

impl Readout {
    pub fn new(title: String, rows: Vec<String>) -> Self {
        Self { title, rows, level: None, label: None }
    }

    /// Eşik uyarısı için ölçülen değeri iliştirir.
    pub fn with_level(mut self, level: f64, label: Option<String>) -> Self {
        self.level = Some(level);
        self.label = label;
        self
    }
}

/// Ağ dışındaki modüller; sıra menü çubuğundaki soldan sağa dizilimdir.
pub const IDS: [&str; 5] = ["cpu", "memory", "disk", "battery", "sensors"];

/// Saat en sağda dursun diye ayrı: menü çubuğunda sistem saatinin yanına düşer.
pub const CLOCK_ID: &str = "clock";

/// Kimlikten modül üretir; tanınmayan kimlik `None` döner.
pub fn make(id: &str) -> Option<Box<dyn Module>> {
    Some(match id {
        "cpu" => Box::<cpu::Cpu>::default() as Box<dyn Module>,
        "memory" => Box::<memory::Memory>::default(),
        "disk" => Box::<disk::Disk>::default(),
        "battery" => Box::<battery::Battery>::default(),
        "sensors" => Box::<sensors::Sensors>::default(),
        "clock" => Box::<clock::Clock>::default(),
        _ => return None,
    })
}

pub trait Module {
    /// Ayarlarda ve yapılandırmada kullanılan sabit kimlik ("cpu", "disk"…).
    fn id(&self) -> &'static str;

    /// Açılır menüdeki bilgi satırı sayısı. Kurulumdan sonra değişmez.
    fn rows(&self) -> usize;

    /// Veriyi tazeler ve gösterilecek metinleri üretir.
    fn read(&mut self, cfg: &Config) -> Readout;

    /// Kaç saniyede bir okunacağı; pahalı kaynaklar seyrek yoklanır.
    fn period(&self) -> u32 {
        1
    }
}

/// Eksik veri yerine kullanılan yer tutucu.
pub fn dash() -> String {
    "—".to_string()
}

/// `rows` uzunluğunu sabitler: eksikse yer tutucuyla doldurur, fazlasını atar.
pub fn fit(mut rows: Vec<String>, n: usize) -> Vec<String> {
    rows.truncate(n);
    while rows.len() < n {
        rows.push(dash());
    }
    rows
}
