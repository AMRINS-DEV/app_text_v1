import { Controller, Post, NotImplementedException } from "@nestjs/common";

@Controller("api/auth")
export class AuthController {
  @Post("login")
  login(): never {
    // Phase 4: JWT+refresh issuance, TOTP verification.
    throw new NotImplementedException("auth.login is Phase 4 scope");
  }
}
