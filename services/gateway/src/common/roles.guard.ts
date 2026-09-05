import { CanActivate, ExecutionContext, Injectable, SetMetadata, ForbiddenException } from "@nestjs/common";
import { Reflector } from "@nestjs/core";

import { Role } from "./roles";
import type { AuthenticatedUser } from "./current-user.decorator";

export const ROLES_KEY = "roles";
export const Roles = (...roles: Role[]) => SetMetadata(ROLES_KEY, roles);

/** Per-action guard (§11.1): must run after `JwtAuthGuard` has set `req.user`. */
@Injectable()
export class RolesGuard implements CanActivate {
  constructor(private readonly reflector: Reflector) {}

  canActivate(ctx: ExecutionContext): boolean {
    const required = this.reflector.getAllAndOverride<Role[] | undefined>(ROLES_KEY, [
      ctx.getHandler(),
      ctx.getClass(),
    ]);
    if (!required || required.length === 0) return true;
    const { user } = ctx.switchToHttp().getRequest<{ user: AuthenticatedUser }>();
    if (!required.includes(user.role)) {
      throw new ForbiddenException(`role '${user.role}' cannot perform this action`);
    }
    return true;
  }
}
