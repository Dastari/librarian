import { expect, test } from "@playwright/test";

import { gotoHome, gotoPage, signInViaApi } from "./helpers";

test.beforeEach(async ({ page }) => {
  await signInViaApi(page);
});

test.describe("Theme", () => {
  test("switches theme from the account menu and remembers it across a reload", async ({ page }) => {
    await gotoHome(page);
    const html = page.locator("html");
    const original = (await html.getAttribute("data-theme")) ?? "dark";

    await page.getByRole("button", { name: "Account" }).click();
    await page.getByRole("menuitem", { name: "Daylight" }).click();
    await expect(html).toHaveAttribute("data-theme", "light");
    await expect(html).not.toHaveClass(/dark/);

    await page.reload();
    await expect(html).toHaveAttribute("data-theme", "light");

    // Put the browser back on the theme it arrived with.
    await page.getByRole("button", { name: "Account" }).click();
    await page.getByRole("menuitem", { name: "Cinema" }).click();
    await expect(html).toHaveAttribute("data-theme", original);
  });

  test("switches theme from the settings picker", async ({ page }) => {
    await gotoPage(page, "/settings/general", "Settings");
    const themes = page.getByRole("radiogroup", { name: "Theme" });
    await themes.getByRole("radio", { name: "Midnight" }).click();
    await expect(page.locator("html")).toHaveAttribute("data-theme", "midnight");
    await themes.getByRole("radio", { name: "Cinema" }).click();
    await expect(page.locator("html")).toHaveAttribute("data-theme", "dark");
  });
});

test.describe("Phone layout", () => {
  test.use({ viewport: { width: 390, height: 844 } });

  test("swaps the rail for the bottom tab bar", async ({ page }) => {
    await gotoHome(page);
    // Both the rail and the bottom tabs are "Primary" navigation; only one is visible at a time.
    const tabs = page.getByRole("navigation", { name: "Primary" });
    await expect(tabs).toBeVisible();
    await expect(page.locator("aside nav")).toBeHidden();
    for (const label of ["Home", "Libraries", "Search", "Wanted", "Downloads", "Settings"]) {
      await expect(tabs.getByRole("link", { name: label })).toBeVisible();
    }
  });

  test("navigates from the tab bar and marks the current tab", async ({ page }) => {
    await gotoHome(page);
    const tabs = page.getByRole("navigation", { name: "Primary" });
    await tabs.getByRole("link", { name: "Wanted" }).click();
    await expect(page.getByRole("heading", { level: 1, name: "Wanted" })).toBeVisible();
    await expect(tabs.getByRole("link", { name: "Wanted" })).toHaveAttribute("aria-current", "page");
    await tabs.getByRole("link", { name: "Home" }).click();
    await expect(page.getByRole("heading", { name: "TV guide" })).toBeVisible();
  });
});
