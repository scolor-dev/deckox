// @vitest-environment jsdom
/* eslint-disable vue/one-component-per-file */
import { mount } from "@vue/test-utils";
import { afterEach, describe, expect, it } from "vitest";
import { defineComponent, h, nextTick, ref, type Component } from "vue";
import { AppModal, AppPopover, AppPopoverItem, ConfirmDialog, TabBar } from "./components";

const mounted: { unmount: () => void }[] = [];
afterEach(() => {
  while (mounted.length > 0) mounted.pop()?.unmount();
  document.body.innerHTML = "";
});

function attach<T extends { unmount: () => void }>(wrapper: T): T {
  mounted.push(wrapper);
  return wrapper;
}

function must<T>(value: T | null | undefined): T {
  if (value === null || value === undefined) throw new Error("expected a value");
  return value;
}
const byId = (id: string) => must(document.getElementById(id));

const key = (name: string, init: KeyboardEventInit = {}) =>
  new KeyboardEvent("keydown", { key: name, bubbles: true, cancelable: true, ...init });

async function settle() {
  await nextTick();
  await nextTick();
}

describe("focus trap in overlays", () => {
  function host(dialog: Component) {
    const open = ref(false);
    const Host = defineComponent({
      setup: () => () => h("div", [
        h("button", { id: "opener" }, "open"),
        h(dialog, { open: open.value, title: "T" }, {
          default: () => h("input", { id: "first" }),
          ...(dialog === AppModal
            ? { footer: () => h("button", { id: "last" }, "ok") }
            : { actions: () => h("button", { id: "last" }, "ok") }),
        }),
      ]),
    });
    const wrapper = attach(mount(Host, { attachTo: document.body }));
    return { wrapper, open };
  }

  it.each([["AppModal", AppModal], ["ConfirmDialog", ConfirmDialog]] as const)("%s moves focus into the dialog, wraps Tab and restores focus", async (_name, dialog) => {
    const { open } = host(dialog);
    const opener = byId("opener");
    opener.focus();
    open.value = true;
    await settle();

    const dialogEl = document.querySelector<HTMLElement>("[role='dialog']");
    expect(document.activeElement).toBe(dialogEl);

    const last = byId("last");
    last.focus();
    const forward = key("Tab");
    window.dispatchEvent(forward);
    expect(forward.defaultPrevented).toBe(true);
    expect(dialogEl?.contains(document.activeElement)).toBe(true);
    expect(document.activeElement).not.toBe(last);

    const first = must(dialogEl?.querySelector<HTMLElement>("button, input"));
    first.focus();
    const backward = key("Tab", { shiftKey: true });
    window.dispatchEvent(backward);
    expect(backward.defaultPrevented).toBe(true);
    expect(document.activeElement).toBe(last);

    open.value = false;
    await settle();
    expect(document.activeElement).toBe(opener);
  });

  it("pulls focus back in when it has escaped the dialog", async () => {
    const { open } = host(AppModal);
    open.value = true;
    await settle();
    byId("opener").focus();
    const event = key("Tab");
    window.dispatchEvent(event);
    expect(event.defaultPrevented).toBe(true);
    expect(document.querySelector("[role='dialog']")?.contains(document.activeElement)).toBe(true);
  });

  it("only the topmost of two open overlays traps focus", async () => {
    const outer = ref(true);
    const inner = ref(false);
    const Host = defineComponent({
      setup: () => () => h("div", [
        h(AppModal, { open: outer.value, title: "outer" }, { default: () => h("button", { id: "outer-btn" }, "a") }),
        h(ConfirmDialog, { open: inner.value, title: "inner" }, { default: () => h("button", { id: "inner-btn" }, "b") }),
      ]),
    });
    attach(mount(Host, { attachTo: document.body }));
    await settle();
    inner.value = true;
    await settle();
    const innerDialog = document.querySelectorAll<HTMLElement>("[role='dialog']")[1];
    expect(document.activeElement).toBe(innerDialog);
    byId("inner-btn").focus();
    window.dispatchEvent(key("Tab"));
    expect(innerDialog.contains(document.activeElement)).toBe(true);
  });
});

describe("TabBar keyboard", () => {
  const tabs = [
    { key: "a", label: "A" },
    { key: "b", label: "B" },
    { key: "c", label: "C" },
  ];

  it("uses roving tabindex and arrow / Home / End navigation", async () => {
    const wrapper = attach(mount(TabBar, { props: { tabs, modelValue: "a", label: "Tabs" }, attachTo: document.body }));
    const buttons = wrapper.findAll("[role='tab']");
    expect(buttons.map((b) => b.attributes("tabindex"))).toEqual(["0", "-1", "-1"]);

    await buttons[0].trigger("keydown", { key: "ArrowRight" });
    expect(wrapper.emitted("update:modelValue")?.at(-1)).toEqual(["b"]);
    expect(document.activeElement).toBe(buttons[1].element);

    await buttons[0].trigger("keydown", { key: "ArrowLeft" });
    expect(wrapper.emitted("update:modelValue")?.at(-1)).toEqual(["c"]);

    await buttons[2].trigger("keydown", { key: "ArrowRight" });
    expect(wrapper.emitted("update:modelValue")?.at(-1)).toEqual(["a"]);

    await buttons[0].trigger("keydown", { key: "End" });
    expect(wrapper.emitted("update:modelValue")?.at(-1)).toEqual(["c"]);
    await buttons[2].trigger("keydown", { key: "Home" });
    expect(wrapper.emitted("update:modelValue")?.at(-1)).toEqual(["a"]);
  });
});

describe("AppPopover keyboard", () => {
  function popover() {
    const open = ref(false);
    const closed: string[] = [];
    const Host = defineComponent({
      setup: () => () => h("div", [
        h("button", { id: "after" }, "after"),
        h(AppPopover, { open: open.value, onClose: () => { closed.push("close"); open.value = false; } }, {
          trigger: () => h("button", { id: "trigger" }, "menu"),
          default: () => [
            h(AppPopoverItem, null, () => "one"),
            h(AppPopoverItem, null, () => "two"),
            h(AppPopoverItem, null, () => "three"),
          ],
        }),
      ]),
    });
    const wrapper = attach(mount(Host, { attachTo: document.body }));
    return { wrapper, open, closed };
  }

  it("focuses the first item, cycles with arrows, and returns focus to the trigger on Escape", async () => {
    const { open, closed } = popover();
    const trigger = byId("trigger");
    trigger.focus();
    open.value = true;
    await settle();

    const items = [...document.querySelectorAll<HTMLElement>("[role='menuitem']")];
    expect(document.activeElement).toBe(items[0]);
    items[0].dispatchEvent(key("ArrowDown"));
    expect(document.activeElement).toBe(items[1]);
    items[1].dispatchEvent(key("End"));
    expect(document.activeElement).toBe(items[2]);
    items[2].dispatchEvent(key("ArrowDown"));
    expect(document.activeElement).toBe(items[0]);
    items[0].dispatchEvent(key("ArrowUp"));
    expect(document.activeElement).toBe(items[2]);
    items[2].dispatchEvent(key("Home"));
    expect(document.activeElement).toBe(items[0]);

    window.dispatchEvent(key("Escape"));
    await settle();
    expect(closed).toEqual(["close"]);
    expect(document.activeElement).toBe(trigger);
  });

  it("closes on Tab and hands focus back to the trigger so Tab continues from it", async () => {
    const { open, closed } = popover();
    const trigger = byId("trigger");
    trigger.focus();
    open.value = true;
    await settle();
    const item = must(document.querySelector<HTMLElement>("[role='menuitem']"));
    item.dispatchEvent(key("Tab"));
    await settle();
    expect(closed).toEqual(["close"]);
    expect(document.activeElement).toBe(trigger);
  });
});
