// @vitest-environment jsdom
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { ApiError, api, observeJobs, waitForJob, type Job } from "./client";

function job(state: Job["state"], extra: Partial<Job> = {}): Job {
  return {
    id: "job-1", kind: "software_action", subject: "git", state, created_ms: 1,
    started_ms: null, finished_ms: null, message: null, error_code: null, request_id: null, ...extra,
  };
}

function reply(status: number, body: unknown) {
  return new Response(JSON.stringify(body), { status, headers: { "Content-Type": "application/json" } });
}

const fetchMock = vi.fn<typeof fetch>();
const noPause = () => Promise.resolve();

beforeEach(() => {
  vi.stubGlobal("fetch", fetchMock);
  fetchMock.mockReset();
  observeJobs(null);
});
afterEach(() => {
  vi.unstubAllGlobals();
});

describe("waitForJob", () => {
  it("polls until the job succeeds and tells the observer about each step", async () => {
    const seen: string[] = [];
    observeJobs((entry) => seen.push(entry.state));
    fetchMock
      .mockResolvedValueOnce(reply(200, job("running")))
      .mockResolvedValueOnce(reply(200, job("succeeded", { message: "ok" })));
    const done = await waitForJob(job("queued"), noPause);
    expect(done.state).toBe("succeeded");
    expect(seen).toEqual(["queued", "running", "succeeded"]);
  });

  it("throws the error a failed job carries, with its code", async () => {
    fetchMock.mockResolvedValueOnce(reply(200, job("failed", { message: "package is locked", error_code: "conflict" })));
    await expect(waitForJob(job("running"), noPause)).rejects.toMatchObject({
      message: "package is locked",
      code: "conflict",
    });
  });

  it("reports a job the Agent no longer knows as lost", async () => {
    fetchMock.mockResolvedValueOnce(reply(404, { code: "not_found", message: "job not found" }));
    await expect(waitForJob(job("running"), noPause)).rejects.toMatchObject({ code: "job_lost" });
  });

  it("rides out a few failed reads but gives up after several in a row", async () => {
    fetchMock
      .mockRejectedValueOnce(new Error("network"))
      .mockRejectedValueOnce(new Error("network"))
      .mockResolvedValueOnce(reply(200, job("succeeded")));
    await expect(waitForJob(job("running"), noPause)).resolves.toMatchObject({ state: "succeeded" });

    fetchMock.mockReset();
    fetchMock.mockRejectedValue(new Error("network"));
    await expect(waitForJob(job("running"), noPause)).rejects.toThrow("network");
    expect(fetchMock).toHaveBeenCalledTimes(5);
  });

  it("stops at once when the session has ended", async () => {
    fetchMock.mockResolvedValueOnce(reply(401, { code: "authentication_required", message: "no" }));
    await expect(waitForJob(job("running"), noPause)).rejects.toBeInstanceOf(ApiError);
    expect(fetchMock).toHaveBeenCalledTimes(1);
  });
});

describe("actions run as jobs", () => {
  it("starts a software action as a job and waits for it", async () => {
    fetchMock
      .mockResolvedValueOnce(reply(202, job("queued")))
      .mockResolvedValueOnce(reply(200, job("succeeded")));
    vi.useFakeTimers();
    const pending = api.softwareAction("git", "install", "pw");
    await vi.runAllTimersAsync();
    await pending;
    vi.useRealTimers();
    const [url, init] = fetchMock.mock.calls[0];
    expect(url).toBe("/api/v1/software/git/install?async=true");
    expect(JSON.parse(init?.body as string)).toEqual({ current_password: "pw" });
  });

  it("does not wait when an older backend answers with the finished result", async () => {
    fetchMock.mockResolvedValueOnce(reply(200, { command_id: "c", status: "completed", message: null }));
    await api.serviceAction("nginx.service", "restart");
    expect(fetchMock).toHaveBeenCalledTimes(1);
  });

  it("keeps allowlist changes in the foreground", async () => {
    fetchMock.mockResolvedValueOnce(reply(200, { command_id: "c", status: "completed", message: null }));
    await api.serviceAction("nginx.service", "allow");
    expect(fetchMock.mock.calls[0][0]).toBe("/api/v1/services/nginx.service/allow");
  });
});
