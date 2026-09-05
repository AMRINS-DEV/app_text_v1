import * as bcrypt from "bcryptjs";
import { Injectable } from "@nestjs/common";

import { Role } from "../../common/roles";
import { deterministicTotpSecretForFixture } from "./totp";

export interface UserRecord {
  id: string;
  username: string;
  passwordHash: string;
  role: Role;
  totpSecret: string;
}

/**
 * §11.1's real deployment backs this with a Postgres user repository; this
 * sandbox has no Postgres to run one against, so this is an in-memory store
 * seeded with one dev account per role — the same "real logic, mock
 * infrastructure" split used for `SimBroker` in Phase 2. Passwords and TOTP
 * secrets below are fixture values for this repo only, never real
 * credentials.
 */
@Injectable()
export class UsersStore {
  private readonly byUsername = new Map<string, UserRecord>();

  constructor() {
    for (const [username, role] of [
      ["owner", Role.Owner],
      ["trader", Role.Trader],
      ["analyst", Role.Analyst],
      ["viewer", Role.Viewer],
    ] as const) {
      this.byUsername.set(username, {
        id: `user-${username}`,
        username,
        passwordHash: bcrypt.hashSync(`${username}-dev-password`, 10),
        role,
        totpSecret: deterministicTotpSecretForFixture(`tradeos-fixture-totp:${username}`),
      });
    }
  }

  findByUsername(username: string): UserRecord | undefined {
    return this.byUsername.get(username);
  }

  findById(id: string): UserRecord | undefined {
    for (const user of this.byUsername.values()) {
      if (user.id === id) return user;
    }
    return undefined;
  }
}
