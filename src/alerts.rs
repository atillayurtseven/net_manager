//! Eşik uyarıları.
//!
//! Bildirim macOS'un kendi merkezinden `osascript` ile gönderilir; paket
//! imzalı olduğu için ek yetki gerekmez. Aynı uyarı sürekli tekrarlamasın
//! diye her tür için bekleme süresi tutulur.

use crate::i18n::tn;
use std::collections::HashMap;
use std::time::{Duration, Instant};

/// Aynı uyarının yeniden gönderilmesi için beklenecek süre.
const COOLDOWN: Duration = Duration::from_secs(600);

#[derive(Default)]
pub struct Alerts {
    last: HashMap<String, Instant>,
}

#[cfg(target_os = "macos")]
fn notify(title: &str, body: &str) {
    // Tırnaklar AppleScript dizesini bozmasın.
    let esc = |s: &str| s.replace('\\', "\\\\").replace('"', "\\\"");
    let script = format!(
        "display notification \"{}\" with title \"{}\"",
        esc(body),
        esc(title)
    );
    let _ = std::process::Command::new("osascript")
        .args(["-e", &script])
        .spawn();
}

#[cfg(not(target_os = "macos"))]
fn notify(_title: &str, _body: &str) {}

impl Alerts {
    /// `value` eşiği aştıysa uyarır. `threshold` 0 ise uyarı kapalıdır.
    pub fn check(&mut self, kind: &str, key: &str, threshold: u8, value: f64, arg: Option<&str>) {
        if threshold == 0 || value < threshold as f64 {
            // Değer normale döndüyse bir sonraki aşımda hemen uyarabilelim.
            self.last.remove(kind);
            return;
        }
        let fresh = self
            .last
            .get(kind)
            .is_none_or(|t| t.elapsed() >= COOLDOWN);
        if !fresh {
            return;
        }
        self.last.insert(kind.to_string(), Instant::now());

        let pct = format!("{value:.0}%");
        let body = match arg {
            Some(a) => tn(key, &[a, &pct]),
            None => tn(key, &[&pct]),
        };
        notify(&crate::i18n::t("app.name"), &body);
    }
}
