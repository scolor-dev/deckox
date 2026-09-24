import { ref } from "vue";
import { api } from "../api/client";
import { notify } from "../notifications";
import { ensureLockedWidgets, recommendedLayout } from "./defaults";
import { normalizeLayout } from "./model";
import type { Layout } from "../widgets/types";

/**
 * Where the layout lives. The Server keeps one copy so every device starts
 * from the same screens; each browser can keep its own, and that one wins
 * where it exists. `revision` is the Server's count of saves that this
 * browser's copy was last in step with, which is how a browser notices the
 * Server has something newer.
 */
const LOCAL_KEY = "deckox:layout";

export type LayoutSource = "recommended" | "browser" | "server";

export const layout = ref<Layout>(recommendedLayout());
export const layoutSource = ref<LayoutSource>("recommended");
/** A layout the Server holds that is newer than this browser's own copy. */
export const newerServerLayout = ref<{ revision: number; layout: Layout } | null>(null);
export const layoutSaving = ref(false);

let baseRevision = 0;

interface LocalCopy {
  revision: number;
  layout: unknown;
}

function readLocal(): { revision: number; layout: Layout } | null {
  try {
    const raw = localStorage.getItem(LOCAL_KEY);
    if (raw === null) return null;
    const copy = JSON.parse(raw) as Partial<LocalCopy>;
    const parsed = normalizeLayout(copy.layout);
    return parsed ? { revision: typeof copy.revision === "number" ? copy.revision : 0, layout: parsed } : null;
  } catch {
    return null;
  }
}

function writeLocal(revision: number, value: Layout) {
  try {
    localStorage.setItem(LOCAL_KEY, JSON.stringify({ revision, layout: value } satisfies LocalCopy));
  } catch {
    // Private mode or full storage: the Server copy still holds the layout.
  }
}

function clearLocal() {
  try {
    localStorage.removeItem(LOCAL_KEY);
  } catch {
    // Nothing to clear.
  }
}

/** Reads the browser's copy and the Server's, and picks what to show. */
export async function loadLayout() {
  const local = readLocal();
  let serverRevision = 0;
  let server: Layout | null = null;
  try {
    const stored = await api.layout();
    serverRevision = stored.revision;
    const parsed = stored.layout === null ? null : normalizeLayout(stored.layout);
    server = parsed === null ? null : ensureLockedWidgets(parsed);
  } catch {
    // Offline or not signed in: the browser's copy or the recommended one is used.
  }

  newerServerLayout.value = null;
  if (local) {
    layout.value = ensureLockedWidgets(local.layout);
    layoutSource.value = "browser";
    baseRevision = local.revision;
    if (server && serverRevision > local.revision) {
      newerServerLayout.value = { revision: serverRevision, layout: server };
    }
  } else if (server) {
    layout.value = server;
    layoutSource.value = "server";
    baseRevision = serverRevision;
  } else {
    layout.value = recommendedLayout();
    layoutSource.value = "recommended";
    baseRevision = serverRevision;
  }
}

/**
 * Keeps `next` in this browser at once and on the Server as soon as it can.
 * Returns whether the Server has it; a browser that cannot reach the Server
 * still keeps what was saved.
 */
export async function saveLayout(next: Layout, t: (key: string) => string): Promise<boolean> {
  layout.value = next;
  layoutSource.value = "browser";
  writeLocal(baseRevision, next);
  layoutSaving.value = true;
  try {
    const stored = await api.saveLayout(next);
    baseRevision = stored.revision;
    writeLocal(baseRevision, next);
    newerServerLayout.value = null;
    return true;
  } catch {
    notify("warning", t("layout.serverSaveFailed"));
    return false;
  } finally {
    layoutSaving.value = false;
  }
}

/** Drops this browser's copy in favour of the newer one on the Server. */
export function useServerLayout() {
  const newer = newerServerLayout.value;
  if (!newer) return;
  clearLocal();
  layout.value = newer.layout;
  layoutSource.value = "server";
  baseRevision = newer.revision;
  newerServerLayout.value = null;
}

export function resetLayout(t: (key: string) => string) {
  return saveLayout(recommendedLayout(), t);
}

export function forgetLayout() {
  layout.value = recommendedLayout();
  layoutSource.value = "recommended";
  newerServerLayout.value = null;
  baseRevision = 0;
}
