//! Disk doluluğu ve okuma/yazma hızı.
//!
//! macOS'ta sistem geneli disk I/O sayacı için taşınabilir bir API yok; hız,
//! süreçlerin okuma/yazma toplamlarının iki tur arasındaki farkından çıkarılır.

use super::{Module, Readout};
use crate::config::Config;
use crate::i18n::{t, tn};
use crate::stats::fmt_bytes;
use sysinfo::{Disks, ProcessRefreshKind, RefreshKind, System};

/// Menüyü şişirmemek için listelenecek en fazla birim.
const MAX_VOLUMES: usize = 5;

pub struct Disk {
    disks: Disks,
    sys: System,
    last: (u64, u64),
}

impl Default for Disk {
    fn default() -> Self {
        Self {
            disks: Disks::new_with_refreshed_list(),
            sys: System::new_with_specifics(
                RefreshKind::new()
                    .with_processes(ProcessRefreshKind::new().with_disk_usage()),
            ),
            last: (0, 0),
        }
    }
}

impl Module for Disk {
    fn id(&self) -> &'static str {
        "disk"
    }


    fn read(&mut self, _cfg: &Config) -> Readout {
        self.disks.refresh();
        self.sys.refresh_processes(sysinfo::ProcessesToUpdate::All, true);

        // Süreç sayaçları kümülatif: fark almadan önce toplamı topla.
        let (mut read, mut written) = (0u64, 0u64);
        for p in self.sys.processes().values() {
            let u = p.disk_usage();
            read += u.total_read_bytes;
            written += u.total_written_bytes;
        }
        let delta = (
            read.saturating_sub(self.last.0),
            written.saturating_sub(self.last.1),
        );
        // İlk turda referans yok: hız sıfır sayılır.
        let rate = if self.last == (0, 0) { (0, 0) } else { delta };
        self.last = (read, written);

        // Aynı fiziksel diski birden çok kez listeleme: bağlama noktası bazında tekilleştir.
        let mut seen: Vec<String> = Vec::new();
        let mut rows = Vec::new();
        let mut worst = 0.0f64;
        let mut worst_name = String::new();
        for d in self.disks.list() {
            let mount = d.mount_point().to_string_lossy().to_string();
            // macOS'ta kök birim imzalı ve salt okunur; onu eleme.
            // Elenecek olan, /Volumes altına bağlanmış salt okunur disk
            // imajları (uygulama DMG'leri): hep %100 dolu görünüp doluluk
            // uyarısını yanlış tetikliyorlar. /System/Volumes/Data ise kökle
            // aynı alanı ikinci kez sayar.
            let is_dmg = mount.starts_with("/Volumes/") && d.is_read_only();
            if is_dmg || mount.starts_with("/System/") || mount.starts_with("/dev") {
                continue;
            }
            if seen.contains(&mount) {
                continue;
            }
            let (avail, total) = (d.available_space(), d.total_space());
            if total == 0 {
                continue;
            }
            let used_pct = (total - avail) as f64 / total as f64 * 100.0;
            if used_pct > worst {
                worst = used_pct;
                worst_name = d.name().to_string_lossy().to_string();
            }
            rows.push(tn(
                "disk.volume",
                &[&d.name().to_string_lossy(), &fmt_bytes(avail), &fmt_bytes(total)],
            ));
            seen.push(mount);
            if rows.len() >= MAX_VOLUMES {
                break;
            }
        }

        rows.push(tn(
            "disk.io",
            &[&fmt_bytes(rate.0), &fmt_bytes(rate.1)],
        ));

        Readout::new(format!("{} {worst:.0}%", t("module.disk")), rows)
            .with_level(worst, Some(worst_name))
    }
}
