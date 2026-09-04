import { Body, Controller, Get, Put, UseGuards } from "@nestjs/common";

import { JwtAuthGuard } from "../../common/jwt-auth.guard";
import { Roles, RolesGuard } from "../../common/roles.guard";
import { TRADING_ROLES } from "../../common/roles";
import { ZodBody } from "../../common/zod-body.pipe";
import { UpdateSettingsDto } from "./settings.dto";
import { SettingsService } from "./settings.service";

@Controller("api/settings")
@UseGuards(JwtAuthGuard, RolesGuard)
export class SettingsController {
  constructor(private readonly settings: SettingsService) {}

  @Get()
  get() {
    return this.settings.get();
  }

  @Put()
  @Roles(...TRADING_ROLES)
  update(@Body(new ZodBody(UpdateSettingsDto)) body: UpdateSettingsDto) {
    return this.settings.update(body);
  }
}
