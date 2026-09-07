#!/usr/bin/env node
/**
 * Builds the sign-in constellation dataset from Wikidata: well-known films and TV series
 * (ranked by the number of Wikipedia language editions that cover them) joined to their
 * best-known cast. The result is a compact bipartite graph that the sign-in backdrop grows
 * at random: title → cast → their other titles → and so on.
 *
 * Each title and person also gets the thumbnail of its English Wikipedia page image (the poster
 * for most films and series, a portrait for most actors), stored as the path under
 * https://upload.wikimedia.org/wikipedia/ so the sign-in screen can show them without any API.
 *
 * Output: src/assets/film-graph.json
 *   { titles: [[name, year, kind, image]], people: [[name, image]], edges: [[title, person]] }
 * Run with `pnpm run film-graph`. Needs network access; the checked-in file is the fallback.
 */
import { writeFileSync } from "node:fs";
import { resolve } from "node:path";

const ENDPOINT = "https://query.wikidata.org/sparql";
const USER_AGENT = "librarian-web/1.0 (sign-in constellation generator)";
const FILM_LIMIT = 900;
const SERIES_LIMIT = 300;
const MIN_ACTOR_LINKS = 35;
const MAX_CAST_PER_TITLE = 7;

async function sparql(query, attempt = 0) {
  const url = `${ENDPOINT}?query=${encodeURIComponent(query)}`;
  const response = await fetch(url, { headers: { Accept: "application/sparql-results+json", "User-Agent": USER_AGENT } });
  if (response.status === 429 && attempt < 4) {
    await new Promise((done) => setTimeout(done, 3000 * (attempt + 1)));
    return sparql(query, attempt + 1);
  }
  if (!response.ok) throw new Error(`Wikidata ${response.status}: ${await response.text()}`);
  const json = await response.json();
  return json.results.bindings;
}

const qid = (uri) => uri.slice(uri.lastIndexOf("/") + 1);
const articleTitle = (url) => decodeURIComponent(url.slice(url.lastIndexOf("/wiki/") + 6)).replaceAll("_", " ");

const THUMB_WIDTH = 200;
const UPLOAD_PREFIX = "https://upload.wikimedia.org/wikipedia/";

async function fetchJson(url, attempt = 0) {
  const response = await fetch(url, { headers: { "User-Agent": USER_AGENT } });
  if ((response.status === 429 || response.status >= 500) && attempt < 6) {
    const wait = Number(response.headers.get("retry-after")) * 1000 || 4000 * (attempt + 1);
    await new Promise((done) => setTimeout(done, wait));
    return fetchJson(url, attempt + 1);
  }
  if (!response.ok) throw new Error(`${new URL(url).host} ${response.status}`);
  return response.json();
}

/**
 * Thumbnail paths repeat the file name and carry tracking parameters; store them as
 * `wiki:hash:width:file` and let the app rebuild the URL. Unusual paths are kept verbatim.
 */
function compactPath(path) {
  const clean = path.split("?")[0];
  const match = /^([a-z]+)\/thumb\/([0-9a-f]\/[0-9a-f]{2})\/([^/]+)\/(\d+)px-([^/]+)$/.exec(clean);
  if (!match || match[3] !== match[5]) return clean;
  return `${match[1]}:${match[2]}:${match[4]}:${match[3]}`;
}

/** English Wikipedia page images, 50 titles per request; returns article title → thumbnail path. */
async function pageImages(articles) {
  const found = new Map();
  for (let index = 0; index < articles.length; index += 50) {
    const batch = articles.slice(index, index + 50);
    const params = new URLSearchParams({ action: "query", prop: "pageimages", piprop: "thumbnail", pithumbsize: String(THUMB_WIDTH), pilicense: "any", format: "json", formatversion: "2", redirects: "1", titles: batch.join("|") });
    const json = await fetchJson(`https://en.wikipedia.org/w/api.php?${params}`);
    await new Promise((done) => setTimeout(done, 350));
    const redirected = new Map((json.query?.redirects ?? []).map((entry) => [entry.to, entry.from]));
    const normalized = new Map((json.query?.normalized ?? []).map((entry) => [entry.to, entry.from]));
    for (const page of json.query?.pages ?? []) {
      const source = page.thumbnail?.source;
      if (!source || !source.startsWith(UPLOAD_PREFIX)) continue;
      const original = normalized.get(redirected.get(page.title) ?? page.title) ?? redirected.get(page.title) ?? page.title;
      found.set(original, compactPath(source.slice(UPLOAD_PREFIX.length)));
    }
    process.stdout.write(`images ${Math.min(index + 50, articles.length)}/${articles.length}\r`);
  }
  console.log();
  return found;
}

async function titles(classId, kind, limit, dateProperty) {
  const rows = await sparql(`
    SELECT ?item ?label ?article (MIN(YEAR(?date)) AS ?year) ?links WHERE {
      ?item wdt:P31 wd:${classId}; wikibase:sitelinks ?links; wdt:${dateProperty} ?date; rdfs:label ?label.
      ?article schema:about ?item; schema:isPartOf <https://en.wikipedia.org/>.
      FILTER(LANG(?label) = "en")
      FILTER(?links >= 40)
    }
    GROUP BY ?item ?label ?article ?links
    ORDER BY DESC(?links)
    LIMIT ${limit}`);
  return rows.map((row) => ({ id: qid(row.item.value), name: row.label.value, year: Number(row.year.value), kind, links: Number(row.links.value), article: articleTitle(row.article.value) }));
}

/** Wikidata's cast property also covers archive footage, so people must be actors by occupation. */
const ACTOR_OCCUPATIONS = ["Q33999", "Q10800557", "Q10798782", "Q2259451", "Q2405480", "Q948329"];

async function cast(ids) {
  const values = ids.map((id) => `wd:${id}`).join(" ");
  const occupations = ACTOR_OCCUPATIONS.map((id) => `wd:${id}`).join(" ");
  return sparql(`
    SELECT DISTINCT ?item ?actor ?label ?links ?article WHERE {
      VALUES ?item { ${values} }
      VALUES ?occupation { ${occupations} }
      ?item wdt:P161 ?actor.
      ?actor wdt:P106 ?occupation; wikibase:sitelinks ?links; rdfs:label ?label.
      OPTIONAL { ?article schema:about ?actor; schema:isPartOf <https://en.wikipedia.org/>. }
      FILTER(LANG(?label) = "en")
      FILTER(?links >= ${MIN_ACTOR_LINKS})
    }`);
}

const films = await titles("Q11424", "film", FILM_LIMIT, "P577");
console.log(`films: ${films.length}`);
const series = await titles("Q5398426", "series", SERIES_LIMIT, "P580");
console.log(`series: ${series.length}`);

const all = [...films, ...series];
const byId = new Map(all.map((title) => [title.id, title]));
const people = new Map(); // actorId -> { name, links, titles: Set }
const castByTitle = new Map(); // titleId -> [{ actorId, links }]

const CHUNK = 100;
for (let index = 0; index < all.length; index += CHUNK) {
  const chunk = all.slice(index, index + CHUNK).map((title) => title.id);
  const rows = await cast(chunk);
  for (const row of rows) {
    const titleId = qid(row.item.value);
    const actorId = qid(row.actor.value);
    const links = Number(row.links.value);
    if (!people.has(actorId)) people.set(actorId, { name: row.label.value, links, titles: new Set(), article: row.article ? articleTitle(row.article.value) : null });
    people.get(actorId).titles.add(titleId);
    if (!castByTitle.has(titleId)) castByTitle.set(titleId, []);
    castByTitle.get(titleId).push({ actorId, links });
  }
  process.stdout.write(`cast ${Math.min(index + CHUNK, all.length)}/${all.length}\r`);
}
console.log();

// Keep the best-known cast per title; prefer actors that connect several titles.
const keptEdges = new Set();
for (const [titleId, members] of castByTitle) {
  const ranked = members
    .map((member) => ({ ...member, degree: people.get(member.actorId).titles.size }))
    .sort((a, b) => b.degree - a.degree || b.links - a.links)
    .slice(0, MAX_CAST_PER_TITLE);
  for (const member of ranked) keptEdges.add(`${titleId}|${member.actorId}`);
}

const usedTitles = new Set();
const usedPeople = new Set();
for (const key of keptEdges) {
  const [titleId, actorId] = key.split("|");
  usedTitles.add(titleId);
  usedPeople.add(actorId);
}
const titleImages = await pageImages([...usedTitles].map((id) => byId.get(id).article));
const personImages = await pageImages([...usedPeople].map((id) => people.get(id).article).filter(Boolean));
console.log(`images: ${titleImages.size} titles, ${personImages.size} people`);

const titleIndex = new Map();
const personIndex = new Map();
const outTitles = [];
const outPeople = [];
const outEdges = [];
for (const key of keptEdges) {
  const [titleId, actorId] = key.split("|");
  const title = byId.get(titleId);
  const person = people.get(actorId);
  if (!title || !person) continue;
  if (!titleIndex.has(titleId)) {
    titleIndex.set(titleId, outTitles.length);
    outTitles.push([title.name, title.year, title.kind === "film" ? 0 : 1, titleImages.get(title.article) ?? ""]);
  }
  if (!personIndex.has(actorId)) {
    personIndex.set(actorId, outPeople.length);
    outPeople.push([person.name, (person.article && personImages.get(person.article)) ?? ""]);
  }
  outEdges.push([titleIndex.get(titleId), personIndex.get(actorId)]);
}

const output = { generatedAt: new Date().toISOString().slice(0, 10), source: "Wikidata (CC0)", titles: outTitles, people: outPeople, edges: outEdges };
const path = resolve(process.cwd(), "src/assets/film-graph.json");
writeFileSync(path, JSON.stringify(output));
console.log(`titles ${outTitles.length}, people ${outPeople.length}, edges ${outEdges.length} → ${path}`);
