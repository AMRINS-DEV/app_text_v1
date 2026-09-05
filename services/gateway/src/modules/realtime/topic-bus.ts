import { Injectable } from "@nestjs/common";

export type Unsubscribe = () => void;

/**
 * In-process pub/sub used as the fan-out point between data producers
 * (`MarketFeedService`, `PositionsStore`) and `RealtimeGateway`'s per-client
 * WS delivery. §11.1 describes this as "Redis pub/sub → per-topic
 * fan-out"; this sandbox runs a single gateway instance with no Redis, so
 * an in-process bus is the honest single-instance equivalent — the
 * publish/subscribe contract is exactly what a Redis-backed version would
 * expose, so swapping the implementation later doesn't change any caller.
 */
@Injectable()
export class TopicBus {
  private readonly subscribers = new Map<string, Set<(payload: unknown) => void>>();

  publish(topic: string, payload: unknown): void {
    for (const handler of this.subscribers.get(topic) ?? []) {
      handler(payload);
    }
  }

  subscribe(topic: string, handler: (payload: unknown) => void): Unsubscribe {
    let set = this.subscribers.get(topic);
    if (!set) {
      set = new Set();
      this.subscribers.set(topic, set);
    }
    set.add(handler);
    return () => set.delete(handler);
  }
}
