import { resolveFixedHorizonMove, resolvePatternOutcome } from "./outcome-resolution";

describe("resolvePatternOutcome", () => {
  it("confirms a long pattern when the target is hit before invalidation", () => {
    const resolution = resolvePatternOutcome(
      "Long",
      100.0,
      104.0,
      98.0,
      [101.0, 102.5, 104.5],
      [99.5, 100.5, 103.0],
      0.1,
      2.0,
    );
    expect(resolution.verdict).toBe("confirmed");
    expect(resolution.barsToResolution).toBe(2);
    expect(resolution.rMultiple).toBeCloseTo(2.0, 6); // (104-100)/(100-98)
    expect(resolution.movePips).toBeCloseTo(40.0, 6);
    expect(resolution.moveAtr).toBeCloseTo(2.0, 6);
    expect(resolution.mfe).toBeCloseTo(4.5, 6);
  });

  it("fails a short pattern when invalidation is hit before the target", () => {
    const resolution = resolvePatternOutcome(
      "Short",
      100.0,
      96.0,
      102.0,
      [100.5, 101.0, 103.0],
      [99.0, 98.5, 98.0],
      0.1,
      2.0,
    );
    expect(resolution.verdict).toBe("failed");
    expect(resolution.rMultiple).toBeCloseTo(-1.0, 6);
    expect(resolution.movePips).toBeCloseTo(-20.0, 6);
  });

  it("times out when neither barrier is hit", () => {
    const resolution = resolvePatternOutcome("Long", 100.0, 110.0, 90.0, [101.0, 102.0], [99.0, 100.0], 0.1, 2.0);
    expect(resolution.verdict).toBe("timeout");
    expect(resolution.barsToResolution).toBeNull();
    expect(resolution.rMultiple).toBe(0);
    expect(resolution.mfe).toBeCloseTo(2.0, 6);
  });

  it("resolves a bar touching both barriers toward invalidation", () => {
    const resolution = resolvePatternOutcome("Long", 100.0, 104.0, 98.0, [105.0], [97.0], 0.1, 2.0);
    expect(resolution.verdict).toBe("failed");
  });

  it("rejects a target/invalidation pair equal to the entry price", () => {
    expect(() => resolvePatternOutcome("Long", 100.0, 104.0, 100.0, [101.0], [99.0], 0.1, 2.0)).toThrow();
  });
});

describe("resolveFixedHorizonMove", () => {
  it("detects a direction hit", () => {
    const move = resolveFixedHorizonMove(1.1, 1.105, "Long", 0.0001, 0.002);
    expect(move.direction).toBe("Long");
    expect(move.directionHit).toBe(true);
    expect(move.movePips).toBeCloseTo(50.0, 3);
    expect(move.moveAtr).toBeCloseTo(2.5, 3);
  });

  it("detects a direction miss", () => {
    const move = resolveFixedHorizonMove(1.1, 1.095, "Long", 0.0001, 0.002);
    expect(move.direction).toBe("Short");
    expect(move.directionHit).toBe(false);
  });

  it("reports flat when there is no movement", () => {
    const move = resolveFixedHorizonMove(1.1, 1.1, "Flat", 0.0001, 0.002);
    expect(move.direction).toBe("Flat");
    expect(move.directionHit).toBe(true);
  });
});
