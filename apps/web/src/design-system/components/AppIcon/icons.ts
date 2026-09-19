export const ICON_PATHS = {
  close: ["M6 6l12 12", "M18 6L6 18"],
  "chevron-left": ["M15 6l-6 6 6 6"],
  "chevron-right": ["M9 6l6 6-6 6"],
  "chevron-down": ["M6 9l6 6 6-6"],
  "chevron-up": ["M6 15l6-6 6 6"],
  check: ["M5 12.5l4.5 4.5L19 7.5"],
  plus: ["M12 5v14", "M5 12h14"],
  minus: ["M5 12h14"],
  alert: ["M12 4l9 16H3z", "M12 10v4", "M12 17.5v.01"],
  info: ["M12 21a9 9 0 100-18 9 9 0 000 18z", "M12 11v5", "M12 7.5v.01"],
  refresh: ["M4 12a8 8 0 0113.7-5.7L20 9", "M20 4v5h-5", "M20 12a8 8 0 01-13.7 5.7L4 15", "M4 20v-5h5"],
  search: ["M11 18a7 7 0 100-14 7 7 0 000 14z", "M20 20l-4-4"],
  trash: ["M4 7h16", "M9 7V4h6v3", "M6 7l1 13h10l1-13", "M10 11v6", "M14 11v6"],
  external: ["M14 4h6v6", "M20 4l-9 9", "M18 14v5H5V6h5"],
} as const;

export type IconName = keyof typeof ICON_PATHS;
