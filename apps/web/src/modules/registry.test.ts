import { describe, expect, it } from "vitest";
import type { ModuleInfo, ModuleList } from "../api/client";
import {
  availabilityOf,
  availableModules,
  isAvailable,
  isPartAvailable,
  staleMsOf,
  WEB_MODULES,
} from "./registry";

const BACKEND_MODULES = new Set([
  "system", "power", "update", "storage", "diagnostics", "backups", "services", "schedules",
  "software", "audit", "realtime", "notifications", "update-check",
]);

const info = (id: string, enabled: boolean): ModuleInfo => ({ id, requires: [], enabled, routes: [] });

function manifest(agent: ModuleInfo[], server: ModuleInfo[] = [], connected = true): ModuleList {
  return {
    agent: { connected, agent_version: null, protocol_version: null, compatible: true },
    modules: agent,
    server_modules: server,
  };
}

function offered(list: ModuleList | null) {
  const { enabled, known } = availabilityOf(list);
  return availableModules(enabled, known).map((module) => module.id);
}

describe("web modules", () => {
  it("gives every module a unique id, path and component name", () => {
    for (const key of ["id", "path", "viewName"] as const) {
      const values = WEB_MODULES.map((module) => module[key]);
      expect(new Set(values).size).toBe(values.length);
    }
  });

  it("offers everything until the manifest is known", () => {
    expect(offered(null)).toEqual(WEB_MODULES.map((module) => module.id));
  });

  it("hides a page whose Agent module is switched off", () => {
    const ids = offered(manifest([info("software", false), info("services", true)]));
    expect(ids).not.toContain("software");
    expect(ids).toContain("services");
  });

  it("hides a page whose Server module is switched off", () => {
    expect(offered(manifest([], [info("audit", false)]))).not.toContain("audit");
  });

  it("keeps the core pages whatever is switched off", () => {
    const everythingOff = manifest(
      ["services", "software", "storage", "diagnostics"].map((id) => info(id, false)),
      [info("audit", false)],
    );
    expect(offered(everythingOff)).toEqual(["overview", "settings"]);
  });

  it("does not hide Agent pages while the Agent is unreachable", () => {
    const ids = offered(manifest([], [info("audit", true)], false));
    expect(ids).toContain("software");
    expect(ids).toContain("services");
  });

  it("requires every listed backend module", () => {
    const needsBoth = { ...WEB_MODULES[1], requires: ["a", "b"] };
    const known = new Set(["a", "b"]);
    expect(isAvailable(needsBoth, new Set(["a"]), known)).toBe(false);
    expect(isAvailable(needsBoth, new Set(["a", "b"]), known)).toBe(true);
  });

  it("names only backend modules the Agent or Server actually define", () => {
    for (const module of WEB_MODULES) {
      const named = [...module.requires, ...Object.values(module.parts ?? {}).flat()];
      for (const id of named) expect(BACKEND_MODULES.has(id), `${module.id} requires ${id}`).toBe(true);
    }
  });

  it("hides a part of a page without hiding the page", () => {
    const overview = WEB_MODULES.find((module) => module.id === "overview");
    if (!overview) throw new Error("overview is not registered");
    const { enabled, known } = availabilityOf(manifest([info("system", false), info("storage", true)]));
    expect(isPartAvailable(overview, "system", enabled, known)).toBe(false);
    expect(isPartAvailable(overview, "storage", enabled, known)).toBe(true);
    expect(isPartAvailable(overview, "live", enabled, known)).toBe(false);
    expect(offered(manifest([info("system", false)]))).toContain("overview");
  });

  it("needs every module a part lists, and keeps parts nobody declared", () => {
    const settings = WEB_MODULES.find((module) => module.id === "settings");
    if (!settings) throw new Error("settings is not registered");
    const noPower = availabilityOf(manifest([info("power", false), info("system", true)]));
    expect(isPartAvailable(settings, "reboot", noPower.enabled, noPower.known)).toBe(false);
    const noSystem = availabilityOf(manifest([info("power", true), info("system", false)]));
    expect(isPartAvailable(settings, "reboot", noSystem.enabled, noSystem.known)).toBe(true);
    expect(isPartAvailable(settings, "undeclared", noPower.enabled, noPower.known)).toBe(true);
    const halfLive = availabilityOf(manifest([info("system", false)], [info("realtime", true)]));
    expect(isPartAvailable(settings, "live", halfLive.enabled, halfLive.known)).toBe(false);
    const unreachable = availabilityOf(manifest([], [], false));
    expect(isPartAvailable(settings, "reboot", unreachable.enabled, unreachable.known)).toBe(true);
  });

  it("keeps rarely changing data longer than fast changing data", () => {
    expect(staleMsOf("software")).toBeGreaterThan(staleMsOf("services"));
  });

  it("does not keep a page alive when it holds secrets", () => {
    expect(WEB_MODULES.find((module) => module.id === "settings")?.staleMs).toBeNull();
  });
});
