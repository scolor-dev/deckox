import type { Component } from "vue";
import type { ModuleInfo, ModuleList } from "../api/client";

/**
 * A feature of the Web, declared the same way the Agent and the Server declare
 * theirs: an id, what it needs, and what it contributes. The shell builds the
 * router, the navigation and the page cache from this list, so adding a feature
 * means adding one entry here.
 *
 * `id` matches the backend module it shows wherever there is one (`services`,
 * `software`, `storage`, `diagnostics`, `audit`). `overview` and `settings`
 * are core: they need nothing and are always offered.
 */
export interface WebModule {
  id: string;
  path: string;
  /** i18n key for the navigation label and the page title. */
  titleKey: string;
  load: () => Promise<{ default: Component }>;
  /** Component name, which `<KeepAlive include>` matches. */
  viewName: string;
  /** Backend module ids (Agent or Server) that must be enabled. */
  requires: readonly string[];
  /**
   * Panels, tabs and sections inside the page that need more than the page
   * does, each with the backend modules it needs. A part whose modules are off
   * is left out and never calls their endpoints.
   */
  parts?: Readonly<Record<string, readonly string[]>>;
  /**
   * Keep the page alive when leaving it and reload only after this many
   * milliseconds. `null` drops the page on leave, for screens holding secrets.
   */
  staleMs: number | null;
}

const MINUTE = 60_000;

export const WEB_MODULES: readonly WebModule[] = [
  {
    id: "overview",
    path: "/",
    titleKey: "nav.overview",
    load: () => import("../views/OverviewView.vue"),
    viewName: "OverviewView",
    requires: [],
    parts: {
      system: ["system"],
      storage: ["storage"],
      live: ["realtime", "system"],
    },
    staleMs: 5 * MINUTE,
  },
  {
    id: "services",
    path: "/services",
    titleKey: "nav.services",
    load: () => import("../views/ServicesView.vue"),
    viewName: "ServicesView",
    requires: ["services"],
    parts: { schedules: ["schedules"] },
    staleMs: MINUTE / 2,
  },
  {
    id: "software",
    path: "/software",
    titleKey: "nav.software",
    load: () => import("../views/SoftwareView.vue"),
    viewName: "SoftwareView",
    requires: ["software"],
    staleMs: 10 * MINUTE,
  },
  {
    id: "storage",
    path: "/storage",
    titleKey: "nav.storage",
    load: () => import("../views/StorageView.vue"),
    viewName: "StorageView",
    requires: ["storage"],
    staleMs: MINUTE,
  },
  {
    id: "diagnostics",
    path: "/diagnostics",
    titleKey: "nav.diagnostics",
    load: () => import("../views/DiagnosticsView.vue"),
    viewName: "DiagnosticsView",
    requires: ["diagnostics"],
    parts: { backups: ["backups"] },
    staleMs: MINUTE,
  },
  {
    id: "audit",
    path: "/audit",
    titleKey: "nav.audit",
    load: () => import("../views/AuditView.vue"),
    viewName: "AuditView",
    requires: ["audit"],
    staleMs: MINUTE / 2,
  },
  {
    id: "settings",
    path: "/settings",
    titleKey: "nav.settings",
    load: () => import("../views/SettingsView.vue"),
    viewName: "SettingsView",
    requires: [],
    parts: {
      webhook: ["notifications"],
      "update-check": ["update-check"],
      "update-now": ["update", "system"],
      reboot: ["power", "system"],
      live: ["realtime", "system"],
    },
    staleMs: null,
  },
];

/**
 * Which backend modules are switched on. `null` means the answer is not known
 * yet, which offers everything rather than hiding pages behind a slow request.
 */
export type ModuleAvailability = ReadonlySet<string> | null;

function enabledIds(modules: readonly ModuleInfo[]): string[] {
  return modules.filter((module) => module.enabled).map((module) => module.id);
}

/**
 * Turns the manifest into the set of enabled backend module ids. While the
 * Agent is unreachable its list is empty; that is "unknown", not "all off", so
 * only the Server's own modules are then enforced.
 */
export function availabilityOf(list: ModuleList | null): {
  enabled: ModuleAvailability;
  known: ReadonlySet<string>;
} {
  if (!list) return { enabled: null, known: new Set() };
  const all = [...list.modules, ...list.server_modules];
  return {
    enabled: new Set(enabledIds(all)),
    known: new Set(all.map((module) => module.id)),
  };
}

/**
 * A module is offered when everything it requires is enabled. A requirement the
 * manifest does not mention at all is treated as enabled: it means the list is
 * incomplete (the Agent is down), not that the feature is off.
 */
export function isAvailable(
  module: WebModule,
  enabled: ModuleAvailability,
  known: ReadonlySet<string>,
): boolean {
  return requirementsMet(module.requires, enabled, known);
}

function requirementsMet(
  requires: readonly string[],
  enabled: ModuleAvailability,
  known: ReadonlySet<string>,
) {
  if (enabled === null) return true;
  return requires.every((id) => !known.has(id) || enabled.has(id));
}

/** Whether a part of a page (see `WebModule.parts`) should be shown. */
export function isPartAvailable(
  module: WebModule,
  part: string,
  enabled: ModuleAvailability,
  known: ReadonlySet<string>,
): boolean {
  const requires = module.parts?.[part];
  return requires === undefined ? true : requirementsMet(requires, enabled, known);
}

export function availableModules(
  enabled: ModuleAvailability,
  known: ReadonlySet<string>,
  modules: readonly WebModule[] = WEB_MODULES,
): WebModule[] {
  return modules.filter((module) => isAvailable(module, enabled, known));
}

export function staleMsOf(id: string, modules: readonly WebModule[] = WEB_MODULES) {
  return modules.find((module) => module.id === id)?.staleMs ?? MINUTE;
}

export function moduleForPath(path: string, modules: readonly WebModule[] = WEB_MODULES) {
  return modules.find((module) => module.path === path);
}
