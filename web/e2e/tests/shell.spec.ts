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
    await page.getByRole("menuitem", { name: "Theme" }).click();
    await page.getByRole("menuitemradio", { name: "Light" }).click();
    await expect(html).toHaveAttribute("data-theme", "light");
    await expect(html).toHaveAttribute("data-palette", "cinema");
    await expect(html).not.toHaveClass(/dark/);

    await page.reload();
    await expect(html).toHaveAttribute("data-theme", "light");

    // Put the browser back on the theme it arrived with.
    await page.getByRole("button", { name: "Account" }).click();
    await page.getByRole("menuitem", { name: "Theme" }).click();
    await page.getByRole("menuitemradio", { name: original === "light" ? "Light" : "Dark" }).click();
    await expect(html).toHaveAttribute("data-theme", original);
  });

  test("offers only colour mode and blur in the settings picker", async ({ page }) => {
    await gotoPage(page, "/settings/general", "Settings");
    const html = page.locator("html");
    await page.getByRole("button", { name: "Light", exact: true }).click();
    await expect(html).toHaveAttribute("data-mode", "light");
    await page.getByRole("button", { name: "Dark", exact: true }).click();
    await expect(html).toHaveAttribute("data-mode", "dark");
    await expect(html).toHaveAttribute("data-palette", "cinema");

    const blur = page.getByRole("switch", { name: "Blur" });
    await blur.click();
    await expect(html).toHaveAttribute("data-glass-rendering", "none");
    await blur.click();
    await expect(html).not.toHaveAttribute("data-glass-rendering", /.+/);
    await expect(page.getByText(/corner radius|interface size|density/i)).toHaveCount(0);
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
