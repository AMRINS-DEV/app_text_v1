import { describe, expect, it, vi } from "vitest";

import { TopicMultiplexer } from "./topic-multiplexer";

describe("TopicMultiplexer", () => {
  it("fires onFirstSubscriber only for the first handler on a topic", () => {
    const onFirst = vi.fn();
    const onLast = vi.fn();
    const mux = new TopicMultiplexer(onFirst, onLast);

    mux.subscribe("ticks:EURUSD", vi.fn());
    mux.subscribe("ticks:EURUSD", vi.fn());

    expect(onFirst).toHaveBeenCalledTimes(1);
    expect(onFirst).toHaveBeenCalledWith("ticks:EURUSD");
  });

  it("fires onLastUnsubscriber only once every handler has unsubscribed", () => {
    const onFirst = vi.fn();
    const onLast = vi.fn();
    const mux = new TopicMultiplexer(onFirst, onLast);

    const unsubA = mux.subscribe("positions", vi.fn());
    const unsubB = mux.subscribe("positions", vi.fn());

    unsubA();
    expect(onLast).not.toHaveBeenCalled();
    unsubB();
    expect(onLast).toHaveBeenCalledTimes(1);
    expect(onLast).toHaveBeenCalledWith("positions");
  });

  it("dispatches a frame only to handlers subscribed to its topic", () => {
    const mux = new TopicMultiplexer(vi.fn(), vi.fn());
    const tickHandler = vi.fn();
    const pnlHandler = vi.fn();
    mux.subscribe("ticks:EURUSD", tickHandler);
    mux.subscribe("pnl", pnlHandler);

    mux.dispatch({ topic: "ticks:EURUSD", ts: 1, payload: [] });

    expect(tickHandler).toHaveBeenCalledTimes(1);
    expect(pnlHandler).not.toHaveBeenCalled();
  });

  it("re-subscribing after the last unsubscribe fires onFirstSubscriber again", () => {
    const onFirst = vi.fn();
    const mux = new TopicMultiplexer(onFirst, vi.fn());

    const unsub = mux.subscribe("agent_status", vi.fn());
    unsub();
    mux.subscribe("agent_status", vi.fn());

    expect(onFirst).toHaveBeenCalledTimes(2);
  });

  it("calling the same unsubscribe function twice is a no-op the second time", () => {
    const onLast = vi.fn();
    const mux = new TopicMultiplexer(vi.fn(), onLast);
    const unsub = mux.subscribe("signals", vi.fn());

    unsub();
    unsub();

    expect(onLast).toHaveBeenCalledTimes(1);
  });

  it("activeTopics reflects only topics with at least one live subscriber", () => {
    const mux = new TopicMultiplexer(vi.fn(), vi.fn());
    const unsub = mux.subscribe("bars:EURUSD:5s", vi.fn());
    mux.subscribe("pnl", vi.fn());

    expect(mux.activeTopics().sort()).toEqual(["bars:EURUSD:5s", "pnl"]);
    unsub();
    expect(mux.activeTopics()).toEqual(["pnl"]);
  });
});
