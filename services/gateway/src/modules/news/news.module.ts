import { Module } from "@nestjs/common";

import { JwtAuthGuard } from "../../common/jwt-auth.guard";
import { AuthModule } from "../auth/auth.module";
import { NewsController } from "./news.controller";
import { NewsService } from "./news.service";

/** Feed, impact analysis, graph queries (§11.1, §12.4). */
@Module({
  imports: [AuthModule],
  controllers: [NewsController],
  providers: [NewsService, JwtAuthGuard],
})
export class NewsModule {}
