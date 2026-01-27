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
// Ensure TypedDocumentNode is a type-only import (erased at runtime) so we don't
// require @apollo/client to export TypedDocumentNode at runtime.
content = content.replace(
  /^import (?:type )?\{ (TypedDocumentNode as DocumentNode) \} from ["']([^"']+)["'];?/m,
  'import type { TypedDocumentNode as DocumentNode } from "$2";'
);
writeFileSync(generatedPath, content);
