import { createParamDecorator, ExecutionContext } from "@nestjs/common";

import { Role } from "./roles";

export interface AuthenticatedUser {
  id: string;
  role: Role;
}

export const CurrentUser = createParamDecorator((_: unknown, ctx: ExecutionContext): AuthenticatedUser => {
  const req = ctx.switchToHttp().getRequest<{ user: AuthenticatedUser }>();
  return req.user;
});
