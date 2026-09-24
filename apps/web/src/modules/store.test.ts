// @vitest-environment jsdom
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { ModuleList } from "../api/client";

const api = vi.hoisted(() => ({ modules: vi.fn() }));
vi.mock("../api/client", () => ({ api }));

import { agentLink, agentState, loadModules, resetModules } from "./store";

function list(agent: Partial<ModuleList["agent"]>): ModuleList {
  return {
    protocol_version: 2,
    agent: { connected: true, agent_version: "0.7.0", protocol_version: 2, compatible: true, ...agent },
    modules: [],
    server_modules: [],
  };
}

beforeEach(() => {
  api.modules.mockReset();
  resetModules();
});

describe("how the Server sees the Agent", () => {
  it("is unknown until the manifest has been read", () => {
    expect(agentState.value).toBe("unknown");
  });

  it("is ready when the Agent answers with the same protocol", async () => {
    api.modules.mockResolvedValue(list({}));
    await loadModules();
    expect(agentState.value).toBe("ready");
  });

  it("is unreachable when the Agent does not answer", async () => {
    api.modules.mockResolvedValue(list({ connected: false, agent_version: null, protocol_version: null, compatible: true }));
    await loadModules();
    expect(agentState.value).toBe("unreachable");
  });

  it("is incompatible when only one side was updated, and says which versions", async () => {
    api.modules.mockResolvedValue(list({ protocol_version: 1, agent_version: "0.6.8", compatible: false }));
    await loadModules();
    expect(agentState.value).toBe("incompatible");
    expect(agentLink.value).toEqual({ agentVersion: "0.6.8", agentProtocol: 1, serverProtocol: 2 });
  });

  it("shows an Agent from before the protocol was numbered as old", async () => {
    api.modules.mockResolvedValue(list({ protocol_version: 0, agent_version: "unknown", compatible: false }));
    await loadModules();
    expect(agentState.value).toBe("incompatible");
    expect(agentLink.value).toEqual({ agentVersion: null, agentProtocol: null, serverProtocol: 2 });
  });

  it("keeps the last answer when a later read fails", async () => {
    api.modules.mockResolvedValueOnce(list({ compatible: false }));
    await loadModules();
    api.modules.mockRejectedValueOnce(new Error("offline"));
    await loadModules();
    expect(agentState.value).toBe("incompatible");
  });
});
