import { Injectable } from "@nestjs/common";
// Deep import into the package's CommonJS build, not its `main` (which
// points at the ESM build bundler consumers like the dashboard need,
// since `lightweight-charts` — pulled in by the barrel via `ChartHost` —
// only exposes an ESM `import` condition). `lttbDownsample` itself has
// zero dependencies, so the CJS build's copy of it is all the gateway
// needs, without ever touching `chart-host.js`.
import { lttbDownsample } from "@tradeos/chart-engine/dist/cjs/lttb";

import { generateHistoricalBars } from "./historical-bars";
import type { BarsQueryDto } from "./charts.dto";
import type { BarMessage } from "../realtime/market-feed.service";

/** §11.2's `GET /api/charts/bars ... → downsampled`. */
@Injectable()
export class ChartsService {
  bars(query: BarsQueryDto): BarMessage[] {
    const bars = generateHistoricalBars(query.sym, query.tf, query.from, query.to);
    if (!query.max_points) return bars;
    return lttbDownsample(
      bars,
      query.max_points,
      (bar) => bar.openTime,
      (bar) => bar.close,
    );
  }
}
