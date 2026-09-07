import { expect, test } from "@playwright/test";

import { gotoPage, signInViaApi } from "./helpers";

test.beforeEach(async ({ page }) => {
  await signInViaApi(page);
});

/** Every settings route hangs off the same layout, so one heading proves the section loaded. */
const openSettings = async (page: import("@playwright/test").Page, path: string) => {
  await gotoPage(page, path, "Settings");
};

test.describe("Settings", () => {
  test("moves between the sections from the side nav", async ({ page }) => {
    await openSettings(page, "/settings/general");
    for (const [label, panel] of [
      ["Libraries", "Libraries"],
      ["Sources", "Sources"],
      ["Quality", "Quality profiles"],
      ["Downloads", "Automatic downloads"],
      ["General", "Appearance"],
    ] as const) {
      await page.getByRole("main").getByRole("link", { name: label, exact: true }).click();
      await expect(page.getByRole("heading", { name: panel })).toBeVisible();
    }
  });

  test("General offers the theme picker and the playback form", async ({ page }) => {
    await openSettings(page, "/settings/general");
    await expect(page.getByRole("radiogroup", { name: "Theme" })).toBeVisible();
    await expect(page.getByRole("textbox", { name: "Save progress every (seconds)" })).toBeVisible();
    await expect(page.getByRole("textbox", { name: /Metadata language/ })).toBeVisible();
    // Nothing is dirty on arrival, so there is nothing to save.
    await expect(page.getByRole("button", { name: "Save", exact: true })).toBeDisabled();
  });

  test("Libraries lists what the server knows about", async ({ page }) => {
    await openSettings(page, "/settings/libraries");
    await expect(page.getByRole("heading", { name: "Libraries" })).toBeVisible();
    await expect(page.getByRole("table").or(page.getByText("No libraries yet"))).toBeVisible();
    await expect(page.getByRole("button", { name: /Add library/ })).toBeVisible();
  });

  test("Sources lists the configured indexers", async ({ page }) => {
    await openSettings(page, "/settings/sources");
    await expect(page.getByRole("heading", { name: "Sources" })).toBeVisible();
    await expect(page.getByRole("table").or(page.getByText("No sources yet"))).toBeVisible();
  });

  test("Quality opens a profile in the editor and closes it again", async ({ page }) => {
    await openSettings(page, "/settings/quality");
    await expect(page.getByRole("heading", { name: "Quality profiles" })).toBeVisible();
    const rows = page.getByRole("row");
    test.skip((await rows.count()) < 2, "the sandbox has no quality profiles");

    await rows.nth(1).click();
    const dialog = page.getByRole("dialog");
    await expect(dialog).toBeVisible();
    await expect(dialog.getByText(/^Edit /)).toBeVisible();
    await expect(dialog.getByRole("textbox", { name: /^Name/ })).not.toHaveValue("");
    await expect(dialog.getByRole("textbox", { name: "Resolutions" }).or(dialog.getByRole("textbox", { name: "Audio formats" }))).toBeVisible();
    await expect(dialog.getByText("Preferred languages", { exact: true })).toBeVisible();
    await expect(dialog.getByText("Preferred release groups", { exact: true })).toBeVisible();
    await expect(dialog.getByRole("switch", { name: "Allow season packs" })).toBeVisible();
    // Leave without saving so the sandbox profile is untouched.
    await dialog.getByRole("button", { name: "Cancel" }).click();
    await expect(dialog).toBeHidden();
  });

  test("Quality opens a blank editor for a new profile", async ({ page }) => {
    await openSettings(page, "/settings/quality");
    await page.getByRole("button", { name: /New profile/ }).click();
    const dialog = page.getByRole("dialog");
    await expect(dialog.getByText("New quality profile")).toBeVisible();
    await expect(dialog.getByRole("textbox", { name: /^Name/ })).toHaveValue("");
    await dialog.getByRole("button", { name: "Cancel" }).click();
    await expect(dialog).toBeHidden();
  });

  test("Downloads shows the torrent, seeding and auto-download panels", async ({ page }) => {
    await openSettings(page, "/settings/downloads");
    for (const panel of ["Automatic downloads", "Torrent client", "Seeding"]) {
      await expect(page.getByRole("heading", { name: panel })).toBeVisible();
    }
    await expect(page.getByRole("textbox", { name: /Download folder/ })).not.toHaveValue("");
    await expect(page.getByRole("switch", { name: /Search and download automatically/ })).toBeVisible();
    await expect(page.getByRole("button", { name: /Run now/ })).toBeVisible();
    // The form arrives clean; the suite never writes server settings.
    await expect(page.getByRole("button", { name: "Save", exact: true })).toBeDisabled();
  });
});
