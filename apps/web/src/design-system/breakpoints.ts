// CSS custom properties can't be used inside @media conditions, so unlike
// the other scales this one cannot live in tokens.css. This is the single
// source of truth instead: components' `@media (max-width: …)` must use one
// of these values (enforced by breakpoints.test.ts), and script that needs
// the same cut-off (matchMedia) imports it from here.
export const breakpoints = {
  /** Sidebar collapses to a top bar; tables stick their first column. */
  mobile: 700,
} as const;

export const mobileQuery = `(max-width: ${String(breakpoints.mobile)}px)`;
