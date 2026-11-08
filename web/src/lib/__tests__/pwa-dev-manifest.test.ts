import { readFileSync } from "node:fs";
import { resolve } from "node:path";

import { describe, expect, it } from "vitest";

/**
 * The live site on :3000 is the Vite dev server. If it does not serve the web manifest, the SPA
 * fallback answers /manifest.webmanifest with index.html and a Home Screen install gets no
 * name, icons, scope or standalone display. Keep the dev manifest on, and the dev worker off.
 */
describe("dev server web manifest", () => {
  const config = readFileSync(resolve(process.cwd(), "vite.config.ts"), "utf8");

  it("serves the manifest from the dev server", () => {
    expect(config).toMatch(/devOptions:\s*\{\s*enabled:\s*true\s*\}/);
    expect(config).toMatch(/display:\s*"standalone"/);
  });

  it("registers no service worker on the dev server", () => {
    const source = readFileSync(resolve(process.cwd(), "src/lib/pwa.ts"), "utf8");
    expect(source).toMatch(/if \(!import\.meta\.env\.PROD\) \{\s*void retireWorkers\(\);\s*return;/);
  });
});
