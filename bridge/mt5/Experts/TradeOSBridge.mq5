//+------------------------------------------------------------------+
//|                                              TradeOSBridge.mq5   |
//| §5.4: the only way to get push-based tick delivery from MT5.    |
//| Publishes ticks over a ZeroMQ PUB socket per docs/protocol.md's  |
//| verified frame layout (WireTick in protocol.mqh). Cannot be      |
//| compiled or tested outside a real MetaTrader 5 terminal (Windows |
//| or Wine) — this environment has neither, so this file is         |
//| structure/pseudocode only. The tick-publishing half is a         |
//| concrete, small task once on real hardware: serialize WireTick   |
//| per the documented byte offsets and send over a bound PUB        |
//| socket. The order-accepting half (OnTradeTransaction / a REP     |
//| socket responder) is NOT sketched here because the wire format   |
//| it would speak doesn't exist yet — see protocol.mqh's "IMPORTANT"|
//| note and docs/protocol.md's "Important correction": the Rust     |
//| side's order encoding is rkyv-native and Rust-peer-only, so this  |
//| EA has nothing valid to decode against until a flat order frame   |
//| format is designed. Implementing OrderSend/Modify/Close here      |
//| before that exists would just be code with no correct wire        |
//| format to target — worse than leaving it unwritten.               |
//+------------------------------------------------------------------+
#property strict

#include <zmq.mqh>
#include <protocol.mqh>
#include <serializer.mqh>

ulong g_next_seq = 0;

int OnInit()
{
   // Phase 1 (unverified, needs a real terminal): bind a ZMQ PUB socket
   // at a configurable endpoint (e.g. via an input parameter), matching
   // whatever address crates/adapters/mt5's Mt5MarketData::new(..) is
   // pointed at. The REQ/REP order socket is deliberately not bound here
   // -- see the file header.
   return(INIT_SUCCEEDED);
}

void OnTick()
{
   // Phase 1 (unverified): build a WireTick from SymbolInfoTick(), prefix
   // with [g_next_seq][FRAME_KIND_TICK], send over the PUB socket, then
   // g_next_seq++. Must not block -- MT5 skips ticks under EA execution
   // stalls, and the Rust side's gap-detection (TickFlags::GAP) is
   // designed around occasional drops, not zero-drop delivery.
}

void OnDeinit(const int reason)
{
   // Phase 1: close the PUB socket cleanly.
}
