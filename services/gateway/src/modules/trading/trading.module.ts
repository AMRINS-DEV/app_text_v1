import { Module } from "@nestjs/common";
import { TradingController } from "./trading.controller";

/**
 * gRPC -> core: positions, orders, mode switch, kill switch (§11.1-11.2).
 * The gRPC client to `tradeos-core` is Phase 4 scope — until then every
 * endpoint here is a fixed contract with a not-implemented body, so the
 * dashboard's trading page (Phase 4) has a stable surface to build against.
 */
@Module({
  controllers: [TradingController],
})
export class TradingModule {}
