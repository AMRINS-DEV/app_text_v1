import { Inject, Injectable } from "@nestjs/common";

import { TopicBus } from "../realtime/topic-bus";
import { AuditLogService } from "./audit-log.service";
import { CORE_CLIENT } from "./trading.constants";
import type { CoreClient, KillSwitchResult, Position, TradingMode } from "./trading.types";

/** Orchestrates `CoreClient` calls with the audit logging and realtime
 * fan-out §11.1/§11.2/§11.3 all require around them. */
@Injectable()
export class TradingService {
  constructor(
    @Inject(CORE_CLIENT) private readonly core: CoreClient,
    private readonly bus: TopicBus,
    private readonly audit: AuditLogService,
  ) {}

  getPositions(): Position[] {
    return this.core.getAccount().positions;
  }

  /** §11.2: "immediate flatten+halt, audit-logged" — no step-up, by design:
   * a safety stop must never be gated behind a second auth prompt. */
  killSwitch(userId: string): KillSwitchResult {
    const result = this.core.killSwitch();
    this.audit.record(userId, "trading.kill_switch", { ...result });
    this.publishAccountAndPnl();
    return result;
  }

  /** §13: re-enabling trading after a kill switch is on the step-up list
   * ("kill-switch disable"), enforced by `StepUpGuard` on the controller
   * route, not here. */
  resetKillSwitch(userId: string): void {
    this.core.resetKillSwitch();
    this.audit.record(userId, "trading.kill_switch_reset", {});
    this.publishAccountAndPnl();
  }

  setMode(userId: string, mode: TradingMode): void {
    this.core.setMode(mode);
    this.audit.record(userId, "trading.set_mode", { mode });
    this.publishAccountAndPnl();
  }

  closePosition(userId: string, id: string, fraction?: number): Position | null {
    const result = this.core.closePosition(id, fraction);
    this.audit.record(userId, "trading.close_position", { id, fraction: fraction ?? 1 });
    this.publishAccountAndPnl();
    return result;
  }

  private publishAccountAndPnl(): void {
    this.bus.publish("positions", this.core.getAccount());
    this.bus.publish("pnl", this.core.getPnl());
  }
}
