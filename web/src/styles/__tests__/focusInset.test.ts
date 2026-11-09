import { readFileSync } from "node:fs";
import { join } from "node:path";

import { describe, expect, it } from "vitest";

const css = (name: string) => readFileSync(join(__dirname, "..", name), "utf8");
const rule = (source: string, selector: string) => {
  const start = source.indexOf(`${selector} {`);
  expect(start, `${selector} is styled`).toBeGreaterThanOrEqual(0);
  return source.slice(start, source.indexOf("}", start));
};

describe("focus rings are never clipped by a scroller", () => {
  it("insets the rail's scrolling list by the focus reach, top and bottom, with scroll padding", () => {
    const list = rule(css("media.css"), ".app-shell-rail .fa-shell-rail-list");
    expect(list).toMatch(/padding-block:\s*max\(var\(--space-1\),\s*var\(--focus-reach\)\)/);
    expect(list).toMatch(/scroll-padding-block:\s*var\(--focus-reach\)/);
    // Padding, not a removed clip.
    expect(list).not.toMatch(/overflow:\s*visible/);
  });

  it("keeps the poster row's block padding at the focus reach", () => {
    expect(rule(css("utilities.css").replace("@utility snap-row", ".snap-row"), ".snap-row")).toMatch(/padding-block:\s*max\(0\.5rem,\s*var\(--focus-reach\)\)/);
  });

  it("pads the search page's term strip by the focus reach", () => {
    const strip = rule(css("media.css"), ".search-terms");
    expect(strip).toMatch(/padding:\s*var\(--focus-reach\)/);
    expect(strip).toMatch(/scroll-padding-inline:\s*var\(--focus-reach\)/);
  });
});
