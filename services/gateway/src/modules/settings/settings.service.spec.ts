import { SettingsService } from "./settings.service";
import { DEFAULT_SETTINGS } from "./settings.types";

describe("SettingsService", () => {
  it("returns the default settings before any update", () => {
    const service = new SettingsService();
    expect(service.get()).toEqual(DEFAULT_SETTINGS);
  });

  it("merges a partial update into the risk profile without dropping other fields", () => {
    const service = new SettingsService();
    service.update({ riskProfile: { riskPerTradePct: 1 } as never });
    const settings = service.get();
    expect(settings.riskProfile.riskPerTradePct).toBe(1);
    expect(settings.riskProfile.fractionalKellyCap).toBe(DEFAULT_SETTINGS.riskProfile.fractionalKellyCap);
  });

  it("replaces allowedPairs wholesale rather than merging (it's a list, not a map)", () => {
    const service = new SettingsService();
    service.update({ allowedPairs: ["EURUSD"] });
    expect(service.get().allowedPairs).toEqual(["EURUSD"]);
  });

  it("get() returns a defensive copy — mutating it doesn't affect stored state", () => {
    const service = new SettingsService();
    const settings = service.get();
    settings.allowedPairs.push("FAKE");
    expect(service.get().allowedPairs).not.toContain("FAKE");
  });
});
