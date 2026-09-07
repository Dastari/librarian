import { expect, type Page } from "@playwright/test";

import { CREDENTIALS } from "../../playwright.config";

/**
 * Signs in over the API, which is all the app needs: credentials live in HttpOnly cookies and
 * `page.request` shares the browser context's jar. Refresh tokens rotate on every use, so a
 * saved `storageState` goes stale after one test — each test signs in for itself instead.
 */
export async function signInViaApi(page: Page, credentials = CREDENTIALS): Promise<void> {
  const response = await page.request.post("/graphql", {
    data: {
      query: "mutation E2eLogin($input: LoginInput!) { login(input: $input) { success error } }",
      variables: { input: { usernameOrEmail: credentials.username, password: credentials.password } },
    },
  });
  const body = (await response.json()) as { data?: { login?: { success: boolean; error: string | null } } };
  expect(body.data?.login?.success, body.data?.login?.error ?? "sign-in failed").toBe(true);
}

/** Signs in through the form and waits for the shell to take over. */
export async function signIn(page: Page, credentials = CREDENTIALS): Promise<void> {
  await page.goto("/login");
  await page.getByLabel("Username or email").fill(credentials.username);
  await page.getByLabel("Password").fill(credentials.password);
  await page.getByRole("button", { name: "Sign in" }).click();
  await expect(page).toHaveURL(/\/$|\/\?/);
  await expect(page.getByRole("button", { name: "Account" })).toBeVisible();
}

/** Opens a page and waits for its `<h1>`, which every route renders once its data lands. */
export async function gotoPage(page: Page, path: string, heading: string | RegExp): Promise<void> {
  await page.goto(path);
  // The dev server compiles each route chunk on its first request, so give the heading room.
  await expect(page.getByRole("heading", { level: 1, name: heading })).toBeVisible({ timeout: 25_000 });
}

/** Home has no page heading of its own; the TV guide is the first thing it renders. */
export async function gotoHome(page: Page): Promise<void> {
  await page.goto("/");
  await expect(page.getByRole("heading", { name: "TV guide" })).toBeVisible({ timeout: 25_000 });
}

/** Waits for a HeroUI toast by its message. */
export async function expectToast(page: Page, message: string | RegExp): Promise<void> {
  await expect(page.getByText(message).first()).toBeVisible();
}

/** The first TV library card on /libraries, or null when the sandbox has no TV library. */
export async function openFirstLibrary(page: Page, type?: RegExp): Promise<string | null> {
  await gotoPage(page, "/libraries", "Libraries");
  const cards = page.locator('a[href^="/libraries/"]');
  // Wait for the grid before counting, or an empty count reads as "no library of that kind".
  await expect(cards.first()).toBeVisible();
  const count = await cards.count();
  for (let index = 0; index < count; index += 1) {
    const card = cards.nth(index);
    if (type && !type.test((await card.innerText()).trim())) continue;
    const name = (await card.locator("h2, h3, p").first().innerText()).trim();
    await card.click();
    await expect(page).toHaveURL(/\/libraries\/[^/]+/);
    return name;
  }
  return null;
}
