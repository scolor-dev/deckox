// @vitest-environment jsdom
import { mount } from "@vue/test-utils";
import { describe, expect, it } from "vitest";
import AppDataTable from "./AppDataTable.vue";
import { sortRows, type DataColumn, type DataRow } from "./dataTable";

const columns: DataColumn[] = [
  { key: "name", label: "Name", sortable: true },
  { key: "memory", label: "Memory", sortable: true, align: "end" },
  { key: "state", label: "State" },
];
const rows: DataRow[] = [
  { name: "nginx", memory: 120, state: "running" },
  { name: "docker", memory: 900, state: "running" },
  { name: "cron", memory: null, state: "stopped" },
  { name: "sshd10", memory: 8, state: "running" },
  { name: "sshd2", memory: 9, state: "running" },
];

describe("sortRows", () => {
  it("sorts numbers numerically and strings naturally, with null last in both directions", () => {
    expect(sortRows(rows, "memory", "asc").map((r) => r.name)).toEqual(["sshd10", "sshd2", "nginx", "docker", "cron"]);
    expect(sortRows(rows, "memory", "desc").map((r) => r.name)).toEqual(["docker", "nginx", "sshd2", "sshd10", "cron"]);
    expect(sortRows(rows, "name", "asc").map((r) => r.name)).toEqual(["cron", "docker", "nginx", "sshd2", "sshd10"]);
  });

  it("is stable and returns the input when no key is set", () => {
    const tied = sortRows(rows, "state", "asc").map((r) => r.name);
    expect(tied).toEqual(["nginx", "docker", "sshd10", "sshd2", "cron"]);
    expect(sortRows(rows, null, "asc")).toBe(rows);
    expect(rows[0].name).toBe("nginx");
  });
});

describe("AppDataTable", () => {
  function render(props: Record<string, unknown> = {}) {
    return mount(AppDataTable, { props: { caption: "Services", columns, rows, rowKey: "name", ...props } });
  }

  it("renders a captioned table and sorts locally by the controlled sort props", () => {
    const wrapper = render({ sortKey: "memory", sortDirection: "desc" });
    expect(wrapper.find("caption").text()).toBe("Services");
    expect(wrapper.findAll("tbody tr td:first-child").map((td) => td.text())).toEqual(["docker", "nginx", "sshd2", "sshd10", "cron"]);
    const headers = wrapper.findAll("th");
    expect(headers.map((th) => th.attributes("aria-sort"))).toEqual(["none", "descending", undefined]);
  });

  it("does not sort in manual mode", () => {
    const wrapper = render({ sortKey: "name", manual: true });
    expect(wrapper.findAll("tbody tr td:first-child").map((td) => td.text())).toEqual(["nginx", "docker", "cron", "sshd10", "sshd2"]);
  });

  it("emits sort with a toggling direction", async () => {
    const wrapper = render({ sortKey: "name", sortDirection: "asc" });
    const buttons = wrapper.findAll("th button");
    await buttons[0].trigger("click");
    await buttons[1].trigger("click");
    expect(wrapper.emitted("sort")).toEqual([["name", "desc"], ["memory", "asc"]]);
  });

  it("selects rows, selects all, and labels every checkbox", async () => {
    const wrapper = render({ selectable: true, selected: ["nginx"] });
    const boxes = wrapper.findAll("input[type='checkbox']");
    expect(boxes).toHaveLength(rows.length + 1);
    expect(boxes[0].attributes("aria-label")).toBe("Select all rows");
    expect(boxes[1].attributes("aria-label")).toBe("Select row nginx");
    expect((boxes[0].element as HTMLInputElement).indeterminate).toBe(true);

    await boxes[2].setValue(true);
    await boxes[1].setValue(false);
    await boxes[0].setValue(true);
    expect(wrapper.emitted("update:selected")).toEqual([
      [["nginx", "docker"]],
      [[]],
      [["nginx", "docker", "cron", "sshd10", "sshd2"]],
    ]);
  });

  it("uses cell slots and marks selected rows", () => {
    const wrapper = mount(AppDataTable, {
      props: { caption: "Services", columns, rows, rowKey: "name", selectable: true, selected: ["cron"] },
      slots: { "cell-state": `<template #cell-state="{ value }"><b>{{ value }}!</b></template>` },
    });
    expect(wrapper.findAll("tbody tr")[2].find("b").text()).toBe("stopped!");
    expect(wrapper.findAll("tr.ds-data-table-row--selected")).toHaveLength(1);
  });
});
