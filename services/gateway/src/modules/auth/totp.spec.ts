import { base32Decode, base32Encode, deterministicTotpSecretForFixture, generateTotp, hotp, verifyTotp } from "./totp";

// RFC 4226 Appendix D's published HOTP test vectors: secret "12345678901234567890"
// (ASCII, 20 bytes), counters 0..9. This is the standard proof that an HOTP
// implementation is bit-for-bit correct, not just internally self-consistent.
const RFC4226_SECRET = Buffer.from("12345678901234567890", "ascii");
const RFC4226_EXPECTED = [
  "755224",
  "287082",
  "359152",
  "969429",
  "338314",
  "254676",
  "287922",
  "162583",
  "399871",
  "520489",
];

describe("hotp", () => {
  it.each(RFC4226_EXPECTED.map((expected, counter) => [counter, expected] as const))(
    "matches RFC 4226's test vector for counter %i",
    (counter, expected) => {
      expect(hotp(RFC4226_SECRET, counter)).toBe(expected);
    },
  );
});

describe("base32", () => {
  it("round-trips arbitrary bytes", () => {
    const bytes = Buffer.from([0, 1, 2, 253, 254, 255, 17, 42]);
    expect(base32Decode(base32Encode(bytes))).toEqual(bytes);
  });

  it("matches RFC 4648's own worked example", () => {
    // RFC 4648 §10: BASE32("foobar") = "MZXW6YTBOI======"
    expect(base32Encode(Buffer.from("foobar", "ascii"))).toBe("MZXW6YTBOI");
    expect(base32Decode("MZXW6YTBOI======").toString("ascii")).toBe("foobar");
  });
});

describe("deterministicTotpSecretForFixture", () => {
  it("returns the same secret for the same seed every time", () => {
    expect(deterministicTotpSecretForFixture("owner")).toBe(deterministicTotpSecretForFixture("owner"));
  });

  it("returns different secrets for different seeds", () => {
    expect(deterministicTotpSecretForFixture("owner")).not.toBe(deterministicTotpSecretForFixture("trader"));
  });
});

describe("verifyTotp", () => {
  it("accepts the code generated for the same instant", () => {
    const secret = base32Encode(RFC4226_SECRET);
    const now = 1_700_000_000;
    const code = generateTotp(secret, {}, now);
    expect(verifyTotp(secret, code, {}, now)).toBe(true);
  });

  it("tolerates one period of clock skew but not two", () => {
    const secret = base32Encode(RFC4226_SECRET);
    const now = 1_700_000_000;
    const code = generateTotp(secret, {}, now);
    expect(verifyTotp(secret, code, {}, now + 30)).toBe(true);
    expect(verifyTotp(secret, code, {}, now + 90)).toBe(false);
  });

  it("rejects a code from a different secret", () => {
    const secretA = base32Encode(RFC4226_SECRET);
    const secretB = base32Encode(Buffer.from("different-secret-bytes!"));
    const code = generateTotp(secretA, {}, 1_700_000_000);
    expect(verifyTotp(secretB, code, {}, 1_700_000_000)).toBe(false);
  });
});
