//+------------------------------------------------------------------+
//| ZeroMQ bindings for MQL5 (§5.4). MT5 has no native ZMQ support — |
//| production code wraps a DLL import (mql-zmq or similar) here.    |
//| Phase 1 scope.                                                    |
//+------------------------------------------------------------------+
#property strict

// Phase 1: #import "libzmq.dll" bindings for zmq_socket/zmq_bind/
// zmq_send/zmq_recv go here, matching whichever MQL5 ZMQ wrapper the
// implementation picks (e.g. mql-zmq).
