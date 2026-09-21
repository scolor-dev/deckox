import { describe, expect, it } from "vitest";
import type { ModuleList } from "../api/client";
import { recommendedLayout } from "../layout/defaults";
import { messages } from "../i18n";
import { availabilityOf, requirementsMet } from "./availability";
import { WEB_MODULES, WIDGETS, widgetById } from "./registry";

const BACKEND_MODULES = new Set([
  "system", "power", "update", "storage", "diagnostics", "backups", "services", "schedules",
  "software", "audit", "realtime", "notifications", "update-check",
]);

function manifest(agent: string[], server: string[] = [], connected = true, off: string[] = []): ModuleList {
  const info = (id: string) => ({ id, requires: [], enabled: !off.includes(id), routes: [] });
  return {
    agent: { connected, agent_version: null, protocol_version: null, compatible: true },
    modules: agent.map(info),
    server_modules: server.map(info),
  };
}

function lookup(tree: Record<string, unknown>, path: string): unknown {
  return path.split(".").reduce<unknown>((node, key) => (
    typeof node === "object" && node !== null ? (node as Record<string, unknown>)[key] : undefined
  ), tree);
}

describe("web modules", () => {
  it("are found from their folders, core first, with unique ids", () => {
    expect(WEB_MODULES[0].id).toBe("core");
    expect(new Set(WEB_MODULES.map((module) => module.id)).size).toBe(WEB_MODULES.length);
    expect(new Set(WIDGETS.map((widget) => widget.id)).size).toBe(WIDGETS.length);
  });

  it("name their widgets after their own id", () => {
    for (const module of WEB_MODULES) {
      for (const widget of module.widgets) expect(widget.id.startsWith(`${module.id}.`)).toBe(true);
    }
  });

  it("require only modules the Agent or Server define", () => {
    for (const widget of WIDGETS) {
      for (const id of widget.requires) expect(BACKEND_MODULES.has(id), `${widget.id} requires ${id}`).toBe(true);
    }
  });

  it("have a title and a description in both languages for every widget", () => {
    for (const module of WEB_MODULES) {
      for (const language of ["ja", "en"] as const) {
        const tree = { ...messages[language], ...(module.messages?.[language] ?? {}) } as Record<string, unknown>;
        expect(typeof lookup(tree, module.titleKey), `${module.id} ${language}`).toBe("string");
        for (const widget of module.widgets) {
          for (const key of [widget.titleKey, widget.descriptionKey]) {
            expect(typeof lookup(tree, key), `${key} (${language})`).toBe("string");
          }
        }
      }
    }
  });

  it("keep default sizes inside their own limits", () => {
    for (const { id, size } of WIDGETS) {
      const { default: initial, min, max } = size;
      expect(initial.w, id).toBeGreaterThanOrEqual(min.w);
      expect(initial.w, id).toBeLessThanOrEqual(max?.w ?? 12);
      if (typeof initial.h === "number" && typeof min.h === "number") {
        expect(initial.h, id).toBeGreaterThanOrEqual(min.h);
        expect(initial.h, id).toBeLessThanOrEqual(max?.h ?? 24);
      }
    }
  });
});

describe("recommended layout", () => {
  it("uses only widgets that exist, at sizes they allow", () => {
    for (const page of recommendedLayout().pages) {
      for (const placement of page.widgets) {
        const definition = widgetById(placement.widget);
        expect(definition, placement.widget).toBeDefined();
        expect(placement.w).toBeGreaterThanOrEqual(definition?.size.min.w ?? 1);
      }
    }
  });

  it("has a name for every page", () => {
    for (const page of recommendedLayout().pages) {
      const key = page.titleKey ?? "";
      expect(typeof lookup(messages.ja as Record<string, unknown>, key), key).toBe("string");
      expect(typeof lookup(messages.en as Record<string, unknown>, key), key).toBe("string");
    }
  });

  it("offers a widget of every module", () => {
    const used = new Set(recommendedLayout().pages.flatMap((page) => page.widgets.map((widget) => widget.widget.split(".")[0])));
    for (const module of WEB_MODULES.filter((entry) => entry.id !== "core")) {
      expect(used.has(module.id), module.id).toBe(true);
    }
  });
});

describe("backend availability", () => {
  it("offers everything until the manifest is known", () => {
    expect(requirementsMet(["software"], availabilityOf(null))).toBe(true);
  });

  it("follows the Agent and the Server modules", () => {
    const list = availabilityOf(manifest(["software", "storage"], ["audit"], true, ["software", "audit"]));
    expect(requirementsMet(["software"], list)).toBe(false);
    expect(requirementsMet(["storage"], list)).toBe(true);
    expect(requirementsMet(["audit"], list)).toBe(false);
    expect(requirementsMet([], list)).toBe(true);
  });

  it("treats a module the manifest does not mention as on (the Agent is unreachable)", () => {
    const list = availabilityOf(manifest([], ["audit"], false));
    expect(requirementsMet(["software"], list)).toBe(true);
    expect(requirementsMet(["audit"], list)).toBe(true);
  });

  it("needs every module a widget lists", () => {
    const list = availabilityOf(manifest(["services", "schedules"], [], true, ["schedules"]));
    expect(requirementsMet(["services", "schedules"], list)).toBe(false);
  });
});
