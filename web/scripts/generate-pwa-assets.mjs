#!/usr/bin/env node
/**
 * Renders every installable-app asset from the sign-in art: the Cinema canvas (its ambient glows),
 * a film constellation like the one behind the sign-in card (drawn without its Wikipedia pictures
 * or names) and the Librarian mark in the middle.
 *
 *   pnpm pwa-assets            # writes public/ and the splash links in index.html
 *
 * Output (all committed):
 *   public/favicon.ico, public/favicon.svg, public/apple-touch-icon.png, public/og-image.png,
 *   public/browserconfig.xml, public/icons/* (favicons, Android, maskable, monochrome, iOS,
 *   Windows tiles, shortcuts, Safari pinned tab), public/splash/* (iOS startup images for every
 *   current iPhone and iPad, portrait and landscape, as JPEG), and src/assets/brand/pwa-colors.json
 *   (the brand and canvas colours as hex, read by vite.config.ts and lib/theme.tsx).
 *
 * The mark is drawn in the brand colour, Cinema's accent, read from src/styles/palettes.css.
 *
 * Chromium (Playwright's, as the e2e tests use) draws the art, so the gradients, oklch colours and
 * the Outfit wordmark match the app exactly; sharp compresses the PNGs and packs the ICO. The
 * layout is stable between runs (the constellation comes from a seeded generator); a rerun may
 * still change a few bytes of a JPEG where Chromium antialiases text differently.
 */
import { mkdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

import { chromium } from "@playwright/test";
import sharp from "sharp";

const web = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const pub = resolve(web, "public");
const iconsDir = resolve(pub, "icons");
const splashDir = resolve(pub, "splash");

/* ── Cinema, dark ─────────────────────────────────────────────────────────────────────────────────
 * Hues and the accent come from the palette parameters in src/styles/palettes.css; lightness and
 * chroma of the canvas, glows and text are the ones Glass derives from them in dark mode. */
const palettes = readFileSync(resolve(web, "src/styles/palettes.css"), "utf8");
function paletteParam(selector, name) {
  const block = palettes.match(new RegExp(`${selector.replace(/[[\]"=]/g, "\\$&")}[^{]*\\{([^}]*)\\}`))?.[1] ?? "";
  const value = block.match(new RegExp(`${name}:\\s*([\\d.]+)`))?.[1];
  if (value === undefined) throw new Error(`palettes.css has no ${name} for ${selector}`);
  return Number(value);
}
const cinema = (name) => paletteParam('[data-palette="cinema"]', name);
const neutralH = cinema("--neutral-h");

/**
 * The brand colour: Cinema's accent in dark mode (--accent, --brand-primary), the amber of the
 * mark, the active navigation item and the start of the primary button's gradient.
 */
const BRAND_COLOR = `oklch(${paletteParam('[data-mode="dark"][data-palette="cinema"]', "--accent-l")} ${cinema("--accent-c")} ${cinema("--accent-h")})`;

const CINEMA = {
  canvas: `oklch(0.17 0.022 ${neutralH})`,
  canvasLight: `oklch(0.94 0.016 ${neutralH})`,
  ambient1: `oklch(0.42 0.13 ${cinema("--ambient-1-h")})`,
  ambient2: `oklch(0.36 0.11 ${cinema("--ambient-2-h")})`,
  ambient3: `oklch(0.38 0.1 ${cinema("--ambient-3-h")})`,
  brand: BRAND_COLOR,
  foreground: `oklch(0.965 0.006 ${neutralH})`,
  muted: `oklch(0.77 0.016 ${neutralH})`,
  rim: "oklch(1 0 0 / 0.17)",
};

/** The product mark (src/lib/glass/brand.ts), in its 735 × 638 box. */
const MARK_VIEWBOX = [735, 638];
const MARK_PATH = readFileSync(resolve(web, "src/assets/brand/mark.svg"), "utf8").match(/<path d="([^"]+)"/)[1].replace(/\s+/g, " ");
const MARK_TRANSFORM = "translate(0,638) scale(0.1,-0.1)";

const fontData = readFileSync(resolve(web, "node_modules/@fontsource-variable/outfit/files/outfit-latin-wght-normal.woff2")).toString("base64");

/* ── Deterministic constellation ─────────────────────────────────────────────────────────────── */

function random(seed) {
  let a = 0;
  for (const char of seed) a = (Math.imul(a ^ char.charCodeAt(0), 2654435761) + 1) >>> 0;
  return () => {
    a = (a + 0x6d2b79f5) >>> 0;
    let t = a;
    t = Math.imul(t ^ (t >>> 15), t | 1);
    t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

/**
 * Posters (titles, some series) and portraits (people) on a jittered grid, linked to their
 * nearest neighbours, kept out of the ellipse `clear` where the mark sits. Sizes are the sign-in
 * screen's (46 × 68 posters, 21 px portraits) times `scale`.
 */
function constellation({ width, height, scale, clear, seed }) {
  const rand = random(seed);
  const cell = 132 * scale;
  const nodes = [];
  for (let gy = -0.5; gy * cell < height + cell / 2; gy += 1) {
    for (let gx = -0.5; gx * cell < width + cell / 2; gx += 1) {
      if (rand() > 0.72) continue;
      const x = (gx + 0.15 + rand() * 0.7) * cell;
      const y = (gy + 0.15 + rand() * 0.7) * cell;
      const dx = (x - clear.cx) / clear.rx;
      const dy = (y - clear.cy) / clear.ry;
      if (dx * dx + dy * dy < 1) continue;
      const roll = rand();
      nodes.push({ x, y, kind: roll < 0.42 ? "person" : roll < 0.58 ? "series" : "title" });
    }
  }
  const edges = new Set();
  nodes.forEach((node, index) => {
    const near = nodes
      .map((other, j) => ({ j, d: Math.hypot(other.x - node.x, other.y - node.y) }))
      .filter(({ j, d }) => j !== index && d < cell * 1.9)
      .sort((a, b) => a.d - b.d)
      .slice(0, 1 + Math.floor(rand() * 2));
    for (const { j } of near) edges.add(index < j ? `${index}-${j}` : `${j}-${index}`);
  });

  const accent = { title: CINEMA.brand, series: CINEMA.ambient2.replace("0.36", "0.72"), person: CINEMA.foreground };
  const poster = { w: 46 * scale, h: 68 * scale, r: 6 * scale };
  const portrait = 21 * scale;
  const defs = [];
  const lines = [];
  const shapes = [];
  for (const key of edges) {
    const [a, b] = key.split("-").map((value) => nodes[Number(value)]);
    lines.push(`<line x1="${a.x}" y1="${a.y}" x2="${b.x}" y2="${b.y}" stroke="${CINEMA.brand}" stroke-linecap="round" stroke-width="${3 * scale}" opacity="0.1"/>`);
    lines.push(`<line x1="${a.x}" y1="${a.y}" x2="${b.x}" y2="${b.y}" stroke="${CINEMA.brand}" stroke-linecap="round" stroke-width="${Math.max(0.6, scale)}" opacity="0.45"/>`);
  }
  nodes.forEach((node, index) => {
    const colour = accent[node.kind];
    const person = node.kind === "person";
    const w = person ? portrait * 2 : poster.w;
    const h = person ? portrait * 2 : poster.h;
    const x = node.x - w / 2;
    const y = node.y - h / 2;
    defs.push(`<radialGradient id="h${index}"><stop offset="0" stop-color="${colour}"/><stop offset="1" stop-color="${colour}" stop-opacity="0"/></radialGradient>`);
    defs.push(`<linearGradient id="f${index}" x1="0" y1="0" x2="1" y2="1"><stop offset="0" stop-color="${colour}"/><stop offset="1" stop-color="rgb(0 0 0 / 0.6)"/></linearGradient>`);
    shapes.push(`<circle cx="${node.x}" cy="${node.y}" r="${Math.max(w, h) * 0.9}" fill="url(#h${index})" opacity="0.18"/>`);
    const shape = person
      ? (attrs) => `<circle cx="${node.x}" cy="${node.y}" r="${portrait}" ${attrs}/>`
      : (attrs) => `<rect x="${x}" y="${y}" width="${w}" height="${h}" rx="${poster.r}" ${attrs}/>`;
    shapes.push(shape(`fill="url(#f${index})" opacity="${person ? 0.55 : 0.6}"`));
    shapes.push(shape(`fill="none" stroke="${CINEMA.rim}" stroke-width="${Math.max(0.6, scale)}" opacity="0.9"`));
  });
  return `<svg class="constellation" width="${width}" height="${height}" viewBox="0 0 ${width} ${height}" xmlns="http://www.w3.org/2000/svg"><defs>${defs.join("")}</defs>${lines.join("")}${shapes.join("")}</svg>`;
}

/* ── One piece of art ────────────────────────────────────────────────────────────────────────── */

/**
 * @param {object} options
 * @param {number} options.width CSS px
 * @param {number} options.height CSS px
 * @param {number} options.mark mark width as a fraction of the shorter side
 * @param {"art"|"mono"} [options.style] mono: the brand-coloured mark on transparency, nothing else
 * @param {number} [options.radius] corner radius as a fraction of the shorter side (transparent corners)
 * @param {boolean} [options.stars] draw the constellation (off for tiny favicons)
 * @param {number} [options.starScale] constellation size relative to the sign-in screen
 * @param {"none"|"below"|"beside"} [options.wordmark]
 * @param {string} [options.glyph] SVG markup (24 × 24, stroked) drawn instead of the mark
 */
function artHtml({ width, height, mark, style = "art", radius = 0, stars = true, starScale = 1, wordmark = "none", glyph, seed = "librarian", bare = false }) {
  const short = Math.min(width, height);
  const markW = short * mark;
  const markH = (markW * MARK_VIEWBOX[1]) / MARK_VIEWBOX[0];
  const markSvg = (fill) => `<svg class="mark" width="${markW}" height="${markH}" viewBox="0 0 ${MARK_VIEWBOX.join(" ")}" xmlns="http://www.w3.org/2000/svg"><path transform="${MARK_TRANSFORM}" fill="${fill}" d="${MARK_PATH}"/></svg>`;

  if (style === "mono") {
    return page(`<div class="center">${markSvg(CINEMA.brand)}</div>`, "");
  }

  const wordSize = wordmark === "beside" ? markH * 0.62 : short * 0.075;
  const word = wordmark === "none" ? "" : `<div class="word"><div class="name" style="font-size:${wordSize}px">Librarian</div>${wordmark === "beside" ? `<div class="tag" style="font-size:${wordSize * 0.36}px">Your media, your server</div>` : ""}</div>`;
  const clearW = wordmark === "beside" ? markW + wordSize * 6.6 : markW * 1.25;
  const clearH = wordmark === "below" ? markH + wordSize * 2.6 : markH * 1.25;
  const clear = { cx: width / 2, cy: height / 2, rx: clearW * 0.75 + 40 * starScale, ry: clearH * 0.75 + 40 * starScale };
  const centre = glyph
    ? `<svg class="mark" width="${short * mark}" height="${short * mark}" viewBox="0 0 24 24" fill="none" stroke="${CINEMA.brand}" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" xmlns="http://www.w3.org/2000/svg">${glyph}</svg>`
    : markSvg(CINEMA.brand);
  const glow = Math.max(1, short * 0.02);
  return page(
    `<div class="art" style="border-radius:${radius * short}px">
      <div class="layer ambient"></div>
      ${stars ? `<div class="layer stars">${constellation({ width, height, scale: starScale, clear, seed })}</div>` : ""}
      <div class="layer vignette"></div>
      <div class="layer fade"></div>
      ${bare ? "" : `<div class="center ${wordmark === "beside" ? "row" : "column"}" style="--glow:${glow}px;gap:${wordmark === "beside" ? markH * 0.32 : wordSize * 0.55}px">${centre}${word}</div>`}
    </div>`,
    `.art{position:fixed;inset:0;overflow:hidden;background:${CINEMA.canvas}}
     .layer{position:absolute;inset:0}
     .ambient{background-image:
        radial-gradient(46% 58% at 92% 4%, oklch(from ${CINEMA.ambient1} l c h / 0.55), transparent 72%),
        radial-gradient(42% 52% at 6% 96%, oklch(from ${CINEMA.ambient2} l c h / 0.55), transparent 72%),
        radial-gradient(36% 46% at 0% 22%, oklch(from ${CINEMA.ambient3} l c h / 0.55), transparent 70%),
        radial-gradient(34% 40% at 62% 104%, oklch(from ${CINEMA.ambient1} l c h / 0.33), transparent 70%)}
     .stars{opacity:0.62}
     .vignette{background:radial-gradient(ellipse at center, transparent 30%, ${CINEMA.canvas} 125%);opacity:0.75}
     .fade{background:linear-gradient(to top, oklch(from ${CINEMA.canvas} l c h / 0.6), transparent, oklch(from ${CINEMA.canvas} l c h / 0.2))}
     .mark{filter:drop-shadow(0 0 var(--glow) oklch(from ${CINEMA.brand} l c h / 0.45))}
     .row{flex-direction:row}.column{flex-direction:column}
     .word{display:flex;flex-direction:column;align-items:flex-start}
     .column .word{align-items:center}
     .name{font-family:Outfit;font-weight:600;letter-spacing:-0.01em;line-height:1;color:${CINEMA.foreground}}
     .tag{font-family:Outfit;font-weight:400;line-height:1;margin-top:0.7em;color:${CINEMA.muted}}`,
  );
}

function page(body, css) {
  return `<!doctype html><html><head><meta charset="utf-8"><style>
    @font-face{font-family:Outfit;src:url(data:font/woff2;base64,${fontData}) format("woff2");font-weight:100 900}
    html,body{margin:0;width:100%;height:100%;background:transparent}
    .center{position:fixed;inset:0;display:flex;align-items:center;justify-content:center}
    ${css}</style></head><body>${body}</body></html>`;
}

/* ── Device sizes ────────────────────────────────────────────────────────────────────────────── */

/** iOS startup images: CSS points in portrait and the device pixel ratio. Landscape swaps the pixels. */
const APPLE_DEVICES = [
  // iPhone
  { w: 440, h: 956, dpr: 3, name: "iPhone 16 Pro Max, 17 Pro Max" },
  { w: 420, h: 912, dpr: 3, name: "iPhone Air" },
  { w: 402, h: 874, dpr: 3, name: "iPhone 16 Pro, 17, 17 Pro" },
  { w: 430, h: 932, dpr: 3, name: "iPhone 14 Pro Max, 15 Plus, 15 Pro Max, 16 Plus" },
  { w: 393, h: 852, dpr: 3, name: "iPhone 14 Pro, 15, 15 Pro, 16" },
  { w: 428, h: 926, dpr: 3, name: "iPhone 12 Pro Max, 13 Pro Max, 14 Plus" },
  { w: 390, h: 844, dpr: 3, name: "iPhone 12, 12 Pro, 13, 13 Pro, 14, 16e" },
  { w: 375, h: 812, dpr: 3, name: "iPhone X, XS, 11 Pro, 12 mini, 13 mini" },
  { w: 414, h: 896, dpr: 3, name: "iPhone XS Max, 11 Pro Max" },
  { w: 414, h: 896, dpr: 2, name: "iPhone XR, 11" },
  { w: 414, h: 736, dpr: 3, name: "iPhone 6s Plus, 7 Plus, 8 Plus" },
  { w: 375, h: 667, dpr: 2, name: "iPhone 6s, 7, 8, SE (2nd and 3rd gen)" },
  { w: 320, h: 568, dpr: 2, name: "iPhone SE (1st gen), iPod touch" },
  // iPad
  { w: 1032, h: 1376, dpr: 2, name: "iPad Pro 13-inch (M4, M5)", ipad: true },
  { w: 1024, h: 1366, dpr: 2, name: "iPad Pro 12.9-inch, iPad Air 13-inch", ipad: true },
  { w: 834, h: 1210, dpr: 2, name: "iPad Pro 11-inch (M4, M5)", ipad: true },
  { w: 834, h: 1194, dpr: 2, name: "iPad Pro 11-inch, iPad Air 11-inch (M2)", ipad: true },
  { w: 820, h: 1180, dpr: 2, name: "iPad Air 10.9-inch, iPad 10th gen, iPad (A16)", ipad: true },
  { w: 834, h: 1112, dpr: 2, name: "iPad Air 10.5-inch, iPad Pro 10.5-inch", ipad: true },
  { w: 810, h: 1080, dpr: 2, name: "iPad 10.2-inch", ipad: true },
  { w: 768, h: 1024, dpr: 2, name: "iPad 9.7-inch, iPad mini 5", ipad: true },
  { w: 744, h: 1133, dpr: 2, name: "iPad mini (6th gen, A17 Pro)", ipad: true },
];

/* ── Rendering ───────────────────────────────────────────────────────────────────────────────── */

const browser = await chromium.launch();
const colours = await hexColours();
const contexts = new Map();
async function render(html, { width, height, dpr = 1, transparent = false }) {
  if (!contexts.has(dpr)) {
    const context = await browser.newContext({ deviceScaleFactor: dpr, viewport: { width: 64, height: 64 } });
    contexts.set(dpr, await context.newPage());
  }
  const tab = contexts.get(dpr);
  await tab.setViewportSize({ width, height });
  await tab.setContent(html, { waitUntil: "load" });
  await tab.evaluate(() => document.fonts.ready);
  return tab.screenshot({ type: "png", omitBackground: transparent, animations: "disabled" });
}

const written = [];
/** PNG, or JPEG for the opaque splash screens (iOS accepts both; a gradient is a tenth the size). */
async function save(file, png, { jpeg = false } = {}) {
  const image = sharp(png);
  const output = jpeg
    ? await image.jpeg({ quality: 88, mozjpeg: true, chromaSubsampling: "4:4:4" }).toBuffer()
    : await image.png({ compressionLevel: 9, adaptiveFiltering: true }).toBuffer();
  mkdirSync(dirname(file), { recursive: true });
  writeFileSync(file, output);
  written.push(file);
  return output;
}

/** Square icon art at `size` pixels. */
async function icon(size, options = {}) {
  const { radius = 0, mark = 0.5, stars = size >= 96, transparent = radius > 0 } = options;
  const html = artHtml({ width: size, height: size, mark, radius, stars, starScale: size / 520, seed: "icon" });
  return render(html, { width: size, height: size, transparent });
}

/* ── Android TV app (android-tv/) ─────────────────────────────────────────────────────────────────
 * The launcher banner (320 × 180, xhdpi, as Android TV asks), legacy launcher icons for API 24–25,
 * the adaptive icon's background art, and the mark as vector drawables for the adaptive icon's
 * foreground and monochrome layers and the server screen. `pnpm pwa-assets --android-tv` writes only these. */
const androidRes = resolve(web, "../android-tv/app/src/main/res");

/** The mark as an Android vector: the SVG path keeps its own flip and scale in a group. */
function markVector({ size, fit, colour }) {
  const [vw, vh] = MARK_VIEWBOX;
  const scale = size ? (size * fit) / vw : 1;
  const tx = size ? (size - vw * scale) / 2 : 0;
  const ty = size ? (size - vh * scale) / 2 : 0;
  const viewport = size ? `android:viewportWidth="${size}" android:viewportHeight="${size}"` : `android:viewportWidth="${vw}" android:viewportHeight="${vh}"`;
  const dimensions = size ? `android:width="${size}dp" android:height="${size}dp"` : `android:width="${vw / 10}dp" android:height="${vh / 10}dp"`;
  return `<?xml version="1.0" encoding="utf-8"?>
<!-- Generated by web/scripts/generate-pwa-assets.mjs from src/assets/brand/mark.svg. Do not edit. -->
<vector xmlns:android="http://schemas.android.com/apk/res/android"
    ${dimensions}
    ${viewport}>
    <group android:translateX="${tx.toFixed(3)}" android:translateY="${ty.toFixed(3)}" android:scaleX="${scale.toFixed(5)}" android:scaleY="${scale.toFixed(5)}">
        <group android:translateY="${vh}" android:scaleX="0.1" android:scaleY="-0.1">
            <path android:fillColor="${colour}" android:pathData="${MARK_PATH.trim()}" />
        </group>
    </group>
</vector>
`;
}

async function androidTvAssets() {
  const write = (path, text) => {
    mkdirSync(dirname(path), { recursive: true });
    writeFileSync(path, text);
    written.push(path);
  };
  await save(
    resolve(androidRes, "drawable-xhdpi/tv_banner.png"),
    await render(artHtml({ width: 320, height: 180, mark: 0.3, wordmark: "beside", starScale: 0.42, seed: "tv-banner" }), { width: 320, height: 180 }),
  );
  for (const [density, size] of [["mdpi", 48], ["hdpi", 72], ["xhdpi", 96], ["xxhdpi", 144], ["xxxhdpi", 192]]) {
    await save(resolve(androidRes, `mipmap-${density}/ic_launcher.png`), await icon(size, { radius: 0.22, mark: 0.52 }));
  }
  // Adaptive icon (API 26+): the art without the mark behind, the mark as a vector in front, kept
  // inside the 66 dp safe zone of the 108 dp layer.
  await save(
    resolve(androidRes, "mipmap-xxxhdpi/ic_launcher_background.png"),
    await render(artHtml({ width: 432, height: 432, mark: 0.5, starScale: 432 / 520, seed: "icon", bare: true }), { width: 432, height: 432 }),
  );
  write(resolve(androidRes, "drawable/ic_launcher_foreground.xml"), markVector({ size: 108, fit: 0.5, colour: colours.brand }));
  write(resolve(androidRes, "drawable/ic_mark.xml"), markVector({ colour: colours.brand }));
}

if (process.argv.includes("--android-tv")) {
  await androidTvAssets();
  await browser.close();
  console.log(`Wrote ${written.length} Android TV assets:\n${written.map((file) => `  ${file}`).join("\n")}`);
  process.exit(0);
}

rmSync(splashDir, { recursive: true, force: true });
mkdirSync(iconsDir, { recursive: true });
await androidTvAssets();

// Favicons: no constellation at these sizes, a bigger mark on a rounded tile.
const favicon = {};
for (const size of [16, 32, 48]) favicon[size] = await save(resolve(iconsDir, `favicon-${size}.png`), await icon(size, { radius: 0.2, mark: 0.74, stars: false }));
writeFileSync(resolve(pub, "favicon.ico"), buildIco([favicon[16], favicon[32], favicon[48]]));
written.push(resolve(pub, "favicon.ico"));

// Manifest "any" icons: the art on a rounded tile with transparent corners. Chrome also draws
// Android's install splash from the largest of these, centred on background_color.
for (const size of [48, 72, 96, 128, 144, 192, 256, 384, 512]) {
  await save(resolve(iconsDir, `icon-${size}.png`), await icon(size, { radius: 0.22, mark: 0.52 }));
}

// Maskable: full bleed; the mark stays inside the 80 % safe circle with room to spare.
for (const size of [192, 512]) await save(resolve(iconsDir, `icon-maskable-${size}.png`), await icon(size, { mark: 0.46 }));

// Monochrome (Android themed icons): only the alpha channel counts; the mark is drawn in the brand colour anyway.
for (const size of [192, 512]) {
  const html = artHtml({ width: size, height: size, mark: 0.46, style: "mono" });
  await save(resolve(iconsDir, `icon-monochrome-${size}.png`), await render(html, { width: size, height: size, transparent: true }));
}

// iOS home screen: square and opaque; iOS rounds the corners itself.
for (const size of [120, 152, 167, 180]) await save(resolve(iconsDir, `apple-touch-icon-${size}.png`), await icon(size, { mark: 0.52, transparent: false }));
await save(resolve(pub, "apple-touch-icon.png"), await icon(180, { mark: 0.52, transparent: false }));

// Windows tiles (browserconfig.xml): full bleed, the wide tile keeps the mark's proportions.
for (const size of [70, 144, 150, 310]) await save(resolve(iconsDir, `mstile-${size}x${size}.png`), await icon(size, { mark: size <= 70 ? 0.56 : 0.44, transparent: false }));
await save(
  resolve(iconsDir, "mstile-310x150.png"),
  await render(artHtml({ width: 310, height: 150, mark: 0.5, starScale: 0.42, seed: "wide" }), { width: 310, height: 150 }),
);

// App shortcuts.
for (const [name, glyph] of [
  ["search", '<circle cx="10" cy="10" r="7"/><path d="m21 21-6-6"/>'],
  ["downloads", '<path d="M12 4v12m0 0 5-5m-5 5-5-5"/><path d="M4 20h16"/>'],
]) {
  const html = artHtml({ width: 192, height: 192, mark: 0.42, glyph, starScale: 192 / 520, seed: "icon" });
  await save(resolve(iconsDir, `shortcut-${name}.png`), await render(html, { width: 192, height: 192 }));
}

// Open Graph / Twitter card: the mark with the wordmark, as on the sign-in card.
await save(
  resolve(pub, "og-image.png"),
  await render(artHtml({ width: 1200, height: 630, mark: 0.26, wordmark: "beside", starScale: 1.05, seed: "og" }), { width: 1200, height: 630 }),
);

// iOS startup images, drawn at the device's CSS size and pixel ratio so the constellation has the
// sign-in screen's proportions on every device. An installed app on iPad shows the portrait image
// in landscape too, stretched to fit (a long-standing Safari bug that ignores the landscape link),
// so iPads get only the background: a gradient stretched either way still looks right, where the
// mark, wordmark and constellation would be squashed.
const splashLinks = [];
for (const device of APPLE_DEVICES) {
  for (const orientation of ["portrait", "landscape"]) {
    const width = orientation === "portrait" ? device.w : device.h;
    const height = orientation === "portrait" ? device.h : device.w;
    const file = `apple-splash-${width * device.dpr}x${height * device.dpr}.jpg`;
    const html = artHtml({ width, height, mark: 0.2, wordmark: "below", seed: `splash-${orientation}`, stars: !device.ipad, bare: device.ipad });
    await save(resolve(splashDir, file), await render(html, { width, height, dpr: device.dpr }), { jpeg: true });
    splashLinks.push(
      `<link rel="apple-touch-startup-image" href="/splash/${file}" media="(device-width: ${device.w}px) and (device-height: ${device.h}px) and (-webkit-device-pixel-ratio: ${device.dpr}) and (orientation: ${orientation})" />`,
    );
  }
}
await browser.close();

// Vector favicon and Safari pinned tab.
writeFileSync(resolve(pub, "favicon.svg"), faviconSvg(colours));
writeFileSync(resolve(iconsDir, "safari-pinned-tab.svg"), `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 ${MARK_VIEWBOX.join(" ")}"><path transform="${MARK_TRANSFORM}" d="${MARK_PATH}"/></svg>\n`);
written.push(resolve(pub, "favicon.svg"), resolve(iconsDir, "safari-pinned-tab.svg"));

writeFileSync(
  resolve(pub, "browserconfig.xml"),
  `<?xml version="1.0" encoding="utf-8"?>
<browserconfig>
  <msapplication>
    <tile>
      <square70x70logo src="/icons/mstile-70x70.png"/>
      <square150x150logo src="/icons/mstile-150x150.png"/>
      <wide310x150logo src="/icons/mstile-310x150.png"/>
      <square310x310logo src="/icons/mstile-310x310.png"/>
      <TileColor>${colours.canvas}</TileColor>
    </tile>
  </msapplication>
</browserconfig>
`,
);
written.push(resolve(pub, "browserconfig.xml"));

// The splash links live in index.html between markers, so the device list has one home.
const indexPath = resolve(web, "index.html");
const index = readFileSync(indexPath, "utf8");
const start = "<!-- pwa-assets:splash:start -->";
const end = "<!-- pwa-assets:splash:end -->";
const from = index.indexOf(start);
const to = index.indexOf(end);
if (from < 0 || to < 0) throw new Error(`index.html needs ${start} … ${end}`);
const indent = index.slice(index.lastIndexOf("\n", from) + 1, from);
writeFileSync(indexPath, `${index.slice(0, from + start.length)}\n${splashLinks.map((link) => indent + link).join("\n")}\n${indent}${index.slice(to)}`);

console.log(`Wrote ${written.length} assets to public/ and ${splashLinks.length} splash links to index.html.`);
// The same colours for the manifest, index.html and the runtime theme-color (vite.config.ts, lib/theme.tsx).
const brandColours = { brand: colours.brand, canvasDark: colours.canvas, canvasLight: colours.canvasLight, brandOklch: BRAND_COLOR };
writeFileSync(resolve(web, "src/assets/brand/pwa-colors.json"), `${JSON.stringify(brandColours, null, 2)}\n`);
console.log(`Brand (Cinema accent): ${BRAND_COLOR} = ${colours.brand}; canvas ${colours.canvas} (dark), ${colours.canvasLight} (light)`);

/* ── Helpers ─────────────────────────────────────────────────────────────────────────────────── */

/** sRGB hex for the opaque oklch tokens, as Chromium converts them. */
async function hexColours() {
  const tab = await browser.newPage();
  const result = await tab.evaluate((colours) => {
    const canvas = document.createElement("canvas");
    canvas.width = canvas.height = 1;
    const context = canvas.getContext("2d", { willReadFrequently: true });
    const out = {};
    for (const [key, value] of Object.entries(colours)) {
      context.clearRect(0, 0, 1, 1);
      context.fillStyle = value;
      context.fillRect(0, 0, 1, 1);
      const [r, g, b] = context.getImageData(0, 0, 1, 1).data;
      out[key] = `#${[r, g, b].map((part) => part.toString(16).padStart(2, "0")).join("")}`;
    }
    return out;
  }, CINEMA);
  await tab.close();
  return result;
}

/** The art as a scalable favicon: canvas, three ambient glows and the mark on a rounded tile. */
function faviconSvg(hex) {
  const size = 64;
  const markW = size * 0.74;
  const markH = (markW * MARK_VIEWBOX[1]) / MARK_VIEWBOX[0];
  const glow = (id, cx, cy, r, colour, stop) =>
    `<radialGradient id="${id}" cx="${cx}" cy="${cy}" r="${r}"><stop offset="0" stop-color="${colour}" stop-opacity="0.55"/><stop offset="${stop}" stop-color="${colour}" stop-opacity="0"/></radialGradient>`;
  return `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 ${size} ${size}">
  <defs>
    ${glow("a", 0.92, 0.04, 0.52, hex.ambient1, 0.72)}
    ${glow("b", 0.06, 0.96, 0.47, hex.ambient2, 0.72)}
    ${glow("c", 0, 0.22, 0.41, hex.ambient3, 0.7)}
    <clipPath id="t"><rect width="${size}" height="${size}" rx="${size * 0.2}"/></clipPath>
  </defs>
  <g clip-path="url(#t)">
    <rect width="${size}" height="${size}" fill="${hex.canvas}"/>
    <rect width="${size}" height="${size}" fill="url(#a)"/>
    <rect width="${size}" height="${size}" fill="url(#b)"/>
    <rect width="${size}" height="${size}" fill="url(#c)"/>
  </g>
  <svg x="${(size - markW) / 2}" y="${(size - markH) / 2}" width="${markW}" height="${markH}" viewBox="0 0 ${MARK_VIEWBOX.join(" ")}"><path transform="${MARK_TRANSFORM}" fill="${hex.brand}" d="${MARK_PATH}"/></svg>
</svg>
`;
}

/** Minimal ICO container with embedded PNG entries (supported by every modern browser). */
function buildIco(pngs) {
  const header = Buffer.alloc(6);
  header.writeUInt16LE(0, 0);
  header.writeUInt16LE(1, 2);
  header.writeUInt16LE(pngs.length, 4);
  const entries = [];
  let offset = 6 + 16 * pngs.length;
  for (const png of pngs) {
    const size = png.readUInt32BE(16);
    const entry = Buffer.alloc(16);
    entry.writeUInt8(size >= 256 ? 0 : size, 0);
    entry.writeUInt8(size >= 256 ? 0 : size, 1);
    entry.writeUInt16LE(1, 4);
    entry.writeUInt16LE(32, 6);
    entry.writeUInt32LE(png.length, 8);
    entry.writeUInt32LE(offset, 12);
    entries.push(entry);
    offset += png.length;
  }
  return Buffer.concat([header, ...entries, ...pngs]);
}
