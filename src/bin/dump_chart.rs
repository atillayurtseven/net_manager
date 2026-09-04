//! Geliştirme aracı: grafik HTML'ini dosyaya yazar (tarayıcıda önizleme için).
#[path = "../i18n.rs"]
mod i18n;
#[path = "../chart.rs"]
mod chart;
#[path = "../stats.rs"]
mod stats;

fn main() {
    // İsteğe bağlı 2. argüman: dil kodu ("tr"), yoksa varsayılan dil.
    i18n::set_lang(&std::env::args().nth(2).unwrap_or_else(|| i18n::DEFAULT.to_string()));
    let db = stats::Stats::open().expect("db açılamadı");
    let (range, bucket) = (stats::Range::Last24h, stats::Bucket::Hour);
    let data = db.series(range, bucket, None);
    let by_iface = db.totals_by_iface(range);

    let out = std::env::args().nth(1).unwrap_or_else(|| "chart.html".into());
    std::fs::write(
        &out,
        chart::html(&chart::View {
            range,
            bucket,
            metric: stats::Metric::Both,
            iface: None,
            ifaces: &[],
            data: &data,
            by_iface: &by_iface,
        }),
    )
    .unwrap();
    println!("yazıldı: {out}");
}
