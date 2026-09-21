import { onActivated, onMounted } from "vue";

/**
 * Loads a view's data once when it first appears and again only when it has
 * gone stale, so moving between pages does not re-run every request.
 *
 * Views are kept alive by the shell, which means `onMounted` runs once while
 * `onActivated` runs on every return to the page. The returned function is the
 * view's own reload: call it from a Refresh button or after a change, and it
 * restarts the staleness clock.
 */
export function useStaleRefresh(
  load: () => Promise<unknown>,
  staleMs: number,
  now: () => number = Date.now,
): () => Promise<void> {
  let loadedAt: number | null = null;
  let skipFirstActivation = true;

  async function reload() {
    loadedAt = now();
    await load();
  }

  onMounted(() => {
    void reload();
  });

  onActivated(() => {
    // The first activation is the mount itself, which already loaded.
    if (skipFirstActivation) {
      skipFirstActivation = false;
      return;
    }
    if (loadedAt === null || now() - loadedAt >= staleMs) void reload();
  });

  return reload;
}
