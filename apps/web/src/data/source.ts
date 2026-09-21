import { onActivated, onMounted, ref, shallowRef } from "vue";

/**
 * A piece of data several widgets show, fetched once for all of them. Widgets
 * that need it call `use()`; the first asks the Server, the rest share the
 * answer, and a widget that appears later refetches only when it is stale.
 */
export function createSource<T>(fetcher: () => Promise<T>, staleMs: number) {
  const data = shallowRef<T | null>(null);
  const loading = ref(false);
  const error = ref<unknown>(null);
  let loadedAt: number | null = null;
  let inFlight: Promise<void> | null = null;

  function refresh(): Promise<void> {
    inFlight ??= (async () => {
      loading.value = true;
      error.value = null;
      try {
        data.value = await fetcher();
        loadedAt = Date.now();
      } catch (cause) {
        error.value = cause;
      } finally {
        loading.value = false;
        inFlight = null;
      }
    })();
    return inFlight;
  }

  function ensureFresh() {
    if (inFlight) return;
    if (loadedAt === null || Date.now() - loadedAt >= staleMs) void refresh();
  }

  /** Call from a widget's `setup`. */
  function use() {
    onMounted(ensureFresh);
    onActivated(ensureFresh);
    return { data, loading, error, refresh };
  }

  /** Forgets the answer, as at sign-out. */
  function reset() {
    data.value = null;
    error.value = null;
    loadedAt = null;
  }

  return { data, loading, error, refresh, use, reset };
}
