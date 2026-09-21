import { computed } from "vue";
import { widgetAvailable } from "../modules/store";
import type { PageLayout } from "../widgets/types";
import { pageShown } from "./model";
import { layout } from "./store";

/** The pages to offer in the navigation: those with something to show. */
export const navPages = computed(() => layout.value.pages.filter((page) => pageShown(page, widgetAvailable)));

export function pageTitle(page: PageLayout, translate: (key: string) => string): string {
  if (page.title !== undefined) return page.title;
  return page.titleKey === undefined ? page.id : translate(page.titleKey);
}
