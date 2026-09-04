//! Saat: yerel zaman, dünya saatleri ve ay evresi.

use super::{fit, Module, Readout};
use crate::config::Config;
use crate::i18n::{t, tn};
use chrono::{Local, Utc};
use chrono_tz::Tz;

const ROWS: usize = 5;

#[derive(Default)]
pub struct Clock;

/// Ay evresi: bilinen bir yeni aydan (6 Ocak 2000, 18:14 UTC) geçen sinodik
/// ay sayısının kesirli kısmı. Takvim gösterimi için yeterli doğrulukta.
fn moon_phase() -> &'static str {
    const SYNODIC: f64 = 29.530_588_853;
    const KNOWN_NEW_MOON: i64 = 947_182_440; // 2000-01-06 18:14 UTC
    let days = (Utc::now().timestamp() - KNOWN_NEW_MOON) as f64 / 86_400.0;
    let age = days.rem_euclid(SYNODIC) / SYNODIC;

    // Sekiz evre; dördün ve dolunay adları dar bir bant kaplar.
    match (age * 8.0).round() as i64 % 8 {
        0 => "moon.new",
        1 => "moon.waxing_crescent",
        2 => "moon.first_quarter",
        3 => "moon.waxing_gibbous",
        4 => "moon.full",
        5 => "moon.waning_gibbous",
        6 => "moon.last_quarter",
        _ => "moon.waning_crescent",
    }
}

impl Module for Clock {
    fn id(&self) -> &'static str {
        "clock"
    }

    fn rows(&self) -> usize {
        ROWS
    }

    fn read(&mut self, cfg: &Config) -> Readout {
        let now = Local::now();
        let time_fmt = t("format.time");
        let date_fmt = t("format.day_long");

        let mut rows = vec![now.format(&date_fmt).to_string()];

        // Kullanıcının seçtiği bölgeler; tanınmayan ad sessizce atlanır.
        rows.extend(
            cfg.timezones
                .iter()
                .filter_map(|name| {
                    let tz: Tz = name.parse().ok()?;
                    let there = now.with_timezone(&tz);
                    // "Europe/Istanbul" yerine "Istanbul" göster.
                    let label = name.rsplit('/').next().unwrap_or(name).replace('_', " ");
                    Some(format!("{label}  {}", there.format(&time_fmt)))
                })
                .take(ROWS - 2),
        );

        rows.push(tn("clock.moon", &[&t(moon_phase())]));

        Readout::new(now.format(&time_fmt).to_string(), fit(rows, ROWS))
    }
}
