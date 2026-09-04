use criterion::{black_box, criterion_group, criterion_main, Criterion};
use indicators::{Atr, Ema, Incremental, OhlcInput, Rsi};

fn bench_ema(c: &mut Criterion) {
    c.bench_function("ema_update", |b| {
        let mut ema = Ema::new(14);
        b.iter(|| ema.update(black_box(100.0)));
    });
}

fn bench_atr(c: &mut Criterion) {
    c.bench_function("atr_update", |b| {
        let mut atr = Atr::new(14);
        b.iter(|| atr.update(black_box(OhlcInput { high: 101.0, low: 99.0, close: 100.0 })));
    });
}

fn bench_rsi(c: &mut Criterion) {
    c.bench_function("rsi_update", |b| {
        let mut rsi = Rsi::new(14);
        let mut price = 100.0;
        b.iter(|| {
            price += 0.1;
            rsi.update(black_box(price))
        });
    });
}

criterion_group!(benches, bench_ema, bench_atr, bench_rsi);
criterion_main!(benches);
