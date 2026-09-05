import { Injectable } from "@nestjs/common";

export interface AuditLogEntry {
  ts: number;
  userId: string;
  action: string;
  meta: Record<string, unknown>;
}

/**
 * §11.1: "append-only audit logging." In-memory here (no Postgres in this
 * sandbox); `record` never removes or mutates an existing entry, which is
 * the property that actually matters for an audit log and the one this
 * module's tests check.
 */
@Injectable()
export class AuditLogService {
  private readonly entries: AuditLogEntry[] = [];

  record(userId: string, action: string, meta: Record<string, unknown> = {}): void {
    this.entries.push({ ts: Date.now(), userId, action, meta });
  }

  list(): readonly AuditLogEntry[] {
    return this.entries;
  }
}
