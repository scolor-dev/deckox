// @vitest-environment jsdom
import { beforeEach, describe, expect, it, vi } from "vitest";
import { recommendedLayout } from "./defaults";
import type { Layout } from "../widgets/types";

const api = vi.hoisted(() => ({ layout: vi.fn(), saveLayout: vi.fn() }));
vi.mock("../api/client", () => ({ api }));
vi.mock("../notifications", () => ({ notify: vi.fn() }));

import { notify } from "../notifications";
import { layout, layoutSource, loadLayout, newerServerLayout, resetLayout, saveLayout, useServerLayout } from "./store";

const t = (key: string) => key;
const KEY = "deckox:layout";

function custom(title: string): Layout {
  return { version: 1, pages: [{ id: "mine", title, kind: "scroll", rows: 8, widgets: [] }] };
}

beforeEach(() => {
  localStorage.clear();
  api.layout.mockReset();
  api.saveLayout.mockReset();
  vi.mocked(notify).mockReset();
});

describe("layout store", () => {
  it("starts from the recommended layout when nothing is saved", async () => {
    api.layout.mockResolvedValue({ revision: 0, layout: null });
    await loadLayout();
    expect(layoutSource.value).toBe("recommended");
    expect(layout.value).toEqual(recommendedLayout());
  });

  it("uses the Server's layout when this browser has none, without copying it here", async () => {
    api.layout.mockResolvedValue({ revision: 3, layout: custom("server") });
    await loadLayout();
    expect(layoutSource.value).toBe("server");
    expect(layout.value.pages[0].title).toBe("server");
    expect(localStorage.getItem(KEY)).toBeNull();
  });

  it("prefers this browser's own copy over the Server's", async () => {
    localStorage.setItem(KEY, JSON.stringify({ revision: 3, layout: custom("browser") }));
    api.layout.mockResolvedValue({ revision: 3, layout: custom("server") });
    await loadLayout();
    expect(layoutSource.value).toBe("browser");
    expect(layout.value.pages[0].title).toBe("browser");
    expect(newerServerLayout.value).toBeNull();
  });

  it("notices when the Server has a newer layout than the browser's copy", async () => {
    localStorage.setItem(KEY, JSON.stringify({ revision: 1, layout: custom("browser") }));
    api.layout.mockResolvedValue({ revision: 4, layout: custom("server") });
    await loadLayout();
    expect(layout.value.pages[0].title).toBe("browser");
    expect(newerServerLayout.value?.revision).toBe(4);

    useServerLayout();
    expect(layout.value.pages[0].title).toBe("server");
    expect(layoutSource.value).toBe("server");
    expect(localStorage.getItem(KEY)).toBeNull();
    expect(newerServerLayout.value).toBeNull();
  });

  it("ignores a damaged browser copy", async () => {
    localStorage.setItem(KEY, "{oops");
    api.layout.mockResolvedValue({ revision: 2, layout: custom("server") });
    await loadLayout();
    expect(layoutSource.value).toBe("server");
  });

  it("keeps working when the Server cannot be reached", async () => {
    localStorage.setItem(KEY, JSON.stringify({ revision: 1, layout: custom("browser") }));
    api.layout.mockRejectedValue(new Error("offline"));
    await loadLayout();
    expect(layout.value.pages[0].title).toBe("browser");
  });

  it("saves to this browser and the Server, and remembers the revision", async () => {
    api.layout.mockResolvedValue({ revision: 0, layout: null });
    await loadLayout();
    api.saveLayout.mockResolvedValue({ revision: 5, layout: null });
    expect(await saveLayout(custom("edited"), t)).toBe(true);
    const stored = JSON.parse(localStorage.getItem(KEY) ?? "{}") as { revision: number; layout: Layout };
    expect(stored.revision).toBe(5);
    expect(stored.layout.pages[0].title).toBe("edited");
    expect(api.saveLayout).toHaveBeenCalledOnce();
  });

  it("keeps the browser's copy and says so when the Server refuses the save", async () => {
    api.layout.mockResolvedValue({ revision: 0, layout: null });
    await loadLayout();
    api.saveLayout.mockRejectedValue(new Error("nope"));
    expect(await saveLayout(custom("edited"), t)).toBe(false);
    expect(localStorage.getItem(KEY)).not.toBeNull();
    expect(layout.value.pages[0].title).toBe("edited");
    expect(notify).toHaveBeenCalledWith("warning", "layout.serverSaveFailed");
  });

  it("restores the recommended layout on reset", async () => {
    api.saveLayout.mockResolvedValue({ revision: 6, layout: null });
    await resetLayout(t);
    expect(layout.value).toEqual(recommendedLayout());
  });
});
