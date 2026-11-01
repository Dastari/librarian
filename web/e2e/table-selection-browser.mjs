#!/usr/bin/env node
/** UI FIXTURE CHECKS ONLY. No backend/runtime/device qualification.
 * Run from web: node --check e2e/table-selection-browser.mjs (syntax only)
 * Then, with the managed Vite server already running and browser lane free:
 * node e2e/table-selection-browser.mjs
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
const BASE = process.env.TABLE_SELECTION_UI_URL ?? 'http://127.0.0.1:3000';
assert(['localhost', '127.0.0.1'].includes(new URL(BASE).hostname), 'Use the existing local managed Vite server');
const OUT = resolve(WEB, 'tmp/screenshots/table-selection-browser');
const roots = { castSettings:'CastSetting', collections:'Collection', seasons:'Season', users: 'User', inviteTokens: 'InviteToken', sources: 'Source', qualityProfiles: 'QualityProfile', namingPatterns: 'NamingPattern', releaseBlocklists: 'ReleaseBlocklist', appLogs: 'AppLog', libraryScanRuns: 'LibraryScanRun', libraryScanIssues: 'LibraryScanIssue',
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
  add('mediaFiles', { id:'file-video-two',libraryId:'library-movies',path:'/fixture/second.mp4',container:'mp4',size:1024,duration:120});
  add('mediaFiles', { id: 'file-video', libraryId: 'library-movies', movieId: 'movie-night', path: '/fixture/video.mp4', container: 'mp4', size: 1024, duration: 5400, width: 1280, height: 720 });
  add('mediaFiles', { id: 'file-audio', libraryId: 'library-music', trackId: 'track-one', path: '/fixture/audio.wav', container: 'wav', size: 1024, duration: 120 });
  add('mediaFiles', { id: 'file-audio-two', libraryId: 'library-music', trackId: 'track-two', path: '/fixture/audio-two.wav', container: 'wav', size: 1024, duration: 150 });
  add('personalLists', { id: 'playlist-seed', name: 'Fixture playlist', kind: 'MANUAL' });
  add('listEntries', { id: 'entry-track-one', listId: 'playlist-seed', trackId: 'track-one', position: 0 });
  add('listEntries', { id: 'entry-track-two', listId: 'playlist-seed', trackId: 'track-two', position: 1024 });
  add('subtitles', { id: 'subtitle-one', mediaFileId: 'file-video', language: 'eng', title: 'English', format: 'srt', streamIndex: 2 });
  add('audioStreams', { id: 'audio-one', mediaFileId: 'file-video', streamIndex: 0, language: 'eng', title: 'English', codec: 'aac' });
  add('audioStreams', { id: 'audio-two', mediaFileId: 'file-video', streamIndex: 1, language: 'fra', title: 'French', codec: 'aac' });


  add('movies', { id: 'movie-more', libraryId: 'library-movies', title: 'Fixture Wanted Two', hasFile: false, wanted: true });
  state.movies.find(row=>row.id==='movie-dawn').wanted=true;
  add('libraries', { id: 'library-tv', name: 'Fixture TV', libraryType: 'tv', path: '/fixture/tv' });
  add('libraries', { id: 'library-books', name: 'Fixture Books', libraryType: 'audiobooks', path: '/fixture/books' });
  add('seasons',{id:'season-one',showId:'show-one',season:1,name:'Season 1'});
  add('savedViews',{id:'view-tracks',name:'Fixture Rule',version:1,contentType:'TRACK',availability:'ANY',sort:'TITLE',descending:false});
  add('personalLists',{id:'playlist-smart',name:'Fixture smart list',kind:'SMART',savedViewId:'view-tracks'});
  add('shows', { id: 'show-one', libraryId: 'library-tv', name: 'Fixture Show' });
  add('audiobooks', { id: 'book-one', libraryId: 'library-books', title: 'Fixture Book', authorName: 'Fixture Author', hasFiles: true });
  for(let i=1;i<=2;i++) {
    add('episodes', { id:'episode-'+i, showId:'show-one', libraryId:'library-tv', title:'Fixture Episode '+i, season:1, episode:i, episodeNumber:i });
    add('chapters', { id:'chapter-'+i, audiobookId:'book-one', title:'Fixture Chapter '+i, chapterNumber:i, durationSecs:120 });
    add('users', { id:i===1?OWNER:'other-user', username:'fixture-user-'+i, displayName:'Fixture User '+i, role:i===1?'admin':'member', isActive:true });
    add('inviteTokens', {id:'invite-'+i,token:'fixture-invite-'+i,role:'member',maxUses:1,useCount:0,isActive:true});
    add('sources', {id:'source-'+i,name:'Fixture Source '+i,definitionId:'torznab',sourceType:'torznab',siteUrl:'https://fixture.invalid',enabled:true,priority:i,supportsSearch:true,supportsMovieSearch:true});
    add('qualityProfiles',{id:'profile-'+i,name:'Fixture Profile '+i,mediaKind:'VIDEO',isDefault:i===1,allowedResolutions:['1080p'],allowedVideoCodecs:['h264']});
    add('namingPatterns',{id:'pattern-'+i,name:'Fixture Pattern '+i,libraryType:'movies',pattern:'{title}',isSystem:false});
    add('releaseBlocklists',{id:'blocked-'+i,title:'Fixture Block '+i,reason:'manual',movieId:'movie-night'});
    add('castDevices',{id:'receiver-'+i,name:'Fixture Receiver '+i,address:'192.0.2.'+i,port:8009,deviceType:'chromecast',enabled:true,isManual:true});
    add('notifications',{id:'notice-'+i,title:'Fixture Notice '+i,message:'Fixture message '+i,notificationType:'info',category:'system'});
    add('appLogs',{id:'log-'+i,timestamp:STAMP,level:'INFO',target:'fixture',message:'Fixture Log '+i});
    add('libraryScanRuns',{id:'run-'+i,libraryId:'library-movies',status:'COMPLETED',startedAt:STAMP,completedAt:STAMP,discoveredCount:2,matchedCount:1,unmatchedCount:1});
    add('libraryScanIssues',{id:'issue-'+i,libraryId:'library-movies',scanRunId:'run-'+i,severity:'WARNING',stage:'ANALYSIS',issueCode:'ANALYSIS_QUEUE_FAILED',message:'Fixture scan issue '+i});
  }
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
      if (key === 'backupCapabilities') return entity('BackupCapabilities',{fullDatabaseBackupAvailable:true,objectBackupAvailable:true,restoreAvailable:false});
      if (key === 'backupSnapshots') return [1,2].map(i=>entity('BackupSnapshotSummary',{snapshotId:'fixture-snapshot-'+i,createdAt:1793404800,kind:'full',appVersion:'fixture',tableCount:1,objectCount:1,totalObjectBytes:1024}));
      if (key === 'appLogsKeyset') return connection('appLogs',args);
      if (key === 'liveTorrents') return state.torrents.map((row,index)=>entity('LiveTorrent',{
        id:index+1,infoHash:row.infoHash,name:row.name,state:row.state,progress:row.progress,size:row.totalBytes,downloaded:row.downloadedBytes,uploaded:row.uploadedBytesTotal,downloadSpeed:0,uploadSpeed:1024,peers:3,savePath:row.savePath,
        files:Array.from({length:80},(_,i)=>({index:i,path:'folder/'+('filename-'.repeat(8))+i+'.mkv',size:10000,progress:1})),
      }));
      if (key === 'hasEnabledSources') return true;
      if (key === 'searchSources') {
        const resultType=getNamedType(schema.getQueryType().getFields()[key].type);
        const sourceType=getNamedType(resultType.getFields().sources.type);
        const releaseType=getNamedType(sourceType.getFields().releases.type);
        const parsedType=getNamedType(releaseType.getFields().parsed.type);
        const pageType=getNamedType(sourceType.getFields().pageInfo.type);
        const releases=[1,2].map(i=>entity(releaseType.name,{stableId:'release-'+i,title:'Fixture Release '+i,guid:'guid-'+i,sourceId:'source-1',sourceName:'Fixture Source',seeders:10,magnetUri:'magnet:?xt=urn:btih:'+i,parsed:entity(parsedType.name,{languages:[],episodes:[]})}));
        return entity(resultType.name,{totalReleases:2,totalElapsedMs:1,sourcesSearched:1,sources:[entity(sourceType.name,{sourceId:'source-1',sourceName:'Fixture Source',releases,pageInfo:entity(pageType.name,{support:'SUPPORTED',state:'EXHAUSTED',hasMore:false,returnedCount:2})})]});
      }
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
      if (['pauseTorrent','resumeTorrent','removeTorrent','processSource'].includes(key)) {
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
    const collection = key === 'issues' ? 'libraryScanIssues' : key === 'entries' ? 'listEntries' : key === 'castCredits' ? 'movieCastCredits' : key;
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
    localStorage.setItem('librarian.pref.show.episodes.view',JSON.stringify('table'));
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
const cases=[
 ['/downloads','Downloads'],['/activity','Activity'],['/wanted','Wanted'],
 ['/libraries/library-music/tracks','Tracks'],['/libraries/library-movies/files','Files'],
 ['/libraries/library-movies/scans','Open issues'],['/libraries/library-movies/scans?view=history','Scan history'],
 ['/lists/playlist-smart','Smart collection'],['/movies/movie-dawn','Releases'],['/lists/playlist-seed','Fixture playlist'],['/albums/album-one','Fixture Album tracks'],
 ['/audiobooks/book-one','Fixture Book chapters'],['/shows/show-one','Season 1 episodes'],
 ['/settings/users','Users'],['/settings/users','Open invites'],['/settings/sources','Sources'],
 ['/settings/libraries','Libraries'],['/settings/quality','Quality profiles'],['/settings/organization','Naming patterns'],
 ['/settings/casting','Cast devices'],['/settings/backup','Snapshots'],['/settings/downloads','Blocked releases'],['/settings/logs','System log'],
];
const results=[];
try {
 for(const width of [1440,390]) for(const [path,label] of cases.filter(([,name])=>!process.env.TABLE_CASES||process.env.TABLE_CASES.split('|').includes(name))){
  console.log('Checking',width,path,label);
  const context=await browser.newContext({viewport:{width,height:1000},serviceWorkers:'block',reducedMotion:'reduce'});
  const model=fixture();await install(context,model,'dark');const page=await context.newPage();page.on('pageerror',error=>model.pageErrors.push(String(error)));page.setDefaultTimeout(12000);
  try {
   await page.goto(BASE+path,{waitUntil:'domcontentloaded'});
   if(label==='Releases'){if(width<600){await page.getByRole('button',{name:'More actions for Fixture Dawn',exact:true}).click();await page.getByRole('menuitem',{name:'Find releases',exact:true}).click();}else await page.getByRole('button',{name:'Find releases',exact:true}).click();}
   const grid=page.getByRole('grid',{name:label,exact:true}).first();await expect(grid).toBeVisible({timeout:20000});
   const table=page.locator('.fa-dt').filter({has:grid});
   const checks=table.getByRole('checkbox',{name:/^Select (?!all)/});await expect.poll(()=>checks.count(),{timeout:15000}).toBeGreaterThanOrEqual(2);
   await checks.nth(0).click();await checks.nth(1).click();
   await expect(checks.nth(0)).toHaveAttribute('aria-checked','true');await expect(checks.nth(1)).toHaveAttribute('aria-checked','true');
   const bulk=table.getByRole('button',{name:/^Actions for 2 selected /});await expect(bulk).toBeVisible();await bulk.click();
   const menu=page.getByRole('menu').last();await expect(menu).toBeVisible();assert(await menu.getByRole('menuitem').count()>0,'empty selection context');
   const box=await menu.boundingBox();assert(box.x>=-1&&box.x+box.width<=width+1,'selection menu overflow');
   await page.keyboard.press('Escape');
   await table.getByRole('button',{name:/^Clear( selection)?$/}).click();
   await expect(checks.nth(0)).toHaveAttribute('aria-checked','false');
   assert(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth+1),'horizontal document overflow');
   assert(!model.errors.length,JSON.stringify(model.errors));assert(!model.pageErrors.length,JSON.stringify(model.pageErrors));assert(!model.blocked.length,JSON.stringify(model.blocked));
   results.push({width,path,label,passed:true});
  }catch(error){console.error('FAILED',width,path,label,String(error).slice(0,600),JSON.stringify(model.errors));await page.screenshot({path:resolve(OUT,`failure-${width}-${label.replaceAll(/[^a-z0-9]/gi,'-')}.png`)});results.push({width,path,label,passed:false,error:String(error),graphqlErrors:model.errors});}
  finally{writeFileSync(resolve(OUT,'results.json'),JSON.stringify({qualification:'UI fixtures only; no real GraphQL reads/writes',results},null,2));await context.close();}
 }
 writeFileSync(resolve(OUT,'results.json'),JSON.stringify({qualification:'UI fixtures only; no real GraphQL reads/writes, playback or receivers',results},null,2));
 console.log(results.filter(r=>r.passed).length+'/'+results.length+' table selection browser checks passed');
 assert(results.every(r=>r.passed),'Some table checks failed; inspect fixture errors and evidence');
}finally{await browser.close();}
