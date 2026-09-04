import { Module } from "@nestjs/common";
import { RealtimeGateway } from "./realtime.gateway";

/**
 * WS gateway, Redis pub/sub fan-out, per-topic subscribe/unsubscribe,
 * 50-100ms coalescing, MessagePack framing, backpressure-to-conflation
 * (§11.1, §11.3). The coalescing/backpressure logic is Phase 4 scope.
 */
@Module({
  providers: [RealtimeGateway],
})
export class RealtimeModule {}
