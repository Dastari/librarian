import { expect, test } from "@playwright/test";

import { gotoHome, gotoPage, openFirstLibrary, signInViaApi } from "./helpers";

test.beforeEach(async ({ page }) => {
  await signInViaApi(page);
});

test.describe("Libraries", () => {
  test("lists the libraries and opens one", async ({ page }) => {
    await gotoPage(page, "/libraries", "Libraries");
    const cards = page.locator('a[href^="/libraries/"]');
    await expect(cards.first()).toBeVisible();
    const name = (await cards.first().innerText()).split("\n")[0]!.trim();
    await cards.first().click();
    await expect(page).toHaveURL(/\/libraries\/[0-9a-f-]{36}/);
    await expect(page.getByRole("heading", { level: 1, name })).toBeVisible();
    // Every library exposes its sections, whatever kind of media it holds.
    await expect(page.getByRole("tablist", { name: "Library sections" }).or(page.getByRole("link", { name: /Files/ }))).toBeVisible();
  });

  test("opens a show from the TV library", async ({ page }) => {
    const opened = await openFirstLibrary(page, /Shows|Episodes|TV/i);
    test.skip(opened === null, "the sandbox has no TV library");
    const shows = page.locator('a[href^="/shows/"]');
    await expect(shows.first()).toBeVisible();
    await shows.first().click();
    await expect(page).toHaveURL(/\/shows\/[0-9a-f-]{36}/);
    await expect(page.getByRole("heading", { level: 1 })).toBeVisible();
    await expect(page.getByRole("tablist", { name: "Seasons" })).toBeVisible();
    // A season is selected on arrival, so there is always an episode list under the tabs.
    await expect(page.getByRole("list").first()).toBeVisible();
  });
});

test.describe("Wanted", () => {
  test("moves between the four tabs", async ({ page }) => {
    await gotoPage(page, "/wanted", "Wanted");
    const tabs = page.getByRole("tablist", { name: "Wanted filter" });
    await expect(tabs).toBeVisible();
    for (const label of ["Missing", "Downloading", "Upgradable", "Wanted"]) {
      await tabs.getByRole("tab", { name: label }).click();
      await expect(tabs.getByRole("tab", { name: label })).toHaveAttribute("aria-selected", "true");
      // Either rows or the tab's empty state — both mean the query came back.
      await expect(page.getByRole("table").or(page.getByText(/Nothing|No /).first())).toBeVisible();
    }
    // "wanted" is the default tab, so nuqs drops it from the URL; another tab is written out.
    await tabs.getByRole("tab", { name: "Missing" }).click();
    await expect(page).toHaveURL(/show=missing/);
  });
});

test.describe("Downloads", () => {
  test("shows the queue and its filters", async ({ page }) => {
    await gotoPage(page, "/downloads", "Downloads");
    const filters = page.getByRole("radiogroup", { name: "Download filter" });
    await expect(filters).toBeVisible();
    await filters.getByRole("radio", { name: "Active" }).click();
    await expect(page.getByRole("table").or(page.getByText(/No downloads|Nothing active/))).toBeVisible();
    await filters.getByRole("radio", { name: "Seeding" }).click();
    await expect(page.getByRole("table").or(page.getByText(/No downloads|Nothing seeding/))).toBeVisible();
    await filters.getByRole("radio", { name: "All" }).click();
    await expect(page.getByRole("table").or(page.getByText("No downloads"))).toBeVisible();
  });
});

test.describe("Search", () => {
  test("searches the library and clears again", async ({ page }) => {
    await gotoPage(page, "/search", "Search");
    const field = page.getByRole("searchbox", { name: "Search" }).or(page.getByPlaceholder("Titles, people, albums…"));
    await expect(page.getByText("Search your library")).toBeVisible();
    await field.fill("a");
    // One character is below the threshold, so the prompt stays put.
    await expect(page.getByText("Search your library")).toBeVisible();
    await field.fill("the");
    await expect(page.getByText("Search your library")).toBeHidden();
    await field.fill("");
    await expect(page.getByText("Search your library")).toBeVisible();
  });

  test("says so when nothing matches", async ({ page }) => {
    await gotoPage(page, "/search", "Search");
    await page.getByPlaceholder("Titles, people, albums…").fill("zzzqqqxxnothing");
    await expect(page.getByText(/Nothing in your library matches/)).toBeVisible();
  });
});

test.describe("Navigation rail", () => {
  /*
   * BUG: the collapsed rail (the default) renders each primary link as a bare icon with no
   * `aria-label` and no visible text, so screen readers announce seven unnamed links. The
   * `Tooltip` beside each one only supplies a visual label — and HeroUI v3 needs a pressable
   * child, which a TanStack `Link` is not, so it never opens either. Unskip once the rail links
   * carry names.
   */
  test("names every primary destination", async ({ page }) => {
    await gotoHome(page);
    const rail = page.getByRole("navigation", { name: "Primary" });
    for (const label of ["Home", "Libraries", "Search", "Wanted", "Downloads", "Activity", "Settings"]) {
      await expect(rail.getByRole("link", { name: label })).toBeVisible();
    }
  });

  test("moves between destinations from the rail", async ({ page }) => {
    await gotoHome(page);
    const rail = page.getByRole("navigation", { name: "Primary" });
    await rail.locator('a[href="/libraries"]').click();
    await expect(page.getByRole("heading", { level: 1, name: "Libraries" })).toBeVisible();
    await rail.locator('a[href="/downloads"]').click();
    await expect(page.getByRole("heading", { level: 1, name: "Downloads" })).toBeVisible();
  });
});
