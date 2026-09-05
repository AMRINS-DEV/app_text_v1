export type Unsubscribe = () => void;

export interface WsFrame<T = unknown> {
  topic: string;
  ts: number;
  payload: T;
}

/**
 * The pure ref-counting/dispatch logic behind `ws-client.ts`'s topic
 * multiplexing (§11.2's "one WS connection per client, multiplexed topics
 * with explicit subscribe/unsubscribe"), split out from the actual
 * `WebSocket`/`Worker` wiring so it's unit-testable without a browser —
 * the same reason `chart-engine`'s `DataProvider` and `SyncBus` don't
 * touch the DOM directly.
 *
 * `onFirstSubscriber`/`onLastUnsubscriber` fire only on the 0→1 and 1→0
 * transitions for a topic, which is exactly when a real client needs to
 * send a `subscribe`/`unsubscribe` control frame to the gateway — not on
 * every individual component's subscribe/unsubscribe call.
 */
export class TopicMultiplexer {
  private readonly handlers = new Map<string, Set<(frame: WsFrame) => void>>();

  constructor(
    private readonly onFirstSubscriber: (topic: string) => void,
    private readonly onLastUnsubscriber: (topic: string) => void,
  ) {}

  subscribe(topic: string, handler: (frame: WsFrame) => void): Unsubscribe {
    let set = this.handlers.get(topic);
    if (!set) {
      set = new Set();
      this.handlers.set(topic, set);
    }
    const isFirst = set.size === 0;
    set.add(handler);
    if (isFirst) this.onFirstSubscriber(topic);

    return () => {
      const current = this.handlers.get(topic);
      if (!current?.delete(handler)) return;
      if (current.size === 0) {
        this.handlers.delete(topic);
        this.onLastUnsubscriber(topic);
      }
    };
  }

  dispatch(frame: WsFrame): void {
    for (const handler of this.handlers.get(frame.topic) ?? []) handler(frame);
  }

  activeTopics(): string[] {
    return [...this.handlers.keys()];
  }
}
