// @vitest-environment jsdom
import { mount } from "@vue/test-utils";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { ModuleList } from "../api/client";

const api = vi.hoisted(() => ({ modules: vi.fn() }));
vi.mock("../api/client", () => ({ api }));

import { i18n } from "../i18n";
import { loadModules, resetModules } from "../modules/store";
import AgentLinkBanner from "./AgentLinkBanner.vue";

function list(agent: Partial<ModuleList["agent"]>): ModuleList {
  return {
    protocol_version: 2,
    agent: { connected: true, agent_version: "0.7.0", protocol_version: 2, compatible: true, ...agent },
    modules: [],
    server_modules: [],
  };
}

async function textFor(agent: Partial<ModuleList["agent"]>) {
  api.modules.mockResolvedValue(list(agent));
  await loadModules();
  return mount(AgentLinkBanner, { global: { plugins: [i18n] } }).text();
}

beforeEach(() => {
  i18n.global.locale.value = "en";
  api.modules.mockReset();
  resetModules();
});

describe("AgentLinkBanner", () => {
  it("says nothing while the Agent is fine", async () => {
    expect(await textFor({})).toBe("");
  });

  it("says nothing before the state is known", () => {
    expect(mount(AgentLinkBanner, { global: { plugins: [i18n] } }).text()).toBe("");
  });

  it("tells how to check an Agent that does not answer", async () => {
    expect(await textFor({ connected: false })).toContain("systemctl status deckox-agent");
  });

  it("names both versions when they differ", async () => {
    const text = await textFor({ compatible: false, agent_version: "0.7.1", protocol_version: 3 });
    expect(text).toContain("0.7.1");
    expect(text).toContain("protocol 3");
    expect(text).toContain("protocol 2");
  });

  it("explains an Agent that predates the protocol", async () => {
    const text = await textFor({ compatible: false, agent_version: "unknown", protocol_version: 0 });
    expect(text).toContain("predates the protocol");
    expect(text).not.toContain("protocol 0");
  });
});
