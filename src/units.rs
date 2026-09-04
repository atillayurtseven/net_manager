//! Hız birimi ölçekleme.
//!
//! İki sistem: bayt (ikilik taban, 1 KB = 1024 B) ve bit (SI taban, 1 Kbps = 1000 bit/s).
//! ISP hız tanımlarıyla uyuşması için bit tarafı bilerek 1000'lik.

use crate::i18n::t;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Mode {
    #[default]
    Byte,
    Bit,
}

pub const MODES: [Mode; 2] = [Mode::Byte, Mode::Bit];

impl Mode {
    pub fn label(self) -> String {
        t(match self {
            Mode::Byte => "unit.byte",
            Mode::Bit => "unit.bit",
        })
    }

    pub fn key(self) -> &'static str {
        match self {
            Mode::Byte => "byte",
            Mode::Bit => "bit",
        }
    }

    pub fn from_key(k: &str) -> Option<Self> {
        Some(match k {
            "byte" => Mode::Byte,
            "bit" => Mode::Bit,
            _ => return None,
        })
    }

    fn base(self) -> f64 {
        match self {
            Mode::Byte => 1024.0,
            Mode::Bit => 1000.0,
        }
    }
}

/// Büyüklük basamağı. Etiketi seçili moda göre değişir.
#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum Unit {
    KB,
    MB,
    GB,
    TB,
}

pub const ALL: [Unit; 4] = [Unit::KB, Unit::MB, Unit::GB, Unit::TB];

impl Unit {
    /// 1000/1024 üssü: K=1, M=2, G=3, T=4
    fn exp(self) -> i32 {
        match self {
            Unit::KB => 1,
            Unit::MB => 2,
            Unit::GB => 3,
            Unit::TB => 4,
        }
    }

    pub fn label(self, mode: Mode) -> &'static str {
        match (mode, self) {
            (Mode::Byte, Unit::KB) => "KB",
            (Mode::Byte, Unit::MB) => "MB",
            (Mode::Byte, Unit::GB) => "GB",
            (Mode::Byte, Unit::TB) => "TB",
            (Mode::Bit, Unit::KB) => "Kbps",
            (Mode::Bit, Unit::MB) => "Mbps",
            (Mode::Bit, Unit::GB) => "Gbps",
            (Mode::Bit, Unit::TB) => "Tbps",
        }
    }

    pub fn key(self) -> &'static str {
        match self {
            Unit::KB => "kb",
            Unit::MB => "mb",
            Unit::GB => "gb",
            Unit::TB => "tb",
        }
    }

    pub fn from_key(k: &str) -> Option<Self> {
        ALL.into_iter().find(|u| u.key() == k)
    }

    fn divisor(self, mode: Mode) -> f64 {
        mode.base().powi(self.exp())
    }
}

/// Bayt/saniyeyi seçili sistem ve `min`..`max` birim aralığıyla biçimler.
pub fn fmt_rate(bytes_per_sec: f64, mode: Mode, min: Unit, max: Unit) -> String {
    let (min, max) = if min <= max { (min, max) } else { (max, min) };

    let value = match mode {
        Mode::Byte => bytes_per_sec,
        Mode::Bit => bytes_per_sec * 8.0,
    };

    // Doğal birimi bul, sonra aralığa kıstır.
    let mut unit = min;
    for u in ALL {
        if u > max {
            break;
        }
        if u >= min && value >= u.divisor(mode) {
            unit = u;
        }
    }

    let v = value / unit.divisor(mode);
    let decimals = if v < 10.0 {
        2
    } else if v < 100.0 {
        1
    } else {
        0
    };

    let suffix = if mode == Mode::Byte { "/s" } else { "" };
    format!("{v:.decimals$} {}{suffix}", unit.label(mode))
}
