import { describe, expect, it } from "vitest";
import type { ModuleInfo, ModuleList } from "../api/client";
import {
  availabilityOf,
  availableModules,
  isAvailable,
  staleMsOf,
  WEB_MODULES,
} from "./registry";

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
    const backend = new Set([
      "system", "power", "update", "storage", "diagnostics", "backups", "services", "schedules",
      "software", "audit", "realtime", "notifications", "update-check",
    ]);
    for (const module of WEB_MODULES) {
      for (const id of module.requires) expect(backend.has(id), `${module.id} requires ${id}`).toBe(true);
    }
  });

  it("keeps rarely changing data longer than fast changing data", () => {
    expect(staleMsOf("software")).toBeGreaterThan(staleMsOf("services"));
  });

  it("does not keep a page alive when it holds secrets", () => {
    expect(WEB_MODULES.find((module) => module.id === "settings")?.staleMs).toBeNull();
  });
});
