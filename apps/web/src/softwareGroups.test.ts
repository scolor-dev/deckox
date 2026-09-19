import { describe, expect, it } from "vitest";
import type { SoftwarePackage } from "./api/client";
import { buildSoftwareRows } from "./softwareGroups";

const pkg = (name: string, parent: string | null = null, installed = true): SoftwarePackage => ({
  name,
  installed,
  installed_version: installed ? "1" : null,
  available_version: "1",
  upgradable: false,
  parent,
});

const packages = [
  pkg("docker-ce"),
  pkg("docker-ce-cli", "docker-ce"),
  pkg("containerd.io", "docker-ce"),
  pkg("git"),
  pkg("orphan-child", "missing-parent"),
];

describe("buildSoftwareRows", () => {
  it("keeps bundles collapsed by default and counts their members", () => {
    const rows = buildSoftwareRows(packages, new Set(), () => true);
    expect(rows.map((row) => [row.pkg.name, row.depth, row.childCount])).toEqual([
      ["docker-ce", 0, 2],
      ["git", 0, 0],
      ["orphan-child", 0, 0],
    ]);
  });

  it("shows the members under an expanded parent", () => {
    const rows = buildSoftwareRows(packages, new Set(["docker-ce"]), () => true);
    expect(rows.map((row) => [row.pkg.name, row.depth, row.expanded])).toEqual([
      ["docker-ce", 0, true],
      ["docker-ce-cli", 1, false],
      ["containerd.io", 1, false],
      ["git", 0, false],
      ["orphan-child", 0, false],
    ]);
  });

  it("hides a whole bundle when its parent is filtered out", () => {
    const rows = buildSoftwareRows(packages, new Set(["docker-ce"]), (item) => item.name !== "docker-ce");
    expect(rows.map((row) => row.pkg.name)).toEqual(["git", "orphan-child"]);
  });
});
