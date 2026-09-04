import { Module } from "@nestjs/common";

/**
 * Roles: owner | trader | analyst | viewer; per-action guards (§11.1, §13).
 * Guard implementations are Phase 4 scope.
 */
@Module({})
export class RbacModule {}
