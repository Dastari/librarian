#!/usr/bin/env node
/** Real media acceptance against the disposable TestApp manifest; no media mocks.
 * node e2e/playback-browser.mjs /tmp/<acceptance-run>/ready.json
 * Requires the isolated Vite server on PLAYBACK_UI_URL (default :3003).
 */
import assert from 'node:assert/strict';
import { readFileSync, mkdirSync, writeFileSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { chromium, expect } from '@playwright/test';

const manifestPath = resolve(process.argv[2] ?? '');
const fixture = JSON.parse(readFileSync(manifestPath, 'utf8'));
assert.equal(fixture.version, 1);
const base = process.env.PLAYBACK_UI_URL ?? 'http://127.0.0.1:3003';
assert(['127.0.0.1', 'localhost'].includes(new URL(base).hostname), 'Disposable local server only');
const out = resolve(dirname(manifestPath), 'browser');
mkdirSync(out, { recursive: true });
const results = [];
const selectedCases = process.env.PLAYBACK_CASE ? new RegExp(process.env.PLAYBACK_CASE) : null;
const browser = await chromium.launch({ headless: true, args: ['--no-sandbox', '--autoplay-policy=no-user-gesture-required'] });
let page;
let pageErrors = [];
async function gql(query, variables = {}) {
  const response = await page.request.post(`${base}/graphql`, { headers: { Origin: base }, data: { query, variables } });
  assert.equal(response.status(), 200);
  const body = await response.json();
  assert(!body.errors, JSON.stringify(body.errors));
  return body.data;
}
async function login(viewport) {
  const context = await browser.newContext({ viewport });
  await context.tracing.start({ screenshots: true, snapshots: true });
  page = await context.newPage();
  pageErrors = [];
  page.on('pageerror', (error) => pageErrors.push(String(error.stack ?? error)));
  page.setDefaultTimeout(15000);
  const data = await gql('mutation($input:LoginInput!){login(input:$input){success error}}', { input: { usernameOrEmail: fixture.credentials.email, password: fixture.credentials.password } });
  assert.equal(data.login.success, true, data.login.error);
  return context;
}
async function check(name, run) {
  if (selectedCases && !selectedCases.test(name)) return;
  const errorCount = pageErrors.length;
  const started = Date.now();
  try {
    const detail = await run();
    assert.equal(pageErrors.length, errorCount, pageErrors.slice(errorCount).join('\n'));
    results.push({ name, passed: true, elapsedMs: Date.now() - started, detail });
    console.log(`PASS ${name}`);
  } catch (error) {
    await page.screenshot({ path: resolve(out, `${name.replaceAll(/[^a-z0-9]/gi, '-')}-failure.png`), fullPage: true }).catch(() => {});
    results.push({ name, passed: false, elapsedMs: Date.now() - started, error: String(error.stack ?? error) });
    console.log(`FAIL ${name}: ${error.message}`);
  }
  writeFileSync(resolve(out, 'results.json'), JSON.stringify({ qualification: 'Real Chromium media decode and API persistence; physical devices remain separate', results }, null, 2));
}
async function wake() { await page.mouse.move(90, 100); await page.mouse.move(120, 110); }
async function open(path) {
  await page.goto(`${base}${path}`, { waitUntil: 'domcontentloaded' });
  // A document load is not session readiness. Wait for the real authenticated
  // shell before mutating a fixture or starting another full-page navigation.
  await expect(page.getByRole('button', { name: 'Account', exact: true })).toBeVisible({ timeout: 30000 });
}
async function decoded(selector = 'video') {
  await page.waitForFunction((tag) => {
    const media = document.querySelector(tag);
    return media && media.readyState >= 3 && !media.paused && media.currentTime > 0.25 && (tag !== 'video' || media.getVideoPlaybackQuality().totalVideoFrames > 0);
  }, selector, { timeout: 30000 });
  return page.locator(selector).evaluate((m) => ({ position: m.currentTime, duration: m.duration, paused: m.paused, readyState: m.readyState, src: m.currentSrc, frames: m instanceof HTMLVideoElement ? m.getVideoPlaybackQuality().totalVideoFrames : undefined }));
}
async function movie(kind) {
  await open(`/movies/${fixture.movies[kind].id}`);
  await page.getByRole('button', { name: /^(Play|Resume)$/, exact: true }).first().click();
  await expect(page).toHaveURL(/\/watch\//);
}
async function progressRows(fileId) {
  return (await gql('query($where:PlaybackProgressWhereInput!){playbackProgresses(where:$where,page:{limit:100,offset:0}){edges{node{id mediaFileId currentPosition duration isWatched}}}}', { where: { mediaFileId: { eq: fileId }, userId: { eq: fixture.ownerId } } })).playbackProgresses.edges.map((e) => e.node);
}
async function seedPosition(fileId, position, duration) {
  const rows = await progressRows(fileId);
  for (const row of rows) await gql('mutation($id:String!){deletePlaybackProgress(id:$id){success error}}', { id: row.id });
  const data = await gql('mutation($input:CreatePlaybackProgressInput!){createPlaybackProgress(input:$input){success error}}', { input: { userId: fixture.ownerId, mediaFileId: fileId, currentPosition: position, duration, progressPercent: position / duration * 100, isWatched: false } });
  assert(data.createPlaybackProgress.success, data.createPlaybackProgress.error);
}

try {
  let context = await login({ width: 1440, height: 900 });
  for (const kind of ['direct', 'remux', 'transcode']) await check(`${kind} real video decode and keyboard seek`, async () => {
    await movie(kind);
    const before = await decoded();
    await wake();
    await page.getByRole('button', { name: 'Pause', exact: true }).click();
    const seek = page.getByRole('slider', { name: 'Seek', exact: true });
    await seek.focus(); await seek.press('Home'); await seek.press('ArrowRight'); await seek.press('ArrowRight');
    await expect.poll(() => page.locator('video').evaluate((m) => m.currentTime)).toBeGreaterThan(9.5);
    await expect.poll(() => page.locator('video').evaluate((m) => m.paused)).toBe(true);
    await page.getByRole('button', { name: 'Play', exact: true }).click();
    const after = await decoded();
    await page.screenshot({ path: resolve(out, `${kind}-desktop.png`) });
    return { before, after };
  });
  await check('subtitles and paused audio language replacement', async () => {
    await movie('direct'); await decoded(); await wake();
    await page.getByRole('button', { name: 'Pause', exact: true }).click();
    await page.getByRole('button', { name: 'Subtitles', exact: true }).click();
    const captions = page.getByRole('menuitemradio').filter({ hasText: /eng|English/i });
    await captions.first().click();
    await page.getByRole('slider', { name: 'Seek', exact: true }).press('Home');
    await page.getByRole('slider', { name: 'Seek', exact: true }).press('ArrowRight');
    await expect.poll(() => page.locator('video').evaluate((m) => Array.from(m.textTracks).some((t) => t.mode === 'showing' && Array.from(t.activeCues ?? []).some((c) => c.text.includes('Acceptance caption'))))).toBe(true);
    await page.screenshot({ path: resolve(out, 'subtitles-desktop.png') });
    await page.getByRole('button', { name: 'Subtitles', exact: true }).click();
    await page.getByRole('menuitemradio', { name: 'Off', exact: true }).click();
    await expect.poll(() => page.locator('video').evaluate((m) => Array.from(m.textTracks).every((t) => t.mode === 'disabled'))).toBe(true);
    await page.getByRole('button', { name: 'Audio track', exact: true }).click();
    const languageResponse = page.waitForResponse((r) => r.url().includes(`/api/media/${fixture.movies.direct.mediaFileId}/info?audioStream=2`));
    await page.getByRole('menuitemradio').filter({ hasText: /fra|French/i }).first().click();
    assert.equal((await languageResponse).status(), 200);
    await expect.poll(() => page.locator('video').evaluate((m) => m.readyState)).toBeGreaterThanOrEqual(3);
    await expect.poll(() => page.locator('video').evaluate((m) => m.currentTime)).toBeGreaterThan(4.5);
    assert(await page.locator('video').evaluate((m) => m.paused), 'Changing language must preserve pause');
    await page.getByRole('button', { name: 'Play', exact: true }).click();
    return decoded();
  });
  await check('episode Home resume identity and natural next', async () => {
    await open('/');
    const [first, second] = fixture.show.episodes;
    await seedPosition(first.mediaFileId, 9, 30);
    await page.reload({ waitUntil: 'domcontentloaded' });
    await expect(page.getByRole('button', { name: 'Account', exact: true })).toBeVisible({ timeout: 30000 });
    await page.getByRole('button').filter({ hasText: 'Acceptance Show' }).first().click();
    await expect(page).toHaveURL(new RegExp(`/watch/${first.mediaFileId}`));
    await decoded();
    assert(await page.locator('video').evaluate((m) => m.currentTime >= 8.5));
    await wake();
    await expect(page.getByRole('button', { name: /Next/ })).toBeVisible();
    await page.getByRole('slider', { name: 'Seek', exact: true }).press('End');
    await expect(page).toHaveURL(new RegExp(`/watch/${second.mediaFileId}`), { timeout: 20000 });
    await decoded();
    const data = await gql('query($where:PlaybackSessionWhereInput!){playbackSessions(where:$where,page:{limit:100,offset:0}){edges{node{mediaFileId episodeId currentPosition}}}}', { where: { userId: { eq: fixture.ownerId }, mediaFileId: { eq: first.mediaFileId } } });
    assert(data.playbackSessions.edges.some(({ node }) => node.episodeId === first.id));
    assert((await progressRows(first.mediaFileId)).some((row) => row.isWatched));
    return data.playbackSessions.edges;
  });
  await check('audio queue Next Previous and page navigation', async () => {
    await open(`/albums/${fixture.album.id}`);
    await page.getByRole('button', { name: /^(Play|Resume)$/, exact: true }).first().click();
    await decoded('audio');
    await page.getByRole('button', { name: 'Next', exact: true }).click();
    await expect.poll(() => page.locator('audio').evaluate((m) => m.currentSrc)).toContain(fixture.album.tracks[1].mediaFileId);
    await decoded('audio');
    await page.getByRole('button', { name: 'Previous', exact: true }).click();
    await expect.poll(() => page.locator('audio').evaluate((m) => m.currentSrc)).toContain(fixture.album.tracks[0].mediaFileId);
    await decoded('audio');
    await page.getByRole('link', { name: 'Home', exact: true }).first().click();
    return decoded('audio');
  });
  await check('audiobook resume and parent session identity', async () => {
    await open('/');
    await seedPosition(fixture.book.chapters[0].mediaFileId, 6, 12);
    await open(`/audiobooks/${fixture.book.id}`);
    await page.getByRole('button', { name: /^Resume/, exact: false }).first().click();
    await decoded('audio');
    assert(await page.locator('audio').evaluate((m) => m.currentTime >= 5.5));
    await page.getByRole('button', { name: 'Next', exact: true }).click();
    await expect.poll(() => page.locator('audio').evaluate((m) => m.currentSrc)).toContain(fixture.book.chapters[1].mediaFileId);
    const playing = await decoded('audio');
    assert(playing.position < 5, 'New chapter must not inherit old position');
    const data = await gql('query($where:PlaybackSessionWhereInput!){playbackSessions(where:$where,page:{limit:100,offset:0}){edges{node{mediaFileId audiobookId}}}}', { where: { userId: { eq: fixture.ownerId }, mediaFileId: { eq: fixture.book.chapters[0].mediaFileId } } });
    assert(data.playbackSessions.edges.some(({ node }) => node.audiobookId === fixture.book.id));
    return playing;
  });
  await check('rapid audio source replacement preserves file checkpoints', async () => {
    await open('/');
    const [a, b] = fixture.album.tracks;
    await seedPosition(a.mediaFileId, 2, 12);
    await seedPosition(b.mediaFileId, 7, 12);
    const pattern = `**/api/media/${b.mediaFileId}/info*`;
    let requested;
    const started = new Promise((resolve) => { requested = resolve; });
    await page.route(pattern, async (route) => {
      requested();
      await new Promise((resolve) => setTimeout(resolve, 700));
      await route.continue().catch(() => {}); // A source change legitimately aborts this request.
    });
    await open(`/albums/${fixture.album.id}`);
    await page.getByRole('button', { name: /^(Play|Resume)$/, exact: true }).first().click();
    await decoded('audio');
    await page.getByRole('button', { name: 'Next', exact: true }).click();
    await started;
    await page.getByRole('button', { name: 'Previous', exact: true }).click();
    await expect.poll(() => page.locator('audio').evaluate((m) => m.currentSrc)).toContain(a.mediaFileId);
    await decoded('audio');
    await page.getByRole('slider', { name: 'Seek', exact: true }).press('Home');
    await page.getByRole('slider', { name: 'Seek', exact: true }).press('ArrowRight');
    await page.getByRole('button', { name: 'Pause', exact: true }).click();
    await expect.poll(async () => (await progressRows(a.mediaFileId))[0]?.currentPosition).toBeGreaterThanOrEqual(5);
    const aRows = await progressRows(a.mediaFileId);
    const bRows = await progressRows(b.mediaFileId);
    assert.equal(aRows.length, 1);
    assert.equal(bRows.length, 1);
    assert.equal(bRows[0].currentPosition, 7, 'Aborted B must keep its own checkpoint');
    await page.unroute(pattern);
    return { a: aRows, b: bRows };
  });
  await check('missing file actionable error', async () => {
    await movie('missing');
    await expect(page.getByRole('alert')).toContainText(/missing|unavailable|not found/i);
    await expect(page.getByRole('button', { name: 'Retry playback', exact: true })).toBeVisible();
  });
  await check('source failure Retry recovers actual decode', async () => {
    const pattern = `**/api/media/${fixture.movies.direct.mediaFileId}/info*`;
    await page.route(pattern, (route) => route.fulfill({ status: 503, body: 'Acceptance temporary failure' }));
    await movie('direct');
    await expect(page.getByRole('button', { name: 'Retry playback', exact: true })).toBeVisible();
    await page.unroute(pattern);
    await page.getByRole('button', { name: 'Retry playback', exact: true }).click();
    return decoded();
  });
  await check('casting empty state and focus return', async () => {
    await movie('direct'); await decoded(); await wake();
    const trigger = page.getByRole('button', { name: 'Cast', exact: true });
    await trigger.click();
    const dialog = page.getByRole('dialog', { name: 'Casting', exact: true });
    await expect(dialog).toContainText('No receivers found');
    await expect(dialog.getByRole('button', { name: 'Start casting', exact: true })).toBeDisabled();
    await page.keyboard.press('Escape');
    await expect(trigger).toBeFocused();
  });
  await check('audio source failure and Retry recovers', async () => {
    const pattern = `**/api/media/${fixture.album.tracks[0].mediaFileId}/info*`;
    await page.route(pattern, (route) => route.fulfill({ status: 503, body: 'Acceptance temporary failure' }));
    await open(`/albums/${fixture.album.id}`);
    await page.getByRole('button', { name: /^(Play|Resume)$/, exact: true }).first().click();
    await expect(page.getByRole('alert')).toContainText(/unavailable|retry|failed/i);
    await page.unroute(pattern);
    await page.getByRole('button', { name: 'Retry', exact: true }).click();
    return decoded('audio');
  });
  await check('persistent HLS segment failure terminates and Retry recovers', async () => {
    const pattern = `**/api/media/${fixture.movies.remux.mediaFileId}/hls/*.ts*`;
    await page.route(pattern, (route) => route.fulfill({ status: 404, body: 'Acceptance unavailable segment' }));
    await movie('remux');
    await expect(page.getByRole('button', { name: 'Retry playback', exact: true })).toBeVisible({ timeout: 70000 });
    await page.unroute(pattern);
    await page.getByRole('button', { name: 'Retry playback', exact: true }).click();
    return decoded();
  });
  await context.tracing.stop({ path: resolve(out, 'desktop-trace.zip') });
  await context.close();
  for (const viewport of [{ width: 390, height: 844 }, { width: 834, height: 1112 }]) {
    context = await login(viewport);
    await check(`${viewport.width}px decoded player controls fit`, async () => {
      await movie('direct'); await decoded(); await wake();
      for (const name of ['Seek', 'Subtitles', 'Audio track', 'Fullscreen']) {
        const control = name === 'Seek' ? page.getByRole('slider', { name, exact: true }) : page.getByRole('button', { name, exact: true });
        await expect(control).toBeVisible();
        const box = await control.boundingBox();
        assert(box.x >= 0 && box.x + box.width <= viewport.width + 1, `${name} outside viewport`);
      }
      await page.screenshot({ path: resolve(out, `player-${viewport.width}.png`) });
      return { viewport, overflow: await page.evaluate(() => document.documentElement.scrollWidth > innerWidth) };
    });
    await check(`${viewport.width}px audio sheet queue and transport`, async () => {
      await open(`/albums/${fixture.album.id}`);
      await page.getByRole('button', { name: /^(Play|Resume)$/, exact: true }).first().click();
      await decoded('audio');
      await page.getByRole('button', { name: 'Open now playing', exact: true }).click();
      const dialog = page.getByRole('dialog', { name: 'Now playing', exact: true });
      await expect(dialog).toBeVisible();
      await dialog.getByText('Acceptance Track 2', { exact: true }).click();
      await expect.poll(() => page.locator('audio').evaluate((m) => m.currentSrc)).toContain(fixture.album.tracks[1].mediaFileId);
      await decoded('audio');
      await dialog.getByRole('button', { name: 'Pause', exact: true }).click();
      await expect.poll(() => page.locator('audio').evaluate((m) => m.paused)).toBe(true);
      await page.screenshot({ path: resolve(out, `queue-${viewport.width}.png`) });
      await page.keyboard.press('Escape');
    });
    await context.tracing.stop({ path: resolve(out, `trace-${viewport.width}.zip`) });
    await context.close();
  }
} finally { await browser.close(); }
assert(results.length > 0, 'No playback acceptance cases matched');
assert(results.every((result) => result.passed), `${results.filter((r) => !r.passed).length} playback checks failed; see ${out}/results.json`);
