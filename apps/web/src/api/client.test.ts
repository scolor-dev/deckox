import { afterEach, describe, expect, it, vi } from "vitest";
import {
  api,
  appendMetricHistory,
  DIAGNOSTICS_REPORT_FILENAME,
  usagePercentage,
} from "./client";

afterEach(() => {
  vi.unstubAllGlobals();
});

describe("service APIs", () => {
  it("encodes the service id and accepted log filters", async () => {
    const fetchMock = vi.fn<typeof fetch>().mockResolvedValue(new Response(JSON.stringify({
      service_id: "sample/name.service",
      entries: [{
        timestamp_ms: 1_700_000_000_000,
        priority: 4,
        message: "<script>must remain text</script>",
        process: null,
        pid: null,
      }],
    }), { status: 200, headers: { "Content-Type": "application/json" } }));
    vi.stubGlobal("fetch", fetchMock);

    const result = await api.serviceLogs("sample/name.service", 200, "warning");

    expect(fetchMock).toHaveBeenCalledWith(
      "/api/v1/services/sample%2Fname.service/logs?lines=200&priority=warning",
      expect.objectContaining({ credentials: "same-origin" }),
    );
    expect(result.entries[0]?.message).toBe("<script>must remain text</script>");
  });

  it("supports service enable and disable actions", async () => {
    const fetchMock = vi.fn<typeof fetch>().mockResolvedValue(new Response(JSON.stringify({
      command_id: "command-1",
      status: "completed",
      message: null,
    }), { status: 200, headers: { "Content-Type": "application/json" } }));
    vi.stubGlobal("fetch", fetchMock);

    await api.serviceAction("nginx.service", "enable");
    await api.serviceAction("nginx.service", "disable");

    expect(fetchMock).toHaveBeenNthCalledWith(
      1,
      "/api/v1/services/nginx.service/enable",
      expect.objectContaining({ method: "POST" }),
    );
    expect(fetchMock).toHaveBeenNthCalledWith(
      2,
      "/api/v1/services/nginx.service/disable",
      expect.objectContaining({ method: "POST" }),
    );
  });
});

describe("extended metric helpers", () => {
  it("keeps only the latest 120 samples", () => {
    const initial = Array.from({ length: 120 }, (_, index) => index);
    const history = appendMetricHistory(initial, 120);

    expect(history).toHaveLength(120);
    expect(history[0]).toBe(1);
    expect(history[119]).toBe(120);
  });

  it("treats zero totals and invalid samples as unavailable", () => {
    expect(usagePercentage(5, 0)).toBeNull();
    expect(usagePercentage(80, 100)).toBe(80);
    expect(appendMetricHistory([1], Number.NaN)).toEqual([1]);
  });
});

describe("diagnostics APIs", () => {
  it("fetches partial diagnostics and the authenticated JSON report", async () => {
    const partialReport = {
      generated_at_ms: 1_700_000_000_000,
      server: { version: "0.3.7", status: "degraded" },
      agent: { connected: false, version: null, error_code: "agent_unavailable" },
      host: null,
      deckox_services: null,
      runtime_config: null,
    };
    const fetchMock = vi.fn<typeof fetch>()
      .mockResolvedValueOnce(new Response(JSON.stringify(partialReport), {
        status: 200,
        headers: { "Content-Type": "application/json" },
      }))
      .mockResolvedValueOnce(new Response(JSON.stringify(partialReport), {
        status: 200,
        headers: { "Content-Type": "application/json" },
      }));
    vi.stubGlobal("fetch", fetchMock);

    await expect(api.diagnostics()).resolves.toEqual(partialReport);
    const blob = await api.diagnosticsReport();

    expect(blob.type).toBe("application/json");
    expect(fetchMock).toHaveBeenNthCalledWith(
      2,
      "/api/v1/diagnostics/report",
      expect.objectContaining({ credentials: "same-origin" }),
    );
    expect(DIAGNOSTICS_REPORT_FILENAME).toBe("deckox-diagnostics.json");
  });
});
