import { Body, Controller, Post, UseGuards } from "@nestjs/common";

import { ZodBody } from "../../common/zod-body.pipe";
import { CurrentUser } from "../../common/current-user.decorator";
import { JwtAuthGuard } from "../../common/jwt-auth.guard";
import { AuthService } from "./auth.service";
import { LoginDto, TotpDto, RefreshDto, StepUpDto } from "./auth.dto";
import type { AuthenticatedUser } from "../../common/current-user.decorator";

/** §11.2: POST /api/auth/login → tokens (here split into login → TOTP, per §11.1's 2FA flow). */
@Controller("api/auth")
export class AuthController {
  constructor(private readonly auth: AuthService) {}

  @Post("login")
  login(@Body(new ZodBody(LoginDto)) body: LoginDto) {
    return this.auth.login(body.username, body.password);
  }

  @Post("totp")
  totp(@Body(new ZodBody(TotpDto)) body: TotpDto) {
    return this.auth.verifyTotpAndIssueTokens(body.preAuthToken, body.totpCode);
  }

  @Post("refresh")
  refresh(@Body(new ZodBody(RefreshDto)) body: RefreshDto) {
    return this.auth.refresh(body.refreshToken);
  }

  @Post("step-up")
  @UseGuards(JwtAuthGuard)
  stepUp(@CurrentUser() user: AuthenticatedUser, @Body(new ZodBody(StepUpDto)) body: StepUpDto) {
    return this.auth.stepUp(user.id, body.totpCode);
  }
}
