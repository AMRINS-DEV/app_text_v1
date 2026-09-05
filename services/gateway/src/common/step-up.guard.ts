import { CanActivate, ExecutionContext, Injectable, SetMetadata, ForbiddenException } from "@nestjs/common";
import { Reflector } from "@nestjs/core";

import { TokenService } from "../modules/auth/token.service";
import type { AuthenticatedUser } from "./current-user.decorator";

export const REQUIRE_STEP_UP_KEY = "requireStepUp";
/** Marks a route as needing a fresh step-up token (§13's 2FA re-prompt list). */
export const RequireStepUp = () => SetMetadata(REQUIRE_STEP_UP_KEY, true);

interface RequestLike {
  headers: { "x-step-up-token"?: string };
  user: AuthenticatedUser;
}

/** Must run after `JwtAuthGuard`. Only enforces when `@RequireStepUp()` is present. */
@Injectable()
export class StepUpGuard implements CanActivate {
  constructor(
    private readonly reflector: Reflector,
    private readonly tokens: TokenService,
  ) {}

  canActivate(ctx: ExecutionContext): boolean {
    const required = this.reflector.getAllAndOverride<boolean | undefined>(REQUIRE_STEP_UP_KEY, [
      ctx.getHandler(),
      ctx.getClass(),
    ]);
    if (!required) return true;

    const req = ctx.switchToHttp().getRequest<RequestLike>();
    const header = req.headers["x-step-up-token"];
    if (!header) throw new ForbiddenException("this action requires step-up (2FA) re-authentication");
    const claims = this.tokens.verify(header, "stepup");
    if (claims.sub !== req.user.id) {
      throw new ForbiddenException("step-up token does not belong to the authenticated user");
    }
    return true;
  }
}
