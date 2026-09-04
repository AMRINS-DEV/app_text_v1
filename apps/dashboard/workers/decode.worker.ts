/// <reference lib="webworker" />

/**
 * WS binary frames -> decoded objects, off the main thread (§12.2
 * performance technique #1: zero main-thread MessagePack decoding). The
 * incoming `ArrayBuffer` arrives transferred (no copy); the decoded object
 * is structurally cloned back — the "transferable typed arrays" half of
 * §12.2's description is future chart-engine work once a series consumer
 * wants raw numeric buffers instead of plain objects.
 */
import { decode } from "@msgpack/msgpack";

import type { WsFrame } from "../lib/topic-multiplexer";

declare const self: DedicatedWorkerGlobalScope;

self.onmessage = (event: MessageEvent<ArrayBuffer>) => {
  const frame = decode(new Uint8Array(event.data)) as WsFrame;
  self.postMessage(frame);
};

export {};
