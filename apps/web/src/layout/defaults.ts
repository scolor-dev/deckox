import { DEFAULT_SCREEN_ROWS, type Layout, type PageLayout, type WidgetHeight } from "../widgets/types";

function page(id: string, widgets: [string, number, WidgetHeight][]): PageLayout {
  return {
    id,
    titleKey: `nav.${id}`,
    kind: "scroll",
    rows: DEFAULT_SCREEN_ROWS,
    widgets: widgets.map(([widget, w, h]) => ({
      id: `${id}-${widget.replace(/[^a-z0-9]+/g, "-")}`,
      widget,
      w,
      h,
      config: {},
    })),
  };
}

/**
 * The arrangement a fresh install starts with: one page for each feature
 * area, with the widgets that fit it. A page whose modules are switched off
 * is left out of the navigation, so this can list every module's widgets.
 */
export function recommendedLayout(): Layout {
  return {
    version: 1,
    pages: [
      page("overview", [
        ["core.server-status", 4, 2],
        ["system.summary", 4, 2],
        ["system.live", 4, 2],
        ["system.cpu", 4, 3],
        ["system.memory", 4, 3],
        ["system.load", 4, 3],
        ["system.swap", 4, 3],
        ["system.network", 4, 3],
        ["system.disk-io", 4, 3],
        ["storage.overall", 6, 4],
        ["system.temperature", 6, 4],
        ["system.info", 12, "auto"],
      ]),
      page("services", [
        ["services.list", 12, "auto"],
        ["services.schedules", 12, "auto"],
      ]),
      page("software", [
        ["software.list", 12, "auto"],
        ["software.add", 12, "auto"],
        ["software.installed", 12, "auto"],
      ]),
      page("storage", [
        ["storage.allocation", 12, "auto"],
        ["storage.mounts", 12, "auto"],
        ["storage.disks", 12, "auto"],
      ]),
      page("diagnostics", [
        ["diagnostics.summary", 12, "auto"],
        ["diagnostics.backups", 12, "auto"],
      ]),
      page("audit", [["audit.log", 12, "auto"]]),
      page("maintenance", [
        ["update-check.status", 6, "auto"],
        ["power.restart", 6, "auto"],
        ["notifications.webhook", 12, "auto"],
      ]),
    ],
  };
}
