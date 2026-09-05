import { Injectable } from "@nestjs/common";

import { DEFAULT_SETTINGS, Settings } from "./settings.types";
import type { UpdateSettingsDto } from "./settings.dto";

/** §11.1: "risk profiles, allowed pairs, modes, agent config, model
 * routing." In-memory here — Phase 4's real deployment backs this with
 * Postgres, which this sandbox doesn't have. */
@Injectable()
export class SettingsService {
  private settings: Settings = structuredClone(DEFAULT_SETTINGS);

  get(): Settings {
    return structuredClone(this.settings);
  }

  update(patch: UpdateSettingsDto): Settings {
    this.settings = {
      ...this.settings,
      ...patch,
      riskProfile: { ...this.settings.riskProfile, ...patch.riskProfile },
      modelRouting: { ...this.settings.modelRouting, ...patch.modelRouting },
    };
    return this.get();
  }
}
