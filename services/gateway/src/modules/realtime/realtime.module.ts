import { Module } from "@nestjs/common";

import { AuthModule } from "../auth/auth.module";
import { AgentStatusFeedService } from "./agent-status-feed.service";
import { MarketFeedService } from "./market-feed.service";
import { RealtimeGateway } from "./realtime.gateway";
import { TopicBus } from "./topic-bus";

/**
 * WS gateway, per-topic fan-out, 50-100ms coalescing, MessagePack framing,
 * per-topic RBAC and backpressure-to-conflation (§11.1, §11.3). Market data
 * (ticks/bars) and agent status are synthetic feeds — see
 * `MarketFeedService`'s and `AgentStatusFeedService`'s doc comments for why.
 * `TopicBus` is exported so `TradingModule` can publish real
 * positions/pnl updates onto the same fan-out the gateway already serves.
 */
@Module({
  imports: [AuthModule],
  providers: [TopicBus, MarketFeedService, AgentStatusFeedService, RealtimeGateway],
  exports: [TopicBus],
})
export class RealtimeModule {}
