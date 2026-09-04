//! Trafik grafiği penceresinin içeriği: veriyi gömülü SVG + küçük bir JS ile HTML'e çevirir.
//! Butonlar `window.ipc.postMessage` ile Rust tarafına seçim bildirir.

use crate::i18n::{active, t, t1};
use crate::stats::{fmt_bytes, Bucket, Metric, Range, METRICS, RANGES};

const W: f64 = 900.0;
const H: f64 = 340.0;
const PAD_L: f64 = 74.0;
const PAD_R: f64 = 16.0;
const PAD_T: f64 = 16.0;
const PAD_B: f64 = 34.0;

const BUCKETS: [Bucket; 4] = [Bucket::Hour, Bucket::Day, Bucket::Week, Bucket::Month];

/// Grafiğe verilecek her şey.
pub struct View<'a> {
    pub range: Range,
    pub bucket: Bucket,
    pub metric: Metric,
    /// None = tüm arayüzler
    pub iface: Option<&'a str>,
    /// Seçim listesi: (id, etiket)
    pub ifaces: &'a [(String, String)],
    /// Seri: (kısa etiket, uzun etiket, rx, tx)
    pub data: &'a [(String, String, u64, u64)],
    /// Arayüz dökümü: (etiket, rx, tx)
    pub by_iface: &'a [(String, u64, u64)],
}

/// Eksen üst sınırı: 1024'lük tabanda yuvarlak bir değer.
fn nice_ceil(v: f64) -> f64 {
    if v <= 0.0 {
        return 1024.0;
    }
    let mut unit = 1024.0f64;
    while v / unit >= 1024.0 {
        unit *= 1024.0;
    }
    let m = v / unit;
    const STEPS: [f64; 10] = [1.0, 2.0, 4.0, 8.0, 16.0, 32.0, 64.0, 128.0, 256.0, 512.0];
    let step = STEPS.iter().copied().find(|&s| s >= m).unwrap_or(1024.0);
    step * unit
}

fn esc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn seg(id: &str, active: bool, key: &str, label: &str) -> String {
    format!(
        r#"<button class="seg{}" data-kind="{id}" data-key="{key}">{}</button>"#,
        if active { " on" } else { "" },
        esc(label)
    )
}

/// Tooltip gövdesini katalogdaki biçimden JS ifadesine çevirir:
/// "↓ {}<br>↑ {}" -> "'↓ ' + g.dataset.dl + '<br>↑ ' + g.dataset.ul"
fn tooltip_js() -> String {
    let fields = ["g.dataset.dl", "g.dataset.ul", "g.dataset.tot"];
    let mut out = String::new();
    for (i, part) in t("chart.tooltip").split("{}").enumerate() {
        if i > 0 {
            if let Some(f) = fields.get(i - 1) {
                out.push_str(&format!(" + {f}"));
            }
        }
        if !part.is_empty() {
            let lit = part.replace('\\', "\\\\").replace('\'', "\\'");
            if i > 0 {
                out.push_str(" + ");
            }
            out.push_str(&format!("'{lit}'"));
        }
    }
    out
}

pub fn html(v: &View) -> String {
    let n = v.data.len().max(1) as f64;
    let plot_w = W - PAD_L - PAD_R;
    let plot_h = H - PAD_T - PAD_B;

    // Seçili ölçüte göre çubuk değerleri
    let vals: Vec<(f64, f64)> = v
        .data
        .iter()
        .map(|(_, _, rx, tx)| match v.metric {
            Metric::Both => (*rx as f64, *tx as f64),
            Metric::Dl => (*rx as f64, -1.0),
            Metric::Ul => (*tx as f64, -1.0),
            Metric::Total => ((*rx + *tx) as f64, -1.0),
        })
        .collect();

    let max_val = vals
        .iter()
        .map(|(a, b)| a.max(*b))
        .fold(0.0f64, f64::max);
    let y_max = nice_ceil(max_val);

    let slot = plot_w / n;
    let paired = v.metric == Metric::Both;
    let bar_w = if paired {
        (slot * 0.34).min(18.0)
    } else {
        (slot * 0.66).min(30.0)
    };

    // Etiket sıklığı: çubuklar sıklaşınca atla
    let label_step = ((n / 16.0).ceil() as usize).max(1);

    let mut bars = String::new();
    let mut ticks = String::new();

    for (i, ((_, long, rx, tx), (a, b))) in v.data.iter().zip(vals.iter()).enumerate() {
        let cx = PAD_L + slot * (i as f64 + 0.5);
        let bar = |value: f64, x: f64, class: &str| -> String {
            let h = (value / y_max) * plot_h;
            if h <= 0.0 {
                return String::new();
            }
            format!(
                r#"<rect x="{x:.1}" y="{:.1}" width="{bar_w:.1}" height="{h:.1}" rx="2" class="{class}"/>"#,
                PAD_T + plot_h - h
            )
        };

        let shapes = if paired {
            format!(
                "{}{}",
                bar(*a, cx - bar_w - 1.0, "dl"),
                bar(*b, cx + 1.0, "ul")
            )
        } else {
            let class = match v.metric {
                Metric::Ul => "ul",
                Metric::Total => "tt",
                _ => "dl",
            };
            bar(*a, cx - bar_w / 2.0, class)
        };

        bars.push_str(&format!(
            r#"<g class="bar" data-t="{}" data-dl="{}" data-ul="{}" data-tot="{}">{shapes}<rect x="{:.1}" y="{PAD_T:.1}" width="{slot:.1}" height="{plot_h:.1}" fill="transparent"/></g>"#,
            esc(long),
            esc(&fmt_bytes(*rx)),
            esc(&fmt_bytes(*tx)),
            esc(&fmt_bytes(rx + tx)),
            cx - slot / 2.0,
        ));

        if i % label_step == 0 {
            ticks.push_str(&format!(
                r#"<text x="{cx:.1}" y="{:.1}" class="xlab">{}</text>"#,
                PAD_T + plot_h + 16.0,
                esc(&v.data[i].0)
            ));
        }
    }

    let mut grid = String::new();
    for k in 0..=4 {
        let frac = k as f64 / 4.0;
        let y = PAD_T + plot_h - frac * plot_h;
        grid.push_str(&format!(
            r#"<line x1="{PAD_L:.1}" y1="{y:.1}" x2="{:.1}" y2="{y:.1}" class="grid"/><text x="{:.1}" y="{:.1}" class="ylab">{}</text>"#,
            W - PAD_R,
            PAD_L - 9.0,
            y + 4.0,
            if k == 0 {
                "0".to_string()
            } else {
                fmt_bytes((y_max * frac) as u64)
            }
        ));
    }

    let total_rx: u64 = v.data.iter().map(|d| d.2).sum();
    let total_tx: u64 = v.data.iter().map(|d| d.3).sum();

    // Seçiciler
    let range_btns: String = RANGES
        .iter()
        .map(|r| seg("range", *r == v.range, r.key(), &r.title()))
        .collect();
    let bucket_btns: String = BUCKETS
        .iter()
        .map(|b| seg("bucket", *b == v.bucket, b.key(), &b.title()))
        .collect();
    let metric_btns: String = METRICS
        .iter()
        .map(|m| seg("metric", *m == v.metric, m.key(), &m.title()))
        .collect();
    let iface_btns: String = std::iter::once(seg("iface", v.iface.is_none(), "*", &t("chart.all")))
        .chain(
            v.ifaces
                .iter()
                .map(|(id, label)| seg("iface", v.iface == Some(id.as_str()), id, label)),
        )
        .collect();

    // Arayüz dökümü tablosu
    let grand: u64 = v.by_iface.iter().map(|(_, rx, tx)| rx + tx).sum();
    let rows: String = v
        .by_iface
        .iter()
        .map(|(label, rx, tx)| {
            let tot = rx + tx;
            let pct = if grand > 0 {
                tot as f64 / grand as f64 * 100.0
            } else {
                0.0
            };
            format!(
                r#"<tr><td>{}</td><td class="num">{}</td><td class="num">{}</td><td class="num strong">{}</td><td class="pct"><span style="width:{pct:.1}%"></span></td></tr>"#,
                esc(label),
                fmt_bytes(*rx),
                fmt_bytes(*tx),
                fmt_bytes(tot),
            )
        })
        .collect();

    format!(
        r##"<!doctype html><html lang="{lang}"><head><meta charset="utf-8"><title>{title}</title>
<style>
:root {{
  --bg:#fff; --fg:#1c1f26; --muted:#6b7280; --grid:#e6e8ec; --line:#e3e5ea;
  --dl:#2f6df6; --ul:#12a45a; --tt:#8b5cf6; --card:#f6f7f9; --segbg:#eef0f3;
}}
@media (prefers-color-scheme: dark) {{
  :root {{
    --bg:#16181d; --fg:#e8eaed; --muted:#9aa1ac; --grid:#2a2e36; --line:#2a2e36;
    --dl:#5b8cff; --ul:#2bd07f; --tt:#a78bfa; --card:#1e2128; --segbg:#22262e;
  }}
}}
* {{ box-sizing:border-box; }}
body {{ margin:0; padding:16px 18px 22px; background:var(--bg); color:var(--fg);
  font:13px -apple-system,BlinkMacSystemFont,"Segoe UI",sans-serif; }}
.row {{ display:flex; flex-wrap:wrap; gap:4px; margin-bottom:7px; align-items:center; }}
.row .lbl {{ color:var(--muted); font-size:11px; width:64px; flex:none; }}
.seg {{ font:inherit; font-size:11.5px; padding:4px 9px; border:0; border-radius:6px;
  background:var(--segbg); color:var(--fg); cursor:pointer; }}
.seg:hover {{ filter:brightness(1.08); }}
.seg.on {{ background:var(--dl); color:#fff; }}
.cards {{ display:flex; gap:9px; margin:12px 0 10px; }}
.card {{ background:var(--card); border-radius:8px; padding:8px 13px; min-width:118px; }}
.card .k {{ color:var(--muted); font-size:11px; }}
.card .v {{ font-size:16px; font-weight:600; font-variant-numeric:tabular-nums; }}
.dot {{ display:inline-block; width:8px; height:8px; border-radius:2px; margin-right:5px; }}
svg {{ width:100%; height:auto; display:block; }}
.grid {{ stroke:var(--grid); stroke-width:1; }}
.dl {{ fill:var(--dl); }} .ul {{ fill:var(--ul); }} .tt {{ fill:var(--tt); }}
.bar:hover rect {{ opacity:.75; }}
.xlab,.ylab {{ fill:var(--muted); font-size:10px; font-variant-numeric:tabular-nums; }}
.xlab {{ text-anchor:middle; }} .ylab {{ text-anchor:end; }}
h2 {{ font-size:12px; font-weight:600; color:var(--muted); margin:16px 0 6px; }}
table {{ width:100%; border-collapse:collapse; font-variant-numeric:tabular-nums; }}
th,td {{ text-align:left; padding:5px 8px; border-bottom:1px solid var(--line); font-size:12px; }}
th {{ color:var(--muted); font-weight:500; font-size:11px; }}
td.num {{ text-align:right; }} td.strong {{ font-weight:600; }}
td.pct {{ width:110px; }}
td.pct span {{ display:block; height:6px; border-radius:3px; background:var(--dl); }}
#tip {{ position:fixed; pointer-events:none; opacity:0; transition:opacity .08s;
  background:var(--fg); color:var(--bg); padding:6px 9px; border-radius:6px;
  font-size:11.5px; line-height:1.5; white-space:nowrap; z-index:9; }}
#tip b {{ font-weight:600; }}
</style></head><body>
<div class="row"><span class="lbl">{l_range}</span>{range_btns}</div>
<div class="row"><span class="lbl">{l_bucket}</span>{bucket_btns}</div>
<div class="row"><span class="lbl">{l_metric}</span>{metric_btns}</div>
<div class="row"><span class="lbl">{l_iface}</span>{iface_btns}</div>

<div class="cards">
  <div class="card"><div class="k"><span class="dot" style="background:var(--dl)"></span>{l_dl}</div><div class="v">{dl_total}</div></div>
  <div class="card"><div class="k"><span class="dot" style="background:var(--ul)"></span>{l_ul}</div><div class="v">{ul_total}</div></div>
  <div class="card"><div class="k"><span class="dot" style="background:var(--tt)"></span>{l_tot}</div><div class="v">{gr_total}</div></div>
</div>

<svg viewBox="0 0 {W} {H}" xmlns="http://www.w3.org/2000/svg">{grid}{bars}{ticks}</svg>

<h2>{range_title}</h2>
<table><thead><tr><th>{l_col_iface}</th><th class="num">{l_dl}</th><th class="num">{l_ul}</th><th class="num">{l_tot}</th><th></th></tr></thead>
<tbody>{rows}</tbody></table>

<div id="tip"></div>
<script>
  document.querySelectorAll('.seg').forEach(function (b) {{
    b.addEventListener('click', function () {{
      window.ipc.postMessage(b.dataset.kind + ':' + b.dataset.key);
    }});
  }});

  var tip = document.getElementById('tip');
  document.querySelectorAll('.bar').forEach(function (g) {{
    g.addEventListener('mousemove', function (e) {{
      tip.innerHTML = '<b>' + g.dataset.t + '</b><br>' + {tooltip_js};
      tip.style.opacity = 1;
      var r = tip.getBoundingClientRect();
      var x = Math.min(e.clientX + 14, window.innerWidth - r.width - 8);
      var y = Math.max(e.clientY - r.height - 12, 8);
      tip.style.left = x + 'px';
      tip.style.top = y + 'px';
    }});
    g.addEventListener('mouseleave', function () {{ tip.style.opacity = 0; }});
  }});
</script></body></html>"##,
        lang = active(),
        title = esc(&t("window.chart")),
        tooltip_js = tooltip_js(),
        l_range = esc(&t("chart.range")),
        l_bucket = esc(&t("chart.bucket")),
        l_metric = esc(&t("chart.metric")),
        l_iface = esc(&t("chart.iface")),
        l_dl = esc(&t("chart.download")),
        l_ul = esc(&t("chart.upload")),
        l_tot = esc(&t("chart.total")),
        l_col_iface = esc(&t("chart.col_iface")),
        dl_total = fmt_bytes(total_rx),
        ul_total = fmt_bytes(total_tx),
        gr_total = fmt_bytes(total_rx + total_tx),
        range_title = esc(&t1("chart.by_iface", &v.range.title())),
    )
}
