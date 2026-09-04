import { describe, expect, it, vi } from "vitest";

import { SyncBus } from "./sync-bus";

describe("SyncBus", () => {
  it("delivers a published crosshair sync to every subscriber", () => {
    const bus = new SyncBus();
    const a = vi.fn();
    const b = vi.fn();
    bus.onCrosshair(a);
    bus.onCrosshair(b);
    bus.publishCrosshair({ time: 123, sourceId: "pane-1" });
    expect(a).toHaveBeenCalledWith({ time: 123, sourceId: "pane-1" });
    expect(b).toHaveBeenCalledWith({ time: 123, sourceId: "pane-1" });
  });

  it("stops delivering to a listener after it unsubscribes", () => {
    const bus = new SyncBus();
    const listener = vi.fn();
    const unsubscribe = bus.onCrosshair(listener);
    unsubscribe();
    bus.publishCrosshair({ time: 1, sourceId: "pane-1" });
    expect(listener).not.toHaveBeenCalled();
  });

  it("keeps crosshair and range subscriptions independent", () => {
    const bus = new SyncBus();
    const crosshairListener = vi.fn();
    const rangeListener = vi.fn();
    bus.onCrosshair(crosshairListener);
    bus.onRange(rangeListener);

    bus.publishRange({ fromMs: 0, toMs: 1000, sourceId: "pane-2" });

    expect(rangeListener).toHaveBeenCalledTimes(1);
    expect(crosshairListener).not.toHaveBeenCalled();
  });

  it("a pane can filter out its own published sync by sourceId", () => {
    const bus = new SyncBus();
    const received: string[] = [];
    const mySourceId = "pane-1";
    bus.onCrosshair((sync) => {
      if (sync.sourceId === mySourceId) return;
      received.push(sync.sourceId);
    });

    bus.publishCrosshair({ time: 1, sourceId: "pane-1" });
    bus.publishCrosshair({ time: 2, sourceId: "pane-2" });

    expect(received).toEqual(["pane-2"]);
  });
});
