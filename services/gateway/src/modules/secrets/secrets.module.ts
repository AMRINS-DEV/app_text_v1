import { Module } from "@nestjs/common";

/**
 * Envelope encryption: OS keychain -> master key -> AES-256-GCM data keys in
 * Postgres (§13). Provider API keys are write-only via API; GET returns
 * masked values. Phase 4 scope.
 */
@Module({})
export class SecretsModule {}
