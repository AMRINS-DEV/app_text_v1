/**
 * Binary WebSocket client for /ws/stream (§11.2-11.3). Decode happens in
 * `workers/decode.worker.ts`, off the main thread, per §12.2's "decode Web
 * Worker consuming MessagePack frames into transferable typed arrays" (the
 * transfer itself is real — the `ArrayBuffer` moves to the worker with no
 * copy; the "typed arrays" part is future chart-engine work once a series
 * needs raw numeric buffers rather than the plain decoded objects this
 * client hands back today).
 *
 * The actual `WebSocket`/`Worker` wiring below is deliberately thin and has
 * no unit tests in this sandbox (no real browser to open a socket in); the
 * ref-counted subscribe/unsubscribe/dispatch logic it delegates to is
 * `TopicMultiplexer`, which is fully unit tested precisely because it
 * doesn't touch either of those.
 */
import { TopicMultiplexer, type Unsubscribe, type WsFrame } from "./topic-multiplexer";

export type WsTopic = `ticks:${string}` | `bars:${string}:${string}` | "signals" | "positions" | "pnl" | "agent_status";

export interface WsClient {
  subscribe<T = unknown>(topic: WsTopic, handler: (frame: WsFrame<T>) => void): Unsubscribe;
  close(): void;
}

export function createWsClient(url: string, accessToken: string): WsClient {
  const socket = new WebSocket(`${url}?token=${encodeURIComponent(accessToken)}`);
  socket.binaryType = "arraybuffer";
  const worker = new Worker(new URL("../workers/decode.worker.ts", import.meta.url));

  function sendControl(event: "subscribe" | "unsubscribe", topic: string): void {
    if (socket.readyState === WebSocket.OPEN) {
      socket.send(JSON.stringify({ event, data: { topic } }));
    }
  }

  const multiplexer = new TopicMultiplexer(
    (topic) => sendControl("subscribe", topic),
    (topic) => sendControl("unsubscribe", topic),
  );

  // A reconnect would need to re-issue subscribe for every currently active
  // topic; not implemented here (single-connection, no auto-reconnect) —
  // real reconnect-with-resubscribe is left as follow-up work, same as the
  // MT5 bridge's own reconnect-with-resync (Phase 1's README notes it as
  // Phase 2+ scope there for the same reason: it needs a live peer that can
  // actually disconnect to test against meaningfully).
  socket.addEventListener("open", () => {
    for (const topic of multiplexer.activeTopics()) sendControl("subscribe", topic);
  });

  socket.addEventListener("message", (event) => {
    if (event.data instanceof ArrayBuffer) worker.postMessage(event.data, [event.data]);
  });

  worker.onmessage = (event: MessageEvent<WsFrame>) => multiplexer.dispatch(event.data);

  return {
    subscribe: (topic, handler) => multiplexer.subscribe(topic, handler as (frame: WsFrame) => void),
    close: () => {
      socket.close();
      worker.terminate();
    },
  };
}
