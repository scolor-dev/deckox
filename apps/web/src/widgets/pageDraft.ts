import type { PageKind } from "./types";

/** What the page dialog edits. */
export interface PageDraft {
  title: string;
  kind: PageKind;
  rows: number;
}
