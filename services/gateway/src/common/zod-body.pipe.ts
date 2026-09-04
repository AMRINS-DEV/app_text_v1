import { ArgumentMetadata, BadRequestException, PipeTransform } from "@nestjs/common";
import type { ZodType } from "zod";

/** §11.1's "zod-validated DTOs," as a reusable pipe: `@Body(new ZodBody(schema))`. */
export class ZodBody<T> implements PipeTransform<unknown, T> {
  constructor(private readonly schema: ZodType<T>) {}

  transform(value: unknown, _metadata: ArgumentMetadata): T {
    const result = this.schema.safeParse(value);
    if (!result.success) {
      throw new BadRequestException(result.error.issues.map((issue) => `${issue.path.join(".")}: ${issue.message}`));
    }
    return result.data;
  }
}
