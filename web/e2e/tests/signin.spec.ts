import { expect, test } from "@playwright/test";

import { CREDENTIALS } from "../../playwright.config";

import { signIn } from "./helpers";

test.describe("Signing in", () => {
  test("shows the constellation behind the sign-in card", async ({ page }) => {
    await page.goto("/login");
    await expect(page.getByRole("heading", { name: "Sign in" })).toBeVisible();
    await expect(page.locator("canvas")).toBeVisible();
    await expect(page.getByLabel("Username or email")).toBeFocused();
  });

  test("rejects the wrong password without leaving the page", async ({ page }) => {
    await page.goto("/login");
    await page.getByLabel("Username or email").fill(CREDENTIALS.username);
    await page.getByLabel("Password").fill("definitely-not-the-password");
    await page.getByRole("button", { name: "Sign in" }).click();
    await expect(page.getByText("Invalid username/email or password")).toBeVisible();
    await expect(page).toHaveURL(/\/login/);
  });

  test("asks for both fields", async ({ page }) => {
    await page.goto("/login");
    await page.getByRole("button", { name: "Sign in" }).click();
    await expect(page.getByText(/required|Enter your/i).first()).toBeVisible();
    await expect(page).toHaveURL(/\/login/);
  });

  test("lands on Home with the TV guide, then signs out again", async ({ page }) => {
    await signIn(page);
    await expect(page.locator('section[aria-label="TV guide"]')).toBeVisible();
    await expect(page.getByRole("heading", { name: "TV guide" })).toBeVisible();

    await page.getByRole("button", { name: "Account" }).click();
    await page.getByRole("menuitem", { name: "Sign out" }).click();
    await expect(page).toHaveURL(/\/login/);
    await expect(page.getByRole("heading", { name: "Sign in" })).toBeVisible();

    // The session really is gone: a protected route bounces back to sign-in.
    await page.goto("/libraries");
    await expect(page).toHaveURL(/\/login/);
  });

  test("sends a signed-out visitor to sign-in and back where they were headed", async ({ page }) => {
    await page.goto("/settings/general");
    await expect(page).toHaveURL(/\/login\?redirect=/);
    await page.getByLabel("Username or email").fill(CREDENTIALS.username);
    await page.getByLabel("Password").fill(CREDENTIALS.password);
    await page.getByRole("button", { name: "Sign in" }).click();
    await expect(page).toHaveURL(/\/settings\/general/);
  });
});
