// @vitest-environment jsdom
import { flushPromises, mount, type VueWrapper } from "@vue/test-utils";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { createMemoryHistory, createRouter } from "vue-router";
import type { Layout } from "../widgets/types";

const api = vi.hoisted(() => ({ layout: vi.fn(), saveLayout: vi.fn(), modules: vi.fn() }));
vi.mock("../api/client", async (importOriginal) => ({
  ...(await importOriginal<Record<string, unknown>>()),
  api,
}));

import { i18n } from "../i18n";
import { layout, layoutSource } from "../layout/store";
import { mergeModuleMessages } from "../modules/messages";
import PageView from "./PageView.vue";

mergeModuleMessages(i18n);

function twoPages(): Layout {
  return {
    version: 1,
    pages: [
      {
        id: "home", title: "Home", kind: "scroll", rows: 8,
        widgets: [{ id: "note", widget: "core.text", w: 6, h: 2, config: { title: "Hello", body: "world" } }],
      },
      { id: "other", title: "Other", kind: "scroll", rows: 8, widgets: [] },
    ],
  };
}

let wrapper: VueWrapper;

async function open(path: string) {
  const router = createRouter({
    history: createMemoryHistory(),
    routes: [
      { path: "/settings", component: { template: "<div />" } },
      { path: "/:pageId?", name: "page", component: PageView },
    ],
  });
  await router.push(path);
  await router.isReady();
  wrapper = mount({ template: "<RouterView />" }, { global: { plugins: [i18n, router] }, attachTo: document.body });
  await flushPromises();
  return router;
}

const button = (label: string) =>
  [...document.body.querySelectorAll("button")].find((entry) => entry.textContent.trim() === label);

async function press(label: string) {
  const target = button(label);
  if (!target) throw new Error(`no button "${label}"`);
  target.click();
  await flushPromises();
}

beforeEach(() => {
  localStorage.clear();
  i18n.global.locale.value = "en";
  layout.value = twoPages();
  layoutSource.value = "server";
  api.saveLayout.mockReset();
  api.saveLayout.mockImplementation((saved: unknown) => Promise.resolve({ revision: 1, layout: saved }));
  api.modules.mockResolvedValue({ agent: { connected: true, agent_version: null, protocol_version: null, compatible: true }, modules: [], server_modules: [] });
});

afterEach(() => {
  wrapper.unmount();
  document.body.innerHTML = "";
});

describe("PageView", () => {
  it("shows the widgets of the page in the address", async () => {
    await open("/home");
    expect(document.body.querySelector('[data-widget="core.text"]')).not.toBeNull();
    await vi.waitFor(() => { expect(document.body.textContent).toContain("world"); });
  });

  it("enters edit mode and shows the layout controls", async () => {
    await open("/home");
    await press("Edit layout");
    expect(button("Done")).toBeDefined();
    expect(button("Add widget")).toBeDefined();
    expect(document.body.querySelector('[aria-label="Remove widget"]')).not.toBeNull();
  });

  it("adds a widget from the palette and saves the page when done", async () => {
    await open("/home");
    await press("Edit layout");
    await press("Add widget");
    const divider = [...document.body.querySelectorAll(".palette-item")]
      .find((item) => item.textContent.includes("Divider"))
      ?.querySelector("button");
    expect(divider).toBeTruthy();
    divider?.click();
    await flushPromises();
    await press("Done");

    expect(api.saveLayout).toHaveBeenCalledOnce();
    const saved = api.saveLayout.mock.calls[0][0] as Layout;
    expect(saved.pages[0].widgets.map((widget) => widget.widget)).toEqual(["core.text", "core.divider"]);
    expect(layout.value.pages[0].widgets).toHaveLength(2);
    expect(localStorage.getItem("deckox:layout")).not.toBeNull();
    expect(document.body.querySelector('[aria-label="Remove widget"]')).toBeNull();
  });

  it("discards the changes on cancel", async () => {
    await open("/home");
    await press("Edit layout");
    await press("Add widget");
    [...document.body.querySelectorAll(".palette-item button")][0]?.dispatchEvent(new Event("click"));
    await flushPromises();
    await press("Cancel");
    expect(api.saveLayout).not.toHaveBeenCalled();
    expect(layout.value.pages[0].widgets).toHaveLength(1);
  });

  it("does not save when nothing changed", async () => {
    await open("/home");
    await press("Edit layout");
    await press("Done");
    expect(api.saveLayout).not.toHaveBeenCalled();
  });

  it("adds a page and opens it", async () => {
    const router = await open("/home");
    await press("Edit layout");
    await press("Add page");
    const name = document.body.querySelector<HTMLInputElement>("#page-title");
    expect(name).not.toBeNull();
    if (name) {
      name.value = "Wall display";
      name.dispatchEvent(new Event("input"));
    }
    await flushPromises();
    await press("Save");
    expect(router.currentRoute.value.path).toBe("/wall-display");
    await press("Done");
    const saved = api.saveLayout.mock.calls[0][0] as Layout;
    expect(saved.pages.map((entry) => entry.id)).toEqual(["home", "other", "wall-display"]);
  });

  it("removes a widget in edit mode", async () => {
    await open("/home");
    await press("Edit layout");
    const remove = document.body.querySelector<HTMLButtonElement>('[aria-label="Remove widget"]');
    expect(remove).not.toBeNull();
    remove?.click();
    await flushPromises();
    await press("Done");
    const saved = api.saveLayout.mock.calls[0][0] as Layout;
    expect(saved.pages[0].widgets).toEqual([]);
  });
});
