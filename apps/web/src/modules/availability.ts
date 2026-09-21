import type { ModuleInfo, ModuleList } from "../api/client";

/**
 * Which backend modules are switched on. `null` means the answer is not known
 * yet, which offers everything rather than hiding widgets behind a slow request.
 */
export type ModuleAvailability = ReadonlySet<string> | null;

export interface Availability {
  enabled: ModuleAvailability;
  /** Every module id the manifest mentioned, on or off. */
  known: ReadonlySet<string>;
}

const enabledIds = (modules: readonly ModuleInfo[]) =>
  modules.filter((module) => module.enabled).map((module) => module.id);

/**
 * Turns the manifest into the set of enabled backend module ids. While the
 * Agent is unreachable its list is empty; that is "unknown", not "all off", so
 * only the Server's own modules are then enforced.
 */
export function availabilityOf(list: ModuleList | null): Availability {
  if (!list) return { enabled: null, known: new Set() };
  const all = [...list.modules, ...list.server_modules];
  return {
    enabled: new Set(enabledIds(all)),
    known: new Set(all.map((module) => module.id)),
  };
}

/**
 * Everything required is on. A requirement the manifest does not mention at
 * all counts as on: it means the list is incomplete (the Agent is down), not
 * that the feature is off.
 */
export function requirementsMet(requires: readonly string[], availability: Availability): boolean {
  if (availability.enabled === null) return true;
  const { enabled, known } = availability;
  return requires.every((id) => !known.has(id) || enabled.has(id));
}
