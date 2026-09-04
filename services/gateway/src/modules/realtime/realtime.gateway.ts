import { Logger } from "@nestjs/common";
import { ConnectedSocket, MessageBody, OnGatewayConnection, OnGatewayDisconnect, SubscribeMessage, WebSocketGateway } from "@nestjs/websockets";
import { encode } from "@msgpack/msgpack";
import type { IncomingMessage } from "node:http";
import type { WebSocket } from "ws";

import { Role, ALL_ROLES } from "../../common/roles";
import { TokenService } from "../auth/token.service";
import { TopicBus, type Unsubscribe } from "./topic-bus";

/** Topics whose subscribers only ever want the latest value, never a
 * backlog — the natural fit for §11.3's "conflated snapshot mode." */
const CONFLATED_TOPIC_PREFIXES = new Set(["positions", "pnl", "agent_status"]);

/** Per-topic-prefix RBAC (§11.3: "per-topic RBAC check"). Every topic is
 * open to all four roles except `agent_status`, kept as one concrete
 * example of the mechanism actually differentiating — see the module doc
 * comment in `realtime.module.ts` for why the rest aren't further split. */
const TOPIC_ROLES: Record<string, Role[]> = {
  ticks: ALL_ROLES,
  bars: ALL_ROLES,
  signals: ALL_ROLES,
  positions: ALL_ROLES,
  pnl: ALL_ROLES,
  agent_status: [Role.Owner, Role.Trader, Role.Analyst],
};

const FLUSH_INTERVAL_MS = 75;
const BACKPRESSURE_THRESHOLD_BYTES = 64 * 1024;
const VALID_TOPIC = /^(ticks|bars):[A-Za-z0-9]+(:[A-Za-z0-9]+)?$|^(signals|positions|pnl|agent_status)$/;

function topicPrefix(topic: string): string {
  return topic.split(":", 1)[0];
}

interface ConnectionState {
  role: Role;
  subscriptions: Map<string, { buffer: unknown[]; unsubscribe: Unsubscribe }>;
  flushTimer: NodeJS.Timeout;
}

/**
 * §11.2-11.3: WS /ws/stream, topic multiplexing with explicit
 * subscribe/unsubscribe, MessagePack binary frames, 50-100ms server-side
 * coalescing, per-topic RBAC, and backpressure-to-conflation. Control
 * messages (subscribe/unsubscribe) are the adapter's plain JSON envelope;
 * data frames bypass that entirely and are written straight to the raw
 * `ws` socket as MessagePack so the client never has to JSON.parse a hot
 * path payload.
 */
@WebSocketGateway({ path: "/ws/stream" })
export class RealtimeGateway implements OnGatewayConnection, OnGatewayDisconnect {
  private readonly logger = new Logger(RealtimeGateway.name);
  private readonly connections = new WeakMap<WebSocket, ConnectionState>();

  constructor(
    private readonly bus: TopicBus,
    private readonly tokens: TokenService,
  ) {}

  handleConnection(client: WebSocket, request: IncomingMessage): void {
    const token = new URL(request.url ?? "", "ws://placeholder").searchParams.get("token");
    if (!token) {
      client.close(4001, "missing token");
      return;
    }
    let role: Role;
    try {
      role = this.tokens.verify(token, "access").role;
    } catch {
      client.close(4001, "invalid or expired token");
      return;
    }

    const flushTimer = setInterval(() => this.flush(client), FLUSH_INTERVAL_MS);
    this.connections.set(client, { role, subscriptions: new Map(), flushTimer });
  }

  handleDisconnect(client: WebSocket): void {
    const state = this.connections.get(client);
    if (!state) return;
    clearInterval(state.flushTimer);
    for (const { unsubscribe } of state.subscriptions.values()) unsubscribe();
    this.connections.delete(client);
  }

  @SubscribeMessage("subscribe")
  handleSubscribe(@MessageBody() data: { topic?: string }, @ConnectedSocket() client: WebSocket): void {
    const state = this.connections.get(client);
    const topic = data?.topic;
    if (!state || !topic || !VALID_TOPIC.test(topic)) return;
    if (state.subscriptions.has(topic)) return;

    const allowedRoles = TOPIC_ROLES[topicPrefix(topic)];
    if (!allowedRoles?.includes(state.role)) {
      this.logger.warn(`role '${state.role}' denied subscription to '${topic}'`);
      return;
    }

    const entry = { buffer: [] as unknown[], unsubscribe: () => {} };
    entry.unsubscribe = this.bus.subscribe(topic, (payload) => entry.buffer.push(payload));
    state.subscriptions.set(topic, entry);
  }

  @SubscribeMessage("unsubscribe")
  handleUnsubscribe(@MessageBody() data: { topic?: string }, @ConnectedSocket() client: WebSocket): void {
    const state = this.connections.get(client);
    const topic = data?.topic;
    if (!state || !topic) return;
    const entry = state.subscriptions.get(topic);
    if (!entry) return;
    entry.unsubscribe();
    state.subscriptions.delete(topic);
  }

  private flush(client: WebSocket): void {
    const state = this.connections.get(client);
    if (!state || client.readyState !== client.OPEN) return;

    const underBackpressure = client.bufferedAmount > BACKPRESSURE_THRESHOLD_BYTES;
    for (const [topic, entry] of state.subscriptions) {
      if (entry.buffer.length === 0) continue;
      const conflate = underBackpressure || CONFLATED_TOPIC_PREFIXES.has(topicPrefix(topic));
      const payload = conflate ? entry.buffer[entry.buffer.length - 1] : entry.buffer.slice();
      entry.buffer.length = 0;
      client.send(encode({ topic, ts: Date.now(), payload }));
    }
  }
}
