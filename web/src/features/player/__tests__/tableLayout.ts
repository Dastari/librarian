import { afterEach, beforeEach, vi } from "vitest";

/**
 * jsdom has no layout. Gives the real Glass table the dimensions it needs to draw its rows, while
 * keeping its row renderers, reorder grips and action runner.
 */
export function useTableLayout(width = 1200) {
  beforeEach(() => {
    vi.spyOn(HTMLElement.prototype, "getBoundingClientRect").mockImplementation(function (this: HTMLElement) {
      const measured = this.hasAttribute("data-measure") ? 110 : width;
      return { x: 0, y: 0, top: 0, left: 0, bottom: 800, right: measured, width: measured, height: 800, toJSON: () => ({}) };
    });
    vi.spyOn(HTMLElement.prototype, "offsetWidth", "get").mockImplementation(function (this: HTMLElement) { return this.classList.contains("fa-dt-probe") ? 32 : width; });
    vi.spyOn(HTMLElement.prototype, "offsetHeight", "get").mockImplementation(function (this: HTMLElement) { return this.classList.contains("fa-dt-probe") ? 48 : 800; });
    vi.spyOn(HTMLElement.prototype, "clientWidth", "get").mockImplementation(() => width);
    vi.spyOn(HTMLElement.prototype, "clientHeight", "get").mockReturnValue(800);
  });
  afterEach(() => vi.restoreAllMocks());
}
