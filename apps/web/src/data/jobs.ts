import { onActivated, onBeforeUnmount, onDeactivated, onMounted, shallowRef } from "vue";
import { api, jobFinished, observeJobs, type Job } from "../api/client";

const MAX_KEPT = 30;
const POLL_MS = 2_000;

/** Recent jobs, newest first: those this browser started and those the Agent still remembers. */
export const recentJobs = shallowRef<Job[]>([]);

function merge(incoming: readonly Job[]) {
  const byId = new Map(recentJobs.value.map((job) => [job.id, job]));
  for (const job of incoming) byId.set(job.id, job);
  recentJobs.value = [...byId.values()]
    .sort((a, b) => b.created_ms - a.created_ms)
    .slice(0, MAX_KEPT);
}

export function trackJob(job: Job) {
  merge([job]);
}

observeJobs(trackJob);

export async function refreshJobs() {
  try {
    merge(await api.jobs());
  } catch {
    // The list is a convenience; a failed read leaves what is already shown.
  }
}

export const hasActiveJobs = () => recentJobs.value.some((job) => !jobFinished(job));

export function resetJobs() {
  recentJobs.value = [];
}

/**
 * Call from a widget that lists jobs. It reads the Agent's list when the
 * widget appears, and keeps reading while something is queued or running.
 */
export function useJobs() {
  let timer: number | null = null;

  function schedule() {
    if (timer !== null) return;
    timer = window.setTimeout(() => {
      timer = null;
      void tick();
    }, POLL_MS);
  }

  async function tick() {
    await refreshJobs();
    if (hasActiveJobs()) schedule();
  }

  function stop() {
    if (timer !== null) window.clearTimeout(timer);
    timer = null;
  }

  onMounted(() => void tick());
  onActivated(() => void tick());
  onDeactivated(stop);
  onBeforeUnmount(stop);

  return { jobs: recentJobs, refresh: tick };
}
