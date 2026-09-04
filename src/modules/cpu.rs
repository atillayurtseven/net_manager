//! CPU ve GPU kullanımı.
//!
//! GPU tarafı macOS'ta `ioreg`'in IOAccelerator düğümünden okunur; bu düğüm
//! sudo istemez, `powermetrics`'in aksine. Diğer platformlarda GPU satırı boş kalır.

use super::{fit, Module, Readout};
use crate::config::Config;
use crate::i18n::{t, tn};
use sysinfo::{ProcessRefreshKind, RefreshKind, System};

const ROWS: usize = 6;
/// GPU yoklaması CPU'dan pahalı: her turda değil, birkaç saniyede bir.
const GPU_EVERY: u32 = 3;

pub struct Cpu {
    sys: System,
    gpu: Option<f32>,
    ticks: u32,
}

impl Default for Cpu {
    fn default() -> Self {
        Self {
            sys: System::new_with_specifics(
                RefreshKind::new()
                    .with_cpu(sysinfo::CpuRefreshKind::everything())
                    .with_processes(ProcessRefreshKind::new().with_cpu()),
            ),
            gpu: None,
            ticks: 0,
        }
    }
}

fn pct(v: f32) -> String {
    format!("{v:.0}%")
}

/// macOS: IOAccelerator düğümündeki anlık kullanım yüzdesi.
#[cfg(target_os = "macos")]
fn read_gpu() -> Option<f32> {
    let out = std::process::Command::new("ioreg")
        .args(["-r", "-d", "1", "-w", "0", "-c", "IOAccelerator"])
        .output()
        .ok()?;
    let text = String::from_utf8_lossy(&out.stdout);
    let key = "\"Device Utilization %\"=";
    text.lines()
        .find_map(|l| l.split_once(key))
        .and_then(|(_, rest)| {
            let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
            digits.parse::<f32>().ok()
        })
}

#[cfg(not(target_os = "macos"))]
fn read_gpu() -> Option<f32> {
    None
}

impl Module for Cpu {
    fn id(&self) -> &'static str {
        "cpu"
    }

    fn rows(&self) -> usize {
        ROWS
    }

    fn read(&mut self, _cfg: &Config) -> Readout {
        self.sys.refresh_cpu_all();
        self.sys.refresh_processes(sysinfo::ProcessesToUpdate::All, true);

        let usage = self.sys.global_cpu_usage();
        let cores: Vec<f32> = self.sys.cpus().iter().map(|c| c.cpu_usage()).collect();

        self.ticks += 1;
        if self.ticks % GPU_EVERY == 1 || self.gpu.is_none() {
            self.gpu = read_gpu();
        }

        // En çok CPU yiyen üç süreç
        let mut procs: Vec<(String, f32)> = self
            .sys
            .processes()
            .values()
            .map(|p| (p.name().to_string_lossy().to_string(), p.cpu_usage()))
            .collect();
        procs.sort_by(|a, b| b.1.total_cmp(&a.1));
        procs.truncate(3);

        let load = System::load_average();

        let mut rows = vec![
            tn("cpu.usage", &[&pct(usage)]),
            tn(
                "cpu.gpu",
                &[&self.gpu.map(pct).unwrap_or_else(super::dash)],
            ),
            tn(
                "cpu.cores",
                &[
                    &cores.len().to_string(),
                    &cores
                        .iter()
                        .map(|c| pct(*c))
                        .collect::<Vec<_>>()
                        .join(" "),
                ],
            ),
            tn(
                "cpu.load",
                &[&format!("{:.2}  {:.2}  {:.2}", load.one, load.five, load.fifteen)],
            ),
        ];
        rows.extend(
            procs
                .into_iter()
                .map(|(name, u)| tn("cpu.top", &[&format!("{name}  {}", pct(u))])),
        );

        Readout::new(format!("{} {}", t("module.cpu"), pct(usage)), fit(rows, ROWS))
            .with_level(usage as f64, None)
    }
}
