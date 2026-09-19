import { describe, expect, it } from "vitest";

const sfcModules = import.meta.glob("./components/*/*.vue", { query: "?raw", import: "default", eager: true });

describe("motion", () => {
  it("every component that animates opts out under prefers-reduced-motion", () => {
    const animated = Object.entries(sfcModules).filter(([, source]) => /transition|animation|@keyframes|--duration-/i.test(source));
    expect(animated.length).toBeGreaterThan(0);
    const missing = animated.filter(([, source]) => !source.includes("@media (prefers-reduced-motion: reduce)")).map(([path]) => path);
    expect(missing).toEqual([]);
  });
});
