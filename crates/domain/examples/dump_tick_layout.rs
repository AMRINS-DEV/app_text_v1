//! Dumps the actual rkyv-archived byte layout of `Tick`, field values chosen
//! so every byte is distinguishable in a hex dump. This is how the byte
//! offsets documented in `docs/protocol.md` (and mirrored in
//! `bridge/mt5/Include/protocol.mqh`) were derived — not assumed from
//! reading rkyv's source, but observed directly. Re-run this whenever
//! `Tick`'s fields change, since padding/offsets are layout-sensitive.
//!
//! `cargo run -p domain --example dump_tick_layout`

use domain::Tick;

fn main() {
    let t = Tick {
        ts_ns: 0x0102030405060708,
        recv_ns: 0x1112131415161718,
        symbol_id: 0x2122,
        bid: 0x3132333435363738,
        ask: 0x4142434445464748,
        bid_volume: 0x51525354,
        ask_volume: 0x61626364,
        flags: 0x7172,
    };
    let bytes = rkyv::to_bytes::<rkyv::rancor::Error>(&t).unwrap();
    println!("ArchivedTick total size: {} bytes\n", bytes.len());
    for (i, b) in bytes.iter().enumerate() {
        print!("{b:02x} ");
        if (i + 1) % 8 == 0 {
            println!("  <- bytes {}..{}", i - 7, i + 1);
        }
    }
    println!(
        "\nField layout (little-endian, matching #[repr(C)] with natural alignment):\n\
         ts_ns       @  0..8\n\
         recv_ns     @  8..16\n\
         symbol_id   @ 16..18\n\
         (padding)   @ 18..24  (bid needs 8-byte alignment)\n\
         bid         @ 24..32\n\
         ask         @ 32..40\n\
         bid_volume  @ 40..44\n\
         ask_volume  @ 44..48\n\
         flags       @ 48..50\n\
         (padding)   @ 50..56  (struct size rounds up to its 8-byte alignment)"
    );
}
