import "@testing-library/jest-dom/vitest";

import { afterEach, vi } from "vitest";

/*
 * jsdom leaves out most of the layout and observer APIs the UI kit relies on. These are the
 * smallest stand-ins that let components mount and behave deterministically in tests; nothing
 * here changes what the components do in a browser. Pure modules opt out of jsdom with a
 * `// @vitest-environment node` docblock, so everything below is guarded.
 */

class MockObserver {
  observe(): void {}
  unobserve(): void {}
  disconnect(): void {}
  takeRecords(): [] {
    return [];
  }
}

if (typeof window !== "undefined") {
  if (!("ResizeObserver" in globalThis)) {
    Object.defineProperty(globalThis, "ResizeObserver", { configurable: true, writable: true, value: MockObserver });
  }

  if (!("IntersectionObserver" in globalThis)) {
    Object.defineProperty(globalThis, "IntersectionObserver", {
      configurable: true,
      writable: true,
      value: class extends MockObserver {
        readonly root = null;
        readonly rootMargin = "";
        readonly thresholds: number[] = [];
      },
    });
  }

  if (typeof window.matchMedia !== "function") {
    Object.defineProperty(window, "matchMedia", {
      configurable: true,
      writable: true,
      value: (query: string) => ({
        matches: false,
        media: query,
        onchange: null,
        addEventListener: () => {},
        removeEventListener: () => {},
        addListener: () => {},
        removeListener: () => {},
        dispatchEvent: () => false,
      }),
    });
  }

  if (!Element.prototype.scrollIntoView) Element.prototype.scrollIntoView = () => {};
  if (!Element.prototype.hasPointerCapture) Element.prototype.hasPointerCapture = () => false;
  if (!Element.prototype.setPointerCapture) Element.prototype.setPointerCapture = () => {};
  if (!Element.prototype.releasePointerCapture) Element.prototype.releasePointerCapture = () => {};
  // jsdom logs "Not implemented" for these; the components only use them for polish.
  Object.defineProperty(window, "scrollTo", { configurable: true, writable: true, value: () => {} });
  if (!("PointerEvent" in window)) Object.defineProperty(window, "PointerEvent", { writable: true, value: MouseEvent });
}

afterEach(async () => {
  if (typeof window !== "undefined") {
    const { cleanup } = await import("@testing-library/react");
    cleanup();
    // The HeroUI toast queue lives outside React, so it survives unmounting.
    const { toast } = await import("@heroui/react");
    toast.clear();
    window.localStorage.clear();
    window.sessionStorage.clear();
  }
  vi.clearAllMocks();
});
