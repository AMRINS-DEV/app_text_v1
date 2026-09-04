/** §11.1: "roles: owner | trader | analyst | viewer; per-action guards." */
export enum Role {
  Owner = "owner",
  Trader = "trader",
  Analyst = "analyst",
  Viewer = "viewer",
}

export const ALL_ROLES = [Role.Owner, Role.Trader, Role.Analyst, Role.Viewer];

/** Roles allowed to mutate trading state (mode, kill switch, positions). */
export const TRADING_ROLES = [Role.Owner, Role.Trader];
