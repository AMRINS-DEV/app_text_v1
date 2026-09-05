import { Controller, Get, Query, UseGuards } from "@nestjs/common";

import { JwtAuthGuard } from "../../common/jwt-auth.guard";
import { ZodBody } from "../../common/zod-body.pipe";
import { ListNewsQueryDto, NewsImpactStabilityQueryDto } from "./news.dto";
import { NewsService } from "./news.service";

@Controller("api/news")
@UseGuards(JwtAuthGuard)
export class NewsController {
  constructor(private readonly news: NewsService) {}

  @Get()
  list(@Query(new ZodBody(ListNewsQueryDto)) query: ListNewsQueryDto) {
    return this.news.timeline(query.symbol);
  }

  @Get("impact-stability")
  impactStability(@Query(new ZodBody(NewsImpactStabilityQueryDto)) query: NewsImpactStabilityQueryDto) {
    return this.news.impactStability(query.event_type, query.symbol, query.horizon_min);
  }
}
