import { describe, expect, it } from "vitest";
import { ensureLockedWidgets, recommendedLayout } from "./defaults";
import {
  addPage,
  addWidget,
  clampSize,
  moveWidget,
  moveWidgetTo,
  normalizeLayout,
  pageShown,
  removePage,
  removeWidget,
  resizeWidget,
  setWidgetConfig,
  slugFor,
  spansFor,
  updatePage,
} from "./model";
import { widgetById } from "../modules/registry";
import type { Layout } from "../widgets/types";

const cpu = () => {
  const found = widgetById("system.cpu");
  if (!found) throw new Error("system.cpu is not registered");
  return found;
};

function small(): Layout {
  return {
    version: 1,
    pages: [{ id: "home", title: "Home", kind: "scroll", rows: 8, widgets: [] }],
  };
}

describe("normalizeLayout", () => {
  it("accepts what recommendedLayout produces", () => {
    const layout = recommendedLayout();
    expect(normalizeLayout(JSON.parse(JSON.stringify(layout)))).toEqual(layout);
  });

  it.each([null, 3, "x", [], {}, { version: 2, pages: [] }, { version: 1, pages: [] }, { version: 1, pages: [{}] }])(
    "returns nothing for %j",
    (raw) => {
      expect(normalizeLayout(raw)).toBeNull();
    },
  );

  it("repairs sizes, duplicate ids and unusable configuration", () => {
    const layout = normalizeLayout({
      version: 1,
      pages: [
        {
          id: "Bad Id!",
          kind: "nonsense",
          rows: 999,
          widgets: [
            { id: "a", widget: "core.text", w: 99, h: -3, config: { title: "x", nested: { no: 1 }, n: 2 } },
            { id: "a", widget: "core.text", w: 0.4, h: "auto" },
            { widget: "" },
            "junk",
          ],
        },
        { id: "restarting", kind: "scroll", widgets: [] },
      ],
    });
    expect(layout).not.toBeNull();
    const [first, second] = layout?.pages ?? [];
    expect(first.id).toMatch(/^[a-z0-9-]+$/);
    expect(first.kind).toBe("scroll");
    expect(first.rows).toBe(24);
    expect(first.widgets).toHaveLength(2);
    expect(first.widgets[0]).toMatchObject({ w: 12, h: 1, config: { title: "x", n: 2 } });
    expect(first.widgets[1]).toMatchObject({ w: 1, h: "auto" });
    expect(new Set(first.widgets.map((widget) => widget.id)).size).toBe(2);
    expect(second.id).not.toBe("restarting");
  });
});

describe("sizes", () => {
  it("keeps a widget inside the sizes it allows", () => {
    expect(clampSize({ w: 1, h: 1 }, cpu())).toEqual({ w: 2, h: 2 });
    expect(clampSize({ w: 12, h: "auto" }, cpu())).toEqual({ w: 12, h: "auto" });
  });

  it("narrows columns for tablets and phones", () => {
    expect(spansFor(12)).toEqual({ wide: 12, medium: 6, narrow: 2 });
    expect(spansFor(2)).toEqual({ wide: 2, medium: 2, narrow: 1 });
    expect(spansFor(6)).toEqual({ wide: 6, medium: 3, narrow: 2 });
  });
});

describe("locked widgets", () => {
  const placed = (layout: Layout) => layout.pages.flatMap((page) => page.widgets.map((widget) => widget.widget));

  it("come back to a layout that lacks them, on their own page", () => {
    const old: Layout = {
      version: 1,
      pages: [{ id: "home", title: "Home", kind: "scroll", rows: 8, widgets: [] }],
    };
    const fixed = ensureLockedWidgets(old);
    expect(placed(fixed)).toEqual(["settings.display", "settings.password", "settings.totp"]);
    expect(fixed.pages.map((page) => page.id)).toEqual(["home", "settings"]);
  });

  it("are added to the page that already carries their name", () => {
    const partial: Layout = {
      version: 1,
      pages: [{ id: "settings", title: "Mine", kind: "scroll", rows: 8, widgets: [
        { id: "kept", widget: "settings.password", w: 12, h: "auto", config: {} },
      ] }],
    };
    const fixed = ensureLockedWidgets(partial);
    expect(fixed.pages).toHaveLength(1);
    expect(fixed.pages[0].title).toBe("Mine");
    expect(placed(fixed)).toEqual(["settings.password", "settings.display", "settings.totp"]);
  });

  it("leave a complete layout alone", () => {
    const whole = recommendedLayout();
    expect(ensureLockedWidgets(whole)).toBe(whole);
  });

  it("are never offered by the palette groups' source data", () => {
    const definition = widgetById("settings.totp");
    expect(definition?.locked).toBe(true);
    expect(definition?.requires).toEqual([]);
  });
});

describe("moving by drag and drop", () => {
  it("puts the dragged widget where the target is", () => {
    let layout = small();
    for (let count = 0; count < 3; count += 1) layout = addWidget(layout, "home", cpu());
    const [first, second, third] = layout.pages[0].widgets.map((widget) => widget.id);
    const order = (value: Layout) => value.pages[0].widgets.map((widget) => widget.id);

    expect(order(moveWidgetTo(layout, "home", first, third))).toEqual([second, third, first]);
    expect(order(moveWidgetTo(layout, "home", third, first))).toEqual([third, first, second]);
    expect(order(moveWidgetTo(layout, "home", first, first))).toEqual([first, second, third]);
    expect(order(moveWidgetTo(layout, "home", first, "missing"))).toEqual([first, second, third]);
    expect(order(moveWidgetTo(layout, "home", "missing", first))).toEqual([first, second, third]);
  });
});

describe("editing", () => {
  it("adds, moves, resizes, configures and removes widgets", () => {
    let layout = addWidget(small(), "home", cpu());
    layout = addWidget(layout, "home", cpu());
    const [first, second] = layout.pages[0].widgets;
    expect(first).toMatchObject({ widget: "system.cpu", w: 4, h: 3 });

    layout = moveWidget(layout, "home", first.id, 1);
    expect(layout.pages[0].widgets.map((widget) => widget.id)).toEqual([second.id, first.id]);
    layout = moveWidget(layout, "home", first.id, 5);
    expect(layout.pages[0].widgets.map((widget) => widget.id)).toEqual([second.id, first.id]);

    layout = resizeWidget(layout, "home", first.id, { w: 1, h: 9 }, widgetById);
    expect(layout.pages[0].widgets[1]).toMatchObject({ w: 2, h: 9 });

    layout = setWidgetConfig(layout, "home", first.id, "title", "CPU");
    expect(layout.pages[0].widgets[1].config).toEqual({ title: "CPU" });

    layout = removeWidget(layout, "home", second.id);
    expect(layout.pages[0].widgets.map((widget) => widget.id)).toEqual([first.id]);
  });

  it("gives a new widget its default settings", () => {
    const mount = widgetById("storage.mount-usage");
    if (!mount) throw new Error("storage.mount-usage is not registered");
    const layout = addWidget(small(), "home", mount);
    expect(layout.pages[0].widgets[0].config).toEqual({ mount: "/" });
  });

  it("does not change the layout it was given", () => {
    const before = small();
    const snapshot = JSON.stringify(before);
    addWidget(before, "home", cpu());
    addPage(before, "Other", "screen");
    expect(JSON.stringify(before)).toBe(snapshot);
  });

  it("adds, renames and removes pages, keeping the last one", () => {
    const { layout, pageId } = addPage(small(), "My Dashboard", "screen");
    expect(pageId).toBe("my-dashboard");
    expect(layout.pages.at(-1)).toMatchObject({ kind: "screen", title: "My Dashboard" });
    const renamed = updatePage(layout, pageId, { title: "Wall", rows: 100 });
    expect(renamed.pages.at(-1)).toMatchObject({ title: "Wall", rows: 24 });
    const removed = removePage(renamed, pageId);
    expect(removed.pages.map((page) => page.id)).toEqual(["home"]);
    expect(removePage(removed, "home").pages).toHaveLength(1);
  });

  it("makes unique route-safe slugs", () => {
    expect(slugFor("Services", ["services"])).toBe("services-2");
    expect(slugFor("Restarting", [])).not.toBe("restarting");
    expect(slugFor("Settings", [])).toBe("settings");
    expect(slugFor("日本語", [])).toMatch(/^[a-z0-9]+$/);
  });
});

describe("visibility", () => {
  it("hides a page whose widgets are all unavailable, but not an empty one", () => {
    const page = recommendedLayout().pages[1];
    expect(pageShown(page, () => true)).toBe(true);
    expect(pageShown(page, () => false)).toBe(false);
    expect(pageShown({ ...page, widgets: [] }, () => false)).toBe(true);
  });
});
