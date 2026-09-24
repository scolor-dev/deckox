import {
  DEFAULT_SCREEN_ROWS,
  GRID_COLUMNS,
  MAX_ROWS,
  MAX_SCREEN_ROWS,
  MIN_SCREEN_ROWS,
  type ConfigValue,
  type Layout,
  type PageKind,
  type PageLayout,
  type WidgetConfig,
  type WidgetDefinition,
  type WidgetHeight,
  type WidgetPlacement,
  type WidgetSize,
} from "../widgets/types";

/** Page ids that are routes of their own. */
export const RESERVED_PAGE_IDS: readonly string[] = ["restarting"];

const PAGE_ID = /^[a-z0-9][a-z0-9-]{0,39}$/;
const MAX_TITLE = 60;
const MAX_WIDGETS_PER_PAGE = 60;
const MAX_TEXT = 4000;

type Lookup = (widgetId: string) => WidgetDefinition | undefined;

export function newId(): string {
  if (typeof crypto !== "undefined" && "randomUUID" in crypto) return crypto.randomUUID().slice(0, 8);
  return Math.random().toString(36).slice(2, 10);
}

const clamp = (value: number, min: number, max: number) => Math.min(Math.max(value, min), max);

export function clampWidth(width: number, min = 1, max = GRID_COLUMNS): number {
  return clamp(Math.round(width), min, max);
}

export function clampHeight(height: WidgetHeight, min: WidgetHeight = 1, max = MAX_ROWS): WidgetHeight {
  if (height === "auto") return "auto";
  const floor = min === "auto" ? 1 : min;
  return clamp(Math.round(height), floor, max);
}

/** Keeps a size inside what the widget allows. */
export function clampSize(size: WidgetSize, definition: WidgetDefinition | undefined): WidgetSize {
  if (!definition) return { w: clampWidth(size.w), h: clampHeight(size.h) };
  const { min, max } = definition.size;
  return {
    w: clampWidth(size.w, min.w, max?.w ?? GRID_COLUMNS),
    h: clampHeight(size.h, min.h, max?.h ?? MAX_ROWS),
  };
}

/**
 * How many columns a widget takes as the grid narrows: 12 columns wide, 6 at
 * tablet width, 2 on a phone (where the small ones sit two to a row).
 */
export function spansFor(width: number): { wide: number; medium: number; narrow: number } {
  const wide = clampWidth(width);
  return {
    wide,
    medium: clamp(Math.max(2, Math.ceil(wide / 2)), 1, 6),
    narrow: wide <= 3 ? 1 : 2,
  };
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function cleanConfig(raw: unknown): WidgetConfig {
  if (!isRecord(raw)) return {};
  const config: WidgetConfig = {};
  for (const [key, value] of Object.entries(raw)) {
    if (typeof value === "string") config[key] = value.slice(0, MAX_TEXT);
    else if (typeof value === "number" && Number.isFinite(value)) config[key] = value;
    else if (typeof value === "boolean") config[key] = value;
  }
  return config;
}

function cleanPlacement(raw: unknown, seen: Set<string>): WidgetPlacement | null {
  if (!isRecord(raw) || typeof raw.widget !== "string" || raw.widget === "") return null;
  let id = typeof raw.id === "string" && raw.id !== "" ? raw.id : newId();
  while (seen.has(id)) id = newId();
  seen.add(id);
  const w = typeof raw.w === "number" ? clampWidth(raw.w) : GRID_COLUMNS;
  const h = raw.h === "auto" ? "auto" : typeof raw.h === "number" ? clampHeight(raw.h) : "auto";
  return { id, widget: raw.widget, w, h, config: cleanConfig(raw.config) };
}

function cleanPage(raw: unknown, seenPages: Set<string>): PageLayout | null {
  if (!isRecord(raw) || !Array.isArray(raw.widgets)) return null;
  let id = typeof raw.id === "string" && PAGE_ID.test(raw.id) && !RESERVED_PAGE_IDS.includes(raw.id) ? raw.id : newId();
  while (seenPages.has(id)) id = newId();
  seenPages.add(id);
  const seen = new Set<string>();
  const widgets = raw.widgets
    .slice(0, MAX_WIDGETS_PER_PAGE)
    .map((entry) => cleanPlacement(entry, seen))
    .filter((entry): entry is WidgetPlacement => entry !== null);
  const page: PageLayout = {
    id,
    kind: raw.kind === "screen" ? "screen" : "scroll",
    rows: typeof raw.rows === "number" ? clamp(Math.round(raw.rows), MIN_SCREEN_ROWS, MAX_SCREEN_ROWS) : DEFAULT_SCREEN_ROWS,
    widgets,
  };
  if (typeof raw.title === "string" && raw.title.trim() !== "") page.title = raw.title.trim().slice(0, MAX_TITLE);
  if (typeof raw.titleKey === "string" && raw.titleKey !== "") page.titleKey = raw.titleKey;
  if (page.title === undefined && page.titleKey === undefined) page.title = id;
  return page;
}

/**
 * Reads a layout from storage (the browser's or the Server's), dropping what
 * it cannot make sense of. `null` means there is nothing usable.
 */
export function normalizeLayout(raw: unknown): Layout | null {
  if (!isRecord(raw) || raw.version !== 1 || !Array.isArray(raw.pages)) return null;
  const seenPages = new Set<string>();
  const pages = raw.pages
    .map((page) => cleanPage(page, seenPages))
    .filter((page): page is PageLayout => page !== null);
  return pages.length === 0 ? null : { version: 1, pages };
}

export function pageById(layout: Layout, pageId: string): PageLayout | undefined {
  return layout.pages.find((page) => page.id === pageId);
}

function withPage(layout: Layout, pageId: string, change: (page: PageLayout) => PageLayout): Layout {
  return { ...layout, pages: layout.pages.map((page) => (page.id === pageId ? change(page) : page)) };
}

/** A slug for a page name; falls back to a random id for names with no letters. */
export function slugFor(title: string, taken: readonly string[]): string {
  const base = title
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-+|-+$/g, "")
    .slice(0, 30);
  let slug = base;
  let counter = 2;
  while (slug === "" || taken.includes(slug) || RESERVED_PAGE_IDS.includes(slug)) {
    slug = base === "" ? newId() : `${base}-${String(counter)}`;
    counter += 1;
  }
  return slug;
}

export function addPage(layout: Layout, title: string, kind: PageKind): { layout: Layout; pageId: string } {
  const pageId = slugFor(title, layout.pages.map((page) => page.id));
  const page: PageLayout = {
    id: pageId,
    title: title.trim().slice(0, MAX_TITLE) || pageId,
    kind,
    rows: DEFAULT_SCREEN_ROWS,
    widgets: [],
  };
  return { layout: { ...layout, pages: [...layout.pages, page] }, pageId };
}

export function updatePage(
  layout: Layout,
  pageId: string,
  patch: { title?: string; kind?: PageKind; rows?: number },
): Layout {
  return withPage(layout, pageId, (page) => {
    const next = { ...page };
    if (patch.title !== undefined && patch.title.trim() !== "") {
      next.title = patch.title.trim().slice(0, MAX_TITLE);
      delete next.titleKey;
    }
    if (patch.kind !== undefined) next.kind = patch.kind;
    if (patch.rows !== undefined) next.rows = clamp(Math.round(patch.rows), MIN_SCREEN_ROWS, MAX_SCREEN_ROWS);
    return next;
  });
}

/** The last page cannot be removed: the app needs somewhere to land. */
export function removePage(layout: Layout, pageId: string): Layout {
  if (layout.pages.length <= 1) return layout;
  return { ...layout, pages: layout.pages.filter((page) => page.id !== pageId) };
}

function moved<T>(items: readonly T[], index: number, delta: number): T[] {
  const target = clamp(index + delta, 0, items.length - 1);
  if (index < 0 || target === index) return [...items];
  const next = [...items];
  const [item] = next.splice(index, 1);
  next.splice(target, 0, item);
  return next;
}

export function movePage(layout: Layout, pageId: string, delta: number): Layout {
  const index = layout.pages.findIndex((page) => page.id === pageId);
  return { ...layout, pages: moved(layout.pages, index, delta) };
}

export function defaultConfigOf(definition: WidgetDefinition): WidgetConfig {
  const config: WidgetConfig = {};
  for (const field of definition.config ?? []) {
    if (field.default !== undefined) config[field.key] = field.default;
  }
  return config;
}

export function addWidget(layout: Layout, pageId: string, definition: WidgetDefinition): Layout {
  const { w, h } = definition.size.default;
  const placement: WidgetPlacement = { id: newId(), widget: definition.id, w, h, config: defaultConfigOf(definition) };
  return withPage(layout, pageId, (page) => ({ ...page, widgets: [...page.widgets, placement] }));
}

export function removeWidget(layout: Layout, pageId: string, placementId: string): Layout {
  return withPage(layout, pageId, (page) => ({
    ...page,
    widgets: page.widgets.filter((widget) => widget.id !== placementId),
  }));
}

export function moveWidget(layout: Layout, pageId: string, placementId: string, delta: number): Layout {
  return withPage(layout, pageId, (page) => ({
    ...page,
    widgets: moved(page.widgets, page.widgets.findIndex((widget) => widget.id === placementId), delta),
  }));
}

export function moveWidgetTo(layout: Layout, pageId: string, placementId: string, targetId: string): Layout {
  return withPage(layout, pageId, (page) => {
    const from = page.widgets.findIndex((widget) => widget.id === placementId);
    const to = page.widgets.findIndex((widget) => widget.id === targetId);
    if (from < 0 || to < 0) return page;
    return { ...page, widgets: moved(page.widgets, from, to - from) };
  });
}

export function resizeWidget(
  layout: Layout,
  pageId: string,
  placementId: string,
  size: WidgetSize,
  lookup: Lookup,
): Layout {
  return withPage(layout, pageId, (page) => ({
    ...page,
    widgets: page.widgets.map((widget) => {
      if (widget.id !== placementId) return widget;
      return { ...widget, ...clampSize(size, lookup(widget.widget)) };
    }),
  }));
}

export function setWidgetConfig(
  layout: Layout,
  pageId: string,
  placementId: string,
  key: string,
  value: ConfigValue,
): Layout {
  return withPage(layout, pageId, (page) => ({
    ...page,
    widgets: page.widgets.map((widget) =>
      widget.id === placementId ? { ...widget, config: { ...widget.config, [key]: value } } : widget),
  }));
}

/** The placements of a page that can be shown right now. */
export function shownWidgets(page: PageLayout, isAvailable: (widgetId: string) => boolean): WidgetPlacement[] {
  return page.widgets.filter((widget) => isAvailable(widget.widget));
}

/**
 * A page is offered unless it holds widgets and none of them can be shown, as
 * when the module behind its only widgets is switched off. An empty page
 * stays, since it is one the user has just made.
 */
export function pageShown(page: PageLayout, isAvailable: (widgetId: string) => boolean): boolean {
  return page.widgets.length === 0 || shownWidgets(page, isAvailable).length > 0;
}

export function sameLayout(a: Layout, b: Layout): boolean {
  return JSON.stringify(a) === JSON.stringify(b);
}
