#!/usr/bin/env node
/**
 * Screenshot helper for design review.
 *
 *   node e2e/shoot.mjs <name> <path> [--viewport=1440x900|phone|tablet|tv] [--login] [--wait=<selector>] [--full]
 *                                    [--click=<button text>] [--motion] [--settle=<ms>] [--theme=<name>]
 *
 * Signs in with the credentials in LIBRARIAN_E2E_USER / LIBRARIAN_E2E_PASSWORD when --login is set,
 * navigates to the path on the dev server and writes screenshots/<name>.png.
 */
import { mkdirSync } from "node:fs";
import { resolve } from "node:path";

import { chromium } from "@playwright/test";

const args = process.argv.slice(2);
const name = args[0];
const path = args[1] ?? "/";
const options = Object.fromEntries(args.slice(2).filter((a) => a.startsWith("--")).map((a) => {
  const [key, value] = a.slice(2).split("=");
  return [key, value ?? true];
}));

const VIEWPORTS = {
  desktop: { width: 1440, height: 900 },
  wide: { width: 1920, height: 1080 },
  phone: { width: 390, height: 844, mobile: true },
  tablet: { width: 1024, height: 768 },
  tv: { width: 1920, height: 1080 },
};

const baseUrl = process.env.E2E_BASE_URL ?? "http://127.0.0.1:3002";
const viewportKey = typeof options.viewport === "string" ? options.viewport : "desktop";
const custom = /^(\d+)x(\d+)$/.exec(viewportKey);
const viewport = custom ? { width: Number(custom[1]), height: Number(custom[2]) } : (VIEWPORTS[viewportKey] ?? VIEWPORTS.desktop);

const browser = await chromium.launch();
const context = await browser.newContext({
  viewport: { width: viewport.width, height: viewport.height },
  deviceScaleFactor: 1,
  isMobile: Boolean(viewport.mobile),
  hasTouch: Boolean(viewport.mobile),
  colorScheme: "dark",
  reducedMotion: options.motion ? "no-preference" : "reduce",
});
const page = await context.newPage();
const errors = [];
page.on("pageerror", (error) => errors.push(`pageerror: ${error.message}`));
page.on("console", (message) => {
  if (message.type() === "error") errors.push(`console: ${message.text()}`);
});

if (options.login) {
  await page.goto(`${baseUrl}/login`, { waitUntil: "networkidle" });
  await page.getByLabel(/username or email/i).fill(process.env.LIBRARIAN_E2E_USER ?? "");
  await page.getByLabel(/^password/i).fill(process.env.LIBRARIAN_E2E_PASSWORD ?? "");
  await page.getByRole("button", { name: /sign in/i }).click();
  await page.waitForURL((url) => !url.pathname.startsWith("/login"), { timeout: 15000 });
}

await page.goto(`${baseUrl}${path}`, { waitUntil: "networkidle" });
if (typeof options.wait === "string") await page.waitForSelector(options.wait, { timeout: 15000 });
if (options["tv-mode"]) {
  await page.evaluate(() => {
    localStorage.setItem("librarian.inputMode", "tv");
  });
  await page.reload({ waitUntil: "networkidle" });
}
if (typeof options.theme === "string") {
  await page.evaluate((theme) => localStorage.setItem("librarian.theme", theme), options.theme);
  await page.reload({ waitUntil: "networkidle" });
}
if (typeof options.click === "string") {
  await page.getByText(options.click, { exact: true }).first().click();
  await page.waitForLoadState("networkidle");
}
await page.waitForTimeout(Number(options.settle ?? 800));

mkdirSync(resolve("screenshots"), { recursive: true });
const file = resolve("screenshots", `${name}.png`);
await page.screenshot({ path: file, fullPage: Boolean(options.full) });
console.log(`wrote ${file}`);
if (errors.length) {
  console.log("browser errors:");
  for (const error of errors) console.log(`  ${error}`);
}
await browser.close();
