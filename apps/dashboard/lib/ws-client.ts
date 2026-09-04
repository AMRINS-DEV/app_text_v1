/**
 * Binary WebSocket client for /ws/stream (§11.2-11.3). MessagePack decode
 * happens in `workers/decode.worker.ts`, off the main thread. Real
 * connection/reconnect/backoff logic is Phase 4 scope.
 */
export type WsTopic = `ticks:${string}` | `bars:${string}:${string}` | "signals" | "positions" | "pnl" | "agent_status";

export interface WsClient {
  subscribe(topic: WsTopic): void;
  unsubscribe(topic: WsTopic): void;
}

export function createWsClient(_url: string): WsClient {
  throw new Error("ws-client is Phase 4 scope");
}
