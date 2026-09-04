//! Bellek ve takas kullanımı.

use super::{fit, Module, Readout};
use crate::config::Config;
use crate::i18n::{t, tn};
use crate::stats::fmt_bytes;
use sysinfo::{MemoryRefreshKind, ProcessRefreshKind, RefreshKind, System};

const ROWS: usize = 6;

pub struct Memory {
    sys: System,
}

impl Default for Memory {
    fn default() -> Self {
        Self {
            sys: System::new_with_specifics(
                RefreshKind::new()
                    .with_memory(MemoryRefreshKind::everything())
                    .with_processes(ProcessRefreshKind::new().with_memory()),
            ),
        }
    }
}

impl Module for Memory {
    fn id(&self) -> &'static str {
        "memory"
    }

    fn rows(&self) -> usize {
        ROWS
    }

    fn read(&mut self, _cfg: &Config) -> Readout {
        self.sys.refresh_memory();
        self.sys.refresh_processes(sysinfo::ProcessesToUpdate::All, true);

        let (used, total) = (self.sys.used_memory(), self.sys.total_memory());
        let free = total.saturating_sub(used);
        let (swap_used, swap_total) = (self.sys.used_swap(), self.sys.total_swap());
        let pct = if total > 0 {
            used as f64 / total as f64 * 100.0
        } else {
            0.0
        };

        let mut procs: Vec<(String, u64)> = self
            .sys
            .processes()
            .values()
            .map(|p| (p.name().to_string_lossy().to_string(), p.memory()))
            .collect();
        procs.sort_by(|a, b| b.1.cmp(&a.1));
        procs.truncate(3);

        let mut rows = vec![
            tn("mem.used", &[&fmt_bytes(used), &fmt_bytes(total)]),
            tn("mem.free", &[&fmt_bytes(free)]),
            tn(
                "mem.swap",
                &[&fmt_bytes(swap_used), &fmt_bytes(swap_total)],
            ),
        ];
        rows.extend(
            procs
                .into_iter()
                .map(|(name, m)| tn("mem.top", &[&format!("{name}  {}", fmt_bytes(m))])),
        );

        Readout::new(format!("{} {pct:.0}%", t("module.memory")), fit(rows, ROWS))
            .with_level(pct, None)
    }
}
