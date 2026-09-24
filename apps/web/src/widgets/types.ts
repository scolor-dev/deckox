import type { Component, Ref } from "vue";

/** Rows of the grid, or `auto` to take the height the content needs. */
export type WidgetHeight = number | "auto";

export interface WidgetSize {
  w: number;
  h: WidgetHeight;
}

export type ConfigValue = string | number | boolean;
export type WidgetConfig = Record<string, ConfigValue>;

export interface SelectOption {
  value: string;
  label: string;
}

/** One setting of a widget instance, rendered by the editor. */
export type ConfigField =
  | { key: string; kind: "text"; labelKey: string; multiline?: boolean; default?: string }
  | { key: string; kind: "select"; labelKey: string; default: string; options: () => Readonly<Ref<readonly SelectOption[]>> };

/**
 * A widget a module offers. The Web core draws the grid, the frame and the
 * editor; the module supplies what goes inside.
 */
export interface WidgetDefinition {
  /** `<module>.<name>`, unique across all modules. */
  id: string;
  titleKey: string;
  descriptionKey: string;
  /** Agent and Server module ids that must be on; empty for core widgets. */
  requires: readonly string[];
  component: () => Promise<{ default: Component }>;
  size: {
    default: WidgetSize;
    min: WidgetSize;
    max?: { w: number; h: number };
  };
  config?: readonly ConfigField[];
  /**
   * `frame` draws a card with the widget's title; `bare` leaves the widget to
   * draw its own box (metric cards, tables), so it fills the space it is given.
   */
  chrome: "frame" | "bare";
  /**
   * A widget that stays where the recommended layout puts it: it cannot be
   * removed, moved, resized or configured, and the palette does not offer it.
   * A layout that lacks one gets it back when it is loaded.
   */
  locked?: boolean;
  /** A title taken from the configuration (a text block's own heading). */
  titleFromConfig?: (config: WidgetConfig) => string | null;
}

export interface WebModule {
  /** Matches the Agent or Server module it shows, where there is one. */
  id: string;
  /** Position of the module in the widget palette. */
  order: number;
  titleKey: string;
  widgets: readonly WidgetDefinition[];
  /** Texts the module's widgets use; merged into the app's messages. */
  messages?: { ja: Record<string, unknown>; en: Record<string, unknown> };
}

export interface WidgetPlacement {
  /** Identifies this placement on its page. */
  id: string;
  /** The `WidgetDefinition.id` it shows. */
  widget: string;
  w: number;
  h: WidgetHeight;
  config: WidgetConfig;
}

/** `scroll` grows downwards; `screen` fits one screen and never scrolls. */
export type PageKind = "scroll" | "screen";

export interface PageLayout {
  id: string;
  /** A name typed by the user. */
  title?: string;
  /** The name of a page that ships with Deckox, translated. */
  titleKey?: string;
  kind: PageKind;
  /** Rows a `screen` page is divided into. */
  rows: number;
  widgets: WidgetPlacement[];
}

export interface Layout {
  version: 1;
  pages: PageLayout[];
}

export const GRID_COLUMNS = 12;
export const MIN_SCREEN_ROWS = 4;
export const MAX_SCREEN_ROWS = 24;
export const DEFAULT_SCREEN_ROWS = 8;
export const MAX_ROWS = 24;
export const WIDTH_CHOICES = [2, 3, 4, 6, 8, 12] as const;
