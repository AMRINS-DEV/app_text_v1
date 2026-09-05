/**
 * §12.2's "DataProvider with windowed fetch + live append + gap repair."
 * Deliberately has no dependency on `ChartHost`, `lightweight-charts`, or
 * the DOM — it is pure data-flow logic (fetch a window, merge in live
 * updates, detect a gap) that a `ChartHost` wrapper drives by calling
 * `onData`/`onAppend` into the underlying series. That split is what makes
 * this testable without a browser/canvas, unlike `ChartHost` itself.
 */
export interface DataProviderOptions<TBar> {
  fetchWindow: (fromMs: number, toMs: number) => Promise<TBar[]>;
  timeOf: (bar: TBar) => number;
  periodMs: number;
  onData: (bars: readonly TBar[]) => void;
  onAppend: (bar: TBar) => void;
  /** Fires when a live bar's open time isn't exactly one period after the
   * previous bar — i.e. the feed skipped a period. Real gap *repair*
   * (re-fetching the missing range) is left to the caller via this hook
   * rather than done automatically here, since only the caller knows
   * whether a re-fetch is affordable/desired for a given pane. */
  onGapDetected?: (expectedTimeMs: number, actualTimeMs: number) => void;
}

export class DataProvider<TBar> {
  private bars: TBar[] = [];

  constructor(private readonly options: DataProviderOptions<TBar>) {}

  async loadWindow(fromMs: number, toMs: number): Promise<void> {
    this.bars = await this.options.fetchWindow(fromMs, toMs);
    this.options.onData(this.bars.slice());
  }

  /** A live bar for the currently-open period updates the last bar in
   * place; a bar for the next period appends (checking for a skipped
   * period first); anything else is silently ignored as stale/out-of-order. */
  appendLive(bar: TBar): void {
    const last = this.bars[this.bars.length - 1];
    if (!last) {
      this.bars.push(bar);
      this.options.onAppend(bar);
      return;
    }

    const lastTime = this.options.timeOf(last);
    const barTime = this.options.timeOf(bar);

    if (barTime === lastTime) {
      this.bars[this.bars.length - 1] = bar;
      this.options.onAppend(bar);
      return;
    }

    if (barTime < lastTime) return; // stale/out-of-order, drop it

    const expected = lastTime + this.options.periodMs;
    if (barTime !== expected) this.options.onGapDetected?.(expected, barTime);
    this.bars.push(bar);
    this.options.onAppend(bar);
  }

  currentBars(): readonly TBar[] {
    return this.bars;
  }
}
