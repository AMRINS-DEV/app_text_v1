import {
  CandlestickSeries,
  ColorType,
  createChart,
  LineSeries,
  type IChartApi,
  type ISeriesApi,
  type UTCTimestamp,
} from "lightweight-charts";

export interface Bar {
  /** ms epoch (open time). */
  time: number;
  open: number;
  high: number;
  low: number;
  close: number;
}

export interface LinePoint {
  /** ms epoch. */
  time: number;
  value: number;
}

export interface ChartHostOptions {
  container: HTMLElement;
  dark?: boolean;
}

function toUtcTimestamp(msEpoch: number): UTCTimestamp {
  return Math.floor(msEpoch / 1000) as UTCTimestamp;
}

function candlestickPoint(bar: Bar) {
  return { time: toUtcTimestamp(bar.time), open: bar.open, high: bar.high, low: bar.low, close: bar.close };
}

/**
 * §12.2's "ChartHost lifecycle" over TradingView Lightweight Charts v5.
 * This is the one piece of the chart engine that genuinely needs a real
 * browser (canvas rendering, `ResizeObserver`) — it has no unit tests in
 * this sandbox for that reason; `DataProvider` and `SyncBus` carry the
 * chart-engine's tested logic, and this class is deliberately kept thin,
 * mostly delegating straight into `lightweight-charts` calls, so there is
 * as little untested logic here as possible.
 */
export class ChartHost {
  readonly chart: IChartApi;
  private readonly resizeObserver: ResizeObserver;
  private readonly container: HTMLElement;
  private candlestickSeries: ISeriesApi<"Candlestick"> | undefined;
  private lineSeries: ISeriesApi<"Line"> | undefined;

  constructor(options: ChartHostOptions) {
    this.container = options.container;
    const dark = options.dark ?? true;
    this.chart = createChart(options.container, {
      layout: {
        background: { type: ColorType.Solid, color: dark ? "#0b0e14" : "#ffffff" },
        textColor: dark ? "#c9d1d9" : "#1a1a1a",
      },
      grid: {
        vertLines: { color: dark ? "#1c2128" : "#e1e4e8" },
        horzLines: { color: dark ? "#1c2128" : "#e1e4e8" },
      },
      width: options.container.clientWidth,
      height: options.container.clientHeight,
      timeScale: { timeVisible: true, secondsVisible: true },
    });

    this.resizeObserver = new ResizeObserver(() => {
      this.chart.resize(this.container.clientWidth, this.container.clientHeight);
    });
    this.resizeObserver.observe(this.container);
  }

  setCandlesticks(bars: readonly Bar[]): void {
    this.candlestickSeries ??= this.chart.addSeries(CandlestickSeries);
    this.candlestickSeries.setData(bars.map(candlestickPoint));
  }

  upsertCandlestick(bar: Bar): void {
    this.candlestickSeries ??= this.chart.addSeries(CandlestickSeries);
    this.candlestickSeries.update(candlestickPoint(bar));
  }

  setLine(points: readonly LinePoint[]): void {
    this.lineSeries ??= this.chart.addSeries(LineSeries);
    this.lineSeries.setData(points.map((p) => ({ time: toUtcTimestamp(p.time), value: p.value })));
  }

  /** Appends (or updates, for the same timestamp) a single point without
   * replacing the whole series — the shape a live feed update needs. */
  updateLine(point: LinePoint): void {
    this.lineSeries ??= this.chart.addSeries(LineSeries);
    this.lineSeries.update({ time: toUtcTimestamp(point.time), value: point.value });
  }

  subscribeCrosshairMove(callback: (timeMs: number | null) => void): () => void {
    const handler = (param: Parameters<Parameters<IChartApi["subscribeCrosshairMove"]>[0]>[0]) => {
      callback(param.time ? Number(param.time) * 1000 : null);
    };
    this.chart.subscribeCrosshairMove(handler);
    return () => this.chart.unsubscribeCrosshairMove(handler);
  }

  setVisibleRange(fromMs: number, toMs: number): void {
    this.chart.timeScale().setVisibleRange({ from: toUtcTimestamp(fromMs), to: toUtcTimestamp(toMs) });
  }

  dispose(): void {
    this.resizeObserver.disconnect();
    this.chart.remove();
  }
}
