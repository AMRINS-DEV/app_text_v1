import { CanActivate, ExecutionContext, Injectable, UnauthorizedException } from "@nestjs/common";

import { TokenService } from "../modules/auth/token.service";
import type { AuthenticatedUser } from "./current-user.decorator";

interface RequestLike {
  headers: { authorization?: string };
  user?: AuthenticatedUser;
}

function bearerToken(req: RequestLike): string {
  const header = req.headers.authorization;
  if (!header?.startsWith("Bearer ")) throw new UnauthorizedException("missing bearer token");
  return header.slice("Bearer ".length);
}

/** Verifies a short-lived access token and attaches `req.user` (§11.1). */
@Injectable()
export class JwtAuthGuard implements CanActivate {
  constructor(private readonly tokens: TokenService) {}

  canActivate(ctx: ExecutionContext): boolean {
    const req = ctx.switchToHttp().getRequest<RequestLike>();
    const claims = this.tokens.verify(bearerToken(req), "access");
    req.user = { id: claims.sub, role: claims.role };
    return true;
  }
}
