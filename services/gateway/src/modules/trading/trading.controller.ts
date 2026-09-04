import { Body, Controller, Get, Param, Post, UseGuards } from "@nestjs/common";

import { CurrentUser } from "../../common/current-user.decorator";
import type { AuthenticatedUser } from "../../common/current-user.decorator";
import { JwtAuthGuard } from "../../common/jwt-auth.guard";
import { Roles, RolesGuard } from "../../common/roles.guard";
import { TRADING_ROLES } from "../../common/roles";
import { RequireStepUp, StepUpGuard } from "../../common/step-up.guard";
import { ZodBody } from "../../common/zod-body.pipe";
import { ClosePositionDto, SetModeDto } from "./trading.dto";
import { TradingService } from "./trading.service";

/** §11.2 key API surface (abridged) for the trading module. */
@Controller("api/trading")
@UseGuards(JwtAuthGuard, RolesGuard, StepUpGuard)
export class TradingController {
  constructor(private readonly trading: TradingService) {}

  @Post("mode")
  @Roles(...TRADING_ROLES)
  @RequireStepUp()
  setMode(@CurrentUser() user: AuthenticatedUser, @Body(new ZodBody(SetModeDto)) body: SetModeDto) {
    this.trading.setMode(user.id, body.mode);
    return { ok: true };
  }

  @Post("kill-switch")
  @Roles(...TRADING_ROLES)
  killSwitch(@CurrentUser() user: AuthenticatedUser) {
    return this.trading.killSwitch(user.id);
  }

  @Post("kill-switch/reset")
  @Roles(...TRADING_ROLES)
  @RequireStepUp()
  resetKillSwitch(@CurrentUser() user: AuthenticatedUser) {
    this.trading.resetKillSwitch(user.id);
    return { ok: true };
  }

  @Get("positions")
  positions() {
    return this.trading.getPositions();
  }

  @Post("positions/:id/close")
  @Roles(...TRADING_ROLES)
  closePosition(
    @CurrentUser() user: AuthenticatedUser,
    @Param("id") id: string,
    @Body(new ZodBody(ClosePositionDto)) body: ClosePositionDto,
  ) {
    return this.trading.closePosition(user.id, id, body.fraction);
  }
}
