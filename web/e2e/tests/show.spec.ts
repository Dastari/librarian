import { expect, test, type Page } from "@playwright/test";

import { openFirstLibrary, signInViaApi } from "./helpers";

test.beforeEach(async ({ page }) => {
  await signInViaApi(page);
});

/** Opens the first show in the first TV library, or skips when the sandbox has none. */
async function openFirstShow(page: Page): Promise<void> {
  const library = await openFirstLibrary(page, /Shows|Episodes|TV/i);
  test.skip(library === null, "the sandbox has no TV library");
  const shows = page.locator('a[href^="/shows/"]');
  await expect(shows.first()).toBeVisible();
  await shows.first().click();
  await expect(page).toHaveURL(/\/shows\/[0-9a-f-]{36}/);
  await expect(page.getByRole("tablist", { name: "Seasons" })).toBeVisible();
}

test.describe("Show page", () => {
  test("lists the seasons and their episodes", async ({ page }) => {
    await openFirstShow(page);
    const seasons = page.getByRole("tablist", { name: "Seasons" });
    const tabs = seasons.getByRole("tab");
    await expect(tabs.first()).toBeVisible();
    // Each season heading says how much of it is on disk.
    await expect(page.getByText(/\d+ of \d+/).first()).toBeVisible();
    if ((await tabs.count()) > 1) {
      await tabs.nth(1).click();
      await expect(tabs.nth(1)).toHaveAttribute("aria-selected", "true");
    }
    await expect(page.getByRole("heading", { level: 3, name: /Season \d+|Specials/ }).first()).toBeVisible();
    await expect(page.getByRole("heading", { name: "Details" })).toBeVisible();
  });

  test("changes the download mode from the acquisition dialog and puts it back", async ({ page }) => {
    await openFirstShow(page);
    const chip = page.getByRole("button", { name: /^Download settings — / });
    const original = (await chip.getAttribute("aria-label"))!.replace("Download settings — ", "");

    await chip.click();
    const dialog = page.getByRole("dialog");
    await expect(dialog).toBeVisible();
    await expect(dialog.getByText("Automatic downloads")).toBeVisible();
    // The dialog opens on the mode the show is already in.
    const modes = { "Auto-download off": "Off", "Auto-download wanted": "Wanted only", "Auto-download all": "Everything missing" } as const;
    const current = modes[original as keyof typeof modes];
    await expect(dialog.getByRole("radio", { name: current })).toBeChecked();
    // The quality picker names the library's own profile so "inherit" is never a mystery.
    await expect(dialog.getByRole("button", { name: /Inherit from library \(.+\)/ })).toBeVisible();

    const target = current === "Everything missing" ? "Wanted only" : "Everything missing";
    await dialog.getByRole("radio", { name: target }).click();
    await dialog.getByRole("button", { name: "Save" }).click();
    await expect(page.getByText("Download settings saved")).toBeVisible();
    await expect(dialog).toBeHidden();
    await expect(chip).not.toHaveAttribute("aria-label", `Download settings — ${original}`);

    // Put the show back the way it was; the sandbox is shared with the screenshot workflow.
    await chip.click();
    await expect(page.getByRole("dialog").getByRole("radio", { name: current })).toBeVisible();
    await page.getByRole("dialog").getByRole("radio", { name: current }).click();
    await page.getByRole("dialog").getByRole("button", { name: "Save" }).click();
    await expect(page.getByText("Download settings saved")).toBeVisible();
    await expect(chip).toHaveAttribute("aria-label", `Download settings — ${original}`);
  });

  test("closes the acquisition dialog without saving", async ({ page }) => {
    await openFirstShow(page);
    const chip = page.getByRole("button", { name: /^Download settings — / });
    const before = await chip.getAttribute("aria-label");
    await chip.click();
    const dialog = page.getByRole("dialog");
    await dialog.getByRole("radio", { name: "Off" }).click();
    await dialog.getByRole("button", { name: "Cancel" }).click();
    await expect(dialog).toBeHidden();
    await expect(chip).toHaveAttribute("aria-label", before!);
  });

  test("offers the header actions an admin needs", async ({ page }) => {
    await openFirstShow(page);
    await expect(page.getByRole("button", { name: "Download settings", exact: true })).toBeVisible();
    await expect(page.getByRole("button", { name: "Refresh metadata" })).toBeVisible();
    await expect(page.getByRole("button", { name: "Remove from library" })).toBeVisible();
  });
});

test.describe("Finding releases", () => {
  /** A canned SearchSources answer, so the suite never talks to a real indexer. */
  const RELEASES = [
    {
      __typename: "SourceRelease",
      title: "Sandbox.Show.S01E01.2160p.WEB-DL.HEVC-NTb",
      guid: "sandbox-1",
      link: "https://example.invalid/one.torrent",
      magnetUri: null,
      infoHash: null,
      details: "https://example.invalid/one",
      publishDate: "2026-09-01T00:00:00Z",
      categories: ["TV"],
      size: 6000000000,
      sizeFormatted: "5.6 GB",
      seeders: 42,
      leechers: 3,
      peers: 45,
      grabs: 100,
      isFreeleech: true,
      imdbId: null,
      poster: null,
      description: null,
      sourceId: "sandbox",
      sourceName: "Sandbox Tracker",
      profileMatch: "optimal",
      rejectReasons: [],
      parsed: { __typename: "ParsedRelease", resolution: "2160p", codec: "hevc", hdrType: "hdr10", sourceType: "web", audio: "atmos", releaseGroup: "NTb", languages: ["en"], isSeasonPack: false, isProper: false, isRepack: false, season: 1, episodes: [1], year: null },
    },
    {
      __typename: "SourceRelease",
      title: "Sandbox.Show.S01E01.480p.CAM-BADGRP",
      guid: "sandbox-2",
      link: "https://example.invalid/two.torrent",
      magnetUri: null,
      infoHash: null,
      details: "https://example.invalid/two",
      publishDate: "2026-09-01T00:00:00Z",
      categories: ["TV"],
      size: 300000000,
      sizeFormatted: "286 MB",
      seeders: 1,
      leechers: 0,
      peers: 1,
      grabs: 1,
      isFreeleech: false,
      imdbId: null,
      poster: null,
      description: null,
      sourceId: "sandbox",
      sourceName: "Sandbox Tracker",
      profileMatch: "rejected",
      rejectReasons: ["Below the minimum size", "Blocked release group"],
      parsed: { __typename: "ParsedRelease", resolution: "480p", codec: null, hdrType: null, sourceType: "cam", audio: null, releaseGroup: "BADGRP", languages: [], isSeasonPack: false, isProper: false, isRepack: false, season: 1, episodes: [1], year: null },
    },
  ];

  /** Answers only `SearchSources`; everything else still goes to the sandbox backend. */
  async function stubSearch(page: import("@playwright/test").Page) {
    await page.route("**/graphql", async (route) => {
      const body = route.request().postDataJSON() as { operationName?: string } | null;
      if (body?.operationName !== "SearchSources") return route.fallback();
      await route.fulfill({
        contentType: "application/json",
        body: JSON.stringify({
          data: {
            searchSources: {
              __typename: "SearchSourcesResult",
              totalReleases: RELEASES.length,
              totalElapsedMs: 120,
              sourcesSearched: 1,
              sources: [{ __typename: "SourceSearchResult", sourceId: "sandbox", sourceName: "Sandbox Tracker", elapsedMs: 120, fromCache: false, error: null, releases: RELEASES }],
            },
          },
        }),
      });
    });
  }

  test("lists releases with their tags and filters them", async ({ page }) => {
    await stubSearch(page);
    await openFirstShow(page);
    await page.getByRole("button", { name: "Find releases" }).click();
    const dialog = page.getByRole("dialog");
    await expect(dialog.getByText("Sandbox.Show.S01E01.2160p.WEB-DL.HEVC-NTb")).toBeVisible();
    await expect(dialog.getByText("2160p").first()).toBeVisible();
    await expect(dialog.getByText("HDR10").first()).toBeVisible();
    await expect(dialog.getByText("Freeleech").first()).toBeVisible();
    await expect(dialog.getByText("Optimal")).toBeVisible();
    await expect(dialog.getByText("Rejected")).toBeVisible();

    const resolutions = dialog.getByRole("radiogroup", { name: "Resolution" });
    await resolutions.getByRole("radio", { name: "2160p" }).click();
    await expect(dialog.getByText("1 of 2")).toBeVisible();
    await expect(dialog.getByText("Sandbox.Show.S01E01.480p.CAM-BADGRP")).toBeHidden();
    await resolutions.getByRole("radio", { name: "Any" }).click();
    await expect(dialog.getByText("2 of 2")).toBeVisible();
  });

  test("explains why a release was rejected", async ({ page }) => {
    await stubSearch(page);
    await openFirstShow(page);
    await page.getByRole("button", { name: "Find releases" }).click();
    const dialog = page.getByRole("dialog");
    const chip = dialog.getByRole("button", { name: /^Why rejected:/ });
    await expect(chip).toBeVisible();
    await chip.hover();
    await expect(page.getByText("Below the minimum size · Blocked release group")).toBeVisible();
  });
});
