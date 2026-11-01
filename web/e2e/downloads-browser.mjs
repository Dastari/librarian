#!/usr/bin/env node
/** UI FIXTURE CHECKS ONLY. No backend/runtime/device qualification.
 * Run from web: node --check e2e/downloads-browser.mjs (syntax only)
 * Then, with the managed Vite server already running and browser lane free:
 * node e2e/downloads-browser.mjs
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
const BASE = process.env.DOWNLOADS_UI_URL ?? 'http://127.0.0.1:3000';
assert(['localhost', '127.0.0.1'].includes(new URL(BASE).hostname), 'Use the existing local managed Vite server');
const OUT = resolve(WEB, 'tmp/screenshots/downloads-browser');
const roots = {
  torrents: 'Torrent', castSessions: 'CastSession', castDevices: 'CastDevice', libraries: 'Library', movies: 'Movie', tracks: 'Track', albums: 'Album', artists: 'Artist',
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

  for(let index=1;index<=30;index++) add('torrents', {
    id:'torrent-'+index,infoHash:String(index).padStart(40,'0'),name:index===1?'Fallout.2024.S01E04.1080p.HEVC.x265-MeGusta.With.A.Very.Long.Name':'Download '+String(index).padStart(2,'0'),
    state:'seeding',progress:1,totalBytes:492096000,downloadedBytes:492096000,uploadedBytesTotal:245000000,
    libraryId:null,savePath:'/data/downloads/'+('very-long-folder/'.repeat(8))+index,postProcessStatus:index===1?'failed':null,postProcessError:index===1?'Import could not complete. '+('Long error context '.repeat(30)):null,addedAt:STAMP,
  });
  const resolver = (source, args, _context, info) => {
    const key = info.fieldName;
    if (info.parentType.name === 'Query') {
      if (key === 'liveTorrents') return state.torrents.map((row,index)=>entity('LiveTorrent',{
        id:index+1,infoHash:row.infoHash,name:row.name,state:row.state,progress:row.progress,size:row.totalBytes,downloaded:row.downloadedBytes,uploaded:row.uploadedBytesTotal,downloadSpeed:0,uploadSpeed:1024,peers:3,savePath:row.savePath,
        files:Array.from({length:80},(_,i)=>({index:i,path:'folder/'+('filename-'.repeat(8))+i+'.mkv',size:10000,progress:1})),
      }));
      if (key === 'hasEnabledSources') return false;
      if (key === 'me') return authenticated ? user : null;
      if (key === 'needsSetup') return false;
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
      if (['pauseTorrent','resumeTorrent','processSource'].includes(key)) {
        const row=state.torrents[args.id-1];
        if(key==='pauseTorrent'&&row) row.state='paused';
        if(key==='resumeTorrent'&&row) row.state='seeding';
        return {success:true,error:null,filesProcessed:1,filesFailed:0};
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


mkdirSync(OUT,{recursive:true});
const browser=await chromium.launch({args:['--no-sandbox']});
const results=[];
try {
 for (const [width,height] of [[1920,1000],[1440,900],[834,1112],[390,844]]) {
  console.log('Checking Downloads',width,height);
  const context=await browser.newContext({viewport:{width,height},serviceWorkers:'block',reducedMotion:'reduce'});
  const model=fixture();await install(context,model,'dark');const page=await context.newPage();page.on('pageerror',error=>model.pageErrors.push(String(error)));
  try {
   await page.goto(BASE+'/downloads',{waitUntil:'domcontentloaded'});
   const first='Fallout.2024.S01E04.1080p.HEVC.x265-MeGusta.With.A.Very.Long.Name';
   await expect(page.getByText(first,{exact:true})).toBeVisible({timeout:15000});
   await page.getByRole('checkbox',{name:'Select '+first,exact:true}).click();
   await page.getByRole('checkbox',{name:'Select Download 02',exact:true}).click();
   await page.getByRole('button',{name:'Actions for 2 selected downloads',exact:true}).click();
   await page.getByRole('menuitem',{name:'Pause 2 downloads',exact:true}).click();
   await expect.poll(()=>model.requests.filter(r=>r.operationName==='PauseTorrent').length).toBe(2);
   results.push({width,height,name:'bulk selection pause',passed:true});
   await page.getByText(first,{exact:true}).first().click();
   const close=page.getByRole('button',{name:'Close download details',exact:true});await expect(close).toBeVisible();
   const details=page.locator('section[aria-label="Download details"]').last();
   if(width>=1280){
    const divider=page.getByRole('separator',{name:'Resize Downloads table',exact:true});await expect(divider).toBeVisible();
    const old=await divider.getAttribute('aria-valuenow');await divider.focus();await page.keyboard.press('ArrowLeft');await expect(divider).not.toHaveAttribute('aria-valuenow',old);
    const box=await divider.boundingBox();await page.mouse.move(box.x+box.width/2,box.y+box.height/2);await page.mouse.down();await page.mouse.move(box.x-120,box.y+box.height/2,{steps:5});await page.mouse.up();
    const bulk=page.getByRole('button',{name:'Actions for 2 selected downloads',exact:true});
    const bb=await bulk.boundingBox(), fb=await page.locator('.fa-dt-footer').boundingBox();
    assert(bb.x>=fb.x&&bb.x+bb.width<=fb.x+fb.width,'bulk control clipped after resizing');
    await bulk.click();
    await expect(page.getByRole('menuitem',{name:'Resume 2 downloads',exact:true})).toBeVisible();
    await expect(page.getByRole('menuitem',{name:'Import 2 downloads',exact:true})).toBeVisible();
    await expect(page.getByRole('menuitem',{name:'Remove 2 downloads',exact:true})).toBeVisible();
    await expect(page.getByRole('menuitem',{name:'Remove 2 and delete files',exact:true})).toBeVisible();
    await page.keyboard.press('Escape');
    results.push({width,height,name:'mouse and keyboard resizing with reachable bulk actions',passed:true});
   }else await expect(page.getByRole('dialog',{name:'Download details',exact:true})).toBeVisible();
   const identity=details.locator('[data-torrent-identity]');
   const ib=await identity.boundingBox(), cb=await close.boundingBox();assert(cb.y>=ib.y-1&&cb.y+cb.height<=ib.y+ib.height+1,'close wrapped below identity');
   const frame=await details.boundingBox();assert(frame.y>=0&&frame.y+frame.height<=height+1,'detail frame outside viewport');
   const scroll=width>=1280?details.locator('[aria-label="Download details content"]'):details.locator('.fa-scroll-viewport');
   await expect.poll(()=>scroll.evaluate(e=>e.scrollHeight>e.clientHeight)).toBe(true);
   await page.evaluate(()=>window.getSelection()?.removeAllRanges());
   await page.screenshot({path:resolve(OUT,`downloads-${width}-top.png`)});
   const header=await identity.boundingBox();await scroll.evaluate(e=>{e.scrollTop=e.scrollHeight});await page.waitForTimeout(80);const headerAfter=await identity.boundingBox();assert(Math.abs(header.y-headerAfter.y)<1,'identity scrolled out');
   assert(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth+1),'horizontal overflow');
   assert(await page.locator('#main-scroll').evaluate(e=>e.scrollHeight<=e.clientHeight+1),'page overflow');
   if(width>=1280){const footer=await page.locator('.fa-dt-footer').boundingBox();assert(footer.y+footer.height<=height,'table footer below screen');}
   await page.screenshot({path:resolve(OUT,`downloads-${width}.png`)});
   results.push({width,height,name:'bounded details, fixed single-row header, internal scrolling',passed:true});
   assert(!model.errors.length,JSON.stringify(model.errors));assert(!model.pageErrors.length,JSON.stringify(model.pageErrors));assert(!model.blocked.length,JSON.stringify(model.blocked));
  }catch(error){await page.screenshot({path:resolve(OUT,`failure-${width}.png`)});console.error('fixture errors',JSON.stringify(model.errors));throw error;}
  finally{await context.close();}
 }
 writeFileSync(resolve(OUT,'results.json'),JSON.stringify({qualification:'UI fixtures only; all GraphQL and torrent actions intercepted, no real torrents changed',results},null,2));console.log('Passed',results.length,'Downloads checks');
}finally{await browser.close();}
