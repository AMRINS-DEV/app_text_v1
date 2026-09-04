import { describe, expect, it, vi } from "vitest";

import { DataProvider } from "./data-provider";

interface Bar {
  t: number;
  close: number;
}

describe("DataProvider", () => {
  it("loadWindow fetches and forwards the window to onData", async () => {
    const bars: Bar[] = [{ t: 0, close: 1 }, { t: 60_000, close: 2 }];
    const onData = vi.fn();
    const provider = new DataProvider<Bar>({
      fetchWindow: async () => bars,
      timeOf: (b) => b.t,
      periodMs: 60_000,
      onData,
      onAppend: vi.fn(),
    });

    await provider.loadWindow(0, 120_000);

    expect(onData).toHaveBeenCalledWith(bars);
    expect(provider.currentBars()).toEqual(bars);
  });

  it("appendLive with a matching open time updates the last bar in place, not appends", () => {
    const onAppend = vi.fn();
    const provider = new DataProvider<Bar>({
      fetchWindow: async () => [],
      timeOf: (b) => b.t,
      periodMs: 60_000,
      onData: vi.fn(),
      onAppend,
    });
    provider.appendLive({ t: 0, close: 1 });
    provider.appendLive({ t: 0, close: 1.5 });

    expect(provider.currentBars()).toEqual([{ t: 0, close: 1.5 }]);
    expect(onAppend).toHaveBeenCalledTimes(2);
  });

  it("appendLive with the next expected open time appends without a gap callback", () => {
    const onGapDetected = vi.fn();
    const provider = new DataProvider<Bar>({
      fetchWindow: async () => [],
      timeOf: (b) => b.t,
      periodMs: 60_000,
      onData: vi.fn(),
      onAppend: vi.fn(),
      onGapDetected,
    });
    provider.appendLive({ t: 0, close: 1 });
    provider.appendLive({ t: 60_000, close: 2 });

    expect(provider.currentBars()).toHaveLength(2);
    expect(onGapDetected).not.toHaveBeenCalled();
  });

  it("appendLive with a skipped period reports the gap and still appends", () => {
    const onGapDetected = vi.fn();
    const provider = new DataProvider<Bar>({
      fetchWindow: async () => [],
      timeOf: (b) => b.t,
      periodMs: 60_000,
      onData: vi.fn(),
      onAppend: vi.fn(),
      onGapDetected,
    });
    provider.appendLive({ t: 0, close: 1 });
    provider.appendLive({ t: 180_000, close: 2 }); // skipped the bar at 60_000

    expect(onGapDetected).toHaveBeenCalledWith(60_000, 180_000);
    expect(provider.currentBars()).toHaveLength(2);
  });

  it("appendLive silently drops a bar older than the last one (stale/out-of-order)", () => {
    const onAppend = vi.fn();
    const provider = new DataProvider<Bar>({
      fetchWindow: async () => [],
      timeOf: (b) => b.t,
      periodMs: 60_000,
      onData: vi.fn(),
      onAppend,
    });
    provider.appendLive({ t: 120_000, close: 3 });
    provider.appendLive({ t: 60_000, close: 2 });

    expect(provider.currentBars()).toEqual([{ t: 120_000, close: 3 }]);
    expect(onAppend).toHaveBeenCalledTimes(1);
  });
});
