//! SQLite tabanlı trafik istatistikleri: dakikalık geçmiş + günlük zirve hızlar.

use crate::i18n::t;
use chrono::{DateTime, Datelike, Duration, Local, TimeZone, Timelike};
use std::collections::HashMap;
use rusqlite::{params, Connection};
use std::path::PathBuf;

pub struct Stats {
    conn: Connection,
}

fn db_path() -> PathBuf {
    let mut p = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
    p.push("net_manager");
    let _ = std::fs::create_dir_all(&p);
    p.push("stats.db");
    p
}

/// Toplam bayt -> KB/MB/GB/TB
pub fn fmt_bytes(b: u64) -> String {
    let kb = b as f64 / 1024.0;
    if kb < 1024.0 {
        format!("{kb:.0} {}", t("unit.kb"))
    } else if kb < 1024.0 * 1024.0 {
        format!("{:.1} {}", kb / 1024.0, t("unit.mb"))
    } else if kb < 1024.0 * 1024.0 * 1024.0 {
        format!("{:.2} {}", kb / (1024.0 * 1024.0), t("unit.gb"))
    } else {
        format!("{:.2} {}", kb / (1024.0 * 1024.0 * 1024.0), t("unit.tb"))
    }
}

fn start_of_today() -> i64 {
    let now = Local::now();
    Local
        .with_ymd_and_hms(now.year(), now.month(), now.day(), 0, 0, 0)
        .single()
        .map(|d| d.timestamp())
        .unwrap_or(0)
}

fn start_of_month() -> i64 {
    let now = Local::now();
    Local
        .with_ymd_and_hms(now.year(), now.month(), 1, 0, 0, 0)
        .single()
        .map(|d| d.timestamp())
        .unwrap_or(0)
}

fn today_key() -> String {
    Local::now().format("%Y-%m-%d").to_string()
}

impl Stats {
    pub fn open() -> rusqlite::Result<Self> {
        let conn = Connection::open(db_path())?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "synchronous", "NORMAL")?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS traffic (
                 minute INTEGER NOT NULL,
                 iface  TEXT    NOT NULL,
                 rx     INTEGER NOT NULL DEFAULT 0,
                 tx     INTEGER NOT NULL DEFAULT 0,
                 PRIMARY KEY (minute, iface)
             );
             CREATE INDEX IF NOT EXISTS traffic_minute ON traffic(minute);

             CREATE TABLE IF NOT EXISTS peaks (
                 day    TEXT    NOT NULL,
                 iface  TEXT    NOT NULL,
                 max_rx INTEGER NOT NULL DEFAULT 0,
                 max_tx INTEGER NOT NULL DEFAULT 0,
                 PRIMARY KEY (day, iface)
             );",
        )?;
        Ok(Self { conn })
    }

    /// Bir saniyelik ölçümü kaydeder. `samples`: (arayüz, rx, tx)
    pub fn record(&self, samples: &[(String, u64, u64)]) -> rusqlite::Result<()> {
        let minute = Local::now().timestamp() / 60 * 60;
        let day = today_key();
        let tx = self.conn.unchecked_transaction()?;

        // "*" satırı: anlık toplamın zirvesi (arayüz maksimumlarının toplamı değil).
        let sum_rx: u64 = samples.iter().map(|s| s.1).sum();
        let sum_tx: u64 = samples.iter().map(|s| s.2).sum();
        if sum_rx > 0 || sum_tx > 0 {
            tx.execute(
                "INSERT INTO peaks (day, iface, max_rx, max_tx) VALUES (?1, '*', ?2, ?3)
                 ON CONFLICT(day, iface) DO UPDATE SET
                     max_rx = MAX(max_rx, ?2),
                     max_tx = MAX(max_tx, ?3)",
                params![day, sum_rx, sum_tx],
            )?;
        }

        for (iface, rx, tx_bytes) in samples {
            if *rx == 0 && *tx_bytes == 0 {
                continue;
            }
            tx.execute(
                "INSERT INTO traffic (minute, iface, rx, tx) VALUES (?1, ?2, ?3, ?4)
                 ON CONFLICT(minute, iface) DO UPDATE SET rx = rx + ?3, tx = tx + ?4",
                params![minute, iface, rx, tx_bytes],
            )?;
            tx.execute(
                "INSERT INTO peaks (day, iface, max_rx, max_tx) VALUES (?1, ?2, ?3, ?4)
                 ON CONFLICT(day, iface) DO UPDATE SET
                     max_rx = MAX(max_rx, ?3),
                     max_tx = MAX(max_tx, ?4)",
                params![day, iface, rx, tx_bytes],
            )?;
        }
        tx.commit()
    }

    /// `since` zamanından bu yana toplam (rx, tx). `iface` None ise tümü.
    fn total_since(&self, since: i64, iface: Option<&str>) -> (u64, u64) {
        let row = match iface {
            Some(i) => self.conn.query_row(
                "SELECT COALESCE(SUM(rx),0), COALESCE(SUM(tx),0)
                 FROM traffic WHERE minute >= ?1 AND iface = ?2",
                params![since, i],
                |r| Ok((r.get(0)?, r.get(1)?)),
            ),
            None => self.conn.query_row(
                "SELECT COALESCE(SUM(rx),0), COALESCE(SUM(tx),0)
                 FROM traffic WHERE minute >= ?1",
                params![since],
                |r| Ok((r.get(0)?, r.get(1)?)),
            ),
        };
        row.unwrap_or((0, 0))
    }

    pub fn today(&self, iface: Option<&str>) -> (u64, u64) {
        self.total_since(start_of_today(), iface)
    }

    pub fn month(&self, iface: Option<&str>) -> (u64, u64) {
        self.total_since(start_of_month(), iface)
    }

    /// Bugünün zirve hızları (bayt/sn).
    pub fn peak_today(&self, iface: Option<&str>) -> (u64, u64) {
        let day = today_key();
        let row = match iface {
            Some(i) => self.conn.query_row(
                "SELECT COALESCE(MAX(max_rx),0), COALESCE(MAX(max_tx),0)
                 FROM peaks WHERE day = ?1 AND iface = ?2",
                params![day, i],
                |r| Ok((r.get(0)?, r.get(1)?)),
            ),
            None => self.conn.query_row(
                "SELECT COALESCE(MAX(max_rx),0), COALESCE(MAX(max_tx),0)
                 FROM peaks WHERE day = ?1 AND iface = '*'",
                params![day],
                |r| Ok((r.get(0)?, r.get(1)?)),
            ),
        };
        row.unwrap_or((0, 0))
    }
}

/// Zaman kırılımı.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Bucket {
    Hour,
    Day,
    Week,
    Month,
}

impl Bucket {
    pub fn key(self) -> &'static str {
        match self {
            Bucket::Hour => "hour",
            Bucket::Day => "day",
            Bucket::Week => "week",
            Bucket::Month => "month",
        }
    }

    pub fn title(self) -> String {
        t(match self {
            Bucket::Hour => "bucket.hour",
            Bucket::Day => "bucket.day",
            Bucket::Week => "bucket.week",
            Bucket::Month => "bucket.month",
        })
    }

    pub fn from_key(k: &str) -> Option<Self> {
        Some(match k {
            "hour" => Bucket::Hour,
            "day" => Bucket::Day,
            "week" => Bucket::Week,
            "month" => Bucket::Month,
            _ => return None,
        })
    }

    /// SQLite ve chrono'da aynı anahtarı üreten biçim.
    fn fmt(self) -> &'static str {
        match self {
            Bucket::Hour => "%Y-%m-%d %H",
            Bucket::Day => "%Y-%m-%d",
            Bucket::Week => "%Y-%W",
            Bucket::Month => "%Y-%m",
        }
    }

    /// Eksende gösterilecek kısa etiket. Biçim dile göre değişir.
    fn label(self, d: DateTime<Local>) -> String {
        d.format(&t(&format!("format.{}", self.key()))).to_string()
    }

    /// Tooltip için uzun etiket.
    fn long_label(self, d: DateTime<Local>) -> String {
        d.format(&t(&format!("format.{}_long", self.key()))).to_string()
    }
}

/// Hazır zaman aralıkları.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Range {
    Last24h,
    Last30d,
    Last60d,
    Last90d,
    ThisMonth,
    LastMonth,
    Last3Months,
    All,
}

pub const RANGES: [Range; 8] = [
    Range::Last24h,
    Range::Last30d,
    Range::Last60d,
    Range::Last90d,
    Range::ThisMonth,
    Range::LastMonth,
    Range::Last3Months,
    Range::All,
];

impl Range {
    pub fn key(self) -> &'static str {
        match self {
            Range::Last24h => "24h",
            Range::Last30d => "30d",
            Range::Last60d => "60d",
            Range::Last90d => "90d",
            Range::ThisMonth => "this_month",
            Range::LastMonth => "last_month",
            Range::Last3Months => "3m",
            Range::All => "all",
        }
    }

    pub fn title(self) -> String {
        t(&format!("range.{}", self.key()))
    }

    pub fn from_key(k: &str) -> Option<Self> {
        RANGES.into_iter().find(|r| r.key() == k)
    }

    /// Aralığa uygun varsayılan kırılım.
    pub fn default_bucket(self) -> Bucket {
        match self {
            Range::Last24h => Bucket::Hour,
            Range::Last30d | Range::ThisMonth | Range::LastMonth => Bucket::Day,
            Range::Last60d | Range::Last90d | Range::Last3Months => Bucket::Day,
            Range::All => Bucket::Month,
        }
    }
}

fn add_months(d: DateTime<Local>, n: i32) -> DateTime<Local> {
    let total = d.year() * 12 + d.month0() as i32 + n;
    let (y, m0) = (total.div_euclid(12), total.rem_euclid(12));
    Local
        .with_ymd_and_hms(y, m0 as u32 + 1, 1, 0, 0, 0)
        .single()
        .unwrap_or(d)
}

fn day_start(d: DateTime<Local>) -> DateTime<Local> {
    Local
        .with_ymd_and_hms(d.year(), d.month(), d.day(), 0, 0, 0)
        .single()
        .unwrap_or(d)
}

impl Stats {
    /// Kayıtlı en eski ölçümün zamanı.
    fn earliest(&self) -> Option<i64> {
        self.conn
            .query_row("SELECT MIN(minute) FROM traffic", [], |r| {
                r.get::<_, Option<i64>>(0)
            })
            .ok()
            .flatten()
    }

    /// Aralığın [başlangıç, bitiş) sınırları.
    fn bounds(&self, range: Range) -> (DateTime<Local>, DateTime<Local>) {
        let now = Local::now();
        let today = day_start(now);
        let tomorrow = today + Duration::days(1);
        let month_start = add_months(now, 0);

        match range {
            Range::Last24h => {
                let h = now - Duration::hours(23);
                let start = Local
                    .with_ymd_and_hms(h.year(), h.month(), h.day(), h.hour(), 0, 0)
                    .single()
                    .unwrap_or(h);
                (start, now + Duration::hours(1))
            }
            Range::Last30d => (today - Duration::days(29), tomorrow),
            Range::Last60d => (today - Duration::days(59), tomorrow),
            Range::Last90d => (today - Duration::days(89), tomorrow),
            Range::ThisMonth => (month_start, tomorrow),
            Range::LastMonth => (add_months(now, -1), month_start),
            Range::Last3Months => (add_months(now, -2), tomorrow),
            Range::All => {
                let first = self
                    .earliest()
                    .and_then(|t| Local.timestamp_opt(t, 0).single())
                    .unwrap_or(today);
                (day_start(first), tomorrow)
            }
        }
    }

    /// Grafik serisi: (kısa etiket, uzun etiket, rx, tx). Boş kovalar sıfırla doldurulur.
    pub fn series(
        &self,
        range: Range,
        bucket: Bucket,
        iface: Option<&str>,
    ) -> Vec<(String, String, u64, u64)> {
        let (from, to) = self.bounds(range);
        let (from_ts, to_ts) = (from.timestamp(), to.timestamp());

        let sql = format!(
            "SELECT strftime('{}', minute, 'unixepoch', 'localtime') AS k, SUM(rx), SUM(tx)
             FROM traffic WHERE minute >= ?1 AND minute < ?2 {}
             GROUP BY k",
            bucket.fmt(),
            if iface.is_some() { "AND iface = ?3" } else { "" }
        );

        let mut found: HashMap<String, (u64, u64)> = HashMap::new();
        let mut query = || -> rusqlite::Result<()> {
            let mut stmt = self.conn.prepare(&sql)?;
            let mut rows = match iface {
                Some(i) => stmt.query(params![from_ts, to_ts, i])?,
                None => stmt.query(params![from_ts, to_ts])?,
            };
            while let Some(r) = rows.next()? {
                let k: String = r.get(0)?;
                found.insert(k, (r.get(1)?, r.get(2)?));
            }
            Ok(())
        };
        let _ = query();

        // Kova dizisini üret; SQLite ile aynı biçim anahtarı kullanılır.
        let mut out = Vec::new();
        let mut cursor = from;
        let mut seen: Vec<String> = Vec::new();
        while cursor < to {
            let key = cursor.format(bucket.fmt()).to_string();
            if !seen.contains(&key) {
                let (rx, tx) = found.get(&key).copied().unwrap_or((0, 0));
                out.push((bucket.label(cursor), bucket.long_label(cursor), rx, tx));
                seen.push(key);
            }
            cursor = match bucket {
                Bucket::Hour => cursor + Duration::hours(1),
                Bucket::Day => cursor + Duration::days(1),
                Bucket::Week => cursor + Duration::days(7),
                Bucket::Month => add_months(cursor, 1),
            };
            if out.len() > 2000 {
                break;
            }
        }
        out
    }
}

/// Grafikte gösterilecek ölçüt.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Metric {
    Both,
    Dl,
    Ul,
    Total,
}

pub const METRICS: [Metric; 4] = [Metric::Both, Metric::Dl, Metric::Ul, Metric::Total];

impl Metric {
    pub fn key(self) -> &'static str {
        match self {
            Metric::Both => "both",
            Metric::Dl => "dl",
            Metric::Ul => "ul",
            Metric::Total => "total",
        }
    }

    pub fn title(self) -> String {
        t(&format!("metric.{}", self.key()))
    }

    pub fn from_key(k: &str) -> Option<Self> {
        METRICS.into_iter().find(|m| m.key() == k)
    }
}

impl Stats {
    /// Aralık içinde arayüz bazında toplamlar: (arayüz, rx, tx), toplama göre azalan.
    pub fn totals_by_iface(&self, range: Range) -> Vec<(String, u64, u64)> {
        let (from, to) = self.bounds(range);
        let mut out = Vec::new();
        let _ = (|| -> rusqlite::Result<()> {
            let mut stmt = self.conn.prepare(
                "SELECT iface, SUM(rx), SUM(tx) FROM traffic
                 WHERE minute >= ?1 AND minute < ?2
                 GROUP BY iface ORDER BY SUM(rx) + SUM(tx) DESC",
            )?;
            let mut rows = stmt.query(params![from.timestamp(), to.timestamp()])?;
            while let Some(r) = rows.next()? {
                out.push((r.get(0)?, r.get(1)?, r.get(2)?));
            }
            Ok(())
        })();
        out
    }
}
