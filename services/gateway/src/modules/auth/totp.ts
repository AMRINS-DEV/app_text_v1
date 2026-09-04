import { createHash, createHmac, randomBytes } from "node:crypto";

/**
 * RFC 4226 (HOTP) + RFC 6238 (TOTP), hand-rolled instead of depending on
 * `otplib`: otplib v13's default base32 plugin (`@scure/base`) is
 * ESM-only, and this workspace's Jest setup (ts-jest, CommonJS) can't
 * transform it without a babel detour. HOTP/TOTP are ~40 lines of
 * well-specified HMAC-SHA1 arithmetic — implementing them directly removes
 * the dependency entirely rather than fighting its module format. Verified
 * below against RFC 4226 Appendix D's published test vectors, not just
 * "it round-trips."
 */

const BASE32_ALPHABET = "ABCDEFGHIJKLMNOPQRSTUVWXYZ234567";

export function base32Encode(bytes: Buffer): string {
  let bits = 0;
  let value = 0;
  let output = "";
  for (const byte of bytes) {
    value = (value << 8) | byte;
    bits += 8;
    while (bits >= 5) {
      output += BASE32_ALPHABET[(value >>> (bits - 5)) & 0x1f];
      bits -= 5;
    }
  }
  if (bits > 0) {
    output += BASE32_ALPHABET[(value << (5 - bits)) & 0x1f];
  }
  return output;
}

export function base32Decode(encoded: string): Buffer {
  const cleaned = encoded.toUpperCase().replace(/=+$/, "");
  let bits = 0;
  let value = 0;
  const bytes: number[] = [];
  for (const char of cleaned) {
    const index = BASE32_ALPHABET.indexOf(char);
    if (index === -1) throw new Error(`invalid base32 character: ${char}`);
    value = (value << 5) | index;
    bits += 5;
    if (bits >= 8) {
      bytes.push((value >>> (bits - 8)) & 0xff);
      bits -= 8;
    }
  }
  return Buffer.from(bytes);
}

export function generateTotpSecret(): string {
  return base32Encode(randomBytes(20));
}

/** A *deterministic* secret derived from `seed` — used only for
 * `UsersStore`'s fixture accounts, so a fresh gateway process (and this
 * repo's own smoke-testing scripts, which need to compute a valid code
 * externally) always land on the same secret for e.g. "owner" rather than
 * a new random one every restart. Never use this for a real account. */
export function deterministicTotpSecretForFixture(seed: string): string {
  return base32Encode(createHash("sha1").update(seed).digest());
}

/** RFC 4226 §5.3: HMAC-SHA1 over an 8-byte big-endian counter, dynamically truncated. */
export function hotp(secret: Buffer, counter: number, digits = 6): string {
  const counterBuffer = Buffer.alloc(8);
  counterBuffer.writeUInt32BE(Math.floor(counter / 2 ** 32), 0);
  counterBuffer.writeUInt32BE(counter >>> 0, 4);

  const hmac = createHmac("sha1", secret).update(counterBuffer).digest();
  const offset = hmac[hmac.length - 1] & 0x0f;
  const binary =
    ((hmac[offset] & 0x7f) << 24) |
    ((hmac[offset + 1] & 0xff) << 16) |
    ((hmac[offset + 2] & 0xff) << 8) |
    (hmac[offset + 3] & 0xff);
  const code = binary % 10 ** digits;
  return code.toString().padStart(digits, "0");
}

export interface TotpOptions {
  periodSeconds?: number;
  digits?: number;
  /** Number of ±periods of clock skew to tolerate on verification. */
  window?: number;
}

function counterForTime(unixSeconds: number, periodSeconds: number): number {
  return Math.floor(unixSeconds / periodSeconds);
}

export function generateTotp(base32Secret: string, options: TotpOptions = {}, unixSeconds = Date.now() / 1000): string {
  const { periodSeconds = 30, digits = 6 } = options;
  return hotp(base32Decode(base32Secret), counterForTime(unixSeconds, periodSeconds), digits);
}

export function verifyTotp(
  base32Secret: string,
  token: string,
  options: TotpOptions = {},
  unixSeconds = Date.now() / 1000,
): boolean {
  const { periodSeconds = 30, digits = 6, window = 1 } = options;
  const secretBytes = base32Decode(base32Secret);
  const currentCounter = counterForTime(unixSeconds, periodSeconds);
  for (let delta = -window; delta <= window; delta++) {
    if (hotp(secretBytes, currentCounter + delta, digits) === token) return true;
  }
  return false;
}
