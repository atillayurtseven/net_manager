//! Görüntülenecek ağ arayüzlerinin tespiti ve okunabilir isimlendirmesi.

use std::collections::HashMap;
use sysinfo::Networks;

/// Hiç sayılmayan arayüzler: loopback ve tünel kapsülleme sözde-arayüzleri.
/// Bunların baytları ya gerçek değil (lo) ya da başka bir arayüzde zaten sayılıyor.
const HIDDEN_PREFIXES: [&str; 3] = ["lo", "gif", "stf"];

/// Listeye girmek için gereken en az toplam trafik (1 MiB).
const MIN_TOTAL_BYTES: u64 = 1024 * 1024;

fn has_prefix(name: &str, p: &str) -> bool {
    name.starts_with(p) && name[p.len()..].chars().all(|c| c.is_ascii_digit())
}

fn is_hidden(name: &str) -> bool {
    HIDDEN_PREFIXES.iter().any(|p| has_prefix(name, p))
}

/// VPN tüneli mi? Tünel trafiği fiziksel arayüzde de sayıldığı için
/// "Tümü" toplamına katılmaz, ama ayrı ayrı seçilebilir.
fn is_tunnel(name: &str) -> bool {
    has_prefix(name, "utun") || has_prefix(name, "ipsec") || has_prefix(name, "ppp")
}

/// macOS: en0 -> "Wi-Fi", en5 -> "USB 10/100/1000 LAN" eşlemesi.
#[cfg(target_os = "macos")]
fn hardware_ports() -> HashMap<String, String> {
    let mut map = HashMap::new();
    let Ok(out) = std::process::Command::new("networksetup")
        .arg("-listallhardwareports")
        .output()
    else {
        return map;
    };
    let text = String::from_utf8_lossy(&out.stdout);
    let mut port: Option<String> = None;
    for line in text.lines() {
        if let Some(p) = line.strip_prefix("Hardware Port: ") {
            port = Some(p.trim().to_string());
        } else if let Some(dev) = line.strip_prefix("Device: ") {
            if let Some(p) = port.take() {
                map.insert(dev.trim().to_string(), p);
            }
        }
    }
    map
}

#[cfg(not(target_os = "macos"))]
fn hardware_ports() -> HashMap<String, String> {
    HashMap::new()
}

pub struct Iface {
    pub id: String,
    pub label: String,
}

/// Trafiği olan, gösterilmeye değer arayüzleri okunabilir etiketleriyle döndürür.
pub fn list(networks: &Networks) -> Vec<Iface> {
    let ports = hardware_ports();

    let mut out: Vec<Iface> = networks
        .iter()
        .filter(|(name, data)| {
            // Sadece kayda değer trafik görmüş arayüzler; onlarca boş utun listeyi doldurmasın.
            !is_hidden(name)
                && (data.total_received() + data.total_transmitted()) > MIN_TOTAL_BYTES
        })
        .map(|(name, _)| {
            let label = match ports.get(name.as_str()) {
                Some(p) => format!("{p} ({name})"),
                None if is_tunnel(name) => format!("VPN ({name})"),
                None => name.to_string(),
            };
            Iface { id: name.to_string(), label }
        })
        .collect();

    out.sort_by(|a, b| a.label.to_lowercase().cmp(&b.label.to_lowercase()));
    out
}

/// "Tümü" toplamına dahil edilecek mi?
/// Tüneller hariç: VPN'den geçen baytlar fiziksel arayüzde zaten sayılıyor.
pub fn counts_toward_total(name: &str) -> bool {
    !is_hidden(name) && !is_tunnel(name)
}

/// İstatistik kaydına girecek mi? Tüneller de kaydedilir (ayrı ayrı incelenebilsin).
pub fn is_recordable(name: &str) -> bool {
    !is_hidden(name)
}
