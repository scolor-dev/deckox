import type { SoftwarePackage } from "./api/client";

export interface SoftwareRow {
  pkg: SoftwarePackage;
  depth: 0 | 1;
  childCount: number;
  expanded: boolean;
}

/**
 * Lays the managed packages out as bundles: a package with a `parent` that is
 * itself in the list becomes a child row under it, shown only while the
 * parent is expanded. `isVisible` filters the top-level rows; hiding a parent
 * hides its whole bundle.
 */
export function buildSoftwareRows(
  packages: SoftwarePackage[],
  expanded: ReadonlySet<string>,
  isVisible: (pkg: SoftwarePackage) => boolean,
): SoftwareRow[] {
  const names = new Set(packages.map((pkg) => pkg.name));
  const children = new Map<string, SoftwarePackage[]>();
  const topLevel: SoftwarePackage[] = [];
  for (const pkg of packages) {
    if (pkg.parent != null && pkg.parent !== pkg.name && names.has(pkg.parent)) {
      children.set(pkg.parent, [...(children.get(pkg.parent) ?? []), pkg]);
    } else {
      topLevel.push(pkg);
    }
  }

  return topLevel.filter(isVisible).flatMap((pkg) => {
    const bundle = children.get(pkg.name) ?? [];
    const open = expanded.has(pkg.name);
    return [
      { pkg, depth: 0 as const, childCount: bundle.length, expanded: open },
      ...(open ? bundle.map((child) => ({ pkg: child, depth: 1 as const, childCount: 0, expanded: false })) : []),
    ];
  });
}
