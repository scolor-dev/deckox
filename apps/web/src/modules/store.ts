import { computed, ref } from "vue";
import { api, type ModuleList } from "../api/client";
import { availabilityOf, availableModules } from "./registry";

const manifest = ref<ModuleList | null>(null);

const availability = computed(() => availabilityOf(manifest.value));

/** The modules to offer right now, in navigation order. */
export const enabledModules = computed(() =>
  availableModules(availability.value.enabled, availability.value.known),
);

/** The page names `<KeepAlive>` should hold on to. */
export const cachedViewNames = computed(() =>
  enabledModules.value.filter((module) => module.staleMs !== null).map((module) => module.viewName),
);

/** Whether the Agent answered when the manifest was fetched. */
export const agentReachable = computed(() => manifest.value?.agent.connected ?? true);

export function moduleEnabled(id: string) {
  return enabledModules.value.some((module) => module.id === id);
}

let inFlight: Promise<void> | null = null;

/** Reads the manifest; calls made while a read is under way share it. */
export function loadModules() {
  inFlight ??= api
    .modules()
    .then((list) => {
      manifest.value = list;
    })
    .catch(() => {
      // Keep the last answer, or offer everything when there never was one.
    })
    .finally(() => {
      inFlight = null;
    });
  return inFlight;
}

export function resetModules() {
  manifest.value = null;
}
