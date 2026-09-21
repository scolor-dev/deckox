import { defineAsyncComponent, type Component } from "vue";
import type { WidgetDefinition } from "./types";

const cache = new Map<string, Component>();

/** The widget's component, loaded on first use and kept for every placement. */
export function componentOf(definition: WidgetDefinition): Component {
  let component = cache.get(definition.id);
  if (!component) {
    component = defineAsyncComponent(definition.component);
    cache.set(definition.id, component);
  }
  return component;
}
