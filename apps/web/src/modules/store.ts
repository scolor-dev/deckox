import { computed, ref } from "vue";
import { api, type ModuleList } from "../api/client";
import { availabilityOf, requirementsMet } from "./availability";
import { widgetById } from "./registry";

const manifest = ref<ModuleList | null>(null);

const availability = computed(() => availabilityOf(manifest.value));

/** Whether the Agent answered when the manifest was fetched. */
export const agentReachable = computed(() => manifest.value?.agent.connected ?? true);

export type AgentState = "unknown" | "ready" | "unreachable" | "incompatible";

/**
 * How the Server sees the Agent: not answering, or answering with a protocol
 * this Server does not speak (after only one of the two was updated).
 */
export const agentState = computed<AgentState>(() => {
  const link = manifest.value?.agent;
  if (!link) return "unknown";
  if (!link.connected) return "unreachable";
  return link.compatible ? "ready" : "incompatible";
});

export const agentLink = computed(() => ({
  agentVersion: manifest.value?.agent.agent_version ?? null,
  agentProtocol: manifest.value?.agent.protocol_version ?? null,
  serverProtocol: manifest.value?.protocol_version ?? null,
}));

/** Read inside a `computed` or a template so it follows the manifest. */
export function backendEnabled(requires: readonly string[]) {
  return requirementsMet(requires, availability.value);
}

export function widgetAvailable(widgetId: string) {
  const definition = widgetById(widgetId);
  return definition !== undefined && backendEnabled(definition.requires);
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
