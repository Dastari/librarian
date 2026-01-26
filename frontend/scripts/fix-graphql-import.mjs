#!/usr/bin/env node
/**
 * After codegen, fix the TypedDocumentNode import to be type-only for verbatimModuleSyntax.
 */
import { readFileSync, writeFileSync } from "fs";
import { fileURLToPath } from "url";
import { dirname, join } from "path";

const __dirname = dirname(fileURLToPath(import.meta.url));
const generatedPath = join(__dirname, "../src/lib/graphql/generated/graphql.ts");
let content = readFileSync(generatedPath, "utf8");
content = content.replace(
  /^import \{ (TypedDocumentNode as DocumentNode) \} from "([^"]+)";/m,
  'import type { $1 } from "$2";'
);
writeFileSync(generatedPath, content);
