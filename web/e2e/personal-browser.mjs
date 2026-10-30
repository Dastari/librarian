#!/usr/bin/env node
/** UI FIXTURE CHECKS ONLY. No backend/runtime/device qualification.
 * Run from web: node e2e/personal-browser.mjs --check (no browser)
 * Then, with the managed Vite server already running and browser lane free:
 * node e2e/personal-browser.mjs
 * Never imports playwright.config.ts, starts services, or uses live credentials.
 */
import assert from 'node:assert/strict';
import { readFileSync, mkdirSync, writeFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { resolve, dirname } from 'node:path';
import { buildSchema, parse, validate, execute, getNamedType, isNonNullType, isListType, isEnumType } from 'graphql';
import { chromium, expect } from '@playwright/test';

const WEB = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const schema = buildSchema(readFileSync(resolve(WEB, 'schema.graphql'), 'utf8'));
const OWNER = 'fixture-user';
const STAMP = '2026-10-26T00:00:00Z';
const POST_SMOKE = process.argv.includes('--post-smoke');
const BASE = process.env.PERSONAL_UI_URL ?? 'http://127.0.0.1:3000';
assert(['localhost', '127.0.0.1'].includes(new URL(BASE).hostname), 'Use the existing local managed Vite server');
const OUT = resolve(WEB, 'tmp/screenshots/personal-browser');
const roots = {
  libraries: 'Library', movies: 'Movie', tracks: 'Track', albums: 'Album', artists: 'Artist',
  shows: 'Show', episodes: 'Episode', audiobooks: 'Audiobook', chapters: 'Chapter',
  mediaFiles: 'MediaFile', personalLists: 'PersonalList', listEntries: 'ListEntry',
  savedViews: 'SavedView', homeShelves: 'HomeShelf', libraryPreferences: 'LibraryPreference',
  notifications: 'Notification', appSettings: 'AppSetting', playbackProgresses: 'PlaybackProgress',
  playbackSessions: 'PlaybackSession', movieCastCredits: 'MovieCastCredit',
  scheduleCaches: 'ScheduleCache', scheduleSyncStates: 'ScheduleSyncState',
  videoStreams: 'VideoStream', audioStreams: 'AudioStream', subtitles: 'Subtitle', mediaChapters: 'MediaChapter',
};
const mutable = new Set(['PersonalList', 'ListEntry', 'SavedView', 'HomeShelf', 'LibraryPreference', 'PlaybackProgress', 'PlaybackSession']);
const singular = Object.fromEntries(Object.entries(roots).map(([key, type]) => [type[0].toLowerCase() + type.slice(1), key]));

// Defaults only fill scalar catalogue metadata; unknown roots and missing relations fail.
function entity(type, values) {
  const node = { __typename: type };
  for (const [key, field] of Object.entries(schema.getType(type).getFields())) {
    if (!isNonNullType(field.type)) { node[key] = null; continue; }
    const raw = field.type.ofType;
    const named = getNamedType(raw);
    if (isListType(raw)) node[key] = [];
    else if (isEnumType(named)) node[key] = named.getValues()[0].name;
    else if (named.name === 'Boolean') node[key] = false;
    else if (['Int', 'Float'].includes(named.name)) node[key] = 0;
    else if (['String', 'ID'].includes(named.name)) node[key] = /At$/.test(key) ? STAMP : '';
  }
  return Object.assign(node, values);
}

function matches(row, where) {
  if (!where) return true;
  return Object.entries(where).every(([key, test]) => {
    if (key === 'and') return test.every((part) => matches(row, part));
    if (key === 'or') return test.some((part) => matches(row, part));
    if (key === 'not') return !matches(row, test);
    return Object.entries(test ?? {}).every(([op, value]) => {
      const actual = row[key];
      if (op === 'eq') return actual === value;
      if (op === 'ne') return actual !== value;
      if (op === 'inList') return value.includes(actual);
      if (op === 'isNull') return (actual == null) === value;
      if (op === 'contains') return String(actual ?? '').toLowerCase().includes(String(value).toLowerCase());
      if (op === 'gte') return actual >= value;
      if (op === 'lte') return actual <= value;
      if (op === 'gt') return actual > value;
      if (op === 'lt') return actual < value;
      throw new Error(`Unimplemented fixture filter ${key}.${op}`);
    });
  });
}

function fixture() {
  let sequence = 0;
  const state = Object.fromEntries(Object.keys(roots).map((name) => [name, []]));
  const requests = [], errors = [], blocked = [], sockets = [], consoleErrors = [], pageErrors = [];
  let authenticated = true;
  const user = { id: OWNER, username: 'fixture', displayName: 'UI Fixture User', email: 'fixture@example.invalid', role: 'admin' };
  const tokens = { expiresIn: 3600, tokenType: 'Bearer' };
  const add = (collection, values) => { const row = entity(roots[collection], { userId: OWNER, ...values }); state[collection].push(row); return row; };
  add('libraries', { id: 'library-movies', name: 'Fixture Movies', libraryType: 'movies', path: '/fixture/movies', namingPattern: '{title}', scanIntervalMinutes: 60 });
  add('libraries', { id: 'library-music', name: 'Fixture Music', libraryType: 'music', path: '/fixture/music', namingPattern: '{title}', scanIntervalMinutes: 60 });
  add('movies', { id: 'movie-night', libraryId: 'library-movies', title: 'Fixture Night', year: 2024, hasFile: true, mediaFileId: 'file-video', runtime: 90, overview: 'Isolated UI fixture movie.' });
  add('movies', { id: 'movie-dawn', libraryId: 'library-movies', title: 'Fixture Dawn', year: 2025, hasFile: false });
  add('artists', { id: 'artist-one', libraryId: 'library-music', name: 'Fixture Artist' });
  add('albums', { id: 'album-one', libraryId: 'library-music', artistId: 'artist-one', name: 'Fixture Album', year: 2024 });
  add('tracks', { id: 'track-one', libraryId: 'library-music', albumId: 'album-one', title: 'Fixture Track One', artistName: 'Fixture Artist', trackNumber: 1, durationSecs: 120, mediaFileId: 'file-audio' });
  add('tracks', { id: 'track-two', libraryId: 'library-music', albumId: 'album-one', title: 'Fixture Track Two', artistName: 'Fixture Artist', trackNumber: 2, durationSecs: 150, mediaFileId: 'file-audio-two' });
  add('mediaFiles', { id: 'file-video', libraryId: 'library-movies', movieId: 'movie-night', path: '/fixture/video.mp4', container: 'mp4', size: 1024, duration: 5400, width: 1280, height: 720 });
  add('mediaFiles', { id: 'file-audio', libraryId: 'library-music', trackId: 'track-one', path: '/fixture/audio.wav', container: 'wav', size: 1024, duration: 120 });
  add('mediaFiles', { id: 'file-audio-two', libraryId: 'library-music', trackId: 'track-two', path: '/fixture/audio-two.wav', container: 'wav', size: 1024, duration: 150 });
  add('personalLists', { id: 'playlist-seed', name: 'Fixture playlist', kind: 'MANUAL' });
  add('listEntries', { id: 'entry-track-one', listId: 'playlist-seed', trackId: 'track-one', position: 0 });
  add('listEntries', { id: 'entry-track-two', listId: 'playlist-seed', trackId: 'track-two', position: 1024 });
  add('subtitles', { id: 'subtitle-one', mediaFileId: 'file-video', language: 'eng', title: 'English', format: 'srt', streamIndex: 2 });
  add('audioStreams', { id: 'audio-one', mediaFileId: 'file-video', streamIndex: 0, language: 'eng', title: 'English', codec: 'aac' });
  add('audioStreams', { id: 'audio-two', mediaFileId: 'file-video', streamIndex: 1, language: 'fra', title: 'French', codec: 'aac' });

  function connection(collection, args = {}, parent, fieldName) {
    let rows = state[collection];
    if (parent?.__typename === 'Library') rows = rows.filter((row) => row.libraryId === parent.id);
    if (parent?.__typename === 'PersonalList' && fieldName === 'entries') rows = rows.filter((row) => row.listId === parent.id);
    if (parent?.__typename === 'Album' && collection === 'tracks') rows = rows.filter((row) => row.albumId === parent.id);
    if (parent?.__typename === 'MediaFile') rows = rows.filter((row) => row.mediaFileId === parent.id);
    rows = rows.filter((row) => matches(row, args.where));
    const orders = Array.isArray(args.orderBy) ? args.orderBy : args.orderBy ? [args.orderBy] : [];
    rows = [...rows].sort((a, b) => {
      for (const order of orders) for (const [key, direction] of Object.entries(order)) {
        if (a[key] === b[key]) continue;
        return (a[key] < b[key] ? -1 : 1) * (direction === 'DESC' ? -1 : 1);
      }
      return 0;
    });
    const totalCount = rows.length, offset = args.page?.offset ?? 0, limit = args.page?.limit ?? 100;
    return { edges: rows.slice(offset, offset + limit).map((node, index) => ({ node, cursor: String(offset + index) })), pageInfo: { totalCount, hasNextPage: offset + limit < totalCount, hasPreviousPage: offset > 0, startCursor: null, endCursor: null } };
  }

  const resolver = (source, args, _context, info) => {
    const key = info.fieldName;
    if (info.parentType.name === 'Query') {
      if (key === 'me') return authenticated ? user : null;
      if (key === 'needsSetup') return false;
      if (key === 'activeDownloadCount') return 0;
      if (key === 'contentStatuses') return args.inputs.map((input) => ({ ...input, status: state.movies.some((row) => row.id === input.id && row.hasFile) || state.tracks.some((row) => row.id === input.id && row.mediaFileId) ? 'AVAILABLE' : 'MISSING' }));
      if (key === 'resumeShelf') return { items: [], hasMore: false, complete: true, scannedCount: 0, error: null };
      if (key === 'movieReleases') return [];
      if (roots[key]) return connection(key, args);
      if (singular[key]) return state[singular[key]].find((row) => row.id === args.id) ?? null;
      throw new Error(`Unimplemented fixture Query.${key}`);
    }
    if (info.parentType.name === 'Mutation') {
      if (key === 'refreshToken') return { success: authenticated, error: authenticated ? null : 'Fixture signed out', tokens: authenticated ? tokens : null };
      if (key === 'logout') { authenticated = false; return { success: true, error: null }; }
      if (key === 'login') {
        const success = args.input.password === 'fixture-password';
        if (success) authenticated = true;
        return { success, error: success ? null : 'Invalid username/email or password', user: success ? user : null, tokens: success ? tokens : null };
      }
      const match = /^(create|update|delete)(.+)$/.exec(key);
      if (!match || !mutable.has(match[2])) throw new Error(`Unimplemented fixture Mutation.${key}`);
      const [, verb, type] = match, field = type[0].toLowerCase() + type.slice(1), collection = singular[field];
      let row;
      if (verb === 'create') {
        assert(authenticated, 'Fixture requires authentication');
        if (args.input.userId) assert.equal(args.input.userId, OWNER);
        row = add(collection, { id: `fixture-${field}-${++sequence}`, ...args.input });
      } else {
        row = state[collection].find((item) => item.id === args.id);
        assert(row, `Missing fixture ${type} ${args.id}`);
        if (verb === 'update') Object.assign(row, args.input);
        else {
          state[collection] = state[collection].filter((item) => item.id !== args.id);
          if (type === 'PersonalList') state.listEntries = state.listEntries.filter((item) => item.listId !== args.id);
        }
      }
      return { success: true, error: null, [field]: row, affectedCount: 1 };
    }
    const collection = key === 'entries' ? 'listEntries' : key === 'castCredits' ? 'movieCastCredits' : key;
    if (roots[collection] && String(info.returnType).includes('Connection')) return connection(collection, args, source, key);
    if (singular[key] && source?.[`${key}Id`]) return state[singular[key]].find((row) => row.id === source[`${key}Id`]) ?? null;
    if (source && key in source) return source[key];
    if (!isNonNullType(info.returnType)) return null;
    throw new Error(`Missing fixture field ${info.parentType.name}.${key}`);
  };
  async function operation(payload) {
    requests.push({ operationName: payload.operationName, variables: payload.variables });
    try {
      const document = parse(payload.query), failures = validate(schema, document);
      if (failures.length) throw new Error(failures.map((error) => error.message).join('; '));
      const result = await execute({ schema, document, variableValues: payload.variables, operationName: payload.operationName, fieldResolver: resolver });
      if (result.errors?.length) errors.push(...result.errors.map((error) => ({ operation: payload.operationName, message: error.message })));
      return result;
    } catch (error) {
      errors.push({ operation: payload.operationName, message: error.message });
      return { errors: [{ message: error.message }] };
    }
  }
  return { state, requests, errors, blocked, sockets, consoleErrors, pageErrors, operation, signOut: () => { authenticated = false; } };
}

async function install(context, model, mode) {
  await context.addInitScript(({ mode }) => {
    localStorage.setItem('librarian.appearance.v1', JSON.stringify({ mode, palette: 'cinema', glass: 'solid', density: 'comfortable', uiScale: 1, radius: 1, lineWeight: 1, fontSans: 'inter', fontDisplay: 'outfit' }));
  }, { mode });
  await context.routeWebSocket('**/graphql/ws', (ws) => {
    // Never connect to the live GraphQL socket. Vite's own HMR remains untouched.
    ws.onMessage((raw) => {
      const message = JSON.parse(String(raw));
      if (message.type === 'connection_init') ws.send(JSON.stringify({ type: 'connection_ack' }));
      else if (message.type === 'ping') ws.send(JSON.stringify({ type: 'pong' }));
      else if (message.type === 'subscribe') {
        const failures = validate(schema, parse(message.payload.query));
        model.sockets.push({ operationName: message.payload.operationName, errors: failures.map((error) => error.message) });
        if (failures.length) model.errors.push({ operation: message.payload.operationName, message: failures.map((error) => error.message).join('; ') });
        // Acknowledged, silent subscriptions: this does not test backend invalidation.
      }
    });
  });
  await context.route('**/*', async (route) => {
    const request = route.request(), url = new URL(request.url());
    if (url.pathname === '/graphql') {
      try {
        const payload = request.method() === 'GET' ? { query: url.searchParams.get('query'), variables: JSON.parse(url.searchParams.get('variables') ?? '{}'), operationName: url.searchParams.get('operationName') } : request.postDataJSON();
        assert(!Array.isArray(payload), 'No batched fixture transport');
        return route.fulfill({ json: await model.operation(payload) });
      } catch (error) {
        model.errors.push({ operation: 'transport', message: error.message });
        return route.fulfill({ status: 500, json: { errors: [{ message: error.message }] } });
      }
    }
    if (url.pathname.startsWith('/api/')) {
      if (url.pathname.startsWith('/api/artwork/')) return route.fulfill({ contentType: 'image/svg+xml', body: '<svg xmlns="http://www.w3.org/2000/svg" width="300" height="450"><rect width="100%" height="100%" fill="#455a64"/><text x="20" y="60" fill="white">UI FIXTURE</text></svg>' });
      if (/\/api\/media\/[^/]+\/info$/.test(url.pathname)) return route.fulfill({ json: { exists: true, duration: 5400, needs_hls: false, playback_url: '/fixture/no-media', content_type: 'video/mp4' } });
      // Deliberately no playable media. Controls/menu checks do not qualify playback.
      if (/\/api\/media\/[^/]+\/stream$/.test(url.pathname)) return route.fulfill({ status: 200, contentType: 'video/mp4', body: Buffer.alloc(0) });
      if (url.pathname.includes('/subtitle')) return route.fulfill({ contentType: 'text/vtt', body: 'WEBVTT\n\n' });
      model.blocked.push({ url: request.url(), method: request.method(), reason: 'Unimplemented API fixture' });
      return route.fulfill({ status: 501, json: { error: 'UI fixture did not implement this endpoint' } });
    }
    if (url.origin !== new URL(BASE).origin || !['GET', 'HEAD'].includes(request.method())) {
      model.blocked.push({ url: request.url(), method: request.method(), reason: 'External request or non-asset write blocked' });
      return route.abort('blockedbyclient');
    }
    return route.continue();
  });
}

async function choose(page, scope, label, name) {
  const select = scope.getByRole('combobox', { name: label, exact: true });
  await select.click();
  await page.getByRole('option', { name, exact: true }).click();
}
async function action(page, name, scope = page) {
  const button = scope.getByRole('button', { name, exact: true });
  await expect(button.or(scope.getByRole('button', { name: /More actions/i })).first()).toBeVisible();
  if (await button.first().isVisible()) return button.first().click();
  await scope.getByRole('button', { name: /More actions/i }).first().click();
  await page.getByRole('menuitem', { name, exact: true }).click();
}
async function focusInside(dialog) {
  await expect.poll(() => dialog.evaluate((node) => node.contains(document.activeElement))).toBe(true);
}
async function noOverflow(page) {
  const geometry = await page.evaluate(() => ({ width: innerWidth, scroll: document.documentElement.scrollWidth }));
  assert(geometry.scroll <= geometry.width + 1, `Document overflow ${JSON.stringify(geometry)}`);
}

async function runMatrix(browser, width, mode) {
  const id = `${width}-${mode}-solid`, model = fixture(), checks = [];
  const context = await browser.newContext({ viewport: { width, height: width < 500 ? 844 : 1112 }, hasTouch: width < 500, isMobile: width < 500, colorScheme: mode, reducedMotion: 'reduce', serviceWorkers: 'block', deviceScaleFactor: 1 });
  await install(context, model, mode);
  const page = await context.newPage();
  page.setDefaultTimeout(8000);
  page.on('pageerror', (error) => model.pageErrors.push(error.message));
  page.on('console', (message) => { if (message.type() === 'error') model.consoleErrors.push(message.text()); });
  const go = async (path) => { await page.goto(`${BASE}${path}`, { waitUntil: 'domcontentloaded' }); };
  const shot = async (name) => { const path = resolve(OUT, `${POST_SMOKE ? 'post-smoke-' : ''}${id}-${name}.png`); await page.screenshot({ path }); return path; };
  const step = async (name, work) => {
    const start = Date.now();
    console.log(`${id} START ${name}`);
    try { await work(); checks.push({ name, status: 'passed', ms: Date.now() - start, screenshot: await shot(name) }); }
    catch (error) { checks.push({ name, status: 'failed', ms: Date.now() - start, error: error.stack, screenshot: await shot(`${name}-FAILED`).catch(() => null) }); await page.keyboard.press('Escape').catch(() => {}); }
    console.log(`${id} ${checks.at(-1).status.toUpperCase()} ${name}`);
  };
  let manualId, smartId;
  try {
    await step('lists-dialog-focus', async () => {
      await go('/lists');
      const opener = page.getByRole('button', { name: 'New list', exact: true });
      await expect(opener).toBeVisible();
      await expect(page.locator('html')).toHaveAttribute('data-mode', mode);
      await expect(page.locator('html')).toHaveAttribute('data-glass', 'solid');
      await opener.click();
      const dialog = page.getByRole('dialog', { name: 'New list or collection' });
      await expect(dialog).toBeVisible(); await focusInside(dialog);
      for (let n = 0; n < 10; n++) { await page.keyboard.press('Tab'); await focusInside(dialog); }
      await noOverflow(page);
      await page.keyboard.press('Escape'); await expect(dialog).toBeHidden(); await expect(opener).toBeFocused();
    });
    await step('manual-create', async () => {
      await go('/lists'); await page.getByRole('button', { name: 'New list', exact: true }).click();
      const dialog = page.getByRole('dialog', { name: 'New list or collection' });
      await dialog.getByRole('textbox', { name: 'Name', exact: true }).fill('Fixture evening');
      await dialog.getByRole('button', { name: 'Create', exact: true }).click();
      await expect(dialog).toBeHidden();
      // Creation invalidation is qualified by a fresh route snapshot as well.
      await go('/lists'); await expect(page.getByRole('link', { name: /Fixture evening/ })).toBeVisible();
      manualId = model.state.personalLists.find((row) => row.name === 'Fixture evening')?.id;
      assert(manualId);
    });
    await step('manual-save', async () => {
      assert(manualId, 'manual-create prerequisite failed');
      await go('/movies/movie-night'); await action(page, 'Save to list');
      const dialog = page.getByRole('dialog', { name: 'Save Fixture Night', exact: true });
      await expect(dialog).toBeVisible(); await focusInside(dialog);
      await choose(page, dialog, 'List', 'Fixture evening');
      await expect(dialog.getByRole('button', { name: 'Save', exact: true })).toBeEnabled();
      await dialog.getByRole('button', { name: 'Save', exact: true }).click(); await expect(dialog).toBeHidden();
      assert(model.state.listEntries.some((row) => row.listId === manualId && row.movieId === 'movie-night'));
      await go(`/lists/${manualId}`); await expect(page.getByRole('link', { name: /Fixture Night/ }).first()).toBeVisible();
    });
    await step('watch-later', async () => {
      await go('/movies/movie-dawn'); await action(page, 'Save to list');
      const dialog = page.getByRole('dialog', { name: 'Save Fixture Dawn', exact: true });
      await dialog.getByRole('button', { name: 'Choose Watch Later', exact: true }).click();
      const save = dialog.getByRole('button', { name: 'Save', exact: true }); await expect(save).toBeEnabled(); await save.click();
      await expect(dialog).toBeHidden();
      const watch = model.state.personalLists.find((row) => row.kind === 'WATCH_LATER'); assert(watch);
      await go(`/lists/${watch.id}`); await expect(page.getByRole('link', { name: /Fixture Dawn/ }).first()).toBeVisible();
    });
    await step('smart-create-results', async () => {
      await go('/lists'); await page.getByRole('button', { name: 'New list', exact: true }).click();
      const dialog = page.getByRole('dialog', { name: 'New list or collection' });
      await dialog.getByRole('textbox', { name: 'Name', exact: true }).fill('Fixture smart night');
      await choose(page, dialog, 'Type', 'Smart collection');
      await dialog.getByRole('textbox', { name: 'Title contains', exact: true }).fill('Night');
      await choose(page, dialog, 'Availability', 'Has a file');
      await dialog.getByRole('button', { name: 'Create', exact: true }).click(); await expect(dialog).toBeHidden();
      smartId = model.state.personalLists.find((row) => row.name === 'Fixture smart night')?.id; assert(smartId);
      await go(`/lists/${smartId}`); await expect(page.getByText('Fixture Night', { exact: true }).first()).toBeVisible();
      await expect(page.getByText('Fixture Dawn', { exact: true })).toHaveCount(0); await noOverflow(page);
    });
    await step('playlist-reorder', async () => {
      await go('/lists/playlist-seed');
      await expect(page.getByRole('link', { name: /Fixture Track One/ })).toBeVisible();
      // Responsive Glass cards and table rows both expose the same action names.
      const first = page.getByRole('button', { name: /^Actions for / }).first();
      await first.click(); await page.getByRole('menuitem', { name: 'Move down', exact: true }).click();
      await expect.poll(() => model.state.listEntries.find((row) => row.id === 'entry-track-one').position).toBeGreaterThan(1024);
      await expect(page.getByRole('link', { name: /Fixture Track Two/ })).toBeVisible(); await noOverflow(page);
    });
    await step('pin-home-shelf', async () => {
      await go('/lists/playlist-seed'); await page.getByRole('button', { name: 'Pin to Home', exact: true }).click();
      await expect(page.getByRole('button', { name: 'Unpin from Home', exact: true })).toBeVisible();
      await go('/'); await expect(page.getByText('Fixture playlist', { exact: true }).first()).toBeVisible();
      await page.getByRole('button', { name: 'Customize Home', exact: true }).click();
      const dialog = page.getByRole('dialog', { name: 'Customize Home', exact: true });
      await expect(dialog).toBeVisible(); await focusInside(dialog);
      await dialog.getByRole('switch', { name: 'Show Recently added movies', exact: true }).click();
      await expect.poll(() => model.state.homeShelves.find((row) => row.shelfKey === 'movies')?.visible).toBe(false);
      await dialog.getByRole('button', { name: 'Move New music up', exact: true }).click();
      await expect.poll(() => model.state.homeShelves.find((row) => row.shelfKey === 'albums')?.position).toBe(25);
      await dialog.getByRole('button', { name: 'Done', exact: true }).click(); await expect(dialog).toBeHidden();
      await expect(page.getByRole('button', { name: 'Customize Home', exact: true })).toBeFocused(); await noOverflow(page);
    });
    await step('favourites', async () => {
      await go('/libraries');
      await page.getByRole('button', { name: 'Favourite Fixture Music', exact: true }).click();
      await expect(page.getByRole('button', { name: 'Unfavourite Fixture Music', exact: true })).toHaveAttribute('aria-pressed', 'true');
      assert(model.state.libraryPreferences.some((row) => row.libraryId === 'library-music' && row.favourite));
      await noOverflow(page);
    });
    await step('smart-edit-rules', async () => {
      assert(smartId, 'smart-create-results prerequisite failed');
      await go(`/lists/${smartId}`); await page.getByRole('button', { name: 'Edit rules', exact: true }).click();
      const dialog = page.getByRole('dialog', { name: 'Edit collection rules', exact: true });
      await dialog.getByRole('textbox', { name: 'Name', exact: true }).fill('Fixture smart dawn');
      await dialog.getByRole('textbox', { name: 'Title contains', exact: true }).fill('Dawn');
      await choose(page, dialog, 'Availability', 'Any');
      await dialog.getByRole('button', { name: 'Save rules', exact: true }).click(); await expect(dialog).toBeHidden();
      await go(`/lists/${smartId}`); await expect(page.getByText('Fixture Dawn', { exact: true }).first()).toBeVisible();
      await expect(page.getByText('Fixture Night', { exact: true })).toHaveCount(0);
      assert.equal(model.state.personalLists.find((row) => row.id === smartId).name, 'Fixture smart dawn');
    });
    await step('manual-remove-delete', async () => {
      assert(manualId, 'manual-create prerequisite failed');
      await go(`/lists/${manualId}`);
      await page.getByRole('button', { name: /^Actions for / }).first().click();
      await page.getByRole('menuitem', { name: 'Remove from list', exact: true }).click();
      await expect(page.getByText(/^Nothing saved yet/)).toBeVisible();
      assert(!model.state.listEntries.some((row) => row.listId === manualId));
      await page.getByRole('button', { name: 'Delete list', exact: true }).click();
      const dialog = page.getByRole('dialog', { name: 'Delete Fixture evening?', exact: true });
      await expect(dialog.getByRole('button', { name: 'Cancel', exact: true })).toBeFocused();
      await dialog.getByRole('button', { name: 'Delete list', exact: true }).click();
      await expect(page).toHaveURL(/\/lists$/);
      assert(!model.state.personalLists.some((row) => row.id === manualId));
    });
    await step('search-controls', async () => {
      await go('/search'); const field = page.getByRole('searchbox', { name: 'Search', exact: true });
      await expect(field).toBeFocused(); await field.fill('Night');
      await expect(page.getByText('Fixture Night', { exact: true }).first()).toBeVisible();
      await field.fill('zzz-unmatched-fixture'); await expect(page.getByText(/Nothing in your library matches/)).toBeVisible();
      await field.fill(''); await expect(page.getByText('Type at least two characters.', { exact: true })).toBeVisible(); await noOverflow(page);
    });
    await step('account-menu-focus', async () => {
      await go('/lists'); const opener = page.getByRole('button', { name: 'Account', exact: true });
      await opener.focus(); await page.keyboard.press('Enter'); await expect(page.getByRole('menu', { name: 'Account menu' })).toBeVisible();
      await page.keyboard.press('Escape'); await expect(opener).toBeFocused();
    });
    if (POST_SMOKE) await step('audio-dock-now-playing-volume-queue', async () => {
      await go('/lists/playlist-seed');
      await page.getByRole('button', { name: 'Play playlist', exact: true }).click();
      const dock = page.getByRole('region', { name: 'Now playing', exact: true });
      await expect(dock).toBeVisible(); await noOverflow(page);
      const rect = await dock.boundingBox(); assert(rect && rect.x >= -1 && rect.x + rect.width <= width + 1, 'Populated dock fits viewport');
      const volume = dock.getByRole('slider', { name: 'Volume', exact: true });
      if (width >= 768) {
        await expect(volume).toBeVisible();
        await volume.focus(); await page.keyboard.press('End'); await page.keyboard.press('ArrowLeft');
        await expect.poll(() => page.locator('audio').evaluate((node) => node.volume)).toBeCloseTo(0.98, 2);
        await dock.getByRole('button', { name: 'Mute', exact: true }).click();
        await expect.poll(() => page.locator('audio').evaluate((node) => node.muted)).toBe(true);
        await dock.getByRole('button', { name: 'Unmute', exact: true }).click();
        await dock.getByRole('button', { name: 'Queue', exact: true }).click();
      } else {
        await expect(volume).toBeHidden();
        await dock.getByRole('button', { name: 'Open now playing', exact: true }).click();
      }
      const dialog = page.getByRole('dialog', { name: 'Now playing', exact: true });
      await expect(dialog).toBeVisible(); await focusInside(dialog);
      await expect(dialog.getByRole('listitem')).toHaveCount(2);
      const sheetVolume = dialog.getByRole('slider', { name: 'Volume', exact: true });
      await expect(sheetVolume).toBeVisible(); await sheetVolume.focus();
      await page.keyboard.press('Home'); await page.keyboard.press('ArrowRight');
      await expect.poll(() => page.locator('audio').evaluate((node) => node.volume)).toBeCloseTo(0.02, 2);
      const second = dialog.getByRole('listitem').nth(1).getByRole('button');
      await second.click(); await expect(second).toHaveAttribute('aria-current', 'true');
      await shot('audio-now-playing-open'); await noOverflow(page);
      await page.keyboard.press('Escape'); await expect(dialog).toBeHidden();
      await expect(dock.getByRole('button', { name: width >= 768 ? 'Queue' : 'Open now playing', exact: true })).toBeFocused();
      await shot('audio-dock-populated');
      await dock.getByRole('button', { name: 'Stop', exact: true }).click(); await expect(dock).toBeHidden();
    });
    await step('video-player-menu-focus', async () => {
      await go('/watch/file-video');
      const opener = page.getByRole('button', { name: 'Playback settings', exact: true });
      await expect(opener).toBeVisible(); await opener.focus(); await page.keyboard.press('Enter');
      await expect(page.getByRole('menuitemradio', { name: 'Normal speed', exact: true })).toBeVisible();
      await page.keyboard.press('Escape'); await expect(opener).toBeFocused();
      await opener.click(); await page.getByRole('menuitemradio', { name: '1.5×', exact: true }).click();
      await expect.poll(() => page.locator('video').evaluate((node) => node.playbackRate)).toBe(1.5);
      await noOverflow(page);
    });
    await step('auth-controls', async () => {
      model.signOut(); await go('/login');
      await expect(page.getByRole('heading', { name: 'Sign in', exact: true })).toBeVisible();
      const identifier = page.getByRole('textbox', { name: 'Username or email', exact: true });
      await expect(identifier).toBeFocused(); await identifier.fill('fixture');
      await page.getByLabel('Password', { exact: true }).fill('wrong-fixture-password');
      await page.getByRole('button', { name: 'Sign in', exact: true }).click();
      await expect(page.getByText('Invalid username/email or password', { exact: true })).toBeVisible();
      await page.getByLabel('Password', { exact: true }).fill('fixture-password');
      await page.getByRole('button', { name: 'Sign in', exact: true }).click();
      await expect(page.getByRole('button', { name: 'Account', exact: true })).toBeVisible(); await noOverflow(page);
    });
    await step('fixture-boundary', async () => {
      assert.deepEqual(model.errors, [], 'GraphQL fixture validation/execution errors');
      assert.deepEqual(model.blocked.filter((item) => item.reason !== 'External request or non-asset write blocked'), [], 'Missing API fixture');
      assert.deepEqual(model.pageErrors, [], 'Uncaught browser errors');
      assert(model.requests.some((request) => request.operationName === 'RefreshSession'));
      assert(model.requests.some((request) => request.operationName === 'CurrentUser'));
    });
    return { id, width, mode, glass: 'solid', checks, ...model, signOut: undefined, operation: undefined };
  } finally { await context.close(); }
}

async function staticCheck() {
  // Exercise actual source operations without a browser or live HTTP connection.
  const source = readFileSync(resolve(WEB, 'src/graphql/entities/personal-list.graphql'), 'utf8');
  const document = parse(source);
  assert.deepEqual(validate(schema, document).map((error) => error.message), []);
  const model = fixture();
  const created = await model.operation({ query: source, operationName: 'EntityPersonalListCreate', variables: { input: { userId: OWNER, name: 'Static fixture check', kind: 'MANUAL' } } });
  assert.equal(created.data?.createPersonalList?.personalList?.name, 'Static fixture check');
  assert.deepEqual(model.errors, []);
  for (const file of ['home-shelf', 'list-entry', 'library-preference', 'saved-view']) {
    const doc = parse(readFileSync(resolve(WEB, `src/graphql/entities/${file}.graphql`), 'utf8'));
    assert.deepEqual(validate(schema, doc).map((error) => error.message), []);
  }
  const entries = await model.operation({ query: readFileSync(resolve(WEB, 'src/graphql/documents/personal.graphql'), 'utf8'), operationName: 'PersonalEntries', variables: { listId: 'playlist-seed', page: { offset: 0, limit: 100 } } });
  assert.equal(entries.data?.listEntries.edges[0].node.track.title, 'Fixture Track One');
  assert.equal(entries.data?.listEntries.edges[0].node.track.album.id, 'album-one');
  const mutation = readFileSync(resolve(WEB, 'src/graphql/entities/list-entry.graphql'), 'utf8');
  await model.operation({ query: mutation, operationName: 'EntityListEntryUpdate', variables: { id: 'entry-track-one', input: { position: 2048 } } });
  const reordered = await model.operation({ query: readFileSync(resolve(WEB, 'src/graphql/documents/personal.graphql'), 'utf8'), operationName: 'PersonalEntries', variables: { listId: 'playlist-seed' } });
  assert.equal(reordered.data?.listEntries.edges[0].node.track.title, 'Fixture Track Two');
  assert.deepEqual(model.errors, []);
  console.log('Source/schema fixture checks passed. No browser launched; no backend contacted.');
}

await staticCheck();
if (!process.argv.includes('--check')) {
  mkdirSync(OUT, { recursive: true });
  const result = { qualification: 'UI FIXTURE CHECKS ONLY — not backend/runtime/device qualification', sourceSchema: 'web/schema.graphql', baseURL: BASE, startedAt: new Date().toISOString(), limitations: ['GraphQL operations execute against in-memory fixtures after real SDL validation.', 'GraphQL subscriptions ack but emit no invalidations.', 'No playable audio/video: menus and controls only.', 'Widths emulate Chromium viewports; no physical devices or TV qualification.', 'External requests are blocked; artwork is a fixture SVG.'], matrices: [] };
  try {
    const response = await fetch(BASE, { method: 'HEAD', signal: AbortSignal.timeout(5000) });
    assert(response.ok, `Managed Vite origin returned HTTP ${response.status}`);
  } catch (error) {
    result.status = 'blocked'; result.prerequisiteError = `Existing managed Vite origin unavailable: ${error.message}`;
    result.finishedAt = new Date().toISOString();
    writeFileSync(resolve(OUT, 'results.json'), JSON.stringify(result, null, 2));
    console.error(result.prerequisiteError); process.exit(2);
  }
  const browser = await chromium.launch({ headless: true, args: ['--no-sandbox', '--disable-dev-shm-usage', '--disable-gpu', '--renderer-process-limit=2'], ...(process.env.PLAYWRIGHT_CHROMIUM_EXECUTABLE ? { executablePath: process.env.PLAYWRIGHT_CHROMIUM_EXECUTABLE } : {}) });
  try {
    for (const width of (process.env.PERSONAL_UI_WIDTHS ?? (POST_SMOKE ? '375,834' : '375,390,768,834')).split(',').map(Number)) {
      for (const mode of (process.env.PERSONAL_UI_MODES ?? (POST_SMOKE ? 'dark' : 'light,dark')).split(',')) {
        const matrix = await runMatrix(browser, width, mode); result.matrices.push(matrix);
        for (const check of matrix.checks) console.log(`${matrix.id} ${check.status.toUpperCase()} ${check.name}${check.status === 'failed' ? `: ${check.error.split('\n')[0]}` : ''}`);
        writeFileSync(resolve(OUT, 'results.json'), JSON.stringify(result, null, 2));
      }
    }
  } finally {
    await browser.close(); result.finishedAt = new Date().toISOString();
    result.passed = result.matrices.reduce((sum, matrix) => sum + matrix.checks.filter((check) => check.status === 'passed').length, 0);
    result.failed = result.matrices.reduce((sum, matrix) => sum + matrix.checks.filter((check) => check.status === 'failed').length, 0);
    writeFileSync(resolve(OUT, 'results.json'), JSON.stringify(result, null, 2));
  }
  console.log(`UI fixture checks: ${result.passed} passed, ${result.failed} failed. ${resolve(OUT, 'results.json')}`);
  if (result.failed) process.exitCode = 1;
}
