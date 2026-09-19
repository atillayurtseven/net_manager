//! Menü çubuğu modülleri.
//!
//! Her modül kendi menü çubuğu öğesini alır: solda kısa bir gösterge metni,
//! tıklanınca okunabilen veriler. Satır sayısı turdan tura değişebilir;
//! menü buna göre kısalıp uzar, okunamayan veri için boş satır bırakılmaz.

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
    /// Açılır menüdeki bilgi satırları; yalnızca okunabilenler.
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

    /// Veriyi tazeler ve gösterilecek metinleri üretir.
    fn read(&mut self, cfg: &Config) -> Readout;

    /// Kaç saniyede bir okunacağı; pahalı kaynaklar seyrek yoklanır.
    fn period(&self) -> u32 {
        1
    }
}

/// Tek bir alan okunamadığında değerinin yerine yazılan işaret.
/// Satırın tamamı boşsa satır hiç eklenmez, bu yalnızca alan içindir.
pub fn dash() -> String {
    "—".to_string()
}
