#!/usr/bin/env node
/**
 * Renders the Librarian mark into every raster icon the PWA manifest, iOS home screen and
 * browser tabs need. Run with `pnpm icons`; output lands in `public/`.
 */
import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { resolve } from "node:path";

import sharp from "sharp";

const root = process.cwd();
const mark = readFileSync(resolve(root, "src/assets/brand/mark.svg"));
const iconsDir = resolve(root, "public/icons");
mkdirSync(iconsDir, { recursive: true });

const BACKGROUND = "#0a0a0f";

/** The mark centred on a rounded dark tile; `inset` is the fraction of the canvas kept clear. */
async function tile(size, inset, radiusFraction) {
  const inner = Math.round(size * (1 - inset * 2));
  const artwork = await sharp(mark).resize(inner, inner, { fit: "contain", background: { r: 0, g: 0, b: 0, alpha: 0 } }).png().toBuffer();
  const radius = Math.round(size * radiusFraction);
  const shape = Buffer.from(`<svg xmlns="http://www.w3.org/2000/svg" width="${size}" height="${size}"><rect width="${size}" height="${size}" rx="${radius}" fill="${BACKGROUND}"/></svg>`);
  return sharp(shape).composite([{ input: artwork, gravity: "centre" }]).png().toBuffer();
}

async function write(file, buffer) {
  writeFileSync(resolve(root, file), buffer);
}

async function shortcut(size, file, glyph) {
  const inner = Math.round(size * 0.5);
  const svg = Buffer.from(`<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" width="${inner}" height="${inner}" fill="none" stroke="#f5b642" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">${glyph}</svg>`);
  const buffer = await sharp({ create: { width: size, height: size, channels: 4, background: BACKGROUND } })
    .composite([{ input: await sharp(svg).png().toBuffer(), gravity: "centre" }])
    .png()
    .toBuffer();
  await write(file, buffer);
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

await Promise.all([
  tile(192, 0.14, 0.22).then((buffer) => write("public/icons/icon-192.png", buffer)),
  tile(512, 0.14, 0.22).then((buffer) => write("public/icons/icon-512.png", buffer)),
  tile(180, 0.14, 0).then((buffer) => write("public/apple-touch-icon.png", buffer)),
  tile(192, 0.2, 0).then((buffer) => write("public/icons/icon-maskable-192.png", buffer)),
  tile(512, 0.2, 0).then((buffer) => write("public/icons/icon-maskable-512.png", buffer)),
  shortcut(192, "public/icons/shortcut-search.png", '<circle cx="10" cy="10" r="7"/><path d="m21 21-6-6"/>'),
  shortcut(192, "public/icons/shortcut-downloads.png", '<path d="M12 4v12m0 0 5-5m-5 5-5-5"/><path d="M4 20h16"/>'),
  Promise.all([tile(32, 0.08, 0.2), tile(48, 0.08, 0.2)]).then((pngs) => write("public/favicon.ico", buildIco(pngs))),
]);
writeFileSync(resolve(iconsDir, "icon.svg"), mark);
console.log("Icons written to public/");
