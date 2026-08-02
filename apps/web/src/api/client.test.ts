import { afterEach, describe, expect, it, vi } from "vitest";
import {
  api,
  appendMetricHistory,
  buildUpdateCommand,
  DIAGNOSTICS_REPORT_FILENAME,
  safeReleaseUrl,
  usagePercentage,
  writeClipboardText,
} from "./client";

afterEach(() => {
  vi.unstubAllGlobals();
});

describe("update APIs", () => {
  it("fetches update status", async () => {
    const status = {
      status: "available",
      current_version: "0.3.8",
      latest_version: "v0.3.9",
      update_available: true,
      release_url: "https://github.com/scolor-dev/deckox/releases/tag/v0.3.9",
      checked_at_ms: 1_700_000_000_000,
    };
    const fetchMock = vi.fn<typeof fetch>().mockResolvedValue(new Response(JSON.stringify(status), {
      status: 200,
      headers: { "Content-Type": "application/json" },
    }));
    vi.stubGlobal("fetch", fetchMock);

    await expect(api.updateStatus()).resolves.toEqual(status);
    expect(fetchMock).toHaveBeenCalledWith(
      "/api/v1/update",
      expect.objectContaining({ credentials: "same-origin" }),
    );
  });

  it("builds only a fixed safe installer command", () => {
    expect(buildUpdateCommand("0.3.9")).toBe(
      "curl -fsSL https://raw.githubusercontent.com/scolor-dev/deckox/main/packaging/scripts/install.sh | sudo DECKOX_VERSION=v0.3.9 sh",
    );
    expect(buildUpdateCommand("v0.3.9; reboot")).toBeNull();
  });

  it("accepts only HTTPS GitHub release links", () => {
    expect(safeReleaseUrl("https://github.com/scolor-dev/deckox/releases/tag/v0.3.9"))
      .toBe("https://github.com/scolor-dev/deckox/releases/tag/v0.3.9");
    expect(safeReleaseUrl("javascript:alert(1)")).toBeNull();
    expect(safeReleaseUrl("https://example.com/release")).toBeNull();
    expect(safeReleaseUrl("https://github.com/other/project/releases/tag/v0.3.9")).toBeNull();
  });

  it("reports clipboard failures without throwing", async () => {
    const clipboard = { writeText: vi.fn().mockRejectedValue(new Error("denied")) };
    await expect(writeClipboardText("command", clipboard)).resolves.toBe(false);
    expect(clipboard.writeText).toHaveBeenCalledWith("command");
  });
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
