import { Injectable, OnModuleDestroy, OnModuleInit } from "@nestjs/common";

import { TopicBus } from "./topic-bus";

export interface AgentStatusSnapshot {
  agents: Array<{ name: string; healthy: boolean; costUsdPerHour: number; lastSignalAgeSeconds: number }>;
  ts: number;
}

/**
 * `agent_status` (§11.2) has no real agent layer to report on yet — that's
 * Phase 5 scope. This publishes a static roster with a synthetic heartbeat
 * so the WS topic and dashboard wiring exist and are testable ahead of the
 * agents actually existing, the same "fix the surface, fill the logic
 * later" pattern used for the Phase 0 gateway stubs.
 */
@Injectable()
export class AgentStatusFeedService implements OnModuleInit, OnModuleDestroy {
  private timer: NodeJS.Timeout | undefined;
  private readonly agentNames = ["technical-tier1", "sentiment-tier2", "pattern-tier3", "risk-overseer"];

  constructor(private readonly bus: TopicBus) {}

  onModuleInit(): void {
    this.timer = setInterval(() => this.emit(), 3_000);
    this.emit();
  }

  onModuleDestroy(): void {
    if (this.timer) clearInterval(this.timer);
  }

  private emit(): void {
    const snapshot: AgentStatusSnapshot = {
      agents: this.agentNames.map((name) => ({
        name,
        healthy: true,
        costUsdPerHour: 0,
        lastSignalAgeSeconds: 0,
      })),
      ts: Date.now(),
    };
    this.bus.publish("agent_status", snapshot);
  }
}
