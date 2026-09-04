//! Dumps the rkyv-archived byte layout of OrderRequest::Submit and
//! OrderReply, the same way domain's dump_tick_layout does for Tick.
//! `cargo run -p adapter-mt5 --example dump_order_layout`

use adapter_mt5::protocol::{OrderReply, OrderRequest};
use domain::{OrderIntent, OrderType, Side, TimeInForce, TradingMode};
use smallvec::SmallVec;

fn dump(label: &str, bytes: &[u8]) {
    println!("--- {label} ({} bytes) ---", bytes.len());
    for (i, b) in bytes.iter().enumerate() {
        print!("{b:02x} ");
        if (i + 1) % 8 == 0 {
            println!();
        }
    }
    println!("\n");
}

fn main() {
    let submit = OrderRequest::Submit(OrderIntent {
        client_id: 0x0102030405060708090a0b0c0d0e0f10,
        symbol_id: 0x1122,
        side: Side::Buy,
        qty: 0x2122232425262728,
        order_type: OrderType::Market,
        limit_px: None,
        sl: Some(0x3132333435363738),
        tp: Some(0x4142434445464748),
        tif: TimeInForce::Gtc,
        mode: TradingMode::Normal,
        max_slippage_pts: 0x51525354,
        signal_ids: SmallVec::from_slice(&[0x6162636465666768]),
    });
    let bytes = rkyv::to_bytes::<rkyv::rancor::Error>(&submit).unwrap();
    dump("OrderRequest::Submit(...)", &bytes);

    let accepted = OrderReply::Accepted { broker_order_id: 0x0102030405060708 };
    let bytes = rkyv::to_bytes::<rkyv::rancor::Error>(&accepted).unwrap();
    dump("OrderReply::Accepted", &bytes);

    let rejected = OrderReply::Rejected { reason: "no".into() };
    let bytes = rkyv::to_bytes::<rkyv::rancor::Error>(&rejected).unwrap();
    dump("OrderReply::Rejected(\"no\")", &bytes);
}
