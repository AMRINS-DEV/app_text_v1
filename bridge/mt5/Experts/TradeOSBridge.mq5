//+------------------------------------------------------------------+
//|                                              TradeOSBridge.mq5   |
//| §5.4: the only way to get push-based tick delivery from MT5.    |
//| Publishes ticks + DOM over a ZeroMQ PUB socket and accepts       |
//| OrderSend/Modify/Close over a REQ/REP socket, per                |
//| docs/protocol.md. Cannot be compiled or tested outside a real    |
//| MetaTrader 5 terminal (Windows or Wine) — this environment has   |
//| neither, so this file is structure only for Phase 0. Phase 1     |
//| scope: implement OnTick()/OnBookEvent() publishing and the       |
//| REP-socket order handler against docs/protocol.md's wire format. |
//+------------------------------------------------------------------+
#property strict

#include <zmq.mqh>
#include <protocol.mqh>
#include <serializer.mqh>

int OnInit()
{
   // Phase 1: bind PUB socket for ticks/DOM, bind REP socket for orders.
   return(INIT_SUCCEEDED);
}

void OnTick()
{
   // Phase 1: serialize the current tick per protocol.mqh's fixed layout
   // and publish over ZMQ PUB. Must not block — MT5 will skip ticks under
   // EA execution stalls.
}

void OnBookEvent(const string &symbol)
{
   // Phase 1: publish DOM/depth updates where the symbol supports it.
}

void OnDeinit(const int reason)
{
   // Phase 1: close sockets cleanly.
}
