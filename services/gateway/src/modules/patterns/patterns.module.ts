import { Module } from "@nestjs/common";

import { JwtAuthGuard } from "../../common/jwt-auth.guard";
import { AuthModule } from "../auth/auth.module";
import { PatternsController } from "./patterns.controller";
import { PatternsService } from "./patterns.service";

/** Job trigger -> agent, results, verification stats (§11.1, §12.3). */
@Module({
  imports: [AuthModule],
  controllers: [PatternsController],
  providers: [PatternsService, JwtAuthGuard],
})
export class PatternsModule {}
