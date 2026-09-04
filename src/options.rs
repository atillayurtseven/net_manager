//! Seçenekler penceresinin içeriği.
//!
//! Her denetim değişince `window.ipc.postMessage("opt.<alan>:<değer>")` yollar;
//! Rust tarafı ayarı kaydedip pencereyi yeniden çizer.

use crate::config::Config;
use crate::i18n;
use crate::units::{Mode, ALL as UNITS, MODES};

fn esc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// `<option>` listesi; `sel` olan seçili gelir.
fn options(items: &[(String, String)], sel: &str) -> String {
    items
        .iter()
        .map(|(value, label)| {
            format!(
                r#"<option value="{}"{}>{}</option>"#,
                esc(value),
                if value == sel { " selected" } else { "" },
                esc(label)
            )
        })
        .collect()
}

fn row(label: &str, control: String) -> String {
    format!(
        r#"<div class="row"><label>{}</label><div class="ctl">{control}</div></div>"#,
        esc(label)
    )
}

fn select(field: &str, items: &[(String, String)], sel: &str) -> String {
    format!(
        r#"<select data-field="{field}">{}</select>"#,
        options(items, sel)
    )
}

/// `ifaces`: (id, etiket) — grafik penceresiyle aynı liste.
pub fn html(cfg: &Config, ifaces: &[(String, String)]) -> String {
    let mut langs: Vec<(String, String)> = vec![(
        "system".to_string(),
        i18n::t("options.language_system"),
    )];
    langs.extend(
        i18n::available()
            .into_iter()
            .map(|(code, name)| (code.to_string(), name.to_string())),
    );
    let lang_sel = cfg.lang.clone().unwrap_or_else(|| "system".to_string());

    let mut iface_items: Vec<(String, String)> =
        vec![("*".to_string(), i18n::t("options.interface_all"))];
    iface_items.extend(ifaces.iter().cloned());
    let iface_sel = cfg.iface.clone().unwrap_or_else(|| "*".to_string());

    let modes: Vec<(String, String)> = MODES
        .iter()
        .map(|m| (m.key().to_string(), m.label()))
        .collect();

    let units = |mode: Mode| -> Vec<(String, String)> {
        UNITS
            .iter()
            .map(|u| (u.key().to_string(), u.label(mode).to_string()))
            .collect()
    };

    let body = [
        row(&i18n::t("options.language"), select("lang", &langs, &lang_sel)),
        row(
            &i18n::t("options.interface"),
            select("iface", &iface_items, &iface_sel),
        ),
        row(
            &i18n::t("options.unit_system"),
            select("mode", &modes, cfg.unit_mode.key()),
        ),
        row(
            &i18n::t("options.min_unit"),
            select("min", &units(cfg.unit_mode), cfg.min_unit.key()),
        ),
        row(
            &i18n::t("options.max_unit"),
            select("max", &units(cfg.unit_mode), cfg.max_unit.key()),
        ),
        row(
            &i18n::t("options.autostart"),
            format!(
                r#"<input type="checkbox" data-field="autostart"{}>"#,
                if cfg.autostart { " checked" } else { "" }
            ),
        ),
    ]
    .concat();

    format!(
        r##"<!doctype html><html lang="{lang}"><head><meta charset="utf-8"><title>{title}</title>
<style>
:root {{
  --bg:#fff; --fg:#1c1f26; --muted:#6b7280; --line:#e3e5ea; --card:#f6f7f9;
}}
@media (prefers-color-scheme: dark) {{
  :root {{ --bg:#16181d; --fg:#e8eaed; --muted:#9aa1ac; --line:#2a2e36; --card:#1e2128; }}
}}
* {{ box-sizing:border-box; }}
body {{ margin:0; padding:18px 20px; background:var(--bg); color:var(--fg);
  font:13px -apple-system,BlinkMacSystemFont,"Segoe UI",sans-serif; }}
.row {{ display:flex; align-items:center; gap:14px; padding:9px 0; border-bottom:1px solid var(--line); }}
.row:last-of-type {{ border-bottom:0; }}
.row label {{ flex:none; width:160px; color:var(--muted); }}
.ctl {{ flex:1; }}
select {{ font:inherit; width:100%; max-width:280px; padding:5px 8px; border-radius:6px;
  border:1px solid var(--line); background:var(--card); color:var(--fg); }}
input[type=checkbox] {{ width:16px; height:16px; accent-color:#2f6df6; }}
.note {{ margin-top:14px; color:var(--muted); font-size:11.5px; }}
</style></head><body>
{body}
<div class="note">{note}</div>
<script>
  function send(el) {{
    var v = el.type === 'checkbox' ? (el.checked ? '1' : '0') : el.value;
    window.ipc.postMessage('opt.' + el.dataset.field + ':' + v);
  }}
  document.querySelectorAll('[data-field]').forEach(function (el) {{
    el.addEventListener('change', function () {{ send(el); }});
  }});
</script></body></html>"##,
        lang = i18n::active(),
        title = esc(&i18n::t("window.options")),
        note = esc(&i18n::t("options.restart_note")),
    )
}
