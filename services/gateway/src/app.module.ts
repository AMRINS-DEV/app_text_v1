import { Module } from "@nestjs/common";

import { AuthModule } from "./modules/auth/auth.module";
import { RbacModule } from "./modules/rbac/rbac.module";
import { SecretsModule } from "./modules/secrets/secrets.module";
import { SettingsModule } from "./modules/settings/settings.module";
import { TradingModule } from "./modules/trading/trading.module";
import { ChartsModule } from "./modules/charts/charts.module";
import { PatternsModule } from "./modules/patterns/patterns.module";
import { NewsModule } from "./modules/news/news.module";
import { SignalsModule } from "./modules/signals/signals.module";
import { StatsModule } from "./modules/stats/stats.module";
import { JournalModule } from "./modules/journal/journal.module";
import { AgentsModule } from "./modules/agents/agents.module";
import { BacktestModule } from "./modules/backtest/backtest.module";
import { SystemModule } from "./modules/system/system.module";
import { RealtimeModule } from "./modules/realtime/realtime.module";

/**
 * §11.1 module map. Every module below is a Phase 0 stub (no providers,
 * empty controllers) except where noted — they exist so the module
 * boundaries and RBAC/auth wiring points are fixed before Phase 4 fills
 * in real business logic against the Rust core over gRPC.
 */
@Module({
  imports: [
    AuthModule,
    RbacModule,
    SecretsModule,
    SettingsModule,
    TradingModule,
    ChartsModule,
    PatternsModule,
    NewsModule,
    SignalsModule,
    StatsModule,
    JournalModule,
    AgentsModule,
    BacktestModule,
    SystemModule,
    RealtimeModule,
  ],
})
export class AppModule {}
