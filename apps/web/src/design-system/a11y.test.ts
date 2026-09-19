// @vitest-environment jsdom
import { mount } from "@vue/test-utils";
import axe from "axe-core";
import { afterEach, describe, expect, it } from "vitest";
import { h } from "vue";
import DesignSystemCatalog from "./catalog/DesignSystemCatalog.vue";
import { AppModal, AppPopover, AppPopoverItem, ConfirmDialog } from "./components";

const sfcModules = import.meta.glob("./components/*/*.vue", { query: "?raw", import: "default", eager: true });

afterEach(() => {
  document.body.innerHTML = "";
});

async function violations(root: Element) {
  const result = await axe.run(root, {
    rules: { "color-contrast": { enabled: false }, region: { enabled: false } },
  });
  return result.violations.map((v) => `${v.id}: ${v.nodes.map((n) => n.target.join(" ")).join(" | ")}`);
}

describe("static accessibility contract", () => {
  const interactive = Object.entries(sfcModules).filter(([, source]) => /<(button|a|input|select|textarea)\b|tabindex=/.test(source.slice(0, source.indexOf("<style"))));

  it("finds the interactive components", () => {
    expect(interactive.length).toBeGreaterThan(10);
  });

  it("every interactive component defines its own keyboard focus style", () => {
    const missing = interactive.filter(([, source]) => !/:focus(-visible|-within)?\b|:has\(input:focus-visible\)/.test(source.slice(source.indexOf("<style")))).map(([path]) => path);
    expect(missing).toEqual([]);
  });
});

describe("axe", () => {
  it("finds no violations in the full component catalog", async () => {
    mount(DesignSystemCatalog, { attachTo: document.body });
    expect(await violations(document.body)).toEqual([]);
  });

  it("finds no violations in open overlays", async () => {
    mount(AppModal, {
      props: { open: true, title: "Edit service" },
      slots: { default: () => h("p", "Body"), footer: () => h("button", { type: "button" }, "Save") },
      attachTo: document.body,
    });
    mount(ConfirmDialog, {
      props: { open: true, title: "Remove package" },
      slots: { default: () => h("p", "Are you sure?"), actions: () => h("button", { type: "button" }, "Remove") },
      attachTo: document.body,
    });
    mount(AppPopover, {
      props: { open: true },
      slots: {
        trigger: () => h("button", { type: "button" }, "Actions"),
        default: () => h(AppPopoverItem, null, () => "Restart"),
      },
      attachTo: document.body,
    });
    expect(await violations(document.body)).toEqual([]);
  });
});
