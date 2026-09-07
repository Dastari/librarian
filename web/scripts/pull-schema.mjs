#!/usr/bin/env node
/**
 * Pulls the live GraphQL schema from the backend and writes it as SDL to
 * `schema.graphql`. Codegen reads the SDL so builds are reproducible offline.
 *
 *   SCHEMA_URL=http://127.0.0.1:3001/graphql node scripts/pull-schema.mjs
 */
import { writeFileSync } from "node:fs";
import { resolve } from "node:path";

import { buildClientSchema, getIntrospectionQuery, printSchema } from "graphql";

const url = process.env.SCHEMA_URL ?? "http://127.0.0.1:3001/graphql";
const target = resolve(process.cwd(), "schema.graphql");

const response = await fetch(url, {
  method: "POST",
  headers: { "content-type": "application/json" },
  body: JSON.stringify({ query: getIntrospectionQuery({ descriptions: true, inputValueDeprecation: true }) }),
});

if (!response.ok) {
  console.error(`Schema introspection failed: ${response.status} ${response.statusText}`);
  process.exit(1);
}

const payload = await response.json();
if (payload.errors?.length) {
  console.error("Schema introspection returned errors:", payload.errors);
  process.exit(1);
}

const sdl = printSchema(buildClientSchema(payload.data));
writeFileSync(target, `${sdl}\n`);
console.log(`Wrote ${target} (${sdl.length.toLocaleString()} bytes)`);
