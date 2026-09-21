import type { WebModule, WidgetDefinition } from "../widgets/types";

/**
 * Every Web module, found by looking for `modules/<id>/index.ts`. Adding a
 * feature means adding a folder there; nothing else has to list it.
 */
const found = import.meta.glob<{ default: WebModule }>("./*/index.ts", { eager: true });

export const WEB_MODULES: readonly WebModule[] = Object.values(found)
  .map((entry) => entry.default)
  .sort((a, b) => a.order - b.order);

export const WIDGETS: readonly WidgetDefinition[] = WEB_MODULES.flatMap((module) => module.widgets);

const byId = new Map(WIDGETS.map((widget) => [widget.id, widget]));

export function widgetById(id: string): WidgetDefinition | undefined {
  return byId.get(id);
}

export function moduleOfWidget(id: string): WebModule | undefined {
  return WEB_MODULES.find((module) => module.widgets.some((widget) => widget.id === id));
}
