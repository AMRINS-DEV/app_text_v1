/** Zustand slices (§4, §12.2 layoutStore). `auth` is real Phase 4 work;
 * per-feature slices beyond it (layout, chart workspace state) remain
 * later scope. */
export { useAuthStore } from "./auth";
export type { Role } from "./auth";
