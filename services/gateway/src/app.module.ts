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
 * §11.1 module map. Auth, RBAC, realtime, trading, settings, stats and
 * charts are real as of Phase 4; patterns and news are real as of Phase 6
 * (see README's Phase 4/6 sections for what's synthetic vs. real
 * underneath each). Signals, agents, backtest, journal and system remain
 * Phase 0 stubs — none of them are in §17's Phase 4 or 6 exit lists; they
 * need a live gRPC bridge to `tradeos-core` and/or further phases to be
 * meaningful.
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
