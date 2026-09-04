import { Test } from "@nestjs/testing";

import { TopicBus } from "../realtime/topic-bus";
import { AuditLogService } from "./audit-log.service";
import { InMemoryCoreClient } from "./in-memory-core-client";
import { CORE_CLIENT } from "./trading.constants";
import { TradingService } from "./trading.service";

async function buildTradingService(): Promise<{ trading: TradingService; bus: TopicBus; audit: AuditLogService }> {
  const moduleRef = await Test.createTestingModule({
    providers: [
      TradingService,
      TopicBus,
      AuditLogService,
      { provide: CORE_CLIENT, useClass: InMemoryCoreClient },
    ],
  }).compile();
  return {
    trading: moduleRef.get(TradingService),
    bus: moduleRef.get(TopicBus),
    audit: moduleRef.get(AuditLogService),
  };
}

describe("TradingService", () => {
  it("starts with the seeded open positions", async () => {
    const { trading } = await buildTradingService();
    expect(trading.getPositions().length).toBeGreaterThan(0);
  });

  it("kill switch flattens every open position in well under the §17 500ms budget", async () => {
    const { trading } = await buildTradingService();
    const before = trading.getPositions().length;
    expect(before).toBeGreaterThan(0);

    const result = trading.killSwitch("user-1");

    expect(result.flattenedCount).toBe(before);
    expect(trading.getPositions()).toHaveLength(0);
    expect(result.elapsedMs).toBeLessThan(500);
  });

  it("kill switch is audit-logged", async () => {
    const { trading, audit } = await buildTradingService();
    trading.killSwitch("user-1");
    const entries = audit.list();
    expect(entries).toHaveLength(1);
    expect(entries[0]).toMatchObject({ userId: "user-1", action: "trading.kill_switch" });
  });

  it("publishes a positions and a pnl snapshot to the topic bus on kill switch", async () => {
    const { trading, bus } = await buildTradingService();
    const positionsFrames: unknown[] = [];
    const pnlFrames: unknown[] = [];
    bus.subscribe("positions", (p) => positionsFrames.push(p));
    bus.subscribe("pnl", (p) => pnlFrames.push(p));

    trading.killSwitch("user-1");

    expect(positionsFrames).toHaveLength(1);
    expect(pnlFrames).toHaveLength(1);
    expect(positionsFrames[0]).toMatchObject({ mode: "halted", killSwitchEngaged: true, positions: [] });
  });

  it("setMode refuses to change mode while the kill switch is engaged", async () => {
    const { trading } = await buildTradingService();
    trading.killSwitch("user-1");
    expect(() => trading.setMode("user-1", "live")).toThrow();
  });

  it("resetKillSwitch clears the halt so mode can be set again", async () => {
    const { trading } = await buildTradingService();
    trading.killSwitch("user-1");
    trading.resetKillSwitch("user-1");
    expect(() => trading.setMode("user-1", "live")).not.toThrow();
  });

  it("closePosition with a fraction partially reduces quantity instead of removing it", async () => {
    const { trading } = await buildTradingService();
    const before = trading.getPositions().length;
    const [position] = trading.getPositions();
    const originalQuantity = position.quantity;

    const updated = trading.closePosition("user-1", position.id, 0.5);

    expect(updated).not.toBeNull();
    expect(updated?.quantity).toBeCloseTo(originalQuantity * 0.5);
    expect(trading.getPositions()).toHaveLength(before);
  });

  it("closePosition with no fraction fully removes the position", async () => {
    const { trading } = await buildTradingService();
    const [position] = trading.getPositions();
    const before = trading.getPositions().length;

    const result = trading.closePosition("user-1", position.id);

    expect(result).toBeNull();
    expect(trading.getPositions()).toHaveLength(before - 1);
  });
});
