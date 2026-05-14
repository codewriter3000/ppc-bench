export * from "./bus";
export * from "./contracts";
export * from "./events";

import { KernelBus } from "./bus";
import type { KernelEvents } from "./events";

/** Convenience alias for the application-wide bus type. */
export type PPCBenchBus = KernelBus<KernelEvents>;

export const createBus = (): PPCBenchBus => new KernelBus<KernelEvents>();
