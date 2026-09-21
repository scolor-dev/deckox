// @vitest-environment jsdom
import { mount } from "@vue/test-utils";
import { defineComponent, h, KeepAlive, nextTick, ref } from "vue";
import { describe, expect, it, vi } from "vitest";
import { useStaleRefresh } from "./useStaleRefresh";

function harness(staleMs: number, clock: { value: number }) {
  const load = vi.fn(() => Promise.resolve());
  let reload: () => Promise<void> = () => Promise.resolve();
  const Page = defineComponent({
    name: "StaleRefreshPage",
    setup() {
      reload = useStaleRefresh(load, staleMs, () => clock.value);
      return () => h("div");
    },
  });
  const shown = ref(true);
  const Shell = () => h(KeepAlive, null, { default: () => (shown.value ? h(Page) : h("span")) });
  const wrapper = mount(Shell);
  return { load, wrapper, shown, reload: () => reload() };
}

async function leaveAndReturn(shown: { value: boolean }) {
  shown.value = false;
  await nextTick();
  shown.value = true;
  await nextTick();
}

describe("useStaleRefresh", () => {
  it("loads once on the first visit", () => {
    const { load } = harness(60_000, { value: 0 });
    expect(load).toHaveBeenCalledTimes(1);
  });

  it("does not reload when returning while the data is fresh", async () => {
    const clock = { value: 0 };
    const { load, shown } = harness(60_000, clock);
    clock.value = 59_999;
    await leaveAndReturn(shown);
    expect(load).toHaveBeenCalledTimes(1);
  });

  it("reloads on return once the data is older than the limit", async () => {
    const clock = { value: 0 };
    const { load, shown } = harness(60_000, clock);
    clock.value = 60_000;
    await leaveAndReturn(shown);
    expect(load).toHaveBeenCalledTimes(2);
  });

  it("restarts the clock when the view reloads itself", async () => {
    const clock = { value: 0 };
    const { load, shown, reload } = harness(60_000, clock);
    clock.value = 50_000;
    await reload();
    clock.value = 100_000;
    await leaveAndReturn(shown);
    expect(load).toHaveBeenCalledTimes(2);
  });
});
