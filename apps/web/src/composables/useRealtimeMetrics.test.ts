import { describe, expect, it } from "vitest";
import { parseMetricsEvent } from "./useRealtimeMetrics";

describe("parseMetricsEvent", () => {
  it("accepts the SSE payload contract", () => {
    expect(parseMetricsEvent(JSON.stringify({
      sequence: 12,
      timestamp_ms: 1_700_000_000_000,
      agent_online: false,
      metrics: null,
      error_code: "agent_unavailable",
    }))).toEqual({
      sequence: 12,
      timestamp_ms: 1_700_000_000_000,
      agent_online: false,
      metrics: null,
      error_code: "agent_unavailable",
    });
  });

  it("rejects malformed and incomplete events", () => {
    expect(parseMetricsEvent("not-json")).toBeNull();
    expect(parseMetricsEvent(JSON.stringify({ sequence: 1 }))).toBeNull();
  });
});
