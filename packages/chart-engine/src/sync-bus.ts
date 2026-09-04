export type Unsubscribe = () => void;

export interface CrosshairSync {
  /** ms epoch, or null when the crosshair has left the chart. */
  time: number | null;
  sourceId: string;
}

export interface RangeSync {
  fromMs: number;
  toMs: number;
  sourceId: string;
}

/**
 * §12.2's "SyncBus for crosshair and time-range sync" across the panes in
 * a `dockview` multi-chart workspace. Deliberately has no dependency on
 * `ChartHost` or the DOM — each `ChartHost` instance both publishes its own
 * crosshair/range changes here and subscribes to the others', filtering out
 * its own `sourceId` so it doesn't react to itself.
 */
export class SyncBus {
  private readonly crosshairListeners = new Set<(sync: CrosshairSync) => void>();
  private readonly rangeListeners = new Set<(sync: RangeSync) => void>();

  publishCrosshair(sync: CrosshairSync): void {
    for (const listener of this.crosshairListeners) listener(sync);
  }

  onCrosshair(listener: (sync: CrosshairSync) => void): Unsubscribe {
    this.crosshairListeners.add(listener);
    return () => this.crosshairListeners.delete(listener);
  }

  publishRange(sync: RangeSync): void {
    for (const listener of this.rangeListeners) listener(sync);
  }

  onRange(listener: (sync: RangeSync) => void): Unsubscribe {
    this.rangeListeners.add(listener);
    return () => this.rangeListeners.delete(listener);
  }
}
