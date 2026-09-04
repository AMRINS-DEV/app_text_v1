//! Quick manual latency check against the §5.2 budget table. CI's actual
//! regression gate runs `crates/bench` (criterion) instead of this binary.

use indicators::{Ema, Incremental};
use std::time::Instant;

fn main() {
    const N: usize = 1_000_000;
    let mut ema = Ema::new(14);
    let mut samples = Vec::with_capacity(N);

    for i in 0..N {
        let start = Instant::now();
        std::hint::black_box(ema.update(std::hint::black_box(100.0 + (i % 7) as f64)));
        samples.push(start.elapsed().as_nanos() as u64);
    }

    samples.sort_unstable();
    let p50 = samples[N / 2];
    let p99 = samples[N * 99 / 100];
    let p999 = samples[N * 999 / 1000];
    println!("Ema::update over {N} calls (ns): p50={p50} p99={p99} p999={p999}");
    println!("§5.2 budget for 'Feature update (incremental)': 3-10 microseconds per stage");
}
