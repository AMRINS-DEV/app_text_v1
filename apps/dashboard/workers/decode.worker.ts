/// <reference lib="webworker" />

/**
 * WS binary frames -> typed arrays via transferable ArrayBuffer (§12.2
 * performance technique #1: zero main-thread JSON parsing). Real MessagePack
 * decode wiring is Phase 4 scope, once packages/chart-engine exists.
 */
declare const self: DedicatedWorkerGlobalScope;

self.onmessage = () => {
  // Phase 4: decode incoming ArrayBuffer frames and post back typed arrays.
};

export {};
