#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod chart;
mod config;
mod i18n;
#[cfg(not(target_os = "macos"))]
mod icon;
mod ifaces;
mod options;
mod stats;
mod units;

use std::net::TcpListener;
use std::time::{Duration, Instant};

use auto_launch::AutoLaunchBuilder;
use config::Config;
use i18n::t;
use stats::{Bucket, Metric, Range, Stats};
use sysinfo::Networks;
use units::{Mode, Unit};
use tao::event::{Event, StartCause, WindowEvent};
use tao::event_loop::{ControlFlow, EventLoopBuilder};
use tao::window::{Window, WindowBuilder};
use wry::{WebView, WebViewBuilder};
#[cfg(target_os = "macos")]
use tao::platform::macos::{ActivationPolicy, EventLoopExtMacOS, EventLoopWindowTargetExtMacOS};
use tray_icon::menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem, Submenu};
use tray_icon::{TrayIcon, TrayIconBuilder};

const TICK: Duration = Duration::from_secs(1);
/// Tek çalışma kilidi: port açık kalırsa ikinci kopya başlayamaz.
const LOCK_ADDR: &str = "127.0.0.1:47823";

enum UserEvent {
    Menu(MenuEvent),
    /// Pencerelerden gelen seçim: "range:30d", "opt.lang:tr" gibi
    Ipc(String),
}

/// Grafik penceresinin görünüm durumu.
struct ChartState {
    range: Range,
    bucket: Bucket,
    metric: Metric,
    iface: Option<String>,
}

impl Default for ChartState {
    fn default() -> Self {
        Self {
            range: Range::Last24h,
            bucket: Range::Last24h.default_bucket(),
            metric: Metric::Both,
            iface: None,
        }
    }
}

/// Alan sırası önemli: WebView, Window'dan önce drop edilmeli.
struct WinView {
    webview: WebView,
    window: Window,
}

/// Seçili görünüm için grafik HTML'ini üretir.
fn render_chart(db: &Stats, st: &ChartState, ifaces: &[(String, String)]) -> String {
    let data = db.series(st.range, st.bucket, st.iface.as_deref());

    // Arayüz dökümünde ham adı okunabilir etikete çevir.
    let by_iface: Vec<(String, u64, u64)> = db
        .totals_by_iface(st.range)
        .into_iter()
        .map(|(id, rx, tx)| {
            let label = ifaces
                .iter()
                .find(|(i, _)| *i == id)
                .map(|(_, l)| l.clone())
                .unwrap_or(id);
            (label, rx, tx)
        })
        .collect();

    chart::html(&chart::View {
        range: st.range,
        bucket: st.bucket,
        metric: st.metric,
        iface: st.iface.as_deref(),
        ifaces,
        data: &data,
        by_iface: &by_iface,
    })
}

fn autolaunch() -> Option<auto_launch::AutoLaunch> {
    let exe = std::env::current_exe().ok()?;
    AutoLaunchBuilder::new()
        .set_app_name("Net Manager")
        .set_app_path(&exe.to_string_lossy())
        .build()
        .ok()
}

fn main() {
    // Zaten çalışan bir kopya varsa sessizce çık.
    let Ok(_lock) = TcpListener::bind(LOCK_ADDR) else {
        eprintln!("Net Manager is already running.");
        return;
    };

    let mut cfg = Config::load();
    i18n::set_lang(cfg.lang.as_deref());

    let mut networks = Networks::new_with_refreshed_list();
    let iface_list = ifaces::list(&networks);

    // Kayıtlı arayüz artık yoksa "Tümü"ye dön.
    if let Some(sel) = &cfg.iface {
        if !iface_list.iter().any(|i| &i.id == sel) {
            cfg.iface = None;
        }
    }

    let al = autolaunch();
    if let Some(al) = &al {
        let enabled = al.is_enabled().unwrap_or(false);
        if cfg.autostart && !enabled {
            let _ = al.enable();
        } else if !cfg.autostart && enabled {
            let _ = al.disable();
        }
    }
    let db = Stats::open().ok();

    // --- Menü: yalnızca İstatistikler ve Seçenekler (artı Çıkış) ---
    let menu = Menu::new();

    let stats_menu = Submenu::new(t("menu.stats"), true);
    let today_item = MenuItem::new(format!("{}: {}", t("stats.today"), t("stats.empty")), false, None);
    let month_item = MenuItem::new(format!("{}: {}", t("stats.month"), t("stats.empty")), false, None);
    let peak_item = MenuItem::new(
        format!("{}: {}", t("stats.peak_today"), t("stats.empty")),
        false,
        None,
    );
    let chart_item = MenuItem::new(t("stats.chart"), true, None);
    stats_menu.append(&today_item).unwrap();
    stats_menu.append(&month_item).unwrap();
    stats_menu.append(&PredefinedMenuItem::separator()).unwrap();
    stats_menu.append(&peak_item).unwrap();
    stats_menu.append(&PredefinedMenuItem::separator()).unwrap();
    stats_menu.append(&chart_item).unwrap();

    let options_item = MenuItem::new(t("menu.options"), true, None);
    let quit_item = MenuItem::new(t("menu.quit"), true, None);

    menu.append(&stats_menu).unwrap();
    menu.append(&options_item).unwrap();
    menu.append(&PredefinedMenuItem::separator()).unwrap();
    menu.append(&quit_item).unwrap();

    // --- Event loop ---
    // Pencerelerdeki arayüz seçim listesi (id, etiket)
    let mut iface_choices: Vec<(String, String)> = iface_list
        .iter()
        .map(|f| (f.id.clone(), f.label.clone()))
        .collect();

    #[allow(unused_mut)]
    let mut event_loop = EventLoopBuilder::<UserEvent>::with_user_event().build();
    // Dock'ta ve Cmd+Tab'da görünme: yalnızca menü çubuğu uygulaması.
    #[cfg(target_os = "macos")]
    {
        event_loop.set_activation_policy(ActivationPolicy::Accessory);
        event_loop.set_activate_ignoring_other_apps(false);
    }
    let proxy = event_loop.create_proxy();
    MenuEvent::set_event_handler(Some(move |e| {
        let _ = proxy.send_event(UserEvent::Menu(e));
    }));
    let ipc_proxy = event_loop.create_proxy();

    let mut tray: Option<TrayIcon> = None;
    let mut next_tick = Instant::now() + TICK;

    let mut chart: Option<WinView> = None;
    let mut opts: Option<WinView> = None;
    let mut ticks_since_chart: u32 = 0;
    let mut chart_state = ChartState::default();
    let mut rescan_ticks: u32 = 0;

    event_loop.run(move |event, target, control_flow| {
        *control_flow = ControlFlow::WaitUntil(next_tick);

        // macOS: pencere varken Dock'ta görün, hiç yokken menü çubuğuna çekil.
        #[cfg(target_os = "macos")]
        let sync_policy = |chart: &Option<WinView>, opts: &Option<WinView>| {
            let policy = if chart.is_some() || opts.is_some() {
                ActivationPolicy::Regular
            } else {
                ActivationPolicy::Accessory
            };
            target.set_activation_policy_at_runtime(policy);
        };

        match event {
            Event::NewEvents(StartCause::Init) => {
                let builder = TrayIconBuilder::new()
                    .with_menu(Box::new(menu.clone()))
                    .with_title("↓ 0 KB/s  ↑ 0 KB/s")
                    .with_tooltip(t("app.name"));
                // macOS menü çubuğunda yalnızca metin: ikon yer kaplamasın.
                #[cfg(not(target_os = "macos"))]
                let builder = builder.with_icon(icon::build());

                tray = Some(builder.build().expect("tray icon could not be created"));
            }

            Event::NewEvents(StartCause::ResumeTimeReached { .. }) => {
                next_tick = Instant::now() + TICK;
                *control_flow = ControlFlow::WaitUntil(next_tick);

                // refresh() sonradan eklenen arayüzü görmez; refresh_list() görür ve
                // mevcut arayüzlerin deltasını bozmaz (yeni kayıt old == current ile başlar).
                networks.refresh_list();

                let mut samples: Vec<(String, u64, u64)> = Vec::new();
                let (mut down, mut up) = (0u64, 0u64);
                for (name, data) in &networks {
                    if !ifaces::is_recordable(name) {
                        continue;
                    }
                    let (rx, tx) = (data.received(), data.transmitted());
                    samples.push((name.to_string(), rx, tx));

                    let selected = match &cfg.iface {
                        None => ifaces::counts_toward_total(name),
                        Some(sel) => sel == name,
                    };
                    if selected {
                        down += rx;
                        up += tx;
                    }
                }

                if let Some(db) = &db {
                    let _ = db.record(&samples);
                }

                // Sonradan aktifleşen arayüzler (USB adaptör, VPN, vb.) seçim listesine girsin.
                rescan_ticks += 1;
                if rescan_ticks >= 10 {
                    rescan_ticks = 0;
                    for f in ifaces::list(&networks) {
                        if iface_choices.iter().any(|(id, _)| *id == f.id) {
                            continue;
                        }
                        iface_choices.push((f.id.clone(), f.label));
                    }
                }

                let title = format!("↓ {}  ↑ {}", units::fmt_rate(down as f64, cfg.unit_mode, cfg.min_unit, cfg.max_unit),
                    units::fmt_rate(up as f64, cfg.unit_mode, cfg.min_unit, cfg.max_unit));
                if let Some(t) = &tray {
                    t.set_title(Some(&title));
                    let _ = t.set_tooltip(Some(&title));
                }

                // Grafik penceresi açıksa dakikada bir tazele
                if let (Some(c), Some(db)) = (&chart, &db) {
                    ticks_since_chart += 1;
                    if ticks_since_chart >= 60 {
                        ticks_since_chart = 0;
                        let _ =
                            c.webview
                                .load_html(&render_chart(db, &chart_state, &iface_choices));
                    }
                }

                if let Some(db) = &db {
                    let sel = cfg.iface.as_deref();
                    let (d, m, p) = (db.today(sel), db.month(sel), db.peak_today(sel));
                    today_item.set_text(format!(
                        "{}:  ↓ {}   ↑ {}",
                        t("stats.today"),
                        stats::fmt_bytes(d.0),
                        stats::fmt_bytes(d.1)
                    ));
                    month_item.set_text(format!(
                        "{}:  ↓ {}   ↑ {}",
                        t("stats.month"),
                        stats::fmt_bytes(m.0),
                        stats::fmt_bytes(m.1)
                    ));
                    peak_item.set_text(format!(
                        "{}:  ↓ {}   ↑ {}",
                        t("stats.peak_today"),
                        units::fmt_rate(p.0 as f64, cfg.unit_mode, cfg.min_unit, cfg.max_unit),
                        units::fmt_rate(p.1 as f64, cfg.unit_mode, cfg.min_unit, cfg.max_unit)
                    ));
                }
            }

            Event::UserEvent(UserEvent::Menu(ev)) => {
                if ev.id == *chart_item.id() {
                    if let Some(c) = &chart {
                        c.window.set_focus();
                    } else if let Some(db) = &db {
                        chart_state = ChartState {
                            iface: cfg.iface.clone(),
                            ..ChartState::default()
                        };
                        if let Ok(w) = WindowBuilder::new()
                            .with_title(t("window.chart"))
                            .with_inner_size(tao::dpi::LogicalSize::new(980.0, 780.0))
                            .build(target)
                        {
                            let p = ipc_proxy.clone();
                            match WebViewBuilder::new()
                                .with_html(render_chart(db, &chart_state, &iface_choices))
                                .with_ipc_handler(move |req| {
                                    let _ = p.send_event(UserEvent::Ipc(req.body().clone()));
                                })
                                .build(&w)
                            {
                                Ok(webview) => {
                                    w.set_focus();
                                    chart = Some(WinView { webview, window: w });
                                    #[cfg(target_os = "macos")]
                                    sync_policy(&chart, &opts);
                                }
                                Err(e) => eprintln!("webview error: {e}"),
                            }
                        }
                    }
                    return;
                }

                if ev.id == *options_item.id() {
                    if let Some(o) = &opts {
                        o.window.set_focus();
                    } else if let Ok(w) = WindowBuilder::new()
                        .with_title(t("window.options"))
                        .with_inner_size(tao::dpi::LogicalSize::new(520.0, 340.0))
                        .build(target)
                    {
                        let p = ipc_proxy.clone();
                        match WebViewBuilder::new()
                            .with_html(options::html(&cfg, &iface_choices))
                            .with_ipc_handler(move |req| {
                                let _ = p.send_event(UserEvent::Ipc(req.body().clone()));
                            })
                            .build(&w)
                        {
                            Ok(webview) => {
                                w.set_focus();
                                opts = Some(WinView { webview, window: w });
                                #[cfg(target_os = "macos")]
                                sync_policy(&chart, &opts);
                            }
                            Err(e) => eprintln!("webview error: {e}"),
                        }
                    }
                    return;
                }

                if ev.id == *quit_item.id() {
                    *control_flow = ControlFlow::Exit;
                }
            }

            Event::UserEvent(UserEvent::Ipc(msg)) => {
                let Some((kind, key)) = msg.split_once(':') else {
                    return;
                };

                // --- Seçenekler penceresi ---
                if let Some(field) = kind.strip_prefix("opt.") {
                    let mut relabel = false;
                    match field {
                        "lang" => {
                            cfg.lang = if key == "system" {
                                None
                            } else {
                                Some(key.to_string())
                            };
                            i18n::set_lang(cfg.lang.as_deref());
                            relabel = true;
                        }
                        "iface" => {
                            cfg.iface = if key == "*" { None } else { Some(key.to_string()) };
                        }
                        "mode" => {
                            if let Some(m) = Mode::from_key(key) {
                                cfg.unit_mode = m;
                            }
                        }
                        "min" => {
                            if let Some(u) = Unit::from_key(key) {
                                cfg.min_unit = u;
                                if cfg.max_unit < u {
                                    cfg.max_unit = u;
                                }
                            }
                        }
                        "max" => {
                            if let Some(u) = Unit::from_key(key) {
                                cfg.max_unit = u;
                                if cfg.min_unit > u {
                                    cfg.min_unit = u;
                                }
                            }
                        }
                        "autostart" => {
                            cfg.autostart = key == "1";
                            if let Some(al) = &al {
                                let ok = if cfg.autostart { al.enable() } else { al.disable() };
                                if ok.is_err() {
                                    cfg.autostart = !cfg.autostart;
                                }
                            }
                        }
                        _ => return,
                    }
                    cfg.save();

                    // Dil değiştiyse menü etiketlerini yenile.
                    if relabel {
                        stats_menu.set_text(t("menu.stats"));
                        chart_item.set_text(t("stats.chart"));
                        options_item.set_text(t("menu.options"));
                        quit_item.set_text(t("menu.quit"));
                        if let Some(c) = &chart {
                            c.window.set_title(&t("window.chart"));
                        }
                        if let Some(o) = &opts {
                            o.window.set_title(&t("window.options"));
                        }
                    }

                    // Seçim listeleri ve etiketler ayarla birlikte değişir: yeniden çiz.
                    if let Some(o) = &opts {
                        let _ = o.webview.load_html(&options::html(&cfg, &iface_choices));
                    }
                    if let (Some(c), Some(db)) = (&chart, &db) {
                        let _ = c.webview.load_html(&render_chart(db, &chart_state, &iface_choices));
                        ticks_since_chart = 0;
                    }
                    return;
                }

                // --- Grafik penceresi ---
                match kind {
                    "range" => {
                        if let Some(r) = Range::from_key(key) {
                            chart_state.range = r;
                            chart_state.bucket = r.default_bucket();
                        }
                    }
                    "bucket" => {
                        if let Some(b) = Bucket::from_key(key) {
                            chart_state.bucket = b;
                        }
                    }
                    "metric" => {
                        if let Some(m) = Metric::from_key(key) {
                            chart_state.metric = m;
                        }
                    }
                    "iface" => {
                        chart_state.iface = if key == "*" { None } else { Some(key.to_string()) };
                    }
                    _ => return,
                }
                if let (Some(c), Some(db)) = (&chart, &db) {
                    let _ = c.webview.load_html(&render_chart(db, &chart_state, &iface_choices));
                }
                ticks_since_chart = 0;
            }

            Event::WindowEvent {
                window_id,
                event: WindowEvent::CloseRequested,
                ..
            } => {
                if chart.as_ref().is_some_and(|c| c.window.id() == window_id) {
                    chart = None;
                } else if opts.as_ref().is_some_and(|o| o.window.id() == window_id) {
                    opts = None;
                }
                #[cfg(target_os = "macos")]
                sync_policy(&chart, &opts);
            }

            _ => {}
        }
    });
}
